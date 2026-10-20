//! `aios-plugin-sign` — the development helper that signs plugin manifests.
//!
//! ```text
//! aios-plugin-sign --gen-key <key-file>
//! aios-plugin-sign --key <key-file> <plugin.toml>
//! ```
//!
//! `--gen-key` writes a fresh Ed25519 key pair: the secret key goes to
//! `<key-file>` and the public key to `<key-file>.pub` (when `<key-file>`
//! already ends in `.pub` the roles are swapped). Both files hold the standard
//! base64 encoding of the raw key bytes.
//!
//! `--key` signs the exact bytes of `<plugin.toml>` and writes the standard
//! base64 encoded signature to `<plugin.toml>.sig`.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use aios_registry::{generate_keypair, read_secret_key, sign_file, write_key_pair};

const USAGE: &str = "\
aios-plugin-sign — sign plugin manifests for the AiOS registry

usage:
  aios-plugin-sign --gen-key <key-file>          generate a new key pair
  aios-plugin-sign --key <key-file> <manifest>   sign <manifest> to <manifest>.sig

The secret key is written to <key-file>, the public key to <key-file>.pub.
When <key-file> ends in .pub the public key is written there and the secret
key to <key-file>.key. Both files hold base64 encoded raw key bytes.
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args)? {
        Command::Help => {
            print!("{USAGE}");
            Ok(())
        }
        Command::GenKey(path) => {
            let pair = generate_keypair();
            let (secret_path, public_path) = write_key_pair(&path, &pair)
                .with_context(|| format!("cannot write the key pair to {}", path.display()))?;
            println!("private key: {}", secret_path.display());
            println!("public key:  {}", public_path.display());
            println!("public key (base64): {}", pair.public().to_base64());
            Ok(())
        }
        Command::Sign { key, manifest } => {
            let secret = read_secret_key(&key)
                .with_context(|| format!("cannot read the secret key {}", key.display()))?;
            let signature_path = sign_file(&manifest, &secret)
                .with_context(|| format!("cannot sign {}", manifest.display()))?;
            println!("signature: {}", signature_path.display());
            Ok(())
        }
    }
}

enum Command {
    GenKey(PathBuf),
    Sign { key: PathBuf, manifest: PathBuf },
    Help,
}

fn parse_args(args: &[String]) -> Result<Command> {
    if args.is_empty() || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(Command::Help);
    }

    let mut key: Option<PathBuf> = None;
    let mut gen_key: Option<PathBuf> = None;
    let mut manifest: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--gen-key" => {
                let value = args.get(index + 1).context("--gen-key needs a file")?;
                gen_key = Some(PathBuf::from(value));
                index += 2;
            }
            "--key" => {
                let value = args.get(index + 1).context("--key needs a file")?;
                key = Some(PathBuf::from(value));
                index += 2;
            }
            other if other.starts_with('-') => bail!("unknown option {other}"),
            other => {
                if manifest.is_some() {
                    bail!("only one manifest can be signed at a time");
                }
                manifest = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }

    match (gen_key, key, manifest) {
        (Some(_), Some(_), _) => bail!("--gen-key and --key cannot be used together"),
        (Some(path), None, None) => Ok(Command::GenKey(path)),
        (Some(_), None, Some(_)) => bail!("--gen-key does not sign a manifest"),
        (None, Some(key), Some(manifest)) => Ok(Command::Sign { key, manifest }),
        (None, Some(_), None) => bail!("--key needs a manifest to sign"),
        (None, None, Some(_)) => bail!("a manifest needs a key: use --key <key-file>"),
        (None, None, None) => Ok(Command::Help),
    }
}
