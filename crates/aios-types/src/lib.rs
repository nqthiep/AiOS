//! Shared types for AiOS M0 crates. Spec: docs/m0/README.md §3.
//! Changing a type here is a leader decision; other crates must not redefine these.

use serde::{Deserialize, Serialize};

/// Privacy label. Ordering: Public < Personal < Secret (design.md §5.2).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Privacy {
    Public,
    Personal,
    /// Unlabelled data is treated as Secret (REQ design §5.2).
    #[default]
    Secret,
}

impl Privacy {
    pub fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
}

/// Effective risk of a tool. Ordering: Low < Medium < High.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
}

impl Risk {
    pub fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
}

/// Monotonic taint (design.md §6.3).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Taint {
    #[default]
    Clean,
    Tainted,
}

impl Taint {
    pub fn or(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Local,
    Voice,
    Remote,
    Schedule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    Stated,
    Observed,
    Inferred,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// From SO_PEERCRED or a uid-bound credential, never from a request body (design.md §18.1).
    pub uid: u32,
    pub is_admin: bool,
    pub channel: Channel,
    /// UUIDv7 string.
    pub session_id: Option<String>,
}

fn default_memory_max_mb() -> u32 {
    512
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfile {
    #[serde(default)]
    pub ro_paths: Vec<String>,
    #[serde(default)]
    pub rw_paths: Vec<String>,
    /// Empty = no network.
    #[serde(default)]
    pub tcp_connect: Vec<u16>,
    #[serde(default)]
    pub allow_unix_socket: Vec<String>,
    #[serde(default = "default_memory_max_mb")]
    pub memory_max_mb: u32,
    /// Must be true for every tool with risk >= Medium (tech-stack.md §8, fail closed).
    #[serde(default)]
    pub landlock_required: bool,
}

fn default_result_privacy() -> Privacy {
    Privacy::Personal
}
fn default_result_provenance() -> Provenance {
    Provenance::Observed
}
fn default_timeout_ms() -> u64 {
    30_000
}

/// Built by aios-registry from a signed manifest. `risk` is the EFFECTIVE risk (design.md §15.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub plugin_id: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub risk: Risk,
    pub reversible: bool,
    #[serde(default)]
    pub rollback_tool: Option<String>,
    #[serde(default)]
    pub always_confirm: bool,
    #[serde(default)]
    pub system_scope: bool,
    #[serde(default)]
    pub writes_trusted_core: bool,
    #[serde(default = "default_result_privacy")]
    pub result_privacy: Privacy,
    #[serde(default = "default_result_provenance")]
    pub result_provenance: Provenance,
    pub entry: Vec<String>,
    pub sandbox: SandboxProfile,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl ToolSpec {
    /// A tool "writes" when it is risk >= Medium or not reversible (used for the voice autonomy cap, design.md §11.2).
    pub fn writes(&self) -> bool {
        self.risk >= Risk::Medium || !self.reversible
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub uid: u32,
    /// Only ever raised (design.md §5.2).
    pub label: Privacy,
    /// Only ever raised (design.md §6.3).
    pub taint: Taint,
}

impl SessionState {
    pub fn new(session_id: impl Into<String>, uid: u32) -> Self {
        Self {
            session_id: session_id.into(),
            uid,
            label: Privacy::Public,
            taint: Taint::Clean,
        }
    }

    /// Raise label and taint monotonically; returns true when anything changed.
    pub fn raise(&mut self, label: Privacy, taint: Taint) -> bool {
        let (l, t) = (self.label.max(label), self.taint.or(taint));
        let changed = l != self.label || t != self.taint;
        self.label = l;
        self.taint = t;
        changed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmLevel {
    Dialog,
    DialogWithPin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict", content = "detail")]
pub enum Verdict {
    Allow,
    Confirm(ConfirmLevel),
    Deny(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
    /// Required (REQ tech-stack §5 idempotency key).
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// MCP result content[].
    pub content: serde_json::Value,
    pub is_error: bool,
    /// = ToolSpec.result_privacy, may be raised by the tool, never lowered.
    pub privacy: Privacy,
    pub provenance: Provenance,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enums_serialize_lowercase() {
        assert_eq!(
            serde_json::to_value(Privacy::Personal).unwrap(),
            json!("personal")
        );
        assert_eq!(serde_json::to_value(Risk::High).unwrap(), json!("high"));
        assert_eq!(
            serde_json::to_value(Taint::Tainted).unwrap(),
            json!("tainted")
        );
        assert_eq!(
            serde_json::to_value(Channel::Schedule).unwrap(),
            json!("schedule")
        );
        assert_eq!(
            serde_json::to_value(Provenance::External).unwrap(),
            json!("external")
        );
        assert_eq!(
            serde_json::to_value(ConfirmLevel::DialogWithPin).unwrap(),
            json!("dialog_with_pin")
        );
    }

    #[test]
    fn ordering_and_max() {
        assert!(Privacy::Public < Privacy::Personal && Privacy::Personal < Privacy::Secret);
        assert!(Risk::Low < Risk::Medium && Risk::Medium < Risk::High);
        assert_eq!(Privacy::Personal.max(Privacy::Public), Privacy::Personal);
        assert_eq!(Taint::Clean.or(Taint::Tainted), Taint::Tainted);
        assert_eq!(Privacy::default(), Privacy::Secret);
    }

    #[test]
    fn session_raise_is_monotonic() {
        let mut s = SessionState::new("s1", 1000);
        assert!(s.raise(Privacy::Secret, Taint::Clean));
        assert!(!s.raise(Privacy::Public, Taint::Clean)); // cannot lower
        assert_eq!(s.label, Privacy::Secret);
        assert!(s.raise(Privacy::Public, Taint::Tainted));
        assert!(!s.raise(Privacy::Public, Taint::Clean));
        assert_eq!(s.taint, Taint::Tainted);
    }

    #[test]
    fn toolspec_roundtrip_with_defaults() {
        let v = json!({
            "name": "fs.search", "plugin_id": "aios.fs", "description": "d",
            "input_schema": {"type": "object"}, "risk": "low", "reversible": true,
            "entry": ["python3", "server.py"], "sandbox": {"ro_paths": ["$WORKSPACE"]}
        });
        let t: ToolSpec = serde_json::from_value(v).unwrap();
        assert_eq!(t.result_privacy, Privacy::Personal);
        assert_eq!(t.result_provenance, Provenance::Observed);
        assert_eq!(t.timeout_ms, 30_000);
        assert_eq!(t.sandbox.memory_max_mb, 512);
        assert!(!t.writes());
        let back: ToolSpec = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn verdict_roundtrip() {
        for v in [
            Verdict::Allow,
            Verdict::Confirm(ConfirmLevel::Dialog),
            Verdict::Deny("no-permit".into()),
        ] {
            let back: Verdict = serde_json::from_value(serde_json::to_value(&v).unwrap()).unwrap();
            assert_eq!(back, v);
        }
    }
}
