//! Tool schema definitions — exposed via list_tools and validated on call_tool.

use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use sycophancy_core::skill::types::{CorrectionMode, InputContext, Strictness, TargetType};

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DetectSycophancyInput {
    pub content: String,
    pub target: TargetStr,
    pub strictness: Option<StrictnessStr>,
    pub context: Option<ContextInput>,
    pub agent_did: Option<String>,
    pub original_intent: Option<String>,
    pub prior_completions: Option<Vec<String>>,
    pub evaluation_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CorrectSycophancyInput {
    pub content: String,
    pub target: TargetStr,
    pub correction_mode: Option<ModeStr>,
    pub mode: Option<ModeStr>,
    pub strictness: Option<StrictnessStr>,
    pub context: Option<ContextInput>,
    pub agent_did: Option<String>,
    pub original_intent: Option<String>,
    pub prior_completions: Option<Vec<String>>,
    pub evaluation_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeReflectPhaseInput {
    pub reflect_output: String,
    /// If true (default), return a corrected reflect output
    pub correct: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContextInput {
    pub original_intent: Option<String>,
    #[serde(default)]
    pub prior_completions: Vec<String>,
    pub evaluation_domain: Option<String>,
}

// ── String newtypes with parse helpers ───────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TargetStr(pub String);

impl TargetStr {
    pub fn parse_target(&self) -> TargetType {
        match self.0.as_str() {
            "prompt" => TargetType::Prompt,
            "agent_descriptor" => TargetType::AgentDescriptor,
            "pipeline" => TargetType::Pipeline,
            _ => TargetType::Completion,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StrictnessStr(pub String);

impl StrictnessStr {
    pub fn parse_strictness(&self) -> Strictness {
        match self.0.as_str() {
            "permissive" => Strictness::Permissive,
            "strict" => Strictness::Strict,
            _ => Strictness::Standard,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ModeStr(pub String);

impl ModeStr {
    pub fn parse_mode(&self) -> CorrectionMode {
        match self.0.as_str() {
            "detect_only" => CorrectionMode::DetectOnly,
            "annotate" => CorrectionMode::Annotate,
            "full_restructure" => CorrectionMode::FullRestructure,
            _ => CorrectionMode::Rewrite,
        }
    }
}

impl DetectSycophancyInput {
    pub fn parse_strictness(&self) -> Strictness {
        self.strictness
            .as_ref()
            .map(StrictnessStr::parse_strictness)
            .unwrap_or_default()
    }

    pub fn input_context(&self) -> InputContext {
        merge_context(
            self.context.as_ref(),
            self.original_intent.as_deref(),
            self.prior_completions.as_deref(),
            self.evaluation_domain.as_deref(),
        )
    }
}

impl CorrectSycophancyInput {
    pub fn parse_strictness(&self) -> Strictness {
        self.strictness
            .as_ref()
            .map(StrictnessStr::parse_strictness)
            .unwrap_or_default()
    }

    pub fn parse_mode(&self) -> CorrectionMode {
        self.correction_mode
            .as_ref()
            .or(self.mode.as_ref())
            .map(ModeStr::parse_mode)
            .unwrap_or_default()
    }

    pub fn input_context(&self) -> InputContext {
        merge_context(
            self.context.as_ref(),
            self.original_intent.as_deref(),
            self.prior_completions.as_deref(),
            self.evaluation_domain.as_deref(),
        )
    }
}

fn merge_context(
    context: Option<&ContextInput>,
    original_intent: Option<&str>,
    prior_completions: Option<&[String]>,
    evaluation_domain: Option<&str>,
) -> InputContext {
    let nested = context.cloned().unwrap_or_default();

    InputContext {
        original_intent: nested
            .original_intent
            .or_else(|| original_intent.map(str::to_owned)),
        prior_completions: if nested.prior_completions.is_empty() {
            prior_completions.unwrap_or_default().to_vec()
        } else {
            nested.prior_completions
        },
        evaluation_domain: nested
            .evaluation_domain
            .or_else(|| evaluation_domain.map(str::to_owned)),
    }
}

// ── Tool definitions ──────────────────────────────────────────────────────────

pub fn tool_definitions() -> Vec<Tool> {
    vec![
        Tool::new(
            "detect_sycophancy",
            "Analyzes an LLM completion, prompt, agent descriptor, or pipeline \
                 configuration for sycophantic patterns (S-01 through S-08). \
                 Returns a score in [0.0, 1.0], a list of classified patterns with \
                 severity and rationale, and an audit trail. \
                 Use this before correct_sycophancy to understand what will be changed.",
            schema(json!({
                "type": "object",
                "required": ["content", "target"],
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The text to evaluate"
                    },
                    "target": {
                        "type": "string",
                        "enum": ["prompt", "completion", "agent_descriptor", "pipeline"],
                        "description": "What kind of artifact is being evaluated",
                        "default": "completion"
                    },
                    "strictness": {
                        "type": "string",
                        "enum": ["permissive", "standard", "strict"],
                        "description": "Detection sensitivity.",
                        "default": "standard"
                    },
                    "context": {
                        "type": "object",
                        "description": "Canonical context object from the skill spec",
                        "properties": {
                            "original_intent": { "type": "string" },
                            "prior_completions": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "evaluation_domain": { "type": "string" }
                        }
                    },
                    "agent_did": {
                        "type": "string",
                        "description": "DID of the invoking agent — recorded in audit trail"
                    },
                    "original_intent": {
                        "type": "string",
                        "description": "What the original request was trying to accomplish"
                    },
                    "prior_completions": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Previous turns — required to detect S-05 context drift"
                    },
                    "evaluation_domain": {
                        "type": "string",
                        "description": "Domain hint: 'technical review', 'strategic planning', etc."
                    }
                }
            })),
        ),
        Tool::new(
            "correct_sycophancy",
            "Detects and rewrites a sycophantic artifact. Returns the corrected artifact, \
                 the detection score, a list of corrected patterns, and a delta summary \
                 explaining what changed and why. Use 'full_restructure' mode for agent \
                 descriptors and pipeline configs — it runs a second validation pass.",
            schema(json!({
                "type": "object",
                "required": ["content", "target"],
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The artifact to correct"
                    },
                    "target": {
                        "type": "string",
                        "enum": ["prompt", "completion", "agent_descriptor", "pipeline"],
                        "default": "completion"
                    },
                    "correction_mode": {
                        "type": "string",
                        "enum": ["detect_only", "annotate", "rewrite", "full_restructure"],
                        "description": "Canonical correction mode field from the skill spec",
                        "default": "rewrite"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["detect_only", "annotate", "rewrite", "full_restructure"],
                        "description": "'rewrite' corrects text; 'full_restructure' rebuilds architecture",
                        "default": "rewrite"
                    },
                    "strictness": {
                        "type": "string",
                        "enum": ["permissive", "standard", "strict"],
                        "default": "standard"
                    },
                    "context": {
                        "type": "object",
                        "description": "Canonical context object from the skill spec",
                        "properties": {
                            "original_intent": { "type": "string" },
                            "prior_completions": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "evaluation_domain": { "type": "string" }
                        }
                    },
                    "agent_did":         { "type": "string" },
                    "original_intent":   { "type": "string" },
                    "prior_completions": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "evaluation_domain": { "type": "string" }
                }
            })),
        ),
        Tool::new(
            "analyze_reflect_phase",
            "Specialized tool for PMPO Reflect phase outputs. \
                 Detects S-08 (Reflect Phase Inversion) and restructures the output \
                 into the mandatory Delta → Root Cause → Corrective Actions format. \
                 Always runs at 'strict' strictness regardless of config.",
            schema(json!({
                "type": "object",
                "required": ["reflect_output"],
                "properties": {
                    "reflect_output": {
                        "type": "string",
                        "description": "The PMPO Reflect phase text to analyze"
                    },
                    "correct": {
                        "type": "boolean",
                        "description": "If true (default), return a corrected version",
                        "default": true
                    }
                }
            })),
        ),
        Tool::new(
            "skill_info",
            "Returns skill metadata: version, supported patterns, modes, strictness levels, \
                 and links. No arguments required.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
    ]
}

fn schema(value: Value) -> Arc<Map<String, Value>> {
    match value {
        Value::Object(map) => Arc::new(map),
        _ => Arc::new(Map::new()),
    }
}
