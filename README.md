# Arx Engine

Arx Engine is a Rust implementation of the abstract strategy board game **Arx**, inspired by chess but featuring unique stacking mechanics. This project provides a command-line interface and terminal UI for playing, analyzing, and exporting/importing game states.

## Game Overview
Arx is played on a 9x9 board. Players control unique pieces, each with specific movement rules. Unlike chess, friendly pieces can be stacked to combine their movement abilities, creating new tactical possibilities. For a full description of the rules and piece movements, see [rules.md](./rules.md).

## Features
- Play Arx in the terminal
- Export and import board states using base64 encoding
- Display possible moves for any position
- Visualize the board with colored pieces and stacks

## Building and Running
Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```sh
# Build the project
cargo build --release

# Run the game (default: interactive TUI)
cargo run --release
```

## Command Line Options
The CLI supports several subcommands:

- `play` : Launches the interactive terminal UI for playing Arx. (default command)
- `export` : Prints the current board state as a base64 string.
- `import <data>` : Loads a board state from a base64 string.
- `show-moves [coordinates]` : Displays possible moves for a given position (e.g., `E2`).

Example usage:
```sh
# Start the interactive game
cargo run --release

# Import a board state
cargo run --release -- import "<base64_data>"

# Show possible moves for position E2
cargo run --release -- show-moves E2
```

## Documentation
- [Game Rules](./rules.md): Full rules and piece movements
- [Piece Encoding](.github/instructions/piece_encoding.instructions.md): Details on board and piece encoding

## License
This project is licensed under the MIT License.

