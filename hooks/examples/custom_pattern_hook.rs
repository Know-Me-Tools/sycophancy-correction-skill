//! Example: Custom Pattern Hook
//!
//! Injects a domain-specific sycophancy pattern at runtime without modifying
//! the core pattern library. Useful when your evaluation domain has signals
//! that the canonical S-01..S-08 set does not cover.
//!
//! # Registration
//! ```rust
//! executor.hooks_mut().register(DomainPatternHook::new(
//!     "SP-01",
//!     r"(?i)\bimpressive\s+architecture\b",
//!     sycophancy_core::skill::types::Severity::Medium,
//!     "Technical flattery specific to architecture reviews",
//! ));
//! ```

use async_trait::async_trait;
use regex::Regex;
use sycophancy_core::{
    hooks::{Hook, HookContext, HookMutation, HookResult},
    skill::types::{HeuristicMatch, Severity, SkillInput},
};

pub struct DomainPatternHook {
    id:       String,
    pattern:  Regex,
    severity: Severity,
    rationale: String,
}

impl DomainPatternHook {
    pub fn new(
        id:        impl Into<String>,
        pattern:   &str,
        severity:  Severity,
        rationale: impl Into<String>,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            id:       id.into(),
            pattern:  Regex::new(pattern)?,
            severity,
            rationale: rationale.into(),
        })
    }
}

#[async_trait]
impl Hook for DomainPatternHook {
    fn name(&self) -> &str { "example.domain_pattern" }

    /// Run before detection so we can normalize content (optional)
    async fn before_detect(
        &self,
        _ctx:   &mut HookContext,
        _input: &SkillInput,
    ) -> HookResult {
        HookResult::Continue
    }

    /// After the core detector runs, inject any matches for our custom pattern.
    async fn after_detect(
        &self,
        ctx:    &mut HookContext,
        result: &sycophancy_core::skill::types::DetectionResult,
    ) -> HookResult {
        if !self.pattern.is_match(&result.classifications
            .iter()
            .map(|c| c.rationale.as_str())
            .collect::<Vec<_>>()
            .join(" "))
        {
            // Check original content — stored in ctx metadata by a preceding hook
            let content = ctx.get_meta("original_content").unwrap_or("").to_string();
            if self.pattern.is_match(&content) {
                ctx.log(format!("[{}] custom pattern matched", self.id));

                return HookResult::Mutate(HookMutation {
                    inject_classifications: vec![HeuristicMatch {
                        pattern_id: self.id.clone(),
                        severity:   self.severity.clone(),
                        location:   None,
                        rationale:  self.rationale.clone(),
                    }],
                    ..Default::default()
                });
            }
        }
        HookResult::Continue
    }
}
