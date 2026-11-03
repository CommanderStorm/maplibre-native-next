#!/usr/bin/env bash
# Renders the C++ reference images for tools/liberty-compare.
#
# For every view in views.txt (or only those whose "style/name" matches the
# optional regex argument) this runs the C++ `mbgl-render` against
# https://tiles.openfreemap.org/styles/<style>, writes the PNG as the view's
# expected.png and creates the matching style.json (the OpenFreeMap style
# with the view's camera and test metadata) for the Rust render-test runner.
# Every resource mbgl-render downloads lands in the C++ cache database
# $LIBERTY_COMPARE_DIR/cache.db, which rtlib.sh then reads offline.
#
# Environment:
#   MBGL_RENDER          path to a C++ mbgl-render binary (required; see README)
#   LIBERTY_COMPARE_DIR  work directory (default ~/.cache/mln-liberty-compare)
#   VK_DRIVER_FILES      Vulkan ICD (default: lavapipe on NixOS)
#   LIBERTY_COMPARE_VIEWS  view list (default: views.txt next to this script)
#
# Usage: tools/liberty-compare/mklib.sh [regex]
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
work="${LIBERTY_COMPARE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/mln-liberty-compare}"
icd="${VK_DRIVER_FILES:-/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json}"
filter="${1:-.}"
views="${LIBERTY_COMPARE_VIEWS:-$here/views.txt}"

if [ -z "${MBGL_RENDER:-}" ] || [ ! -x "$MBGL_RENDER" ]; then
    echo "MBGL_RENDER must point to a C++ mbgl-render binary (see README.md)" >&2
    exit 1
fi
if [ ! -d "$root/.nix-shell" ]; then
    echo "$root/.nix-shell is missing: run scripts/bazel once to create it" >&2
    exit 1
fi

mkdir -p "$work/styles" "$work/tests/render-tests"

grep -v '^#' "$views" | while read -r style name lon lat zoom bearing pitch ratio width height; do
    [ -n "$style" ] || continue
    echo "$style/$name" | grep -Eq -- "$filter" || continue

    if [ ! -s "$work/styles/$style.json" ]; then
        curl -sSf -m 60 -o "$work/styles/$style.json" "https://tiles.openfreemap.org/styles/$style"
    fi

    dir="$work/tests/render-tests/$style/$name"
    mkdir -p "$dir"
    if nix develop "path:$root/.nix-shell#linux" --command env VK_DRIVER_FILES="$icd" \
        "$MBGL_RENDER" -s "https://tiles.openfreemap.org/styles/$style" \
        -x "$lon" -y "$lat" -z "$zoom" -b "$bearing" -p "$pitch" -r "$ratio" \
        -w "$width" -h "$height" -o "$dir/expected.png" -c "$work/cache.db" \
        >"$dir/cpp.log" 2>&1; then
        echo "rendered $style/$name"
    else
        echo "mbgl-render failed for $style/$name (see $dir/cpp.log)" >&2
    fi

    python3 -I - "$work/styles/$style.json" "$dir/style.json" \
        "$lon" "$lat" "$zoom" "$bearing" "$pitch" "$ratio" "$width" "$height" <<'EOF'
import json, sys
src, dst, lon, lat, zoom, bearing, pitch, ratio, width, height = sys.argv[1:]
style = json.load(open(src))
style["center"] = [float(lon), float(lat)]
style["zoom"] = float(zoom)
style["bearing"] = float(bearing)
style["pitch"] = float(pitch)
style["metadata"] = {"test": {"width": int(width), "height": int(height), "pixelRatio": float(ratio)}}
json.dump(style, open(dst, "w"), indent=1)
EOF
done
