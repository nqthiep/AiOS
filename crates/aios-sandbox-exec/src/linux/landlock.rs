//! Stage 2 Landlock ruleset (docs/m0/sandbox.md).
//!
//! Landlock ABI V9 ruleset: handles every filesystem and network access,
//! grants read to `ro_paths` + `/usr` + `/etc/ssl`, write to `rw_paths` +
//! `/tmp`, connect (`RESOLVE_UNIX`, V9) to each `allow_unix_socket`, TCP
//! connect to each port in `tcp_connect`, and scopes abstract UNIX sockets
//! and signals. Rules on non-directories only carry file access rights
//! (S-14 L-1); V9 is needed so pathname UNIX sockets outside
//! `allow_unix_socket` cannot be connected (S-14 L-2).
//!
//! When the profile sets `landlock_required` and the status is not
//! `FullyEnforced`, the helper refuses to run the command (fail closed,
//! REQ-T-8-01).

use std::path::Path;

use aios_types::SandboxProfile;
use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, NetPort, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr, RulesetStatus, Scope,
};

use crate::linux::error::{Failure, code};

/// Landlock ABI requested by the sandbox.
pub const REQUESTED_ABI: ABI = ABI::V9;

/// Paths always readable in the sandbox.
const READ_PATHS: &[&str] = &["/usr", "/etc/ssl"];

/// Paths always writable in the sandbox.
const WRITE_PATHS: &[&str] = &["/tmp"];

/// Test-only environment variable: treat the kernel as supporting only this
/// ABI. It can only make the result stricter (never upgrades enforcement).
const FORCE_ABI_ENV: &str = "AIOS_SANDBOX_TEST_FORCE_ABI";

/// Outcome of applying Landlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    /// The kernel enforces every requested access right.
    FullyEnforced,
    /// Some requested access rights are not enforced.
    PartiallyEnforced,
}

/// Applies the profile's Landlock rules to the current thread.
pub fn apply(profile: &SandboxProfile) -> Result<Enforcement, Failure> {
    let abi = REQUESTED_ABI;
    let read = AccessFs::from_read(abi);
    let write = AccessFs::from_all(abi);

    let mut ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(abi)))
        .and_then(|ruleset| ruleset.scope(Scope::from_all(abi)))
        .map_err(|err| {
            Failure::new(
                code::LANDLOCK,
                anyhow::anyhow!("cannot build the landlock ruleset: {err}"),
            )
        })?
        .create()
        .map_err(|err| {
            Failure::new(
                code::LANDLOCK,
                anyhow::anyhow!("cannot create the landlock ruleset: {err}"),
            )
        })?;

    for path in READ_PATHS {
        if Path::new(path).exists() {
            ruleset = add_path_rule(ruleset, path, read)?;
        }
    }
    for path in &profile.ro_paths {
        ruleset = add_path_rule(ruleset, path, read)?;
    }
    for path in WRITE_PATHS {
        ruleset = add_path_rule(ruleset, path, write)?;
    }
    for path in &profile.rw_paths {
        ruleset = add_path_rule(ruleset, path, write)?;
    }
    for path in &profile.allow_unix_socket {
        ruleset = add_path_rule(ruleset, path, AccessFs::ResolveUnix.into())?;
    }
    for port in &profile.tcp_connect {
        ruleset = ruleset
            .add_rule(NetPort::new(*port, AccessNet::ConnectTcp))
            .map_err(|err| {
                Failure::new(
                    code::LANDLOCK,
                    anyhow::anyhow!("cannot allow the tcp port {port}: {err}"),
                )
            })?;
    }

    let status = ruleset.restrict_self().map_err(|err| {
        Failure::new(
            code::LANDLOCK,
            anyhow::anyhow!("cannot restrict the current process: {err}"),
        )
    })?;

    Ok(enforcement(&status.ruleset))
}

/// Decides whether the sandbox is fully enforced.
fn enforcement(status: &RulesetStatus) -> Enforcement {
    // Test hook: emulate an older kernel on which the V9 access rights
    // cannot all be enforced.
    if let Some(abi) = forced_abi()
        && abi < REQUESTED_ABI
    {
        return Enforcement::PartiallyEnforced;
    }
    match status {
        RulesetStatus::FullyEnforced => Enforcement::FullyEnforced,
        RulesetStatus::PartiallyEnforced | RulesetStatus::NotEnforced => {
            Enforcement::PartiallyEnforced
        }
    }
}

/// Reads the test hook that forces the kernel ABI.
fn forced_abi() -> Option<ABI> {
    let raw = std::env::var(FORCE_ABI_ENV).ok()?;
    let value: i32 = raw.trim().parse().ok()?;
    Some(ABI::from(value))
}

/// Adds a rule allowing `access` beneath `path`.
fn add_path_rule(
    ruleset: RulesetCreated,
    path: &str,
    access: BitFlags<AccessFs>,
) -> Result<RulesetCreated, Failure> {
    // Directory-only rights on a file make the ruleset partially enforced,
    // which fails closed (S-14 L-1): keep only the rights valid for files.
    let access = if Path::new(path).is_dir() {
        access
    } else {
        access & AccessFs::from_file(REQUESTED_ABI)
    };
    let fd = PathFd::new(path).map_err(|err| {
        Failure::new(
            code::LANDLOCK,
            anyhow::anyhow!("cannot open {path} for the landlock rules: {err}"),
        )
    })?;
    ruleset
        .add_rule(PathBeneath::new(fd, access))
        .map_err(|err| {
            Failure::new(
                code::LANDLOCK,
                anyhow::anyhow!("cannot allow {path}: {err}"),
            )
        })
}
