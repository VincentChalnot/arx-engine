//! Shared AI types used across the engine.

use crate::moves::Move;

/// Whether a transposition table score is exact, a lower bound, or an upper bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundType {
    /// The score is exact.
    Exact,
    /// The score is a lower bound (failed high / beta cutoff).
    LowerBound,
    /// The score is an upper bound (failed low).
    UpperBound,
}

/// The result returned from a NegaMax search call.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// NegaMax-relative score (positive = good for the side to move).
    pub score: i32,
    /// Best move found at this node (None at leaves / when no moves).
    pub best_move: Option<Move>,
}

/// Configuration flags that can disable individual engine features, plus the
/// dials that control how much the root move choice deviates from the
/// engine's true best move (see `for_level`).
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// Enable transposition table.
    pub use_tt: bool,
    /// Enable alpha-beta pruning.
    pub use_alpha_beta: bool,
    /// Enable quiescence search.
    pub use_quiescence: bool,
    /// Enable killer move heuristic.
    pub use_killers: bool,
    /// Maximum search depth.
    pub max_depth: usize,
    /// Softmax temperature (in eval-score units) used when picking the root
    /// move. `0.0` always plays the true best move (deterministic argmax).
    /// Above `0.0`, other root moves get picked with probability that decays
    /// the further their score is below the best — see
    /// `search::select_root_move`. Every root move is already searched to
    /// full depth independently (see `root_search`), so this costs no extra
    /// search time; it only changes which of the already-computed scores
    /// gets played.
    pub noise_temperature: f32,
    /// Probability `[0.0, 1.0]` of ignoring the search result entirely and
    /// playing a uniformly random legal root move instead — an outright
    /// blunder, independent of `noise_temperature`.
    pub blunder_chance: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            use_tt: true,
            use_alpha_beta: true,
            use_quiescence: true,
            use_killers: true,
            max_depth: crate::engine::constants::MAX_DEPTH,
            noise_temperature: 0.0,
            blunder_chance: 0.0,
        }
    }
}

impl SearchConfig {
    /// Build a config for an engine strength `level`, clamped to
    /// `[MIN_LEVEL, MAX_LEVEL]`. This is the single place that maps a
    /// user-facing difficulty knob onto engine feature flags, so new dials
    /// get added here rather than scattered across callers.
    ///
    /// `use_tt` and `use_alpha_beta` stay on regardless of level: both are
    /// transparent optimizations that only affect search speed, not the
    /// move the engine picks, so disabling them would not weaken play —
    /// only slow it down.
    ///
    /// `noise_temperature` and `blunder_chance` ease off together as level
    /// increases, ramping linearly from "complete beginner" (level 1: heavy
    /// noise and blunders) down to zero at `MAX_LEVEL - 1`. `max_depth` ramps
    /// on its own, separately-spaced schedule: two levels per ply, from depth
    /// 1 at levels 1-2 up to depth 4 by level 7 — evenly distributing the
    /// jumps in playing strength that an extra search ply brings across the
    /// whole 1..=MAX_LEVEL-1 range, rather than bunching them at the top.
    /// Levels 7-9 all sit at depth 4 and are told apart purely by their
    /// (still-decreasing) noise and blunder chance, landing on zero at
    /// `MAX_LEVEL - 1` — "one ply short of full strength". `MAX_LEVEL` itself
    /// is a step above that ramp (depth 5, still no noise) — the "no mercy"
    /// top tier — which reproduces the old always-play-the-best-move-at-
    /// full-depth behavior exactly.
    ///
    /// Quiescence/killers switch on once `max_depth >= 3`: below that the
    /// search is already so shallow that the extra tactical accuracy they
    /// buy just makes it feel erratic rather than weak.
    pub fn for_level(level: u8) -> Self {
        use crate::engine::constants::{MAX_LEVEL, MIN_LEVEL};
        let level = level.clamp(MIN_LEVEL, MAX_LEVEL);

        let (max_depth, noise_temperature, blunder_chance) = if level == MAX_LEVEL {
            (5, 0.0, 0.0)
        } else {
            // 0.0 at level 1, 1.0 at level `MAX_LEVEL - 1`.
            let t = (level - 1) as f32 / (MAX_LEVEL - 2) as f32;
            // Two levels per ply: 1-2 -> depth 1, 3-4 -> depth 2, ...,
            // capped at depth 4 (reached at level 7 and held through 9).
            let max_depth = 1 + (((level - 1) / 2) as usize).min(3);
            (max_depth, 35.0 * (1.0 - t), 0.20 * (1.0 - t))
        };
        let use_quiescence_and_killers = max_depth >= 3;

        SearchConfig {
            use_tt: true,
            use_alpha_beta: true,
            use_quiescence: use_quiescence_and_killers,
            use_killers: use_quiescence_and_killers,
            max_depth,
            noise_temperature,
            blunder_chance,
        }
    }
}

