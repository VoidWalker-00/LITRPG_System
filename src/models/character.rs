use serde::{Deserialize, Serialize};
use crate::models::attribute::Attributes;
use crate::models::grade::Grade;
use crate::models::profession::CharacterProfession;
use crate::models::skill::MasteryRank;

/// Base attribute points earned per level before grade scaling.
const BASE_ATTRIBUTE_POINTS_PER_LEVEL: u32 = 3;

/// A skill attached to a character, tracking current rank and level.
/// References a SkillDefinition by name (looked up from the library at runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSkill {
    pub definition_name: String,
    pub rank: MasteryRank,
    pub level: u32,
}

/// An innate skill attached to a character, tracking current level.
/// References an InnateSkillDefinition by name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInnateSkill {
    pub definition_name: String,
    pub level: u32,
}

/// A tree attached to a character, tracking points invested.
/// References a TreeDefinition by name.
/// Kept temporarily for system_panel.rs compatibility until cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTree {
    pub definition_name: String,
    pub current_points: u32,
    pub max_points: u32,
}

/// The main character struct — holds all progression state.
/// Created at Grade G, Level 0 with initial attributes.
/// All mutations (leveling, point distribution) go through methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub grade: Grade,
    pub race: String,
    pub level: u32,
    pub xp: f64,
    pub attributes: Attributes,
    pub unspent_attribute_points: u32,
    pub skills: Vec<CharacterSkill>,
    pub innate_skill: Option<CharacterInnateSkill>,
    pub professions: Vec<CharacterProfession>,
    pub profession_slots: u32,
    /// Extra attribute points per level granted by bonuses.
    pub bonus_attribute_points_per_level: u32,
}

impl Character {
    /// Create a new character at Grade G, Level 0 with the given attributes.
    pub fn new(
        name: String,
        attributes: Attributes,
        innate_skill: Option<CharacterInnateSkill>,
    ) -> Self {
        Self {
            name,
            grade: Grade::G,
            race: "Human".to_string(),
            level: 0,
            xp: 0.0,
            attributes,
            unspent_attribute_points: 0,
            skills: Vec::new(),
            innate_skill,
            professions: Vec::new(),
            profession_slots: 1,
            bonus_attribute_points_per_level: 0,
        }
    }

    /// Attribute points earned per level-up.
    /// Formula: (base + bonus) * 2^grade_numeric
    /// Base is 3, bonus comes from milestones/professions.
    pub fn attribute_points_per_level(&self) -> u32 {
        let base = BASE_ATTRIBUTE_POINTS_PER_LEVEL + self.bonus_attribute_points_per_level;
        base * 2u32.pow(self.grade.numeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attribute::Attributes;

    #[test]
    fn test_new_character() {
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let character = Character::new("Kael".to_string(), attrs, None);
        assert_eq!(character.name, "Kael");
        assert_eq!(character.grade, Grade::G);
        assert_eq!(character.race, "Human");
        assert_eq!(character.level, 0);
        assert_eq!(character.xp, 0.0);
        assert_eq!(character.unspent_attribute_points, 0);
        assert_eq!(character.professions.len(), 0);
        assert_eq!(character.profession_slots, 1);
        assert!(character.innate_skill.is_none());
    }

    #[test]
    fn test_attribute_points_per_level_base() {
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let character = Character::new("Test".to_string(), attrs, None);
        // Grade G (numeric 0), no bonuses: 3 * 2^0 = 3
        assert_eq!(character.attribute_points_per_level(), 3);
    }

    #[test]
    fn test_attribute_points_per_level_grade_scaling() {
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let mut character = Character::new("Test".to_string(), attrs, None);
        character.grade = Grade::E; // numeric 2
        // Grade E (numeric 2), no bonuses: 3 * 2^2 = 12
        assert_eq!(character.attribute_points_per_level(), 12);
    }
}
