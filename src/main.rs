use arx_engine::{cli_rendering::display_stack, display_board, run_tui, Game, Position, BOARD_DIMENSION, BOARD_SIZE};
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
    let game = Game::new();

    match &cli.command {
        Some(Commands::Export) => {
            let all_bytes = game.to_binary();
            let mut byte_vec = Vec::new();
            for byte in all_bytes.iter() {
                byte_vec.push(*byte);
            }
            println!("{}", general_purpose::STANDARD.encode(&byte_vec));
        }
        Some(Commands::Import(args)) => {
            match general_purpose::STANDARD.decode(&args.data) {
                Ok(bytes) => {
                    // Convert bytes back to [u8; 81]
                    if bytes.len() != BOARD_SIZE + 1 {
                        eprintln!("Invalid data length: expected {} bytes, got {}", BOARD_SIZE + 1, bytes.len());
                        return;
                    }
                    
                    let mut board_data = [0; BOARD_SIZE + 1];
                    for (i, &byte) in bytes.iter().enumerate() {
                        board_data[i] = byte;
                    }
                    
                    match Game::from_binary(board_data) {
                        Ok(imported_game) => {
                            display_board(&imported_game.board);
                        }
                        Err(e) => {
                            eprintln!("Failed to create game from data: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to decode base64 data: {}", e);
                }
            }
        }
        Some(Commands::ShowMoves(args)) => {
            display_board(&game.board);
            if let Some(coordinates) = &args.coordinates {
                let position = parse_position(coordinates).unwrap_or_else(|err| {
                    eprintln!("Error parsing position: {}", err);
                    std::process::exit(1);
                });
                show_moves_for_position(&game, &position, true);
            } else {
                show_all_moves(&game);
            }
        }
        _ => {
            if let Err(e) = run_tui(Some(game)) {
                eprintln!("TUI error: {}", e);
            }
        }
    }

    fn show_all_moves(game: &Game) {
        for y in 0..BOARD_DIMENSION {
            for x in 0..BOARD_DIMENSION {
                let position = Position { x, y };
                show_moves_for_position(game, &position, false);
            }
        }
    }

    fn show_moves_for_position(game: &Game, position: &Position, display_empty_message: bool) {        
        let moves = game.get_moves(position);
        if moves.is_empty() {
            if display_empty_message {
                println!("No moves available for position {}.", position.to_string());
            }
            return;
        }
        let piece = game.board.get_piece(position);
        let piece_string = if let Some(piece) = piece {
            display_stack(piece)
        } else {
            "?".to_string()
        };
        println!("Available moves for {}@{}: ", piece_string, position.to_string());
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