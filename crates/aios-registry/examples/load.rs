//! Example: load a registry and print the tools it provides.
//!
//! ```text
//! cargo run -p aios-registry --example load -- --core <dir> [--vendor <dir>] [--local <dir>] --trust <dir>
//! ```
//!
//! Every directory can also be given through the environment variables
//! `AIOS_REGISTRY_CORE_DIR`, `AIOS_REGISTRY_VENDOR_DIR`,
//! `AIOS_REGISTRY_LOCAL_DIR` and `AIOS_REGISTRY_TRUST_DIR`.

use std::path::PathBuf;

use anyhow::{Context, Result};

use aios_registry::{Registry, RegistryConfig};
use aios_types::Risk;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{}", usage());
        return Ok(());
    }

    let config = parse_args(&args)?;

    let (registry, report) = Registry::load(&config);

    println!("plugins: {}", registry.plugins().count());
    for plugin in registry.plugins() {
        println!(
            "  {} {}  trust={}  runtime={}  inferred_risk={}",
            plugin.id,
            plugin.version,
            plugin.trust,
            plugin.runtime,
            risk_name(plugin.risk)
        );
    }

    println!("tools: {}", registry.tools().count());
    for tool in registry.tools() {
        let trust = registry
            .plugin(&tool.plugin_id)
            .map_or("-", |plugin| plugin.trust.as_str());
        println!(
            "  {}  risk={}  plugin={}  trust={}  always_confirm={}  system_scope={}  timeout_ms={}",
            tool.name,
            risk_name(tool.risk),
            tool.plugin_id,
            trust,
            tool.always_confirm,
            tool.system_scope,
            tool.timeout_ms
        );
    }

    if !report.rejected.is_empty() {
        println!("errors: {}", report.rejected.len());
        for (path, reason) in &report.rejected {
            println!("  {}: {reason}", path.display());
        }
    }

    Ok(())
}

fn usage() -> String {
    "usage: load --core <dir> [--vendor <dir>] [--local <dir>] --trust <dir>\n".to_string()
}

fn parse_args(args: &[String]) -> Result<RegistryConfig> {
    let mut core: Option<PathBuf> = None;
    let mut vendor: Option<PathBuf> = None;
    let mut local: Option<PathBuf> = None;
    let mut trust: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        let value = args
            .get(index + 1)
            .with_context(|| format!("{arg} needs a directory"))?;
        match arg {
            "--core" => core = Some(PathBuf::from(value)),
            "--vendor" => vendor = Some(PathBuf::from(value)),
            "--local" => local = Some(PathBuf::from(value)),
            "--trust" => trust = Some(PathBuf::from(value)),
            other => anyhow::bail!("unknown option {other}"),
        }
        index += 2;
    }

    Ok(RegistryConfig {
        core_dir: core
            .or_else(|| env_dir("AIOS_REGISTRY_CORE_DIR"))
            .unwrap_or_else(|| PathBuf::from("core")),
        vendor_dir: vendor
            .or_else(|| env_dir("AIOS_REGISTRY_VENDOR_DIR"))
            .unwrap_or_else(|| PathBuf::from("vendor")),
        local_dir: local
            .or_else(|| env_dir("AIOS_REGISTRY_LOCAL_DIR"))
            .unwrap_or_else(|| PathBuf::from("local")),
        trust_dir: trust
            .or_else(|| env_dir("AIOS_REGISTRY_TRUST_DIR"))
            .unwrap_or_else(|| PathBuf::from("trust")),
    })
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// The name of a risk level.
fn risk_name(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "low",
        Risk::Medium => "medium",
        Risk::High => "high",
    }
}
