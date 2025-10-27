use arx_engine::board::{Board, BOARD_SIZE};
use arx_engine::engine::{EngineConfig, MctsEngine};
use arx_engine::game::{Game, Move, PotentialMove};
use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

// Shared engine state
struct AppState {
    engine: Mutex<Option<MctsEngine>>,
}

#[tokio::main]
async fn main() {
    // Initialize the engine with configuration from engine_demo.rs
    let config = EngineConfig {
        max_depth: 16,
        simulations_per_move: 10000,
        exploration_constant: 1.414,
        gpu_batch_size: 2048,
        use_gpu_simulation: true,
    };

    let engine = match MctsEngine::with_config(config) {
        Ok(e) => {
            println!("✓ Engine initialized successfully");
            Some(e)
        }
        Err(e) => {
            eprintln!("⚠ Failed to initialize engine: {}", e);
            eprintln!("  Engine move endpoint will return errors");
            None
        }
    };

    let state = Arc::new(AppState {
        engine: Mutex::new(engine),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/new", get(new_game))
        .route("/moves", post(post_moves))
        .route("/play", post(play_move))
        .route("/engine-move", post(engine_move))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn new_game() -> impl IntoResponse {
    let game = Game::new();
    let binary_board = game.to_binary();
    (StatusCode::OK, binary_board)
}

async fn post_moves(payload: Bytes) -> Result<Vec<u8>, StatusCode> {
    let board_bytes = payload;
    if board_bytes.len() != BOARD_SIZE + 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut board_array = [0u8; BOARD_SIZE + 1];
    board_array.copy_from_slice(&board_bytes);
    let board = Board::from_binary(board_array).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let game = Game::from_board(board);
    let moves = game.get_all_moves();
    let mut response = Vec::new();
    for m in moves {
        response.extend_from_slice(&m.to_u16().to_le_bytes());
    }
    Ok(response)
}

async fn play_move(payload: Bytes) -> Result<Vec<u8>, StatusCode> {
    let payload = payload;
    if payload.len() < BOARD_SIZE + 3 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let board_bytes = &payload[..BOARD_SIZE + 1];
    let move_bytes = &payload[BOARD_SIZE + 1..BOARD_SIZE + 3];
    let mut board_array = [0u8; BOARD_SIZE + 1];
    board_array.copy_from_slice(board_bytes);
    let board = Board::from_binary(board_array).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut game = Game::from_board(board);
    let mv = Move::from_u16(u16::from_le_bytes([move_bytes[0], move_bytes[1]]));
    game.apply_move(mv).map_err(|_| StatusCode::BAD_REQUEST)?;
    let new_binary_board = game.to_binary();
    Ok(new_binary_board.to_vec())
}

async fn engine_move(
    State(state): State<Arc<AppState>>,
    payload: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    let board_bytes = payload;
    if board_bytes.len() != BOARD_SIZE + 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut board_array = [0u8; BOARD_SIZE + 1];
    board_array.copy_from_slice(&board_bytes);

    // Get the engine from state
    let mut engine_guard = state
        .engine
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let engine = engine_guard
        .as_mut()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Find best move using the engine
    let best_move_u16 = engine.find_best_move(&board_array).map_err(|e| {
        eprintln!("Engine error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert PotentialMove encoding to Move encoding
    // The engine returns moves in PotentialMove format (with force_unstack at bit 15)
    // We need to convert to Move format (unstack at bit 14)
    let potential_move = PotentialMove::from_u16(best_move_u16);

    // If force_unstack is set, we must unstack. Otherwise, move the whole stack.
    let unstack = potential_move.force_unstack;
    let actual_move = potential_move.to_move(unstack);

    // Return the move as 2-byte little-endian u16 in Move format
    Ok(actual_move.to_u16().to_le_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_potential_move_to_move_conversion() {
        // Test case 1: Simple move without unstack flags
        // From position 0, to position 9, no flags
        let from = 0u16;
        let to = 9u16;
        let potential_move_u16 = from | (to << 7); // Basic encoding: from | (to << 7)
        let potential_move = PotentialMove::from_u16(potential_move_u16);
        let actual_move = potential_move.to_move(false);
        let move_u16 = actual_move.to_u16();

        // Move format should have same from/to but unstack at bit 14
        assert_eq!(move_u16 & 0x7F, 0, "From position should be 0");
        assert_eq!((move_u16 >> 7) & 0x7F, 9, "To position should be 9");
        assert_eq!((move_u16 >> 14) & 0x1, 0, "Unstack should be false");

        // Test case 2: Move with force_unstack flag set (must also have unstackable)
        // From position 40, to position 41, unstackable=true, force_unstack=true
        let from = 40u16;
        let to = 41u16;
        let potential_move_u16 = from | (to << 7) | (1 << 14) | (1 << 15); // both unstackable and force_unstack
        let potential_move = PotentialMove::from_u16(potential_move_u16);
        assert!(potential_move.unstackable, "unstackable should be true");
        assert!(potential_move.force_unstack, "force_unstack should be true");

        let actual_move = potential_move.to_move(true);
        let move_u16 = actual_move.to_u16();

        assert_eq!(move_u16 & 0x7F, 40, "From position should be 40");
        assert_eq!((move_u16 >> 7) & 0x7F, 41, "To position should be 41");
        assert_eq!((move_u16 >> 14) & 0x1, 1, "Unstack should be true");

        // Test case 3: Move with unstackable flag but not force_unstack
        // From position 10, to position 20, unstackable=true, force_unstack=false
        let from = 10u16;
        let to = 20u16;
        let potential_move_u16 = from | (to << 7) | (1 << 14); // unstackable at bit 14
        let potential_move = PotentialMove::from_u16(potential_move_u16);
        assert!(potential_move.unstackable, "unstackable should be true");
        assert!(
            !potential_move.force_unstack,
            "force_unstack should be false"
        );

        // When force_unstack is false, we move the whole stack
        let actual_move = potential_move.to_move(false);
        let move_u16 = actual_move.to_u16();

        assert_eq!(move_u16 & 0x7F, 10, "From position should be 10");
        assert_eq!((move_u16 >> 7) & 0x7F, 20, "To position should be 20");
        assert_eq!((move_u16 >> 14) & 0x1, 0, "Unstack should be false");
    }
}
