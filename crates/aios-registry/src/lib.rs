//! `aios-registry` — the signed plugin registry.
//!
//! The registry reads `plugin.toml` manifests from the three load directories
//! of the system, verifies the Ed25519 signature over the **exact bytes** of
//! the manifest, assigns the trust level from the directory the plugin was
//! loaded from, computes the effective risk of every tool and hands the
//! resulting [`ToolSpec`]s to the Tool Broker.
//!
//! Spec: [docs/m0/registry.md](../../docs/m0/registry.md),
//! [docs/design.md](../../docs/design.md) §9.2, §15.2 and §15.7.
//!
//! # Security invariants
//!
//! * The `trust` field of a manifest is ignored; the trust level comes from the
//!   directory the plugin is loaded from.
//! * A plugin without a signature, with an invalid signature or signed with a
//!   revoked key is rejected and reported; it never breaks the rest of the
//!   registry.
//! * The effective risk of a tool is `max(declared risk, risk inferred from the
//!   capabilities of the plugin)`. A capability that matches none of the known
//!   patterns fails closed to `high`.
//! * `risk_overrides` are honoured for `core` and `vendor` plugins only and can
//!   only raise a risk, never lower one.
//! * `always_confirm` and `system_scope` can only be turned on.
//! * A tool whose name does not live in the namespace of its plugin is
//!   dropped.

mod manifest;
mod risk;
mod signing;
mod trust;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aios_types::{Risk, ToolSpec};

pub use manifest::{JsonSchema, Manifest, ManifestSandbox, Runtime, ToolManifest};
pub use risk::{RiskResolution, capability_risk, inferred_risk};
pub use signing::{
    KeyPair, PublicKey, SecretKey, SigningError, decode_signature, encode_signature,
    generate_keypair, key_pair_paths, read_secret_key, sign_bytes, sign_file, signature_path_for,
    verify_bytes, write_key_pair,
};
pub use trust::{RevokedList, RevokedPlugin, TrustLevel, TrustStore, TrustStoreError, TrustedKey};

/// The manifest API version this crate supports.
pub const PLUGIN_API: &str = "aios:plugin@0.1";
/// The name of the manifest file inside a plugin directory.
pub const MANIFEST_FILE: &str = "plugin.toml";
/// The name of the detached signature file next to a manifest.
pub const SIGNATURE_FILE: &str = "plugin.toml.sig";

/// Where the registry loads plugins from and which keys it trusts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryConfig {
    /// The directory of `core` plugins (`/usr/share/aios/plugins`).
    pub core_dir: PathBuf,
    /// The directory of `vendor` plugins (`/var/lib/aios/plugins/vendor`).
    pub vendor_dir: PathBuf,
    /// The directory of `community` plugins (`/var/lib/aios/plugins/local`).
    pub local_dir: PathBuf,
    /// The trust store (`/usr/share/aios/trust`).
    pub trust_dir: PathBuf,
}

/// What happened while the registry was loaded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// The ids of the plugins that were loaded.
    pub loaded: Vec<String>,
    /// The plugins that were refused, with the reason why.
    pub rejected: Vec<(PathBuf, String)>,
}

/// A plugin the registry loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    /// The plugin id.
    pub id: String,
    /// The version of the plugin.
    pub version: String,
    /// The trust level, assigned from the load directory.
    pub trust: TrustLevel,
    /// The directory of the plugin.
    pub path: PathBuf,
    /// The runtime of the plugin.
    pub runtime: Runtime,
    /// The capabilities the plugin declared.
    pub capabilities: Vec<String>,
    /// The risk inferred from the capabilities of the plugin.
    pub risk: Risk,
    /// The names of the tools the plugin provides.
    pub tools: Vec<String>,
}

/// The registry: the tools the broker may call.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    tools: BTreeMap<String, ToolSpec>,
    plugins: BTreeMap<String, PluginInfo>,
}

impl Registry {
    /// Loads every plugin of the three load directories.
    ///
    /// The plugins are loaded from the most trusted directory to the least
    /// trusted one. A plugin that cannot be loaded is reported in
    /// [`LoadReport::rejected`] and does not stop the other plugins from being
    /// loaded.
    pub fn load(config: &RegistryConfig) -> (Registry, LoadReport) {
        let mut report = LoadReport::default();
        let mut registry = Registry::default();

        let (store, store_error) = match TrustStore::load(&config.trust_dir) {
            Ok(store) => (store, None),
            Err(error) => {
                // The revocation list could not be read: trust nothing.
                tracing::error!(
                    path = %config.trust_dir.display(),
                    error = %error,
                    "the trust store cannot be read, no plugin is loaded"
                );
                (TrustStore::empty(), Some(error.to_string()))
            }
        };

        let sources = [
            (TrustLevel::Core, &config.core_dir),
            (TrustLevel::Vendor, &config.vendor_dir),
            (TrustLevel::Community, &config.local_dir),
        ];

        for (trust, directory) in sources {
            let plugin_dirs = match plugin_dirs(directory) {
                Ok(dirs) => dirs,
                Err(error) => {
                    report.rejected.push((
                        directory.clone(),
                        format!("cannot read the plugin directory: {error}"),
                    ));
                    continue;
                }
            };
            for plugin_dir in plugin_dirs {
                let manifest_path = plugin_dir.join(MANIFEST_FILE);
                if !manifest_path.is_file() {
                    continue;
                }
                if let Some(error) = &store_error {
                    report.rejected.push((
                        manifest_path,
                        format!("the trust store cannot be read: {error}"),
                    ));
                    continue;
                }
                match load_plugin(&manifest_path, trust, &store) {
                    Ok(loaded) => {
                        let id = loaded.info.id.clone();
                        if registry.insert(loaded) {
                            report.loaded.push(id);
                        }
                    }
                    Err(reason) => report.rejected.push((manifest_path, reason)),
                }
            }
        }

        (registry, report)
    }

