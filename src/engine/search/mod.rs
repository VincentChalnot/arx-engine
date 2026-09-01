//! Root search orchestration: parallel root-move evaluation using Rayon.

pub mod alpha_beta;
pub mod killer;
pub mod loop_detection;
pub mod move_ordering;
pub mod negamax;
pub mod quiescence;
pub mod rng;

use crate::engine::constants::MAX_KILLER_DEPTH;
use crate::engine::search::killer::KillerTable;
use crate::engine::search::loop_detection::LoopDetector;
use crate::engine::search::negamax::negamax;
use crate::engine::search::rng::Rng;
use crate::engine::tree_recorder::TreeRecorder;
use crate::engine::tt::TranspositionTable;
use crate::engine::types::SearchConfig;
use crate::game::Game;
use crate::moves::Move;
use rayon::prelude::*;
use std::time::Instant;

/// Statistics collected during a root search.
#[derive(Debug, Default)]
pub struct SearchStats {
    /// Number of leaf or quiescence nodes visited.
    pub nodes_visited: u64,
    /// Time elapsed during the search.
    pub elapsed: std::time::Duration,
}

/// Result of a root search.
#[derive(Debug)]
pub struct RootSearchResult {
    /// Best move found.
    pub best_move: Option<Move>,
    /// Score for the best move (NegaMax-relative).
    pub best_score: i32,
    /// Principal variation (list of moves from root to leaf).
    pub pv: Vec<Move>,
    /// Search statistics.
    pub stats: SearchStats,
}

/// Run a full root search on `game` using Rayon at the root level.
///
/// Each root move is evaluated in its own Rayon task, with a per-thread
/// transposition table shard (shared via DashMap) and independent killer/
/// loop-detector state.
///
/// `game_history` is an optional slice of board hashes for positions already
/// played in the real game. These are pre-loaded into each thread's
/// `LoopDetector` so that the engine avoids repeating any previously seen
/// position (draw by repetition from the 1st recurrence).
pub fn root_search(
    game: &Game,
    config: &SearchConfig,
    game_history: &[u64],
    recorder: Option<&TreeRecorder>,
) -> RootSearchResult {
    let start = Instant::now();

    // Generate all root moves.
    let potential_moves = game.get_all_moves();
    let root_moves: Vec<Move> = potential_moves
        .iter()
        .flat_map(|pm| pm.to_moves())
        .collect();

    if root_moves.is_empty() {
        return RootSearchResult {
            best_move: None,
            best_score: -crate::engine::constants::KING_VALUE,
            pv: vec![],
            stats: SearchStats {
                nodes_visited: 0,
                elapsed: start.elapsed(),
            },
        };
    }

    // Evaluate each root move in parallel.
    let results: Vec<(Move, i32)> = root_moves
        .par_iter()
        .map(|&mv| {
            let mut game_clone = game.clone();
            let undo = game_clone.make_unchecked(&mv);
            if undo.is_king_captured() {
                game_clone.unmake(&mv, undo);
                return (mv, crate::engine::constants::KING_VALUE);
            }

            let mut local_tt = TranspositionTable::new(crate::engine::constants::TT_SIZE / 8);
            let tt_ptr = Some(&mut local_tt as *mut TranspositionTable);
            let mut ld = LoopDetector::new();
            // Pre-populate with real-game history so the engine avoids repeating
            // any already-played position (draw by repetition from 1st recurrence).
            for &h in game_history {
                let _ = ld.push(h);
            }
            let root_hash = game.board_hash();
            let _ = ld.push(root_hash);

            let mut killers = KillerTable::new(MAX_KILLER_DEPTH);

            let score = -negamax(
                &mut game_clone,
                1,
                -crate::engine::constants::KING_VALUE,
                crate::engine::constants::KING_VALUE,
                config,
                &mut ld,
                &mut killers,
                tt_ptr,
                recorder,
                0,
            );

            game_clone.unmake(&mv, undo);
            (mv, score)
        })
        .collect();

    // Pick the root move to play. `results` already holds every legal root
    // move searched to full depth, so `select_root_move` picks among
    // already-computed scores — see `SearchConfig::noise_temperature` /
    // `blunder_chance` for what makes it deviate from the true best move.
    let mut rng = Rng::new();
    let (best_move, best_score) = select_root_move(&results, config, &mut rng)
        .map(|(mv, s)| (Some(mv), s))
        .unwrap_or((None, -crate::engine::constants::KING_VALUE));

    // Extract PV by re-running a single-threaded search and following best moves.
    let pv = if let Some(bm) = best_move {
        extract_pv(game, bm, config)
    } else {
        vec![]
    };

    RootSearchResult {
        best_move,
        best_score,
        pv,
        stats: SearchStats {
            nodes_visited: 0, // simplified; full counting omitted for brevity
            elapsed: start.elapsed(),
        },
    }
}

