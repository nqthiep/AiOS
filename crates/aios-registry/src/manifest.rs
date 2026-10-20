//! The `plugin.toml` manifest (docs/m0/registry.md, design.md §15.2 and §9.2).
//!
//! Unknown fields are rejected: a manifest that carries a field this crate does
//! not know is refused instead of being silently interpreted. The same holds
//! for the sandbox profile of the manifest.

use std::collections::BTreeMap;

use aios_types::{Privacy, Provenance, Risk, SandboxProfile};
use serde::{Deserialize, Deserializer, de};

use crate::PLUGIN_API;

/// The runtime of a plugin (design.md §15.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    /// A wasm component.
    Wasm,
    /// A separate process.
    Process,
    /// An MCP server.
    Mcp,
}

impl Runtime {
    /// The name of the runtime as used in manifests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wasm => "wasm",
            Self::Process => "process",
            Self::Mcp => "mcp",
        }
    }
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A JSON Schema, either written as a JSON string or as an inline table.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonSchema(pub serde_json::Value);

impl JsonSchema {
    /// The schema as a JSON value.
    pub fn value(&self) -> &serde_json::Value {
        &self.0
    }
}

impl<'de> Deserialize<'de> for JsonSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        match value {
            toml::Value::String(text) => serde_json::from_str(&text)
                .map(JsonSchema)
                .map_err(de::Error::custom),
            other => Ok(JsonSchema(toml_to_json(other))),
        }
    }
}

fn toml_to_json(value: toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(text) => serde_json::Value::String(text),
        toml::Value::Integer(number) => serde_json::Value::Number(number.into()),
        toml::Value::Float(number) => serde_json::Number::from_f64(number)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(flag) => serde_json::Value::Bool(flag),
        toml::Value::Datetime(stamp) => serde_json::Value::String(stamp.to_string()),
        toml::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => serde_json::Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
        ),
    }
}

fn default_input_schema() -> JsonSchema {
    JsonSchema(serde_json::json!({ "type": "object" }))
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

fn default_memory_max_mb() -> u32 {
    512
}

/// The sandbox profile of a plugin (design.md §15.2, §15.3).
///
/// `$WORKSPACE` is kept as the plain string it is written as; the executor
/// substitutes the real workspace path when the tool runs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestSandbox {
    /// Paths the plugin may read.
    #[serde(default)]
    pub ro_paths: Vec<String>,
    /// Paths the plugin may write.
    #[serde(default)]
    pub rw_paths: Vec<String>,
    /// TCP ports the plugin may connect to. Empty means no network.
    #[serde(default)]
    pub tcp_connect: Vec<u16>,
    /// Unix sockets the plugin may use.
    #[serde(default)]
    pub allow_unix_socket: Vec<String>,
    /// Memory limit of the plugin.
    #[serde(default = "default_memory_max_mb")]
    pub memory_max_mb: u32,
    /// Whether Landlock has to be enforced.
    #[serde(default)]
    pub landlock_required: bool,
}

impl ManifestSandbox {
    /// The shared sandbox profile of `aios-types`.
    pub fn to_profile(&self) -> SandboxProfile {
        SandboxProfile {
            ro_paths: self.ro_paths.clone(),
            rw_paths: self.rw_paths.clone(),
            tcp_connect: self.tcp_connect.clone(),
            allow_unix_socket: self.allow_unix_socket.clone(),
            memory_max_mb: self.memory_max_mb,
            landlock_required: self.landlock_required,
        }
    }
}

/// A tool of a plugin.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolManifest {
    /// The tool name; it has to live in the namespace of the plugin.
    pub name: String,
    /// What the tool does.
    #[serde(default)]
    pub description: String,
    /// The declared risk of the tool.
    pub risk: Risk,
    /// Whether the tool can be undone.
    #[serde(default)]
    pub reversible: bool,
    /// The tool that undoes this one.
    #[serde(default)]
    pub rollback_tool: Option<String>,
    /// Whether the tool supports a dry run.
    #[serde(default)]
    pub supports_dry_run: bool,
    /// The privileged helper the tool runs through.
    #[serde(default)]
    pub runs_via: Option<String>,
    /// Always ask the user, whatever the autonomy level is.
    #[serde(default)]
    pub always_confirm: bool,
    /// The tool affects the whole system; only an administrator may call it.
    #[serde(default)]
    pub system_scope: bool,
    /// The tool writes into the trusted core.
    #[serde(default)]
    pub writes_trusted_core: bool,
    /// The JSON Schema of the arguments of the tool.
    #[serde(default = "default_input_schema")]
    pub input_schema: JsonSchema,
    /// The privacy label of the result of the tool.
    #[serde(default)]
    pub result_privacy: Option<Privacy>,
    /// The provenance of the result of the tool.
    #[serde(default)]
    pub result_provenance: Option<Provenance>,
}

