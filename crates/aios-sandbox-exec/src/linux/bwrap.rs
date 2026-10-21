//! Stage 1: build the `bwrap` command line and `execve` it.
//!
//! bwrap flags per docs/m0/sandbox.md:
//! - `--unshare-all`, plus `--share-net` when the profile allows TCP connects,
//!   otherwise `--unshare-net`;
//! - `--die-with-parent --new-session --cap-drop ALL`;
//! - `--ro-bind /usr /usr` and the Fedora `/bin`, `/lib`, `/lib64`, `/sbin` symlinks;
//! - `--proc /proc --dev /dev --tmpfs /tmp`;
//! - each `ro_paths` entry → `--ro-bind`, each `rw_paths` entry → `--bind`;
//! - the command inside bubblewrap is this helper with `--stage2 --profile <file> -- <cmd...>`.

use std::path::Path;

use aios_types::SandboxProfile;

use crate::linux::cli::Cli;
use crate::linux::error::{Failure, code};
use crate::linux::exec;

/// Read-only directories always present in the sandbox (match the Landlock read rules).
const READ_PATHS: &[&str] = &["/etc/ssl"];

/// Runs the child command inside bubblewrap.
pub fn run(cli: &Cli, profile: &SandboxProfile) -> Result<(), Failure> {
    let helper = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|err| {
            Failure::new(
                code::USAGE,
                anyhow::anyhow!("cannot locate the sandbox helper: {err}"),
            )
        })?;
    let profile_path = cli.profile.canonicalize().map_err(|err| {
        Failure::new(
            code::USAGE,
            anyhow::anyhow!("cannot locate the profile {}: {err}", cli.profile.display()),
        )
    })?;
    let bwrap = exec::resolve("bwrap").ok_or_else(|| {
        Failure::new(
            code::NO_BWRAP,
            anyhow::anyhow!("bubblewrap (bwrap) is not installed"),
        )
    })?;

    let args = bwrap_args(cli, profile, &helper, &profile_path);
    Err(exec::exec(&bwrap, &args))
}

/// Builds the bubblewrap argv.
fn bwrap_args(
    cli: &Cli,
    profile: &SandboxProfile,
    helper: &Path,
    profile_path: &Path,
) -> Vec<String> {
    let mut args: Vec<String> = vec!["bwrap".to_string(), "--unshare-all".to_string()];
    if profile.tcp_connect.is_empty() {
        args.push("--unshare-net".to_string());
    } else {
        args.push("--share-net".to_string());
    }
    args.extend(
        [
            // Nested user namespaces (clone(CLONE_NEWUSER)) would bypass the
            // seccomp `unshare` deny; --disable-userns needs --unshare-user.
            "--unshare-user",
            "--disable-userns",
            "--die-with-parent",
            "--new-session",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/usr",
            "/usr",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--symlink",
            "usr/sbin",
            "/sbin",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
        ]
        .map(String::from),
    );

    // TLS trust anchors: Landlock grants read access to /etc/ssl, so it must
    // exist inside the sandbox.
    for path in READ_PATHS {
        if Path::new(path).exists() {
            args.extend(["--ro-bind", path, path].map(String::from));
        }
    }

    for path in &profile.ro_paths {
        args.extend(["--ro-bind", path, path].map(String::from));
    }
    for path in &profile.rw_paths {
        args.extend(["--bind", path, path].map(String::from));
    }
    // Connecting to a UNIX socket needs write access to the socket inode.
    for path in &profile.allow_unix_socket {
        args.extend(["--bind", path, path].map(String::from));
    }

    // Stage 2 runs this helper again and re-reads the profile: both must be
    // visible inside the sandbox.
    let helper = helper.to_string_lossy();
    let profile_path = profile_path.to_string_lossy();
    args.extend(["--ro-bind", &helper, &helper].map(String::from));
    args.extend(["--ro-bind", &profile_path, &profile_path].map(String::from));

    args.push("--".to_string());
    args.push(helper.to_string());
    args.push("--stage2".to_string());
    args.push("--profile".to_string());
    args.push(profile_path.to_string());
    args.push("--".to_string());
    args.extend(cli.command.iter().cloned());
    args
}
