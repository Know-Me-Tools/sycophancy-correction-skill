use crate::{
    hooks::{Hook, HookContext, HookResult},
    skill::types::{DetectionResult, HeuristicMatch, SkillInput, SkillOutput},
};
use async_trait::async_trait;
use tracing::{debug, info, warn};

/// Built-in hook that emits structured tracing events at every lifecycle point.
///
/// Enable via `skill.toml`:
/// ```toml
/// [hooks]
/// tracing_hook = true
/// ```
pub struct TracingHook {
    pub log_classifications: bool,
}

impl Default for TracingHook {
    fn default() -> Self {
        Self {
            log_classifications: true,
        }
    }
}

#[async_trait]
impl Hook for TracingHook {
    fn name(&self) -> &str {
        "builtin.tracing"
    }
    fn priority(&self) -> i32 {
        -100
    } // runs before all other hooks

    async fn before_detect(&self, ctx: &mut HookContext, input: &SkillInput) -> HookResult {
        info!(
            execution_id = %ctx.execution_id,
            target        = ?input.target,
            mode          = ?input.correction_mode,
            strictness    = ?input.strictness,
            content_len   = input.content.len(),
            "skill execution started"
        );
        HookResult::Continue
    }

    async fn after_detect(&self, ctx: &mut HookContext, result: &DetectionResult) -> HookResult {
        info!(
            execution_id  = %ctx.execution_id,
            score         = result.sycophancy_score,
            match_count   = result.classifications.len(),
            has_critical  = result.has_critical,
            mandatory     = result.correction_mandatory,
            "detection complete"
        );
        HookResult::Continue
    }

    async fn on_classify(&self, ctx: &mut HookContext, c: &HeuristicMatch) -> HookResult {
        if self.log_classifications {
            debug!(
                execution_id = %ctx.execution_id,
                pattern_id   = &c.pattern_id,
                severity     = ?c.severity,
                rationale    = &c.rationale,
                "pattern matched"
            );
        }
        HookResult::Continue
    }

    async fn on_score(&self, ctx: &mut HookContext, score: f32) -> HookResult {
        if score >= 0.6 {
            warn!(execution_id = %ctx.execution_id, score, "high sycophancy score — correction mandatory");
        } else {
            debug!(execution_id = %ctx.execution_id, score, "sycophancy scored");
        }
        HookResult::Continue
    }

    async fn before_correct(&self, ctx: &mut HookContext, result: &DetectionResult) -> HookResult {
        info!(
            execution_id = %ctx.execution_id,
            pass         = ctx.pass,
            score        = result.sycophancy_score,
            "starting correction pass"
        );
        HookResult::Continue
    }

    async fn after_correct(&self, ctx: &mut HookContext, corrected: &str, pass: u32) -> HookResult {
        info!(
            execution_id  = %ctx.execution_id,
            pass,
            corrected_len = corrected.len(),
            "correction pass complete"
        );
        HookResult::Continue
    }

    async fn on_complete(&self, ctx: &mut HookContext, output: &SkillOutput) -> HookResult {
        info!(
            execution_id = %ctx.execution_id,
            final_score  = output.sycophancy_score,
            passes       = output.audit_trail.passes,
            "skill execution complete"
        );
        HookResult::Continue
    }

    async fn on_error(&self, ctx: &mut HookContext, error: &str) -> HookResult {
        warn!(execution_id = %ctx.execution_id, error, "skill execution error");
        HookResult::Continue
    }
}
