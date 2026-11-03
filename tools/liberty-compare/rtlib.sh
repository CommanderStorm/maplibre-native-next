#!/usr/bin/env bash
# Renders the views prepared by mklib.sh with the Rust render-test runner,
# offline from the C++ cache database, and compares them with the C++
# references (the runner's pixelmatch metric and default allowed diff).
#
# Environment:
#   RENDER_TEST          Rust render-test binary (default:
#                        bazel-bin/tools/render-test/render-test; build it with
#                        `scripts/bazel build //tools/render-test:render-test`)
#   LIBERTY_COMPARE_DIR  work directory (default ~/.cache/mln-liberty-compare)
#   VK_DRIVER_FILES      Vulkan ICD (default: lavapipe on NixOS)
#
# Usage: tools/liberty-compare/rtlib.sh [regex]   (matched against the test id,
#        e.g. 'liberty/' or 'positron/ams13')
# Results: $LIBERTY_COMPARE_DIR/results/manifest.html (+ actual/diff PNGs).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
work="${LIBERTY_COMPARE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/mln-liberty-compare}"
icd="${VK_DRIVER_FILES:-/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json}"
binary="${RENDER_TEST:-$root/bazel-bin/tools/render-test/render-test}"
filter="${1:-.}"

if [ ! -f "$work/cache.db" ] || [ ! -d "$work/tests/render-tests" ]; then
    echo "no references in $work: run mklib.sh first" >&2
    exit 1
fi

# A private copy, so a concurrent Bazel build can't replace it mid-run.
mkdir -p "$work/bin" "$work/results"
install -m 755 "$binary" "$work/bin/render-test"
sed "s#@WORK_DIR@#$work#g" "$here/manifest.template.json" >"$work/manifest.json"

cd "$work"
exec nix develop "path:$root/.nix-shell#linux" --command env VK_DRIVER_FILES="$icd" \
    "$work/bin/render-test" -p "$work/manifest.json" -f "$filter" --output "$work/results"
