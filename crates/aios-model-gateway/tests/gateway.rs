//! Model Gateway tests with fake upstreams (docs/m0/gateway.md, test table).
//!
//! Mapping of the 9 rows of the test table in docs/m0/gateway.md:
//!
//! | Row in docs | Test here |
//! | --- | --- |
//! | `public` session, user has cloud -> cloud | `public_session_goes_to_cloud` |
//! | after `raise` to `secret` -> local, cloud gets nothing | `raised_secret_session_goes_local` |
//! | misconfigured route allows `secret` to cloud -> code still blocks | `secret_never_goes_to_cloud_even_if_config_allows` |
//! | client declares `public` on a `personal` session -> still `personal` | `client_cannot_lower_session_label` |
//! | key without session (service key, `cloud: false`) -> local only | `service_key_without_session_is_local_only` |
//! | budget exhausted -> local, or 409 without local | `exhausted_budget_falls_back_to_local` |
//! | users A and B -> cost in each user's own file | `each_user_has_own_usage_file` |
//! | `revoke-all` -> every session key 401 | `revoke_all_invalidates_session_keys` |
//! | `/v1/embeddings` on a `public` session -> still local | `embeddings_always_run_local` |

use std::collections::HashMap;
use std::net::SocketAddr;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aios_model_gateway::budget::{UsageEntry, UsageFile, utc_today};
use aios_model_gateway::config;
use aios_model_gateway::{Gateway, GatewayOptions};
use axum::Json;
use axum::Router;
use axum::extract::{Request, State};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::net::TcpListener;

/// A request received by a fake upstream.
#[derive(Debug, Clone)]
struct RecordedRequest {
    path: String,
    headers: HashMap<String, String>,
    body: Value,
}

#[derive(Clone)]
struct FakeState {
    name: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

/// Fake upstream: records every request and answers OpenAI-style.
struct FakeUpstream {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl FakeUpstream {
    async fn start(name: &str) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = FakeState {
            name: name.to_string(),
            requests: Arc::clone(&requests),
        };
        let app = Router::new().fallback(any(fake_handler)).with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fake upstream");
        let address = listener.local_addr().expect("fake upstream addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app.into_make_service()).await;
        });
        Self { address, requests }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("fake upstream lock").clone()
    }

    fn count(&self) -> usize {
        self.requests.lock().expect("fake upstream lock").len()
    }
}

async fn fake_handler(State(state): State<FakeState>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let body = axum::body::to_bytes(request.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    state
        .requests
        .lock()
        .expect("fake upstream lock")
        .push(RecordedRequest {
            path: path.clone(),
            headers,
            body: body.clone(),
        });

    let payload = if path.ends_with("/embeddings") {
        json!({
            "object": "list",
            "data": [{"object": "embedding", "index": 0, "embedding": [0.1, 0.2]}],
            "model": "fake-embedding",
            "usage": {"prompt_tokens": 1000, "total_tokens": 1000}
        })
    } else {
        json!({
            "id": "chatcmpl-fake",
            "object": "chat.completion",
            "created": 0,
            "model": state.name,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1000, "completion_tokens": 2000, "total_tokens": 3000}
        })
    };
    Json(payload).into_response()
}

/// Test harness: gateway plus fake upstreams.
struct Harness {
    _dir: TempDir,
    api: SocketAddr,
    admin_socket: PathBuf,
    users_dir: PathBuf,
    local: FakeUpstream,
    embed: FakeUpstream,
    cloud: FakeUpstream,
}

impl Harness {
    async fn start() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let local = FakeUpstream::start("fake-local").await;
        let embed = FakeUpstream::start("fake-embed").await;
        let cloud = FakeUpstream::start("fake-cloud").await;

        let users_dir = dir.path().join("users");
        std::fs::create_dir_all(&users_dir).expect("create users dir");
        let config_path = dir.path().join("gateway.yaml");
        std::fs::write(
            &config_path,
            gateway_yaml(local.address, embed.address, cloud.address),
        )
        .expect("write gateway.yaml");
        let config = config::load_gateway_config(&config_path).expect("load gateway.yaml");

