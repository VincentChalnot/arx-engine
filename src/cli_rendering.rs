use crate::{Piece, PieceType};

pub fn display_stack(piece: &Piece) -> String {
    let mut output: String = String::new();

    if let Some(ref top_piece) = piece.top {
        output.push_str(&piece_to_char(top_piece));
        output.push('+');
    }

    output.push_str(&piece_to_char(&piece.bottom));

    output
}

pub fn piece_to_char(piece_type: &PieceType) -> String {
    match piece_type {
        PieceType::Soldier => "S".to_string(),
        PieceType::Jester => "J".to_string(),
        PieceType::Commander => "C".to_string(),
        PieceType::Paladin => "P".to_string(),
        PieceType::Guard => "G".to_string(),
        PieceType::Dragon => "D".to_string(),
        PieceType::Ballista => "B".to_string(),
        PieceType::King => "K".to_string(),
    }
}
