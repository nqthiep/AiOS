//! Helper command line: `--profile <file.json> [--stage2] -- <cmd> [args...]`.

use std::path::PathBuf;

use crate::linux::error::{Failure, code};

/// Parsed command line.
pub struct Cli {
    /// Path to the JSON profile.
    pub profile: PathBuf,
    /// The helper runs as stage 2 (already inside bubblewrap).
    pub stage2: bool,
    /// Child command and its arguments.
    pub command: Vec<String>,
}

/// Parses the arguments (without argv[0]).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, Failure> {
    let mut profile: Option<PathBuf> = None;
    let mut stage2 = false;
    let mut command: Vec<String> = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stage2" => stage2 = true,
            "--profile" => {
                let value = args.next().ok_or_else(|| {
                    Failure::new(code::USAGE, anyhow::anyhow!("--profile needs a file path"))
                })?;
                profile = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--profile=") => {
                profile = Some(PathBuf::from(&arg["--profile=".len()..]));
            }
            "--" => {
                command.extend(args);
                break;
            }
            other => {
                return Err(Failure::new(
                    code::USAGE,
                    anyhow::anyhow!("unknown argument: {other}"),
                ));
            }
        }
    }

    let profile = profile.ok_or_else(|| {
        Failure::new(
            code::USAGE,
            anyhow::anyhow!("missing --profile <file.json>"),
        )
    })?;
    if command.is_empty() {
        return Err(Failure::new(
            code::USAGE,
            anyhow::anyhow!("missing command after --"),
        ));
    }

    Ok(Cli {
        profile,
        stage2,
        command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Cli, Failure> {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn parses_profile_stage2_and_command() {
        let cli = parse_args(&[
            "--stage2",
            "--profile",
            "/tmp/p.json",
            "--",
            "sh",
            "-c",
            "true",
        ])
        .expect("parse");
        assert_eq!(cli.profile, PathBuf::from("/tmp/p.json"));
        assert!(cli.stage2);
        assert_eq!(cli.command, vec!["sh", "-c", "true"]);
    }

    #[test]
    fn parses_profile_with_equal_sign() {
        let cli = parse_args(&["--profile=/tmp/p.json", "--", "true"]).expect("parse");
        assert_eq!(cli.profile, PathBuf::from("/tmp/p.json"));
        assert!(!cli.stage2);
        assert_eq!(cli.command, vec!["true"]);
    }

    #[test]
    fn rejects_missing_profile_and_command() {
        assert!(parse_args(&["--", "true"]).is_err());
        assert!(parse_args(&["--profile", "/tmp/p.json"]).is_err());
        assert!(parse_args(&["--profile"]).is_err());
        assert!(parse_args(&["--nope", "--", "true"]).is_err());
    }
}
