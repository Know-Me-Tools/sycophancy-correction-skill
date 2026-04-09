//! Example: Prometheus AGS Audit Hook
//!
//! Writes skill execution records to the surreal-memory-server using the
//! invoking agent's DID namespace. Replaces the built-in AuditHook when
//! running inside a UAR pipeline.
//!
//! Requires: surreal-memory-server MCP running on localhost:8080
//!
//! # Registration
//! ```rust
//! executor.hooks_mut().register(PrometheusAuditHook::new(
//!     "http://localhost:8080",
//!     "did:prometheus:skills",
//! ));
//! ```

use async_trait::async_trait;
use sycophancy_core::{
    hooks::{Hook, HookContext, HookResult},
    skill::types::SkillOutput,
};
use serde_json::json;

pub struct PrometheusAuditHook {
    memory_server_url: String,
    namespace_did:     String,
}

impl PrometheusAuditHook {
    pub fn new(
        memory_server_url: impl Into<String>,
        namespace_did:     impl Into<String>,
    ) -> Self {
        Self {
            memory_server_url: memory_server_url.into(),
            namespace_did:     namespace_did.into(),
        }
    }
}

#[async_trait]
impl Hook for PrometheusAuditHook {
    fn name(&self)     -> &str { "prometheus.audit" }
    fn priority(&self) -> i32  { 200 } // runs last, after builtin.audit

    async fn on_complete(
        &self,
        ctx:    &mut HookContext,
        output: &SkillOutput,
    ) -> HookResult {
        let record = json!({
            "table": "skill_executions",
            "data": {
                "execution_id":   ctx.execution_id.to_string(),
                "skill_id":       "sycophancy.correction",
                "agent_did":      ctx.agent_did.as_deref().unwrap_or("unknown"),
                "namespace":      &self.namespace_did,
                "score":          output.sycophancy_score,
                "pattern_count":  output.classifications.len(),
                "corrected":      output.corrected_artifact.is_some(),
                "passes":         output.audit_trail.passes,
                "timestamp":      chrono::Utc::now().to_rfc3339(),
            }
        });

        // In production: POST to surreal-memory-server REST endpoint
        // POST {memory_server_url}/api/memory
        // Body: { "key": "execution:{execution_id}", "value": record, "shared": false }
        //
        // Stubbed below — replace with reqwest or hyper call.
        let _ = (&self.memory_server_url, &record);
        tracing::debug!(
            execution_id = %ctx.execution_id,
            server       = &self.memory_server_url,
            "surreal-memory-server audit write (stubbed)"
        );

        ctx.log(format!(
            "PrometheusAuditHook: record queued for {}",
            self.memory_server_url
        ));

        HookResult::Continue
    }
}

// ── UAR Cedar gate ────────────────────────────────────────────────────────────

/// Optional hook that checks a Cedar policy before allowing full_restructure mode.
/// Implements the governance rule from the skill spec:
///   full_restructure requires role "architect" in the agent's Cedar principal.
pub struct CedarGovernanceHook {
    pub require_architect_for_restructure: bool,
}

#[async_trait]
impl Hook for CedarGovernanceHook {
    fn name(&self) -> &str { "prometheus.cedar_governance" }
    fn priority(&self) -> i32 { -200 } // runs before everything

    async fn before_detect(
        &self,
        ctx:   &mut HookContext,
        input: &sycophancy_core::skill::types::SkillInput,
    ) -> HookResult {
        use sycophancy_core::skill::types::CorrectionMode;

        if self.require_architect_for_restructure
            && input.correction_mode == CorrectionMode::FullRestructure
        {
            let has_architect_role = ctx
                .get_meta("cedar.roles")
                .map(|r| r.contains("architect"))
                .unwrap_or(false);

            if !has_architect_role {
                return HookResult::Abort {
                    reason: "Cedar policy violation: full_restructure mode requires \
                             the 'architect' role. Denied."
                        .into(),
                };
            }
        }

        HookResult::Continue
    }
}
