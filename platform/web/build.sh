#!/usr/bin/env bash
# Builds the @bartlwrs/mln-next-web npm package into dist/: the JavaScript
# API (js/), and the engine (src/lib.rs) as two wasm modules, each with its
# wasm-bindgen glue:
#
# - dist/wasm/single: no shared memory. It runs on the page and does its
#   background work between frames.
# - dist/wasm/threads: shared memory and atomics. It runs in a worker, with its
#   thread pools on more workers; pages need cross-origin isolation for it.
#
# Needs: Rust with the wasm32-unknown-unknown target and the rust-src
# component (the threaded module rebuilds std with atomics, through
# RUSTC_BOOTSTRAP), and the wasm-bindgen CLI of the version Cargo.toml pins.
# With wasm-opt (binaryen) on PATH, the modules are optimized afterwards.
#
# Publish with `npm publish dist`.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$repo/target/web}"
out="$here/dist"

python3 -I "$repo/tools/release/assemble.py" assemble --out "$repo/target/release-crate" >/dev/null
version="$(python3 -I "$repo/tools/release/assemble.py" version)"

wanted="$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' "$here/Cargo.toml")"
have="$(wasm-bindgen --version | awk '{print $2}')"
if [[ "$have" != "$wanted" ]]; then
  echo "build.sh: wasm-bindgen $have on PATH, Cargo.toml pins $wanted" >&2
  exit 1
fi

rm -rf "$out"
mkdir -p "$out/wasm"

# build <variant> [cargo arguments]
build() {
  local variant="$1"
  shift
  cargo build --manifest-path "$here/Cargo.toml" --release --target wasm32-unknown-unknown \
    --target-dir "$target_dir/$variant" "$@"
  wasm-bindgen --target web --out-dir "$out/wasm/$variant" --out-name maplibre_native_next_web \
    "$target_dir/$variant/wasm32-unknown-unknown/release/maplibre_native_next_web.wasm"
  if command -v wasm-opt >/dev/null; then
    local module="$out/wasm/$variant/maplibre_native_next_web_bg.wasm"
    wasm-opt -O3 --enable-threads --enable-bulk-memory --enable-nontrapping-float-to-int \
      --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue \
      "$module" -o "$module"
  fi
}

RUSTFLAGS="${RUSTFLAGS:-}" build single

# Shared, imported memory, and the TLS exports wasm-bindgen's thread support
# needs (rustc doesn't add them for +atomics by itself).
RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals \
-C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=4294967296 \
-C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
-C link-arg=--export=__tls_align -C link-arg=--export=__tls_base" \
  build threads -Z build-std=std,panic_abort

cp "$here"/js/*.js "$here/js/index.d.ts" "$out/"
cp "$here/README.md" "$out/README.md"
cp "$repo/LICENSE.md" "$out/LICENSE.md"
# package.json with the version of the crate this package is built from.
python3 -I - "$here/package.json" "$version" "$out/package.json" <<'PY'
import json, sys
package = json.load(open(sys.argv[1]))
package["version"] = sys.argv[2]
with open(sys.argv[3], "w") as f:
    json.dump(package, f, indent=2)
    f.write("\n")
PY
echo "$out: @bartlwrs/mln-next-web $version"
ls -l "$out"/wasm/*/*.wasm
