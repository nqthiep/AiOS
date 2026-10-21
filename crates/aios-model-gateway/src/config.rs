//! Gateway configuration.
//!
//! - `/etc/aios/gateway.yaml`: main config (listen, models, routes, clients).
//! - `<users-dir>/<uid>/gateway.yaml`: per-user config (provider, cloud key,
//!   budget, privacy ceiling).
//!
//! See docs/m0/gateway.md, section "Cấu hình".

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use aios_types::Privacy;
use serde::{Deserialize, Serialize};

/// Upstream kind: `local` (llama-server) or `openai_compatible` (cloud).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    Local,
    OpenaiCompatible,
}

impl ModelKind {
    /// Whether this upstream is a cloud provider.
    pub fn is_cloud(self) -> bool {
        matches!(self, ModelKind::OpenaiCompatible)
    }
}

/// Virtual key scope (integration.md §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyScope {
    /// Per-session key: label comes from the session and only rises.
    Session,
    /// Per-service key: label is always `secret` (mixed-label data).
    Service,
    /// Per-plugin key: label comes from the session, like a session key.
    Plugin,
}

/// One upstream in `models`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub kind: ModelKind,
    /// llama-server base URL (local model), e.g. `http://127.0.0.1:8081/v1`.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Cloud provider base URL, e.g. `https://api.example.com/v1`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Model name sent upstream (if different from the logical name).
    #[serde(default)]
    pub model: Option<String>,
    /// Cloud provider name, matched against the user's `cloud.provider`.
    #[serde(default)]
    pub provider: Option<String>,
    /// Price per million input tokens (USD).
    #[serde(default)]
    pub price_per_mtok_in: Option<f64>,
    /// Price per million output tokens (USD).
    #[serde(default)]
    pub price_per_mtok_out: Option<f64>,
}

impl ModelConfig {
    /// Base URL used to call the upstream.
    pub fn base_url(&self) -> Option<&str> {
        match self.kind {
            ModelKind::Local => self.endpoint.as_deref(),
            ModelKind::OpenaiCompatible => self.base_url.as_deref(),
        }
    }
}

/// A route: logical model name -> upstreams, tried in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteConfig {
    /// Logical model name (`s1`, `s2`, `coder`).
    pub model: String,
    /// Names in `models`, tried in order.
    #[serde(rename = "use")]
    pub use_models: Vec<String>,
}

/// Per-client config (integration.md §3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    /// `false` = the client may never use cloud.
    #[serde(default)]
    pub cloud: Option<bool>,
    /// Highest privacy label allowed to go to cloud.
    #[serde(default)]
    pub max_cloud_privacy: Option<Privacy>,
    /// Scope of virtual keys issued to this client.
    #[serde(default)]
    pub key_scope: Option<KeyScope>,
}

impl ClientConfig {
    /// The client's cloud ceiling: `None` = no cloud (fail closed).
    pub fn cloud_ceiling(&self) -> Option<Privacy> {
        if self.cloud == Some(false) {
            return None;
        }
        self.max_cloud_privacy
    }

    /// Key scope, `session` by default.
    pub fn key_scope(&self) -> KeyScope {
        self.key_scope.unwrap_or(KeyScope::Session)
    }
}

/// Main gateway config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Client API listen address, e.g. `127.0.0.1:8090`.
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Upstreams by name.
    #[serde(default)]
    pub models: BTreeMap<String, ModelConfig>,
    /// Routes by logical model name.
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    /// Client configs by name.
    #[serde(default)]
    pub clients: BTreeMap<String, ClientConfig>,
    /// Name in `models` used for `/v1/embeddings` (must be local).
    #[serde(default)]
    pub embeddings_model: Option<String>,
    /// Per-request upstream timeout (seconds).
    #[serde(default = "default_request_timeout_s")]
    pub request_timeout_s: u64,
}

fn default_listen() -> String {
    "127.0.0.1:8090".to_string()
}

fn default_request_timeout_s() -> u64 {
    120
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            models: BTreeMap::new(),
            routes: Vec::new(),
            clients: BTreeMap::new(),
            embeddings_model: None,
            request_timeout_s: default_request_timeout_s(),
        }
    }
}

impl GatewayConfig {
    /// First route matching the logical model name.
    pub fn route_for(&self, logical_model: &str) -> Option<&RouteConfig> {
        self.routes.iter().find(|r| r.model == logical_model)
    }

