//! Resolves a command in `PATH` and replaces the current process with it.

use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::linux::error::{Failure, code};

/// Resolves the executable path of a command.
///
/// A command containing `/` is used as is; others are searched in `PATH`.
pub fn resolve(command: &str) -> Option<PathBuf> {
    if command.contains('/') {
        return Some(PathBuf::from(command));
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(command))
        .find(|candidate| is_executable(candidate))
}

/// True when the path is a regular file with an execute bit.
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Replaces the current process with `program` and `argv` (argv[0] is the command name).
///
/// Returns only when `execve` fails; on success the child replaces this
/// process, so the child's exit code is the helper's exit code.
pub fn exec(program: &Path, argv: &[String]) -> Failure {
    let program = match to_cstring(program.as_os_str().as_bytes()) {
        Ok(value) => value,
        Err(failure) => return failure,
    };
    let mut args: Vec<CString> = Vec::with_capacity(argv.len());
    for arg in argv {
        match to_cstring(arg.as_bytes()) {
            Ok(value) => args.push(value),
            Err(failure) => return failure,
        }
    }
    let mut argv_ptrs: Vec<*const libc::c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    // The current environment is handed to the child. Environment entries
    // cannot contain NUL bytes (the kernel forbids it), so invalid entries
    // can be skipped.
    let env: Vec<CString> = std::env::vars_os()
        .filter_map(|(key, value)| {
            let mut entry = OsString::with_capacity(key.len() + value.len() + 1);
            entry.push(key);
            entry.push("=");
            entry.push(value);
            CString::new(entry.as_bytes()).ok()
        })
        .collect();
    let mut env_ptrs: Vec<*const libc::c_char> = env.iter().map(|value| value.as_ptr()).collect();
    env_ptrs.push(std::ptr::null());

    // SAFETY: execve is a raw syscall with no safe std wrapper that keeps the
    // current (Landlock + seccomp restricted) process. It only reads the
    // NUL-terminated strings behind the pointers; both arrays are
    // NULL-terminated, the CString vectors outlive the call, and execve only
    // returns on error.
    unsafe {
        libc::execve(program.as_ptr(), argv_ptrs.as_ptr(), env_ptrs.as_ptr());
    }

    Failure::new(
        code::EXEC,
        anyhow::anyhow!(
            "cannot execute {}: {}",
            program.to_string_lossy(),
            std::io::Error::last_os_error()
        ),
    )
}

/// Converts raw bytes into a NUL-terminated C string.
fn to_cstring(bytes: &[u8]) -> Result<CString, Failure> {
    CString::new(bytes)
        .map_err(|_| Failure::new(code::EXEC, anyhow::anyhow!("argument contains a NUL byte")))
}
