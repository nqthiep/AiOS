//! The trust store: trusted public keys and the revocation list.
//!
//! Layout of the trust directory (docs/m0/README.md §4, docs/m0/registry.md):
//!
//! ```text
//! <trust_dir>/
//! ├── core/*.pub        # public keys of `core` plugins
//! ├── vendors/*.pub     # public keys of `vendor` plugins
//! ├── local.pub         # public key of `community` plugins
//! └── revoked.json      # revoked keys and revoked plugins
//! ```
//!
//! Every `*.pub` file holds the standard base64 encoding of the 32 raw public
//! key bytes. `revoked.json` has the shape
//! `{"keys": ["<base64 pubkey>"], "plugins": [{"id": "...", "version": "..."}]}`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::VerifyingKey;
use serde::Deserialize;

use crate::signing::{PUBLIC_KEY_LENGTH, PublicKey};

/// Trust level of a plugin.
///
/// The level is assigned from the directory the plugin is loaded from; the
/// `trust` field of a manifest is ignored (design.md §15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrustLevel {
    /// Loaded from the core directory of the system image.
    Core,
    /// Loaded from the vendor directory.
    Vendor,
    /// Loaded from the local (community) directory.
    Community,
}

impl TrustLevel {
    /// The name of the level as used in manifests and reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Vendor => "vendor",
            Self::Community => "community",
        }
    }

    /// The directory inside the trust directory that holds the public keys of
    /// this level. The community level uses a single file instead.
    fn key_dir(self) -> Option<&'static str> {
        match self {
            Self::Core => Some("core"),
            Self::Vendor => Some("vendors"),
            Self::Community => None,
        }
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A public key the registry trusts.
#[derive(Debug, Clone)]
pub struct TrustedKey {
    /// The file the key was read from.
    pub source: PathBuf,
    /// The raw public key.
    pub key: PublicKey,
}

impl TrustedKey {
    /// The ed25519 verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
}

/// A plugin that must not be loaded any more.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokedPlugin {
    /// The plugin id.
    pub id: String,
    /// The revoked version. When missing, every version of the id is revoked.
    #[serde(default)]
    pub version: Option<String>,
}

/// The revocation list (`revoked.json`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokedList {
    /// Base64 encoded public keys that must not be used any more.
    #[serde(default)]
    pub keys: Vec<String>,
    /// Plugins that must not be loaded any more.
    #[serde(default)]
    pub plugins: Vec<RevokedPlugin>,
}

/// Errors of the trust store.
#[derive(Debug, thiserror::Error)]
pub enum TrustStoreError {
    /// A file of the trust store could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `revoked.json` is not valid JSON or has an unexpected shape.
    #[error("invalid revocation list {path}: {source}")]
    Revocations {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// The trust store: trusted public keys per level plus the revocation list.
#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    keys: BTreeMap<TrustLevel, Vec<TrustedKey>>,
    revoked_keys: BTreeSet<[u8; PUBLIC_KEY_LENGTH]>,
    revoked_plugins: Vec<RevokedPlugin>,
}

impl TrustStore {
    /// An empty trust store: no key is trusted.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Loads the trust store from `trust_dir`.
    ///
    /// A missing directory or a missing `revoked.json` is not an error: it
    /// simply means that no key is trusted and that nothing is revoked. A
    /// `revoked.json` that exists but cannot be parsed **is** an error: the
    /// registry then trusts nothing, because an unreadable revocation list
    /// must not silently disable revocations.
    pub fn load(trust_dir: &Path) -> Result<Self, TrustStoreError> {
        let mut store = Self::default();

        let revocations_path = trust_dir.join("revoked.json");
        if revocations_path.is_file() {
            let text =
                fs::read_to_string(&revocations_path).map_err(|source| TrustStoreError::Io {
                    path: revocations_path.clone(),
                    source,
                })?;
            let list: RevokedList =
                serde_json::from_str(&text).map_err(|source| TrustStoreError::Revocations {
                    path: revocations_path.clone(),
                    source,
                })?;
            for key in &list.keys {
                match PublicKey::from_base64(key) {
                    Ok(key) => {
                        let mut bytes = [0u8; PUBLIC_KEY_LENGTH];
                        bytes.copy_from_slice(key.as_bytes());
                        store.revoked_keys.insert(bytes);
                    }
                    Err(err) => {
                        tracing::warn!(
                            path = %revocations_path.display(),
                            error = %err,
                            "ignoring a revoked key that is not a valid base64 encoded ed25519 key"
                        );
                    }
                }
            }
            store.revoked_plugins = list.plugins;
        }

        for level in [TrustLevel::Core, TrustLevel::Vendor] {
            let dir = trust_dir.join(level.key_dir().unwrap_or_default());
            store.load_key_dir(level, &dir);
        }

        // The community level uses a single key file; a directory of keys is
        // accepted as well so that a trust store can hold several local keys.
        let local_key = trust_dir.join("local.pub");
        if local_key.is_file() {
            store.load_key_file(TrustLevel::Community, &local_key);
        }
        store.load_key_dir(TrustLevel::Community, &trust_dir.join("local"));

        Ok(store)
    }

    /// The trusted keys of a trust level.
    pub fn keys(&self, level: TrustLevel) -> &[TrustedKey] {
        self.keys.get(&level).map_or(&[], Vec::as_slice)
    }

    /// Whether a public key was revoked.
    pub fn is_revoked_key(&self, key: &TrustedKey) -> bool {
        self.revoked_keys.contains(key.key.as_bytes())
    }

    /// Whether a plugin id (and version) was revoked.
    pub fn is_revoked_plugin(&self, id: &str, version: &str) -> bool {
        self.revoked_plugins.iter().any(|revoked| {
            revoked.id == id
                && revoked
                    .version
                    .as_ref()
                    .is_none_or(|revoked_version| revoked_version == version)
        })
    }

    /// The revoked plugins of the revocation list.
    pub fn revoked_plugins(&self) -> &[RevokedPlugin] {
        &self.revoked_plugins
    }

    fn load_key_dir(&mut self, level: TrustLevel, dir: &Path) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
            Err(err) => {
                tracing::warn!(path = %dir.display(), error = %err, "cannot read the trust directory");
                return;
            }
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "pub"))
            .collect();
        paths.sort();
        for path in paths {
            self.load_key_file(level, &path);
        }
    }

    fn load_key_file(&mut self, level: TrustLevel, path: &Path) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "cannot read a trusted public key");
                return;
            }
        };
        match PublicKey::from_base64(&text) {
            Ok(key) => {
                self.keys.entry(level).or_default().push(TrustedKey {
                    source: path.to_path_buf(),
                    key,
                });
            }
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "ignoring an invalid trusted public key");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_levels_are_ordered() {
        assert!(TrustLevel::Core < TrustLevel::Vendor);
        assert!(TrustLevel::Vendor < TrustLevel::Community);
    }

    #[test]
    fn revoked_plugin_matches_version() {
        let store = TrustStore {
            revoked_plugins: vec![RevokedPlugin {
                id: "aios.fs".into(),
                version: Some("1.0.0".into()),
            }],
            ..TrustStore::default()
        };
        assert!(store.is_revoked_plugin("aios.fs", "1.0.0"));
        assert!(!store.is_revoked_plugin("aios.fs", "1.0.1"));
        assert!(!store.is_revoked_plugin("aios.sys", "1.0.0"));
    }
}
