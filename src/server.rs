use arx_engine::board::{Board, BOARD_SIZE};
use arx_engine::game::{Game, PotentialMove};
use arx_engine::engine::{MctsEngine, EngineConfig};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
    body::Bytes,
    extract::State,
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
        max_depth: 12,
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
    let mv = PotentialMove::from_u16(u16::from_le_bytes([move_bytes[0], move_bytes[1]]));
    let move_obj = mv.to_move(false);
    game.apply_move(move_obj).map_err(|_| StatusCode::BAD_REQUEST)?;
    let new_binary_board = game.to_binary();
    Ok(new_binary_board.to_vec())
}

async fn engine_move(State(state): State<Arc<AppState>>, payload: Bytes) -> Result<Vec<u8>, StatusCode> {
    let board_bytes = payload;
    if board_bytes.len() != BOARD_SIZE + 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut board_array = [0u8; BOARD_SIZE + 1];
    board_array.copy_from_slice(&board_bytes);

    // Get the engine from state
    let mut engine_guard = state.engine.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let engine = engine_guard.as_mut().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Find best move using the engine
    let best_move = engine.find_best_move(&board_array)
        .map_err(|e| {
            eprintln!("Engine error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Return the move as 2-byte little-endian u16
    Ok(best_move.to_le_bytes().to_vec())
}
