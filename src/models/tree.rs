use serde::{Deserialize, Serialize};
use crate::models::skill::MasteryRank;

/// How a level/skill requirement is compared (e.g., "level >= 10").
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Comparison {
    GreaterThan,
    LessThan,
    Equal,
    GreaterOrEqual,
    LessOrEqual,
}

/// A prerequisite that must be met before a tree unlocks.
/// Trees can require character level, skill proficiency, or achievements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnlockRequirement {
    /// Character must meet a level threshold.
    Level { comparison: Comparison, value: u32 },
    /// Character must have a skill at a minimum rank and/or level.
    Skill {
        skill_name: String,
        min_rank: Option<MasteryRank>,
        min_level: Option<u32>,
    },
    /// Character must have earned a named achievement.
    Achievement { name: String },
}

/// A milestone bonus that activates when a tree reaches a progress threshold.
/// Trees have exactly 4 milestones at 25%, 50%, 75%, and 100%.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeMilestone {
    pub description: String,
}

/// A single tree with a point pool and milestone bonuses.
/// Points are spent from the character's unspent_tree_points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeDefinition {
    pub name: String,
    pub description: String,
    /// Total points needed to fully complete this tree.
    pub max_points: u32,
    /// Exactly 4 milestones at 25%, 50%, 75%, 100% progress.
    pub milestones: [TreeMilestone; 4],
    /// Prerequisites to unlock this tree.
    pub requirements: Vec<UnlockRequirement>,
}

impl TreeDefinition {
    /// Calculate the point threshold for a milestone (0=25%, 1=50%, 2=75%, 3=100%).
    pub fn milestone_threshold(&self, index: usize) -> u32 {
        let percent = match index {
            0 => 25,
            1 => 50,
            2 => 75,
            3 => 100,
            _ => 100,
        };
        self.max_points * percent / 100
    }

    /// Count how many milestones have been reached at the current point total.
    pub fn milestones_reached(&self, current_points: u32) -> usize {
        (0..4)
            .filter(|&i| current_points >= self.milestone_threshold(i))
            .count()
    }
}

/// A chain of sequential trees with escalating costs.
/// Each step in the chain costs 5x the previous (base_cost * 5^step).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeChain {
    pub name: String,
    /// Point cost for the first tree in the chain.
    pub base_cost: u32,
    /// The individual tree definitions in order.
    pub trees: Vec<TreeDefinition>,
}

impl TreeChain {
    /// Calculate the point cost for a specific step in the chain.
    /// Step 0 = base_cost, step 1 = base_cost * 5, step 2 = base_cost * 25, etc.
    pub fn cost_for_step(&self, step: usize) -> u32 {
        self.base_cost * 5u32.pow(step as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_tree_cost_scaling() {
        let chain = TreeChain {
            name: "Swordsman".to_string(),
            base_cost: 40,
            trees: vec![],
        };
        assert_eq!(chain.cost_for_step(0), 40);   // Tree I: 40
        assert_eq!(chain.cost_for_step(1), 200);  // Tree II: 40 * 5
        assert_eq!(chain.cost_for_step(2), 1000); // Tree III: 40 * 25
        assert_eq!(chain.cost_for_step(3), 5000); // Tree IV: 40 * 125
    }

    #[test]
    fn test_milestone_thresholds() {
        let tree = TreeDefinition {
            name: "Swordsman I".to_string(),
            description: "Basic sword training".to_string(),
            max_points: 40,
            milestones: [
                TreeMilestone { description: "25% bonus".to_string() },
                TreeMilestone { description: "50% bonus".to_string() },
                TreeMilestone { description: "75% bonus".to_string() },
                TreeMilestone { description: "100% bonus".to_string() },
            ],
            requirements: vec![],
        };
        assert_eq!(tree.milestone_threshold(0), 10); // 25% of 40
        assert_eq!(tree.milestone_threshold(1), 20); // 50%
        assert_eq!(tree.milestone_threshold(2), 30); // 75%
        assert_eq!(tree.milestone_threshold(3), 40); // 100%
    }

    #[test]
    fn test_milestones_reached() {
        let tree = TreeDefinition {
            name: "Test".to_string(),
            description: "Test".to_string(),
            max_points: 100,
            milestones: [
                TreeMilestone { description: "M1".to_string() },
                TreeMilestone { description: "M2".to_string() },
                TreeMilestone { description: "M3".to_string() },
                TreeMilestone { description: "M4".to_string() },
            ],
            requirements: vec![],
        };
        assert_eq!(tree.milestones_reached(24), 0);
        assert_eq!(tree.milestones_reached(25), 1);
        assert_eq!(tree.milestones_reached(74), 2);
        assert_eq!(tree.milestones_reached(100), 4);
    }
}
