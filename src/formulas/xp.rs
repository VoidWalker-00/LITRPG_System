/// XP and kill formulas for the leveling system.
///
/// XP formula: XP(L,T) = X * 2^T * (1 + Z*T) * (1 + Y/100 * L)
/// Kill formula: kill_xp = K * (1 + user_level/100) * multiplier
///
/// Where multiplier depends on the level difference:
///   - Within ±10 levels: multiplier = 1
///   - Beyond ±10: multiplier = 2^((D ∓ 10) / 10)

/// Base XP cost at level 0, tier 0.
const X: f64 = 1000.0;

/// Level scaling factor (20% increase per level).
const Y: f64 = 20.0;

/// Tier floor multiplier (each tier adds 5x to the base).
const Z: f64 = 5.0;

/// Base XP per kill before modifiers.
const K: f64 = 100.0;

/// Calculate XP required to advance from the given level at the given tier.
pub fn xp_required(level: u32, tier: u32) -> f64 {
    let tier_exp = 2f64.powi(tier as i32);        // exponential tier scaling
    let tier_floor = 1.0 + Z * tier as f64;       // linear tier floor
    let level_scale = 1.0 + (Y / 100.0) * level as f64; // linear level scaling
    X * tier_exp * tier_floor * level_scale
}

/// Calculate XP earned from killing an enemy at a given level.
/// Level difference (D) determines a multiplier:
///   |D| <= 10  →  1.0 (flat zone)
///   D > 10     →  exponential bonus (harder enemies)
///   D < -10    →  exponential penalty (easier enemies)
pub fn kill_xp(user_level: u32, enemy_level: u32) -> f64 {
    let d = enemy_level as f64 - user_level as f64;
    let multiplier = if d >= -10.0 && d <= 10.0 {
        1.0
    } else if d > 10.0 {
        2f64.powf((d - 10.0) / 10.0)
    } else {
        2f64.powf((d + 10.0) / 10.0)
    };
    K * (1.0 + user_level as f64 / 100.0) * multiplier
}

/// Calculate current XP as a percentage of the level-up requirement.
pub fn xp_percentage(current_xp: f64, level: u32, tier: u32) -> f64 {
    let required = xp_required(level, tier);
    if required <= 0.0 {
        return 0.0;
    }
    (current_xp / required) * 100.0
}

/// Estimate how many kills of a specific enemy level are needed to level up.
pub fn kills_to_level(user_level: u32, enemy_level: u32, tier: u32) -> f64 {
    let required = xp_required(user_level, tier);
    let per_kill = kill_xp(user_level, enemy_level);
    if per_kill <= 0.0 {
        return f64::INFINITY;
    }
    required / per_kill
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_required_tier0_level0() {
        // XP(0, 0) = 1000 * 1 * 1 * 1 = 1000
        assert!((xp_required(0, 0) - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_xp_required_tier0_level50() {
        // XP(50, 0) = 1000 * 1 * 1 * (1 + 0.2 * 50) = 11000
        assert!((xp_required(50, 0) - 11000.0).abs() < 0.01);
    }

    #[test]
    fn test_xp_required_tier1_level0() {
        // XP(0, 1) = 1000 * 2 * 6 * 1 = 12000
        assert!((xp_required(0, 1) - 12000.0).abs() < 0.01);
    }

    #[test]
    fn test_kill_xp_same_level() {
        // D=0, multiplier=1, 100 * (1 + 20/100) = 120
        assert!((kill_xp(20, 20) - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_kill_xp_within_10_levels() {
        // D=5, within ±10, multiplier=1, 100 * 1.2 = 120
        assert!((kill_xp(20, 25) - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_kill_xp_enemy_20_above() {
        // D=20, multiplier = 2^((20-10)/10) = 2, 100 * 1.2 * 2 = 240
        assert!((kill_xp(20, 40) - 240.0).abs() < 0.01);
    }

    #[test]
    fn test_kill_xp_enemy_20_below() {
        // D=-20, multiplier = 2^((-20+10)/10) = 0.5, 100 * 1.2 * 0.5 = 60
        assert!((kill_xp(20, 0) - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_xp_percentage() {
        // Tier 0, level 0 needs 1000 XP. 500 XP => 50%
        assert!((xp_percentage(500.0, 0, 0) - 50.0).abs() < 0.01);
    }
}
