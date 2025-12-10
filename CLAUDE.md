# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a hexagonal chess game implementation using Rust and Raylib for rendering. The game uses a hexagonal grid system with cube coordinates for tile positioning.

## Build and Run Commands

```bash
# Build the project
cargo build

# Run the game
cargo run

# Build with optimizations
cargo build --release

# Run with release optimizations
cargo run --release

# Check code without building
cargo check

# Run tests (when available)
cargo test

# Run a specific test
cargo test test_name
```

## Architecture

### Coordinate System

The game uses **cube coordinates** for hexagonal tiles, which use three coordinates (q, r, s) where q + r + s = 0. This system simplifies hexagonal grid math:

- `CubeCoord`: The core coordinate type with q, r, s components
- `from_vector()`: Converts screen coordinates to cube coordinates
- `to_vector()`: Converts cube coordinates to screen coordinates for rendering
- `round_hex()`: Handles floating-point to cube coordinate conversion

The coordinate system uses a flat-top hexagonal orientation with pointy-top rendering (rotated by 30 degrees via the `draw_poly` rotation parameter).

### Core Components

- **Map**: Central game state container holding:
  - `hexes`: HashMap of all hexagonal tiles
  - `pieces`: HashMap of chess pieces by position
  - `highlighted_location`: Currently highlighted hex under cursor
  - `selected_location`: Currently selected hex (for piece movement)

- **Hexagon**: Individual tile representation with color and highlight state

- **Piece**: Chess piece with kind, position, and texture

### Rendering Constants

Key constants defined at the top of `src/main.rs`:
- `TILE_RADIUS`: 45.0 (hex size)
- `MAP_RADIUS`: 5 (creates 91 total hexagons)
- `TEXTURE_SCALE`: 0.25 (piece image scaling)
- `SCREEN_WIDTH/HEIGHT`: 1440x900

### Game Loop Pattern

The main loop follows this structure:
1. `frame_reset()`: Clears transient state (like highlights)
2. Update logic (cursor tracking, input handling)
3. `draw()`: Renders hexes then pieces

## Dependencies

- **raylib**: Graphics rendering and window management
- **anyhow**: Error handling with context
