//! Client of the Model Gateway admin API over its Unix socket
//! (docs/m0/gateway.md §Quản trị).

use std::path::PathBuf;
use std::time::Duration;

use aios_types::{Privacy, Taint};
use bytes::Bytes;
use http::{Method, Request, StatusCode, header};
use http_body_util::{BodyExt, Full, Limited};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::net::UnixStream;

/// Upper bound for one admin call; the gateway answers from memory.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound for an admin response body.
const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("gateway admin socket unreachable: {0}")]
    Io(#[from] std::io::Error),
    #[error("gateway admin HTTP error: {0}")]
    Http(String),
    #[error("gateway admin call timed out")]
    Timeout,
    #[error("gateway admin returned {0}")]
    Status(StatusCode),
    #[error("gateway admin returned an invalid body")]
    Body,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayAdmin {
    socket: PathBuf,
}

impl GatewayAdmin {
    pub(crate) fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// `POST /v1/sessions`: returns the virtual key of the session.
    pub(crate) async fn create_session(
        &self,
        uid: u32,
        session_id: &str,
        client: &str,
    ) -> Result<String, GatewayError> {
        let body = json!({"uid": uid, "session_id": session_id, "client": client});
        let resp = self.call(Method::POST, "/v1/sessions", Some(body)).await?;
        resp.get("key")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(GatewayError::Body)
    }

    /// `POST /v1/sessions/{id}/raise`.
    pub(crate) async fn raise(
        &self,
        session_id: &str,
        label: Privacy,
        taint: Taint,
    ) -> Result<(), GatewayError> {
        let path = format!("/v1/sessions/{session_id}/raise");
        let body = json!({"label": label, "taint": taint});
        self.call(Method::POST, &path, Some(body)).await.map(drop)
    }

    /// `DELETE /v1/sessions/{id}`.
    pub(crate) async fn revoke(&self, session_id: &str) -> Result<(), GatewayError> {
        let path = format!("/v1/sessions/{session_id}");
        self.call(Method::DELETE, &path, None).await.map(drop)
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, GatewayError> {
        tokio::time::timeout(CALL_TIMEOUT, self.call_inner(method, path, body))
            .await
            .map_err(|_| GatewayError::Timeout)?
    }

    async fn call_inner(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, GatewayError> {
        let http = |e: &dyn std::fmt::Display| GatewayError::Http(e.to_string());
        let stream = UnixStream::connect(&self.socket).await?;
        let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|e| http(&e))?;
        tokio::spawn(conn);
        let bytes = body.map(|b| b.to_string()).unwrap_or_default();
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "localhost")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(bytes)))
            .map_err(|e| http(&e))?;
        let resp = sender.send_request(req).await.map_err(|e| http(&e))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(GatewayError::Status(status));
        }
        let body = Limited::new(resp.into_body(), MAX_BODY)
            .collect()
            .await
            .map_err(|_| GatewayError::Body)?
            .to_bytes();
        if body.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&body).map_err(|_| GatewayError::Body)
    }
}
