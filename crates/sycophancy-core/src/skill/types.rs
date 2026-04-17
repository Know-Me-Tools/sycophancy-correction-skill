use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Input ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    /// What kind of artifact is being evaluated
    pub target: TargetType,
    /// Raw content to evaluate
    pub content: String,
    /// Optional extra context
    #[serde(default)]
    pub context: InputContext,
    #[serde(default)]
    pub correction_mode: CorrectionMode,
    #[serde(default)]
    pub strictness: Strictness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    Prompt,
    Completion,
    AgentDescriptor,
    Pipeline,
}

impl TargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetType::Prompt => "prompt",
            TargetType::Completion => "completion",
            TargetType::AgentDescriptor => "agent_descriptor",
            TargetType::Pipeline => "pipeline",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputContext {
    /// What the original request was attempting to achieve
    pub original_intent: Option<String>,
    /// Prior completions in a multi-turn session (for S-05 drift detection)
    #[serde(default)]
    pub prior_completions: Vec<String>,
    /// Domain hint: "technical review" | "strategic planning" | etc.
    pub evaluation_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionMode {
    /// Return score + classifications only; no mutation
    DetectOnly,
    /// Inline annotations on sycophancy sites
    Annotate,
    /// Return corrected artifact
    #[default]
    Rewrite,
    /// Rebuild agent/pipeline architecture
    FullRestructure,
}

impl CorrectionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CorrectionMode::DetectOnly => "detect_only",
            CorrectionMode::Annotate => "annotate",
            CorrectionMode::Rewrite => "rewrite",
            CorrectionMode::FullRestructure => "full_restructure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Strictness {
    Permissive,
    #[default]
    Standard,
    Strict,
}

impl Strictness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Strictness::Permissive => "permissive",
            Strictness::Standard => "standard",
            Strictness::Strict => "strict",
        }
    }
}

// ── Detection ─────────────────────────────────────────────────────────────────

/// A single matched sycophancy pattern instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicMatch {
    /// Pattern ID: S-01 … S-08
    pub pattern_id: String,
    pub severity: Severity,
    /// Approximate character offset in the source content
    pub location: Option<String>,
    /// Why this pattern was triggered
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }

    pub fn weight(&self) -> f32 {
        match self {
            Severity::Low => 0.1,
            Severity::Medium => 0.25,
            Severity::High => 0.45,
            Severity::Critical => 0.70,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Aggregated score: 0.0 (clean) – 1.0 (fully sycophantic)
    pub sycophancy_score: f32,
    pub classifications: Vec<HeuristicMatch>,
    /// True if any critical classification was found
    pub has_critical: bool,
    /// True if correction is mandatory regardless of mode
    pub correction_mandatory: bool,
}

// ── Output ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub sycophancy_score: f32,
    pub classifications: Vec<HeuristicMatch>,
    /// Present when correction_mode ∈ {Rewrite, FullRestructure}
    pub corrected_artifact: Option<String>,
    /// Human-readable summary of what changed and why
    pub delta_summary: String,
    pub audit_trail: AuditTrail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub skill_id: String,
    pub version: String,
    pub strictness: String,
    pub executed_at: DateTime<Utc>,
    pub agent_did: Option<String>,
    pub passes: u32,
    pub hook_log: Vec<String>,
}