    /// Logical model names that have a route (sorted, deduplicated).
    pub fn logical_models(&self) -> Vec<String> {
        let mut names: Vec<String> = self.routes.iter().map(|r| r.model.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Check the configuration for consistency.
    pub fn validate(&self) -> Result<(), ConfigError> {
        for route in &self.routes {
            if route.model.trim().is_empty() {
                return Err(ConfigError::Invalid("route without model name".into()));
            }
            for name in &route.use_models {
                let model = self.models.get(name).ok_or_else(|| {
                    ConfigError::Invalid(format!("route '{}': unknown model '{name}'", route.model))
                })?;
                if model.base_url().is_none_or(|u| u.trim().is_empty()) {
                    return Err(ConfigError::Invalid(format!(
                        "model '{name}' has no endpoint/base_url"
                    )));
                }
            }
        }
        if let Some(name) = &self.embeddings_model {
            match self.models.get(name) {
                None => {
                    return Err(ConfigError::Invalid(format!(
                        "embeddings_model '{name}' is not defined in models"
                    )));
                }
                Some(model) if model.kind.is_cloud() => {
                    return Err(ConfigError::Invalid(format!(
                        "embeddings_model '{name}' must be a local model"
                    )));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }
}

/// A user's cloud budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// USD per UTC day; `0` = no daily budget.
    #[serde(default)]
    pub usd_per_day: Option<f64>,
    /// USD per UTC month; `0` = no monthly budget.
    #[serde(default)]
    pub usd_per_month: Option<f64>,
    /// Fraction (0..1) at which the user is warned.
    #[serde(default)]
    pub warn_at: Option<f64>,
}

/// Cloud section of the user config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCloudConfig {
    /// Cloud provider chosen by the user.
    #[serde(default)]
    pub provider: Option<String>,
    /// API key reference. M0 supports only `file:<path>`.
    #[serde(default)]
    pub api_key_ref: Option<String>,
    /// Budget.
    #[serde(default)]
    pub budget: Option<BudgetConfig>,
    /// Highest privacy label allowed to go to cloud.
    #[serde(default)]
    pub max_cloud_privacy: Option<Privacy>,
}

/// Per-user config (`<users-dir>/<uid>/gateway.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserConfig {
    /// The user's UID (must match the directory).
    #[serde(default)]
    pub user: Option<u32>,
    /// Cloud config; absent = cloud disabled.
    #[serde(default)]
    pub cloud: Option<UserCloudConfig>,
}

/// Error while reading or parsing configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse YAML file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml_ng::Error,
    },
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// Load the main gateway config.
pub fn load_gateway_config(path: &Path) -> Result<GatewayConfig, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: GatewayConfig =
        serde_yaml_ng::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    config.validate()?;
    Ok(config)
}

/// Path of a user's config file.
pub fn user_config_path(users_dir: &Path, uid: u32) -> PathBuf {
    users_dir.join(uid.to_string()).join("gateway.yaml")
}

/// Load a user's config; `None` when the file is absent (cloud disabled).
pub fn load_user_config(users_dir: &Path, uid: u32) -> Result<Option<UserConfig>, ConfigError> {
    let path = user_config_path(users_dir, uid);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    let config: UserConfig =
        serde_yaml_ng::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.clone(),
            source,
        })?;
    if let Some(declared) = config.user
        && declared != uid
    {
        return Err(ConfigError::Invalid(format!(
            "user config {} declares uid {declared} but lives in the directory of uid {uid}",
            path.display()
        )));
    }
    Ok(Some(config))
}

/// Read the cloud API key from `api_key_ref`. M0 supports only `file:<path>`.
///
/// The key file must live inside the user's own directory (`<users-dir>/<uid>/`),
/// so one user's key is never used for another user's requests (design.md §18.2),
/// and must not be accessible by group or others (mode `0600`).
pub fn resolve_api_key(
    api_key_ref: &str,
    users_dir: &Path,
    uid: u32,
) -> Result<String, ConfigError> {
    let Some(path) = api_key_ref.strip_prefix("file:") else {
        return Err(ConfigError::Invalid(
            "unsupported api_key_ref scheme (M0 supports only file:<path>)".into(),
        ));
    };
    let read_error = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ConfigError::Read { path, source }
    };
    let path = Path::new(path);
    let real = std::fs::canonicalize(path).map_err(read_error(path))?;
    let user_dir = users_dir.join(uid.to_string());
    let user_dir = std::fs::canonicalize(&user_dir).map_err(read_error(&user_dir))?;
    if !real.starts_with(&user_dir) {
        return Err(ConfigError::Invalid(format!(
            "api key file {} is outside the directory of uid {uid}",
            real.display()
        )));
    }
    let mode = std::fs::metadata(&real)
        .map_err(read_error(&real))?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(ConfigError::Invalid(format!(
            "api key file {} must not be accessible by group or others (expected mode 0600)",
            real.display()
        )));
    }
    let raw = std::fs::read_to_string(&real).map_err(read_error(&real))?;
    let key = raw.trim().to_string();
    if key.is_empty() {
        return Err(ConfigError::Invalid(format!(
            "api key file {} is empty",
            real.display()
        )));
    }
    Ok(key)
}
