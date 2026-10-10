#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v ffmpeg >/dev/null || { echo "FFmpeg is required to encode the video." >&2; exit 1; }
out="${1:-artifacts/limited-chess}"
mkdir -p "$out/frames"
# A dedicated fresh directory prevents old frames from extending a new recording.
frames="$(mktemp -d "$out/frames/run-XXXXXX")"
cargo run --release -- --demo --audit-size 1280x900 --capture-dir "$frames" > "$out/demo.log" 2>&1
ffmpeg -hide_banner -loglevel warning -y -framerate 20 -i "$frames/frame-%04d.png" \
  -vf 'scale=1280:900:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=1280:900:(ow-iw)/2:(oh-ih)/2:color=0x131b23,setsar=1' \
  -c:v libx264 -crf 20 -pix_fmt yuv420p "$out/chess3-demo.mp4"
cp "$frames/frame-0070.png" "$out/selected-piece.png"
cp "$frames/frame-0080.png" "$out/incoming-orders.png"
cp "$frames/frame-0090.png" "$out/parallel-moves.png"
cp "$frames/frame-0160.png" "$out/gameplay.png"
cp "$frames/frame-0200.png" "$out/combat.png"
cp "$frames/frame-0230.png" "$out/rules.png"
cp "$frames/frame-0250.png" "$out/movement.png"
cp "$frames/frame-0360.png" "$out/promotion.png"
cp "$frames/frame-0400.png" "$out/incoming-king-threat.png"
cp "$frames/frame-0500.png" "$out/king-capture.png"
printf 'Video: %s/chess3-demo.mp4\nScreenshots: %s/*.png\nRaw frames: %s\n' "$out" "$out" "$frames"
