// Move generation compute shader for Arx engine
// This shader generates all legal moves for a given board position

// Board state representation: 9x9 board (81 squares)
// Each square is encoded in a u32 with the same 7-bit encoding as CPU:
// - Bit 6: Color (0=Black, 1=White)
// - Bits 5-3: Top piece code (000 if no top piece)
// - Bits 2-0: Bottom piece code

// Move encoding: 16 bits
// - Bit 15: force_unstack flag
// - Bit 14: unstackable flag
// - Bits 13-7: to position (0-80)
// - Bits 6-0: from position (0-80)

struct BoardState {
    squares: array<u32, 81>,  // 81 squares
    white_to_move: u32,        // 1 if white to move, 0 if black
}

struct MoveBuffer {
    moves: array<u32, 2048>,   // Buffer for generated moves (max theoretical moves)
    count: atomic<u32>,        // Number of moves generated
}

@group(0) @binding(0) var<storage, read> board: BoardState;
@group(0) @binding(1) var<storage, read_write> moves: MoveBuffer;

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

// Board dimension
const BOARD_DIM: i32 = 9;

// Helper functions
fn is_valid_position(x: i32, y: i32) -> bool {
    return x >= 0 && x < BOARD_DIM && y >= 0 && y < BOARD_DIM;
}

fn position_to_index(x: i32, y: i32) -> u32 {
    return u32(y * BOARD_DIM + x);
}

fn index_to_position(index: u32) -> vec2<i32> {
    let idx = i32(index);
    return vec2<i32>(idx % BOARD_DIM, idx / BOARD_DIM);
}

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

fn is_stackable(piece: u32) -> bool {
    if is_king(piece) {
        return false;
    }
    let top_code = get_top_piece_code(piece);
    return top_code == 0u; // No top piece means stackable
}

fn add_move(from: u32, to: u32, unstackable: bool, force_unstack: bool) {
    var move_encoding: u32 = from | (to << 7u);
    if unstackable {
        move_encoding |= (1u << 14u);
    }
    if force_unstack {
        move_encoding |= (1u << 15u);
    }
    
    let index = atomicAdd(&moves.count, 1u);
    if index < 2048u {
        moves.moves[index] = move_encoding;
    }
}

// Explore a target position and add move if valid
// Returns true if can continue in this direction, false if blocked
fn explore_position(from_pos: vec2<i32>, from_idx: u32, to_pos: vec2<i32>, 
                   piece_color: u32, is_top: bool, has_top: bool) -> bool {
    if !is_valid_position(to_pos.x, to_pos.y) {
        return false;
    }
    
    let to_idx = position_to_index(to_pos.x, to_pos.y);
    let target_piece = board.squares[to_idx];
    
    // Empty square
    if target_piece == 0u {
        add_move(from_idx, to_idx, is_top, false);
        return true;
    }
    
    let target_color = get_piece_color(target_piece);
    
    // Enemy piece - can capture
    if target_color != piece_color {
        add_move(from_idx, to_idx, is_top, false);
        return false;
    }
    
    // Friendly piece
    if !is_top && has_top {
        // Cannot move to stack if piece has a top piece
        return false;
    }
    
    // Can we stack?
    if !is_stackable(target_piece) {
        return false;
    }
    
    add_move(from_idx, to_idx, is_top, is_top);
    return false;
}

// Generate moves for soldier
fn generate_soldier_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    let dy = select(1, -1, color == 1u);
    
    let target1 = vec2<i32>(pos.x + 1, pos.y + dy);
    explore_position(pos, idx, target1, color, is_top, has_top);
    
    let target2 = vec2<i32>(pos.x - 1, pos.y + dy);
    explore_position(pos, idx, target2, color, is_top, has_top);
}

// Generate moves for pieces that move in specific directions with max distance
fn generate_directional_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool,
                              directions: ptr<function, array<vec2<i32>, 8>>, dir_count: u32, max_dist: i32) {
    for (var d = 0u; d < dir_count; d++) {
        let dir = (*directions)[d];
        for (var dist = 1; dist <= max_dist; dist++) {
            let target = vec2<i32>(pos.x + dir.x * dist, pos.y + dir.y * dist);
            if !explore_position(pos, idx, target, color, is_top, has_top) {
                break;
            }
        }
    }
}

