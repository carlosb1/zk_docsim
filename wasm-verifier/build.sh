#!/bin/bash

cargo build --release --target wasm32-unknown-unknown
wasm-pack build --target web
cp -rf pkg/* ../backend/web/verifier/