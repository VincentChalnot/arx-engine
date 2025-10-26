#!/bin/bash
# Performance comparison script for MCTS engine

set -e

echo "======================================"
echo "Arx Engine Performance Test"
echo "======================================"
echo ""

# Check if we have GPU support
echo "Checking GPU availability..."
cargo run --example engine_demo 2>&1 | grep -E "(GPU|Failed|✓)" | head -5

echo ""
echo "Running tests..."
cargo test --release -- --nocapture 2>&1 | grep -E "(test result|running|Skipping)" | head -20

echo ""
echo "======================================"
echo "Test complete!"
echo ""
echo "Key features implemented:"
echo "  ✓ GPU batch simulation for parallel processing"
echo "  ✓ Multi-threaded CPU evaluation with Rayon"
echo "  ✓ Statistics tracking (moves, simulations, GPU/CPU usage)"
echo "  ✓ Configurable batch sizes (64-1024)"
echo "  ✓ Automatic CPU fallback when GPU unavailable"
echo ""
echo "To run the demo:"
echo "  cargo run --example engine_demo"
echo ""
echo "To run with custom configuration, modify examples/engine_demo.rs"
echo "======================================"
