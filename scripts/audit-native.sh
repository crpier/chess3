#!/usr/bin/env bash
# Linux/Wayland native-input audit in a disposable, authenticated Xwayland display.
set -euo pipefail
cd "$(dirname "$0")/.."
for tool in Xwayland xauth cc magick tesseract timeout; do
  command -v "$tool" >/dev/null || { echo "$tool is required for the native audit." >&2; exit 2; }
done
[[ -n "${WAYLAND_DISPLAY:-}" ]] || { echo "This audit requires an active Wayland desktop." >&2; exit 2; }
out="${1:-artifacts/native-audit}"
mkdir -p "$out"
run="$(mktemp -d "$out/run-XXXXXX")"
auth="$run/Xauthority"
server=""; game=""
cleanup() {
  if [[ -n "$game" ]]; then kill "$game" 2>/dev/null || true; wait "$game" 2>/dev/null || true; fi
  if [[ -n "$server" ]]; then kill "$server" 2>/dev/null || true; wait "$server" 2>/dev/null || true; fi
  rm -f "$auth"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
cargo build --release
cc -std=c11 -Wall -Wextra -Werror -O2 scripts/native-input-test.c -lX11 -lm -o "$run/native-input-test"
number=""
for candidate in $(seq 93 119); do
  if [[ ! -S "/tmp/.X11-unix/X$candidate" && ! -e "/tmp/.X$candidate-lock" ]]; then number="$candidate"; break; fi
done
[[ -n "$number" ]] || { echo "No free audit display found." >&2; exit 1; }
touch "$auth"; chmod 600 "$auth"
cookie="$(od -An -N16 -tx1 /dev/urandom | tr -d '[:space:]')"
xauth -f "$auth" add ":$number" . "$cookie"
Xwayland ":$number" -geometry 1280x900 -fullscreen -noreset -nolisten tcp -auth "$auth" > "$run/server.log" 2>&1 & server=$!
for _ in $(seq 1 50); do
  [[ -S "/tmp/.X11-unix/X$number" ]] && break
  kill -0 "$server" 2>/dev/null || { cat "$run/server.log" >&2; exit 1; }
  sleep 0.1
done
[[ -S "/tmp/.X11-unix/X$number" ]] || { echo "Audit display did not start." >&2; exit 1; }
for mode in normal minimum; do
  cases="$run/$mode"
  mkdir -p "$cases/frames"
  DISPLAY=":$number" XAUTHORITY="$auth" ./target/release/chess3 --capture-dir "$cases/frames" --capture-on-demand > "$cases/game.log" 2>&1 & game=$!
  sleep 1
  DISPLAY=":$number" XAUTHORITY="$auth" timeout 60 "$run/native-input-test" "$cases" "$mode" > "$cases/input.log" 2>&1
  wait "$game"; game=""
  ./scripts/check-native-captures.sh "$cases"
  cmp "$cases/paused-before.png" "$cases/paused-after.png"
  printf 'Native %s audit passed; pause captures are byte-identical.\n' "$mode"
done
printf 'Fresh native evidence: %s\n' "$run"
