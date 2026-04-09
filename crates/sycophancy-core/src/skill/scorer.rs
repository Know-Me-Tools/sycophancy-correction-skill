use crate::{config::SkillConfig, skill::types::HeuristicMatch};

pub struct Scorer {
    /// Max possible weight sum for clamping
    ceiling: f32,
}

impl Scorer {
    pub fn new(_config: &SkillConfig) -> Self {
        // Theoretical maximum: all 8 patterns at Critical weight (0.70)
        Self {
            ceiling: 8.0 * 0.70,
        }
    }

    /// Map a set of classifications to a score in [0.0, 1.0].
    ///
    /// Score is the sum of severity weights, clamped to the ceiling, then
    /// normalized. Multiple high-severity findings compound the score
    /// non-linearly through the accumulation mechanism.
    pub fn score(&self, matches: &[HeuristicMatch]) -> f32 {
        if matches.is_empty() {
            return 0.0;
        }

        let raw: f32 = matches.iter().map(|m| m.severity.weight()).sum();
        (raw / self.ceiling).clamp(0.0, 1.0)
    }
}
