// Batch simulation shader for MCTS engine
// This shader processes multiple board positions in parallel to:
// 1. Apply moves to board states
// 2. Evaluate board positions
// This reduces CPU-GPU transfer overhead by batching operations

struct BoardState {
    squares: array<u32, 81>,  // 81 squares
    white_to_move: u32,        // 1 if white to move, 0 if black
}

struct MoveApplication {
    board: BoardState,
    move_encoding: u32,
    result_score: i32,  // Output: evaluation score after applying move
    valid: u32,         // Output: 1 if move was valid, 0 otherwise
}

@group(0) @binding(0) var<storage, read_write> applications: array<MoveApplication>;

// Piece type codes
const PIECE_SOLDIER: u32 = 1u;
const PIECE_JESTER: u32 = 2u;
const PIECE_COMMANDER: u32 = 3u;
const PIECE_PALADIN: u32 = 4u;
const PIECE_GUARD: u32 = 5u;
const PIECE_DRAGON: u32 = 6u;
const PIECE_BALLISTA: u32 = 7u;

// Special encoding for King
const KING_PAYLOAD: u32 = 0x38u; // 0b111000

// Piece values for evaluation
const PIECE_VALUES: array<i32, 8> = array<i32, 8>(
    0,  // Index 0: unused
    1,  // Soldier
    3,  // Jester
    5,  // Commander
    3,  // Paladin
    3,  // Guard
    3,  // Dragon
    5,  // Ballista
);
const KING_VALUE: i32 = 1000;

// Board dimension
const BOARD_DIM: i32 = 9;
const BOARD_SIZE: u32 = 81u;

// Helper functions
fn get_piece_color(piece: u32) -> u32 {
    return (piece >> 6u) & 1u;
}

fn get_top_piece_code(piece: u32) -> u32 {
    return (piece >> 3u) & 7u;
}

fn get_bottom_piece_code(piece: u32) -> u32 {
    return piece & 7u;
}

fn is_king(piece: u32) -> bool {
    let payload = piece & 0x3Fu; // Lower 6 bits
    return payload == KING_PAYLOAD;
}

// Apply a move to a board state
fn apply_move(board: ptr<function, BoardState>, move_encoding: u32) -> bool {
    let from_idx = move_encoding & 0x7Fu;
    let to = (move_encoding >> 7u) & 0x7Fu;
    let unstack = (move_encoding & 0x4000u) != 0u;
    
    if from_idx >= BOARD_SIZE || to >= BOARD_SIZE {
        return false;
    }
    
    let piece = (*board).squares[from_idx];
    if piece == 0u {
        return false;
    }
    
    if unstack {
        // Unstack top piece
        let payload = piece & 0x3Fu;
        let top_code = (payload >> 3u) & 7u;
        let bottom_code = payload & 7u;
        let color_bit = piece & 0x40u;
        
        if top_code == 0u {
            return false; // Cannot unstack: no top piece
        }
        
        // Create new bottom piece (remove top)
        (*board).squares[from_idx] = color_bit | bottom_code;
        
        // Create moving piece (top becomes new bottom)
        let moving_piece = color_bit | top_code;
        
        // Place at destination (simple: just replace, no stacking logic)
        (*board).squares[to] = moving_piece;
    } else {
        // Move entire piece/stack
        (*board).squares[from_idx] = 0u;
        (*board).squares[to] = piece; // Simplified: just capture/replace
    }
    
    // Switch turn
    (*board).white_to_move = select(1u, 0u, (*board).white_to_move == 1u);
    
    return true;
}

// Evaluate a board position
fn evaluate_board(board: ptr<function, BoardState>) -> i32 {
    let white_to_move = (*board).white_to_move;
    var white_value: i32 = 0;
    var black_value: i32 = 0;
    
    for (var i = 0u; i < BOARD_SIZE; i++) {
        let piece = (*board).squares[i];
        if piece == 0u {
            continue;
        }
        
        let is_white = (piece >> 6u) == 1u;
        let payload = piece & 0x3Fu;
        
        // Check for King
        if payload == KING_PAYLOAD {
            if is_white {
                white_value += KING_VALUE;
            } else {
                black_value += KING_VALUE;
            }
            continue;
        }
        
        let top_code = (payload >> 3u) & 7u;
        let bottom_code = payload & 7u;
        
        // Add value for bottom piece
        if bottom_code > 0u && bottom_code < 8u {
            let value = PIECE_VALUES[bottom_code];
            if is_white {
                white_value += value;
            } else {
                black_value += value;
            }
        }
        
        // Add value for top piece if stacked
        if top_code > 0u && top_code < 8u {
            let value = PIECE_VALUES[top_code];
            if is_white {
                white_value += value;
            } else {
                black_value += value;
            }
        }
    }
    
    // Return value from perspective of current player
    if white_to_move == 1u {
        return white_value - black_value;
    } else {
        return black_value - white_value;
    }
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let array_len = arrayLength(&applications);
    
    if idx >= array_len {
        return;
    }
    
    // Copy board to local variable for modification
    var board = applications[idx].board;
    let move_encoding = applications[idx].move_encoding;
    
    // Apply the move
    let valid = apply_move(&board, move_encoding);
    
    if valid {
        // Evaluate the resulting position
        let score = evaluate_board(&board);
        applications[idx].result_score = score;
        applications[idx].valid = 1u;
        applications[idx].board = board;
    } else {
        applications[idx].result_score = 0;
        applications[idx].valid = 0u;
    }
}
