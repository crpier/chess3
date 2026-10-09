# Hex Chess: a real-time battlefield

A small desktop **hex-board RTS**, using chess-piece armies and Rust/Raylib. This is no longer chess with a turn clock: pieces physically march concurrently, fight automatically, take persistent damage, and can retreat or reinforce while other fights continue.

```sh
cargo run --release
```

Piece art and **Adwaita Sans** are embedded in the executable. Font license: `assets/fonts/LICENSE`. Minimum playable window: **1000 × 640**; smaller windows pause and show a resize message.

## The core loop

1. Both armies unlock after a **three-second countdown**. Select beforehand if you like.
2. Click a piece, then **any reachable destination**, even several tiles away. It follows a persistent route—there is no need to click once per tile.
3. **Every piece acts independently.** There is no army lock, move wind-up, recovery interval, action queue, or piece cooldown.
4. Nearby enemies **fight automatically over time**, including when standing still. Orders steer movement; they do not trigger attacks. Reaching an enemy's tile does **not** instantly capture it.
5. Health is persistent. Rooks and queens apply ranged damage along clear sightlines; **both friendly and enemy bodies block their fire**. Bishops heal one injured ally nearby, excluding themselves. Damage and healing are simultaneous each simulation step.
6. **A: advance and engage** sends your whole army to seek accessible enemies, reroute periodically, and hold position when able to attack or heal. This includes your king: redirect or stop him behind your frontline if needed. Manual destinations override autonomous advance and permit movement while fighting—use them to flank or withdraw.
7. **S: stop** cancels a selected unit's march/advance intention; it brakes and still defends itself automatically. Retargeting likewise preserves velocity and health.
8. Reduce the enemy king to zero health to win. Simultaneous king deaths draw. Three active minutes without a casualty or pawn advance also draw. There is no check, checkmate, forced response, or instant capture.

### Why clicking faster does not make attacks faster

Movement has a physical speed cap and acceleration/turning inertia. Repeating an identical destination is idempotent: it cannot restart a route segment, teleport a piece, reset velocity, restore health, or produce an extra attack. Combat is damage per second, not damage per command. You can command multiple units, but the army also fights without constant clicks.

The deterministic simulation advances in **20 ms fixed steps**, independently of render frames. Pausing never accumulates catch-up movement or damage.

## Army roles

Distances and speeds use neighboring tile centers as one unit. These are initial tuning values, not a claim of competitive balance.

| Piece | Health | Speed (tiles/s) | Damage/s | Attack range | Role |
| --- | ---: | ---: | ---: | ---: | --- |
| Pawn | 80 | 0.75 | 18 | 1.12 | Forward-only frontline infantry |
| Knight | 110 | 1.50 | 26 | 1.12 | Fast cavalry; vaults friendly traffic |
| Bishop | 75 | 0.90 | 10 | 1.12 | Support; heals **4 HP/s** to one ally within **2.1** tiles |
| Rook | 120 | 0.65 | 22 | 3.20 | Slow ranged artillery; needs a clear firing lane |
| Queen | 140 | 0.85 | 24 | 2.60 | Mobile ranged support |
| King | 180 | 0.90 | 16 | 1.12 | Commander; protect him behind other units |

**Movement patterns are now route capabilities, not per-order chess moves.** Ground routes follow neighboring tiles and go around occupied waypoints. Pawns only take steps into their forward three-tile fan; they cannot retreat. Other pieces can walk in any direction. Knights additionally take two-tile vaults over friendly traffic, but cannot fly through enemies or stack on another unit's destination. Bodies may cause congestion; stop or redirect units to open a lane. A blocked manual march waits rather than teleporting through the obstruction.

All units defend against nearby enemies in any direction; a pawn's forward restriction affects marching, not its ability to defend itself. Bishops' healing is not blocked by bodies. King danger is indicated when health falls below half.

