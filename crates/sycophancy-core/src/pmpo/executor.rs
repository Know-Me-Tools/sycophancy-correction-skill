//! PMPO Executor — Specify → Plan → Execute → Reflect → Persist → Terminate
//!
//! This is the top-level orchestration layer. It wires together the Detector,
//! Corrector, HookRegistry, and produces a final `SkillOutput`.
//!
//! The executor is intentionally imperative: each phase is a discrete async
//! step, making it easy to instrument, test, and extend.

use crate::{
    config::SkillConfig,
    error::{SkillError, SkillResult},
    hooks::{HookContext, HookRegistry},
    skill::{
        corrector::{Corrector, LlmClient},
        detector::Detector,
        types::{
            AuditTrail, CorrectionMode, DetectionResult, InputContext, SkillInput, SkillOutput,
            Strictness, TargetType,
        },
    },
};
use chrono::Utc;
use tracing::info;

pub struct PmpoExecutor {
    config: SkillConfig,
    detector: Detector,
    corrector: Corrector,
    hooks: HookRegistry,
}

impl PmpoExecutor {
    pub fn new(config: SkillConfig, hooks: HookRegistry) -> Self {
        let detector = Detector::new(config.clone());
        let corrector = Corrector::new(config.llm.clone());
        Self {
            config,
            detector,
            corrector,
            hooks,
        }
    }

    /// Expose hook registry for late registration (e.g. from MCP server init)
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    pub async fn execute(
        &self,
        input: SkillInput,
        llm_client: &dyn LlmClient,
        agent_did: Option<String>,
    ) -> SkillResult<SkillOutput> {
        let mut ctx = HookContext::new(agent_did.clone());
        let started = Utc::now();

        // ── Phase 1: Specify ─────────────────────────────────────────────────
        // Resolve content, potentially mutated by a before_detect hook
        // (e.g. a normalization hook that strips HTML before detection)
        info!(execution_id = %ctx.execution_id, "Phase 1: Specify");
        let content = match self.hooks.before_detect(&mut ctx, &input).await? {
            Some(overridden) => overridden,
            None => input.content.clone(),
        };

        // ── Phase 2: Plan ────────────────────────────────────────────────────
        // (Implicit in the detector/corrector architecture; no separate step needed.)
        info!(execution_id = %ctx.execution_id, "Phase 2: Plan");

        // ── Phase 3: Execute — Detection ─────────────────────────────────────
        info!(execution_id = %ctx.execution_id, "Phase 3: Execute — detection");
        let mut detection = self.detector.detect(
            &content,
            &input.context.prior_completions,
            &input.strictness,
        );

        // Fire on_classify for each match
        for classification in &detection.classifications {
            self.hooks.on_classify(&mut ctx, classification).await?;
        }

        // Fire on_score
        self.hooks
            .on_score(&mut ctx, detection.sycophancy_score)
            .await?;

        // after_detect may inject additional classifications or force a score
        let (injected, force_score) = self.hooks.after_detect(&mut ctx, &detection).await?;
        if !injected.is_empty() {
            detection.classifications.extend(injected);
        }
        if let Some(s) = force_score {
            detection.sycophancy_score = s;
        }

        // ── Phase 4: Reflect — correction decision ───────────────────────────
        info!(execution_id = %ctx.execution_id, "Phase 4: Reflect — correction decision");
        let mode = self.resolve_mode(&input.correction_mode, &detection);

        let (corrected_artifact, passes) = if mode == CorrectionMode::DetectOnly
            || is_clean_result(&detection, self.config.correction.clean_threshold)
        {
            (None, 0u32)
        } else {
            self.run_correction_loop(&content, &input, &detection, &mode, llm_client, &mut ctx)
                .await?
        };

        // ── Phase 5: Persist ─────────────────────────────────────────────────
        info!(execution_id = %ctx.execution_id, "Phase 5: Persist");
        let delta_summary = self.build_delta_summary(&detection, &corrected_artifact);

        let output = SkillOutput {
            sycophancy_score: detection.sycophancy_score,
            classifications: detection.classifications.clone(),
            corrected_artifact,
            delta_summary,
            audit_trail: AuditTrail {
                skill_id: self.config.skill.id.clone(),
                version: self.config.skill.version.clone(),
                strictness: input.strictness.as_str().into(),
                executed_at: started,
                agent_did,
                passes,
                hook_log: ctx.log.clone(),
            },
        };

        self.validate_output(&input, &content, &mode, &output)?;

        self.hooks.on_complete(&mut ctx, &output).await?;

        // ── Phase 6: Terminate ───────────────────────────────────────────────
        info!(execution_id = %ctx.execution_id, "Phase 6: Terminate");
        Ok(output)
    }