// Generate moves for jester (diagonal, unlimited)
fn generate_jester_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(1, 1), vec2<i32>(1, -1), vec2<i32>(-1, 1), vec2<i32>(-1, -1),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 4u, BOARD_DIM);
}

// Generate moves for commander (orthogonal, unlimited)
fn generate_commander_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(1, 0), vec2<i32>(0, 1), vec2<i32>(-1, 0), vec2<i32>(0, -1),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 4u, BOARD_DIM);
}

// Generate moves for paladin (orthogonal, max 2)
fn generate_paladin_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(1, 0), vec2<i32>(0, 1), vec2<i32>(-1, 0), vec2<i32>(0, -1),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 4u, 2);
}

// Generate moves for guard (diagonal, max 2)
fn generate_guard_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(1, 1), vec2<i32>(1, -1), vec2<i32>(-1, 1), vec2<i32>(-1, -1),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 4u, 2);
}

// Generate moves for dragon (knight moves)
fn generate_dragon_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(2, 1), vec2<i32>(2, -1), vec2<i32>(-2, 1), vec2<i32>(-2, -1),
        vec2<i32>(1, 2), vec2<i32>(1, -2), vec2<i32>(-1, 2), vec2<i32>(-1, -2)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 8u, 1);
}

// Generate moves for ballista (forward, unlimited)
fn generate_ballista_moves(pos: vec2<i32>, idx: u32, color: u32, is_top: bool, has_top: bool) {
    let dy = select(1, -1, color == 1u);
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(0, dy),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0),
        vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(0, 0)
    );
    generate_directional_moves(pos, idx, color, is_top, has_top, &directions, 1u, BOARD_DIM);
}

// Generate moves for king (all directions, max 1)
fn generate_king_moves(pos: vec2<i32>, idx: u32, color: u32) {
    var directions = array<vec2<i32>, 8>(
        vec2<i32>(1, 0), vec2<i32>(0, 1), vec2<i32>(-1, 0), vec2<i32>(0, -1),
        vec2<i32>(1, 1), vec2<i32>(1, -1), vec2<i32>(-1, 1), vec2<i32>(-1, -1)
    );
    // King cannot have a top piece, so is_top=false, has_top=false
    generate_directional_moves(pos, idx, color, false, false, &directions, 8u, 1);
}

// Generate moves for a piece type
fn generate_moves_for_piece_type(pos: vec2<i32>, idx: u32, color: u32, 
                                 piece_type: u32, is_top: bool, has_top: bool) {
    switch piece_type {
        case PIECE_SOLDIER: {
            generate_soldier_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_JESTER: {
            generate_jester_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_COMMANDER: {
            generate_commander_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_PALADIN: {
            generate_paladin_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_GUARD: {
            generate_guard_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_DRAGON: {
            generate_dragon_moves(pos, idx, color, is_top, has_top);
        }
        case PIECE_BALLISTA: {
            generate_ballista_moves(pos, idx, color, is_top, has_top);
        }
        default: {}
    }
}

@compute @workgroup_size(9, 9, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    
    if x >= BOARD_DIM || y >= BOARD_DIM {
        return;
    }
    
    let idx = position_to_index(x, y);
    let piece = board.squares[idx];
    
    // Empty square
    if piece == 0u {
        return;
    }
    
    let piece_color = get_piece_color(piece);
    let color_to_move = board.white_to_move;
    
    // Not our turn
    if piece_color != color_to_move {
        return;
    }
    
    let pos = vec2<i32>(x, y);
    
    // Check if it's a king
    if is_king(piece) {
        generate_king_moves(pos, idx, piece_color);
        return;
    }
    
    let top_code = get_top_piece_code(piece);
    let bottom_code = get_bottom_piece_code(piece);
    
    // Generate moves for top piece if it exists
    if top_code != 0u {
        generate_moves_for_piece_type(pos, idx, piece_color, top_code, true, true);
    }
    
    // Generate moves for bottom piece
    generate_moves_for_piece_type(pos, idx, piece_color, bottom_code, false, top_code != 0u);
}
