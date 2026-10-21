//! Admin API of `aiosd-core` on a Unix socket (docs/m0/broker.md §Session).
//! The caller's uid always comes from the socket peer credentials
//! (SO_PEERCRED), never from the request body (design.md §18.1).

use std::sync::Arc;

use aios_types::Channel;
use axum::extract::State;
use axum::extract::connect_info::{ConnectInfo, Connected};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::serve::IncomingStream;
use axum::{Json, Router};
use http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use tokio::net::UnixListener;

use crate::{Broker, SubmitError};

/// Uid of the peer process; `None` when the kernel did not provide it.
#[derive(Debug, Clone, Copy)]
pub struct PeerUid(pub Option<u32>);

impl Connected<IncomingStream<'_, UnixListener>> for PeerUid {
    fn connect_info(stream: IncomingStream<'_, UnixListener>) -> Self {
        Self(stream.io().peer_cred().ok().map(|c| c.uid()))
    }
}

fn error(status: StatusCode, code: &str, detail: impl ToString) -> Response {
    (
        status,
        Json(json!({"error": code, "detail": detail.to_string()})),
    )
        .into_response()
}

fn no_peer() -> Response {
    error(
        StatusCode::FORBIDDEN,
        "no_peer_credentials",
        "peer uid unavailable",
    )
}

#[derive(Deserialize)]
struct SessionRequest {
    channel: Channel,
}

#[derive(Deserialize)]
struct ConfirmRequest {
    request_id: String,
    token: String,
}

#[derive(Deserialize)]
struct DenyRequest {
    request_id: String,
}

async fn create_session(
    State(broker): State<Arc<Broker>>,
    ConnectInfo(info): ConnectInfo<PeerUid>,
    Json(body): Json<SessionRequest>,
) -> Response {
    let Some(uid) = info.0 else {
        return no_peer();
    };
    match broker.create_session(uid, body.channel).await {
        Ok(session) => Json(session).into_response(),
        Err(e) => error(StatusCode::BAD_GATEWAY, "session_failed", e),
    }
}

async fn pending(
    State(broker): State<Arc<Broker>>,
    ConnectInfo(info): ConnectInfo<PeerUid>,
) -> Response {
    match info.0 {
        Some(uid) => Json(broker.pending_for_uid(uid)).into_response(),
        None => no_peer(),
    }
}

fn submit_response(result: Result<(), SubmitError>) -> Response {
    match result {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        // Same answer for "unknown" and "someone else's": no request ids leak across uids.
        Err(SubmitError::NotFound) => {
            error(StatusCode::NOT_FOUND, "not_found", SubmitError::NotFound)
        }
        Err(SubmitError::Invalid(e)) => error(StatusCode::BAD_REQUEST, "confirmation_invalid", e),
    }
}

async fn confirm(
    State(broker): State<Arc<Broker>>,
    ConnectInfo(info): ConnectInfo<PeerUid>,
    Json(body): Json<ConfirmRequest>,
) -> Response {
    match info.0 {
        Some(uid) => submit_response(broker.submit_confirm(uid, &body.request_id, &body.token)),
        None => no_peer(),
    }
}

async fn deny(
    State(broker): State<Arc<Broker>>,
    ConnectInfo(info): ConnectInfo<PeerUid>,
    Json(body): Json<DenyRequest>,
) -> Response {
    match info.0 {
        Some(uid) => submit_response(broker.submit_deny(uid, &body.request_id)),
        None => no_peer(),
    }
}

async fn status(
    State(broker): State<Arc<Broker>>,
    ConnectInfo(info): ConnectInfo<PeerUid>,
) -> Response {
    match info.0 {
        Some(_) => Json(broker.status()).into_response(),
        None => no_peer(),
    }
}

pub fn router(broker: Arc<Broker>) -> Router {
    Router::new()
        .route("/v1/session", post(create_session))
        .route("/v1/pending", get(pending))
        .route("/v1/confirm", post(confirm))
        .route("/v1/deny", post(deny))
        .route("/v1/status", get(status))
        .with_state(broker)
}

pub async fn serve(broker: Arc<Broker>, listener: UnixListener) -> std::io::Result<()> {
    axum::serve(
        listener,
        router(broker).into_make_service_with_connect_info::<PeerUid>(),
    )
    .await
}
