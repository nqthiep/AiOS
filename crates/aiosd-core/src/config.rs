//! `aiosd-core` configuration: `/etc/aios/core.yaml`, every key overridable
//! on the command line as `--<key-with-dashes> <value>` (docs/m0/broker.md).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use aios_broker::Autonomy;
use anyhow::{Context, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
use serde::Deserialize;
use serde_yaml_ng::{Mapping, Value};

pub const DEFAULT_CONFIG: &str = "/etc/aios/core.yaml";

/// Defaults follow docs/m0/README.md §4.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CoreConfig {
    pub registry_core_dir: PathBuf,
    pub registry_vendor_dir: PathBuf,
    pub registry_local_dir: PathBuf,
    pub trust_dir: PathBuf,
    pub policy_dir: PathBuf,
    pub audit_file: PathBuf,
    /// Ed25519 public key of the trusted UI: 32 raw bytes or base64 text.
    pub trusted_ui_pubkey: PathBuf,
    pub autonomy_file: PathBuf,
    pub gateway_admin_socket: PathBuf,
    pub gateway_url: String,
    pub gateway_client: String,
    pub mcp_listen: SocketAddr,
    pub admin_socket: PathBuf,
    pub admin_group: String,
    pub workspace_root: PathBuf,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            registry_core_dir: "/usr/share/aios/plugins".into(),
            registry_vendor_dir: "/var/lib/aios/plugins/vendor".into(),
            registry_local_dir: "/var/lib/aios/plugins/local".into(),
            trust_dir: "/usr/share/aios/trust".into(),
            policy_dir: "/usr/share/aios/policy".into(),
            audit_file: "/var/lib/aios/audit/audit.jsonl".into(),
            trusted_ui_pubkey: "/etc/aios/dev-trusted-ui.pub".into(),
            autonomy_file: "/etc/aios/autonomy.yaml".into(),
            gateway_admin_socket: "/run/aios/gateway-admin.sock".into(),
            gateway_url: "http://127.0.0.1:8090/v1".into(),
            gateway_client: "aiosd-l2".into(),
            mcp_listen: SocketAddr::from(([127, 0, 0, 1], 8091)),
            admin_socket: "/run/aios/core.sock".into(),
            admin_group: "wheel".into(),
            workspace_root: "/var/lib/aios/workspaces".into(),
        }
    }
}

/// Parse `--config <file>` plus `--<key> <value>` overrides. The default
/// config file may be absent; an explicit `--config` must exist.
pub fn load(args: impl IntoIterator<Item = String>) -> anyhow::Result<CoreConfig> {
    let mut explicit = None;
    let mut overrides = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let name = arg
            .strip_prefix("--")
            .ok_or_else(|| anyhow!("unexpected argument {arg:?}"))?;
        let value = args
            .next()
            .ok_or_else(|| anyhow!("--{name} needs a value"))?;
        if name == "config" {
            explicit = Some(PathBuf::from(value));
        } else {
            overrides.push((name.replace('-', "_"), value));
        }
    }
    let path = explicit.clone().unwrap_or_else(|| DEFAULT_CONFIG.into());
    let mut map = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_yaml_ng::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?
        {
            Value::Mapping(m) => m,
            Value::Null => Mapping::new(),
            _ => bail!("{}: expected a mapping", path.display()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && explicit.is_none() => Mapping::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    for (key, value) in overrides {
        map.insert(Value::String(key), Value::String(value));
    }
    serde_yaml_ng::from_value(Value::Mapping(map)).context("invalid aiosd-core configuration")
}

/// `/etc/aios/autonomy.yaml` (design.md §11.2); absent file = level 0.
pub fn load_autonomy(path: &Path) -> anyhow::Result<Autonomy> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        autonomy: Autonomy,
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Autonomy::default()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let file: File =
        serde_yaml_ng::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let a = file.autonomy;
    if a.level > 3 || a.per_tool_override.values().any(|l| *l > 3) {
        bail!("{}: autonomy levels must be 0..=3", path.display());
    }
    Ok(a)
}

pub fn load_pubkey(path: &Path) -> anyhow::Result<VerifyingKey> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let raw = if bytes.len() == 32 {
        bytes
    } else {
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim();
        STANDARD
            .decode(text)
            .or_else(|_| URL_SAFE_NO_PAD.decode(text))
            .with_context(|| format!("{}: not 32 raw bytes or base64", path.display()))?
    };
    let raw: [u8; 32] = raw
        .try_into()
        .map_err(|_| anyhow!("{}: an Ed25519 public key is 32 bytes", path.display()))?;
    VerifyingKey::from_bytes(&raw)
        .with_context(|| format!("{}: invalid Ed25519 key", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn file_then_flags() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("core.yaml");
        std::fs::write(
            &file,
            "policy_dir: /p\nmcp_listen: 127.0.0.1:9000\nadmin_group: adm\n",
        )
        .unwrap();
        let f = file.to_str().unwrap();
        let cfg = load(args(&[
            "--config",
            f,
            "--mcp-listen",
            "127.0.0.1:9100",
            "--audit-file",
            "/a",
        ]))
        .unwrap();
        assert_eq!(cfg.policy_dir, PathBuf::from("/p"));
        assert_eq!(cfg.admin_group, "adm");
        assert_eq!(cfg.mcp_listen.port(), 9100);
        assert_eq!(cfg.audit_file, PathBuf::from("/a"));
        assert_eq!(cfg.admin_socket, CoreConfig::default().admin_socket);
    }

    #[test]
    fn rejects_unknown_keys_and_missing_explicit_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.yaml");
        assert!(load(args(&["--config", missing.to_str().unwrap()])).is_err());
        let file = dir.path().join("core.yaml");
        std::fs::write(&file, "").unwrap();
        let f = file.to_str().unwrap();
        assert_eq!(load(args(&["--config", f])).unwrap(), CoreConfig::default());
        assert!(load(args(&["--config", f, "--no-such-key", "x"])).is_err());
        assert!(load(args(&["--config", f, "--audit-file"])).is_err());
        assert!(load(args(&["stray"])).is_err());
    }

    #[test]
    fn autonomy_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("autonomy.yaml");
        assert_eq!(load_autonomy(&file).unwrap().level, 0);
        std::fs::write(&file, "autonomy:\n  level: 2\n  per_tool_override:\n    fs.search: 1\n  quiet_hours_no_autonomous_actions: [\"23:00\", \"07:00\"]\n").unwrap();
        let a = load_autonomy(&file).unwrap();
        assert_eq!(
            (a.level, a.for_tool("fs.search"), a.for_tool("x")),
            (2, 1, 2)
        );
        std::fs::write(&file, "autonomy:\n  level: 4\n").unwrap();
        assert!(load_autonomy(&file).is_err());
    }

    #[test]
    fn pubkey_raw_or_base64() {
        let dir = tempfile::tempdir().unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key();
        let raw = dir.path().join("raw.pub");
        std::fs::write(&raw, key.as_bytes()).unwrap();
        assert_eq!(load_pubkey(&raw).unwrap(), key);
        let b64 = dir.path().join("b64.pub");
        std::fs::write(&b64, format!("{}\n", STANDARD.encode(key.as_bytes()))).unwrap();
        assert_eq!(load_pubkey(&b64).unwrap(), key);
        std::fs::write(&b64, "short").unwrap();
        assert!(load_pubkey(&b64).is_err());
    }
}
