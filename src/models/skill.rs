use serde::{Deserialize, Serialize};

/// Whether a skill is learned (Acquired), born with (Innate), or granted by a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCategory {
    Acquired,
    Innate,
    Class,
}

/// Whether a skill requires activation (Active) or is always on (Passive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType {
    Active,
    Passive,
}

/// Mastery progression ranks for acquired skills.
/// Each rank has a level cap, tree point yield, and scaling rate.
/// Ordered from weakest (Novice) to strongest (Grandmaster).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MasteryRank {
    Novice,
    Apprentice,
    Journeyman,
    Advanced,
    Expert,
    Master,
    Grandmaster,
}

impl MasteryRank {
    /// Maximum skill level achievable at this rank.
    pub fn max_level(&self) -> u32 {
        match self {
            MasteryRank::Novice => 10,
            MasteryRank::Apprentice => 25,
            MasteryRank::Journeyman => 50,
            MasteryRank::Advanced => 100,
            MasteryRank::Expert => 250,
            MasteryRank::Master => 500,
            MasteryRank::Grandmaster => 1000,
        }
    }

    /// Tree points earned per skill level at this rank.
    /// Ranges from 1 (Novice) to 7 (Grandmaster).
    pub fn tree_points_per_level(&self) -> u32 {
        match self {
            MasteryRank::Novice => 1,
            MasteryRank::Apprentice => 2,
            MasteryRank::Journeyman => 3,
            MasteryRank::Advanced => 4,
            MasteryRank::Expert => 5,
            MasteryRank::Master => 6,
            MasteryRank::Grandmaster => 7,
        }
    }

    /// Passive effect scaling rate per level (1%–7%).
    pub fn passive_increase_per_level(&self) -> f64 {
        match self {
            MasteryRank::Novice => 0.01,
            MasteryRank::Apprentice => 0.02,
            MasteryRank::Journeyman => 0.03,
            MasteryRank::Advanced => 0.04,
            MasteryRank::Expert => 0.05,
            MasteryRank::Master => 0.06,
            MasteryRank::Grandmaster => 0.07,
        }
    }

    /// Active effect scaling rate — always double the passive rate.
    pub fn active_increase_per_level(&self) -> f64 {
        self.passive_increase_per_level() * 2.0
    }

    /// Returns the next rank in the progression, or None at Grandmaster.
    pub fn next(&self) -> Option<MasteryRank> {
        match self {
            MasteryRank::Novice => Some(MasteryRank::Apprentice),
            MasteryRank::Apprentice => Some(MasteryRank::Journeyman),
            MasteryRank::Journeyman => Some(MasteryRank::Advanced),
            MasteryRank::Advanced => Some(MasteryRank::Expert),
            MasteryRank::Expert => Some(MasteryRank::Master),
            MasteryRank::Master => Some(MasteryRank::Grandmaster),
            MasteryRank::Grandmaster => None,
        }
    }

    /// Convenience array for iterating all ranks in order.
    pub const ALL: [MasteryRank; 7] = [
        MasteryRank::Novice,
        MasteryRank::Apprentice,
        MasteryRank::Journeyman,
        MasteryRank::Advanced,
        MasteryRank::Expert,
        MasteryRank::Master,
        MasteryRank::Grandmaster,
    ];

    /// Display name for UI rendering.
    pub fn name(&self) -> &'static str {
        match self {
            MasteryRank::Novice => "Novice",
            MasteryRank::Apprentice => "Apprentice",
            MasteryRank::Journeyman => "Journeyman",
            MasteryRank::Advanced => "Advanced",
            MasteryRank::Expert => "Expert",
            MasteryRank::Master => "Master",
            MasteryRank::Grandmaster => "Grandmaster",
        }
    }
}

/// A single effect that a skill provides (e.g., "Damage Boost +10").
/// The base_value scales with level using the rank's increase rate.
/// Effects can be gated behind a minimum skill level via unlock_level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffect {
    /// Optional name for this effect (e.g., "Damage Boost").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Short description of what this effect does.
    #[serde(default)]
    pub description: String,
    pub base_value: f64,
    /// Skill level required before this effect activates.
    pub unlock_level: u32,
}

