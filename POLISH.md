# Gameplay polish log

## Direction: deliberate simultaneous chess plays

The RTS baseline (`b3bb4d6`) made repeated commanding and autonomous fighting too central. This pass restores chess-piece movement and capture while keeping continuous travel and simultaneous play.

- Two independent committed moves per side; no queue, cancel, retarget or army-wide advance.
- Ordinary travel takes 1.2 seconds per touching step, kings 0.8 seconds, knight jumps 1.2 seconds. A landing immediately releases its slot; there is no post-move cooldown.
- No health, attrition, healing or passive combat. Idle pieces stay put.
- Stationary captures resolve at the occupied waypoint, not on sprite contact. Crossing opposing ground movers trade simultaneously; knights vault intervening traffic until landing approach.
- Friendly routes are reserved. Stable IDs separate selection from vacated origins and current board occupancy.

These changes are intentionally uncommitted for review. Nothing has been pushed. Iterations are managed directly, without a recurring loop.

## Correctness and tactical clarity

1. **Capture timing:** early contact made a defeated king disappear while its capturer was still between squares. Captures now complete at landing. Regression tests verify exact role travel times and a winning attacker fully landed with no pending order.
2. **Reservation precedence:** stress testing exposed a newly arrived enemy overwriting a friendly route reservation. Two allies could then converge on one square. Static occupancy is collected first; friendly reservations take precedence in a second pass. A focused late-arriving-knight scenario prevents regression.
3. **Incoming royal threats:** moving attackers disappear from the static board, so static check detection alone lost the warning precisely when an attack was committed. Warnings now include committed routes to a stationary king. The bot also penalizes leaving its king on those routes and chooses an escape over an unrelated material grab. Warnings remain advisory, not legality restrictions or guaranteed capture predictions.
4. **Pause explanation:** status distinguishes manual pause, menu pause and focus loss. The simulation gate uses that same classification.
5. **Audit honesty:** artificial promotion and king-capture scenes have persistent footer labels; transient move feedback can no longer erase their fixture designation.

6. **Evasion window:** the old 0.45-tile collision radius traded off a corner king even after a half-second response and a legal sideways escape. A failing regression demonstrated this. Moving-center contact is now 0.25 tiles; both mirrored escapes pass, while genuine opposing crossings still trade. A two-slot queen-clearance/king-escape test also verifies this useful combination works without special castling rules, and preserves identities when the king occupies the departing queen's old anchor.
7. **Late blockers and determinism:** a slide stops at a newly occupied waypoint, captures at landing and releases its unused lane. The chunking regression advances past the new travel duration and asserts a real capture, not just pre-capture movement.

8. **Planning feedback:** selected movers show their destination and planned travel remaining. An idle piece selected with both slots full shows the next expected slot opening. Estimates derive from physical route progress, freeze on pause, round up to avoid misleading `0.0s`, and are labeled planned because captures can shorten travel. Destination previews are muted independently for whichever side has no slot available.

9. **Minimum layout:** a failing bounds test exposed the top tile overlapping the command-feedback row. Moving the board down 10 pixels preserves piece size and clears both header and footer across minimum through large layouts. Native controls and dialogs also pass at the real 1000×640 minimum, not merely a cropped 1000×740 audit.
10. **Feedback priority:** native minimum-size review exposed a bot arrival overwriting a rejected command. Human feedback now stays for 1.5 active seconds, freezes with pause, and then restores the latest simulation event. Terminal outcomes take priority. Rejections describe the attempted command in past tense so a slot that frees during the feedback interval is not falsely reported as still occupied. Issuing a local order also switches inspection to that side, and the inspector explicitly labels White/Black. A winning-capture regression ensures a held rejection never hides the outcome.

11. **Committed promotion visibility:** orders and the travel hint expose the promotion actually committed with a pawn's move, not a subsequently changed preset. A controller regression changes the preset in flight and verifies both the displayed Knight choice and the eventual Knight landing.

12. **Interception coordinates:** moving casualties were logged at their old routing anchors. A head-on long-slide regression exposed two misleading coordinates for the same encounter. Moving casualties now report “intercepted near” their physical nearest tile; stationary captures retain “captured at” the occupied square.

13. **Invalid-order boundary:** forged extreme coordinates could overflow the board predicate before an illegal pawn move was rejected. The board check now short-circuits bounded coordinates before addition, and submission validates the destination before promotion arithmetic. Both sides reject off-board and minimum/maximum integer destinations without changing state.

