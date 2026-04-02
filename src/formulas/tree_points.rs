/// Tree point formulas.
///
/// Tree points are earned when leveling skills — the amount depends
/// on the skill's current mastery rank (1–7 points per level).
/// Sequential tree chains multiply cost by 5x per step.

use crate::models::skill::MasteryRank;

/// Tree points earned per skill level-up at the given mastery rank.
/// Delegates to MasteryRank::tree_points_per_level().
pub fn tree_points_for_skill_level(rank: MasteryRank) -> u32 {
    rank.tree_points_per_level()
}

/// Point cost of a tree at a given step in a sequential chain.
/// Formula: base_cost * 5^step
pub fn sequential_tree_cost(base_cost: u32, step: u32) -> u32 {
    base_cost * 5u32.pow(step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill::MasteryRank;

    #[test]
    fn test_tree_points_from_skill_level() {
        assert_eq!(tree_points_for_skill_level(MasteryRank::Novice), 1);
        assert_eq!(tree_points_for_skill_level(MasteryRank::Apprentice), 2);
        assert_eq!(tree_points_for_skill_level(MasteryRank::Grandmaster), 7);
    }

    #[test]
    fn test_sequential_tree_cost() {
        assert_eq!(sequential_tree_cost(40, 0), 40);
        assert_eq!(sequential_tree_cost(40, 1), 200);
        assert_eq!(sequential_tree_cost(40, 2), 1000);
        assert_eq!(sequential_tree_cost(40, 3), 5000);
    }
}
