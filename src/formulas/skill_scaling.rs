/// Skill effect scaling formulas.
///
/// Acquired skills scale based on mastery rank:
///   Passive: +1%–7% per level (rank-dependent)
///   Active: double the passive rate
///
/// Innate skills have fixed rates:
///   Passive: +5% per level
///   Active: +10% per level

use crate::models::skill::{MasteryRank, SkillType};

/// Get the per-level increase rate for an acquired skill at a given rank and type.
pub fn acquired_increase_per_level(rank: MasteryRank, skill_type: SkillType) -> f64 {
    let passive_rate = rank.passive_increase_per_level();
    match skill_type {
        SkillType::Passive => passive_rate,
        SkillType::Active => passive_rate * 2.0,
    }
}

/// Calculate the scaled effect value for an acquired skill.
/// Formula: base * (1 + rate * level)
pub fn acquired_effect_value(base: f64, level: u32, rank: MasteryRank, skill_type: SkillType) -> f64 {
    let rate = acquired_increase_per_level(rank, skill_type);
    base * (1.0 + rate * level as f64)
}

/// Calculate the scaled effect value for an innate skill.
/// Uses fixed rates: +5% passive, +10% active per level.
pub fn innate_effect_value(base: f64, level: u32, skill_type: SkillType) -> f64 {
    let rate = match skill_type {
        SkillType::Passive => 0.05,
        SkillType::Active => 0.10,
    };
    base * (1.0 + rate * level as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill::{MasteryRank, SkillType};

    #[test]
    fn test_acquired_passive_novice() {
        // Novice passive: +1% per level, level 10, base 100
        // 100 * (1 + 0.01 * 10) = 110
        assert!((acquired_effect_value(100.0, 10, MasteryRank::Novice, SkillType::Passive) - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_acquired_active_apprentice() {
        // Apprentice active: +4% per level, level 25, base 50
        // 50 * (1 + 0.04 * 25) = 100
        assert!((acquired_effect_value(50.0, 25, MasteryRank::Apprentice, SkillType::Active) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_innate_passive() {
        // +5% per level, level 10, base 10 => 15
        assert!((innate_effect_value(10.0, 10, SkillType::Passive) - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_innate_active() {
        // +10% per level, level 10, base 10 => 20
        assert!((innate_effect_value(10.0, 10, SkillType::Active) - 20.0).abs() < 0.01);
    }
}
