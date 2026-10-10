#!/usr/bin/env bash
# Fail-closed assertions on canvas screenshots from the headless public-input audit.
set -euo pipefail
[[ $# = 1 ]] || { echo "Usage: $0 CAPTURE_DIRECTORY" >&2; exit 2; }
for tool in magick tesseract; do command -v "$tool" >/dev/null || { echo "$tool is required." >&2; exit 2; }; done
out="$1"
mkdir -p "$out/ocr"
assert() {
  local name="$1" expected="$2" region="${3:-hud}" width height crop actual
  [[ -f "$out/$name.png" ]] || { echo "Missing web capture: $name" >&2; exit 1; }
  width="$(magick identify -format '%w' "$out/$name.png")"
  height="$(magick identify -format '%h' "$out/$name.png")"
  if [[ "$region" = hud ]]; then crop="290x305+$((width-325))+35"; else crop="${width}x${height}+0+0"; fi
  magick "$out/$name.png" -crop "$crop" +repage -alpha off -resize 400% -negate -colorspace Gray "$out/ocr/$name-$region.png"
  tesseract "$out/ocr/$name-$region.png" stdout --psm 11 > "$out/ocr/$name-$region.txt" 2> "$out/ocr/$name-$region.log"
  actual="$(tr -d '[:space:]' < "$out/ocr/$name-$region.txt" | tr '[:upper:]' '[:lower:]')"
  [[ "$actual" = *"$expected"* ]] || { echo "Web HUD assertion failed: $name expected $expected; OCR: $actual" >&2; exit 1; }
}
assert unfocused 'paused:focuslost'
assert opening 'white:wait/startsin'
assert selected 'pawn(ready)'
assert selected 'white:ready/2freeslots'
assert paused-before 'white:1/2movesinflight'
assert paused-before 'paused'
cmp "$out/paused-before.png" "$out/paused-after.png"
assert bot-playing 'black' full
bot_text="$(tr -d '[:space:]' < "$out/ocr/bot-playing-full.txt" | tr '[:upper:]' '[:lower:]')"
[[ "$bot_text" =~ black:1/2movesinflight|black(pawn|knight|bishop|rook|queen|king)arrivedat ]] || {
  echo "No rendered evidence of a CPU reply: $bot_text" >&2; exit 1;
}
assert cpu-normal 'cpu:normal'
assert cpu-hard 'cpu:hard'
assert cpu-easy 'cpu:easy'
assert rules 'thehexchessfieldguide' full
assert local-orders 'local'
assert local-orders 'twoplayers'
assert local-orders 'white:1/2movesinflight'
assert local-orders 'black:1/2movesinflight'
cmp "$out/blur-before.png" "$out/blur-after.png"
assert focus-before 'paused:focuslost'
assert focus-before 'black:1/2movesinflight'
cmp "$out/focus-before.png" "$out/focus-after.png"
assert minimum 'local'
assert minimum 'twoplayers'
assert too-small 'pleasewidenthewindowtoplay' full
assert q-paused 'paused'
assert density-changed-selected 'pawn(ready)'
assert density-changed-move 'white:1/2movesinflight'
echo 'Web canvas assertions passed: loading, ready, movement, levels, rules, both sides, pause, focus, compact resize, undersized pause, Q.'
