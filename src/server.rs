use arx_engine::board::{Board, BOARD_SIZE};
use arx_engine::game::{Game, PotentialMove};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
    body::Bytes,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/new", get(new_game))
        .route("/moves", post(post_moves))
        .route("/play", post(play_move))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
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
