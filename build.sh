#!/bin/bash -ex
# setup wasm
#rustup target add wasm32-unknown-unknown
#cargo install wasm-bindgen-cli

#cargo run --release

# build new wasm file
# cargo build --release --target wasm32-unknown-unknown

#wasm-bindgen target/wasm32-unknown-unknown/release/wasm_verifier.wasm \
#  --out-dir ../web \
#  --target web

#cp host/receipt.bin ./web/receipt.bin
