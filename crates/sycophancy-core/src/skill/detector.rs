use crate::{
    config::SkillConfig,
    skill::{
        patterns::PatternRegistry,
        scorer::Scorer,
        types::{DetectionResult, HeuristicMatch, Severity, Strictness},
    },
};

pub struct Detector {
    registry: PatternRegistry,
    scorer: Scorer,
    config: SkillConfig,
}

impl Detector {
    pub fn new(config: SkillConfig) -> Self {
        let mut registry = PatternRegistry::default();

        // Apply disabled patterns from config
        for id in &config.detection.disabled_patterns {
            registry.disable(id);
        }

        Self {
            scorer: Scorer::new(&config),
            registry,
            config,
        }
    }

    /// Expose registry so hooks can inject custom patterns before detection
    pub fn registry_mut(&mut self) -> &mut PatternRegistry {
        &mut self.registry
    }

    pub fn detect(
        &self,
        content: &str,
        prior_completions: &[String],
        strictness: &Strictness,
    ) -> DetectionResult {
        let mut matches: Vec<HeuristicMatch> = self
            .registry
            .patterns()
            .iter()
            .flat_map(|p| {
                // S-07 (scope creep) is suppressed at permissive strictness
                if p.id() == "S-07" && *strictness == Strictness::Permissive {
                    return vec![];
                }
                p.evaluate(content, prior_completions)
            })
            .collect();

        // Apply per-pattern severity overrides from config
        for m in &mut matches {
            if let Some(override_str) = self.config.detection.severity_overrides.get(&m.pattern_id)
            {
                m.severity = parse_severity(override_str);
            }
        }

        let score = self.scorer.score(&matches);
        let has_critical = matches.iter().any(|m| m.severity == Severity::Critical);
        let mandatory =
            has_critical || score >= self.config.correction.mandatory_correction_threshold;

        DetectionResult {
            sycophancy_score: score,
            classifications: matches,
            has_critical,
            correction_mandatory: mandatory,
        }
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        _ => Severity::Low,
    }
}
