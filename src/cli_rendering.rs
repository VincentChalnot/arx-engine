use crate::{Board, Color, Piece, PieceType, Position, BOARD_DIMENSION};

pub fn display_board(board: &Board) {
    // ANSI color codes
    const WHITE_COLOR: &str = "\x1b[97m"; // Bright white
    const RED_COLOR: &str = "\x1b[91m";   // Bright red
    const RESET_COLOR: &str = "\x1b[0m";  // Reset to default
    
    println!();
    println!("     A   B   C   D   E   F   G   H   I");
    println!("   ┏━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┓");
    for y in 0..BOARD_DIMENSION {
        if y != 0 {
            println!("   ┣━━━╋━━━╋━━━╋━━━╋━━━╋━━━╋━━━╋━━━╋━━━┫");
        }
        print!(" {} ┃",  BOARD_DIMENSION - y);
        for x in 0..9 {
            let position = Position { x, y };
            match board.get_piece(&position) {
                None => print!("   "),
                Some(piece) => {
                    let piece_char = display_stack(&piece);
                    let color_code = match piece.color {
                        Color::White => WHITE_COLOR,
                        Color::Black => RED_COLOR,
                    };
                    
                    if piece.top.is_some() {
                        print!("{}{}{}", color_code, piece_char, RESET_COLOR);
                    } else {
                        print!(" {}{}{} ", color_code, piece_char, RESET_COLOR);
                    }
                }
            }
            print!("┃");
        }
        print!(" {}", BOARD_DIMENSION - y);
        println!();
    }
    println!("   ┗━━━┻━━━┻━━━┻━━━┻━━━┻━━━┻━━━┻━━━┻━━━┛");
    println!("     A   B   C   D   E   F   G   H   I");
    println!("              {} TO MOVE", if board.is_white_to_move() { "WHITE" } else { "BLACK" });
    println!();
}

pub fn display_stack(piece: &Piece) -> String {
    let mut output: String = String::new();

    if let Some(ref top_piece) = piece.top {
        output.push_str(&display_piece(top_piece));
        output.push('+');
    }

    output.push_str(&display_piece(&piece.bottom));

    output
}

fn display_piece(piece_type: &PieceType) -> String {
    match piece_type {
        PieceType::Soldier => "S".to_string(),
        PieceType::Jester => "J".to_string(),
        PieceType::Commander => "C".to_string(),
        PieceType::Paladins => "P".to_string(),
        PieceType::Guards => "G".to_string(),
        PieceType::Dragons => "D".to_string(),
        PieceType::Ballista => "B".to_string(),
        PieceType::King => "K".to_string(),
    }
}