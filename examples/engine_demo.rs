use arx_engine::engine::{MctsEngine, EngineConfig};
use arx_engine::{Game, Move};

fn main() {
    println!("Arx Engine - MCTS GPU Engine Example");
    println!("=====================================\n");

    // Create a new game
    let mut game = Game::new();
    
    // Create engine with different difficulty levels
    println!("Available difficulty levels:");
    println!("1. Beginner   (depth: 2, simulations: 50)");
    println!("2. Easy       (depth: 3, simulations: 100)");
    println!("3. Medium     (depth: 4, simulations: 200)");
    println!("4. Hard       (depth: 5, simulations: 300)");
    println!("5. Expert     (depth: 6, simulations: 500)");
    println!();

    // For this example, we'll use Easy difficulty
    let config = EngineConfig {
        max_depth: 3,
        simulations_per_move: 100,
        exploration_constant: 1.414,
    };

    println!("Creating MCTS engine with Easy difficulty...");
    let mut engine = match MctsEngine::with_config(config) {
        Ok(e) => {
            println!("✓ Engine created successfully\n");
            e
        }
        Err(e) => {
            eprintln!("✗ Failed to create engine: {}", e);
            eprintln!("This may happen if no GPU is available.");
            return;
        }
    };

    println!("Playing first 3 moves with the engine:\n");

    for move_num in 1..=3 {
        // Get current board state
        let board_state = game.to_binary();
        
        // Check if game is over
        if game.board.is_game_over() {
            println!("Game over!");
            break;
        }

        // Find best move
        println!("Move {}: Thinking...", move_num);
        let best_move = match engine.find_best_move(&board_state) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("✗ Failed to find move: {}", e);
                break;
            }
        };

        // Decode and display the move
        let mv = Move::from_u16(best_move);
        let from_str = mv.from.to_string();
        let to_str = mv.to.to_string();
        let unstack_str = if mv.unstack { " (unstack)" } else { "" };
        
        println!("  Best move: {} -> {}{}", from_str, to_str, unstack_str);

        // Apply the move
        match game.apply_move(mv) {
            Ok(_) => println!("  ✓ Move applied\n"),
            Err(e) => {
                eprintln!("  ✗ Failed to apply move: {}", e);
                break;
            }
        }
    }

    println!("Example completed!");
    println!("\nConfiguration tips:");
    println!("- Increase max_depth for stronger play (but slower)");
    println!("- Increase simulations_per_move for more accurate evaluation");
    println!("- Decrease both for faster but weaker play");
}
