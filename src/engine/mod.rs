//! Monte Carlo Tree Search Engine for Arx
//!
//! This module provides a GPU-accelerated MCTS engine for evaluating board positions
//! and finding optimal moves. The engine is completely independent from the main
//! game logic (board.rs and game.rs) and implements its own simplified move application
//! and evaluation functions.
//!
//! # Features
//!
//! - GPU-accelerated move generation via compute shaders
//! - GPU-accelerated batch simulation for move application and evaluation
//! - Multi-threaded CPU processing with Rayon
//! - Configurable search depth and simulation count
//! - Piece value-based position evaluation
//! - Adjustable engine strength
//! - Statistics tracking (moves evaluated, simulations run)
//! - Position caching for instant lookup of previously evaluated positions
//!
//! # Caching
//!
//! The engine maintains an in-memory cache of evaluated board positions. When a position
//! is evaluated, the best move and its score are stored in RAM. On subsequent calls with
//! the same position, the engine returns the cached result immediately without re-running
//! the search algorithm. This significantly improves performance when analyzing positions
//! that appear multiple times (e.g., transpositions in game trees).
//!
//! Cache statistics (hits, misses, hit rate) are tracked and can be accessed via
//! `get_statistics()`. The cache can be cleared with `clear_cache()` if needed.
//!
//! # Example
//!
//! ```no_run
//! use arx_engine::engine::{MctsEngine, EngineConfig};
//!
//! // Create engine with custom configuration
//! let config = EngineConfig {
//!     max_depth: 3,
//!     simulations_per_move: 100,
//!     exploration_constant: 1.414,
//!     gpu_batch_size: 256,
//!     use_gpu_simulation: true,
//! };
//! let mut engine = MctsEngine::with_config(config).expect("Failed to create engine");
//!
//! // Find best move for a board position
//! let board_state = [0u8; 82]; // Your board state
//! let best_move = engine.find_best_move(&board_state).expect("No legal moves");
//!
//! // Get search statistics including cache performance
//! let stats = engine.get_statistics();
//! println!("Moves evaluated: {}", stats.total_moves_evaluated);
//! println!("Simulations run: {}", stats.simulations_run);
//! println!("Cache hits: {}", stats.cache_hits);
//! println!("Cache hit rate: {:.2}%", stats.cache_hit_rate() * 100.0);
//!
//! // Clear cache if needed (e.g., when starting a new game)
//! engine.clear_cache();
//! ```
//!

use rand::Rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

mod gpu_context;
pub use gpu_context::{GpuContext, get_shared_context};

mod gpu_move_gen;
pub use gpu_move_gen::MoveGenerationEngine;

mod gpu_batch_sim;
pub use gpu_batch_sim::BatchSimulationEngine;

const BOARD_SIZE: usize = 81;

/// Piece values for evaluation (based on chess piece values, scaled with Soldier=1)
const PIECE_VALUES: [i32; 8] = [
    0,  // Index 0: unused
    1,  // Soldier
    3,  // Jester (like Bishop)
    5,  // Commander (like Rook)
    3,  // Paladin (like Bishop-lite)
    3,  // Guard (like Bishop-lite)
    3,  // Dragon (like Knight)
    5,  // Ballista (like Rook-lite)
];

const KING_VALUE: i32 = 1000; // King is invaluable

/// Engine configuration
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Maximum search depth
    pub max_depth: u32,
    /// Number of simulations per move evaluation
    pub simulations_per_move: u32,
    /// Exploration constant for UCB1
    pub exploration_constant: f32,
    /// Batch size for GPU processing (number of simulations processed in parallel)
    pub gpu_batch_size: usize,
    /// Enable GPU-accelerated batch simulation (if false, uses CPU fallback)
    pub use_gpu_simulation: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            simulations_per_move: 100,
            exploration_constant: 1.414,
            gpu_batch_size: 256,
            use_gpu_simulation: true,
        }
    }
}

/// Statistics for MCTS search
#[derive(Clone, Debug, Default)]
pub struct SearchStatistics {
    /// Total number of moves evaluated across all simulations
    pub total_moves_evaluated: u64,
    /// Number of simulations run
    pub simulations_run: u64,
    /// Number of moves evaluated in the most recent search
    pub last_search_moves: u64,
    /// Number of GPU batches processed
    pub gpu_batches_processed: u64,
    /// Number of CPU simulations (fallback)
    pub cpu_simulations: u64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
}

