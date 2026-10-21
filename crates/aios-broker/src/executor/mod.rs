//! How the broker runs a tool (docs/m0/broker.md §Executor).
//!
//! The real sandboxed executor is task `executor` (docs/m0/executor.md) and
//! lives next to this file; [`FakeExecutor`] returns preconfigured outputs
//! for tests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use aios_types::{Principal, ToolOutput, ToolSpec};
use async_trait::async_trait;
use serde_json::Value;

/// Runs one tool call that the broker has already authorized.
#[async_trait]
pub trait Executor: Send + Sync {
    async fn run(
        &self,
        tool: &ToolSpec,
        args: &Value,
        ctx: &ExecCtx,
    ) -> Result<ToolOutput, ExecError>;
}

/// Context of one execution, built by the broker from the session.
#[derive(Debug, Clone)]
pub struct ExecCtx {
    pub principal: Principal,
    pub session_id: String,
    pub workspace: PathBuf,
    pub timeout: Duration,
}

/// Why an execution failed. The message is returned to the MCP client, so
/// it must never carry tool output.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecError {
    #[error("tool timed out")]
    Timeout,
    #[error("sandbox failed with exit code {0}")]
    Sandbox(i32),
    #[error("tool failed: {0}")]
    Failed(String),
}

/// Test executor: returns the output configured for a tool and records
/// every call. A tool without a configured output fails.
#[derive(Debug, Default)]
pub struct FakeExecutor {
    outputs: HashMap<String, ToolOutput>,
    calls: Mutex<Vec<(String, Value)>>,
}

impl FakeExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the output returned for `tool`.
    pub fn with_output(mut self, tool: &str, output: ToolOutput) -> Self {
        self.outputs.insert(tool.to_owned(), output);
        self
    }

    /// Every call so far, in order, as `(tool, args)`.
    pub fn calls(&self) -> Vec<(String, Value)> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl Executor for FakeExecutor {
    async fn run(
        &self,
        tool: &ToolSpec,
        args: &Value,
        _ctx: &ExecCtx,
    ) -> Result<ToolOutput, ExecError> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((tool.name.clone(), args.clone()));
        self.outputs
            .get(&tool.name)
            .cloned()
            .ok_or_else(|| ExecError::Failed(format!("no output configured for {}", tool.name)))
    }
}