/// Pick which already-searched root move to actually play.
///
/// `results` holds every legal root move with its full-depth score (NegaMax
/// relative to the side to move). With `config.blunder_chance` and
/// `config.noise_temperature` both `0.0` (the default) this always returns
/// the highest-scoring move, unchanged from a plain `max_by_key`.
///
/// Otherwise, first roll for an outright blunder (`blunder_chance`): ignore
/// every score and play a uniformly random legal move. Failing that, apply
/// softmax noise at `noise_temperature`: every move gets picked with
/// probability proportional to `exp((score - best_score) / temperature)`, so
/// the true best move is always the likeliest pick, nearby-scoring moves get
/// a real chance, and clearly worse ones fade out fast without a hard
/// cutoff.
fn select_root_move(
    results: &[(Move, i32)],
    config: &SearchConfig,
    rng: &mut Rng,
) -> Option<(Move, i32)> {
    if results.is_empty() {
        return None;
    }

    if config.blunder_chance > 0.0 && rng.next_f32() < config.blunder_chance {
        let idx = ((rng.next_f32() * results.len() as f32) as usize).min(results.len() - 1);
        return Some(results[idx]);
    }

    let best_score = results.iter().map(|&(_, s)| s).max().unwrap();

    if config.noise_temperature <= 0.0 {
        return results.iter().copied().max_by_key(|&(_, s)| s);
    }

    let weights: Vec<f32> = results
        .iter()
        .map(|&(_, s)| ((s - best_score) as f32 / config.noise_temperature).exp())
        .collect();
    let total: f32 = weights.iter().sum();
    let mut pick = rng.next_f32() * total;
    for (i, &w) in weights.iter().enumerate() {
        pick -= w;
        if pick <= 0.0 {
            return Some(results[i]);
        }
    }
    results.last().copied()
}