14. **Repeatable native verification:** promoted the successful isolated callback audit into checked-in scripts. The driver requires a numerically newer PNG with a complete IEND chunk; every run gets a fresh directory. OCR checks the actual rendered HUD rather than adding production test hooks. A missed flipped Black order and missing pointer-motion delivery both made assertions fail before the harness was corrected. Both sides now have explicit full-slot/no-queue checks, alongside redirects, four plays, dialogs, flipped inputs and resignation. The disposable display has per-run authentication, no TCP listener, and cleanup traps.

15. **Selection and readiness:** restored the complete legal-destination preview requested by the player. Empty destinations use small side-colored dots, captures use rings, and the current cursor target is emphasized. Waiting previews stay visible but muted. Per-side slot lights and an explicit ready/wait line distinguish available capacity from countdown, pause and full slots; idle blocked pieces no longer claim to be ready. Native OCR now checks countdown waiting, selected-piece readiness, full-slot waiting and readiness after landing. Both normal and minimum runs passed; images were reviewed under `artifacts/limited-chess/move-preview/run-GFRf3J/`.

16. **CPU difficulty:** Easy is now the default: a 2.4-second decision interval, one voluntary move slot and short varied legal moves without capture/safety scoring. Normal keeps tactical choices at 1.5 seconds; Hard preserves the original 0.65-second CPU. The sidebar button and C key cycle levels without restarting or changing committed orders. Restart/mode changes preserve the choice for the session. New regressions cover legal one-slot Easy play, harder-level capacity, think time, frozen timers and level preservation. Both native sizes passed keyboard and mouse selector checks in `artifacts/limited-chess/cpu-levels/run-OUHWCu/`; minimum-size selector imagery was reviewed.

## Verification