        let admin_socket = dir.path().join("gateway-admin.sock");
        // UID of the test process = owner of the temp dir.
        let admin_uid = std::fs::metadata(dir.path()).expect("stat tempdir").uid();

        let gateway = Gateway::bind(GatewayOptions {
            config,
            users_dir: users_dir.clone(),
            admin_socket: Some(admin_socket.clone()),
            admin_uid: Some(admin_uid),
        })
        .await
        .expect("bind gateway");
        let api = gateway.api_addr().expect("gateway addr");
        tokio::spawn(async move {
            let _ = gateway.serve_until(std::future::pending()).await;
        });

        Self {
            _dir: dir,
            api,
            admin_socket,
            users_dir,
            local,
            embed,
            cloud,
        }
    }

    /// Write a user config (with cloud key and budget).
    fn write_user(&self, uid: u32, max_cloud_privacy: &str, usd_per_day: f64) {
        let dir = self.users_dir.join(uid.to_string());
        std::fs::create_dir_all(&dir).expect("create user dir");
        let key_path = dir.join("cloud-key");
        std::fs::write(&key_path, "sk-test-key\n").expect("write cloud key");
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .expect("chmod cloud key");
        let config = format!(
            "user: {uid}\n\
             cloud:\n\
             \x20 provider: openai\n\
             \x20 api_key_ref: file:{}\n\
             \x20 budget:\n\
             \x20   usd_per_day: {usd_per_day}\n\
             \x20   usd_per_month: 30.0\n\
             \x20   warn_at: 0.8\n\
             \x20 max_cloud_privacy: {max_cloud_privacy}\n",
            key_path.display()
        );
        std::fs::write(dir.join("gateway.yaml"), config).expect("write user config");
    }

    /// Write `usage.json` (simulated spend).
    fn write_usage(&self, uid: u32, usd: f64) {
        let dir = self.users_dir.join(uid.to_string());
        std::fs::create_dir_all(&dir).expect("create user dir");
        let mut usage = UsageFile::default();
        usage.days.insert(
            utc_today(),
            UsageEntry {
                usd,
                requests: 1,
                prompt_tokens: 0,
                completion_tokens: 0,
            },
        );
        std::fs::write(
            dir.join("usage.json"),
            serde_json::to_string_pretty(&usage).expect("serialize usage"),
        )
        .expect("write usage.json");
    }

    fn usage(&self, uid: u32) -> UsageFile {
        let path = self.users_dir.join(uid.to_string()).join("usage.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("read usage.json"))
            .expect("parse usage.json")
    }

    async fn admin_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> (reqwest::StatusCode, Value) {
        let client = reqwest::Client::builder()
            .unix_socket(self.admin_socket.as_path())
            .build()
            .expect("admin client");
        let mut request = client.request(method, format!("http://aios-admin{path}"));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.expect("admin request");
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, value)
    }

    /// Issue a session virtual key.
    async fn issue_key(&self, uid: u32, session_id: &str, client: &str) -> String {
        let (status, body) = self
            .admin_request(
                reqwest::Method::POST,
                "/v1/sessions",
                Some(json!({"uid": uid, "session_id": session_id, "client": client})),
            )
            .await;
        assert_eq!(status.as_u16(), 200, "issue key failed: {body}");
        body["key"].as_str().expect("key in body").to_string()
    }

    /// Issue a service key (no session).
    async fn issue_service_key(&self, uid: u32, client: &str) -> String {
        let (status, body) = self
            .admin_request(
                reqwest::Method::POST,
                "/v1/sessions",
                Some(json!({"uid": uid, "client": client})),
            )
            .await;
        assert_eq!(status.as_u16(), 200, "issue service key failed: {body}");
        body["key"].as_str().expect("key in body").to_string()
    }

    async fn raise(&self, session_id: &str, label: &str) -> (reqwest::StatusCode, Value) {
        self.admin_request(
            reqwest::Method::POST,
            &format!("/v1/sessions/{session_id}/raise"),
            Some(json!({"label": label})),
        )
        .await
    }

    async fn chat(&self, key: &str, model: &str) -> (reqwest::StatusCode, Value) {
        self.chat_with_privacy(key, model, None).await
    }

    async fn chat_with_privacy(
        &self,
        key: &str,
        model: &str,
        privacy: Option<&str>,
    ) -> (reqwest::StatusCode, Value) {
        let client = reqwest::Client::new();
        let mut request = client
            .post(format!("http://{}/v1/chat/completions", self.api))
            .bearer_auth(key)
            .json(&json!({"model": model, "messages": [{"role": "user", "content": "xin chào"}]}));
        if let Some(privacy) = privacy {
            request = request.header("X-AIOS-Privacy", privacy);
        }
        let response = request.send().await.expect("chat request");
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, value)
    }

    async fn embeddings(&self, key: &str, model: &str) -> (reqwest::StatusCode, Value) {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/v1/embeddings", self.api))
            .bearer_auth(key)
            .json(&json!({"model": model, "input": "xin chào"}))
            .send()
            .await
            .expect("embeddings request");
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, value)
    }

    async fn models(&self, key: &str) -> (reqwest::StatusCode, Value) {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/v1/models", self.api))
            .bearer_auth(key)
            .send()
            .await
            .expect("models request");
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, value)
    }
}

