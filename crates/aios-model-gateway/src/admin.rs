//! Admin API on a Unix socket (docs/m0/gateway.md "Quản trị").
//!
//! Only the configured UID (`--admin-uid`) may call it; any other UID gets 403.
//! The UID comes from the socket's `SO_PEERCRED`, never from the request body.

use std::future::Future;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;

use aios_types::{Privacy, Taint};
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, post};
use axum::{Json, Router};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::{UnixListener, UnixStream};
use tower::ServiceExt;

use crate::GatewayState;
use crate::config::KeyScope;
use crate::error::GatewayError;

/// Peer UID from `SO_PEERCRED`.
#[derive(Debug, Clone, Copy)]
pub struct PeerUid(pub u32);

/// Admin API router.
pub fn router(state: Arc<GatewayState>) -> Router {
    let admin_uid = state.admin_uid;
    Router::new()
        .route("/v1/sessions", post(create_session))
        .route("/v1/sessions/{id}/raise", post(raise_session))
        .route("/v1/sessions/{id}", delete(revoke_session))
        .route("/v1/revoke-all", post(revoke_all))
        .with_state(state)
        .layer(middleware::from_fn_with_state(admin_uid, check_peer))
}

/// Only the configured UID may call the admin API (fail closed).
async fn check_peer(
    State(admin_uid): State<Option<u32>>,
    request: Request,
    next: Next,
) -> Result<Response, GatewayError> {
    let peer = request.extensions().get::<PeerUid>().copied();
    match (peer, admin_uid) {
        (Some(PeerUid(uid)), Some(expected)) if uid == expected => Ok(next.run(request).await),
        _ => Err(GatewayError::forbidden(
            "only the configured admin uid may call the admin API",
        )),
    }
}

/// Body of `POST /v1/sessions`.
#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    uid: u32,
    #[serde(default)]
    session_id: Option<String>,
    client: String,
}

/// Body of `POST /v1/sessions/{id}/raise`.
#[derive(Debug, Deserialize)]
struct RaiseRequest {
    #[serde(default)]
    label: Option<Privacy>,
    #[serde(default)]
    taint: Option<Taint>,
}

/// `POST /v1/sessions`: issue a virtual key.
async fn create_session(
    State(state): State<Arc<GatewayState>>,
    body: Bytes,
) -> Result<Json<Value>, GatewayError> {
    let request: CreateSessionRequest = serde_json::from_slice(&body)
        .map_err(|error| GatewayError::invalid_request(format!("invalid JSON body: {error}")))?;
    if request.client.trim().is_empty() {
        return Err(GatewayError::invalid_request("missing 'client' field"));
    }
    // The client config decides the key scope; an unknown client gets a session
    // key (and no cloud, since it has no cloud ceiling).
    let scope = state
        .client_config(&request.client)
        .map(|client| client.key_scope())
        .unwrap_or(KeyScope::Session);

    let (key, info, label, taint) = match scope {
        KeyScope::Service => {
            let (key, info) = state
                .registry
                .issue_service_key(request.uid, &request.client);
            (key, info, Privacy::Secret, Taint::Clean)
        }
        _ => {
            let session_id = request.session_id.clone().ok_or_else(|| {
                GatewayError::invalid_request("missing 'session_id' field for a session key")
            })?;
            let (key, info) = state
                .registry
                .issue_session_key(request.uid, &session_id, &request.client, scope)
                .ok_or_else(|| {
                    GatewayError::session_uid_mismatch(format!(
                        "session '{session_id}' belongs to another uid"
                    ))
                })?;
            let label = state
                .registry
                .session_label(&session_id)
                .unwrap_or(Privacy::Public);
            (key, info, label, Taint::Clean)
        }
    };

    tracing::info!(uid = request.uid, client = %request.client, "virtual key issued");
    Ok(Json(json!({
        "key": key,
        "uid": info.uid,
        "client": info.client,
        "session_id": info.session_id,
        "label": label,
        "taint": taint,
    })))
}

/// `POST /v1/sessions/{id}/raise`: raise label and taint (lowering is ignored).
async fn raise_session(
    State(state): State<Arc<GatewayState>>,
    UrlPath(session_id): UrlPath<String>,
    body: Bytes,
) -> Result<Json<Value>, GatewayError> {
    let request: RaiseRequest = serde_json::from_slice(&body)
        .map_err(|error| GatewayError::invalid_request(format!("invalid JSON body: {error}")))?;
    let label = request.label.unwrap_or(Privacy::Public);
    let taint = request.taint.unwrap_or(Taint::Clean);
    let Some((current_label, current_taint)) = state.registry.raise(&session_id, label, taint)
    else {
        return Err(GatewayError::session_not_found(format!(
            "unknown session '{session_id}'"
        )));
    };
    Ok(Json(json!({
        "session_id": session_id,
        "label": current_label,
        "taint": current_taint,
    })))
}

/// `DELETE /v1/sessions/{id}`: revoke the session's keys.
async fn revoke_session(
    State(state): State<Arc<GatewayState>>,
    UrlPath(session_id): UrlPath<String>,
) -> Result<Json<Value>, GatewayError> {
    let revoked = state.registry.revoke_session(&session_id);
    tracing::info!(session_id = %session_id, revoked, "session keys revoked");
    Ok(Json(json!({"session_id": session_id, "revoked": revoked})))
}

/// `POST /v1/revoke-all`: revoke every session key (kill switch).
async fn revoke_all(State(state): State<Arc<GatewayState>>) -> Result<Json<Value>, GatewayError> {
    let revoked = state.registry.revoke_all_session_keys();
    tracing::warn!(revoked, "all session keys revoked");
    Ok(Json(json!({"revoked": revoked})))
}

/// Bind the admin socket.
///
/// Mode `0666`: `aiosd-core` runs under its own UID and must be able to
/// connect; access control is the `SO_PEERCRED` check in [`check_peer`].
pub fn bind_admin_socket(path: &Path) -> std::io::Result<UnixListener> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Replace only a stale socket; never delete some other file at that path.
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => std::fs::remove_file(path)?,
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "admin socket path exists and is not a socket",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;
    Ok(listener)
}

/// Serve the admin API on a Unix socket.
pub async fn serve(
    state: Arc<GatewayState>,
    listener: UnixListener,
    stop: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let app = router(state);
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _ = &mut stop => return Ok(()),
            accepted = listener.accept() => {
                let (stream, _address) = accepted?;
                let uid = peer_uid(&stream);
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(error) = serve_connection(stream, uid, app).await {
                        tracing::warn!(error = %error, "admin connection failed");
                    }
                });
            }
        }
    }
}

/// Serve one admin connection.
async fn serve_connection(stream: UnixStream, uid: Option<u32>, app: Router) -> anyhow::Result<()> {
    let io = TokioIo::new(stream);
    let service = service_fn(move |mut request: hyper::Request<Incoming>| {
        let app = app.clone();
        async move {
            if let Some(uid) = uid {
                request.extensions_mut().insert(PeerUid(uid));
            }
            app.clone().oneshot(request).await
        }
    });
    hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .await?;
    Ok(())
}

/// Peer UID from `SO_PEERCRED`; `None` on error (fail closed).
fn peer_uid(stream: &UnixStream) -> Option<u32> {
    match stream.peer_cred() {
        Ok(cred) => Some(cred.uid()),
        Err(error) => {
            tracing::warn!(error = %error, "cannot read peer credentials");
            None
        }
    }
}
