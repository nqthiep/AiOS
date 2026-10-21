//! Integration tests of docs/m0/broker.md §Kiểm thử, with `FakeExecutor` and
//! a fake Model Gateway on a Unix socket.
//!
//! | Spec row | Test |
//! | --- | --- |
//! | no Bearer → 401 | `missing_or_wrong_bearer_is_401` |
//! | low, autonomy 1 → runs, 1 audit line | `low_tool_autonomy_1_runs_with_one_audit_line` |
//! | low, autonomy 0 → confirm, valid token → runs | `low_tool_autonomy_0_confirms_then_runs` |
//! | token signed for other args → confirmation_invalid | `token_for_other_args_is_invalid` |
//! | reused token → confirmation_invalid | `reused_token_is_invalid` |
//! | same idempotency key twice → executor once | `same_idempotency_key_executes_once` |
//! | privacy=secret → gateway raise before result | `secret_output_raises_gateway_before_result` |
//! | gateway raise fails → label_sync_failed, no content | `gateway_raise_failure_withholds_content` |
//! | external output, next low call at autonomy 3 → Confirm | `external_output_taints_session` |
//! | uid A confirms uid B's request → refused | `other_uid_cannot_confirm_or_see_pending` |
//! | audit verify → ok | every test ends with `Harness::finish` |

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aios_audit::AuditLog;
use aios_broker::{
    Autonomy, Broker, BrokerConfig, BrokerDeps, FakeExecutor, IDEMPOTENCY_TTL, SESSION_TTL,
    SessionInfo, admin, mcp,
};
use aios_confirm::{ConfirmClaims, PendingConfirm, SignedStep, Verifier, args_hash, sign};
use aios_policy::PolicyEngine;
use aios_registry::{Registry, RegistryConfig, generate_keypair, sign_file};
use aios_types::{Channel, Privacy, Provenance, ToolOutput};
use axum::extract::{Path as UrlPath, State};
use axum::routing::{delete, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use bytes::Bytes;
use ed25519_dalek::SigningKey;
use http::{Method, Request, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};

const MANIFEST: &str = r#"id = "aios.t"
version = "1.0.0"
api = "aios:plugin@0.1"
provides = ["tool"]
runtime = "mcp"
entry = ["/bin/true"]
capabilities = ["fs.read:$WORKSPACE"]

[sandbox]
ro_paths = ["$WORKSPACE"]

[[tools]]
name = "t.low"
description = "low tool"
risk = "low"
reversible = true
input_schema = '{"type":"object","properties":{"q":{"type":"string"}},"required":["q"],"additionalProperties":false}'

[[tools]]
name = "t.secret"
description = "returns secret data"
risk = "low"
reversible = true
result_privacy = "secret"

[[tools]]
name = "t.ext"
description = "returns external content"
risk = "low"
reversible = true
result_provenance = "external"

[[tools]]
name = "t.sys"
description = "system scope tool"
risk = "low"
reversible = true
system_scope = true
"#;

/// The fake Model Gateway admin API: records requests, can fail `raise`.
#[derive(Default)]
struct FakeGateway {
    requests: Mutex<Vec<(String, Value)>>,
    fail_raise: AtomicBool,
}

impl FakeGateway {
    fn requests(&self) -> Vec<(String, Value)> {
        self.requests.lock().unwrap().clone()
    }

    fn raises(&self) -> Vec<Value> {
        self.requests()
            .into_iter()
            .filter(|(p, _)| p.ends_with("/raise"))
            .map(|(_, b)| b)
            .collect()
    }
}

async fn gw_create(State(gw): State<Arc<FakeGateway>>, Json(body): Json<Value>) -> Json<Value> {
    gw.requests
        .lock()
        .unwrap()
        .push(("/v1/sessions".into(), body.clone()));
    Json(json!({"key": format!("aios-sk-{}", body["session_id"].as_str().unwrap())}))
}

async fn gw_raise(
    State(gw): State<Arc<FakeGateway>>,
    UrlPath(id): UrlPath<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    gw.requests
        .lock()
        .unwrap()
        .push((format!("/v1/sessions/{id}/raise"), body.clone()));
    if gw.fail_raise.load(Ordering::SeqCst) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(Json(body))
}

async fn gw_delete(
    State(gw): State<Arc<FakeGateway>>,
    UrlPath(id): UrlPath<String>,
) -> Json<Value> {
    gw.requests
        .lock()
        .unwrap()
        .push((format!("DELETE /v1/sessions/{id}"), Value::Null));
    Json(json!({}))
}

struct Opts {
    autonomy: u8,
    admin_group: String,
    confirm_timeout: Duration,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            autonomy: 1,
            admin_group: "aios-no-such-group".into(),
            confirm_timeout: Duration::from_secs(10),
        }
    }
}