/// Gateway config for tests.
fn gateway_yaml(local: SocketAddr, embed: SocketAddr, cloud: SocketAddr) -> String {
    format!(
        r#"listen: "127.0.0.1:0"

models:
  local-s1:
    kind: local
    endpoint: "http://{local}/v1"
    model: "local-s1-model"
  local-s2:
    kind: local
    endpoint: "http://{local}/v1"
    model: "local-s2-model"
  embed-local:
    kind: local
    endpoint: "http://{embed}/v1"
    model: "embed-model"
  cloud-s2:
    kind: openai_compatible
    base_url: "http://{cloud}/v1"
    model: "cloud-s2-model"
    provider: openai
    price_per_mtok_in: 1.0
    price_per_mtok_out: 2.0
  cloud-coder:
    kind: openai_compatible
    base_url: "http://{cloud}/v1"
    model: "cloud-coder-model"
    provider: openai
    price_per_mtok_in: 1.0
    price_per_mtok_out: 2.0

routes:
  - model: s1
    use: [local-s1]
  - model: s2
    use: [cloud-s2, local-s2]
  - model: coder
    use: [cloud-coder]

embeddings_model: embed-local

clients:
  aiosd-l2:
    max_cloud_privacy: personal
    key_scope: session
  ceiling-public:
    max_cloud_privacy: public
    key_scope: session
  ceiling-secret:
    max_cloud_privacy: secret
    key_scope: session
  service-client:
    cloud: false
    key_scope: service
  service-secret:
    max_cloud_privacy: secret
    key_scope: service
"#
    )
}

/// Cost of one request with the fake usage (1000 tokens in, 2000 out).
const EXPECTED_COST: f64 = 0.001 + 0.004;

#[tokio::test]
async fn public_session_goes_to_cloud() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.cloud.count(), 1, "cloud must get the request");
    assert_eq!(harness.local.count(), 0, "local must not be called");

    let requests = harness.cloud.requests();
    assert_eq!(requests[0].path, "/v1/chat/completions");
    assert_eq!(requests[0].body["model"], "cloud-s2-model");
    // The gateway swaps the virtual key for the user's real cloud key.
    assert_eq!(
        requests[0].headers.get("authorization").map(String::as_str),
        Some("Bearer sk-test-key")
    );
}

#[tokio::test]
async fn raised_secret_session_goes_local() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.raise("session-a", "secret").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(body["label"], "secret");
    // A lowering request is ignored: 200 with the current label.
    let (status, body) = harness.raise("session-a", "public").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(body["label"], "secret");

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");

    let requests = harness.local.requests();
    assert_eq!(requests[0].body["model"], "local-s2-model");
    // Local never receives the cloud key.
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[tokio::test]
async fn secret_never_goes_to_cloud_even_if_config_allows() {
    let harness = Harness::start().await;
    // Both client and user ceilings allow `secret` on cloud (misconfiguration);
    // the code check must still block it.
    harness.write_user(1001, "secret", 2.0);
    let key = harness.issue_key(1001, "session-a", "ceiling-secret").await;
    let (status, body) = harness.raise("session-a", "secret").await;
    assert_eq!(status.as_u16(), 200, "{body}");

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
}