/// A plugin manifest (`plugin.toml`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The plugin id, for example `aios.fs`.
    pub id: String,
    /// The version of the plugin.
    pub version: String,
    /// The manifest API version; only `aios:plugin@0.1` is supported.
    pub api: String,
    /// What the plugin provides.
    pub provides: Vec<String>,
    /// The runtime of the plugin.
    pub runtime: Runtime,
    /// The command that starts the plugin.
    #[serde(deserialize_with = "deserialize_entry")]
    pub entry: Vec<String>,
    /// The declared trust level. Ignored: the trust level comes from the
    /// directory the plugin is loaded from (design.md §15.2).
    #[serde(default)]
    pub trust: Option<String>,
    /// The capabilities the plugin needs.
    pub capabilities: Vec<String>,
    /// The services the plugin needs.
    #[serde(default)]
    pub inject: Vec<String>,
    /// The JSON Schema of the plugin configuration.
    #[serde(default)]
    pub config_schema: Option<String>,
    /// The desktop entry of the plugin.
    #[serde(default)]
    pub desktop_entry: Option<String>,
    /// The sandbox profile of the plugin.
    #[serde(default)]
    pub sandbox: ManifestSandbox,
    /// The time limit of a call of a tool of this plugin.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// The default privacy label of the results of the tools of this plugin.
    #[serde(default = "default_result_privacy")]
    pub result_privacy: Privacy,
    /// The default provenance of the results of the tools of this plugin.
    #[serde(default = "default_result_provenance")]
    pub result_provenance: Provenance,
    /// Risk overrides of the plugin. Only honoured for `core` and `vendor`
    /// plugins and they may only raise a risk, never lower one.
    #[serde(default)]
    pub risk_overrides: BTreeMap<String, Risk>,
    /// The tools of the plugin.
    #[serde(default)]
    pub tools: Vec<ToolManifest>,
}

fn deserialize_entry<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Single(String),
        Many(Vec<String>),
    }

    match Entry::deserialize(deserializer)? {
        Entry::Single(entry) => Ok(vec![entry]),
        Entry::Many(entries) => Ok(entries),
    }
}

impl Manifest {
    /// Parses a manifest from the text of a `plugin.toml`.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Checks the parts of the manifest that have a fixed value.
    pub fn validate(&self) -> Result<(), String> {
        if self.api != PLUGIN_API {
            return Err(format!(
                "unsupported api {:?}, expected {:?}",
                self.api, PLUGIN_API
            ));
        }
        if self.id.is_empty() {
            return Err("the plugin id must not be empty".to_string());
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(format!(
                "the plugin id {:?} may only contain letters, digits, '.', '_' and '-'",
                self.id
            ));
        }
        if self.version.is_empty() {
            return Err("the plugin version must not be empty".to_string());
        }
        if self.entry.is_empty() {
            return Err("the plugin entry must not be empty".to_string());
        }
        Ok(())
    }

    /// The namespace the tools of this plugin have to live in: the last part
    /// of the plugin id (`aios.fs` gives `fs`).
    pub fn namespace(&self) -> &str {
        self.id.rsplit('.').next().unwrap_or(&self.id)
    }

    /// Whether a tool name lives in the namespace of this plugin.
    pub fn owns_tool(&self, tool_name: &str) -> bool {
        tool_name
            .strip_prefix(self.namespace())
            .and_then(|rest| rest.strip_prefix('.'))
            .is_some_and(|rest| !rest.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
id = "aios.fs"
version = "1.0.0"
api = "aios:plugin@0.1"
provides = ["tool"]
runtime = "mcp"
entry = ["/usr/bin/env", "python3", "-I", "server.py"]
capabilities = ["fs.read:$WORKSPACE"]

[[tools]]
name = "fs.search"
description = "Search files"
risk = "low"
reversible = true
input_schema = { type = "object" }
"#;

    #[test]
    fn parses_a_manifest() {
        let manifest = Manifest::parse(MANIFEST).unwrap();
        assert_eq!(manifest.id, "aios.fs");
        assert_eq!(manifest.runtime, Runtime::Mcp);
        assert_eq!(manifest.entry.len(), 4);
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.tools[0].name, "fs.search");
        assert_eq!(manifest.result_privacy, Privacy::Personal);
        assert_eq!(manifest.result_provenance, Provenance::Observed);
        assert_eq!(manifest.timeout_ms, 30_000);
        assert!(manifest.owns_tool("fs.search"));
        assert!(!manifest.owns_tool("sys.info"));
        assert!(!manifest.owns_tool("fsx.search"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let text = format!("{MANIFEST}\nnot_a_field = 1\n");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn accepts_a_json_schema_as_a_string() {
        let text = MANIFEST.replace(
            "input_schema = { type = \"object\" }",
            "input_schema = '{ \"type\": \"object\" }'",
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(manifest.tools[0].input_schema.value()["type"], "object");
    }

    #[test]
    fn accepts_a_single_entry_string() {
        let text = MANIFEST.replace(
            "entry = [\"/usr/bin/env\", \"python3\", \"-I\", \"server.py\"]",
            "entry = \"wifi.wasm\"",
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(manifest.entry, vec!["wifi.wasm".to_string()]);
    }
}
