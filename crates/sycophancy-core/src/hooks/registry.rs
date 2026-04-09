//! HookRegistry — the central coordinator for the hook chain.
//!
//! ## Execution Model
//!
//! Hooks are executed in priority order (ascending). For each lifecycle event:
//!   1. Iterate hooks in priority order.
//!   2. Apply any `HookMutation` to the shared pipeline state.
//!   3. On `Abort` — halt immediately and propagate as `SkillError::HookAbort`.
//!   4. On `SkipRemaining` — stop iterating this event's chain; continue execution.
//!   5. On `Continue` — proceed to next hook.

use crate::{
    error::{SkillError, SkillResult},
    hooks::{Hook, HookContext, HookMutation, HookResult},
    skill::types::{DetectionResult, HeuristicMatch, SkillInput, SkillOutput},
};
use std::sync::Arc;
use tracing::{debug, warn};

pub struct HookRegistry {
    hooks: Vec<Arc<dyn Hook>>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self { hooks: Vec::new() }
    }
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook. The registry re-sorts by priority after each registration.
    pub fn register(&mut self, hook: impl Hook + 'static) {
        self.hooks.push(Arc::new(hook));
        self.hooks.sort_by_key(|h| h.priority());
    }

    /// Register a pre-boxed hook (for dynamic/plugin scenarios).
    pub fn register_boxed(&mut self, hook: Arc<dyn Hook>) {
        self.hooks.push(hook);
        self.hooks.sort_by_key(|h| h.priority());
    }

    pub fn hooks(&self) -> &[Arc<dyn Hook>] {
        &self.hooks
    }

    // ── Lifecycle dispatchers ─────────────────────────────────────────────────

    /// Dispatch `before_detect`. Returns the final content string (possibly mutated).
    pub async fn before_detect(
        &self,
        ctx: &mut HookContext,
        input: &SkillInput,
    ) -> SkillResult<Option<String>> {
        let mut override_content: Option<String> = None;

        for hook in self.enabled_hooks() {
            debug!(hook = hook.name(), "before_detect");
            match hook.before_detect(ctx, input).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => {
                    apply_metadata(ctx, &m);
                    if m.override_content.is_some() {
                        override_content = m.override_content;
                    }
                }
            }
        }
        Ok(override_content)
    }

    /// Dispatch `after_detect`. Returns any injected classifications + optional score override.
    pub async fn after_detect(
        &self,
        ctx: &mut HookContext,
        result: &DetectionResult,
    ) -> SkillResult<(Vec<HeuristicMatch>, Option<f32>)> {
        let mut injected: Vec<HeuristicMatch> = vec![];
        let mut force_score: Option<f32> = None;

        for hook in self.enabled_hooks() {
            debug!(hook = hook.name(), "after_detect");
            match hook.after_detect(ctx, result).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => {
                    apply_metadata(ctx, &m);
                    injected.extend(m.inject_classifications);
                    if m.force_score.is_some() {
                        force_score = m.force_score;
                    }
                }
            }
        }
        Ok((injected, force_score))
    }

    /// Dispatch `on_classify` for a single match.
    pub async fn on_classify(
        &self,
        ctx: &mut HookContext,
        classification: &HeuristicMatch,
    ) -> SkillResult<()> {
        for hook in self.enabled_hooks() {
            match hook.on_classify(ctx, classification).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => apply_metadata(ctx, &m),
            }
        }
        Ok(())
    }

    /// Dispatch `on_score`.
    pub async fn on_score(&self, ctx: &mut HookContext, score: f32) -> SkillResult<()> {
        for hook in self.enabled_hooks() {
            match hook.on_score(ctx, score).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => apply_metadata(ctx, &m),
            }
        }
        Ok(())
    }

    /// Dispatch `before_correct`.
    pub async fn before_correct(
        &self,
        ctx: &mut HookContext,
        detection: &DetectionResult,
    ) -> SkillResult<()> {
        for hook in self.enabled_hooks() {
            match hook.before_correct(ctx, detection).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => apply_metadata(ctx, &m),
            }
        }
        Ok(())
    }

    /// Dispatch `after_correct`. Returns an optional corrected artifact override.
    pub async fn after_correct(
        &self,
        ctx: &mut HookContext,
        corrected: &str,
        pass: u32,
    ) -> SkillResult<Option<String>> {
        let mut override_artifact: Option<String> = None;

        for hook in self.enabled_hooks() {
            match hook.after_correct(ctx, corrected, pass).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => {
                    apply_metadata(ctx, &m);
                    if m.override_corrected.is_some() {
                        override_artifact = m.override_corrected;
                    }
                }
            }
        }
        Ok(override_artifact)
    }

    /// Dispatch `before_validate`. Returns an optional corrected artifact override.
    pub async fn before_validate(
        &self,
        ctx: &mut HookContext,
        corrected: &str,
    ) -> SkillResult<Option<String>> {
        let mut override_artifact: Option<String> = None;

        for hook in self.enabled_hooks() {
            match hook.before_validate(ctx, corrected).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => {
                    apply_metadata(ctx, &m);
                    if m.override_corrected.is_some() {
                        override_artifact = m.override_corrected;
                    }
                }
            }
        }
        Ok(override_artifact)
    }

    /// Dispatch `on_complete`.
    pub async fn on_complete(
        &self,
        ctx: &mut HookContext,
        output: &SkillOutput,
    ) -> SkillResult<()> {
        for hook in self.enabled_hooks() {
            match hook.on_complete(ctx, output).await {
                HookResult::Continue => {}
                HookResult::SkipRemaining => break,
                HookResult::Abort { reason } => return Err(SkillError::HookAbort { reason }),
                HookResult::Mutate(m) => apply_metadata(ctx, &m),
            }
        }
        Ok(())
    }

    /// Dispatch `on_error`.
    pub async fn on_error(&self, ctx: &mut HookContext, error: &str) {
        for hook in self.enabled_hooks() {
            if let HookResult::Abort { reason } = hook.on_error(ctx, error).await {
                warn!(
                    hook = hook.name(),
                    reason, "on_error hook attempted abort — ignored"
                );
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn enabled_hooks(&self) -> impl Iterator<Item = &Arc<dyn Hook>> {
        self.hooks.iter().filter(|h| h.enabled())
    }
}

fn apply_metadata(ctx: &mut HookContext, mutation: &HookMutation) {
    for (k, v) in &mutation.metadata {
        ctx.set_meta(k.clone(), v.clone());
    }
}
