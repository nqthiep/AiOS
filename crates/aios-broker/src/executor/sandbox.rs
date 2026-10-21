//! [`SandboxExecutor`]: one short-lived MCP server per tool call, in a
//! transient systemd scope under the user's `aios-agent.slice`, wrapped by
//! `aios-sandbox-exec` (docs/m0/executor.md).
//!
//! ```text
//! systemd-run --user --scope --quiet --collect --slice=<slice> \
//!   --unit=aios-tool-<uuid> -p MemoryMax=<mb>M -p TasksMax=64 \
//!   -- aios-sandbox-exec --profile <file> -- <entry...>
//! ```
//!
//! Inherited file descriptors (S-14 L-3): neither `systemd-run --scope`,
//! `aios-sandbox-exec` nor bubblewrap close inherited fds (measured in the
//! VM), and Landlock does not check fds opened before the sandbox. The child
//! only gets its stdin/stdout/stderr pipes: std and tokio open every fd with
//! `O_CLOEXEC`, and [`check_inherited_fds`] refuses to spawn when this
//! process holds an fd >= 3 without `O_CLOEXEC`.

use std::collections::HashMap;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use aios_types::{Privacy, Provenance, SandboxProfile, ToolOutput, ToolSpec};
use async_trait::async_trait;
use rmcp::ServiceExt as _;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt as _;
use tokio::process::{Child, Command};

use super::{ExecCtx, ExecError, Executor};

/// How long the tool server may take to exit after its stdin is closed.
const EXIT_GRACE: Duration = Duration::from_secs(2);
/// How long the scope gets between SIGTERM and SIGKILL.
const TERM_GRACE: Duration = Duration::from_secs(1);
/// At most this much of the tool's stderr is kept for the log.
const STDERR_CAP: usize = 64 * 1024;
/// `TasksMax` of the transient scope.
const TASKS_MAX: u32 = 64;
/// Environment passed to systemd-run and the sandbox; everything else of
/// aiosd-core's environment is dropped.
const ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "LANG",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
];
/// `O_CLOEXEC` in the `flags:` line of `/proc/self/fdinfo/<fd>` (octal).
const O_CLOEXEC: u32 = 0o2_000_000;

/// Where the executor finds its helpers and the plugins.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// The `aios-sandbox-exec` binary (a bare name is looked up in `PATH`).
    pub sandbox_exec: PathBuf,
    /// The user slice the transient scopes go under.
    pub slice: String,
    /// Directory for the temporary profile files.
    pub profile_dir: PathBuf,
    /// Plugin id → plugin directory, substituted for `$PLUGIN_DIR`.
    pub plugin_dirs: HashMap<String, PathBuf>,
}

impl SandboxConfig {
    /// Configuration from the environment: `AIOS_SANDBOX_EXEC` (default
    /// `aios-sandbox-exec`), `AIOS_AGENT_SLICE` (default `aios-agent.slice`),
    /// `TMPDIR` for the profile files.
    pub fn from_env(plugin_dirs: impl IntoIterator<Item = (String, PathBuf)>) -> Self {
        Self {
            sandbox_exec: std::env::var_os("AIOS_SANDBOX_EXEC")
                .map_or_else(|| PathBuf::from("aios-sandbox-exec"), PathBuf::from),
            slice: std::env::var("AIOS_AGENT_SLICE")
                .unwrap_or_else(|_| "aios-agent.slice".to_owned()),
            profile_dir: std::env::temp_dir(),
            plugin_dirs: plugin_dirs.into_iter().collect(),
        }
    }
}

/// Runs each tool call in its own sandboxed, cgroup-limited process.
#[derive(Debug, Clone)]
pub struct SandboxExecutor {
    config: SandboxConfig,
}