    /// The tool with the given name.
    pub fn tool(&self, name: &str) -> Option<&ToolSpec> {
        self.tools.get(name)
    }

    /// Every tool of the registry, ordered by name.
    pub fn tools(&self) -> impl Iterator<Item = &ToolSpec> {
        self.tools.values()
    }

    /// The plugin with the given id.
    pub fn plugin(&self, id: &str) -> Option<&PluginInfo> {
        self.plugins.get(id)
    }

    /// Every plugin of the registry, ordered by id.
    pub fn plugins(&self) -> impl Iterator<Item = &PluginInfo> {
        self.plugins.values()
    }

    /// The number of tools of the registry.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Adds a plugin to the registry. Returns whether it was added.
    fn insert(&mut self, loaded: LoadedPlugin) -> bool {
        if let Some(previous) = self.plugins.get(&loaded.info.id) {
            tracing::warn!(
                plugin = %loaded.info.id,
                kept = %previous.trust,
                ignored = %loaded.info.trust,
                "a plugin with the same id was already loaded, the higher trust level wins"
            );
            return false;
        }
        for spec in loaded.tools {
            if let Some(previous) = self.tools.get(&spec.name) {
                tracing::warn!(
                    tool = %spec.name,
                    kept_plugin = %previous.plugin_id,
                    ignored_plugin = %spec.plugin_id,
                    "a tool with the same name was already registered, the already registered tool wins"
                );
                continue;
            }
            self.tools.insert(spec.name.clone(), spec);
        }
        self.plugins.insert(loaded.info.id.clone(), loaded.info);
        true
    }
}

/// A plugin that was loaded successfully.
struct LoadedPlugin {
    info: PluginInfo,
    tools: Vec<ToolSpec>,
}

/// The plugin directories of a load directory, ordered by path.
fn plugin_dirs(directory: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// Loads and verifies a single plugin.
fn load_plugin(
    manifest_path: &Path,
    trust: TrustLevel,
    store: &TrustStore,
) -> Result<LoadedPlugin, String> {
    let bytes = std::fs::read(manifest_path)
        .map_err(|error| format!("cannot read the manifest: {error}"))?;

    let signature_path = signature_path_for(manifest_path);
    let signature_text = match std::fs::read_to_string(&signature_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("the plugin has no signature".to_string());
        }
        Err(error) => return Err(format!("cannot read the signature: {error}")),
    };
    let signature = decode_signature(&signature_text)
        .map_err(|error| format!("the signature is invalid: {error}"))?;

    let keys = store.keys(trust);
    if keys.is_empty() {
        return Err(format!("no trusted {trust} public key is available"));
    }
    let mut signed_with_revoked_key = false;
    let mut verified = false;
    for key in keys {
        if !key
            .verifying_key()
            .verify_strict(&bytes, &signature)
            .is_ok()
        {
            continue;
        }
        if store.is_revoked_key(key) {
            signed_with_revoked_key = true;
            continue;
        }
        verified = true;
        break;
    }
    if !verified {
        if signed_with_revoked_key {
            return Err("the plugin is signed with a revoked key".to_string());
        }
        return Err("the signature of the plugin is invalid".to_string());
    }

    let text =
        String::from_utf8(bytes).map_err(|_| "the manifest is not valid UTF-8".to_string())?;
    let manifest = Manifest::parse(&text).map_err(|error| format!("invalid manifest: {error}"))?;
    manifest.validate()?;
    if store.is_revoked_plugin(&manifest.id, &manifest.version) {
        return Err(format!(
            "the plugin {} {} is revoked",
            manifest.id, manifest.version
        ));
    }

    let resolution = risk::resolve(&manifest, trust);
    let mut tools = Vec::new();
    for tool in &manifest.tools {
        if !manifest.owns_tool(&tool.name) {
            tracing::warn!(
                plugin = %manifest.id,
                tool = %tool.name,
                namespace = %manifest.namespace(),
                "the tool does not live in the namespace of the plugin and is dropped"
            );
            continue;
        }
        if jsonschema::validator_for(tool.input_schema.value()).is_err() {
            return Err(format!(
                "the input schema of the tool {} is not a valid JSON Schema",
                tool.name
            ));
        }
        tools.push(ToolSpec {
            name: tool.name.clone(),
            plugin_id: manifest.id.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.value().clone(),
            risk: resolution.effective_risk(&tool.name, tool.risk),
            reversible: tool.reversible,
            rollback_tool: tool.rollback_tool.clone(),
            always_confirm: tool.always_confirm,
            system_scope: tool.system_scope,
            writes_trusted_core: tool.writes_trusted_core,
            result_privacy: tool.result_privacy.unwrap_or(manifest.result_privacy),
            result_provenance: tool.result_provenance.unwrap_or(manifest.result_provenance),
            entry: manifest.entry.clone(),
            sandbox: manifest.sandbox.to_profile(),
            timeout_ms: manifest.timeout_ms,
        });
    }

    let info = PluginInfo {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        trust,
        path: manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        runtime: manifest.runtime,
        capabilities: manifest.capabilities.clone(),
        risk: resolution.inferred,
        tools: tools.iter().map(|tool| tool.name.clone()).collect(),
    };
    Ok(LoadedPlugin { info, tools })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_has_no_tools() {
        let registry = Registry::default();
        assert_eq!(registry.tool_count(), 0);
        assert!(registry.tool("fs.search").is_none());
    }
}
