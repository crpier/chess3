# Real-time Hex Chess

A desktop hex-chess house variant built with Rust and Raylib. **Two committed moves per side** may travel concurrently. Moves begin immediately, pieces travel continuously, and interceptions happen on the board. There are no alternating turns, but this is deliberately **not an army-control RTS**.

```sh
cargo run --release
```

Piece art and readable **Adwaita Sans** are embedded. Font license: `assets/fonts/LICENSE`. Minimum playable window: **1000 × 640**; smaller windows pause and show a resize message.

## Deliberate plays, not a spamfest

- Both sides begin after a **three-second countdown**.
- **Only two pieces per side can be moving at once.** A third order is rejected, never queued. White and Black have independent slots.
- **A committed move cannot be cancelled or redirected.** Keep a slot free for defense rather than committing everything to an attack. Clearing selection does not cancel a move.
- Slots remain occupied until the piece finishes travelling; stationary captures resolve at landing. Captured moving pieces release their slots; they never execute later or resurrect.
- Travel times count the play's steps: **1.2 seconds per touching step** for pawns, bishops, rooks and queens; **0.8 seconds** for a king step; **1.2 seconds** for a knight jump. Longer slides take proportionally longer. Repeated clicks cannot shorten travel or increase the number of plays.
- **No post-move cooldown.** A piece that arrives can be used again immediately. The limit is concurrent commitments, not an army-wide turn clock.
- **No automatic attacks, damage-per-second, health bars, healing, or army advance.** Idle pieces stay put. To capture, choose a legal chess move.

The former RTS controls **A/S** have been removed. The game no longer sends whole armies into automatic combat.

## Movement

Short moves count **edge-touching neighboring tiles**, not two competing kinds of hex diagonals. Colors are decorative; no piece is tied to a color.

| Piece | Legal play |
| --- | --- |
| King | One of the six touching tiles |
| Bishop | Walk one or two touching steps; may turn once, but cannot pass through pieces |
| Knight | Jump to any tile exactly two steps away; ignores intervening pieces |
| Rook | Slide any distance along a straight row of touching tile centers |
| Queen | Rook slides plus bishop's short walks |
| Pawn | One step forward, forward-left, or forward-right; the same pattern moves and captures |

Friendly pieces block destinations. Slides stop at the first occupied tile; bishops need an empty intermediate tile. Your committed travel lanes and destinations are reserved against other friendly orders. A move is a **single play within its piece's range**, not unlimited pathfinding across the board.

Pawns promote at the opposite sloping rim, where another straight-forward step would leave the board. The preset choice is committed with that pawn move; changing the preset afterward affects future plays. There is no double pawn move, en passant or castling.

## Captures and simultaneous play

- A travelling piece captures a stationary enemy **when it lands at the occupied waypoint**, not when sprites first touch. It stops at that captured square rather than continuing through it. This also allows a new blocker to intercept a slide.
- **Opposing ground moves that cross closely capture each other**, evaluated together in the same simulation step—not according to submission order.
- Knights leap over intervening stationary pieces and ground traffic until approaching their landing square. A landing can still be contested.
- Moving pieces vacate the static board; their real positions determine interceptions. Clicking a moving enemy targets its current tile, not an invisible copy at its old starting square. Committed paths are visible so you can plan an interception or escape.
- **Capture the king to win.** King-threat warnings include stationary attacks and incoming committed routes. They are advisory, not a forced response or legality restriction. Simultaneous king captures draw. Three active minutes without a capture or pawn advance also draw.

The fixed-step simulation runs in **20 ms steps**, independently of rendering. Swept collision checks prevent travel from skipping crossings. Moving-center contact uses a quarter-tile radius rather than sprite edges, so a lateral evasion need not be issued almost instantly. Units have stable identities: selecting a moving piece follows the actual piece, even if another piece occupies its vacated origin.

The board has 91 tiles and symmetric 18-piece armies: king, queen, two rooks, two knights, three bishops and nine pawns. This is **not tournament Gliński chess**. White begins at the bottom. Files `a`–`k` run left to right in the unflipped view; flipping preserves coordinates.

## Controls

| Control | Action |
| --- | --- |
| White: left click | Select a visible piece, then a legal destination |
| Black in local mode: arrows, comma / period | Move the blue hex cursor |
| Black: Space | Select / issue a move at the cursor |
| 1 / 2 / 3 / 4 | White promotion: queen / rook / bishop / knight |
| Shift+1 / 2 / 3 / 4 | Black promotion preset |
| Right click / Black: Backspace | Clear selection; committed moves still finish |
| Escape | Close dialog / clear selections |
| P | Pause / resume |
| F | Flip the board |
| N | Confirmed restart with a fresh countdown |
| M | Confirmed switch between solo and local mode |
| C / click CPU level | Cycle Easy → Normal → Hard in solo mode |
| H | Rules and illustrated piece movement |
| R / Shift+R | Confirmed White / Black resignation (Black only in local mode) |
| Enter | Confirm restart, mode change or resignation |
| Q / window close | Quit |

