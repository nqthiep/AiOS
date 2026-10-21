//! `SandboxExecutor` in the VM (docs/m0/executor.md "Kiểm thử và nghiệm thu").
//!
//! Needs Linux, systemd `--user` with `aios-agent.slice`, bubblewrap and
//! python3: `dev/vm-run.sh 'cargo test -p aios-broker --test executor_vm -- --test-threads=1'`.
//! The tests look at every `aios-tool-*` scope, so they must run on one thread.
//!
//! | Test | Function |
//! | --- | --- |
//! | echo, process under `aios-agent.slice` | `echo_runs_under_agent_slice` |
//! | write inside the workspace | `write_inside_workspace` |
//! | write outside the workspace | `write_outside_workspace_fails` |
//! | sleep past the timeout | `timeout_kills_the_scope` |
//! | tool reports `privacy=public` on a `personal` spec | `tool_cannot_lower_privacy` |
//! | (extra) tool raises privacy / provenance | `tool_can_raise_labels` |
//! | (extra) S-14 L-3: only stdio reaches the tool | `no_inherited_fds` |
//! | (extra) sandbox exit codes 64–79 | `sandbox_error_maps_to_exit_code` |
#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use aios_broker::{ExecCtx, ExecError, Executor, SandboxConfig, SandboxExecutor};
use aios_types::{Channel, Principal, Privacy, Provenance, ToolOutput, ToolSpec};
use serde_json::{Value, json};

/// Minimal stdio MCP server (stdlib only) with the test tools.
const SERVER: &str = r#"
import json, os, sys, time

def text(s, is_error=False, meta=None):
    r = {"content": [{"type": "text", "text": s}], "isError": is_error}
    if meta:
        r["_meta"] = meta
    return r

def call(name, args):
    if name == "echo":
        time.sleep(args.get("delay", 0))
        return text(args["text"])
    if name == "write":
        try:
            with open(args["path"], "w") as f:
                f.write(args.get("content", ""))
            return text("written")
        except OSError as e:
            return text(f"cannot write: {e}", True)
    if name == "sleep":
        time.sleep(args["seconds"])
        return text("slept")
    if name == "label":
        return text("labeled", meta=args["meta"])
    if name == "fds":
        fds = []
        for fd in range(3, 1024):
            try:
                os.fstat(fd)
                fds.append(fd)
            except OSError:
                pass
        return text(json.dumps(fds))
    return None

for line in sys.stdin:
    msg = json.loads(line)
    mid = msg.get("id")
    if mid is None:
        continue
    method = msg.get("method")
    if method == "initialize":
        result = {"protocolVersion": msg["params"]["protocolVersion"],
                  "capabilities": {"tools": {}},
                  "serverInfo": {"name": "executor-test", "version": "0"}}
    elif method == "tools/call":
        result = call(msg["params"]["name"], msg["params"].get("arguments") or {})
    elif method == "ping":
        result = {}
    else:
        result = None
    if result is None:
        out = {"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": "unknown"}}
    else:
        out = {"jsonrpc": "2.0", "id": mid, "result": result}
    sys.stdout.write(json.dumps(out) + "\n")
    sys.stdout.flush()
"#;

const PLUGIN: &str = "test.exec";

/// `aios-sandbox-exec`: `AIOS_SANDBOX_EXEC`, else built next to this test.
fn sandbox_exec() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(p) = std::env::var_os("AIOS_SANDBOX_EXEC") {
            return PathBuf::from(p);
        }
        let status = std::process::Command::new(env!("CARGO"))
            .args(["build", "-q", "-p", "aios-sandbox-exec"])
            .status()
            .expect("cargo build");
        assert!(status.success(), "building aios-sandbox-exec failed");
        // target/<profile>/deps/executor_vm-* → target/<profile>/aios-sandbox-exec
        let exe = std::env::current_exe().unwrap();
        exe.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("aios-sandbox-exec")
    })
}

/// Test directories live outside /tmp: inside the sandbox /tmp is a tmpfs
/// with Landlock write access (S-14 §8).
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    executor: SandboxExecutor,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::var("AIOS_TEST_TMPDIR").unwrap_or_else(|_| "/var/tmp".into());
        let dir = tempfile::tempdir_in(base).unwrap();
        let root = dir.path().canonicalize().unwrap();
        let plugin = root.join("plugin");
        std::fs::create_dir_all(&plugin).unwrap();
        std::fs::write(plugin.join("server.py"), SERVER).unwrap();
        let executor = SandboxExecutor::new(SandboxConfig {
            sandbox_exec: sandbox_exec().to_owned(),
            slice: "aios-agent.slice".into(),
            profile_dir: root.clone(),
            plugin_dirs: HashMap::from([(PLUGIN.to_owned(), plugin)]),
        });
        Self {
            _dir: dir,
            root,
            executor,
        }
    }

    fn workspace(&self) -> PathBuf {
        self.root.join("ws")
    }

    async fn run(
        &self,
        spec: &ToolSpec,
        args: Value,
        timeout: Duration,
    ) -> Result<ToolOutput, ExecError> {
        let ctx = ExecCtx {
            principal: Principal {
                uid: nix_uid(),
                is_admin: false,
                channel: Channel::Local,
                session_id: None,
            },
            session_id: "s".into(),
            workspace: self.workspace(),
            timeout,
        };
        self.executor.run(spec, &args, &ctx).await
    }
}

fn nix_uid() -> u32 {
    nix::unistd::getuid().as_raw()
}

