use arx_engine::{display_board, Board, Position, BOARD_DIMENSION, BOARD_SIZE};
use clap::{Parser, Subcommand, Args};
use base64::{Engine as _, engine::general_purpose};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Play,
    Export,
    Import(ImportArgs),
    ShowMoves(ShowMovesArgs),
}

#[derive(Args)]
struct ImportArgs {
    /// Base64 encoded board data to import
    data: String,
}

#[derive(Args)]
struct ShowMovesArgs {
    // Position to show moves for
    coordinates: Option<String>,
}


fn main() {
    let cli = Cli::parse();
    let board = Board::new();

    match &cli.command {
        Some(Commands::Play) => {
            display_board(&board);
        }
        Some(Commands::Export) => {
            let mut all_bytes = Vec::new();
            for line in board.to_binary().iter() {
                all_bytes.extend_from_slice(&line.to_le_bytes());
            }
            println!("{}", general_purpose::STANDARD.encode(&all_bytes));
        }
        Some(Commands::Import(args)) => {
            match general_purpose::STANDARD.decode(&args.data) {
                Ok(bytes) => {
                    // Convert bytes back to [u8; 80]
                    if bytes.len() != BOARD_SIZE + 1 {
                        eprintln!("Invalid data length: expected {} bytes, got {}", BOARD_SIZE + 1, bytes.len());
                        return;
                    }
                    
                    let mut board_data = [0; BOARD_SIZE + 1];
                    for (i, chunk) in bytes.chunks_exact(1).enumerate() {
                        board_data[i] = u8::from_le_bytes(chunk.try_into().unwrap());
                    }
                    
                    match Board::from_binary(board_data) {
                        Ok(imported_board) => {
                            display_board(&imported_board);
                        }
                        Err(e) => {
                            eprintln!("Failed to create board from data: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to decode base64 data: {}", e);
                }
            }
        }
        Some(Commands::ShowMoves(args)) => {
            if let Some(coordinates) = &args.coordinates {
                let position = parse_position(coordinates).unwrap_or_else(|err| {
                    eprintln!("Error parsing position: {}", err);
                    std::process::exit(1);
                });
                show_moves_for_position(&board, &position, true);
            } else {
                show_all_moves(&board);
            }
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    fn show_all_moves(board: &Board) {
        for y in 0..BOARD_DIMENSION {
            for x in 0..BOARD_DIMENSION {
                let position = Position { x, y };
                show_moves_for_position(board, &position, false);
            }
        }
    }

    fn show_moves_for_position(board: &Board, position: &Position, display_empty_message: bool) {        
        let moves = board.get_moves(position);
        if moves.is_empty() {
            if display_empty_message {
                println!("No moves available for position {}.", position.to_string());
            }
            return;
        }
        println!("Available moves for position {}: ", position.to_string());
        for m in moves.iter() {
            print!(" - {}", m.to.to_string());
            if m.unstackable {
                if m.force_unstack {
                    print!(" (forced unstack)");
                } else {
                    print!(" (unstackable)");
                }
            }
            println!();
        }
    }

    fn parse_position(position: &str) -> Result<Position, String> {
        if position.len() != 2 {
            return Err("Invalid position format. Use e.g. 'B4'.".to_string());
        }
        // A1 is (0,8), I9 is (8,0)
        let x = match position.chars().nth(0).unwrap().to_ascii_uppercase() {
            'A'..='I' => position.chars().nth(0).unwrap() as usize - 'A' as usize,
            _ => return Err("Invalid column. Use letters A-I.".to_string()),
        };
        let y = match position.chars().nth(1).unwrap() {
            '1'..='9' => 8 - (position.chars().nth(1).unwrap() as usize - '1' as usize),
            _ => return Err("Invalid row. Use numbers 1-9.".to_string()),
        };
        
        Ok(Position { x, y })
    }
}