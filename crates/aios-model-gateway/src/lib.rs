//! AiOS Model Gateway (M0).
//!
//! Spec: docs/m0/gateway.md.
//!
//! - Client API: `POST /v1/chat/completions`, `POST /v1/embeddings`, `GET /v1/models`,
//!   authenticated with a virtual key.
//! - Admin API on a Unix socket, checked with `SO_PEERCRED`.
//! - Local/cloud routing by session label and the virtual key's cloud ceiling.
//! - `secret` never goes to cloud (checked twice: configuration and code).
//! - Per-user cloud budget.

pub mod admin;
pub mod api;
pub mod budget;
pub mod config;
pub mod error;
pub mod routing;
pub mod session;
pub mod upstream;

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use tokio::net::{TcpListener, UnixListener};

use aios_types::Privacy;

use crate::budget::UsageStore;
use crate::config::{BudgetConfig, ClientConfig, GatewayConfig, UserConfig};
use crate::routing::CloudGate;
use crate::session::SessionRegistry;

#[cfg(not(unix))]
compile_error!("aios-model-gateway requires Unix (admin socket uses SO_PEERCRED)");

/// Shared gateway state.
#[derive(Debug)]
pub struct GatewayState {
    /// Main configuration.
    pub config: GatewayConfig,
    /// Directory holding each user's config and `usage.json`.
    pub users_dir: PathBuf,
    /// Virtual key and session registry.
    pub registry: SessionRegistry,
    /// Per-user cost ledger.
    pub usage: UsageStore,
    /// HTTP client for upstream calls.
    pub http: reqwest::Client,
    /// The only UID allowed to call the admin API.
    pub admin_uid: Option<u32>,
    /// Last UID that used each local model (to avoid sharing prompt cache).
    local_slots: Mutex<HashMap<String, u32>>,
    /// Local queue: at most one in-flight request per (local model, uid).
    local_gates: Mutex<HashMap<(String, u32), Arc<tokio::sync::Semaphore>>>,
}

impl GatewayState {
    /// Build the state from configuration.
    pub fn new(
        config: GatewayConfig,
        users_dir: PathBuf,
        admin_uid: Option<u32>,
    ) -> anyhow::Result<Self> {
        let timeout = Duration::from_secs(config.request_timeout_s.max(1));
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("cannot build HTTP client")?;
        Ok(Self {
            usage: UsageStore::new(users_dir.clone()),
            registry: SessionRegistry::new(),
            config,
            users_dir,
            http,
            admin_uid,
            local_slots: Mutex::new(HashMap::new()),
            local_gates: Mutex::new(HashMap::new()),
        })
    }

    /// A user's config; `None` when absent or unreadable (cloud disabled).
    pub fn user_config(&self, uid: u32) -> Option<UserConfig> {
        match config::load_user_config(&self.users_dir, uid) {
            Ok(config) => config,
            Err(error) => {
                tracing::error!(uid, error = %error, "cannot read user config");
                None
            }
        }
    }

    /// Client config by name.
    pub fn client_config(&self, client: &str) -> Option<&ClientConfig> {
        self.config.clients.get(client)
    }

    /// Cloud routing conditions for a user plus the resolved API key.
    pub fn cloud_access(&self, uid: u32, client: Option<&ClientConfig>) -> CloudAccess {
        let user = self.user_config(uid);
        let ceiling = routing::cloud_ceiling(client, user.as_ref());
        let provider = user
            .as_ref()
            .and_then(|u| u.cloud.as_ref())
            .and_then(|c| c.provider.clone());
        let api_key = user
            .as_ref()
            .and_then(|u| u.cloud.as_ref())
            .and_then(|c| c.api_key_ref.as_ref())
            .and_then(
                |reference| match config::resolve_api_key(reference, &self.users_dir, uid) {
                    Ok(key) => Some(key),
                    Err(error) => {
                        tracing::error!(uid, error = %error, "cannot resolve cloud api key");
                        None
                    }
                },
            );
        let budget = user
            .as_ref()
            .and_then(|u| u.cloud.as_ref())
            .and_then(|c| c.budget.as_ref());
        let spent_today = self.usage.spent_today(uid);
        let spent_month = self.usage.spent_this_month(uid);
        let budget_ok = budget_allows(budget, spent_today, spent_month);
        if let (Some(budget), Some(warn_at)) = (budget, budget.and_then(|b| b.warn_at))
            && warn_at > 0.0
        {
            let limit = budget.usd_per_day.unwrap_or(0.0);
            if limit > 0.0 && spent_today >= limit * warn_at {
                tracing::warn!(
                    uid,
                    spent_today,
                    limit,
                    "cloud budget warning threshold reached"
                );
            }
        }
        CloudAccess {
            gate: CloudGate {
                configured: ceiling.is_some() && budget_ok && api_key.is_some(),
                ceiling,
                provider,
            },
            api_key,
            spent_today,
            spent_month,
        }
    }

