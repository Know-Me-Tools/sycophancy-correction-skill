use thiserror::Error;

pub type SkillResult<T> = Result<T, SkillError>;

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("Hook abort: {reason}")]
    HookAbort { reason: String },

    #[error(
        "Correction failed after {passes} passes: score {final_score:.2} remains above threshold"
    )]
    CorrectionFailed { passes: u32, final_score: f32 },

    #[error("Validation failed: {field} — {message}")]
    ValidationFailed { field: String, message: String },

    #[error("Pattern compilation error in '{pattern_id}': {source}")]
    PatternCompile {
        pattern_id: String,
        #[source]
        source: regex::Error,
    },

    #[error("LLM invocation error: {0}")]
    LlmError(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
