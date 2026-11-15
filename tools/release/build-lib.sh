#!/usr/bin/env bash
# Builds the GDExtension in the release profile and stages it for an export
# (prompt 044, docs/RELEASE.md): target/export/ gets the bridge, Valve's
# steam_api library next to it and THIRD_PARTY_LICENSES.txt.
# godot/mud_and_steel.gdextension points its release entries there.
#
# macOS: a universal library (arm64 + x86_64) when the Rust target
# x86_64-apple-darwin is installed (`rustup target add x86_64-apple-darwin`);
# otherwise only the native architecture, with a warning. REQUIRE_UNIVERSAL=1
# (set in CI) turns the warning into an error.
#
# Works in bash on macOS, Linux and Git Bash on Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
OUT=target/export
mkdir -p "$OUT"

# Newest steam_api library of a steamworks-sys build below $1.
steam_lib() {
    find "$1" -path '*steamworks-sys*' -type f \( -name 'libsteam_api.dylib' -o -name 'libsteam_api.so' -o -name 'steam_api64.dll' \) 2>/dev/null \
        | while read -r f; do echo "$(date -r "$f" +%s 2>/dev/null || stat -c %Y "$f") $f"; done \
        | sort -rn | head -n 1 | cut -d' ' -f2-
}

case "$(uname -s)" in
Darwin)
    if rustup target list --installed | grep -q '^x86_64-apple-darwin$'; then
        for triple in aarch64-apple-darwin x86_64-apple-darwin; do
            cargo build -p bridge --release --locked --target "$triple"
        done
        lipo -create -output "$OUT/libbridge.dylib" \
            target/aarch64-apple-darwin/release/libbridge.dylib \
            target/x86_64-apple-darwin/release/libbridge.dylib
        lib="$(steam_lib target/aarch64-apple-darwin/release/build)"
    else
        if [[ "${REQUIRE_UNIVERSAL:-0}" == "1" ]]; then
            echo "error: x86_64-apple-darwin is not installed; a universal build needs it" >&2
            exit 1
        fi
        echo "warning: Rust target x86_64-apple-darwin not installed: building for $(uname -m) only (not universal)" >&2
        cargo build -p bridge --release --locked
        cp target/release/libbridge.dylib "$OUT/"
        lib="$(steam_lib target/release/build)"
    fi
    ;;
Linux)
    cargo build -p bridge --release --locked
    cp target/release/libbridge.so "$OUT/"
    lib="$(steam_lib target/release/build)"
    ;;
MINGW* | MSYS* | CYGWIN*)
    cargo build -p bridge --release --locked
    cp target/release/bridge.dll "$OUT/"
    lib="$(steam_lib target/release/build)"
    ;;
*)
    echo "error: unsupported system $(uname -s)" >&2
    exit 1
    ;;
esac

if [[ -z "$lib" ]]; then
    echo "error: steam_api library not found in the steamworks-sys build" >&2
    exit 1
fi
cp "$lib" "$OUT/"

if command -v python3 >/dev/null; then PY=python3; else PY=python; fi
"$PY" tools/release/third_party_licenses.py "$OUT/THIRD_PARTY_LICENSES.txt"

echo "staged in $OUT:"
ls -l "$OUT"
if command -v lipo >/dev/null && [[ -f "$OUT/libbridge.dylib" ]]; then
    lipo -info "$OUT/libbridge.dylib" "$OUT/libsteam_api.dylib"
fi
