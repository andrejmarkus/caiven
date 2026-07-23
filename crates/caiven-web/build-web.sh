#!/usr/bin/env bash
# Builds caiven-web for wasm32-unknown-emscripten. Must run where `emcc`/`emar`
# are on PATH (the emscripten SDK); see the Docker recipe below for a
# throwaway container that has it.
#
# Docker (run from the repo root):
#   MSYS_NO_PATHCONV=1 docker run --rm -v "$(pwd):/work" -w /work \
#     emscripten/emsdk:latest bash crates/caiven-web/build-web.sh
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
fi
rustup target add wasm32-unknown-emscripten

export CC_wasm32_unknown_emscripten=emcc
export AR_wasm32_unknown_emscripten=emar
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER=emcc
# Vendored Lua's C objects must be compiled under the same wasm exception
# scheme Rust's emscripten target links against, or it's a link-time ABI
# mismatch (undefined symbol: __cxa_find_matching_catch_3).
export EMCC_CFLAGS="-fwasm-exceptions"

EXPORTED_FUNCS='["_caiven_new","_caiven_load_cart","_caiven_set_button","_caiven_tick","_caiven_pixels","_caiven_width","_caiven_height","_malloc","_free"]'

export EMCC_CFLAGS="$EMCC_CFLAGS -sEXPORTED_FUNCTIONS=$EXPORTED_FUNCS -sEXPORTED_RUNTIME_METHODS=[ccall,cwrap,HEAPU8] -sMODULARIZE=1 -sEXPORT_NAME=CaivenModule -sENVIRONMENT=web,node -sALLOW_MEMORY_GROWTH=1"

cargo build -p caiven-web --release --target wasm32-unknown-emscripten

OUT_DIR="target/wasm32-unknown-emscripten/release"
echo "Built: $OUT_DIR/caiven_web.js + $OUT_DIR/caiven_web.wasm"