impl SkillEffect {
    /// Check if this effect is active at the given skill level.
    pub fn is_unlocked(&self, level: u32) -> bool {
        level >= self.unlock_level
    }

    /// Calculate the scaled value at a given level, rank, and skill type.
    /// Formula: base_value * (1 + increase_rate * level)
    pub fn value_at_level(&self, level: u32, rank: MasteryRank, skill_type: SkillType) -> f64 {
        let increase_per_level = match skill_type {
            SkillType::Passive => rank.passive_increase_per_level(),
            SkillType::Active => rank.active_increase_per_level(),
        };
        self.base_value * (1.0 + increase_per_level * level as f64)
    }
}

/// Per-rank description and effects for an acquired skill.
/// Each mastery rank can have its own flavor text and unique effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankDefinition {
    pub rank: MasteryRank,
    pub description: String,
    pub effects: Vec<SkillEffect>,
}

/// Full definition of a skill in the library.
/// Contains metadata and a list of rank definitions (one per mastery rank).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub category: SkillCategory,
    pub skill_type: SkillType,
    pub description: String,
    /// One entry per mastery rank that has been defined.
    pub ranks: Vec<RankDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mastery_rank_max_level() {
        assert_eq!(MasteryRank::Novice.max_level(), 10);
        assert_eq!(MasteryRank::Apprentice.max_level(), 25);
        assert_eq!(MasteryRank::Journeyman.max_level(), 50);
        assert_eq!(MasteryRank::Advanced.max_level(), 100);
        assert_eq!(MasteryRank::Expert.max_level(), 250);
        assert_eq!(MasteryRank::Master.max_level(), 500);
        assert_eq!(MasteryRank::Grandmaster.max_level(), 1000);
    }

    #[test]
    fn test_mastery_rank_tree_points_per_level() {
        assert_eq!(MasteryRank::Novice.tree_points_per_level(), 1);
        assert_eq!(MasteryRank::Apprentice.tree_points_per_level(), 2);
        assert_eq!(MasteryRank::Grandmaster.tree_points_per_level(), 7);
    }

    #[test]
    fn test_mastery_rank_next() {
        assert_eq!(MasteryRank::Novice.next(), Some(MasteryRank::Apprentice));
        assert_eq!(MasteryRank::Grandmaster.next(), None);
    }

    #[test]
    fn test_skill_effect_at_level() {
        let effect = SkillEffect {
            name: Some("Damage Boost".to_string()),
            description: "Increases damage output".to_string(),
            base_value: 10.0,
            unlock_level: 0,
        };
        // Novice passive: +1% per level, level 5 => 10.0 * (1 + 0.01 * 5) = 10.5
        assert!((effect.value_at_level(5, MasteryRank::Novice, SkillType::Passive) - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_skill_effect_hidden_before_unlock() {
        let effect = SkillEffect {
            name: Some("Hidden Power".to_string()),
            description: "Unleashes latent energy".to_string(),
            base_value: 50.0,
            unlock_level: 5,
        };
        assert!(!effect.is_unlocked(4));
        assert!(effect.is_unlocked(5));
    }

    #[test]
    fn test_skill_creation() {
        let skill = SkillDefinition {
            name: "Fireball".to_string(),
            category: SkillCategory::Acquired,
            skill_type: SkillType::Active,
            description: "Launches a ball of fire".to_string(),
            ranks: vec![
                RankDefinition {
                    rank: MasteryRank::Novice,
                    description: "A weak fireball".to_string(),
                    effects: vec![SkillEffect {
                        name: Some("Fire Damage".to_string()),
                        description: "Burns the target".to_string(),
                        base_value: 20.0,
                        unlock_level: 0,
                    }],
                },
            ],
        };
        assert_eq!(skill.name, "Fireball");
        assert_eq!(skill.ranks.len(), 1);
    }
}
