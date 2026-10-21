//! Client API (OpenAI-compatible), docs/m0/gateway.md "API cho client".

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aios_types::Privacy;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::config::ModelKind;
use crate::error::GatewayError;
use crate::routing::{self, RequestContext};
use crate::session::KeyInfo;
use crate::upstream;
use crate::{CloudAccess, GatewayState};

/// Header with a client-declared privacy label (may only raise).
const HEADER_PRIVACY: &str = "x-aios-privacy";
/// Network probe timeout before using a cloud route.
const NETWORK_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Client API router.
pub fn router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/models", get(list_models))
        .with_state(state)
}

/// Authenticate with a virtual key.
fn authenticate(state: &GatewayState, headers: &HeaderMap) -> Result<KeyInfo, GatewayError> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| GatewayError::unauthorized("missing Authorization header"))?;
    let token = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .ok_or_else(|| {
            GatewayError::unauthorized("expected 'Authorization: Bearer <virtual key>'")
        })?
        .trim();
    if token.is_empty() {
        return Err(GatewayError::unauthorized("empty bearer token"));
    }
    state
        .registry
        .lookup(token)
        .ok_or_else(|| GatewayError::unauthorized("unknown or revoked key"))
}

/// Client-declared privacy label (may only raise, never lower).
fn declared_privacy(headers: &HeaderMap) -> Result<Option<Privacy>, GatewayError> {
    let Some(value) = headers.get(HEADER_PRIVACY) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| GatewayError::invalid_request("invalid X-AIOS-Privacy header"))?;
    let privacy = crate::parse_declared_privacy(value).map_err(GatewayError::invalid_request)?;
    Ok(Some(privacy))
}

/// Plan the route: probe each cloud provider, then select candidates.
async fn plan_route(
    state: &GatewayState,
    logical_model: &str,
    privacy: Privacy,
    access: &CloudAccess,
) -> Result<routing::RoutePlan, GatewayError> {
    let Some(route) = state.config.route_for(logical_model) else {
        return Err(GatewayError::model_not_found(format!(
            "unknown logical model '{logical_model}'"
        )));
    };
    let mut reachability: HashMap<String, bool> = HashMap::new();
    // Probe only when a cloud route could actually be used.
    if access.gate.configured && privacy != Privacy::Secret {
        for name in &route.use_models {
            let Some(model) = state.config.models.get(name) else {
                continue;
            };
            if !model.kind.is_cloud() {
                continue;
            }
            let Some(base_url) = model.base_url() else {
                continue;
            };
            if reachability.contains_key(base_url) {
                continue;
            }
            let online = upstream::network_online(base_url, NETWORK_PROBE_TIMEOUT).await;
            reachability.insert(base_url.to_string(), online);
        }
    }
    routing::select_candidates(
        &state.config,
        logical_model,
        privacy,
        &access.gate,
        |base_url| reachability.get(base_url).copied().unwrap_or(false),
    )
}

