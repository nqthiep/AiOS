//! aiosd-core — Tool Broker (MCP `aios-tools`) plus the admin Unix socket
//! (docs/m0/broker.md).
//!
//! Usage: `aiosd-core [--config /etc/aios/core.yaml] [--<key> <value>]...`

mod config;

use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;
use std::time::Duration;

use aios_audit::AuditLog;
use aios_broker::{
    Broker, BrokerConfig, BrokerDeps, CONFIRM_TIMEOUT, Executor, FakeExecutor, IDEMPOTENCY_TTL,
    SESSION_TTL, admin, mcp,
};
use aios_confirm::Verifier;
use aios_policy::PolicyEngine;
use aios_registry::{Registry, RegistryConfig};
use anyhow::Context;
use config::CoreConfig;
use tokio::net::{TcpListener, UnixListener};

/// How often idle sessions are swept (their gateway keys revoked).
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cfg = config::load(std::env::args().skip(1))?;
    run(cfg, shutdown_signal()).await
}

async fn shutdown_signal() {
    let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(s) => s,
        Err(_) => return std::future::pending().await,
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

async fn run(cfg: CoreConfig, shutdown: impl Future<Output = ()>) -> anyhow::Result<()> {
    let (registry, load_report) = Registry::load(&RegistryConfig {
        core_dir: cfg.registry_core_dir.clone(),
        vendor_dir: cfg.registry_vendor_dir.clone(),
        local_dir: cfg.registry_local_dir.clone(),
        trust_dir: cfg.trust_dir.clone(),
    });
    for (path, reason) in &load_report.rejected {
        tracing::warn!(path = %path.display(), %reason, "plugin rejected");
    }
    let policy = PolicyEngine::load(&cfg.policy_dir)
        .with_context(|| format!("loading policies from {}", cfg.policy_dir.display()))?;
    let verifier = Verifier::new(config::load_pubkey(&cfg.trusted_ui_pubkey)?);
    let audit = AuditLog::open(&cfg.audit_file)
        .with_context(|| format!("opening audit log {}", cfg.audit_file.display()))?;
    let autonomy = config::load_autonomy(&cfg.autonomy_file)?;

    // M0: replaced by the sandboxed executor of task `executor`. With no
    // configured outputs every call fails closed.
    let executor: Arc<dyn Executor> = Arc::new(FakeExecutor::new());

    let mcp_listener = TcpListener::bind(cfg.mcp_listen)
        .await
        .with_context(|| format!("binding {}", cfg.mcp_listen))?;
    let mcp_addr = mcp_listener.local_addr()?;
    // A leftover socket from a previous run would make bind fail.
    let _ = std::fs::remove_file(&cfg.admin_socket);
    let admin_listener = UnixListener::bind(&cfg.admin_socket)
        .with_context(|| format!("binding {}", cfg.admin_socket.display()))?;
    // Every local user may connect; the uid comes from SO_PEERCRED.
    std::fs::set_permissions(&cfg.admin_socket, std::fs::Permissions::from_mode(0o666))?;

    let broker = Broker::new(
        BrokerConfig {
            autonomy,
            admin_group: cfg.admin_group.clone(),
            gateway_admin_socket: cfg.gateway_admin_socket.clone(),
            gateway_client: cfg.gateway_client.clone(),
            broker_url: format!("http://{mcp_addr}{}", mcp::MCP_PATH),
            gateway_url: cfg.gateway_url.clone(),
            workspace_root: cfg.workspace_root.clone(),
            confirm_timeout: CONFIRM_TIMEOUT,
            session_ttl: SESSION_TTL,
            idempotency_ttl: IDEMPOTENCY_TTL,
        },
        BrokerDeps {
            registry,
            load_report,
            policy,
            verifier,
            audit,
            executor,
        },
    );
    tracing::info!(
        mcp = %mcp_addr,
        admin = %cfg.admin_socket.display(),
        tools = broker.registry().tool_count(),
        "aiosd-core started"
    );

    let sweeper = broker.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SWEEP_INTERVAL);
        loop {
            tick.tick().await;
            sweeper.sweep_expired().await;
        }
    });
    tokio::select! {
        r = mcp::serve(broker.clone(), mcp_listener) => r.context("MCP server stopped")?,
        r = admin::serve(broker, admin_listener) => r.context("admin socket stopped")?,
        () = shutdown => tracing::info!("shutting down"),
    }
    let _ = std::fs::remove_file(&cfg.admin_socket);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    /// Start with an empty registry and answer `GET /v1/status` on the admin socket.
    #[tokio::test]
    async fn starts_and_serves_status() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let key = root.join("ui.pub");
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&[3; 32]).verifying_key();
        std::fs::write(&key, pubkey.as_bytes()).unwrap();
        let cfg = CoreConfig {
            registry_core_dir: root.join("core"),
            registry_vendor_dir: root.join("vendor"),
            registry_local_dir: root.join("local"),
            trust_dir: root.join("trust"),
            policy_dir: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../policy"),
            audit_file: root.join("audit.jsonl"),
            trusted_ui_pubkey: key,
            autonomy_file: root.join("autonomy.yaml"),
            gateway_admin_socket: root.join("gw.sock"),
            mcp_listen: "127.0.0.1:0".parse().unwrap(),
            admin_socket: root.join("core.sock"),
            ..CoreConfig::default()
        };
        let sock = cfg.admin_socket.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(run(cfg, async {
            let _ = rx.await;
        }));

        let response = tokio::task::spawn_blocking(move || {
            for _ in 0..100 {
                if let Ok(mut s) = std::os::unix::net::UnixStream::connect(&sock) {
                    s.write_all(b"GET /v1/status HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
                        .unwrap();
                    let mut out = String::new();
                    s.read_to_string(&mut out).unwrap();
                    return out;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            panic!("admin socket never came up");
        })
        .await
        .unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("\"tools\":0"), "{response}");
        assert!(response.contains("policy_version"), "{response}");

        tx.send(()).unwrap();
        server.await.unwrap().unwrap();
    }
}