    pub async fn execute_reflect_phase(
        &self,
        content: String,
        llm_client: &dyn LlmClient,
        agent_did: Option<String>,
        correct: bool,
    ) -> SkillResult<SkillOutput> {
        let input = SkillInput {
            target: TargetType::Completion,
            content,
            context: InputContext {
                original_intent: None,
                prior_completions: Vec::new(),
                evaluation_domain: Some("pmpo_reflect_phase".into()),
            },
            correction_mode: if correct {
                CorrectionMode::Rewrite
            } else {
                CorrectionMode::DetectOnly
            },
            strictness: Strictness::Strict,
        };

        let mut ctx = HookContext::new(agent_did.clone());
        let started = Utc::now();

        info!(execution_id = %ctx.execution_id, "Phase 1: Specify");
        let content = match self.hooks.before_detect(&mut ctx, &input).await? {
            Some(overridden) => overridden,
            None => input.content.clone(),
        };

        info!(execution_id = %ctx.execution_id, "Phase 2: Plan");

        info!(execution_id = %ctx.execution_id, "Phase 3: Execute — detection");
        let mut detection = self.detector.detect(
            &content,
            &input.context.prior_completions,
            &input.strictness,
        );

        for classification in &detection.classifications {
            self.hooks.on_classify(&mut ctx, classification).await?;
        }

        self.hooks
            .on_score(&mut ctx, detection.sycophancy_score)
            .await?;

        let (injected, force_score) = self.hooks.after_detect(&mut ctx, &detection).await?;
        if !injected.is_empty() {
            detection.classifications.extend(injected);
        }
        if let Some(score) = force_score {
            detection.sycophancy_score = score;
        }

        info!(execution_id = %ctx.execution_id, "Phase 4: Reflect — correction decision");
        let (corrected_artifact, passes) = if !correct
            || is_clean_result(&detection, self.config.correction.clean_threshold)
        {
            (None, 0u32)
        } else {
            ctx.pass = 1;
            self.hooks.before_correct(&mut ctx, &detection).await?;
            let corrected = self
                .corrector
                .correct_reflect_phase(llm_client, &content)
                .await?;
            let corrected = match self.hooks.after_correct(&mut ctx, &corrected, 1).await? {
                Some(overridden) => overridden,
                None => corrected,
            };
            let final_corrected = match self.hooks.before_validate(&mut ctx, &corrected).await? {
                Some(overridden) => overridden,
                None => corrected,
            };
            (Some(final_corrected), 1u32)
        };

        info!(execution_id = %ctx.execution_id, "Phase 5: Persist");
        let delta_summary = self.build_delta_summary(&detection, &corrected_artifact);
        let output = SkillOutput {
            sycophancy_score: detection.sycophancy_score,
            classifications: detection.classifications.clone(),
            corrected_artifact,
            delta_summary,
            audit_trail: AuditTrail {
                skill_id: self.config.skill.id.clone(),
                version: self.config.skill.version.clone(),
                strictness: input.strictness.as_str().into(),
                executed_at: started,
                agent_did,
                passes,
                hook_log: ctx.log.clone(),
            },
        };

        self.validate_output(&input, &content, &input.correction_mode, &output)?;
        self.hooks.on_complete(&mut ctx, &output).await?;

        info!(execution_id = %ctx.execution_id, "Phase 6: Terminate");
        Ok(output)
    }

    // ── Correction Loop ───────────────────────────────────────────────────────

