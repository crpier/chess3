#!/usr/bin/env bash
# Fail closed against visible canvas output, with no private game-state inspection.
set -euo pipefail
[[ $# = 1 ]] || { echo "Usage: $0 CAPTURE_DIRECTORY" >&2; exit 2; }
for tool in magick tesseract; do command -v "$tool" >/dev/null || { echo "$tool is required." >&2; exit 2; }; done
out="$1"
mkdir -p "$out/ocr"
declare -A scanned=()
assert() {
  local name="$1" expected="$2" region="${3:-hud}" actual width height
  local key="$name-$region"
  [[ -f "$out/$name.png" ]] || { echo "Missing mobile capture: $name" >&2; exit 1; }
  if [[ -z "${scanned[$key]:-}" ]]; then
    width="$(magick identify -format '%w' "$out/$name.png")"
    height="$(magick identify -format '%h' "$out/$name.png")"
    if [[ "$region" = full ]]; then
      magick "$out/$name.png" "$out/ocr/$key-crop.png"
    elif (( width >= 500 && width > height )); then
      magick "$out/$name.png" -crop "240x${height}+$((width-248))+0" +repage "$out/ocr/$key-crop.png"
    else
      magick \( "$out/$name.png" -crop "${width}x64+0+0" +repage \) \( "$out/$name.png" -crop "${width}x112+0+$((height-112))" +repage \) -append "$out/ocr/$key-crop.png"
    fi
    magick "$out/ocr/$key-crop.png" -alpha off -resize 400% -negate -colorspace Gray "$out/ocr/$key.png"
    tesseract "$out/ocr/$key.png" stdout --psm 11 > "$out/ocr/$key.txt" 2> "$out/ocr/$key.log"
    scanned[$key]=1
  fi
  actual="$(tr -d '[:space:]' < "$out/ocr/$key.txt" | tr '[:upper:]' '[:lower:]')"
  [[ "$actual" = *"$expected"* ]] || { echo "Mobile assertion failed: $name expected $expected; OCR: $actual" >&2; exit 1; }
}
assert unfocused 'paused:focuslost'
assert opening 'startsin'
assert ready 'cpu:easy'
assert ready 'white:0/2ready'
assert selected 'whitepawn(ready'
for frame in drag-ignored multitouch-ignored cancel-ignored; do assert "$frame" 'white:0/2ready'; done
assert paused-before 'white:1/2'
assert paused-before 'paused'
cmp "$out/paused-before.png" "$out/paused-after.png"
assert cpu-normal 'cpu:normal'
assert cpu-hard 'cpu:hard'
assert cpu-easy 'cpu:easy'
assert bot-playing 'black'
bot_text="$(tr -d '[:space:]' < "$out/ocr/bot-playing-hud.txt" | tr '[:upper:]' '[:lower:]')"
[[ "$bot_text" =~ black:1/2|black(pawn|knight|bishop|rook|queen|king)arrivedat ]] || { echo "No mobile CPU reply: $bot_text" >&2; exit 1; }
assert menu 'gamecontrols' full
assert rules-first 'winbycapturingtheking' full
assert rules-last 'touch,localplayandpause' full
assert movement-king 'king' full
assert movement-pawn 'pawn' full
assert promotion 'whitenextpromotion:rook' full
assert flipped 'cpu:easy'
assert mode-confirm 'switchmode' full
assert local-orders 'white:1/2'
assert local-orders 'black:1/2'
assert local-orders 'touch:black'
assert new-confirm 'startanewgame' full
assert new-cancelled 'white:1/2'
assert new-cancelled 'black:1/2'
if [[ -f "$out/rotation-too-small.png" ]]; then assert rotation-too-small 'moreroomneeded' full; fi
assert rotated 'white:1/2'
assert rotated 'black:1/2'
assert resign-confirm 'resignblack' full
assert resigned 'whitewins'
assert focus-before 'paused:focuslost'
assert focus-before 'white:1/2'
cmp "$out/focus-before.png" "$out/focus-after.png"
assert fullscreen 'cpu:easy'
assert too-small 'moreroomneeded' full
assert restored 'cpu:easy'
echo 'Mobile rendered assertions passed: touch, gestures, readiness, both sides, CPU levels, pause, menus, orientation, promotion, resignation, focus, fullscreen, sizing.'