The sidebar shows **moves in flight out of two**, and whether the selected piece is ready or committed. Its planned travel time is shown; selecting an idle piece while both slots are full shows when the next slot is expected to free. These estimates freeze during pause and can shorten when a late blocker stops a slide. Feedback beneath the title explains rejected plays even while a piece remains selected, stays readable briefly instead of being overwritten by arrivals, and freezes during pause. A promoting pawn's travel hint names its committed choice even if the preset changes. Gold and blue paths show White and Black commitments. Selection rings follow physical pieces rather than highlighting empty old waypoints. Selecting a ready piece shows **all legal destinations**: mint dots for White, blue dots for Black, and rings around capturable pieces. The destination under the cursor has a larger marker. Previews turn muted while you must wait, so you can still plan without implying an order will be accepted. Committed pieces show their existing path instead of new destinations.

Two small lights beside each side's move count show its slots: mint is available, gold is in flight, and gray means play is paused or has not started. The sidebar explicitly says **ready** with the number of free slots, or **wait** with the opening countdown, next planned slot opening, or pause state. An idle selected piece is labeled “waiting” when it cannot begin a move.

### CPU difficulty

**Easy is the default.** Click the CPU level at the top of the sidebar or press **C** to cycle levels, even during a game:

| Level | Decision interval | Play style |
| --- | --- | --- |
| Easy | 2.4 seconds | One move at a time; short, varied legal moves without tactical scoring |
| Normal | 1.5 seconds | Up to two moves; prefers captures and protects its king |
| Hard | 0.65 seconds | Original CPU: the same tactical policy, reacting much faster |

Level changes do not restart the game or cancel existing moves. They reset the CPU's decision timer and apply to future orders. Restarting or switching modes preserves the selected level for this session (it is not saved between launches). All levels obey ordinary movement, capture and commitment rules; Easy voluntarily leaves its second slot unused. Human players always retain both slots.

**P, menus, focus loss and undersized windows pause both sides.** The status identifies menu and focus-loss pauses instead of presenting every pause as manual. No catch-up movement accumulates while paused. Either local player can pause. No networking, saved sessions, undo, or time rewind is provided.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo build --release
```

Tests cover two independent slots, 1,000 rejected third orders, 1,000 attempted redirects, immediate reuse after arrival, no passive combat, exact piece geometries, friendly lane reservations, knight leaps, physical king escapes, discrete captures, symmetric interceptions, no resurrection, committed promotion, frame-chunk determinism, bot limits/material invariants, 24 seeded mixed-play runs, incoming king warnings, moving-origin occupancy, stable selection, physical hit testing, pause behavior, layout mapping and texture clearance.

With an active graphical display and FFmpeg:

```sh
./scripts/record-demo.sh
cargo run --release -- --demo --audit-size 1000x740 --capture-dir artifacts/limited-chess/compact
```

The script records a **30-second** visual audit to `artifacts/limited-chess/chess3-demo.mp4` and named screenshots of parallel moves, play, rules, movement, promotion and king capture. Promotion and king capture are clearly labeled artificial fixtures, not a continuation of the opening. Capture directories are fresh per recording; images are safely cropped to the real framebuffer.

On Linux with an active Wayland desktop, Xwayland, xauth, a C compiler/X11 headers, ImageMagick and Tesseract:

```sh
./scripts/audit-native.sh
```

This opens a disposable, authenticated Xwayland display and exercises actual GLFW mouse/keyboard callbacks at 1280×900 and the real 1000×640 minimum. Fresh F10 captures are checked through the visible HUD using OCR: both sides reject third orders without queues, committed moves reject redirects, four local plays coexist, flipped controls work, dialogs open, and Black resignation awards White the win. Paused captures with all four plays in flight must be byte-identical. It cleans up its game/server and changes no desktop configuration. OCR checks deliberately fail closed; diagnostics and original images stay in the fresh run directory for review. This is isolated native verification, not a substitute for a human smoke test of normal desktop focus behavior.

For occasional native-input captures, use `--capture-dir PATH --capture-on-demand` and press **F10**. No desktop/window-manager settings are changed.

## Code

- `src/game.rs`: piece geometry, fixed-step travel, commitments, reservations, captures, identities, outcomes and simple bot.
- `src/main.rs`: physical picking, stable selections, independent controls, rendering, pause/layout, embedded assets and visual audit.
- `scripts/record-demo.sh`: repeatable screenshot/video recording.
- `scripts/audit-native.sh`, `scripts/native-input-test.c`, `scripts/check-native-captures.sh`: isolated native inputs, fresh complete captures, and assertions on the rendered HUD.
- `POLISH.md`: direct iteration notes and outstanding gameplay/testing priorities.
