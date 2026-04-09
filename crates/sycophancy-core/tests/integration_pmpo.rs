use async_trait::async_trait;
use sycophancy_core::{
    config::SkillConfig,
    hooks::{Hook, HookContext, HookMutation, HookRegistry, HookResult},
    pmpo::PmpoExecutor,
    skill::corrector::LlmClient,
    CorrectionMode, InputContext, SkillError, SkillInput, Strictness, TargetType,
};

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
    ) -> sycophancy_core::SkillResult<String> {
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
async fn public_api_detect_only_keeps_report_only_for_critical_patterns() {
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
}

#[tokio::test]
async fn public_api_rewrite_rejects_completion_without_reasoning_block() {
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
async fn public_api_reflect_phase_runs_before_validate_hook() {
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