Pawns promote at the opposite sloping rim, using your preset. Promotion **preserves the health fraction**, rather than restoring full health. There are no separate capture moves, double pawn steps, en passant or castling.

The 91-tile radius-five board retains symmetric 18-piece armies: king, queen, two rooks, two knights, three bishops and nine pawns. Tile colors are decorative. This is **not tournament Gliński chess**. White begins at the bottom. Files `a`–`k` run left to right in the unflipped view; flipping does not change coordinates.

## Controls

| Control | Action |
| --- | --- |
| White: left click | Select a piece at its actual moving position; then choose a destination |
| **A** | White: advance and engage with the whole army |
| **S** | White: stop the selected unit; automatic defense continues |
| Black in local mode: arrows, comma / period | Move the blue hex cursor |
| Black: Space | Select / order at the cursor |
| **Shift+A / Shift+S** | Black: advance army / stop selected unit |
| 1 / 2 / 3 / 4 | White promotion: queen / rook / bishop / knight |
| Shift+1 / 2 / 3 / 4 | Black promotion preset |
| Right click / Black: Backspace | Clear selection; **does not stop an existing order** |
| Escape | Close dialog / clear selections |
| P | Pause / resume |
| F | Flip the board |
| N | Confirmed restart with a fresh countdown |
| M | Confirmed switch between solo and local mode |
| H | Battle rules; **Army guide** shows combat/healing reach and roles |
| R / Shift+R | Confirmed White / Black resignation (Black only in local mode) |
| Enter | Confirm restart, mode change or resignation |
| Q / window close | Quit |

Health bars follow moving units. Gold/blue attack links show current combat; green links show healing. Faint destination markers show persistent goals, and selected units show their destination line. The sidebar counts physically marching and fighting units and explains the selected unit's role and health. Selection follows the unit's stable identity across waypoints.

Solo Black uses the same persistent **advance-and-engage** behavior and exactly the same physics and combat rules, with no hidden bonuses. It is intentionally a lightweight opponent. Local players can issue concurrent commands independently.

**P, menus, focus loss and undersized windows pause movement, combat and healing.** Either local player can pause. No networking, saved sessions, undo, or time rewind is provided.

## Validation

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo build --release
```

Tests exercise whole-army concurrency, continuous long-distance marching, automatic combat without orders, real damage during 1,000 repeated submissions, momentum-preserving redirects/stops, traffic/pathfinding, ranged blockers, healing, advance/hold/withdraw behavior, simultaneous king deaths, health-preserving promotion, quiet draws, rejected orders, deterministic time chunking, battle invariants, pause behavior, native-position hit testing, stable selections, hex mapping and texture clearance. Obsolete cooldown/instant-capture tests were replaced, not kept as rules for the new game.

### Screenshots and video

With an active display and FFmpeg:

```sh
./scripts/record-demo.sh
```

Records a deterministic **30-second** rendered audit to `artifacts/battlefield/chess3-demo.mp4`, with screenshots of concurrent marching, combat, rules, army roles, promotion and king defeat. Promotion/king defeat use clearly labeled artificial fixtures, not a claimed continuation of the opening. The run also exercises flip, pause, and confirmed restart. A fresh frame directory prevents stale frames extending the recording.

Compact audit:

```sh
cargo run --release -- --demo --audit-size 1000x740 --capture-dir artifacts/battlefield/compact
```

The logical audit viewport is bounded by the real framebuffer; images are safely cropped. For occasional native-input screenshots, use `--capture-dir PATH --capture-on-demand` and press **F10**. No window-manager settings are changed.

## Code

- `src/game.rs`: rendering-independent fixed-step movement, pathfinding, momentum, traffic, autonomous combat/healing, unit identity, outcomes and simple army advance.
- `src/main.rs`: mouse/keyboard input, moving-unit picking and selection tracking, rendering, pause/layout handling, embedded assets and visual audit.
- `scripts/record-demo.sh`: repeatable screenshot/video recording.