struct Harness {
    _dir: TempDir,
    broker: Arc<Broker>,
    exec: Arc<FakeExecutor>,
    gw: Arc<FakeGateway>,
    mcp_addr: std::net::SocketAddr,
    admin_sock: PathBuf,
    audit_path: PathBuf,
    signer: SigningKey,
    nonce: AtomicU8,
}

fn output(text: &str, privacy: Privacy, provenance: Provenance) -> ToolOutput {
    ToolOutput {
        content: json!([{"type": "text", "text": text}]),
        is_error: false,
        privacy,
        provenance,
    }
}

fn my_uid() -> u32 {
    nix::unistd::getuid().as_raw()
}

impl Harness {
    async fn new(opts: Opts) -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Signed registry with one core plugin.
        let key = generate_keypair();
        for d in ["core/t", "vendor", "local", "trust/core", "trust/vendors"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        std::fs::write(root.join("trust/core/t.pub"), key.public().to_base64()).unwrap();
        let manifest = root.join("core/t/plugin.toml");
        std::fs::write(&manifest, MANIFEST).unwrap();
        sign_file(&manifest, key.secret()).unwrap();
        let (registry, load_report) = Registry::load(&RegistryConfig {
            core_dir: root.join("core"),
            vendor_dir: root.join("vendor"),
            local_dir: root.join("local"),
            trust_dir: root.join("trust"),
        });
        assert!(
            load_report.rejected.is_empty(),
            "{:?}",
            load_report.rejected
        );
        assert_eq!(registry.tool_count(), 4);

        let policy =
            PolicyEngine::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../policy"))
                .unwrap();
        let signer = SigningKey::from_bytes(&[7; 32]);
        let audit_path = root.join("audit.jsonl");

        let exec = Arc::new(
            FakeExecutor::new()
                .with_output(
                    "t.low",
                    output("low result", Privacy::Personal, Provenance::Observed),
                )
                .with_output(
                    "t.secret",
                    output("the secret", Privacy::Secret, Provenance::Observed),
                )
                .with_output(
                    "t.ext",
                    output("web page", Privacy::Public, Provenance::External),
                )
                .with_output(
                    "t.sys",
                    output("sys result", Privacy::Personal, Provenance::Observed),
                ),
        );

        // Fake gateway.
        let gw = Arc::new(FakeGateway::default());
        let gw_sock = root.join("gw.sock");
        let gw_router = Router::new()
            .route("/v1/sessions", post(gw_create))
            .route("/v1/sessions/{id}/raise", post(gw_raise))
            .route("/v1/sessions/{id}", delete(gw_delete))
            .with_state(gw.clone());
        let gw_listener = UnixListener::bind(&gw_sock).unwrap();
        tokio::spawn(async move { axum::serve(gw_listener, gw_router).await });

        let mcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mcp_addr = mcp_listener.local_addr().unwrap();
        let admin_sock = root.join("core.sock");
        let admin_listener = UnixListener::bind(&admin_sock).unwrap();

        let broker = Broker::new(
            BrokerConfig {
                autonomy: Autonomy {
                    level: opts.autonomy,
                    ..Default::default()
                },
                admin_group: opts.admin_group,
                gateway_admin_socket: gw_sock,
                gateway_client: "aiosd-l2".into(),
                broker_url: format!("http://{mcp_addr}/mcp"),
                gateway_url: "http://127.0.0.1:8090/v1".into(),
                workspace_root: root.join("ws"),
                confirm_timeout: opts.confirm_timeout,
                session_ttl: SESSION_TTL,
                idempotency_ttl: IDEMPOTENCY_TTL,
            },
            BrokerDeps {
                registry,
                load_report,
                policy,
                verifier: Verifier::new(signer.verifying_key()),
                audit: AuditLog::open(&audit_path).unwrap(),
                executor: exec.clone(),
            },
        );
        tokio::spawn(mcp::serve(broker.clone(), mcp_listener));
        tokio::spawn(admin::serve(broker.clone(), admin_listener));

        Self {
            _dir: dir,
            broker,
            exec,
            gw,
            mcp_addr,
            admin_sock,
            audit_path,
            signer,
            nonce: AtomicU8::new(1),
        }
    }