impl SandboxExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Executor for SandboxExecutor {
    async fn run(
        &self,
        tool: &ToolSpec,
        args: &Value,
        ctx: &ExecCtx,
    ) -> Result<ToolOutput, ExecError> {
        check_inherited_fds()?;
        std::fs::create_dir_all(&ctx.workspace)
            .map_err(|e| failed("cannot create the workspace", e))?;
        let workspace = ctx
            .workspace
            .canonicalize()
            .map_err(|e| failed("cannot resolve the workspace", e))?;
        let (entry, profile) = self.expand(tool, &workspace)?;
        let profile_file = ProfileFile::write(&self.config.profile_dir, &profile)?;

        let unit = format!("aios-tool-{}", uuid::Uuid::now_v7());
        let mut cmd = Command::new("systemd-run");
        cmd.args(["--user", "--scope", "--quiet", "--collect"])
            .arg(format!("--slice={}", self.config.slice))
            .arg(format!("--unit={unit}"))
            .arg("-p")
            .arg(format!("MemoryMax={}M", profile.memory_max_mb))
            .arg("-p")
            .arg(format!("TasksMax={TASKS_MAX}"))
            .arg("--")
            .arg(&self.config.sandbox_exec)
            .arg("--profile")
            .arg(&profile_file.0)
            .arg("--")
            .args(&entry)
            .env_clear()
            .envs(
                ENV_ALLOWLIST
                    .iter()
                    .filter_map(|k| Some((k, std::env::var_os(k)?))),
            )
            // Tools find their workspace here (docs/m0/tools3.md, plugins/tools/fs).
            .env("AIOS_WORKSPACE", &workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| failed("cannot start systemd-run", e))?;
        let stderr = tokio::spawn(read_capped(child.stderr.take()));
        let (Some(stdout), Some(stdin)) = (child.stdout.take(), child.stdin.take()) else {
            return Err(ExecError::Failed("tool pipes missing".into()));
        };

        let called = tokio::time::timeout(ctx.timeout, call_tool(stdout, stdin, tool, args)).await;
        // stdin is closed once call_tool has returned (or was dropped).
        let status = match &called {
            Err(_) => {
                terminate(&unit, &mut child).await;
                None
            }
            Ok(_) => match tokio::time::timeout(EXIT_GRACE, child.wait()).await {
                Ok(status) => status.ok(),
                Err(_) => {
                    terminate(&unit, &mut child).await;
                    None
                }
            },
        };
        drop(profile_file);
        log_stderr(tool, &unit, stderr.await.unwrap_or_default());

        match called {
            Err(_) => Err(ExecError::Timeout),
            Ok(Ok(result)) => Ok(output(tool, &result)),
            Ok(Err(reason)) => Err(classify(tool, &unit, status, &reason)),
        }
    }
}

impl SandboxExecutor {
    /// Substitutes `$WORKSPACE` and `$PLUGIN_DIR` in the entry and the
    /// profile. The plugin directory becomes readable when the entry uses it.
    fn expand(
        &self,
        tool: &ToolSpec,
        workspace: &Path,
    ) -> Result<(Vec<String>, SandboxProfile), ExecError> {
        let workspace = to_str(workspace)?;
        let plugin_dir = self
            .config
            .plugin_dirs
            .get(&tool.plugin_id)
            .map(|d| to_str(d).map(str::to_owned))
            .transpose()?;
        let subst = |s: &String| -> Result<String, ExecError> {
            let s = expand_var(s, "$WORKSPACE", workspace);
            if s.starts_with("$PLUGIN_DIR") {
                let dir = plugin_dir.as_deref().ok_or_else(|| {
                    ExecError::Failed(format!("unknown plugin directory for {}", tool.plugin_id))
                })?;
                return Ok(expand_var(&s, "$PLUGIN_DIR", dir));
            }
            Ok(s)
        };
        let entry = tool
            .entry
            .iter()
            .map(subst)
            .collect::<Result<Vec<_>, _>>()?;
        let paths = |v: &[String]| v.iter().map(subst).collect::<Result<Vec<_>, _>>();
        let mut profile = SandboxProfile {
            ro_paths: paths(&tool.sandbox.ro_paths)?,
            rw_paths: paths(&tool.sandbox.rw_paths)?,
            allow_unix_socket: paths(&tool.sandbox.allow_unix_socket)?,
            ..tool.sandbox.clone()
        };
        if tool.entry.iter().any(|a| a.starts_with("$PLUGIN_DIR"))
            && let Some(dir) = plugin_dir
        {
            profile.ro_paths.push(dir);
        }
        if entry.is_empty() {
            return Err(ExecError::Failed(format!("{} has no entry", tool.name)));
        }
        Ok((entry, profile))
    }
}

/// `var` or `var/...` → `value` or `value/...`; anything else is unchanged.
fn expand_var(s: &str, var: &str, value: &str) -> String {
    match s.strip_prefix(var) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{value}{rest}"),
        _ => s.to_owned(),
    }
}

fn to_str(path: &Path) -> Result<&str, ExecError> {
    path.to_str()
        .ok_or_else(|| ExecError::Failed(format!("path is not UTF-8: {}", path.display())))
}

fn failed(what: &str, e: impl std::fmt::Display) -> ExecError {
    ExecError::Failed(format!("{what}: {e}"))
}

/// Fails closed when this process holds an fd >= 3 without `O_CLOEXEC`:
/// the tool would inherit it past Landlock (S-14 L-3).
// ponytail: check-then-spawn race with fds created concurrently by non-std
// code; closing fds in the child needs `pre_exec` (unsafe, forbidden here).
fn check_inherited_fds() -> Result<(), ExecError> {
    let dir = std::fs::read_dir("/proc/self/fd")
        .map_err(|e| failed("cannot list open file descriptors", e))?;
    let leaked: Vec<u32> = dir
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|fd| *fd >= 3)
        .filter(|fd| {
            // A fd closed meanwhile has no fdinfo; it cannot leak.
            let Ok(info) = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")) else {
                return false;
            };
            info.lines()
                .find_map(|l| l.strip_prefix("flags:"))
                .and_then(|f| u32::from_str_radix(f.trim(), 8).ok())
                .is_some_and(|flags| flags & O_CLOEXEC == 0)
        })
        .collect();
    if leaked.is_empty() {
        return Ok(());
    }
    tracing::error!(
        ?leaked,
        "file descriptors without O_CLOEXEC would leak into the tool"
    );
    Err(ExecError::Failed(
        "refusing to run: inherited file descriptors without O_CLOEXEC".into(),
    ))
}

