//! aios-broker — the Tool Broker, the only door to tools (docs/m0/broker.md;
//! design.md §9.4, §9.5, §5.2, §6.3, §18.1).
//!
//! One call: registry lookup → schema check → idempotency → Policy Engine →
//! (confirmation) → [`Executor`] → raise the session label/taint and sync it
//! to the Model Gateway → audit → result. The MCP endpoint is in [`mcp`], the
//! admin Unix socket in [`admin`].

pub mod admin;
pub mod executor;
mod gateway;
pub mod mcp;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aios_audit::{AuditKind, AuditLog, redact_secret};
use aios_confirm::{ConfirmError, PendingConfirm, Verifier, pending_for};
use aios_policy::{PolicyEngine, PolicyInput, effective_autonomy};
use aios_registry::{LoadReport, Registry};
use aios_types::{
    Channel, ConfirmLevel, Principal, Privacy, Provenance, SessionState, Taint, ToolOutput,
    ToolSpec, Verdict,
};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{OnceCell, oneshot};

pub use executor::{ExecCtx, ExecError, Executor, FakeExecutor, SandboxConfig, SandboxExecutor};
pub use gateway::GatewayError;

/// How long a confirmation request waits for a token (docs/m0/broker.md).
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(120);
/// Idle lifetime of a session.
pub const SESSION_TTL: Duration = Duration::from_secs(8 * 3600);
/// How long an idempotent result is kept.
pub const IDEMPOTENCY_TTL: Duration = Duration::from_secs(3600);
/// Bounds of `_meta.idempotency_key`.
pub const MAX_IDEMPOTENCY_KEY: usize = 128;

/// User autonomy (design.md §11.2), from `/etc/aios/autonomy.yaml` in M0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Autonomy {
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub per_tool_override: HashMap<String, u8>,
}

impl Autonomy {
    pub fn for_tool(&self, tool: &str) -> u8 {
        self.per_tool_override
            .get(tool)
            .copied()
            .unwrap_or(self.level)
    }
}

/// Runtime settings of the broker (all from `aiosd-core` configuration).
#[derive(Debug, Clone)]
pub struct BrokerConfig {
    pub autonomy: Autonomy,
    /// Members of this group are admins (`wheel` in production).
    pub admin_group: String,
    pub gateway_admin_socket: PathBuf,
    /// Gateway client name used for session keys (`aiosd-l2`).
    pub gateway_client: String,
    /// Returned to clients in the session response.
    pub broker_url: String,
    pub gateway_url: String,
    /// Workspace of uid N is `<workspace_root>/<N>`.
    pub workspace_root: PathBuf,
    pub confirm_timeout: Duration,
    pub session_ttl: Duration,
    pub idempotency_ttl: Duration,
}

/// Components the broker is built from.
pub struct BrokerDeps {
    pub registry: Registry,
    pub load_report: LoadReport,
    pub policy: PolicyEngine,
    pub verifier: Verifier,
    pub audit: AuditLog,
    pub executor: Arc<dyn Executor>,
}

/// Answer of `POST /v1/session`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub broker_token: String,
    pub gateway_key: String,
    pub broker_url: String,
    pub gateway_url: String,
}

/// Why a tool call failed. [`CallError::code`] is the stable name clients
/// match on (`denied`, `confirmation_*`, ...).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallError {
    #[error("unknown tool {0}")]
    UnknownTool(String),
    #[error("{0}")]
    InvalidParams(String),
    #[error("{0}")]
    Denied(String),
    #[error("{0}")]
    ConfirmationInvalid(String),
    #[error("no confirmation within the timeout")]
    ConfirmationTimeout,
    #[error("the user denied the request")]
    ConfirmationDenied,
    #[error("could not sync the session label with the model gateway")]
    LabelSyncFailed,
    #[error("{0}")]
    Execution(String),
    #[error("the session expired")]
    SessionExpired,
    #[error("{0}")]
    Internal(String),
}

impl CallError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownTool(_) => "unknown_tool",
            Self::InvalidParams(_) => "invalid_params",
            Self::Denied(_) => "denied",
            Self::ConfirmationInvalid(_) => "confirmation_invalid",
            Self::ConfirmationTimeout => "confirmation_timeout",
            Self::ConfirmationDenied => "confirmation_denied",
            Self::LabelSyncFailed => "label_sync_failed",
            Self::Execution(_) => "execution_failed",
            Self::SessionExpired => "session_expired",
            Self::Internal(_) => "internal",
        }
    }
}

