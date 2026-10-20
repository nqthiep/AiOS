//! aios-dev-confirm — M0 stand-in for `aios-trusted-ui` (docs/m0/confirm.md).
//!
//! Reads a `PendingConfirm` as JSON, prints its full contents (args
//! NFC-normalized, control and bidi characters escaped so they cannot
//! silently change how the dialog reads, design.md §7.7), asks y/N on a
//! TTY, signs the claims with the dev trusted-UI key and prints the token
//! on stdout. Refuses to run when stdin is not a TTY. M1 replaces this
//! binary with `aios-trusted-ui`.
//!
//! Usage: `aios-dev-confirm --key <file> --pending <file|->`
//! Exit status: 0 confirmed (token printed), 1 not confirmed, 2 error.
//! Everything human-readable goes to stderr; stdout carries only the token.

use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use aios_confirm::{ConfirmClaims, PendingConfirm, SignedStep, args_hash, normalize, sign};
use anyhow::{Context, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;

const USAGE: &str = "usage: aios-dev-confirm --key <file> --pending <file|->";
const NONCE_BYTES: usize = 16;
/// Single-step tokens live 60 s, batch tokens 3600 s (M0).
const MAX_STEP_SECONDS: u64 = 60;
const MAX_BATCH_SECONDS: u64 = 3600;

enum Outcome {
    Confirmed,
    Denied,
}

fn main() -> ExitCode {
    match run() {
        Ok(Outcome::Confirmed) => ExitCode::SUCCESS,
        Ok(Outcome::Denied) => {
            eprintln!("aios-dev-confirm: not confirmed");
            ExitCode::from(1)
        }
        Err(err) => {
            eprintln!("aios-dev-confirm: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> anyhow::Result<Outcome> {
    // Refuse to run without an interactive user (docs/m0/confirm.md).
    if !std::io::stdin().is_terminal() {
        bail!("stdin is not a TTY; refusing to confirm");
    }

    let (key_path, pending_path) = parse_args()?;
    let key = load_key(&key_path)?;
    let pending = load_pending(&pending_path)?;
    show(&pending);

    if !prompt_yes()? {
        return Ok(Outcome::Denied);
    }

    let token = sign_pending(&pending, &key)?;
    println!("{token}");
    Ok(Outcome::Confirmed)
}

fn parse_args() -> anyhow::Result<(String, String)> {
    let mut argv = std::env::args().skip(1);
    let mut key = None;
    let mut pending = None;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--key" => key = Some(argv.next().context("--key needs a file path")?),
            "--pending" => pending = Some(argv.next().context("--pending needs a file path or -")?),
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    match (key, pending) {
        (Some(k), Some(p)) => Ok((k, p)),
        _ => bail!("missing arguments; {USAGE}"),
    }
}

/// The dev key file holds the 32 raw seed bytes.
fn load_key(path: &str) -> anyhow::Result<SigningKey> {
    let bytes = std::fs::read(path).with_context(|| format!("reading key file {path:?}"))?;
    let seed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("key file {path:?} must hold exactly 32 raw bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

fn load_pending(path: &str) -> anyhow::Result<PendingConfirm> {
    let json = if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading PendingConfirm from stdin")?;
        buf
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?
    };
    serde_json::from_str(&json).context("parsing PendingConfirm JSON")
}

fn show(p: &PendingConfirm) {
    eprintln!("confirm request {}", p.request_id);
    eprintln!("  principal: {}", p.principal);
    eprintln!("  uid: {}", p.uid);
    eprintln!("  task_step: {}", p.task_step);
    eprintln!("  policy_version: {}", p.policy_version);
    eprintln!(
        "  dry_run_hash: {}",
        p.dry_run_hash.as_deref().unwrap_or("(none)")
    );
    eprintln!("  created_unix: {}", p.created_unix);
    for (i, (tool, args, risk)) in p.steps.iter().enumerate() {
        eprintln!(
            "  step {}/{}: {tool} (risk: {})",
            i + 1,
            p.steps.len(),
            risk_label(*risk)
        );
        // NFC before display; no silent truncation of long args (§7.7).
        let canonical = normalize(args);
        let pretty = serde_json::to_string_pretty(&canonical).expect("args are valid JSON");
        eprintln!("{}", indent(&escape_display(&pretty)));
    }
}

fn risk_label(risk: aios_types::Risk) -> &'static str {
    match risk {
        aios_types::Risk::Low => "low",
        aios_types::Risk::Medium => "medium",
        aios_types::Risk::High => "high",
    }
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render control characters and bidi controls (U+202A–U+202E,
/// U+2066–U+2069) as escape codes (design.md §7.7).
fn escape_display(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = u32::from(c);
        if c.is_control() || matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069) {
            let _ = write!(out, "\\u{{{:04x}}}", cp);
        } else {
            out.push(c);
        }
    }
    out
}

fn prompt_yes() -> anyhow::Result<bool> {
    use std::io::Write as _;
    eprint!("Proceed? [y/N] ");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("reading confirmation")?;
    Ok(line.trim().eq_ignore_ascii_case("y"))
}

fn sign_pending(p: &PendingConfirm, key: &SigningKey) -> anyhow::Result<String> {
    let window = if p.steps.len() <= 1 {
        MAX_STEP_SECONDS
    } else {
        MAX_BATCH_SECONDS
    };
    let steps = p
        .steps
        .iter()
        .map(|(tool, args, _)| {
            Ok(SignedStep {
                tool: tool.clone(),
                args_hash: args_hash(args)?,
            })
        })
        .collect::<Result<Vec<_>, aios_confirm::ConfirmError>>()?;
    let claims = ConfirmClaims {
        v: 1,
        request_id: p.request_id.clone(),
        principal: p.principal.clone(),
        uid: p.uid,
        task_step: p.task_step.clone(),
        steps,
        dry_run_hash: p.dry_run_hash.clone(),
        policy_version: p.policy_version.clone(),
        nonce: new_nonce()?,
        exp_unix: p.created_unix.saturating_add(window),
    };
    Ok(sign(&claims, key))
}

fn new_nonce() -> anyhow::Result<String> {
    let mut bytes = [0u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).context("generating nonce")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
