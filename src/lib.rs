pub mod board;
pub mod cli_rendering;
pub mod game;
pub mod tui;
pub mod engine;

// Re-export main types
pub use board::{Board, Color, Piece, PieceType, Position, BOARD_DIMENSION, BOARD_SIZE};
pub use game::{Game, Move, PotentialMove};
pub use tui::run_tui;
pub use engine::{MctsEngine, EngineConfig, SearchStatistics, MoveGenerationEngine, BatchSimulationEngine};
