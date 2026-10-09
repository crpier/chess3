#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v ffmpeg >/dev/null || { echo "FFmpeg is required to encode the video." >&2; exit 1; }
out="${1:-artifacts}"
mkdir -p "$out/frames"
# A dedicated fresh directory prevents old frames from extending a new recording.
frames="$(mktemp -d "$out/frames/run-XXXXXX")"
cargo run --release -- --demo --capture-dir "$frames" > "$out/demo.log" 2>&1
ffmpeg -hide_banner -loglevel warning -y -framerate 20 -i "$frames/frame-%04d.png" \
  -vf 'scale=1280:1400:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=1280:1400:(ow-iw)/2:(oh-ih)/2:color=0x131b23,setsar=1' \
  -c:v libx264 -crf 20 -pix_fmt yuv420p "$out/chess3-demo.mp4"
cp "$frames/frame-0025.png" "$out/selected-piece.png"
cp "$frames/frame-0160.png" "$out/gameplay.png"
cp "$frames/frame-0230.png" "$out/rules.png"
cp "$frames/frame-0350.png" "$out/promotion.png"
cp "$frames/frame-0410.png" "$out/checkmate.png"
printf 'Video: %s/chess3-demo.mp4\nScreenshots: %s/*.png\nRaw frames: %s\n' "$out" "$out" "$frames"