/// `POST /v1/chat/completions`.
async fn chat_completions(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, GatewayError> {
    let info = authenticate(&state, &headers)?;
    let request: Value = serde_json::from_slice(&body)
        .map_err(|error| GatewayError::invalid_request(format!("invalid JSON body: {error}")))?;

    if request.get("stream").and_then(Value::as_bool) == Some(true) {
        return Err(GatewayError::stream_not_supported());
    }

    let logical_model = request
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::invalid_request("missing 'model' field"))?
        .to_string();
    let declared = declared_privacy(&headers)?;
    let session_label = info
        .session_id
        .as_ref()
        .and_then(|session_id| state.registry.session_label(session_id));
    let privacy = routing::effective_privacy(info.scope, session_label, declared);

    let context = RequestContext {
        uid: info.uid,
        client: info.client.clone(),
        scope: info.scope,
        session_id: info.session_id.clone(),
    };
    let client_config = state.client_config(&context.client);
    let access = state.cloud_access(context.uid, client_config);
    let plan = plan_route(&state, &logical_model, privacy, &access).await?;

    let mut last_error: Option<String> = None;
    for candidate in &plan.candidates {
        // Last guard at the send site: `secret` never goes to cloud.
        if candidate.is_cloud && plan.privacy == Privacy::Secret {
            continue;
        }
        let started = Instant::now();
        let api_key = if candidate.is_cloud {
            access.api_key.as_deref()
        } else {
            None
        };
        // At most one in-flight request per user per local model.
        let _slot = if candidate.is_cloud {
            None
        } else {
            Some(state.acquire_local_slot(&candidate.name, context.uid).await)
        };
        let cache_prompt = if candidate.is_cloud {
            None
        } else {
            Some(state.cache_prompt_allowed(&candidate.name, context.uid))
        };
        match upstream::chat(
            &state.http,
            &candidate.model,
            &request,
            api_key,
            cache_prompt,
        )
        .await
        {
            Ok(reply) => {
                let usage = upstream::usage_of(&reply);
                let cost = upstream::cost_usd(&candidate.model, &usage);
                if candidate.is_cloud {
                    state.usage.record(
                        context.uid,
                        cost,
                        usage.prompt_tokens,
                        usage.completion_tokens,
                    );
                }
                tracing::info!(
                    uid = context.uid,
                    session_id = context.session_id.as_deref().unwrap_or("-"),
                    model = %logical_model,
                    route = %candidate.name,
                    kind = if candidate.is_cloud { "cloud" } else { "local" },
                    privacy = ?privacy,
                    prompt_tokens = usage.prompt_tokens,
                    completion_tokens = usage.completion_tokens,
                    usd = cost,
                    latency_ms = started.elapsed().as_millis() as u64,
                    "model request completed"
                );
                return Ok(Json(reply));
            }
            Err(error) => {
                tracing::warn!(
                    uid = context.uid,
                    model = %logical_model,
                    route = %candidate.name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    error = %error,
                    "upstream call failed, trying next candidate"
                );
                last_error = Some(error.to_string());
            }
        }
    }

    Err(GatewayError::upstream_unavailable(format!(
        "all candidates failed for model '{logical_model}'{}",
        last_error
            .map(|error| format!(": {error}"))
            .unwrap_or_default()
    )))
}

/// `POST /v1/embeddings`: always local (REQ-D-5.2-04).
async fn embeddings(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, GatewayError> {
    let info = authenticate(&state, &headers)?;
    let request: Value = serde_json::from_slice(&body)
        .map_err(|error| GatewayError::invalid_request(format!("invalid JSON body: {error}")))?;

    let requested = request.get("model").and_then(Value::as_str);
    let model_name = match requested {
        Some(name)
            if state
                .config
                .models
                .get(name)
                .is_some_and(|model| !model.kind.is_cloud()) =>
        {
            Some(name.to_string())
        }
        _ => state.config.embeddings_model.clone(),
    };
    let Some(model_name) = model_name else {
        return Err(GatewayError::upstream_unavailable(
            "no local embedding model configured",
        ));
    };
    let Some(model) = state.config.models.get(&model_name) else {
        return Err(GatewayError::model_not_found(format!(
            "unknown model '{model_name}'"
        )));
    };
    if model.kind == ModelKind::OpenaiCompatible {
        return Err(GatewayError::invalid_request(
            "embeddings must run on a local model",
        ));
    }

    let started = Instant::now();
    let _slot = state.acquire_local_slot(&model_name, info.uid).await;
    let reply = upstream::embeddings(&state.http, model, &request, None)
        .await
        .map_err(|error| {
            GatewayError::upstream_unavailable(format!("local embedding model failed: {error}"))
        })?;
    let usage = upstream::usage_of(&reply);
    tracing::info!(
        uid = info.uid,
        route = %model_name,
        prompt_tokens = usage.prompt_tokens,
        latency_ms = started.elapsed().as_millis() as u64,
        "embedding request completed"
    );
    Ok(Json(reply))
}

/// `GET /v1/models`: logical model names.
async fn list_models(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, GatewayError> {
    let _info = authenticate(&state, &headers)?;
    let data: Vec<Value> = state
        .config
        .logical_models()
        .into_iter()
        .map(|id| json!({"id": id, "object": "model", "owned_by": "aios"}))
        .collect();
    Ok(Json(json!({"object": "list", "data": data})))
}
