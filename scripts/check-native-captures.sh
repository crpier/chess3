#!/usr/bin/env bash
# Assertions at the approved native-input -> F10 screenshot seam, not game test hooks.
set -euo pipefail
[[ $# = 1 ]] || { echo "Usage: $0 CAPTURE_DIRECTORY" >&2; exit 2; }
for tool in magick tesseract; do
  command -v "$tool" >/dev/null || { echo "$tool is required for native screenshot assertions." >&2; exit 2; }
done
out="$1"
mkdir -p "$out/ocr"
: > "$out/ocr/tesseract.log"
normalize() { tr -d '[:space:]' | tr '[:upper:]' '[:lower:]'; }
text() {
  local name="$1" region="$2" png="$out/$1.png" width crop
  [[ -f "$png" ]] || { echo "Missing native capture: $png" >&2; return 1; }
  width="$(magick identify -format '%w' "$png")" || return 1
  case "$region" in
    hud) crop="290x305+$((width-325))+35" ;;
    feedback) crop="$((width-370))x23+25+79" ;;
    full) crop="$(magick identify -format '%wx%h' "$png")+0+0" ;;
    *) echo "Unknown capture region: $region" >&2; exit 2 ;;
  esac
  # Drop alpha before negation: otherwise ImageMagick can negate the alpha too,
  # leaving text visible to image viewers but transparent to OCR compositing.
  magick "$png" -crop "$crop" +repage -alpha off -resize 400% -negate -colorspace Gray "$out/ocr/$name-$region.png" || return 1
  tesseract "$out/ocr/$name-$region.png" stdout --psm 11 2>> "$out/ocr/tesseract.log" > "$out/ocr/$name-$region.txt" || return 1
  normalize < "$out/ocr/$name-$region.txt"
}
require() {
  local actual="$1" expected="$2" context="$3"
  if [[ "$actual" != *"$expected"* ]]; then
    echo "Native screenshot assertion failed: $context expected $expected" >&2
    echo "OCR: $actual" >&2
    exit 1
  fi
}
counts() {
  local name="$1" white="$2" black="$3" actual
  actual="$(text "$name" hud)"
  if [[ "$white" = any ]]; then
    [[ "$actual" =~ white:[012]/2movesinflight ]] || { echo "Invalid White slot HUD: $actual" >&2; exit 1; }
  else require "$actual" "white:$white/2movesinflight" "$name"; fi
  if [[ "$black" = any ]]; then
    [[ "$actual" =~ black:[012]/2movesinflight ]] || { echo "Invalid Black slot HUD: $actual" >&2; exit 1; }
  else require "$actual" "black:$black/2movesinflight" "$name"; fi
}
require "$(text opening hud)" 'cpu:easy' 'default CPU level'
require "$(text cpu-normal hud)" 'cpu:normal' 'Normal CPU key selector'
require "$(text cpu-hard hud)" 'cpu:hard' 'Hard CPU key selector'
require "$(text cpu-easy hud)" 'cpu:easy' 'Easy CPU mouse selector'
require "$(text opening hud)" 'white:wait/startsin' 'opening wait indicator'
require "$(text selected hud)" 'white:ready/2freeslots' 'ready indicator'
require "$(text third-blocked hud)" 'white:wait/slotin' 'full-slot wait indicator'
counts third-blocked 2 any
require "$(text third-blocked feedback)" 'bothslotswereoccupied' 'third order rejected'
counts released 0 any
require "$(text released hud)" 'white:ready/2freeslots' 'slot release ready indicator'
require "$(text released hud)" 'knight(ready)' 'rejected order was not queued'
counts committed 1 any
require "$(text committed feedback)" 'alreadycommitted' 'redirect rejected'
counts four-plays 2 2
require "$(text four-plays hud)" 'local/twoplayers' 'local mode'
require "$(text paused-before hud)" 'paused' 'manual pause'
counts paused-before 2 2
cmp "$out/paused-before.png" "$out/paused-after.png"
counts black-third-blocked any 2
require "$(text black-third-blocked feedback)" 'black:moverejected:bothslotswereoccupied' 'Black third order rejected'
counts black-slot-freed any 1
require "$(text black-slot-freed hud)" 'knight(ready)' 'Black rejected order was not queued'
require "$(text rules full)" 'thehexchessfieldguide' 'rules dialog'
require "$(text movement full)" 'howthepiecesmove' 'movement dialog'
counts flipped-orders 1 1
require "$(text black-resigned hud)" 'whitewins!' 'Black resignation'
require "$(text black-resigned hud)" 'byresignation' 'resignation outcome'
echo 'Native screenshot assertions passed: cap, no queue, commitment, four plays, pause, dialogs, flipped orders, resignation.'