- `cargo test`: **44 passing tests**.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`, `cargo build --release`, `git diff --check`, `bash -n scripts/*.sh`: clean at this pass.
- Anti-spam coverage includes 1,000 rejected third orders and attempted redirects.
- Deterministic bot play checks slot limits, material conservation and unique stationary occupancy.
- **24 seeded mixed-play games**, up to 1,500 iterations each, vary side submission order, legal plays and time chunks. They check disjoint friendly reservations, exact static-board projection, stable identity uniqueness, finite positions and material conservation. Games stop on an outcome; these are regression stress tests, not a proof of all scenarios or competitive balance.
- A 30-second, 600-frame, 1280×900 graphical recording and 1000×740 compact captures were generated and reviewed after the latest feedback, promotion-inspection and interception-log changes (raw movie run `run-nYPITw`, compact frames in `compact-latest/`). The recording's artificial king fixture deliberately has the bot disabled; incoming-threat defense and timed escapes have focused simulation regressions.

Generated evidence (ignored by Git):

- `artifacts/limited-chess/chess3-demo.mp4`
- `artifacts/limited-chess/video-review.png`
- `artifacts/limited-chess/parallel-moves.png`
- `artifacts/limited-chess/incoming-king-threat.png`
- `artifacts/limited-chess/king-capture.png`
- `artifacts/limited-chess/compact-parallel.png`
- `artifacts/limited-chess/compact-movement.png`
- `artifacts/limited-chess/compact-king-threat.png`

### Native-input audit caveat

Native X11 automation was attempted, but fresh reviewed captures showed intermittent focus loss before orders could execute. Some earlier copies also picked stale frames from a reused capture directory. Those runs are **inconclusive**, not evidence that native mouse/keyboard gameplay passed. Their images are archived under `artifacts/limited-chess/native-inconclusive/`.

The native audit was subsequently repeated successfully in a temporary **rootful Xwayland session on `:93`**, isolating the game's X11 focus and pointer state from other applications. Fresh reviewed screenshots confirm third-order rejection, committed-piece rejection, two White plus two Black moves in flight, rules/movement dialogs, board flipping and Black resignation. Paused-before/after captures with **all four moves still in flight** are byte-identical. The game and temporary server exited cleanly; neither was left running. Evidence is in `artifacts/limited-chess/isolated-native/` and the 1000×640 `artifacts/limited-chess/isolated-minimum/`, including their input-test sources and logs. This verifies native callbacks in isolation; it does not turn the inconclusive direct-desktop runs into passing evidence.

Graphical audits used a temporary fullscreen request targeting only the game window when the compositor provided too little space. No desktop configuration, window rules or monitor modes were changed. Do not disable normal focus pausing merely to make an input harness pass. Future native runs need fresh capture directories, reliable real focus, capture freshness checks and assertions of the intended game state—not merely a zero exit code or identical idle screenshots.

Latest checked-in native pipeline: `./scripts/audit-native.sh artifacts/limited-chess/native-audit` passed both sizes with all rendered-HUD assertions and pause comparisons. Fresh evidence is in `artifacts/limited-chess/native-audit/run-5J8cg8/{normal,minimum}/`, with cropped OCR diagnostics and original PNGs. No production state-dump interface or focus bypass was added. Normal desktop focus integration remains a human smoke-test item.

## Next priorities

- Maintain the isolated native workflow. Normal desktop focus integration still deserves a human smoke test; avoid repeatedly taking focus from other applications.
- Extend sparse tactical scenarios around unequal arrival times, late blockers and knight landings.
- Evaluate human pacing and whether two slots, 1.2-second steps and faster king escapes reward planning sufficiently. Long slides deliberately expose a slot; competitive balance is not claimed.
- Keep the bot lightweight. Its threat handling is conservative and one-ply, not a prediction of every moving-body collision or future blocking play.
- Preserve restrained rendering, readable embedded typography and bounded piece art. Gameplay correctness comes before additional decoration.


## Desktop-browser port

17. **Shared WebAssembly build:** retained the Rust rules and Raylib renderer, targeting `wasm32-unknown-emscripten` with SDK 4.0.20. A small frame-scheduling seam uses browser animation callbacks without Asyncify. Browser frames must not call Raylib's `WindowShouldClose()` (which sleeps on Web) or impose desktop FPS sleeps. The static bundle includes the embedded assets and font; no runtime asset/CDN fetches are added.
18. **Browser interaction:** canvas pointer focus, Tab escape from GLFW's window-wide capture listeners, scrolling prevention for local controls, off-canvas/shortcut keyboard isolation, pause on document/window/canvas focus loss, canvas resizing and fullscreen. A visibility/blur signal resets the elapsed-frame clock, including interruptions that stop animation callbacks before Rust sees the loss. Q pauses; native Q still quits. Minimum size applies to the canvas, not the entire browser window.
19. **Headless evidence:** switched from the interactive browser after desktop use interfered with testing. `npm run test:web` creates an isolated headless Chromium profile and a temporary loopback server; both are closed on exit. Public input and rendered-canvas OCR assertions cover actual movement by both sides, an Easy CPU reply, selectors, dialogs, readiness, pause/focus, layout and fullscreen. No production state-inspection hook or focus-pause bypass was introduced. Six Node host tests cover loading/error states, keyboard boundaries, fullscreen and interruption-clock signals. Firefox/Safari and real hidden-tab behavior remain compatibility smoke tests; headless canvas-focus pause is verified.
20. **Native shutdown regression:** the frame closure initially captured the window before GPU assets and therefore closed the context before `UnloadFont()`. Native exit testing caught a SIGSEGV; the core stack confirmed `rlUnloadTexture → UnloadTexture → UnloadFont → closure drop`, and the log showed the window already closed. An intact `Renderer` owner now drops font/textures before the window. Fresh native normal/minimum audits pass clean shutdown as well as all existing HUD assertions. This is why the native audit remains part of web-port verification.

Verification for the web pass: 44 Rust tests, six host tests, native and Emscripten Clippy with warnings denied, formatting/shell/whitespace checks, release native/WebAssembly builds, and isolated headless rendered-HUD audits. The native evidence is `artifacts/web/native-regression/run-cwPwhS/`; latest headless evidence is `artifacts/web/headless-7nNZAo/`. Reviewed selected, local simultaneous orders and the exact 1000×640 minimum canvas images. The generated four-file bundle is about 1.8 MB uncompressed. Nothing published or pushed.


21. **Browser high-DPI resolution:** a headless regression reproduced a 2× browser using only a 1280×917 framebuffer for a 1280×917 CSS canvas. The framebuffer now scales with device-pixel ratio (1×–4×), while layout, minimum size and picking remain logical CSS pixels. A matched 2D drawing scale and inverse mouse mapping preserve move targets; the canvas CSS size stays independent of GLFW's backing resize. Web text uses a 128-pixel font atlas. The density is polled each frame, so live display/zoom changes do not require a CSS-size change or restart. The full headless audit passes at 1× and 2×; live 1.25× transitions also pass actual selection/movement assertions. At the 1000×640 logical minimum, the 2× framebuffer is exactly 2000×1280. Device-resolution text and board crops were reviewed. Evidence: `artifacts/web/headless-kT5dnK/{dpr-1,dpr-2}/`, `artifacts/web/hidpi-hud.png`, and `artifacts/web/hidpi-board.png`. Latest checks: **45 Rust tests**, **seven host tests**, native/Web Clippy, both native audits (`artifacts/web/hidpi-native/run-CGvbH6/`), formatting and whitespace/shell checks. Testing remains headless; the user's browser was not manipulated.


22. **Mobile browser presentation and touch input:** added `src/mobile.rs` beside the shared board renderer and rules, not a second game engine. Primary-touch browsers get a full-width portrait board or landscape side panel, explicit slot/readiness text, king warnings, legal previews, committed paths/promotion, feedback and 44–48-pixel controls. A paused control sheet provides flip/clear, confirmed new/mode/resign, promotion presets, paginated full rules and six movement diagrams. Local play has an explicit White/Black touch selector, with independent immutable orders and no turns. Full-hex stationary selection is more forgiving than artwork picking; moving selection still uses canonical physical identity. Desktop minimum-size rules and native controls are unchanged. Dynamic viewport height and safe-area insets are included; insufficient touch space pauses instead of clipping (notably small-phone landscape).

   Mobile input is one completed stationary single-finger tap, captured ahead of GLFW’s touch-to-mouse listeners. It prevents duplicate compatibility mouse input and preserves fast taps between frames; drags/multi-touch/cancelled touches never order a move. A bounded raw-input buffer is cleared on focus/visibility/resize interruptions; this is not a legal-move queue, and App/Game still reject full slots or committed pieces. Touch cancellation must check `event.cancelable` before preventing default; the initial headless cancellation test caught a browser console error here, now fixed.

   Regression evidence: all-touch/no-keyboard public-input audits pass at **390×844 / 3×**, **320×568 / 2×**, **844×390 / 2×**, and **768×1024 / 2×**. They assert rendered selection, completed moves, gesture rejection, CPU replies/levels, exact paused frames, both local sides, paginated rules/movement, promotion, flip, confirmations/cancel, rotation, Black resignation, focus pause, fullscreen and too-small recovery. Fast simultaneous local input is delivered as actual ordered browser touch events, avoiding software-WebGL RPC latency taking longer than pawn travel. Evidence: `artifacts/web/headless-vQ5pZ4/phone/`, `headless-cRQBqy/small-phone/`, `headless-k7f2f6/landscape-phone/`, and `headless-tZTGFS/tablet/` (all under `artifacts/web/`). Reviewed small-phone selection/rules, landscape menu and tablet board screenshots. Desktop 1×/2× and live 1.25× regressions pass in `artifacts/web/headless-OIE6Kr/`. Native normal/minimum audits and clean shutdown pass in `artifacts/web/mobile-native/run-MOf5Xe/`. Latest checks: **49 Rust tests**, **ten host tests**, native/Web Clippy warnings denied, release browser build, formatting, JS/shell syntax and whitespace.

   Real Android/iOS device and background/app-switch smoke tests remain a follow-up. Playwright WebKit downloaded but cannot launch on this Arch host: its Ubuntu fallback needs ICU74/libxml2/flite runtime dependencies (`artifacts/web/webkit-smoke.log`); no system packages or desktop configuration were changed. This is Chromium touch emulation evidence, not a claim of real Safari validation. README explains trusted-LAN serving for a phone; nothing is published or pushed.

   Final combined audit also passed in one run: **`artifacts/web/headless-Zg8Czh/`**, with six successful `report.json` files (`dpr-1`, `dpr-2`, `phone`, `small-phone`, `landscape-phone`, `tablet`), zero browser errors and all rendered/OCR assertions. Log: `artifacts/web/mobile-full-audit.log`; exit status: **0**. Its temporary headless browser and loopback server were cleaned up. No permanent or LAN server was started; real-phone serving is an explicit README command.
