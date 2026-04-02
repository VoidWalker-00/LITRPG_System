use serde::{Deserialize, Serialize};

/// Definition of a class in the library.
/// Classes grant skills, a passive, and bonus attribute points per level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub description: String,
    /// Names of skills granted by this class.
    pub skills: Vec<String>,
    /// Name of the passive skill this class provides.
    pub passive_name: String,
    /// Description of the passive skill.
    pub passive_description: String,
    /// Fixed attribute bonuses applied directly each level-up.
    #[serde(default)] pub bonus_str: u32,
    #[serde(default)] pub bonus_agi: u32,
    #[serde(default)] pub bonus_end: u32,
    #[serde(default)] pub bonus_int: u32,
    #[serde(default)] pub bonus_wis: u32,
    #[serde(default)] pub bonus_per: u32,
    /// Free unspent attribute points added to the pool each level-up.
    #[serde(default)] pub bonus_free_points: u32,
}

/// A character's active class instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterClass {
    /// Name of the ClassDefinition this refers to.
    pub definition_name: String,
    /// Current class level (leveled through activity, not XP).
    pub level: u32,
    /// Current evolution rank of the class's passive.
    pub passive_rank: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_definition_creation() {
        let cls = ClassDefinition {
            name: "Blacksmith".to_string(),
            description: "Craft weapons and armor".to_string(),
            skills: vec!["Forging".to_string(), "Tempering".to_string()],
            passive_name: "Forgeborn".to_string(),
            passive_description: "Increases crafting quality".to_string(),
            bonus_str: 0, bonus_agi: 0, bonus_end: 0,
            bonus_int: 0, bonus_wis: 0, bonus_per: 0,
            bonus_free_points: 0,
        };
        assert_eq!(cls.name, "Blacksmith");
        assert_eq!(cls.skills.len(), 2);
    }

    #[test]
    fn test_class_serialization() {
        let cls = ClassDefinition {
            name: "Herbalist".to_string(),
            description: "Gather herbs".to_string(),
            skills: vec!["Gathering".to_string()],
            passive_name: "Nature's Touch".to_string(),
            passive_description: "Improves herb yield".to_string(),
            bonus_str: 0, bonus_agi: 0, bonus_end: 0,
            bonus_int: 0, bonus_wis: 0, bonus_per: 0,
            bonus_free_points: 0,
        };
        let json = serde_json::to_string(&cls).unwrap();
        let parsed: ClassDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Herbalist");
        assert_eq!(parsed.skills.len(), 1);
    }
}
