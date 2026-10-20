//! `aios-audit` — command line of the audit log.
//!
//! Usage: `aios-audit verify <file>`.

use std::path::PathBuf;
use std::process::ExitCode;

use aios_audit::verify;
use anyhow::{Result, bail};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next();
    let file = arguments.next();
    if arguments.next().is_some() {
        bail!("usage: aios-audit verify <file>");
    }
    match (command.as_deref(), file) {
        (Some("verify"), Some(file)) => {
            let report = verify(&PathBuf::from(file))?;
            match report.broken {
                None => {
                    println!("ok {} dòng", report.lines);
                    Ok(ExitCode::SUCCESS)
                }
                Some(broken) => {
                    println!("hỏng ở seq={}: {}", broken.seq, broken.reason);
                    Ok(ExitCode::from(1))
                }
            }
        }
        _ => bail!("usage: aios-audit verify <file>"),
    }
}
