#!/usr/bin/env bash
# Static desktop-browser build. Source emsdk_env.sh before running this script.
set -euo pipefail
cd "$(dirname "$0")/.."
for tool in emcc cargo cmake rustup; do
  command -v "$tool" >/dev/null || { echo "$tool is required; activate the Emscripten SDK first." >&2; exit 2; }
done
rustup target list --installed | grep -qx wasm32-unknown-emscripten || {
  echo 'Run: rustup target add wasm32-unknown-emscripten' >&2; exit 2;
}
export EMCC_CFLAGS='-O3 -sUSE_GLFW=3 -sGL_ENABLE_GET_PROC_ADDRESS=1'
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER=emcc
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS='-C link-arg=-sUSE_GLFW=3 -C link-arg=-sGL_ENABLE_GET_PROC_ADDRESS=1 -C link-arg=-sALLOW_MEMORY_GROWTH=1 -C link-arg=-sSTACK_SIZE=8388608 -C link-arg=-sEXIT_RUNTIME=0 -C link-arg=-sASSERTIONS=1'
cargo build --release --target wasm32-unknown-emscripten
out=target/web
mkdir -p "$out"
cp web/index.html web/host.js "$out/"
cp target/wasm32-unknown-emscripten/release/chess3.js target/wasm32-unknown-emscripten/release/chess3.wasm "$out/"
printf '\nWeb build: %s\nServe with: npx --yes http-server@14.1.1 target/web -a 127.0.0.1 -p 8080 -c-1\n' "$out"
