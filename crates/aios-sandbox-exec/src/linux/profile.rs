//! Reads the JSON profile (`SandboxProfile` from `aios-types`).

use std::path::Path;

use aios_types::SandboxProfile;

use crate::linux::error::{Failure, code};

/// Reads and parses the JSON profile from disk.
pub fn load(path: &Path) -> Result<SandboxProfile, Failure> {
    let raw = std::fs::read_to_string(path).map_err(|err| {
        Failure::new(
            code::USAGE,
            anyhow::anyhow!("cannot read profile {}: {err}", path.display()),
        )
    })?;
    let profile: SandboxProfile = serde_json::from_str(&raw).map_err(|err| {
        Failure::new(
            code::USAGE,
            anyhow::anyhow!("cannot parse profile {}: {err}", path.display()),
        )
    })?;
    Ok(profile)
}

/// Checks the profile paths: each must be absolute and must exist.
///
/// Absolute because bubblewrap mounts at that path; a missing path is a
/// broken profile (exit code 64), not a runtime error.
pub fn check_paths(profile: &SandboxProfile) -> Result<(), Failure> {
    let paths = profile
        .ro_paths
        .iter()
        .chain(profile.rw_paths.iter())
        .chain(profile.allow_unix_socket.iter());
    for path in paths {
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err(Failure::new(
                code::USAGE,
                anyhow::anyhow!("profile path must be absolute: {}", path.display()),
            ));
        }
        if !path.exists() {
            return Err(Failure::new(
                code::USAGE,
                anyhow::anyhow!("profile path does not exist: {}", path.display()),
            ));
        }
    }
    Ok(())
}
