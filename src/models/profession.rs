use serde::{Deserialize, Serialize};

/// Definition of a profession in the library.
/// Professions grant skills and a passive that evolves through achievements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfessionDefinition {
    pub name: String,
    pub description: String,
    /// Names of skills granted by this profession.
    pub skills: Vec<String>,
    /// Name of the passive skill this profession provides.
    pub passive_name: String,
    /// Description of the passive skill.
    pub passive_description: String,
}

/// A character's active profession instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterProfession {
    /// Name of the ProfessionDefinition this refers to.
    pub definition_name: String,
    /// Current profession level (leveled through activity, not XP).
    pub level: u32,
    /// Current evolution rank of the profession's passive.
    pub passive_rank: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profession_definition_creation() {
        let prof = ProfessionDefinition {
            name: "Blacksmith".to_string(),
            description: "Craft weapons and armor".to_string(),
            skills: vec!["Forging".to_string(), "Tempering".to_string()],
            passive_name: "Forgeborn".to_string(),
            passive_description: "Increases crafting quality".to_string(),
        };
        assert_eq!(prof.name, "Blacksmith");
        assert_eq!(prof.skills.len(), 2);
    }

    #[test]
    fn test_profession_serialization() {
        let prof = ProfessionDefinition {
            name: "Herbalist".to_string(),
            description: "Gather herbs".to_string(),
            skills: vec!["Gathering".to_string()],
            passive_name: "Nature's Touch".to_string(),
            passive_description: "Improves herb yield".to_string(),
        };
        let json = serde_json::to_string(&prof).unwrap();
        let parsed: ProfessionDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Herbalist");
        assert_eq!(parsed.skills.len(), 1);
    }
}
