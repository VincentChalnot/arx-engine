pub mod board;
pub mod cli_rendering;
pub mod game;

// Re-export main types
pub use board::{Board, Color, Piece, PieceType, Position, BOARD_DIMENSION, BOARD_SIZE};
pub use cli_rendering::display_board;
pub use game::{Game, Move, PotentialMove};
