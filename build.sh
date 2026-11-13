#!/usr/bin/env bash
# Build the plugin into dist/solidcraft-gridfinity/ and zip it for Install plugin…
# Needs: Rust with the wasm32-unknown-unknown target, wasm-tools
# (https://github.com/bytecodealliance/wasm-tools) and Python 3 for the zip.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --lib --target wasm32-unknown-unknown
id=solidcraft-gridfinity
version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
out=dist/$id
rm -rf dist
mkdir -p "$out"
wasm-tools component new "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/solidcraft_gridfinity.wasm" -o "$out/plugin.wasm"
cp plugin.json README.md LICENSE "$out/"
py=$(command -v python3 || command -v python)
(cd dist && "$py" -c "import shutil, sys; shutil.make_archive(sys.argv[1], 'zip', '.', sys.argv[2])" "$id-$version" "$id")
echo "built $out and dist/$id-$version.zip"
