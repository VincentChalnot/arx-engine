pub mod board;
pub mod cli_rendering;

// Re-export main types
pub use board::{Board, Color, Piece, PieceType, Position, BOARD_DIMENSION, BOARD_SIZE};
pub use cli_rendering::display_board;