#[tokio::test]
async fn client_cannot_lower_session_label() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "ceiling-public").await;
    let (status, body) = harness.raise("session-a", "personal").await;
    assert_eq!(status.as_u16(), 200, "{body}");

    // The client declares `public` on a `personal` session: the label cannot be
    // lowered, so `personal` exceeds the client's `public` ceiling and goes local.
    let (status, body) = harness.chat_with_privacy(&key, "s2", Some("public")).await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
}

#[tokio::test]
async fn service_key_without_session_is_local_only() {
    let harness = Harness::start().await;
    harness.write_user(1001, "secret", 2.0);
    let key = harness.issue_service_key(1001, "service-client").await;

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "service key must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");

    // A service key with a `secret` ceiling still stays local: service keys are
    // always `secret`, and `secret` never goes to cloud.
    let key = harness.issue_service_key(1001, "service-secret").await;
    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 2, "service key must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
}

#[tokio::test]
async fn exhausted_budget_falls_back_to_local() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    harness.write_usage(1001, 3.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    // `s2` has a local route, so it goes local.
    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");

    // `coder` has only a cloud route, so there is no route: 409.
    let (status, body) = harness.chat(&key, "coder").await;
    assert_eq!(status.as_u16(), 409, "{body}");
    assert_eq!(body["error"]["code"], "no_route_privacy");
}

#[tokio::test]
async fn each_user_has_own_usage_file() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    harness.write_user(1002, "personal", 2.0);
    let key_a = harness.issue_key(1001, "session-a", "aiosd-l2").await;
    let key_b = harness.issue_key(1002, "session-b", "aiosd-l2").await;

    let (status, body) = harness.chat(&key_a, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    let (status, body) = harness.chat(&key_b, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.cloud.count(), 2);

    let usage_a = harness.usage(1001);
    let usage_b = harness.usage(1002);
    let day = utc_today();
    let spent_a = usage_a.spent_on(&day);
    let spent_b = usage_b.spent_on(&day);
    assert!(
        (spent_a - EXPECTED_COST).abs() < 1e-9,
        "cost of user 1001 = {spent_a}"
    );
    assert!(
        (spent_b - EXPECTED_COST).abs() < 1e-9,
        "cost of user 1002 = {spent_b}"
    );
    assert_eq!(usage_a.days[&day].requests, 1);
    assert_eq!(usage_b.days[&day].requests, 1);
}

#[tokio::test]
async fn revoke_all_invalidates_session_keys() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");

    let (status, body) = harness
        .admin_request(reqwest::Method::POST, "/v1/revoke-all", None)
        .await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(body["revoked"], 1);

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 401, "{body}");
    assert_eq!(body["error"]["code"], "invalid_api_key");
}

#[tokio::test]
async fn embeddings_always_run_local() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.embeddings(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.embed.count(), 1, "embedding must run local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
    assert_eq!(
        harness.local.count(),
        0,
        "local chat model must not be called"
    );

    let requests = harness.embed.requests();
    assert_eq!(requests[0].path, "/v1/embeddings");
    assert_eq!(requests[0].body["model"], "embed-model");
}

#[tokio::test]
async fn unknown_key_is_unauthorized() {
    let harness = Harness::start().await;
    let (status, body) = harness.chat("aios-sk-khong-ton-tai", "s2").await;
    assert_eq!(status.as_u16(), 401, "{body}");
    assert_eq!(body["error"]["code"], "invalid_api_key");
}

#[tokio::test]
async fn streaming_is_rejected() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/chat/completions", harness.api))
        .bearer_auth(&key)
        .json(&json!({
            "model": "s2",
            "messages": [{"role": "user", "content": "xin chào"}],
            "stream": true
        }))
        .send()
        .await
        .expect("chat request");
    assert_eq!(response.status().as_u16(), 400);
    let body: Value = response.json().await.expect("json body");
    assert_eq!(body["error"]["code"], "stream_not_supported");
    assert_eq!(harness.cloud.count(), 0);
    assert_eq!(harness.local.count(), 0);
}

