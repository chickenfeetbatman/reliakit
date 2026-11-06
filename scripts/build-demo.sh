#!/usr/bin/env bash
# Build the WASM browser demo into site/demo/.
#
# Run from the repo root: scripts/build-demo.sh
# The demo is self-contained: the front end from gui/ plus the
# reliakit_wasm.wasm module built from the wasm/ crate.
set -euo pipefail
cd "$(dirname "$0")/.."

export PATH="$HOME/.cargo/bin:$PATH"

cargo build --release --target wasm32-unknown-unknown -p reliakit-wasm

rm -rf site/demo
mkdir -p site/demo
cp gui/index.html site/demo/
if [ -d gui/src ]; then cp -r gui/src site/demo/; fi
if [ -d gui/styles ]; then cp -r gui/styles site/demo/; fi
if [ -d gui/assets ]; then cp -r gui/assets site/demo/; fi
cp target/wasm32-unknown-unknown/release/reliakit_wasm.wasm site/demo/reliakit_wasm.wasm

echo "Demo assembled in site/demo/"
