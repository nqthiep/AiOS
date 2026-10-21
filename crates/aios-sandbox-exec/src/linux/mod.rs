//! Linux implementation of `aios-sandbox-exec` (docs/m0/sandbox.md).

mod bwrap;
mod cli;
mod error;
mod exec;
mod landlock;
mod profile;
mod seccomp;

use crate::linux::cli::Cli;
use crate::linux::error::{Failure, code};

/// Entry point: parse the command line, then run stage 1 or stage 2.
pub fn run() -> Result<(), Failure> {
    let cli = cli::parse(std::env::args().skip(1))?;
    if cli.stage2 {
        return run_stage2(&cli);
    }
    let profile = profile::load(&cli.profile)?;
    profile::check_paths(&profile)?;
    bwrap::run(&cli, &profile)
}

/// Stage 2: already inside bubblewrap; apply Landlock, then seccomp, then `execve`.
///
/// Execution path (docs/m0/sandbox.md), in a single-threaded process:
/// 1. read the profile and check its paths;
/// 2. resolve the child command in `PATH` (before Landlock restricts opens);
/// 3. `PR_SET_NO_NEW_PRIVS` (required by Landlock and seccomp);
/// 4. Landlock ABI V9 — fail closed when the profile requires it;
/// 5. seccomp — deny dangerous syscalls with `EPERM`;
/// 6. `execve` the child command.
fn run_stage2(cli: &Cli) -> Result<(), Failure> {
    let profile = profile::load(&cli.profile)?;
    profile::check_paths(&profile)?;

    let command = cli
        .command
        .first()
        .ok_or_else(|| Failure::new(code::USAGE, anyhow::anyhow!("missing command after --")))?;
    let program = exec::resolve(command)
        .ok_or_else(|| Failure::new(code::EXEC, anyhow::anyhow!("command not found: {command}")))?;

    set_no_new_privs()?;

    let enforcement = landlock::apply(&profile)?;
    if enforcement != landlock::Enforcement::FullyEnforced {
        if profile.landlock_required {
            return Err(Failure::new(
                code::LANDLOCK,
                anyhow::anyhow!(
                    "landlock is not fully enforced: refusing to run the command (fail closed)"
                ),
            ));
        }
        eprintln!("aios-sandbox-exec: warning: landlock is only partially enforced");
    }

    seccomp::apply()?;

    Err(exec::exec(&program, &cli.command))
}

/// Sets `PR_SET_NO_NEW_PRIVS` on the current thread.
///
/// Unprivileged Landlock and seccomp both require it.
fn set_no_new_privs() -> Result<(), Failure> {
    // SAFETY: prctl is a raw libc call with no safe std wrapper.
    // PR_SET_NO_NEW_PRIVS takes no pointers; it only sets a flag on the
    // calling thread and returns 0 on success or -1 with errno.
    let rc = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if rc != 0 {
        return Err(Failure::new(
            code::LANDLOCK,
            anyhow::anyhow!(
                "cannot set PR_SET_NO_NEW_PRIVS: {}",
                std::io::Error::last_os_error()
            ),
        ));
    }
    Ok(())
}
