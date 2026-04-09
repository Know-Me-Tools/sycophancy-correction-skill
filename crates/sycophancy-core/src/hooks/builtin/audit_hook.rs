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
/// Backends: `stdout` (default), `file`, `surreal_db`.
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

        match &self.backend {
            AuditBackend::Stdout => {
                println!("{}", serde_json::to_string(&record).unwrap_or_default());
            }
            AuditBackend::File => {
                // In production: append to a rotation-aware log file
                eprintln!(
                    "[AUDIT] {}",
                    serde_json::to_string(&record).unwrap_or_default()
                );
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