#[tokio::test]
async fn unknown_model_is_not_found() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.chat(&key, "khong-ton-tai").await;
    assert_eq!(status.as_u16(), 404, "{body}");
    assert_eq!(body["error"]["code"], "model_not_found");
}

#[tokio::test]
async fn models_lists_logical_names() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.models(&key).await;
    assert_eq!(status.as_u16(), 200, "{body}");
    let ids: Vec<String> = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .map(|item| item["id"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(ids, vec!["coder", "s1", "s2"]);
}

#[tokio::test]
async fn admin_socket_rejects_other_uid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let local = FakeUpstream::start("fake-local").await;
    let embed = FakeUpstream::start("fake-embed").await;
    let cloud = FakeUpstream::start("fake-cloud").await;
    let config_path = dir.path().join("gateway.yaml");
    std::fs::write(
        &config_path,
        gateway_yaml(local.address, embed.address, cloud.address),
    )
    .expect("write gateway.yaml");
    let config = config::load_gateway_config(&config_path).expect("load gateway.yaml");
    let admin_socket = dir.path().join("gateway-admin.sock");
    let gateway = Gateway::bind(GatewayOptions {
        config,
        users_dir: dir.path().join("users"),
        admin_socket: Some(admin_socket.clone()),
        // A UID other than the test process UID: every call must be refused.
        admin_uid: Some(424_242),
    })
    .await
    .expect("bind gateway");
    tokio::spawn(async move {
        let _ = gateway.serve_until(std::future::pending()).await;
    });

    let client = reqwest::Client::builder()
        .unix_socket(admin_socket.as_path())
        .build()
        .expect("admin client");
    let response = client
        .post("http://aios-admin/v1/sessions")
        .json(&json!({"uid": 1001, "session_id": "s", "client": "aiosd-l2"}))
        .send()
        .await
        .expect("admin request");
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn cloud_key_with_loose_permissions_disables_cloud() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    let key_path = harness.users_dir.join("1001").join("cloud-key");
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644))
        .expect("chmod cloud key");
    let key = harness.issue_key(1001, "session-a", "aiosd-l2").await;

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
}

#[tokio::test]
async fn cloud_key_of_another_user_is_refused() {
    let harness = Harness::start().await;
    harness.write_user(1001, "personal", 2.0);
    harness.write_user(1002, "personal", 2.0);
    // User 1002's config points at user 1001's key file.
    let config_path = harness.users_dir.join("1002").join("gateway.yaml");
    let other_key = harness.users_dir.join("1001").join("cloud-key");
    let own_key = harness.users_dir.join("1002").join("cloud-key");
    let text = std::fs::read_to_string(&config_path).expect("read user config");
    let text = text.replace(
        &own_key.display().to_string(),
        &other_key.display().to_string(),
    );
    std::fs::write(&config_path, text).expect("write user config");
    let key = harness.issue_key(1002, "session-b", "aiosd-l2").await;

    let (status, body) = harness.chat(&key, "s2").await;
    assert_eq!(status.as_u16(), 200, "{body}");
    assert_eq!(harness.local.count(), 1, "must go local");
    assert_eq!(harness.cloud.count(), 0, "cloud must get nothing");
}

#[tokio::test]
async fn session_of_another_uid_is_refused() {
    let harness = Harness::start().await;
    harness.issue_key(1001, "session-a", "aiosd-l2").await;
    let (status, body) = harness
        .admin_request(
            reqwest::Method::POST,
            "/v1/sessions",
            Some(json!({"uid": 1002, "session_id": "session-a", "client": "aiosd-l2"})),
        )
        .await;
    assert_eq!(status.as_u16(), 409, "{body}");
    assert_eq!(body["error"]["code"], "session_uid_mismatch");
}

/// The sample `examples/gateway.yaml` must parse and validate.
#[test]
fn example_config_is_valid() {
    let text = include_str!("../examples/gateway.yaml");
    let config: config::GatewayConfig =
        serde_yaml_ng::from_str(text).expect("parse example config");
    config.validate().expect("validate example config");
    assert_eq!(config.logical_models(), vec!["coder", "s1", "s2"]);
}
