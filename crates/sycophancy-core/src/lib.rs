// sycophancy-core — agentskills.io compliant skill engine
//
// Crate structure:
//   config   — SkillConfig (deserialized from skill.toml)
//   error    — unified error type
//   skill    — detection, scoring, correction engine
//   hooks    — Hook trait, HookRegistry, HookContext, builtin hooks
//   pmpo     — PMPO meta-loop executor

pub mod config;
pub mod error;
pub mod hooks;
pub mod pmpo;
pub mod skill;

pub use error::{SkillError, SkillResult};
pub use skill::types::{
    CorrectionMode, DetectionResult, HeuristicMatch, InputContext, SkillInput, SkillOutput,
    Strictness, TargetType,
};
