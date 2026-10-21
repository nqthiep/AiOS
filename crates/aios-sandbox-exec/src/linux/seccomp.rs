//! Stage 2 seccomp filter (docs/m0/sandbox.md).
//!
//! Default action is allow; the following fail with `EPERM`:
//! - `socket` with a domain other than `AF_UNIX`/`AF_INET`/`AF_INET6`;
//! - `socket` on `AF_INET`/`AF_INET6` that is not plain TCP: any type other
//!   than `SOCK_STREAM` (covers `SOCK_RAW` and `SOCK_DGRAM`, i.e. raw and UDP
//!   sockets), or a protocol other than 0/`IPPROTO_TCP` (SCTP, MPTCP...),
//!   because Landlock only restricts TCP (tech-stack.md §8);
//! - `ptrace`, `bpf`, `perf_event_open`, `kexec_load`, `init_module`,
//!   `finit_module`, `mount`, `umount2`, `pivot_root`, `unshare`, `setns`;
//! - `io_uring_*`: `IORING_OP_SOCKET` creates sockets without the `socket`
//!   syscall and would bypass the rules above.

use std::collections::BTreeMap;

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};

use crate::linux::error::{Failure, code};

/// Syscalls that are always denied.
const DENIED_SYSCALLS: &[libc::c_long] = &[
    libc::SYS_ptrace,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_kexec_load,
    libc::SYS_init_module,
    libc::SYS_finit_module,
    libc::SYS_mount,
    libc::SYS_umount2,
    libc::SYS_pivot_root,
    libc::SYS_unshare,
    libc::SYS_setns,
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
];

/// Socket domains that may be created.
const ALLOWED_DOMAINS: &[libc::c_int] = &[libc::AF_UNIX, libc::AF_INET, libc::AF_INET6];

/// Socket type mask (`SOCK_TYPE_MASK` in the kernel); strips `SOCK_CLOEXEC`
/// and `SOCK_NONBLOCK`. Other high bits make the kernel return `EINVAL`.
const SOCK_TYPE_MASK: u64 = 0xf;

/// Installs the seccomp filter on the current thread.
pub fn apply() -> Result<(), Failure> {
    let arch = TargetArch::try_from(std::env::consts::ARCH).map_err(|err| {
        Failure::new(
            code::SECCOMP,
            anyhow::anyhow!("unsupported architecture {}: {err}", std::env::consts::ARCH),
        )
    })?;
    let rules = build_rules().map_err(|err| {
        Failure::new(
            code::SECCOMP,
            anyhow::anyhow!("cannot build the seccomp rules: {err}"),
        )
    })?;
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|err| {
        Failure::new(
            code::SECCOMP,
            anyhow::anyhow!("cannot build the seccomp filter: {err}"),
        )
    })?;
    let program: BpfProgram = filter.try_into().map_err(|err| {
        Failure::new(
            code::SECCOMP,
            anyhow::anyhow!("cannot compile the seccomp filter: {err}"),
        )
    })?;
    seccompiler::apply_filter(&program).map_err(|err| {
        Failure::new(
            code::SECCOMP,
            anyhow::anyhow!("cannot install the seccomp filter: {err}"),
        )
    })
}

/// Builds the rule table. Rules of one syscall are OR-ed; conditions inside
/// one rule are AND-ed.
fn build_rules() -> Result<BTreeMap<i64, Vec<SeccompRule>>, seccompiler::Error> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    for syscall in DENIED_SYSCALLS {
        // An empty rule list matches the syscall unconditionally.
        rules.insert(*syscall, Vec::new());
    }
    rules.insert(libc::SYS_socket, socket_rules()?);
    Ok(rules)
}

/// One 32-bit argument comparison (`socket` takes `int` arguments).
fn dword(
    arg: u8,
    op: SeccompCmpOp,
    value: u64,
) -> Result<SeccompCondition, seccompiler::BackendError> {
    SeccompCondition::new(arg, SeccompCmpArgLen::Dword, op, value)
}

/// Deny rules for the `socket(domain, type, protocol)` syscall.
fn socket_rules() -> Result<Vec<SeccompRule>, seccompiler::Error> {
    let mut rules = Vec::new();

    // Domain is none of AF_UNIX, AF_INET, AF_INET6.
    let conditions = ALLOWED_DOMAINS
        .iter()
        .map(|domain| dword(0, SeccompCmpOp::Ne, *domain as u64))
        .collect::<Result<Vec<_>, _>>()?;
    rules.push(SeccompRule::new(conditions)?);

    for domain in [libc::AF_INET, libc::AF_INET6] {
        // Masked type is not SOCK_STREAM. seccompiler has no masked "not
        // equal", so every other value of the 4-bit type field is listed.
        for socket_type in (0..=SOCK_TYPE_MASK).filter(|t| *t != libc::SOCK_STREAM as u64) {
            rules.push(SeccompRule::new(vec![
                dword(0, SeccompCmpOp::Eq, domain as u64)?,
                dword(1, SeccompCmpOp::MaskedEq(SOCK_TYPE_MASK), socket_type)?,
            ])?);
        }
        // Stream socket with a protocol other than TCP.
        rules.push(SeccompRule::new(vec![
            dword(0, SeccompCmpOp::Eq, domain as u64)?,
            dword(2, SeccompCmpOp::Ne, 0)?,
            dword(2, SeccompCmpOp::Ne, libc::IPPROTO_TCP as u64)?,
        ])?);
    }

    Ok(rules)
}