/// Re-run the search from `root` → `first_move` and walk down the TT best
/// moves to extract the principal variation.
fn extract_pv(game: &Game, first_move: Move, config: &SearchConfig) -> Vec<Move> {
    let mut pv = vec![first_move];
    let mut game_clone = game.clone();
    let undo = game_clone.make_unchecked(&first_move);
    if undo.is_king_captured() {
        game_clone.unmake(&first_move, undo);
        return pv;
    }

    let mut tt = TranspositionTable::new(crate::engine::constants::TT_SIZE);
    let tt_ptr = Some(&mut tt as *mut TranspositionTable);
    let mut ld = LoopDetector::new();
    let mut killers = KillerTable::new(MAX_KILLER_DEPTH);

    let _ = negamax(
        &mut game_clone,
        1,
        -crate::engine::constants::KING_VALUE,
        crate::engine::constants::KING_VALUE,
        config,
        &mut ld,
        &mut killers,
        tt_ptr,
        None,
        0,
    );

    game_clone.unmake(&first_move, undo);

    // Walk the TT best moves starting from depth 2.
    let mut depth = 2usize;
    let mut cur_game = game.clone();
    cur_game.make_unchecked(&first_move);
    let max_depth = config.max_depth;

    while depth <= max_depth {
        let hash = cur_game.board_hash();
        if let Some(entry) = tt.get(hash) {
            if let Some(bm) = entry.best_move {
                pv.push(bm);
                cur_game.make_unchecked(&bm);
                depth += 1;
                continue;
            }
        }
        break;
    }

    pv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, Color, Piece, PieceType, Position};
    use crate::game::Game;

    fn minimal_game() -> Game {
        let mut board = Board::empty();
        board.set_piece(
            &Position::new(4, 8),
            Some(Piece::new(Color::White, PieceType::King, None)),
        );
        board.set_piece(
            &Position::new(4, 0),
            Some(Piece::new(Color::Black, PieceType::King, None)),
        );
        Game::from_board(board)
    }

    #[test]
    fn root_search_returns_a_move() {
        let game = minimal_game();
        let config = SearchConfig {
            max_depth: 2,
            ..Default::default()
        };
        let result = root_search(&game, &config, &[], None);
        assert!(
            result.best_move.is_some(),
            "Expected a move from root search"
        );
    }

    #[test]
    fn root_search_with_material_advantage_has_positive_score() {
        let mut game = minimal_game();
        game.board.set_piece(
            &Position::new(3, 5),
            Some(Piece::new(Color::White, PieceType::Rook, None)),
        );
        let config = SearchConfig {
            max_depth: 2,
            ..Default::default()
        };
        let result = root_search(&game, &config, &[], None);
        assert!(
            result.best_score > 0,
            "Expected positive score for material advantage, got {}",
            result.best_score
        );
    }

    #[test]
    fn root_search_accepts_game_history_and_returns_move() {
        // Build a minimal game and pass the initial position hash as game history.
        // The call must not panic and must still return a valid move even when
        // the engine is instructed to treat a revisit to the root as a draw.
        let game = minimal_game();
        let initial_hash = game.board_hash();

        let config = SearchConfig {
            max_depth: 2,
            ..Default::default()
        };
        let result = root_search(&game, &config, &[initial_hash], None);
        // A move should still be found (the engine selects among non-repeating moves).
        assert!(
            result.best_move.is_some(),
            "Expected a move even with game history present"
        );
    }

    fn fake_results() -> Vec<(Move, i32)> {
        vec![
            (
                Move {
                    from: Position::new(0, 0),
                    to: Position::new(0, 1),
                    unstack: false,
                },
                10,
            ),
            (
                Move {
                    from: Position::new(1, 0),
                    to: Position::new(1, 1),
                    unstack: false,
                },
                30,
            ),
            (
                Move {
                    from: Position::new(2, 0),
                    to: Position::new(2, 1),
                    unstack: false,
                },
                20,
            ),
        ]
    }

    #[test]
    fn select_root_move_is_deterministic_argmax_at_zero_noise() {
        let results = fake_results();
        let config = SearchConfig::default(); // noise_temperature == 0.0, blunder_chance == 0.0
        for seed in 0..20u64 {
            let mut rng = Rng::seeded(seed + 1);
            let (_, score) = select_root_move(&results, &config, &mut rng).unwrap();
            assert_eq!(score, 30);
        }
    }

    #[test]
    fn select_root_move_always_blunders_at_full_blunder_chance() {
        let results = fake_results();
        let config = SearchConfig {
            blunder_chance: 1.0,
            ..Default::default()
        };
        let mut rng = Rng::seeded(7);
        let mut saw_non_best = false;
        for _ in 0..50 {
            let (_, score) = select_root_move(&results, &config, &mut rng).unwrap();
            assert!(results.iter().any(|&(_, s)| s == score));
            if score != 30 {
                saw_non_best = true;
            }
        }
        assert!(
            saw_non_best,
            "expected a full blunder chance to sometimes pick a non-best move"
        );
    }

    #[test]
    fn select_root_move_with_noise_sometimes_picks_a_non_best_move() {
        let results = fake_results();
        let config = SearchConfig {
            noise_temperature: 15.0,
            ..Default::default()
        };
        let mut rng = Rng::seeded(99);
        let mut saw_non_best = false;
        for _ in 0..200 {
            let (_, score) = select_root_move(&results, &config, &mut rng).unwrap();
            assert!(results.iter().any(|&(_, s)| s == score));
            if score != 30 {
                saw_non_best = true;
            }
        }
        assert!(
            saw_non_best,
            "expected some noise to occasionally pick a non-best move"
        );
    }

    #[test]
    fn select_root_move_returns_none_for_empty_results() {
        let config = SearchConfig::default();
        let mut rng = Rng::seeded(3);
        assert!(select_root_move(&[], &config, &mut rng).is_none());
    }
}
