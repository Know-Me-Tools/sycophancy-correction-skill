use crate::{
    config::AuditBackend,
    hooks::{Hook, HookContext, HookResult},
    skill::types::SkillOutput,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

/// Built-in hook that writes a structured audit record on every completion.
///
/// Backends:
/// - `stdout` (default): emitted via `tracing::info!` so it lands on stderr
///   when the MCP server initializes its subscriber. Real stdout is reserved
///   for JSON-RPC traffic and must never be written to from a hook running
///   inside an MCP server (see `crates/sycophancy-mcp/src/main.rs`).
/// - `file`: currently routed to stderr with an `[AUDIT]` prefix until a
///   rotation-aware file backend lands.
/// - `surreal_db`: stubbed; intended to write through `surreal-memory-server`.
pub struct AuditHook {
    pub backend: AuditBackend,
    pub skill_version: String,
}

impl AuditHook {
    pub fn new(backend: AuditBackend, skill_version: impl Into<String>) -> Self {
        Self {
            backend,
            skill_version: skill_version.into(),
        }
    }
}

#[async_trait]
impl Hook for AuditHook {
    fn name(&self) -> &str {
        "builtin.audit"
    }
    fn priority(&self) -> i32 {
        100
    } // runs after all other hooks

    async fn on_complete(&self, ctx: &mut HookContext, output: &SkillOutput) -> HookResult {
        let record = json!({
            "event":          "skill.complete",
            "skill_id":       "sycophancy.correction",
            "version":        &self.skill_version,
            "execution_id":   ctx.execution_id.to_string(),
            "agent_did":      ctx.agent_did.as_deref().unwrap_or("unknown"),
            "timestamp":      Utc::now().to_rfc3339(),
            "score":          output.sycophancy_score,
            "match_count":    output.classifications.len(),
            "passes":         output.audit_trail.passes,
            "has_correction": output.corrected_artifact.is_some(),
            "hook_log":       &output.audit_trail.hook_log,
        });

        let payload = serde_json::to_string(&record).unwrap_or_default();
        match &self.backend {
            // NOTE: emits via `tracing` -> stderr. Writing to real stdout here
            // would corrupt the MCP JSON-RPC stream (see sycophancy-mcp/src/main.rs).
            AuditBackend::Stdout => {
                tracing::info!(
                    target: "sycophancy.audit",
                    audit = %payload,
                    "skill.complete"
                );
            }
            AuditBackend::File => {
                // In production: append to a rotation-aware log file.
                // For now, route to stderr with a distinct prefix so it never
                // lands on the JSON-RPC stdout channel.
                eprintln!("[AUDIT] {payload}");
            }
            AuditBackend::SurrealDb => {
                // In production: issue a SurrealDB CREATE statement via the surreal-memory-server
                // using the agent's DID namespace for scoping.
                // Stub for now — replace with actual surreal client call.
                tracing::debug!(
                    execution_id = %ctx.execution_id,
                    "SurrealDB audit write (stubbed)"
                );
            }
        }

        HookResult::Continue
    }

    async fn on_error(&self, ctx: &mut HookContext, error: &str) -> HookResult {
        let record = serde_json::json!({
            "event":        "skill.error",
            "skill_id":     "sycophancy.correction",
            "execution_id": ctx.execution_id.to_string(),
            "timestamp":    chrono::Utc::now().to_rfc3339(),
            "error":        error,
        });
        eprintln!(
            "[AUDIT/ERR] {}",
            serde_json::to_string(&record).unwrap_or_default()
        );
        HookResult::Continue
    }
}
