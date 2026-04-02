use serde::{Deserialize, Serialize};
use crate::models::skill::SkillType;

/// Hard cap on innate skill levels — no innate skill can exceed this.
const INNATE_ABSOLUTE_MAX_LEVEL: u32 = 100;

/// Innate passive skills gain +5% per level.
const INNATE_PASSIVE_PER_LEVEL: f64 = 0.05;

/// Innate active skills gain +10% per level (double passive).
const INNATE_ACTIVE_PER_LEVEL: f64 = 0.10;

/// A single effect on an innate skill (e.g., "Learning Boost +10").
/// Simpler than acquired SkillEffect — no unlock gating, no rank dependency.
/// Scaling is fixed: +5% passive or +10% active per level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnateEffect {
    pub name: String,
    pub base_value: f64,
}

impl InnateEffect {
    /// Calculate scaled value at a given level.
    /// Formula: base_value * (1 + rate * level)
    /// Rate depends on whether the innate skill is passive (+5%) or active (+10%).
    pub fn value_at_level(&self, level: u32, skill_type: SkillType) -> f64 {
        let rate = match skill_type {
            SkillType::Passive => INNATE_PASSIVE_PER_LEVEL,
            SkillType::Active => INNATE_ACTIVE_PER_LEVEL,
        };
        self.base_value * (1.0 + rate * level as f64)
    }
}

/// An evolution option that unlocks at a specific tier.
/// Each tier can grant a new form of the innate skill with different effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnateEvolution {
    /// The tier at which this evolution becomes available.
    pub tier: u32,
    pub name: String,
    pub description: String,
    pub effects: Vec<InnateEffect>,
}

/// Full definition of an innate skill.
/// Innate skills have no mastery ranks — they level linearly up to max_level.
/// The `growable` flag indicates whether the skill can gain levels through use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnateSkillDefinition {
    pub name: String,
    pub skill_type: SkillType,
    pub description: String,
    /// Configured max level (may exceed the absolute cap).
    pub max_level: u32,
    /// Whether this skill can grow through use (vs. being fixed at creation level).
    pub growable: bool,
    pub effects: Vec<InnateEffect>,
    /// Tier-based evolutions that transform the skill.
    pub evolutions: Vec<InnateEvolution>,
}

impl InnateSkillDefinition {
    /// Returns the actual max level, capped at the absolute limit of 100.
    pub fn effective_max_level(&self) -> u32 {
        self.max_level.min(INNATE_ABSOLUTE_MAX_LEVEL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill::SkillType;

    #[test]
    fn test_innate_skill_max_level_capped_at_100() {
        let skill = InnateSkillDefinition {
            name: "Sword Affinity".to_string(),
            skill_type: SkillType::Passive,
            description: "Natural sword talent".to_string(),
            max_level: 150, // exceeds cap
            growable: true,
            effects: vec![],
            evolutions: vec![],
        };
        assert_eq!(skill.effective_max_level(), 100);
    }

    #[test]
    fn test_innate_passive_scaling() {
        let effect = InnateEffect {
            name: "Learning Boost".to_string(),
            base_value: 10.0,
        };
        // Passive innate: +5% per level, level 10 => 10.0 * (1 + 0.05 * 10) = 15.0
        assert!((effect.value_at_level(10, SkillType::Passive) - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_innate_active_scaling() {
        let effect = InnateEffect {
            name: "Strike".to_string(),
            base_value: 10.0,
        };
        // Active innate: +10% per level, level 10 => 10.0 * (1 + 0.10 * 10) = 20.0
        assert!((effect.value_at_level(10, SkillType::Active) - 20.0).abs() < 0.01);
    }
}