    async fn run_correction_loop(
        &self,
        content: &str,
        input: &SkillInput,
        detection: &DetectionResult,
        mode: &CorrectionMode,
        client: &dyn LlmClient,
        ctx: &mut HookContext,
    ) -> SkillResult<(Option<String>, u32)> {
        let max_passes = self.config.correction.max_passes;
        let mut passes = 0u32;
        let mut current_content = content.to_string();

        loop {
            if passes >= max_passes {
                return Err(SkillError::CorrectionFailed {
                    passes,
                    final_score: detection.sycophancy_score,
                });
            }

            ctx.pass = passes + 1;
            self.hooks.before_correct(ctx, detection).await?;

            let corrected = self
                .corrector
                .correct(client, &current_content, &input.target, detection, mode)
                .await?;

            passes += 1;

            // Hooks may override the corrected artifact (e.g. post-processing hooks)
            let final_corrected = match self.hooks.after_correct(ctx, &corrected, passes).await? {
                Some(overridden) => overridden,
                None => corrected,
            };

            // In FullRestructure mode: validate the correction passes a second detection
            if *mode == CorrectionMode::FullRestructure {
                let final_corrected =
                    match self.hooks.before_validate(ctx, &final_corrected).await? {
                        Some(overridden) => overridden,
                        None => final_corrected,
                    };
                let validation = self.detector.detect(
                    &final_corrected,
                    &input.context.prior_completions,
                    &input.strictness,
                );

                if validation.sycophancy_score < self.config.correction.clean_threshold {
                    return Ok((Some(final_corrected), passes));
                }

                if passes >= max_passes {
                    return Err(SkillError::CorrectionFailed {
                        passes,
                        final_score: validation.sycophancy_score,
                    });
                }

                // Still above threshold — loop if we have passes remaining
                current_content = final_corrected;
                continue;
            }

            return Ok((Some(final_corrected), passes));
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Preserve the caller-requested mode. Mandatory correction is surfaced in
    /// detection metadata rather than silently escalating detect_only into rewrite.
    fn resolve_mode(
        &self,
        requested: &CorrectionMode,
        _detection: &DetectionResult,
    ) -> CorrectionMode {
        requested.clone()
    }

    fn build_delta_summary(
        &self,
        detection: &DetectionResult,
        corrected: &Option<String>,
    ) -> String {
        if detection.classifications.is_empty() {
            return "No changes needed: score below clean threshold and no sycophancy patterns were detected.".into();
        }

        let pattern_summary: String = detection
            .classifications
            .iter()
            .map(|c| format!("[{}] {:?} — {}", c.pattern_id, c.severity, c.rationale))
            .collect::<Vec<_>>()
            .join("\n");

        if corrected.is_some() {
            format!(
                "Score: {:.2}. {} pattern(s) detected and corrected:\n{}",
                detection.sycophancy_score,
                detection.classifications.len(),
                pattern_summary
            )
        } else {
            format!(
                "Score: {:.2}. {} pattern(s) detected (detect_only mode — no correction applied):\n{}",
                detection.sycophancy_score,
                detection.classifications.len(),
                pattern_summary
            )
        }
    }

    fn validate_output(
        &self,
        input: &SkillInput,
        original_content: &str,
        mode: &CorrectionMode,
        output: &SkillOutput,
    ) -> SkillResult<()> {
        if !(0.0..=1.0).contains(&output.sycophancy_score) {
            return Err(SkillError::ValidationFailed {
                field: "sycophancy_score".into(),
                message: "must be within [0.0, 1.0]".into(),
            });
        }

        if output
            .classifications
            .iter()
            .any(|classification| classification.rationale.trim().is_empty())
        {
            return Err(SkillError::ValidationFailed {
                field: "classifications.rationale".into(),
                message: "every classification requires a non-empty rationale".into(),
            });
        }

        if output.delta_summary.trim().is_empty() {
            return Err(SkillError::ValidationFailed {
                field: "delta_summary".into(),
                message: "must describe what changed or why no changes were needed".into(),
            });
        }

        if *mode == CorrectionMode::DetectOnly && output.corrected_artifact.is_some() {
            return Err(SkillError::ValidationFailed {
                field: "corrected_artifact".into(),
                message: "must be absent in detect_only mode".into(),
            });
        }

        if matches!(
            mode,
            CorrectionMode::Rewrite | CorrectionMode::FullRestructure
        ) {
            let corrected = output.corrected_artifact.as_deref().ok_or_else(|| {
                SkillError::ValidationFailed {
                    field: "corrected_artifact".into(),
                    message: "is required when correction mode rewrites the artifact".into(),
                }
            })?;

            if corrected.trim() == original_content.trim() {
                return Err(SkillError::ValidationFailed {
                    field: "corrected_artifact".into(),
                    message: "must structurally differ from the input".into(),
                });
            }

            if input.target == TargetType::Completion
                && !is_reflect_phase_request(input)
                && !contains_reasoning_block(corrected)
            {
                return Err(SkillError::ValidationFailed {
                    field: "corrected_artifact".into(),
                    message:
                        "rewritten completions must contain a reasoning block before conclusions"
                            .into(),
                });
            }

            if is_reflect_phase_request(input) && !has_reflect_phase_structure(corrected) {
                return Err(SkillError::ValidationFailed {
                    field: "corrected_artifact".into(),
                    message: "reflect-phase corrections must follow Delta -> Root Cause -> Corrective Actions".into(),
                });
            }
        }

        if output.audit_trail.skill_id.as_str() != self.config.skill.id.as_str() {
            return Err(SkillError::ValidationFailed {
                field: "audit_trail.skill_id".into(),
                message: "must match the configured skill id".into(),
            });
        }

        if output.audit_trail.version.trim().is_empty() {
            return Err(SkillError::ValidationFailed {
                field: "audit_trail.version".into(),
                message: "must be present".into(),
            });
        }

        if output.audit_trail.strictness != input.strictness.as_str() {
            return Err(SkillError::ValidationFailed {
                field: "audit_trail.strictness".into(),
                message: "must match the invocation strictness".into(),
            });
        }

        Ok(())
    }
}

fn is_clean_result(detection: &DetectionResult, clean_threshold: f32) -> bool {
    detection.sycophancy_score < clean_threshold && detection.classifications.is_empty()
}

fn contains_reasoning_block(content: &str) -> bool {
    let normalized = content.to_lowercase();
    normalized.contains("<reasoning>") || normalized.contains("## reasoning")
}

fn is_reflect_phase_request(input: &SkillInput) -> bool {
    matches!(
        input.context.evaluation_domain.as_deref(),
        Some("pmpo_reflect_phase") | Some("reflect_phase")
    )
}

fn has_reflect_phase_structure(content: &str) -> bool {
    let normalized = content.to_lowercase();
    normalized.contains("delta")
        && normalized.contains("root cause")
        && normalized.contains("corrective actions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::{Hook, HookMutation, HookRegistry, HookResult};
    use async_trait::async_trait;

    struct StubClient {
        response: String,
    }

    #[async_trait]
    impl LlmClient for StubClient {
        async fn complete(
            &self,
            _system: &str,
            _user: &str,
            _max_tokens: u32,
        ) -> SkillResult<String> {
            Ok(self.response.clone())
        }
    }

    struct BeforeValidateOverrideHook {
        replacement: String,
    }

    #[async_trait]
    impl Hook for BeforeValidateOverrideHook {
        fn name(&self) -> &str {
            "before_validate_override"
        }

        async fn before_validate(&self, _ctx: &mut HookContext, _corrected: &str) -> HookResult {
            HookResult::Mutate(HookMutation {
                override_corrected: Some(self.replacement.clone()),
                ..Default::default()
            })
        }
    }

    fn executor_with_hooks(hooks: HookRegistry) -> PmpoExecutor {
        PmpoExecutor::new(SkillConfig::default(), hooks)
    }

    #[tokio::test]
    async fn detect_only_keeps_report_only_for_critical_patterns() {
        let executor = executor_with_hooks(HookRegistry::new());
        let client = StubClient {
            response: "<reasoning>unused</reasoning>".into(),
        };

        let input = SkillInput {
            target: TargetType::Completion,
            content: "I've successfully implemented the best design for this task.".into(),
            context: InputContext::default(),
            correction_mode: CorrectionMode::DetectOnly,
            strictness: Strictness::Standard,
        };

        let output = executor.execute(input, &client, None).await.unwrap();

        assert!(output.corrected_artifact.is_none());
        assert!(output
            .classifications
            .iter()
            .any(|m| m.pattern_id == "S-04"));
        assert!(output.delta_summary.contains("detect_only mode"));
    }

    #[tokio::test]
    async fn rewrite_rejects_completion_without_reasoning_block() {
        let executor = executor_with_hooks(HookRegistry::new());
        let client = StubClient {
            response: "Final answer without reasoning.".into(),
        };

        let input = SkillInput {
            target: TargetType::Completion,
            content: "You're right.".into(),
            context: InputContext::default(),
            correction_mode: CorrectionMode::Rewrite,
            strictness: Strictness::Standard,
        };

        let error = executor.execute(input, &client, None).await.unwrap_err();

        match error {
            SkillError::ValidationFailed { field, message } => {
                assert_eq!(field, "corrected_artifact");
                assert!(message.contains("reasoning block"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn reflect_phase_runs_before_validate_hook() {
        let mut hooks = HookRegistry::new();
        let replacement = "## 1. Delta from Spec
Missing risk analysis.

## 2. Root Cause
The response opened with success language.

## 3. Corrective Actions
Lead with deltas before conclusions."
            .to_string();
        hooks.register(BeforeValidateOverrideHook {
            replacement: replacement.clone(),
        });

        let executor = executor_with_hooks(hooks);
        let client = StubClient {
            response: "This output is still invalid.".into(),
        };

        let output = executor
            .execute_reflect_phase(
                "The implementation was successful and all requirements were met.".into(),
                &client,
                None,
                true,
            )
            .await
            .unwrap();

        assert_eq!(
            output.corrected_artifact.as_deref(),
            Some(replacement.as_str())
        );
        assert!(output
            .classifications
            .iter()
            .any(|m| m.pattern_id == "S-08"));
    }
}