/// Why `/v1/confirm` or `/v1/deny` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitError {
    /// No pending request with this id belongs to the caller's uid.
    #[error("no such pending request")]
    NotFound,
    /// The token did not verify; the call fails with `confirmation_invalid`.
    #[error("confirmation_invalid: {0}")]
    Invalid(ConfirmError),
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Gateway(#[from] GatewayError),
    #[error("cannot create the session: {0}")]
    Internal(String),
}

struct Session {
    principal: Principal,
    state: SessionState,
    /// Label and taint the gateway has acknowledged.
    synced: (Privacy, Taint),
    last_used: Instant,
}

enum Decision {
    Approved,
    Invalid(ConfirmError),
    Denied,
}

struct PendingEntry {
    pending: PendingConfirm,
    tx: oneshot::Sender<Decision>,
}

/// Result of an executed call; cached for idempotency.
type Executed = Result<ToolOutput, CallError>;

struct IdemEntry {
    tool: String,
    args: Value,
    created: Instant,
    result: OnceCell<Executed>,
}

pub struct Broker {
    config: BrokerConfig,
    registry: Registry,
    load_report: LoadReport,
    policy: PolicyEngine,
    verifier: Verifier,
    audit: AuditLog,
    executor: Arc<dyn Executor>,
    gateway: gateway::GatewayAdmin,
    /// sha256(broker_token) hex → session.
    sessions: Mutex<HashMap<String, Session>>,
    /// request_id → pending confirmation.
    pending: Mutex<HashMap<String, PendingEntry>>,
    /// (session_id, idempotency_key) → result.
    idempotency: Mutex<HashMap<(String, String), Arc<IdemEntry>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // Every map stays consistent between statements, so a poisoned lock is safe to reuse.
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn token_key(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `uid` is an admin when the group is its primary group or lists its name.
fn is_admin(uid: u32, group: &str) -> bool {
    use nix::unistd::{Group, Uid, User};
    let (Ok(Some(user)), Ok(Some(group))) =
        (User::from_uid(Uid::from_raw(uid)), Group::from_name(group))
    else {
        return false;
    };
    user.gid == group.gid || group.mem.contains(&user.name)
}

fn verdict_str(v: &Verdict) -> &'static str {
    match v {
        Verdict::Allow => "allow",
        Verdict::Confirm(_) => "confirm",
        Verdict::Deny(_) => "deny",
    }
}

fn level_str(level: ConfirmLevel) -> &'static str {
    match level {
        ConfirmLevel::Dialog => "dialog",
        ConfirmLevel::DialogWithPin => "dialog_with_pin",
    }
}

