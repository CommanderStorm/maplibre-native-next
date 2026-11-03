#!/usr/bin/env bash
# Builds web-threads/pkg: the threaded variant, on shared memory. The thread
# pools run on Web Workers (see src/lib.rs); the page needs cross-origin
# isolation (COOP/COEP) for SharedArrayBuffer.
#
# Needs what build.sh needs, plus the rust-src component: std is rebuilt
# with atomics (-Zbuild-std, through RUSTC_BOOTSTRAP on a stable toolchain).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

python3 -I "$repo/tools/release/assemble.py" assemble --out "$repo/target/release-crate"
cd "$here"
export RUSTC_BOOTSTRAP=1
# Shared, imported memory and the TLS exports wasm-bindgen's thread support
# needs (rustc doesn't add them for +atomics by itself).
export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals \
-C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=4294967296 \
-C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
-C link-arg=--export=__tls_align -C link-arg=--export=__tls_base"
# SQLite's C objects must be built for shared memory too.
export CFLAGS_wasm32_unknown_unknown="${CFLAGS_wasm32_unknown_unknown:-} -matomics -mbulk-memory"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}-threads"
wasm-pack build "$here" --release --target web --out-dir web-threads/pkg --no-pack "$@" -- -Z build-std=std,panic_abort
if command -v wasm-opt >/dev/null; then
    wasm-opt -O3 --enable-threads --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
        --enable-mutable-globals --enable-reference-types --enable-multivalue \
        web-threads/pkg/maplibre_native_next_web_bg.wasm -o web-threads/pkg/maplibre_native_next_web_bg.wasm
fi
ls -l web-threads/pkg/*.wasm