    async fn admin(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let io = UnixStream::connect(&self.admin_sock).await.unwrap();
        http(io, method, path, &[], body).await
    }

    async fn session(&self) -> SessionInfo {
        let (status, body) = self
            .admin(
                Method::POST,
                "/v1/session",
                Some(json!({"channel": "local"})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        serde_json::from_value(body).unwrap()
    }

    async fn mcp(&self, token: Option<&str>, method: &str, params: Value) -> (StatusCode, Value) {
        let io = TcpStream::connect(self.mcp_addr).await.unwrap();
        let auth = token.map(|t| format!("Bearer {t}"));
        let mut headers = vec![
            ("accept", "application/json, text/event-stream"),
            ("content-type", "application/json"),
        ];
        if let Some(a) = &auth {
            headers.push(("authorization", a));
        }
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        http(io, Method::POST, "/mcp", &headers, Some(body)).await
    }

    /// `tools/call`; returns the JSON-RPC response object.
    async fn call(&self, token: &str, tool: &str, args: Value, key: &str) -> Value {
        let (status, body) = self
            .mcp(
                Some(token),
                "tools/call",
                json!({"name": tool, "arguments": args, "_meta": {"idempotency_key": key}}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    fn spawn_call(
        self: &Arc<Self>,
        token: &str,
        tool: &str,
        args: Value,
        key: &str,
    ) -> tokio::task::JoinHandle<Value> {
        let (h, token, tool, key) = (
            self.clone(),
            token.to_owned(),
            tool.to_owned(),
            key.to_owned(),
        );
        tokio::spawn(async move { h.call(&token, &tool, args, &key).await })
    }

    /// Wait until `count` requests are pending for `uid` (library view).
    async fn wait_pending(&self, uid: u32, count: usize) -> Vec<PendingConfirm> {
        for _ in 0..200 {
            let p = self.broker.pending_for_uid(uid);
            if p.len() >= count {
                return p;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("no pending confirmation appeared");
    }

    /// A trusted-UI token for `pending`, binding `args`.
    fn token(&self, pending: &PendingConfirm, args: &Value) -> String {
        let n = self.nonce.fetch_add(1, Ordering::SeqCst);
        let claims = ConfirmClaims {
            v: 1,
            request_id: pending.request_id.clone(),
            principal: pending.principal.clone(),
            uid: pending.uid,
            task_step: pending.task_step.clone(),
            steps: vec![SignedStep {
                tool: pending.steps[0].0.clone(),
                args_hash: args_hash(args).unwrap(),
            }],
            dry_run_hash: None,
            policy_version: pending.policy_version.clone(),
            nonce: URL_SAFE_NO_PAD.encode([n; 16]),
            exp_unix: pending.created_unix + 60,
        };
        sign(&claims, &self.signer)
    }

    async fn confirm(&self, request_id: &str, token: &str) -> (StatusCode, Value) {
        self.admin(
            Method::POST,
            "/v1/confirm",
            Some(json!({"request_id": request_id, "token": token})),
        )
        .await
    }

    fn audit_lines(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.audit_path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn audit_of(&self, kind: &str) -> Vec<Value> {
        self.audit_lines()
            .into_iter()
            .filter(|l| l["kind"] == kind)
            .map(|l| l["data"].clone())
            .collect()
    }

    fn exec_count(&self, tool: &str) -> usize {
        self.exec.calls().iter().filter(|(t, _)| t == tool).count()
    }

    fn finish(&self) {
        let report = aios_audit::verify(&self.audit_path).unwrap();
        assert!(report.is_ok(), "{:?}", report.broken);
    }
}

async fn http<IO>(
    io: IO,
    method: Method,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, Value)
where
    IO: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(io))
        .await
        .unwrap();
    tokio::spawn(conn);
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "127.0.0.1");
    if body.is_some() {
        req = req.header("content-type", "application/json");
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let bytes = body.map(|b| b.to_string()).unwrap_or_default();
    let resp = sender
        .send_request(req.body(Full::new(Bytes::from(bytes))).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    let value = serde_json::from_str(&text).unwrap_or_else(|_| {
        // SSE fallback: the first `data:` line holding a JSON-RPC message.
        text.lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .filter_map(|d| serde_json::from_str::<Value>(d.trim()).ok())
            .find(|v| v.get("jsonrpc").is_some())
            .unwrap_or(Value::Null)
    });
    (status, value)
}

fn error_name(resp: &Value) -> &str {
    resp["error"]["data"]["error"].as_str().unwrap_or("<none>")
}

#[tokio::test]
async fn missing_or_wrong_bearer_is_401() {
    let h = Harness::new(Opts::default()).await;
    let (status, _) = h.mcp(None, "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = h.mcp(Some("not-a-token"), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let s = h.session().await;
    let (status, body) = h.mcp(Some(&s.broker_token), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    // system_scope tools are listed for non-admins too.
    assert_eq!(names, ["t.ext", "t.low", "t.secret", "t.sys"]);
    assert_eq!(
        body["result"]["tools"][1]["inputSchema"]["required"],
        json!(["q"])
    );
    h.finish();
}

#[tokio::test]
async fn low_tool_autonomy_1_runs_with_one_audit_line() {
    let h = Harness::new(Opts::default()).await;
    let s = h.session().await;
    let resp = h
        .call(&s.broker_token, "t.low", json!({"q": "x"}), "task:0")
        .await;
    let result = &resp["result"];
    assert_eq!(result["content"][0]["text"], "low result", "{resp}");
    assert_eq!(
        result["_meta"],
        json!({"privacy": "personal", "provenance": "observed"})
    );
    assert_eq!(h.exec_count("t.low"), 1);

    let calls = h.audit_of("tool_call");
    assert_eq!(calls.len(), 1);
    let line = &calls[0];
    assert_eq!(line["verdict"], "allow");
    assert_eq!(line["reason"], "auto");
    assert_eq!(line["result"], "ok");
    assert_eq!(line["approver"], Value::Null);
    assert_eq!(line["args"], json!({"q": "x"}));
    assert_eq!(line["session"], s.session_id.as_str());
    assert_eq!(line["idempotency_key"], "task:0");
    h.finish();
}

#[tokio::test]
async fn low_tool_autonomy_0_confirms_then_runs() {
    let h = Arc::new(
        Harness::new(Opts {
            autonomy: 0,
            ..Default::default()
        })
        .await,
    );
    let s = h.session().await;
    let call = h.spawn_call(&s.broker_token, "t.low", json!({"q": "x"}), "task:0");

    let pending = h.wait_pending(my_uid(), 1).await.remove(0);
    let (_, listed) = h.admin(Method::GET, "/v1/pending", None).await;
    assert_eq!(listed[0]["request_id"], pending.request_id.as_str());
    assert_eq!(listed[0]["steps"][0]["args"], json!({"q": "x"}));
    assert_eq!(h.exec_count("t.low"), 0, "must not run before confirmation");

    let token = h.token(&pending, &json!({"q": "x"}));
    let (status, body) = h.confirm(&pending.request_id, &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let resp = call.await.unwrap();
    assert_eq!(resp["result"]["content"][0]["text"], "low result", "{resp}");
    assert_eq!(h.exec_count("t.low"), 1);
    let confirm = h.audit_of("confirm");
    assert_eq!(confirm.len(), 1);
    assert_eq!(confirm[0]["outcome"], "approved");
    let calls = h.audit_of("tool_call");
    assert_eq!(calls[0]["verdict"], "confirm");
    assert_eq!(calls[0]["approver"], "trusted_ui");
    h.finish();
}

#[tokio::test]
async fn token_for_other_args_is_invalid() {
    let h = Arc::new(
        Harness::new(Opts {
            autonomy: 0,
            ..Default::default()
        })
        .await,
    );
    let s = h.session().await;
    let call = h.spawn_call(&s.broker_token, "t.low", json!({"q": "a"}), "task:0");
    let pending = h.wait_pending(my_uid(), 1).await.remove(0);

    let token = h.token(&pending, &json!({"q": "b"}));
    let (status, body) = h.confirm(&pending.request_id, &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "confirmation_invalid");

    let resp = call.await.unwrap();
    assert_eq!(error_name(&resp), "confirmation_invalid", "{resp}");
    assert!(resp.get("result").is_none());
    assert_eq!(h.exec_count("t.low"), 0);
    assert_eq!(h.audit_of("confirm")[0]["outcome"], "invalid");
    h.finish();
}

#[tokio::test]
async fn reused_token_is_invalid() {
    let h = Arc::new(
        Harness::new(Opts {
            autonomy: 0,
            ..Default::default()
        })
        .await,
    );
    let s = h.session().await;
    let args = json!({"q": "x"});

    let first = h.spawn_call(&s.broker_token, "t.low", args.clone(), "task:0");
    let p1 = h.wait_pending(my_uid(), 1).await.remove(0);
    let token = h.token(&p1, &args);
    assert_eq!(h.confirm(&p1.request_id, &token).await.0, StatusCode::OK);
    assert!(first.await.unwrap().get("result").is_some());

    // Same token again for the consumed request: it is gone.
    assert_eq!(
        h.confirm(&p1.request_id, &token).await.0,
        StatusCode::NOT_FOUND
    );

    // Same token for a new request with the same args.
    let second = h.spawn_call(&s.broker_token, "t.low", args.clone(), "task:1");
    let p2 = h.wait_pending(my_uid(), 1).await.remove(0);
    assert_ne!(p1.request_id, p2.request_id);
    let (status, body) = h.confirm(&p2.request_id, &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let resp = second.await.unwrap();
    assert_eq!(error_name(&resp), "confirmation_invalid", "{resp}");
    assert_eq!(h.exec_count("t.low"), 1);
    h.finish();
}

#[tokio::test]
async fn same_idempotency_key_executes_once() {
    let h = Harness::new(Opts::default()).await;
    let s = h.session().await;
    let a = h
        .call(&s.broker_token, "t.low", json!({"q": "x"}), "task:7")
        .await;
    let b = h
        .call(&s.broker_token, "t.low", json!({"q": "x"}), "task:7")
        .await;
    assert_eq!(a["result"], b["result"]);
    assert_eq!(h.exec_count("t.low"), 1);

    let calls = h.audit_of("tool_call");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].get("replay"), None);
    assert_eq!(calls[1]["replay"], true);

    // The key cannot be reused for another call.
    let c = h
        .call(&s.broker_token, "t.low", json!({"q": "y"}), "task:7")
        .await;
    assert_eq!(error_name(&c), "invalid_params", "{c}");
    // A different session has its own keys.
    let s2 = h.session().await;
    h.call(&s2.broker_token, "t.low", json!({"q": "x"}), "task:7")
        .await;
    assert_eq!(h.exec_count("t.low"), 2);
    h.finish();
}

#[tokio::test]
async fn secret_output_raises_gateway_before_result() {
    let h = Harness::new(Opts::default()).await;
    let s = h.session().await;
    let resp = h
        .call(
            &s.broker_token,
            "t.secret",
            json!({"needle": "hunter2-arg"}),
            "task:0",
        )
        .await;
    // By the time the client holds the result, the gateway already has the label.
    let raises = h.gw.raises();
    assert_eq!(raises, vec![json!({"label": "secret", "taint": "clean"})]);
    let raise_path = format!("/v1/sessions/{}/raise", s.session_id);
    assert!(h.gw.requests().iter().any(|(p, _)| *p == raise_path));
    assert_eq!(resp["result"]["_meta"]["privacy"], "secret", "{resp}");

    // Args of a secret session are redacted in the audit log.
    let audit = std::fs::read_to_string(&h.audit_path).unwrap();
    assert!(!audit.contains("hunter2-arg"));
    assert!(
        h.audit_of("tool_call")[0]["args"]
            .as_str()
            .unwrap()
            .starts_with("<secret:sha256:")
    );
    h.finish();
}

#[tokio::test]
async fn gateway_raise_failure_withholds_content() {
    let h = Harness::new(Opts::default()).await;
    h.gw.fail_raise.store(true, Ordering::SeqCst);
    let s = h.session().await;
    let resp = h
        .call(&s.broker_token, "t.secret", json!({}), "task:0")
        .await;
    assert_eq!(error_name(&resp), "label_sync_failed", "{resp}");
    assert!(resp.get("result").is_none());
    assert!(!resp.to_string().contains("the secret"));
    assert_eq!(h.audit_of("tool_call")[0]["error"], "label_sync_failed");

    // The unsynced label keeps blocking content until the gateway has it.
    let resp = h
        .call(&s.broker_token, "t.low", json!({"q": "x"}), "task:1")
        .await;
    assert_eq!(error_name(&resp), "label_sync_failed", "{resp}");
    h.gw.fail_raise.store(false, Ordering::SeqCst);
    let resp = h
        .call(&s.broker_token, "t.low", json!({"q": "x"}), "task:2")
        .await;
    assert!(resp.get("result").is_some(), "{resp}");
    assert_eq!(
        h.gw.raises().last().unwrap(),
        &json!({"label": "secret", "taint": "clean"})
    );
    h.finish();
}

#[tokio::test]
async fn external_output_taints_session() {
    let h = Arc::new(
        Harness::new(Opts {
            autonomy: 3,
            ..Default::default()
        })
        .await,
    );
    let s = h.session().await;
    let resp = h.call(&s.broker_token, "t.ext", json!({}), "task:0").await;
    assert_eq!(resp["result"]["_meta"]["provenance"], "external", "{resp}");
    // The registry default result_privacy (personal) is a floor for the tool output.
    assert_eq!(
        h.gw.raises(),
        vec![json!({"label": "personal", "taint": "tainted"})]
    );

    let call = h.spawn_call(&s.broker_token, "t.low", json!({"q": "x"}), "task:1");
    let pending = h.wait_pending(my_uid(), 1).await.remove(0);
    let (status, _) = h
        .admin(
            Method::POST,
            "/v1/deny",
            Some(json!({"request_id": pending.request_id})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let resp = call.await.unwrap();
    assert_eq!(error_name(&resp), "confirmation_denied", "{resp}");
    assert_eq!(h.exec_count("t.low"), 0);
    assert_eq!(h.audit_of("confirm")[0]["outcome"], "denied");
    h.finish();
}

#[tokio::test]
async fn other_uid_cannot_confirm_or_see_pending() {
    let h = Arc::new(
        Harness::new(Opts {
            autonomy: 0,
            confirm_timeout: Duration::from_secs(2),
            ..Default::default()
        })
        .await,
    );
    // Session of uid B, created through the library as the socket would for B.
    let uid_b = my_uid() + 1;
    let b = h
        .broker
        .create_session(uid_b, Channel::Local)
        .await
        .unwrap();
    let call = h.spawn_call(&b.broker_token, "t.low", json!({"q": "x"}), "task:0");
    let pending = h.wait_pending(uid_b, 1).await.remove(0);
    assert_eq!(pending.uid, uid_b);

    // This process is uid A on the admin socket.
    let (_, listed) = h.admin(Method::GET, "/v1/pending", None).await;
    assert_eq!(listed, json!([]));
    let token = h.token(&pending, &json!({"q": "x"}));
    let (status, _) = h.confirm(&pending.request_id, &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = h
        .admin(
            Method::POST,
            "/v1/deny",
            Some(json!({"request_id": pending.request_id})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let resp = call.await.unwrap();
    assert_eq!(error_name(&resp), "confirmation_timeout", "{resp}");
    assert_eq!(h.exec_count("t.low"), 0);
    assert!(h.broker.pending_for_uid(uid_b).is_empty());
    h.finish();
}

#[tokio::test]
async fn deny_unknown_tool_and_invalid_params() {
    let h = Harness::new(Opts::default()).await;
    let s = h.session().await;
    let t = &s.broker_token;

    let resp = h.call(t, "t.sys", json!({}), "k:0").await;
    assert_eq!(error_name(&resp), "denied", "{resp}");
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("forbid-system-scope-non-admin")
    );
    assert_eq!(h.audit_of("tool_call")[0]["verdict"], "deny");

    let resp = h.call(t, "t.nope", json!({}), "k:1").await;
    assert_eq!(error_name(&resp), "unknown_tool", "{resp}");
    assert_eq!(resp["error"]["code"], -32602);

    let resp = h.call(t, "t.low", json!({"q": 1}), "k:2").await;
    assert_eq!(error_name(&resp), "invalid_params", "{resp}");
    assert_eq!(resp["error"]["code"], -32602);

    let (_, resp) = h
        .mcp(
            Some(t),
            "tools/call",
            json!({"name": "t.low", "arguments": {"q": "x"}}),
        )
        .await;
    assert_eq!(error_name(&resp), "invalid_params", "{resp}");
    let long = "k".repeat(129);
    let resp = h.call(t, "t.low", json!({"q": "x"}), &long).await;
    assert_eq!(error_name(&resp), "invalid_params", "{resp}");

    assert!(h.exec.calls().is_empty());
    h.finish();
}

#[tokio::test]
async fn session_uses_peer_uid_and_admin_group() {
    let gid = nix::unistd::getgid();
    let group = nix::unistd::Group::from_gid(gid).unwrap().unwrap().name;
    let h = Arc::new(
        Harness::new(Opts {
            admin_group: group,
            ..Default::default()
        })
        .await,
    );
    // A uid in the body is ignored: the gateway sees the peer uid.
    let (status, body) = h
        .admin(
            Method::POST,
            "/v1/session",
            Some(json!({"channel": "local", "uid": 0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let s: SessionInfo = serde_json::from_value(body).unwrap();
    let created = &h.gw.requests()[0];
    assert_eq!(created.0, "/v1/sessions");
    assert_eq!(created.1["uid"], my_uid());
    assert_eq!(created.1["client"], "aiosd-l2");
    assert_eq!(s.gateway_key, format!("aios-sk-{}", s.session_id));
    assert_eq!(s.broker_url, format!("http://{}/mcp", h.mcp_addr));
    assert_eq!(URL_SAFE_NO_PAD.decode(&s.broker_token).unwrap().len(), 32);

    // Admin: the system_scope tool is confirmable instead of denied.
    let call = h.spawn_call(&s.broker_token, "t.sys", json!({}), "k:0");
    let pending = h.wait_pending(my_uid(), 1).await.remove(0);
    assert_eq!(pending.steps[0].0, "t.sys");
    let token = h.token(&pending, &json!({}));
    assert_eq!(
        h.confirm(&pending.request_id, &token).await.0,
        StatusCode::OK
    );
    assert!(call.await.unwrap().get("result").is_some());

    let (status, st) = h.admin(Method::GET, "/v1/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(st["tools"], 4);
    assert_eq!(st["registry"]["loaded"], json!(["aios.t"]));
    assert_eq!(st["policy_version"].as_str().unwrap().len(), 64);
    assert_eq!(h.audit_of("session_create").len(), 1);
    h.finish();
}
