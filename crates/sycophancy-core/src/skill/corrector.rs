//! Corrector — issues LLM calls to produce corrected artifacts.
//!
//! The corrector is intentionally thin: it constructs the critic/rewrite
//! prompt templates from the spec and delegates to any conformant LLM client.
//! Swap the `LlmClient` trait impl to target a different provider.

use crate::{
    config::LlmConfig,
    error::SkillResult,
    skill::types::{CorrectionMode, DetectionResult, TargetType},
};
use async_trait::async_trait;

// ── LLM Client Abstraction ────────────────────────────────────────────────────

/// Implement this to target any LLM provider.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> SkillResult<String>;
}

// ── Corrector ─────────────────────────────────────────────────────────────────

pub struct Corrector {
    config: LlmConfig,
}

impl Corrector {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub async fn correct(
        &self,
        client: &dyn LlmClient,
        content: &str,
        target: &TargetType,
        detection: &DetectionResult,
        mode: &CorrectionMode,
    ) -> SkillResult<String> {
        if *mode == CorrectionMode::Annotate {
            return Ok(self.annotate(content, detection));
        }

        let system = match mode {
            CorrectionMode::FullRestructure => self.red_team_system(target),
            _ => self.standard_critic_system(target),
        };

        let user = self.build_user_prompt(content, detection);
        client
            .complete(&system, &user, self.config.critic_max_tokens)
            .await
    }

    // ── System Prompts ────────────────────────────────────────────────────────

    fn standard_critic_system(&self, target: &TargetType) -> String {
        let target_str = match target {
            TargetType::Prompt => "a system or user prompt",
            TargetType::Completion => "an LLM completion",
            TargetType::AgentDescriptor => "an agent YAML descriptor",
            TargetType::Pipeline => "a multi-agent pipeline configuration",
        };
        format!(
            "You are rewriting {target_str} to remove sycophantic patterns.\n\
             \n\
             Your role is adversarial review. Find and eliminate problems. \
             Do NOT validate or encourage unless directly asked.\n\
             \n\
             Rules:\n\
             - Never emit \"great question\", \"you're right\", or similar approval language.\n\
             - If agreement is appropriate, state WHY in one sentence derivable from your reasoning.\n\
             - Always output a <reasoning> block BEFORE any conclusion.\n\
             - Always surface at least one risk, trade-off, or alternative unless this is a \
               pure factual lookup.\n\
             - Do not expand scope beyond what was asked.\n\
             \n\
             Return ONLY the corrected artifact. No preamble, no explanation after."
        )
    }

    fn red_team_system(&self, target: &TargetType) -> String {
        let target_str = match target {
            TargetType::AgentDescriptor => "agent architecture descriptor",
            TargetType::Pipeline => "multi-agent pipeline",
            _ => "artifact",
        };
        format!(
            "You are a senior staff engineer whose sole function is to identify the single \
             failure mode most likely to cause this {target_str} to fail in production.\n\
             \n\
             You are not helpful. You are skeptical. You are adversarial.\n\
             \n\
             Required output structure:\n\
             1. Primary failure vector — the one thing most likely to fail\n\
             2. Secondary risks — ranked by probability × impact\n\
             3. Minimum changes required to address the primary vector\n\
             4. Corrected artifact\n\
             \n\
             FORBIDDEN:\n\
             - Do not acknowledge what is correct.\n\
             - Do not open with a positive statement.\n\
             - Do not use \"successfully\", \"effectively\", or similar unless you have \
               already addressed all failure vectors.\n\
             \n\
             If no structural failures are found, output: \"No structural failures detected\" \
             and terminate. Do not add encouragement."
        )
    }

    // ── User Prompt ───────────────────────────────────────────────────────────

    fn build_user_prompt(&self, content: &str, detection: &DetectionResult) -> String {
        let pattern_list: String = detection
            .classifications
            .iter()
            .map(|c| format!("  - [{}] {:?}: {}", c.pattern_id, c.severity, c.rationale))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "DETECTED SYCOPHANCY PATTERNS (score: {:.2}):\n{}\n\n\
             ORIGINAL CONTENT:\n---\n{}\n---\n\n\
             Rewrite the content above to eliminate all detected patterns. \
             Preserve all technical accuracy and original intent. \
             Lead with the corrected artifact.",
            detection.sycophancy_score, pattern_list, content
        )
    }

    // ── Reflect Phase Corrector ───────────────────────────────────────────────

    /// Specialized corrector for PMPO Reflect phase outputs (S-08).
    pub async fn correct_reflect_phase(
        &self,
        client: &dyn LlmClient,
        content: &str,
    ) -> SkillResult<String> {
        let system = "\
You are restructuring a PMPO Reflect phase output.

REQUIRED OUTPUT STRUCTURE — do not deviate:

## 1. Delta from Spec
What did the execution actually produce vs. what was specified?
List every gap, deviation, and omission.
If there are none, state: \"No divergence detected\" and explain why.

## 2. Root Cause
For each delta: what caused it?
(underspecified plan, tool limitation, context loss, model error)

## 3. Corrective Actions
Minimal changes required for a second attempt.
Ranked by impact. If no second attempt is needed, state why.

FORBIDDEN:
- Do not summarize what the execution did correctly
- Do not open with a positive statement
- Do not use \"successfully\", \"effectively\", or similar
  unless you have already addressed all deltas

Return ONLY the restructured Reflect output.";

        let user = format!(
            "ORIGINAL REFLECT OUTPUT:\n---\n{content}\n---\n\n\
             Restructure this to follow the mandatory Delta → Root Cause → \
             Corrective Actions format, eliminating all sycophantic opening patterns."
        );

        client
            .complete(system, &user, self.config.critic_max_tokens)
            .await
    }

    fn annotate(&self, content: &str, detection: &DetectionResult) -> String {
        let annotations = detection
            .classifications
            .iter()
            .map(|classification| {
                format!(
                    "- [{}] {}: {}",
                    classification.pattern_id,
                    classification.severity.as_str(),
                    classification.rationale
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("{content}\n\n[sycophancy_annotations]\n{annotations}")
    }
}