impl Broker {
    pub fn new(config: BrokerConfig, deps: BrokerDeps) -> Arc<Self> {
        Arc::new(Self {
            gateway: gateway::GatewayAdmin::new(config.gateway_admin_socket.clone()),
            config,
            registry: deps.registry,
            load_report: deps.load_report,
            policy: deps.policy,
            verifier: deps.verifier,
            audit: deps.audit,
            executor: deps.executor,
            sessions: Mutex::default(),
            pending: Mutex::default(),
            idempotency: Mutex::default(),
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// `GET /v1/status` body.
    pub fn status(&self) -> Value {
        let rejected: Vec<Value> = self
            .load_report
            .rejected
            .iter()
            .map(|(path, reason)| json!({"path": path.display().to_string(), "reason": reason}))
            .collect();
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "tools": self.registry.tool_count(),
            "registry": {"loaded": self.load_report.loaded, "rejected": rejected},
            "policy_version": self.policy.policy_version(),
        })
    }

    /// Create a session for `uid`. The uid MUST come from SO_PEERCRED or
    /// another kernel-provided credential, never from a request body.
    pub async fn create_session(
        &self,
        uid: u32,
        channel: Channel,
    ) -> Result<SessionInfo, SessionError> {
        self.sweep_expired().await;
        let session_id = uuid::Uuid::now_v7().to_string();
        let mut raw = [0u8; 32];
        getrandom::fill(&mut raw).map_err(|e| SessionError::Internal(e.to_string()))?;
        let broker_token = URL_SAFE_NO_PAD.encode(raw);
        let gateway_key = self
            .gateway
            .create_session(uid, &session_id, &self.config.gateway_client)
            .await?;
        let principal = Principal {
            uid,
            is_admin: is_admin(uid, &self.config.admin_group),
            channel,
            session_id: Some(session_id.clone()),
        };
        let audit =
            json!({"session": session_id, "channel": channel, "is_admin": principal.is_admin});
        lock(&self.sessions).insert(
            token_key(&broker_token),
            Session {
                principal,
                state: SessionState::new(session_id.clone(), uid),
                synced: (Privacy::Public, Taint::Clean),
                last_used: Instant::now(),
            },
        );
        if let Err(e) = self.audit.append(uid, AuditKind::SessionCreate, audit) {
            tracing::error!(error = %e, "audit append failed");
        }
        Ok(SessionInfo {
            session_id,
            broker_token,
            gateway_key,
            broker_url: self.config.broker_url.clone(),
            gateway_url: self.config.gateway_url.clone(),
        })
    }

    /// Resolve a Bearer token to the internal session key, refreshing its
    /// idle timer. `None` for unknown or expired tokens.
    pub fn authenticate(&self, token: &str) -> Option<String> {
        let key = token_key(token);
        let mut sessions = lock(&self.sessions);
        let session = sessions.get_mut(&key)?;
        if session.last_used.elapsed() >= self.config.session_ttl {
            let id = session.state.session_id.clone();
            sessions.remove(&key);
            self.revoke_in_background(id);
            return None;
        }
        session.last_used = Instant::now();
        Some(key)
    }

    /// Drop sessions idle for longer than the TTL and revoke their gateway keys.
    pub async fn sweep_expired(&self) {
        let ttl = self.config.session_ttl;
        let expired: Vec<String> = {
            let mut sessions = lock(&self.sessions);
            let ids = sessions
                .values()
                .filter(|s| s.last_used.elapsed() >= ttl)
                .map(|s| s.state.session_id.clone())
                .collect();
            sessions.retain(|_, s| s.last_used.elapsed() < ttl);
            ids
        };
        for id in expired {
            if let Err(e) = self.gateway.revoke(&id).await {
                tracing::warn!(session = %id, error = %e, "gateway revoke failed");
            }
        }
    }

    fn revoke_in_background(&self, session_id: String) {
        let gateway = self.gateway.clone();
        tokio::spawn(async move {
            if let Err(e) = gateway.revoke(&session_id).await {
                tracing::warn!(session = %session_id, error = %e, "gateway revoke failed");
            }
        });
    }

    fn snapshot(&self, key: &str) -> Result<(Principal, SessionState), CallError> {
        lock(&self.sessions)
            .get(key)
            .map(|s| (s.principal.clone(), s.state.clone()))
            .ok_or(CallError::SessionExpired)
    }

    /// The pending confirmations of `uid` (`GET /v1/pending`).
    pub fn pending_for_uid(&self, uid: u32) -> Vec<PendingConfirm> {
        lock(&self.pending)
            .values()
            .filter(|e| e.pending.uid == uid)
            .map(|e| e.pending.clone())
            .collect()
    }

    /// `POST /v1/confirm`: verify `token` against the broker's own copy of
    /// the request. A uid may only answer its own requests. Any verification
    /// failure fails the waiting call with `confirmation_invalid`.
    pub fn submit_confirm(
        &self,
        uid: u32,
        request_id: &str,
        token: &str,
    ) -> Result<(), SubmitError> {
        let entry = self.take_pending(uid, request_id)?;
        match self
            .verifier
            .verify_and_consume(token, &entry.pending, unix_now())
        {
            Ok(_) => {
                let _ = entry.tx.send(Decision::Approved);
                Ok(())
            }
            Err(e) => {
                let _ = entry.tx.send(Decision::Invalid(e));
                Err(SubmitError::Invalid(e))
            }
        }
    }

    /// `POST /v1/deny`.
    pub fn submit_deny(&self, uid: u32, request_id: &str) -> Result<(), SubmitError> {
        let entry = self.take_pending(uid, request_id)?;
        let _ = entry.tx.send(Decision::Denied);
        Ok(())
    }

    fn take_pending(&self, uid: u32, request_id: &str) -> Result<PendingEntry, SubmitError> {
        let mut pending = lock(&self.pending);
        match pending.get(request_id) {
            Some(e) if e.pending.uid == uid => {
                pending.remove(request_id).ok_or(SubmitError::NotFound)
            }
            _ => Err(SubmitError::NotFound),
        }
    }

    /// One `tools/call` for the session behind `key` (from [`Self::authenticate`]).
    pub async fn call(
        &self,
        key: &str,
        tool: &str,
        args: Value,
        idempotency_key: &str,
    ) -> Result<ToolOutput, CallError> {
        let spec = self
            .registry
            .tool(tool)
            .ok_or_else(|| CallError::UnknownTool(tool.to_owned()))?;
        if idempotency_key.is_empty() || idempotency_key.chars().count() > MAX_IDEMPOTENCY_KEY {
            return Err(CallError::InvalidParams(
                "_meta.idempotency_key must be a string of 1-128 characters".into(),
            ));
        }
        // One canonical copy is validated, shown, hashed and executed (design.md §7.7).
        let args = aios_confirm::normalize(&args);
        let validator = jsonschema::validator_for(&spec.input_schema)
            .map_err(|e| CallError::Internal(format!("invalid input schema: {e}")))?;
        if let Err(e) = validator.validate(&args) {
            return Err(CallError::InvalidParams(format!(
                "arguments do not match the schema: {e}"
            )));
        }

        let (principal, _) = self.snapshot(key)?;
        let session_id = principal.session_id.clone().unwrap_or_default();
        let entry = {
            let mut map = lock(&self.idempotency);
            // ponytail: O(n) sweep per call; a timer wheel if the cache grows large.
            let ttl = self.config.idempotency_ttl;
            map.retain(|_, e| e.created.elapsed() < ttl);
            map.entry((session_id.clone(), idempotency_key.to_owned()))
                .or_insert_with(|| {
                    Arc::new(IdemEntry {
                        tool: tool.to_owned(),
                        args: args.clone(),
                        created: Instant::now(),
                        result: OnceCell::new(),
                    })
                })
                .clone()
        };
        if entry.tool != tool || entry.args != args {
            return Err(CallError::InvalidParams(
                "_meta.idempotency_key was already used for a different call".into(),
            ));
        }

        let mut ran = false;
        let result = entry
            .result
            .get_or_try_init(|| {
                ran = true;
                self.run(key, spec, &args, idempotency_key)
            })
            .await?
            .clone();
        if !ran {
            self.audit_replay(&principal, key, spec, &args, idempotency_key, &result);
        }
        result
    }

    fn audit_replay(
        &self,
        principal: &Principal,
        key: &str,
        spec: &ToolSpec,
        args: &Value,
        idempotency_key: &str,
        result: &Executed,
    ) {
        let data = json!({
            "tool": spec.name,
            "args": args,
            "result": if matches!(result, Ok(o) if !o.is_error) { "ok" } else { "error" },
            "session": principal.session_id,
            "idempotency_key": idempotency_key,
            "replay": true,
        });
        // A failed replay audit only loses a duplicate line; the call was audited once.
        if let Err(e) = self.append_tool_call(principal.uid, key, data) {
            tracing::error!(error = %e, "audit append failed");
        }
    }

    /// Audit a `tool_call` line, redacting the args when the session label is secret.
    fn append_tool_call(&self, uid: u32, key: &str, mut data: Value) -> Result<(), CallError> {
        let label = lock(&self.sessions)
            .get(key)
            .map(|s| s.state.label)
            .unwrap_or(Privacy::Secret);
        if label == Privacy::Secret {
            redact_secret(&mut data, &["/args"]).map_err(|e| CallError::Internal(e.to_string()))?;
        }
        self.audit
            .append(uid, AuditKind::ToolCall, data)
            .map(drop)
            .map_err(|e| CallError::Internal(format!("audit failed: {e}")))
    }

    /// Policy → confirmation → execution → label sync → audit. The outer
    /// `Err` means nothing was executed (not cached); the inner result is the
    /// cached outcome of an execution.
    async fn run(
        &self,
        key: &str,
        spec: &ToolSpec,
        args: &Value,
        idempotency_key: &str,
    ) -> Result<Executed, CallError> {
        let (principal, state) = self.snapshot(key)?;
        let autonomy = self.config.autonomy.for_tool(&spec.name);
        let verdict = self.policy.evaluate(&PolicyInput {
            principal: &principal,
            tool: spec,
            user_autonomy: autonomy,
            tainted: state.taint == Taint::Tainted,
        });
        let mut audit = json!({
            "tool": spec.name,
            "args": args,
            "level": effective_autonomy(autonomy, principal.channel, spec.writes()),
            "verdict": verdict_str(&verdict),
            "reason": "auto",
            "result": "error",
            "approver": null,
            "session": state.session_id,
            "idempotency_key": idempotency_key,
        });

        let confirmed = match &verdict {
            Verdict::Deny(reason) => {
                audit["reason"] = json!(reason);
                self.append_tool_call(principal.uid, key, audit)?;
                return Err(CallError::Denied(reason.clone()));
            }
            Verdict::Confirm(level) => {
                self.confirm(&principal, spec, args, idempotency_key, *level)
                    .await
            }
            Verdict::Allow => Ok(false),
        };
        match confirmed {
            Ok(true) => {
                audit["reason"] = json!("confirmed");
                audit["approver"] = json!("trusted_ui");
            }
            Ok(false) => {}
            Err(e) => {
                audit["reason"] = json!(e.code());
                self.append_tool_call(principal.uid, key, audit)?;
                return Err(e);
            }
        }

        let ctx = ExecCtx {
            principal: principal.clone(),
            session_id: state.session_id.clone(),
            workspace: self.config.workspace_root.join(principal.uid.to_string()),
            timeout: Duration::from_millis(spec.timeout_ms),
        };
        let executed = match self.executor.run(spec, args, &ctx).await {
            Ok(mut out) => {
                // The broker never lets a result fall below the registry's labels.
                out.privacy = out.privacy.max(spec.result_privacy);
                if spec.result_provenance == Provenance::External {
                    out.provenance = Provenance::External;
                }
                let taint = if out.provenance == Provenance::External {
                    Taint::Tainted
                } else {
                    Taint::Clean
                };
                // Fail closed: no content leaves before the gateway knows the label (design §5.2).
                self.raise(key, out.privacy, taint).await.map(|()| out)
            }
            Err(e) => Err(CallError::Execution(e.to_string())),
        };
        if matches!(&executed, Ok(o) if !o.is_error) {
            audit["result"] = json!("ok");
        }
        if let Err(e) = &executed {
            audit["error"] = json!(e.code());
        }
        if let Err(e) = self.append_tool_call(principal.uid, key, audit) {
            tracing::error!(error = %e, "audit append failed after execution");
            return Ok(Err(e));
        }
        Ok(executed)
    }

    /// Open a confirmation, wait for the trusted UI, audit the outcome.
    /// `Ok(true)` when a valid token was consumed.
    async fn confirm(
        &self,
        principal: &Principal,
        spec: &ToolSpec,
        args: &Value,
        idempotency_key: &str,
        level: ConfirmLevel,
    ) -> Result<bool, CallError> {
        let pending = pending_for(
            principal,
            idempotency_key,
            vec![(spec.name.clone(), args.clone(), spec.risk)],
            &self.policy.policy_version(),
        )
        .map_err(|e| CallError::Internal(e.to_string()))?;
        let request_id = pending.request_id.clone();
        let (tx, mut rx) = oneshot::channel();
        lock(&self.pending).insert(request_id.clone(), PendingEntry { pending, tx });

        let decision = match tokio::time::timeout(self.config.confirm_timeout, &mut rx).await {
            Ok(d) => d.ok(),
            // Timed out; unless /v1/confirm took the entry meanwhile, the request is gone.
            Err(_) if lock(&self.pending).remove(&request_id).is_some() => None,
            Err(_) => rx.await.ok(),
        };
        let (outcome, result) = match decision {
            Some(Decision::Approved) => ("approved", Ok(true)),
            Some(Decision::Invalid(e)) => (
                "invalid",
                Err(CallError::ConfirmationInvalid(e.to_string())),
            ),
            Some(Decision::Denied) => ("denied", Err(CallError::ConfirmationDenied)),
            None => ("timeout", Err(CallError::ConfirmationTimeout)),
        };
        let data = json!({
            "request_id": request_id,
            "tool": spec.name,
            "level": level_str(level),
            "outcome": outcome,
            "error": result.as_ref().err().map(ToString::to_string),
            "session": principal.session_id,
            "idempotency_key": idempotency_key,
        });
        self.audit
            .append(principal.uid, AuditKind::Confirm, data)
            .map_err(|e| CallError::Internal(format!("audit failed: {e}")))?;
        result
    }

    /// Raise the session label/taint and make sure the gateway has it.
    /// Retries a previously failed sync on every call.
    async fn raise(&self, key: &str, label: Privacy, taint: Taint) -> Result<(), CallError> {
        let (session_id, target) = {
            let mut sessions = lock(&self.sessions);
            let s = sessions.get_mut(key).ok_or(CallError::LabelSyncFailed)?;
            s.state.raise(label, taint);
            let target = (s.state.label, s.state.taint);
            if target == s.synced {
                return Ok(());
            }
            (s.state.session_id.clone(), target)
        };
        if let Err(e) = self.gateway.raise(&session_id, target.0, target.1).await {
            tracing::error!(session = %session_id, error = %e, "label sync failed");
            return Err(CallError::LabelSyncFailed);
        }
        if let Some(s) = lock(&self.sessions).get_mut(key) {
            s.synced = (s.synced.0.max(target.0), s.synced.1.or(target.1));
        }
        Ok(())
    }
}
