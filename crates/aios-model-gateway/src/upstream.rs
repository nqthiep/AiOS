//! Upstream calls: llama-server (local) or an OpenAI-compatible provider (cloud).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::{ModelConfig, ModelKind};

/// Token counts from the upstream `usage`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
}

/// Upstream call error.
#[derive(Debug, thiserror::Error)]
pub enum UpstreamError {
    #[error("upstream request failed: {0}")]
    Transport(String),
    #[error("upstream returned HTTP {0}")]
    Status(u16),
    #[error("upstream returned a body that is not JSON")]
    InvalidBody,
}

/// Build a full URL from the upstream base URL.
fn endpoint(base_url: &str, path: &str) -> Result<String, UpstreamError> {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(UpstreamError::Transport("empty upstream URL".to_string()));
    }
    Ok(format!("{trimmed}/{path}"))
}

/// Prepare the upstream body: replace the logical model name with the upstream one.
fn prepare_body(
    body: &serde_json::Value,
    model: &ModelConfig,
    cache_prompt: Option<bool>,
) -> serde_json::Value {
    let mut body = body.clone();
    if let Some(object) = body.as_object_mut() {
        if let Some(upstream_model) = &model.model {
            object.insert(
                "model".to_string(),
                serde_json::Value::String(upstream_model.clone()),
            );
        }
        if let Some(cache_prompt) = cache_prompt {
            object.insert(
                "cache_prompt".to_string(),
                serde_json::Value::Bool(cache_prompt),
            );
        }
    }
    body
}

/// Send the request and return the upstream JSON body.
async fn send(
    http: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    api_key: Option<&str>,
) -> Result<serde_json::Value, UpstreamError> {
    let mut request = http.post(url).json(body);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|error| UpstreamError::Transport(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(UpstreamError::Status(status.as_u16()));
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| UpstreamError::InvalidBody)
}

/// Call `/chat/completions`.
///
/// `cache_prompt` applies to local models only (llama-server), so prompt cache
/// is not shared between users (design.md §18.4).
pub async fn chat(
    http: &reqwest::Client,
    model: &ModelConfig,
    body: &serde_json::Value,
    api_key: Option<&str>,
    cache_prompt: Option<bool>,
) -> Result<serde_json::Value, UpstreamError> {
    let base_url = model
        .base_url()
        .ok_or_else(|| UpstreamError::Transport("model has no endpoint".to_string()))?;
    let url = endpoint(base_url, "chat/completions")?;
    let cache_prompt = match model.kind {
        ModelKind::Local => cache_prompt,
        ModelKind::OpenaiCompatible => None,
    };
    let mut body = prepare_body(body, model, cache_prompt);
    if let Some(object) = body.as_object_mut() {
        object.insert("stream".to_string(), serde_json::Value::Bool(false));
    }
    send(http, &url, &body, api_key).await
}

/// Call `/embeddings`.
pub async fn embeddings(
    http: &reqwest::Client,
    model: &ModelConfig,
    body: &serde_json::Value,
    api_key: Option<&str>,
) -> Result<serde_json::Value, UpstreamError> {
    let base_url = model
        .base_url()
        .ok_or_else(|| UpstreamError::Transport("model has no endpoint".to_string()))?;
    let url = endpoint(base_url, "embeddings")?;
    let body = prepare_body(body, model, None);
    send(http, &url, &body, api_key).await
}

/// Extract `usage` from an upstream body.
pub fn usage_of(body: &serde_json::Value) -> Usage {
    body.get("usage")
        .and_then(|usage| serde_json::from_value::<Usage>(usage.clone()).ok())
        .unwrap_or_default()
}

/// Cost (USD) of a request, from the price table in `gateway.yaml`.
pub fn cost_usd(model: &ModelConfig, usage: &Usage) -> f64 {
    let price_in = model.price_per_mtok_in.unwrap_or(0.0);
    let price_out = model.price_per_mtok_out.unwrap_or(0.0);
    (usage.prompt_tokens as f64 / 1_000_000.0) * price_in
        + (usage.completion_tokens as f64 / 1_000_000.0) * price_out
}

/// Whether the cloud provider's host is reachable.
///
/// Used to drop cloud routes when offline (design.md §5.3 `offline_mode: auto`).
pub async fn network_online(base_url: &str, timeout: Duration) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let port = url.port_or_known_default().unwrap_or(443);
    matches!(
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect((host, port))).await,
        Ok(Ok(_))
    )
}