/// Per-square evaluation detail (for verbose / debug mode).
#[derive(Clone, Debug)]
pub struct SquareEval {
    pub piece_type: String,
    pub color: String,
    pub base_value: i32,
    pub pst_bonus: i32,
    pub mobility_bonus: i32,
    pub promotion_bonus: i32,
    pub total: i32,
}

/// Full board evaluation detail (for verbose / debug mode).
#[derive(Clone, Debug)]
pub struct BoardEval {
    pub per_square: std::collections::HashMap<(usize, usize), SquareEval>,
    pub white_total: i32,
    pub black_total: i32,
    pub pinned_malus_white: i32,
    pub pinned_malus_black: i32,
    pub king_mobility_white: i32,
    pub king_mobility_black: i32,
    pub tempo: i32,
    pub final_score: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_config_defaults_are_all_enabled() {
        let cfg = SearchConfig::default();
        assert!(cfg.use_tt);
        assert!(cfg.use_alpha_beta);
        assert!(cfg.use_quiescence);
        assert!(cfg.use_killers);
    }

    #[test]
    fn for_level_gates_quiescence_and_killers_on_depth() {
        for level in 1..=10u8 {
            let cfg = SearchConfig::for_level(level);
            assert_eq!(cfg.use_quiescence, cfg.max_depth >= 3);
            assert_eq!(cfg.use_killers, cfg.max_depth >= 3);
            assert!(cfg.use_tt);
            assert!(cfg.use_alpha_beta);
        }
    }

    #[test]
    fn for_level_matches_the_documented_anchor_points() {
        // Level 1: complete beginner.
        let l1 = SearchConfig::for_level(1);
        assert_eq!(l1.max_depth, 1);
        assert_eq!(l1.noise_temperature, 35.0);
        assert_eq!(l1.blunder_chance, 0.20);

        // Level 9: one ply short of full strength, noise/blunder already at 0.
        let l9 = SearchConfig::for_level(9);
        assert_eq!(l9.max_depth, 4);
        assert_eq!(l9.noise_temperature, 0.0);
        assert_eq!(l9.blunder_chance, 0.0);

        // Level 10 (MAX_LEVEL): full strength, one ply deeper than level 9.
        let l10 = SearchConfig::for_level(10);
        assert_eq!(l10.max_depth, 5);
        assert_eq!(l10.noise_temperature, 0.0);
        assert_eq!(l10.blunder_chance, 0.0);
    }

    #[test]
    fn for_level_spreads_depth_increases_two_levels_per_ply() {
        // Levels 1-9 map onto depth 1..=4 two levels at a time (level 9
        // shares depth 4 with 7-8, told apart only by noise/blunder), rather
        // than bunching most levels at low depth and jumping late.
        let expected_depths = [1, 1, 2, 2, 3, 3, 4, 4, 4];
        for (i, &expected) in expected_depths.iter().enumerate() {
            let level = (i + 1) as u8;
            assert_eq!(
                SearchConfig::for_level(level).max_depth,
                expected,
                "level {level}"
            );
        }
    }

    #[test]
    fn for_level_clamps_out_of_range_values() {
        assert_eq!(SearchConfig::for_level(0).max_depth, 1);
        assert_eq!(SearchConfig::for_level(11).max_depth, 5);
    }

    #[test]
    fn for_level_depth_noise_and_blunder_chance_are_monotonic_across_the_full_scale() {
        let mut prev_depth = 0;
        let mut prev_temp = f32::MAX;
        let mut prev_blunder = f32::MAX;
        for level in 1..=10u8 {
            let cfg = SearchConfig::for_level(level);
            assert!(cfg.max_depth >= prev_depth);
            assert!(cfg.noise_temperature <= prev_temp);
            assert!(cfg.blunder_chance <= prev_blunder);
            assert!((0.0..=1.0).contains(&cfg.blunder_chance));
            prev_depth = cfg.max_depth;
            prev_temp = cfg.noise_temperature;
            prev_blunder = cfg.blunder_chance;
        }
    }

    #[test]
    fn bound_type_variants_are_distinct() {
        assert_ne!(BoundType::Exact, BoundType::LowerBound);
        assert_ne!(BoundType::Exact, BoundType::UpperBound);
        assert_ne!(BoundType::LowerBound, BoundType::UpperBound);
    }
}
