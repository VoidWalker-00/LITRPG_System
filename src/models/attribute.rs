use serde::{Deserialize, Serialize};

/// All six core attributes in the system.
/// Physical: Strength, Agility, Endurance
/// Magical: Intelligence, Wisdom, Perception
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeKind {
    Strength,
    Agility,
    Endurance,
    Intelligence,
    Wisdom,
    Perception,
}

impl AttributeKind {
    /// Convenience array for iterating over all attribute types.
    pub const ALL: [AttributeKind; 6] = [
        AttributeKind::Strength,
        AttributeKind::Agility,
        AttributeKind::Endurance,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Perception,
    ];

    /// Display name for UI rendering.
    pub fn name(&self) -> &'static str {
        match self {
            AttributeKind::Strength => "Strength",
            AttributeKind::Agility => "Agility",
            AttributeKind::Endurance => "Endurance",
            AttributeKind::Intelligence => "Intelligence",
            AttributeKind::Wisdom => "Wisdom",
            AttributeKind::Perception => "Perception",
        }
    }
}

/// Holds the six attribute values for a character.
/// Each attribute is stored as a u32 — no upper bound at runtime,
/// but `new_clamped()` enforces 1–10 during character creation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attributes {
    pub strength: u32,
    pub agility: u32,
    pub endurance: u32,
    pub intelligence: u32,
    pub wisdom: u32,
    pub perception: u32,
}

impl Attributes {
    /// Create attributes with exact values (no validation).
    pub fn new(str: u32, agi: u32, end: u32, int: u32, wis: u32, per: u32) -> Self {
        Self {
            strength: str,
            agility: agi,
            endurance: end,
            intelligence: int,
            wisdom: wis,
            perception: per,
        }
    }

    /// Create attributes clamped to the 1–10 range (used during character creation).
    pub fn new_clamped(str: u32, agi: u32, end: u32, int: u32, wis: u32, per: u32) -> Self {
        Self {
            strength: str.clamp(1, 10),
            agility: agi.clamp(1, 10),
            endurance: end.clamp(1, 10),
            intelligence: int.clamp(1, 10),
            wisdom: wis.clamp(1, 10),
            perception: per.clamp(1, 10),
        }
    }

    /// Get the value of a specific attribute by kind.
    pub fn get(&self, kind: AttributeKind) -> u32 {
        match kind {
            AttributeKind::Strength => self.strength,
            AttributeKind::Agility => self.agility,
            AttributeKind::Endurance => self.endurance,
            AttributeKind::Intelligence => self.intelligence,
            AttributeKind::Wisdom => self.wisdom,
            AttributeKind::Perception => self.perception,
        }
    }

    /// Add points to a specific attribute.
    pub fn add(&mut self, kind: AttributeKind, amount: u32) {
        match kind {
            AttributeKind::Strength => self.strength += amount,
            AttributeKind::Agility => self.agility += amount,
            AttributeKind::Endurance => self.endurance += amount,
            AttributeKind::Intelligence => self.intelligence += amount,
            AttributeKind::Wisdom => self.wisdom += amount,
            AttributeKind::Perception => self.perception += amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_default_values() {
        let attrs = Attributes::new(5, 3, 4, 7, 2, 8);
        assert_eq!(attrs.strength, 5);
        assert_eq!(attrs.agility, 3);
        assert_eq!(attrs.endurance, 4);
        assert_eq!(attrs.intelligence, 7);
        assert_eq!(attrs.wisdom, 2);
        assert_eq!(attrs.perception, 8);
    }

    #[test]
    fn test_attribute_validation_clamps_to_range() {
        let attrs = Attributes::new_clamped(0, 15, 5, 5, 5, 5);
        assert_eq!(attrs.strength, 1);  // clamped from 0 to 1
        assert_eq!(attrs.agility, 10);  // clamped from 15 to 10
    }

    #[test]
    fn test_add_points() {
        let mut attrs = Attributes::new(5, 5, 5, 5, 5, 5);
        attrs.add(AttributeKind::Strength, 10);
        assert_eq!(attrs.strength, 15);
    }

    #[test]
    fn test_get_by_kind() {
        let attrs = Attributes::new(1, 2, 3, 4, 5, 6);
        assert_eq!(attrs.get(AttributeKind::Strength), 1);
        assert_eq!(attrs.get(AttributeKind::Perception), 6);
    }
}