impl SearchStatistics {
    /// Reset statistics
    pub fn reset(&mut self) {
        self.total_moves_evaluated = 0;
        self.simulations_run = 0;
        self.last_search_moves = 0;
        self.gpu_batches_processed = 0;
        self.cpu_simulations = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    /// Get average moves per simulation
    pub fn avg_moves_per_simulation(&self) -> f64 {
        if self.simulations_run == 0 {
            0.0
        } else {
            self.total_moves_evaluated as f64 / self.simulations_run as f64
        }
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Cached evaluation data for a board position
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields are stored for potential future use (e.g., incremental updates)
struct CachedEvaluation {
    /// Best move found for this position
    best_move: u16,
    /// Average score of the best move
    avg_score: f32,
    /// Number of simulations that contributed to this evaluation
    simulations: u32,
}

/// Monte Carlo Tree Search Engine
pub struct MctsEngine {
    config: EngineConfig,
    move_gen: MoveGenerationEngine,
    batch_sim: Option<BatchSimulationEngine>,
    stats: Arc<AtomicStats>,
    /// Cache for board position evaluations
    cache: Arc<Mutex<HashMap<[u8; 82], CachedEvaluation>>>,
}

/// Atomic statistics for thread-safe updates
struct AtomicStats {
    total_moves: AtomicU64,
    simulations: AtomicU64,
    gpu_batches: AtomicU64,
    cpu_sims: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl AtomicStats {
    fn new() -> Self {
        Self {
            total_moves: AtomicU64::new(0),
            simulations: AtomicU64::new(0),
            gpu_batches: AtomicU64::new(0),
            cpu_sims: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    fn to_statistics(&self, last_search_moves: u64) -> SearchStatistics {
        SearchStatistics {
            total_moves_evaluated: self.total_moves.load(Ordering::Relaxed),
            simulations_run: self.simulations.load(Ordering::Relaxed),
            last_search_moves,
            gpu_batches_processed: self.gpu_batches.load(Ordering::Relaxed),
            cpu_simulations: self.cpu_sims.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.total_moves.store(0, Ordering::Relaxed);
        self.simulations.store(0, Ordering::Relaxed);
        self.gpu_batches.store(0, Ordering::Relaxed);
        self.cpu_sims.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

impl MctsEngine {
    /// Create a new MCTS engine with default configuration
    pub fn new() -> Result<Self, String> {
        Self::with_config(EngineConfig::default())
    }

    /// Create a new MCTS engine with custom configuration
    pub fn with_config(config: EngineConfig) -> Result<Self, String> {
        let move_gen = MoveGenerationEngine::new_sync()?;
        
        // Try to create batch simulation engine if GPU simulation is enabled
        let batch_sim = if config.use_gpu_simulation {
            match BatchSimulationEngine::new_sync() {
                Ok(engine) => {
                    eprintln!("✓ GPU batch simulation engine initialized");
                    Some(engine)
                }
                Err(e) => {
                    eprintln!("⚠ GPU batch simulation unavailable: {}", e);
                    eprintln!("  Falling back to CPU simulation");
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            config,
            move_gen,
            batch_sim,
            stats: Arc::new(AtomicStats::new()),
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Evaluate a board position and return the value
    /// Positive values favor the current player
    fn evaluate_board(&self, board: &[u8; 82]) -> i32 {
        let white_to_move = board[81] == 1;
        let mut white_value = 0;
        let mut black_value = 0;

        for i in 0..BOARD_SIZE {
            let piece = board[i];
            if piece == 0 {
                continue;
            }

            let is_white = (piece >> 6) == 1;
            let payload = piece & 0x3F;

            // Check for King
            if payload == 0x38 {
                if is_white {
                    white_value += KING_VALUE;
                } else {
                    black_value += KING_VALUE;
                }
                continue;
            }

            let top_code = (payload >> 3) & 0x07;
            let bottom_code = payload & 0x07;

            // Add value for bottom piece
            if bottom_code > 0 && (bottom_code as usize) < PIECE_VALUES.len() {
                let value = PIECE_VALUES[bottom_code as usize];
                if is_white {
                    white_value += value;
                } else {
                    black_value += value;
                }
            }

            // Add value for top piece if stacked
            if top_code > 0 && (top_code as usize) < PIECE_VALUES.len() {
                let value = PIECE_VALUES[top_code as usize];
                if is_white {
                    white_value += value;
                } else {
                    black_value += value;
                }
            }
        }

        // Return value from perspective of current player
        if white_to_move {
            white_value - black_value
        } else {
            black_value - white_value
        }
    }

    /// Apply a move to a board state (simplified version without full game logic)
    /// This is a GPU-independent implementation for the engine
    fn apply_move_simple(&self, board: &[u8; 82], move_encoding: u16) -> Result<[u8; 82], String> {
        let mut new_board = board.clone();
        
        let from = (move_encoding & 0x7F) as usize;
        let to = ((move_encoding >> 7) & 0x7F) as usize;
        let unstack = (move_encoding & 0x4000) != 0;

        if from >= BOARD_SIZE || to >= BOARD_SIZE {
            return Err("Invalid move: position out of bounds".to_string());
        }

        let piece = board[from];
        if piece == 0 {
            return Err("No piece at source position".to_string());
        }

        if unstack {
            // Unstack top piece
            let payload = piece & 0x3F;
            let top_code = (payload >> 3) & 0x07;
            let bottom_code = payload & 0x07;
            let color_bit = piece & 0x40;

            if top_code == 0 {
                return Err("Cannot unstack: no top piece".to_string());
            }

            // Create new bottom piece (remove top)
            new_board[from] = color_bit | bottom_code;

            // Create moving piece (top becomes new bottom)
            let moving_piece = color_bit | top_code;

            // Place at destination (simple: just replace, no stacking logic)
            new_board[to] = moving_piece;
        } else {
            // Move entire piece/stack
            new_board[from] = 0;
            new_board[to] = piece; // Simplified: just capture/replace
        }

        // Switch turn
        new_board[81] = if new_board[81] == 1 { 0 } else { 1 };

        Ok(new_board)
    }

    /// Run simulations from a given board state
    fn simulate(&self, board: &[u8; 82], depth: u32) -> i32 {
        // Terminal condition: max depth reached or game over
        if depth >= self.config.max_depth {
            return self.evaluate_board(board);
        }

        // Generate legal moves
        let moves = match self.move_gen.generate_moves(board) {
            Ok(m) => m,
            Err(_) => return self.evaluate_board(board), // No moves, evaluate position
        };

        if moves.is_empty() {
            return self.evaluate_board(board);
        }

        // Simple rollout: pick random move and continue
        let mut rng = rand::thread_rng();
        let random_move = moves[rng.gen_range(0..moves.len())];

        match self.apply_move_simple(board, random_move) {
            Ok(new_board) => -self.simulate(&new_board, depth + 1), // Negate for opponent's perspective
            Err(_) => self.evaluate_board(board), // Invalid move, evaluate current position
        }
    }

    /// Find the best move using MCTS with GPU acceleration and multi-threading
    pub fn find_best_move(&mut self, board: &[u8; 82]) -> Result<u16, String> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(board) {
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.best_move);
            }
        }
        
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        // Reset search-specific stats
        let search_start_moves = self.stats.total_moves.load(Ordering::Relaxed);
        
        // Generate all legal moves
        let moves = self.move_gen.generate_moves(board)?;

        if moves.is_empty() {
            return Err("No legal moves available".to_string());
        }

        if moves.len() == 1 {
            // Cache the single move before returning
            let best_move = moves[0];
            let mut cache = self.cache.lock().unwrap();
            cache.insert(*board, CachedEvaluation {
                best_move,
                avg_score: 0.0, // No evaluation needed for forced move
                simulations: 0,
            });
            return Ok(best_move);
        }

        // Use GPU batch processing if available
        if let Some(ref batch_sim) = self.batch_sim {
            self.find_best_move_gpu(board, &moves, batch_sim, search_start_moves)
        } else {
            self.find_best_move_cpu(board, &moves, search_start_moves)
        }
    }

    /// GPU-accelerated move evaluation with batch processing
    fn find_best_move_gpu(
        &self,
        board: &[u8; 82],
        moves: &[u16],
        batch_sim: &BatchSimulationEngine,
        _search_start_moves: u64,
    ) -> Result<u16, String> {
        // Evaluate each move using parallel processing
        let move_scores: Vec<(u16, i32, u32)> = moves
            .par_iter()
            .map(|&mv| {
                let mut total_score = 0i32;
                let mut valid_simulations = 0u32;
                let mut moves_evaluated = 0u64;

                // Process simulations in batches
                let batch_size = self.config.gpu_batch_size;
                let num_batches = (self.config.simulations_per_move as usize + batch_size - 1) / batch_size;

                for batch_idx in 0..num_batches {
                    let sims_in_batch = batch_size.min(
                        self.config.simulations_per_move as usize - batch_idx * batch_size
                    );

                    // Prepare batch: apply initial move and create boards for simulation
                    let mut batch_boards = Vec::with_capacity(sims_in_batch);
                    let mut batch_moves = Vec::with_capacity(sims_in_batch);

                    for _ in 0..sims_in_batch {
                        batch_boards.push(*board);
                        batch_moves.push(mv);
                    }

                    // Process batch on GPU
                    match batch_sim.process_batch(&batch_boards, &batch_moves) {
                        Ok(results) => {
                            self.stats.gpu_batches.fetch_add(1, Ordering::Relaxed);
                            
                            for result in results {
                                if result.valid {
                                    // Negate score for opponent's perspective
                                    total_score -= result.score;
                                    valid_simulations += 1;
                                    moves_evaluated += 1;
                                }
                            }
                        }
                        Err(_) => {
                            // Fall back to CPU for this batch
                            self.stats.cpu_sims.fetch_add(sims_in_batch as u64, Ordering::Relaxed);
                            for _ in 0..sims_in_batch {
                                if let Ok(new_board) = self.apply_move_simple(board, mv) {
                                    let score = -self.simulate(&new_board, 1);
                                    total_score += score;
                                    valid_simulations += 1;
                                    moves_evaluated += 1;
                                }
                            }
                        }
                    }
                }

                self.stats.simulations.fetch_add(valid_simulations as u64, Ordering::Relaxed);
                self.stats.total_moves.fetch_add(moves_evaluated, Ordering::Relaxed);

                (mv, total_score, valid_simulations)
            })
            .collect();

        // Find move with best average score
        let best_result = move_scores
            .iter()
            .filter(|(_, _, sims)| *sims > 0)
            .max_by(|a, b| {
                let avg_a = a.1 as f32 / a.2 as f32;
                let avg_b = b.1 as f32 / b.2 as f32;
                avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or("No valid moves found")?;

        let best_move = best_result.0;
        let avg_score = best_result.1 as f32 / best_result.2 as f32;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(*board, CachedEvaluation {
                best_move,
                avg_score,
                simulations: best_result.2,
            });
        }

        Ok(best_move)
    }

    /// CPU-based move evaluation with multi-threading (fallback)
    fn find_best_move_cpu(
        &self,
        board: &[u8; 82],
        moves: &[u16],
        _search_start_moves: u64,
    ) -> Result<u16, String> {
        // Evaluate each move using parallel processing
        let move_scores: Vec<(u16, i32, u32)> = moves
            .par_iter()
            .map(|&mv| {
                let mut total_score = 0;
                let mut simulations = 0;

                for _ in 0..self.config.simulations_per_move {
                    match self.apply_move_simple(board, mv) {
                        Ok(new_board) => {
                            let score = -self.simulate(&new_board, 1);
                            total_score += score;
                            simulations += 1;
                        }
                        Err(_) => continue, // Skip invalid moves
                    }
                }

                self.stats.simulations.fetch_add(simulations as u64, Ordering::Relaxed);
                self.stats.cpu_sims.fetch_add(simulations as u64, Ordering::Relaxed);
                self.stats.total_moves.fetch_add(simulations as u64, Ordering::Relaxed);

                (mv, total_score, simulations)
            })
            .collect();

        // Find move with best average score
        let best_result = move_scores
            .iter()
            .filter(|(_, _, sims)| *sims > 0)
            .max_by(|a, b| {
                let avg_a = a.1 as f32 / a.2 as f32;
                let avg_b = b.1 as f32 / b.2 as f32;
                avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or("No valid moves found")?;

        let best_move = best_result.0;
        let avg_score = best_result.1 as f32 / best_result.2 as f32;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(*board, CachedEvaluation {
                best_move,
                avg_score,
                simulations: best_result.2,
            });
        }

        Ok(best_move)
    }

    /// Get search statistics
    pub fn get_statistics(&self) -> SearchStatistics {
        let current_moves = self.stats.total_moves.load(Ordering::Relaxed);
        self.stats.to_statistics(current_moves)
    }

    /// Reset search statistics
    pub fn reset_statistics(&mut self) {
        self.stats.reset();
    }

    /// Clear the position evaluation cache
    pub fn clear_cache(&mut self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get the current cache size (number of cached positions)
    pub fn cache_size(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// Get the current configuration
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: EngineConfig) {
        // Check if we need to initialize batch sim before moving config
        let use_gpu = config.use_gpu_simulation;
        self.config = config;
        
        // Try to initialize batch sim if needed
        if use_gpu && self.batch_sim.is_none() {
            if let Ok(batch_sim) = BatchSimulationEngine::new_sync() {
                eprintln!("✓ GPU batch simulation engine initialized");
                self.batch_sim = Some(batch_sim);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = MctsEngine::new();
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        assert!(engine.is_ok());
    }

    #[test]
    fn test_board_evaluation() {
        let engine = MctsEngine::new();
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let engine = engine.unwrap();
        
        // Test empty board
        let mut board = [0u8; 82];
        board[81] = 1; // White to move
        let eval = engine.evaluate_board(&board);
        assert_eq!(eval, 0, "Empty board should evaluate to 0");

        // Test board with one white soldier
        board[40] = 0b1000001; // White Soldier at center
        let eval = engine.evaluate_board(&board);
        assert_eq!(eval, 1, "Board with one white soldier should evaluate to 1 for white");
    }

    #[test]
    fn test_engine_config() {
        let config = EngineConfig {
            max_depth: 5,
            simulations_per_move: 200,
            exploration_constant: 2.0,
            gpu_batch_size: 128,
            use_gpu_simulation: true,
        };
        let engine = MctsEngine::with_config(config.clone());
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let engine = engine.unwrap();
        assert_eq!(engine.config().max_depth, 5);
        assert_eq!(engine.config().simulations_per_move, 200);
    }

    #[test]
    fn test_statistics() {
        let engine = MctsEngine::new();
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let mut engine = engine.unwrap();
        
        // Get initial stats
        let stats = engine.get_statistics();
        assert_eq!(stats.total_moves_evaluated, 0);
        assert_eq!(stats.simulations_run, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        
        // Reset stats
        engine.reset_statistics();
        let stats = engine.get_statistics();
        assert_eq!(stats.total_moves_evaluated, 0);
    }

    #[test]
    fn test_cache_basic() {
        let engine = MctsEngine::new();
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let mut engine = engine.unwrap();
        
        // Initial cache should be empty
        assert_eq!(engine.cache_size(), 0);
        
        // Clear cache should work on empty cache
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_cache_statistics() {
        let engine = MctsEngine::new();
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let engine = engine.unwrap();
        
        // Initial stats should show no cache hits or misses
        let stats = engine.get_statistics();
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_integration() {
        let config = EngineConfig {
            max_depth: 2,
            simulations_per_move: 10, // Low count for faster testing
            exploration_constant: 1.414,
            gpu_batch_size: 64,
            use_gpu_simulation: false, // Use CPU for consistency
        };
        
        let engine = MctsEngine::with_config(config);
        if let Err(e) = &engine {
            println!("Skipping test: GPU not available - {}", e);
            return;
        }
        let mut engine = engine.unwrap();
        
        // Create a test board with some pieces
        let mut board = [0u8; 82];
        board[81] = 1; // White to move
        board[40] = 0b1000001; // White Soldier
        
        // First call - should be a cache miss
        let result1 = engine.find_best_move(&board);
        assert!(result1.is_ok());
        
        let stats1 = engine.get_statistics();
        assert_eq!(stats1.cache_misses, 1);
        assert_eq!(stats1.cache_hits, 0);
        assert_eq!(engine.cache_size(), 1);
        
        // Second call with same position - should be a cache hit
        let result2 = engine.find_best_move(&board);
        assert!(result2.is_ok());
        
        let stats2 = engine.get_statistics();
        assert_eq!(stats2.cache_hits, 1);
        assert_eq!(stats2.cache_misses, 1);
        assert_eq!(stats2.cache_hit_rate(), 0.5);
        
        // Results should be the same
        assert_eq!(result1.unwrap(), result2.unwrap());
        
        // Clear cache
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
        
        // Third call after clear - should be a cache miss again
        let _result3 = engine.find_best_move(&board);
        let stats3 = engine.get_statistics();
        assert_eq!(stats3.cache_misses, 2);
        assert_eq!(stats3.cache_hits, 1);
        assert_eq!(engine.cache_size(), 1);
    }
}