fn spec(name: &str) -> ToolSpec {
    serde_json::from_value(json!({
        "name": name,
        "plugin_id": PLUGIN,
        "description": "",
        "input_schema": {"type": "object"},
        "risk": "medium",
        "reversible": true,
        "result_privacy": "personal",
        "result_provenance": "observed",
        "entry": ["python3", "-I", "$PLUGIN_DIR/server.py"],
        "sandbox": {
            "ro_paths": [],
            "rw_paths": ["$WORKSPACE"],
            "tcp_connect": [],
            "allow_unix_socket": [],
            "memory_max_mb": 256,
            "landlock_required": true
        }
    }))
    .unwrap()
}

fn first_text(out: &ToolOutput) -> &str {
    out.content[0]["text"].as_str().unwrap()
}

/// Every live `aios-tool-*` scope directory in the cgroup tree.
fn tool_scopes() -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let path = e.path();
            if !path.is_dir() {
                continue;
            }
            if e.file_name().to_string_lossy().starts_with("aios-tool-") {
                out.push(path);
            } else if depth > 0 {
                walk(&path, depth - 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(Path::new("/sys/fs/cgroup"), 8, &mut out);
    out
}

fn scope_pids(scope: &Path) -> Vec<String> {
    std::fs::read_to_string(scope.join("cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn echo_runs_under_agent_slice() {
    let fx = Fixture::new();
    let spec = spec("echo");
    let call = fx.run(
        &spec,
        json!({"text": "xin chào", "delay": 2}),
        Duration::from_secs(20),
    );
    let watch = async {
        // While the tool runs: its processes sit in a scope under aios-agent.slice.
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            for scope in tool_scopes() {
                for pid in scope_pids(&scope) {
                    if let Ok(cg) = std::fs::read_to_string(format!("/proc/{pid}/cgroup")) {
                        return Some(cg);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    };
    let (out, cgroup) = tokio::join!(call, watch);
    let out = out.unwrap();
    assert!(!out.is_error);
    assert_eq!(first_text(&out), "xin chào");
    assert_eq!(out.privacy, Privacy::Personal);
    assert_eq!(out.provenance, Provenance::Observed);
    let cgroup = cgroup.expect("no process found in an aios-tool-* scope");
    println!("tool cgroup: {cgroup}");
    assert!(
        cgroup.contains("/aios-agent.slice/aios-tool-"),
        "not under aios-agent.slice: {cgroup}"
    );
}

#[tokio::test]
async fn write_inside_workspace() {
    let fx = Fixture::new();
    let target = fx.workspace().join("note.txt");
    let out = fx
        .run(
            &spec("write"),
            json!({"path": target, "content": "hello"}),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    assert!(!out.is_error, "{:?}", out.content);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
}

#[tokio::test]
async fn write_outside_workspace_fails() {
    let fx = Fixture::new();
    let outside = fx.root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let target = outside.join("x.txt");
    let out = fx
        .run(
            &spec("write"),
            json!({"path": target, "content": "nope"}),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    println!("outside write: {}", first_text(&out));
    assert!(out.is_error, "{:?}", out.content);
    assert!(!target.exists());
}

#[tokio::test]
async fn timeout_kills_the_scope() {
    let fx = Fixture::new();
    let started = Instant::now();
    let err = fx
        .run(
            &spec("sleep"),
            json!({"seconds": 60}),
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ExecError::Timeout), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(10));
    tokio::time::sleep(Duration::from_secs(3)).await;
    let left: Vec<_> = tool_scopes()
        .into_iter()
        .filter(|s| !scope_pids(s).is_empty())
        .collect();
    assert!(left.is_empty(), "aios-tool-* processes left: {left:?}");
    let units = std::process::Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "--all",
            "--no-legend",
            "aios-tool-*",
        ])
        .output()
        .unwrap();
    let units = String::from_utf8_lossy(&units.stdout);
    assert!(units.trim().is_empty(), "units left: {units}");
}

#[tokio::test]
async fn tool_cannot_lower_privacy() {
    let fx = Fixture::new();
    let out = fx
        .run(
            &spec("label"),
            json!({"meta": {"privacy": "public"}}),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    assert_eq!(out.privacy, Privacy::Personal);
    assert_eq!(out.provenance, Provenance::Observed);
}

#[tokio::test]
async fn tool_can_raise_labels() {
    let fx = Fixture::new();
    let out = fx
        .run(
            &spec("label"),
            json!({"meta": {"privacy": "secret", "provenance": "external"}}),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    assert_eq!(out.privacy, Privacy::Secret);
    assert_eq!(out.provenance, Provenance::External);
}

#[tokio::test]
async fn no_inherited_fds() {
    let fx = Fixture::new();
    // The broker's own fds (tokio runtime, sockets, files) are O_CLOEXEC.
    let _open = std::fs::File::open("/etc/hostname").unwrap();
    let out = fx
        .run(&spec("fds"), json!({}), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(first_text(&out), "[]", "tool sees fds >= 3");

    // A fd without O_CLOEXEC would cross the sandbox: refuse to run.
    let leaked = nix::fcntl::open(
        "/etc/hostname",
        nix::fcntl::OFlag::O_RDONLY,
        nix::sys::stat::Mode::empty(),
    )
    .unwrap();
    let err = fx
        .run(&spec("fds"), json!({}), Duration::from_secs(20))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ExecError::Failed(ref m) if m.contains("O_CLOEXEC")),
        "{err:?}"
    );
    drop(leaked);
}

#[tokio::test]
async fn sandbox_error_maps_to_exit_code() {
    let fx = Fixture::new();
    let mut spec = spec("echo");
    // A profile path that does not exist: aios-sandbox-exec exits 64.
    spec.sandbox.ro_paths = vec![fx.root.join("missing").to_string_lossy().into_owned()];
    let err = fx
        .run(&spec, json!({"text": "x"}), Duration::from_secs(20))
        .await
        .unwrap_err();
    assert!(matches!(err, ExecError::Sandbox(64)), "{err:?}");
    println!("{err}");
}
