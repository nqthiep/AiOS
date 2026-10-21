//! Helper errors: message + exit code (docs/m0/sandbox.md).

use std::fmt;

/// Helper exit codes (docs/m0/sandbox.md: sandbox errors use 64–79).
pub mod code {
    /// Bad command line, broken profile, or a profile path that does not exist.
    pub const USAGE: i32 = 64;
    /// bubblewrap is missing.
    pub const NO_BWRAP: i32 = 65;
    /// Landlock is missing or not fully enforced (fail closed).
    pub const LANDLOCK: i32 = 70;
    /// The seccomp filter cannot be installed.
    pub const SECCOMP: i32 = 71;
    /// The child command cannot be executed.
    pub const EXEC: i32 = 72;
}

/// A helper error: message plus exit code.
#[derive(Debug)]
pub struct Failure {
    code: i32,
    error: anyhow::Error,
}

impl Failure {
    /// Creates an error with an explicit exit code.
    pub fn new(code: i32, error: impl Into<anyhow::Error>) -> Self {
        Self {
            code,
            error: error.into(),
        }
    }

    /// Helper exit code for this error.
    pub fn code(&self) -> i32 {
        self.code
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#}", self.error)
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}
