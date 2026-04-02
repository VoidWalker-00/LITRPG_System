use serde::{Deserialize, Serialize};

/// Grade represents a being's evolutionary rank.
/// G is the lowest (starting grade), progressing upward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Grade {
    G,
    F,
    E,
    D,
    C,
    B,
    A,
    S,
    SS,
    SSS,
}

impl Grade {
    /// Numeric equivalent for use in formulas (replaces old tier number).
    pub fn numeric(&self) -> u32 {
        match self {
            Grade::G => 0,
            Grade::F => 1,
            Grade::E => 2,
            Grade::D => 3,
            Grade::C => 4,
            Grade::B => 5,
            Grade::A => 6,
            Grade::S => 7,
            Grade::SS => 8,
            Grade::SSS => 9,
        }
    }

    /// Display name for UI rendering.
    pub fn name(&self) -> &'static str {
        match self {
            Grade::G => "G",
            Grade::F => "F",
            Grade::E => "E",
            Grade::D => "D",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::S => "S",
            Grade::SS => "SS",
            Grade::SSS => "SSS",
        }
    }

    /// Return the next grade, or None if already at SSS.
    pub fn next(&self) -> Option<Grade> {
        match self {
            Grade::G => Some(Grade::F),
            Grade::F => Some(Grade::E),
            Grade::E => Some(Grade::D),
            Grade::D => Some(Grade::C),
            Grade::C => Some(Grade::B),
            Grade::B => Some(Grade::A),
            Grade::A => Some(Grade::S),
            Grade::S => Some(Grade::SS),
            Grade::SS => Some(Grade::SSS),
            Grade::SSS => None,
        }
    }

    /// Convert a numeric value back to a Grade. Unknown values clamp to S.
    pub fn from_numeric(n: u32) -> Grade {
        match n {
            0 => Grade::G,
            1 => Grade::F,
            2 => Grade::E,
            3 => Grade::D,
            4 => Grade::C,
            5 => Grade::B,
            6 => Grade::A,
            7 => Grade::S,
            8 => Grade::SS,
            9 => Grade::SSS,
            _ => Grade::S,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_numeric() {
        assert_eq!(Grade::G.numeric(), 0);
        assert_eq!(Grade::F.numeric(), 1);
        assert_eq!(Grade::A.numeric(), 6);
        assert_eq!(Grade::S.numeric(), 7);
    }

    #[test]
    fn test_grade_display() {
        assert_eq!(Grade::G.name(), "G");
        assert_eq!(Grade::SS.name(), "SS");
    }

    #[test]
    fn test_grade_from_numeric() {
        assert_eq!(Grade::from_numeric(0), Grade::G);
        assert_eq!(Grade::from_numeric(6), Grade::A);
        assert_eq!(Grade::from_numeric(99), Grade::S); // clamp unknown to S
    }
}
