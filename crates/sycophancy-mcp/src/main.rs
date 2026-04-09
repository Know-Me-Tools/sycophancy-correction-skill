//! sycophancy-correction — MCP server entrypoint
//!
//! Usage:
//!   sycophancy-correction [--config PATH]
//!
//! Defaults to `skill.toml` in the current directory.
//! Transport: stdio (Claude Code default) or streamable HTTP (--port N).

use anyhow::Result;
use sycophancy_core::{
    config::SkillConfig,
    hooks::{
        builtin::{AuditHook, TracingHook},
        HookRegistry,
    },
    pmpo::PmpoExecutor,
};
use tracing_subscriber::{fmt, EnvFilter};

mod server;
mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr) // MCP servers must keep stdout clean for JSON-RPC
        .json()
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let config_path = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| std::path::PathBuf::from(&w[1]))
        .unwrap_or_else(|| std::path::PathBuf::from("skill.toml"));

    let config = if config_path.exists() {
        SkillConfig::from_file(&config_path)?
    } else {
        tracing::warn!("skill.toml not found, using defaults");
        SkillConfig::default()
    };

    // ── Hook Registry ─────────────────────────────────────────────────────────
    let mut hooks = HookRegistry::new();

    if config.hooks.tracing_hook {
        hooks.register(TracingHook::default());
    }
    if config.hooks.audit_hook {
        hooks.register(AuditHook::new(
            config.audit.backend.clone(),
            &config.skill.version,
        ));
    }

    // ── Executor ──────────────────────────────────────────────────────────────
    let executor = PmpoExecutor::new(config.clone(), hooks);

    // ── MCP Server ────────────────────────────────────────────────────────────
    server::serve(executor, config).await
}
