#!/usr/bin/env sh
# Builds ariadesk-core to wasm and writes the JS bindings into ui/public/wasm.
set -e
cd "$(dirname "$0")/.."
cargo build -p ariadesk-core --release --target wasm32-unknown-unknown --features wasm
wasm-bindgen --target no-modules --no-typescript --out-dir ui/public/wasm --out-name ariadesk_core \
    target/wasm32-unknown-unknown/release/ariadesk_core.wasm
