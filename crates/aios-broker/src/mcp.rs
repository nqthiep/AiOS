//! MCP endpoint `aios-tools` (streamable HTTP, rmcp) with Bearer
//! authentication (docs/m0/broker.md §MCP endpoint).

use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use http::{StatusCode, header};
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ErrorCode, Implementation,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use crate::{Broker, CallError};

/// MCP path under the configured listen address.
pub const MCP_PATH: &str = "/mcp";

/// Internal session key put into the request extensions by [`require_bearer`].
#[derive(Debug, Clone)]
struct SessionKey(String);

/// JSON-RPC error codes of the broker (server-defined range).
fn error_code(e: &CallError) -> ErrorCode {
    match e {
        CallError::UnknownTool(_) | CallError::InvalidParams(_) => ErrorCode::INVALID_PARAMS,
        CallError::Denied(_) => ErrorCode(-32010),
        CallError::ConfirmationInvalid(_) => ErrorCode(-32011),
        CallError::ConfirmationTimeout => ErrorCode(-32012),
        CallError::ConfirmationDenied => ErrorCode(-32013),
        CallError::LabelSyncFailed => ErrorCode(-32014),
        CallError::Execution(_) => ErrorCode(-32015),
        CallError::SessionExpired => ErrorCode(-32016),
        CallError::Internal(_) => ErrorCode::INTERNAL_ERROR,
    }
}

fn to_error(e: &CallError) -> ErrorData {
    ErrorData::new(
        error_code(e),
        format!("{}: {e}", e.code()),
        Some(json!({"error": e.code()})),
    )
}

fn invalid_params(msg: &str) -> ErrorData {
    to_error(&CallError::InvalidParams(msg.to_owned()))
}

#[derive(Clone)]
struct BrokerMcp {
    broker: Arc<Broker>,
}

impl rmcp::ServerHandler for BrokerMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("aios-tools", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        // system_scope tools are listed for everyone; calling them is the policy's job.
        let tools = self
            .broker
            .registry()
            .tools()
            .map(|t| {
                let schema = match &t.input_schema {
                    Value::Object(o) => o.clone(),
                    _ => Default::default(),
                };
                Tool::new(t.name.clone(), t.description.clone(), Arc::new(schema))
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let key = context
            .extensions
            .get::<http::request::Parts>()
            .and_then(|p| p.extensions.get::<SessionKey>())
            .map(|k| k.0.clone())
            .ok_or_else(|| ErrorData::invalid_request("unauthenticated", None))?;
        let idempotency_key = request
            .meta
            .as_ref()
            .and_then(|m| m.get("idempotency_key"))
            .or_else(|| context.meta.get("idempotency_key"))
            .ok_or_else(|| invalid_params("_meta.idempotency_key is required"))?
            .as_str()
            .ok_or_else(|| invalid_params("_meta.idempotency_key must be a string"))?
            .to_owned();
        let tool = request.name.to_string();
        let args = Value::Object(request.arguments.unwrap_or_default());

        // Run detached so a client disconnect cannot interrupt the call between
        // execution, label sync and audit; the result stays cached for replay.
        let broker = self.broker.clone();
        let result =
            tokio::spawn(async move { broker.call(&key, &tool, args, &idempotency_key).await })
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let output = result.map_err(|e| to_error(&e))?;
        // `resultType` is mandatory from protocol 2026-07-28 (rmcp strips it for
        // older peers); from_value would leave it unset.
        let result: CallToolResult = serde_json::from_value(json!({
            "resultType": "complete",
            "content": output.content,
            "isError": output.is_error,
            "_meta": {"privacy": output.privacy, "provenance": output.provenance},
        }))
        .map_err(|e| ErrorData::internal_error(format!("invalid tool output: {e}"), None))?;
        Ok(result.into())
    }
}

/// 401 unless `Authorization: Bearer <broker_token>` names a live session.
async fn require_bearer(
    State(broker): State<Arc<Broker>>,
    mut req: Request,
    next: Next,
) -> Response {
    let key = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|t| broker.authenticate(t.trim()));
    match key {
        Some(key) => {
            req.extensions_mut().insert(SessionKey(key));
            next.run(req).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
        )
            .into_response(),
    }
}

/// The MCP router: `POST /mcp` (stateless streamable HTTP, JSON responses).
pub fn router(broker: Arc<Broker>) -> Router {
    let handler_broker = broker.clone();
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let service = StreamableHttpService::new(
        move || {
            Ok(BrokerMcp {
                broker: handler_broker.clone(),
            })
        },
        Arc::new(NeverSessionManager::default()),
        config,
    );
    Router::new()
        .route_service(MCP_PATH, service)
        .layer(middleware::from_fn_with_state(broker, require_bearer))
}

pub async fn serve(broker: Arc<Broker>, listener: TcpListener) -> std::io::Result<()> {
    axum::serve(listener, router(broker)).await
}