    /// `cache_prompt` for a local model: `true` only when the previous request on
    /// the same model came from the same user (design.md §18.4).
    fn cache_prompt_allowed(&self, model_name: &str, uid: u32) -> bool {
        let mut guard = self.local_slots.lock().unwrap_or_else(|e| e.into_inner());
        let previous = guard.get(model_name).copied();
        guard.insert(model_name.to_string(), uid);
        previous.is_none_or(|previous| previous == uid)
    }

    /// Per-user slot on a local model: at most one in-flight request per uid
    /// per local model (design.md §18.4).
    async fn acquire_local_slot(
        &self,
        model_name: &str,
        uid: u32,
    ) -> tokio::sync::OwnedSemaphorePermit {
        let gate = {
            let mut guard = self.local_gates.lock().unwrap_or_else(|e| e.into_inner());
            Arc::clone(
                guard
                    .entry((model_name.to_string(), uid))
                    .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1))),
            )
        };
        gate.acquire_owned().await.expect("local slot semaphore")
    }
}

/// Result of checking a user's cloud access.
#[derive(Debug, Clone)]
pub struct CloudAccess {
    /// Conditions used by routing.
    pub gate: CloudGate,
    /// Resolved cloud API key (never logged).
    pub api_key: Option<String>,
    /// Spent today (UTC).
    pub spent_today: f64,
    /// Spent this month (UTC).
    pub spent_month: f64,
}

/// Whether the budget still allows cloud use.
///
/// The default budget is 0, which disables cloud (design.md §18.2).
fn budget_allows(budget: Option<&BudgetConfig>, spent_today: f64, spent_month: f64) -> bool {
    let Some(budget) = budget else {
        return false;
    };
    let per_day = budget.usd_per_day.unwrap_or(0.0);
    let per_month = budget.usd_per_month.unwrap_or(0.0);
    if per_day <= 0.0 && per_month <= 0.0 {
        return false;
    }
    if per_day > 0.0 && spent_today >= per_day {
        return false;
    }
    if per_month > 0.0 && spent_month >= per_month {
        return false;
    }
    true
}

/// Gateway runtime options.
#[derive(Debug, Clone)]
pub struct GatewayOptions {
    /// Main configuration.
    pub config: GatewayConfig,
    /// Users directory.
    pub users_dir: PathBuf,
    /// Admin socket path; `None` disables the admin API.
    pub admin_socket: Option<PathBuf>,
    /// The only UID allowed to call the admin API.
    pub admin_uid: Option<u32>,
}

/// A bound gateway, ready to serve.
pub struct Gateway {
    state: Arc<GatewayState>,
    api: TcpListener,
    admin: Option<UnixListener>,
}

impl Gateway {
    /// Bind the API port and the admin socket.
    pub async fn bind(options: GatewayOptions) -> anyhow::Result<Self> {
        let state = Arc::new(GatewayState::new(
            options.config.clone(),
            options.users_dir,
            options.admin_uid,
        )?);
        let api = TcpListener::bind(&options.config.listen)
            .await
            .with_context(|| format!("cannot bind {}", options.config.listen))?;
        let admin = match &options.admin_socket {
            Some(path) => Some(
                admin::bind_admin_socket(path)
                    .with_context(|| format!("cannot bind admin socket {}", path.display()))?,
            ),
            None => None,
        };
        Ok(Self { state, api, admin })
    }

    /// Address the API listens on.
    pub fn api_addr(&self) -> std::io::Result<SocketAddr> {
        self.api.local_addr()
    }

    /// Shared state (used by tests).
    pub fn state(&self) -> Arc<GatewayState> {
        Arc::clone(&self.state)
    }

    /// Serve until `shutdown` completes.
    pub async fn serve_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<()> {
        let Self { state, api, admin } = self;
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let api_server = axum::serve(api, api::router(Arc::clone(&state)).into_make_service())
            .with_graceful_shutdown(wait_for_stop(stop_rx.clone()));
        let admin_server = async move {
            match admin {
                Some(listener) => {
                    admin::serve(Arc::clone(&state), listener, wait_for_stop(stop_rx)).await
                }
                None => std::future::pending::<anyhow::Result<()>>().await,
            }
        };
        tokio::select! {
            result = api_server => result.context("API server failed")?,
            result = admin_server => result.context("admin server failed")?,
            _ = shutdown => {
                let _ = stop_tx.send(true);
            }
        }
        Ok(())
    }
}

/// Wait for the stop signal.
async fn wait_for_stop(mut rx: tokio::sync::watch::Receiver<bool>) {
    while !*rx.borrow() {
        if rx.changed().await.is_err() {
            break;
        }
    }
}

/// Parse a client-declared privacy label (it may only raise the label).
pub fn parse_declared_privacy(value: &str) -> Result<Privacy, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => Ok(Privacy::Public),
        "personal" => Ok(Privacy::Personal),
        "secret" => Ok(Privacy::Secret),
        other => Err(format!(
            "invalid X-AIOS-Privacy value '{other}' (expected public|personal|secret)"
        )),
    }
}
