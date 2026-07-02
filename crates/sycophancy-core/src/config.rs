use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level config — deserializes from `skill.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillConfig {
    pub skill: SkillMeta,
    pub detection: DetectionConfig,
    pub correction: CorrectionConfig,
    pub hooks: HooksConfig,
    pub audit: AuditConfig,
    pub llm: LlmConfig,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            skill: SkillMeta::default(),
            detection: DetectionConfig::default(),
            correction: CorrectionConfig::default(),
            hooks: HooksConfig::default(),
            audit: AuditConfig::default(),
            llm: LlmConfig::default(),
        }
    }
}

impl SkillConfig {
    pub fn from_toml(src: &str) -> crate::SkillResult<Self> {
        toml::from_str(src).map_err(|e| crate::SkillError::Config(e.to_string()))
    }

    pub fn from_file(path: &std::path::Path) -> crate::SkillResult<Self> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| crate::SkillError::Config(format!("Cannot read {path:?}: {e}")))?;
        Self::from_toml(&src)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillMeta {
    pub id: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

impl Default for SkillMeta {
    fn default() -> Self {
        Self {
            id: "sycophancy.correction".into(),
            version: "1.0.0".into(),
            author: "Prometheus AGS".into(),
            description: "Detects and corrects sycophantic patterns in LLM outputs.".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectionConfig {
    /// Patterns disabled by id (e.g. ["S-07"] to suppress scope-creep flattery)
    pub disabled_patterns: Vec<String>,
    /// Per-pattern severity overrides: { "S-01" = "high" }
    pub severity_overrides: HashMap<String, String>,
    /// cosine distance threshold for multi-sample divergence check
    pub divergence_threshold: f32,
    /// Number of samples for multi-sample divergence check
    pub sample_n: usize,
    /// Temperature used for sampling passes
    pub sample_temperature: f32,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            disabled_patterns: vec![],
            severity_overrides: HashMap::new(),
            divergence_threshold: 0.35,
            sample_n: 3,
            sample_temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorrectionConfig {
    /// Score at or above which correction should be flagged as mandatory for callers
    pub mandatory_correction_threshold: f32,
    /// Maximum number of correction loop passes before flagging failure
    pub max_passes: u32,
    /// Score below which a second validation pass is considered clean
    pub clean_threshold: f32,
}

impl Default for CorrectionConfig {
    fn default() -> Self {
        Self {
            mandatory_correction_threshold: 0.6,
            max_passes: 2,
            clean_threshold: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HooksConfig {
    /// Enable the built-in tracing hook
    pub tracing_hook: bool,
    /// Enable the built-in audit-trail hook
    pub audit_hook: bool,
    /// Paths to external hook shared libraries (future: dynamic loading)
    pub external_hooks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    pub enabled: bool,
    pub backend: AuditBackend,
    /// SurrealDB DSN — used when backend = SurrealDb
    pub surreal_dsn: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: AuditBackend::Stdout,
            surreal_dsn: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditBackend {
    #[default]
    Stdout,
    File,
    SurrealDb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Provider base URL. An OpenAI-compatible `/chat/completions` endpoint —
    /// defaults to the local openai-proxy (git@github.com:GQAdonis/openai-proxy.git,
    /// bridges Codex CLI's ChatGPT/OpenAI auth), so no separate API key is needed.
    pub base_url: String,
    /// Model for critic passes
    pub critic_model: String,
    /// Model for correction rewrites
    pub rewrite_model: String,
    /// Max tokens for a critic pass
    pub critic_max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8181/v1".into(),
            critic_model: "gpt-5.4-mini".into(),
            rewrite_model: "gpt-5.5".into(),
            critic_max_tokens: 2048,
        }
    }
}
