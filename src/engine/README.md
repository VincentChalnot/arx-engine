# Arx Engine - MCTS GPU Engine

This module provides a GPU-accelerated Monte Carlo Tree Search (MCTS) engine for evaluating Arx board positions and finding the best moves.

## Architecture

The engine consists of two main components:

### 1. GPU Move Generation (`gpu_move_gen.rs`)

The move generation engine uses WebGPU compute shaders to efficiently generate all legal moves for a given board position in parallel. Each square of the 9x9 board is processed by a separate thread in the shader.

Key features:
- Parallel processing of all board squares
- Full implementation of Arx movement rules in WGSL shader
- Returns encoded moves (16-bit format) that can be used by the MCTS engine

### 2. MCTS Engine (`mod.rs`)

The MCTS engine implements Monte Carlo Tree Search with the following features:
- Configurable search depth
- Configurable number of simulations per move
- Board evaluation based on piece values
- Independent from the main game logic (doesn't use `board.rs` or `game.rs`)

## Piece Values

The engine uses the following piece values for evaluation:
- Soldier: 1 point
- Jester: 3 points (like Bishop in chess)
- Commander: 5 points (like Rook in chess)
- Paladin: 3 points
- Guard: 3 points
- Dragon: 3 points (like Knight in chess)
- Ballista: 5 points
- King: 1000 points (invaluable)

## Usage

### Basic Usage

```rust
use arx_engine::engine::{MctsEngine, EngineConfig};

// Create engine with default configuration
let mut engine = MctsEngine::new()?;

// Get a board state from the game
let board_binary = game.to_binary();

// Find the best move
let best_move = engine.find_best_move(&board_binary)?;

// Apply the move to the game
let mv = arx_engine::game::Move::from_u16(best_move);
game.apply_move(mv)?;
```

### Custom Configuration

```rust
use arx_engine::engine::{MctsEngine, EngineConfig};

// Configure engine strength
let config = EngineConfig {
    max_depth: 5,              // Search up to 5 moves ahead
    simulations_per_move: 200, // Run 200 simulations per candidate move
    exploration_constant: 1.414, // UCB1 exploration constant
};

let mut engine = MctsEngine::with_config(config)?;
```

### Adjusting Engine Strength

You can control the engine's strength by adjusting:

1. **`max_depth`**: How many moves ahead to search
   - Lower values (1-3): Beginner level
   - Medium values (4-6): Intermediate level
   - Higher values (7+): Advanced level (but slower)

2. **`simulations_per_move`**: Number of random playouts per move
   - Lower values (50-100): Faster but less accurate
   - Medium values (100-500): Good balance
   - Higher values (500+): More accurate but slower

## Board Encoding

The engine uses the same 7-bit piece encoding as the rest of the codebase:

```
Bit 6: Color (0=Black, 1=White)
Bits 5-3: Top piece code (000 if no top piece)
Bits 2-0: Bottom piece code
```

Special encoding for King: `0b_111000` (payload)

## Move Encoding

Moves are encoded in 16 bits:
```
Bit 15: force_unstack flag
Bit 14: unstackable flag
Bits 13-7: to position (0-80)
Bits 6-0: from position (0-80)
```

## Shader Implementation

The compute shader (`shaders/move_generation.wgsl`) implements:
- All piece movement patterns (Soldier, Jester, Commander, Paladin, Guard, Dragon, Ballista, King)
- Stacking rules
- Capture mechanics
- Move validation

Each invocation of the shader processes one square of the board, generating moves for the piece at that square if it belongs to the current player.

## Requirements

- WebGPU-compatible GPU
- Rust with async support
- Dependencies: `wgpu`, `bytemuck`, `pollster`, `rand`

## Testing

The engine includes tests that gracefully handle environments without GPU support:

```bash
cargo test --lib
```

Tests will skip GPU-dependent functionality if no adapter is available, making them CI-friendly.

## Performance

The GPU-accelerated move generation provides significant performance benefits:
- All squares are processed in parallel
- Typical move generation completes in microseconds
- Enables deeper search within reasonable time constraints

## Future Improvements

Potential enhancements:
- Full UCB1 tree search implementation
- Transposition tables for position caching
- Alpha-beta pruning integration
- Neural network evaluation
- Opening book support
