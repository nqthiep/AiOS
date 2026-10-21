//! `aios-sandbox-exec` — runs a command in a sandbox described by a `SandboxProfile`.
//!
//! See docs/m0/sandbox.md.
//!
//! - Stage 1 (default): read the profile, build the `bwrap` command, `execve` it.
//! - Stage 2 (`--stage2`): already inside bubblewrap; apply Landlock (ABI V6,
//!   fail closed) + seccomp, then `execve` the child command.
//!
//! Exit codes: the child's exit code is passed through; sandbox errors use
//! 64–79 (64 bad profile/command line, 65 missing bwrap, 70 Landlock not
//! fully enforced, 71 seccomp failure, 72 child cannot be executed).

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("aios-sandbox-exec: linux only");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(failure) = linux::run() {
        eprintln!("aios-sandbox-exec: {failure}");
        std::process::exit(failure.code());
    }
}

#[cfg(target_os = "linux")]
mod linux;
