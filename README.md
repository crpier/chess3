# Hex Chess

A playable desktop hex-chess game built with Rust and Raylib. Play White against a lightweight Black bot, or share the board in local two-player mode.

```sh
cargo run --release
```

The piece art and readable **Adwaita Sans** interface font are embedded in the executable, so the built game can be launched outside the repository. The font's SIL Open Font License is in `assets/fonts/LICENSE`.

## Playing

The playable window minimum is 1000 x 640 pixels. If a tiling window manager overrides that minimum, a clear resize message replaces the board and gameplay pauses until the window is large enough.

Click one of your pieces, then a marked destination. Dots indicate empty destinations; mint outlines indicate captures. Only legal moves are shown. The checked king is highlighted red, and the previous move is tinted green. Promotion opens a choice of four pieces.

| Control | Action |
| --- | --- |
| Left click | Select a piece / move / use buttons |
| Right click | Clear selection |
| U | Undo (a full player/bot turn in solo mode) |
| F | Flip the board |
| N | New game, with confirmation |
| M | Switch solo/local mode, with confirmation and a fresh board |
| H | Open rules |
| Enter | Confirm a new game, mode switch, or resignation |
| Escape | Close a dialog or clear selection; **does not quit** |
| Q / window close | Quit |

The bot deliberately uses only a small one-ply heuristic. There are no clocks, online multiplayer, or saved games; closing the application ends the current session.

## The house variant

This is **not tournament Glinski chess**. It uses a radius-five, 91-tile board and a symmetric 18-piece army: a king, queen, two rooks, two knights, three bishops (one on each tile color), and nine pawns per side. White starts at the bottom and moves first.

- **Rook:** slides along the six edge-adjacent directions.
- **Bishop:** slides along the six vertex diagonals, remaining on one tile color.
- **Queen:** combines rook and bishop movement.
- **King:** one rook or bishop step. Cannot enter check; kings are never captured.
- **Knight:** two steps in one edge direction, then one step in an adjacent direction; jumps over pieces.
- **Pawn:** one empty tile forward toward the opposite edge. Captures along either adjacent forward edge direction. Reaching the far edge of its file promotes to a queen, rook, bishop, or knight.
- There is **no double pawn move, en passant, or castling**.
- Checkmate wins. Stalemate, threefold repetition, 100 half-moves without a pawn move or capture, and bare kings are draws. Other insufficient-material positions are not automatically adjudicated.

Coordinates use files `a` through `k` from left to right in the unflipped view, with each file numbered from the top. Flipping does not change a tile's name.

## Validation

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The tests cover movement, blockers, captures, pawn attack maps and promotion, pins, king safety, checkmate, stalemate, repetition, the 50-move rule, resignation, undo, both board orientations, piece-to-tile clearance, dialog/mode transitions, the click-selection flow, and bot play.

### Screenshots and video

With a working graphical display and FFmpeg:

```sh
./scripts/record-demo.sh
```

This runs a repeatable 24-second visual smoke test and writes PNG frames and `artifacts/chess3-demo.mp4`. It exercises selection, moves, bot replies, animation, flipping, rules, undo, promotion, checkmate, and a confirmed restart. Promotion and checkmate use explicit rule-test fixtures rather than pretending those positions arose from the recorded opening. The rendering uses the real game window; your window manager may resize it during capture.

You can also run just the game-side capture:

```sh
cargo run --release -- --demo --capture-dir artifacts/frames
```

For a compact layout audit, with the real window at least 1000x740:

```sh
cargo run --release -- --demo --audit-size 1000x740 --capture-dir artifacts/compact
```

This renders a compact logical viewport and safely crops the exported PNGs to it, without modifying window-manager settings. If the compositor shrinks the real window further, the viewport follows its available size.

## Code

- `src/game.rs`: deterministic, rendering-independent board rules and game lifecycle.
- `src/main.rs`: desktop input, layout, rendering, the simple UI, embedded assets, and visual smoke test.
