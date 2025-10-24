use rand::Rng;
use std::collections::HashMap;

mod gpu_move_gen;
pub use gpu_move_gen::MoveGenerationEngine;

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
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            simulations_per_move: 100,
            exploration_constant: 1.414,
        }
    }
}

/// MCTS Node
#[derive(Clone, Debug)]
struct MctsNode {
    board_state: [u8; 82],
    parent_move: Option<u16>,
    visits: u32,
    value: f32,
    children: Vec<usize>, // Indices into the node pool
}

/// Monte Carlo Tree Search Engine
pub struct MctsEngine {
    config: EngineConfig,
    move_gen: MoveGenerationEngine,
    nodes: Vec<MctsNode>,
}

impl MctsEngine {
    /// Create a new MCTS engine with default configuration
    pub fn new() -> Result<Self, String> {
        Self::with_config(EngineConfig::default())
    }

    /// Create a new MCTS engine with custom configuration
    pub fn with_config(config: EngineConfig) -> Result<Self, String> {
        let move_gen = MoveGenerationEngine::new_sync()?;
        Ok(Self {
            config,
            move_gen,
            nodes: Vec::new(),
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

    /// Find the best move using MCTS
    pub fn find_best_move(&mut self, board: &[u8; 82]) -> Result<u16, String> {
        // Generate all legal moves
        let moves = self.move_gen.generate_moves(board)?;

        if moves.is_empty() {
            return Err("No legal moves available".to_string());
        }

        if moves.len() == 1 {
            return Ok(moves[0]);
        }

        // Evaluate each move
        let mut move_scores: HashMap<u16, (i32, u32)> = HashMap::new();

        for &mv in &moves {
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

            if simulations > 0 {
                move_scores.insert(mv, (total_score, simulations));
            }
        }

        // Find move with best average score
        let best_move = move_scores
            .iter()
            .max_by(|a, b| {
                let avg_a = a.1.0 as f32 / a.1.1 as f32;
                let avg_b = b.1.0 as f32 / b.1.1 as f32;
                avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(mv, _)| *mv)
            .ok_or("No valid moves found")?;

        Ok(best_move)
    }

    /// Get the current configuration
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: EngineConfig) {
        self.config = config;
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
}