/// The JSON profile for `aios-sandbox-exec`, mode 0600, removed on drop.
struct ProfileFile(PathBuf);

impl ProfileFile {
    fn write(dir: &Path, profile: &SandboxProfile) -> Result<Self, ExecError> {
        let path = dir.join(format!("aios-profile-{}.json", uuid::Uuid::now_v7()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| failed("cannot create the sandbox profile", e))?;
        // Removed on drop, also when the write below fails.
        let guard = Self(path);
        let body =
            serde_json::to_vec(profile).map_err(|e| failed("cannot encode the profile", e))?;
        file.write_all(&body)
            .map_err(|e| failed("cannot write the sandbox profile", e))?;
        Ok(guard)
    }
}

impl Drop for ProfileFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// `initialize` → one `tools/call` → close. The JSON form of the result.
async fn call_tool(
    stdout: tokio::process::ChildStdout,
    stdin: tokio::process::ChildStdin,
    tool: &ToolSpec,
    args: &Value,
) -> Result<Value, String> {
    let client = ().serve((stdout, stdin)).await.map_err(|e| e.to_string())?;
    let mut params = CallToolRequestParams::new(tool.name.clone());
    params.arguments = args.as_object().cloned();
    let result = client.call_tool(params).await.map_err(|e| e.to_string());
    let _ = client.cancel().await;
    serde_json::to_value(result?).map_err(|e| e.to_string())
}

/// SIGTERM to every process of the scope, then SIGKILL.
async fn terminate(unit: &str, child: &mut Child) {
    kill_unit(unit, "SIGTERM").await;
    if tokio::time::timeout(TERM_GRACE, child.wait())
        .await
        .is_err()
    {
        kill_unit(unit, "SIGKILL").await;
        let _ = child.start_kill();
        let _ = child.wait().await;
    }
}

async fn kill_unit(unit: &str, signal: &str) {
    let status = Command::new("systemctl")
        .args(["--user", "kill", "--signal", signal, unit])
        .env_clear()
        .envs(
            ENV_ALLOWLIST
                .iter()
                .filter_map(|k| Some((k, std::env::var_os(k)?))),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    if !matches!(status, Ok(s) if s.success()) {
        // The scope may be gone already (or not registered yet).
        tracing::debug!(unit, signal, ?status, "systemctl kill did not succeed");
    }
}

/// Reads up to [`STDERR_CAP`] bytes, then drains the rest so the child never
/// blocks on a full pipe.
async fn read_capped(stderr: Option<tokio::process::ChildStderr>) -> Vec<u8> {
    let Some(mut stderr) = stderr else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    let mut buf = [0u8; 8192];
    while let Ok(n @ 1..) = stderr.read(&mut buf).await {
        let room = STDERR_CAP.saturating_sub(kept.len());
        kept.extend_from_slice(&buf[..n.min(room)]);
    }
    kept
}

fn log_stderr(tool: &ToolSpec, unit: &str, stderr: Vec<u8>) {
    if stderr.is_empty() {
        return;
    }
    if tool.result_privacy == Privacy::Secret {
        // Never log what a secret-labelled tool prints.
        tracing::warn!(tool = %tool.name, unit, bytes = stderr.len(), "tool stderr (secret, not logged)");
    } else {
        let text = String::from_utf8_lossy(&stderr);
        tracing::warn!(tool = %tool.name, unit, stderr = %text, "tool stderr");
    }
}

/// Labels: the tool may raise privacy and mark its result external, never
/// lower either.
fn output(tool: &ToolSpec, result: &Value) -> ToolOutput {
    let meta = &result["_meta"];
    let privacy = serde_json::from_value::<Privacy>(meta["privacy"].clone())
        .map_or(tool.result_privacy, |p| p.max(tool.result_privacy));
    let provenance = if meta["provenance"] == "external" {
        Provenance::External
    } else {
        tool.result_provenance
    };
    ToolOutput {
        content: match &result["content"] {
            Value::Null => json!([]),
            c => c.clone(),
        },
        is_error: result["isError"].as_bool().unwrap_or(false),
        privacy,
        provenance,
    }
}

/// Maps a failed MCP exchange to an [`ExecError`]. The client only gets the
/// exit code; the MCP error (which may carry tool output) is logged.
fn classify(tool: &ToolSpec, unit: &str, status: Option<ExitStatus>, reason: &str) -> ExecError {
    if tool.result_privacy == Privacy::Secret {
        tracing::warn!(tool = %tool.name, unit, ?status, "tool call failed");
    } else {
        tracing::warn!(tool = %tool.name, unit, ?status, reason, "tool call failed");
    }
    match status.and_then(|s| s.code()) {
        Some(code @ 64..=79) => ExecError::Sandbox(code),
        Some(code) => ExecError::Failed(format!("tool server exited with code {code}")),
        None => ExecError::Failed("tool server did not answer".into()),
    }
}
