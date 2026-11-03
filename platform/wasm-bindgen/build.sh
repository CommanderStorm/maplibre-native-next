#!/usr/bin/env bash
# Builds web/pkg (the wasm module and its JavaScript glue) with wasm-pack.
#
# Needs: Rust with the wasm32-unknown-unknown target, wasm-pack, a
# wasm-bindgen CLI of the version Cargo.toml pins, and a clang that targets
# wasm32 (CC_wasm32_unknown_unknown) for rusqlite's bundled SQLite. With
# wasm-opt (binaryen) on PATH the module is optimized for size afterwards.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

python3 -I "$repo/tools/release/assemble.py" assemble --out "$repo/target/release-crate"
cd "$here"
wasm-pack build --release --target web --out-dir web/pkg --no-pack "$@"
if command -v wasm-opt >/dev/null; then
    wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
        --enable-mutable-globals --enable-reference-types --enable-multivalue \
        web/pkg/maplibre_native_next_web_bg.wasm -o web/pkg/maplibre_native_next_web_bg.wasm
fi
ls -l web/pkg/*.wasm
