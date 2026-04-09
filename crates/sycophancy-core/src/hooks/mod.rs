//! Hook system — lifecycle-driven extensibility for the sycophancy skill.
//!
//! # Implementing a Hook
//!
//! ```rust
//! use sycophancy_core::hooks::{Hook, HookContext, HookResult};
//! use sycophancy_core::DetectionResult;
//! use async_trait::async_trait;
//!
//! pub struct MyHook;
//!
//! #[async_trait]
//! impl Hook for MyHook {
//!     fn name(&self) -> &str { "my_hook" }
//!
//!     async fn after_detect(
//!         &self,
//!         ctx: &mut HookContext,
//!         result: &DetectionResult,
//!     ) -> HookResult {
//!         tracing::info!(score = result.sycophancy_score, "Detection complete");
//!         HookResult::Continue
//!     }
//! }
//! ```
//!
//! Register via `HookRegistry::register(Box::new(MyHook))`.

use crate::skill::types::{DetectionResult, HeuristicMatch, SkillInput, SkillOutput};
use async_trait::async_trait;
use std::collections::HashMap;

pub mod builtin;
pub mod context;
pub mod registry;

pub use context::HookContext;
pub use registry::HookRegistry;

// ── HookResult ────────────────────────────────────────────────────────────────

/// The return value of any hook method.
#[derive(Debug, Default)]
pub enum HookResult {
    /// Pass control to the next hook / continue normal execution.
    #[default]
    Continue,

    /// Mutate pipeline state before continuing.
    Mutate(HookMutation),

    /// Abort execution immediately. The reason is surfaced in SkillError::HookAbort.
    Abort { reason: String },

    /// Skip all remaining hooks registered for this lifecycle event.
    /// (Normal execution continues after the hook chain.)
    SkipRemaining,
}

// ── HookMutation ──────────────────────────────────────────────────────────────

/// Declarative mutations a hook can request.
///
/// The PMPO executor applies these after the hook returns, preserving a clean
/// separation between hook logic and core state.
#[derive(Debug, Default)]
pub struct HookMutation {
    /// Override the content being evaluated (e.g. normalize before detection)
    pub override_content: Option<String>,

    /// Inject additional pattern matches into the detection result
    pub inject_classifications: Vec<HeuristicMatch>,

    /// Force the sycophancy score to a specific value
    pub force_score: Option<f32>,

    /// Inject arbitrary metadata into the hook context
    pub metadata: HashMap<String, String>,

    /// Override the corrected artifact output
    pub override_corrected: Option<String>,
}

// ── Hook Trait ────────────────────────────────────────────────────────────────

#[async_trait]
pub trait Hook: Send + Sync {
    /// Unique name for this hook — used in log output and deduplication.
    fn name(&self) -> &str;

    /// Execution priority. Lower value runs first. Default: 0.
    fn priority(&self) -> i32 {
        0
    }

    /// Whether this hook is enabled. Can be toggled at runtime.
    fn enabled(&self) -> bool {
        true
    }

    // ── Lifecycle Methods ─────────────────────────────────────────────────────

    /// Called before any detection begins. Ideal for content normalization.
    async fn before_detect(&self, _ctx: &mut HookContext, _input: &SkillInput) -> HookResult {
        HookResult::Continue
    }

    /// Called after detection is complete with the full result.
    async fn after_detect(&self, _ctx: &mut HookContext, _result: &DetectionResult) -> HookResult {
        HookResult::Continue
    }

    /// Called when a specific pattern match is found during detection.
    /// Fires once per match, allowing per-classification logic.
    async fn on_classify(
        &self,
        _ctx: &mut HookContext,
        _classification: &HeuristicMatch,
    ) -> HookResult {
        HookResult::Continue
    }

    /// Called after scoring, with the computed score.
    async fn on_score(&self, _ctx: &mut HookContext, _score: f32) -> HookResult {
        HookResult::Continue
    }

    /// Called before a correction pass begins.
    async fn before_correct(
        &self,
        _ctx: &mut HookContext,
        _detection: &DetectionResult,
    ) -> HookResult {
        HookResult::Continue
    }

    /// Called after a correction pass produces a corrected artifact.
    async fn after_correct(
        &self,
        _ctx: &mut HookContext,
        _corrected: &str,
        _pass: u32,
    ) -> HookResult {
        HookResult::Continue
    }

    /// Called before the validation pass (second loop in full_restructure mode).
    async fn before_validate(&self, _ctx: &mut HookContext, _corrected: &str) -> HookResult {
        HookResult::Continue
    }

    /// Called when the skill completes successfully with a final output.
    async fn on_complete(&self, _ctx: &mut HookContext, _output: &SkillOutput) -> HookResult {
        HookResult::Continue
    }

    /// Called when any error occurs during skill execution.
    async fn on_error(&self, _ctx: &mut HookContext, _error: &str) -> HookResult {
        HookResult::Continue
    }
}
