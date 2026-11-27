#!/usr/bin/env bash
# Exports the game for one system (prompt 044, docs/RELEASE.md).
#
#   tools/release/export.sh macos|windows|linux   release export + archive
#   tools/release/export.sh pack                  only the .pck (no export
#                                                 templates needed), for
#                                                 tools/release/smoke-test.sh pack
#
# Needs: Godot 4.7.2 ($GODOT, default `godot`), its export templates for the
# release exports (not for `pack`), and tools/release/build-lib.sh run before
# on the target system (the libraries in target/export/; CI collects those of
# all three systems there). Writes build/export/<system>/ (the content
# SteamPipe uploads) and build/release/MudAndSteel-<version>-<system>.*.
#
# The macOS app is signed by tools/release/macos-sign.sh after data/ was
# added: ad-hoc, or with a Developer ID from the environment.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
GODOT="${GODOT:-godot}"
SYSTEM="${1:-}"
VERSION="$(bash tools/release/version.sh)"
OUT="$ROOT/build/export/$SYSTEM"

case "$SYSTEM" in
macos) PRESET="macOS"; EXE="$OUT/Mud and Steel.app"; LIBS=(libbridge.dylib libsteam_api.dylib) ;;
windows) PRESET="Windows"; EXE="$OUT/MudAndSteel.exe"; LIBS=(bridge.dll steam_api64.dll) ;;
linux) PRESET="Linux"; EXE="$OUT/MudAndSteel.x86_64"; LIBS=(libbridge.so libsteam_api.so) ;;
pack) PRESET="macOS"; EXE="$OUT/MudAndSteel.pck"; LIBS=() ;;
*)
    echo "usage: $0 macos|windows|linux|pack" >&2
    exit 2
    ;;
esac

for lib in ${LIBS[@]+"${LIBS[@]}"}; do
    if [[ ! -f "target/export/$lib" ]]; then
        echo "error: target/export/$lib missing: run tools/release/build-lib.sh on $SYSTEM first" >&2
        exit 1
    fi
done

if command -v python3 >/dev/null; then PY=python3; else PY=python; fi
if [[ ! -f target/export/THIRD_PARTY_LICENSES.txt ]]; then
    "$PY" tools/release/third_party_licenses.py target/export/THIRD_PARTY_LICENSES.txt
fi

# License files into the pack (Credits menu reads res://licenses/).
rm -rf godot/licenses
mkdir -p godot/licenses
cp CREDITS.md LICENSE.txt target/export/THIRD_PARTY_LICENSES.txt godot/licenses/
trap 'rm -rf "$ROOT/godot/licenses"' EXIT

if [[ ! -d godot/.godot ]]; then
    bash tools/godot-import.sh
fi

# A release ships the private sounds (voice lines and recordings, prepared by
# `just audio-private`); ALLOW_PUBLIC_SOUNDS=1 exports without them.
if [[ "$SYSTEM" != pack && "${ALLOW_PUBLIC_SOUNDS:-0}" != 1 ]]; then
    "$PY" -I tools/release/check_private_sounds.py
fi

rm -rf "$OUT"
mkdir -p "$OUT"
echo "exporting $PRESET $VERSION to $EXE"
if [[ "$SYSTEM" == pack ]]; then
    "$GODOT" --headless --path godot --export-pack "$PRESET" "$EXE"
else
    "$GODOT" --headless --path godot --export-release "$PRESET" "$EXE"
fi
if [[ ! -e "$EXE" ]]; then
    echo "error: Godot wrote no $EXE (export templates of Godot 4.7.2 installed? see docs/RELEASE.md)" >&2
    exit 1
fi
[[ "$SYSTEM" == pack ]] && exit 0

# Maps and scenarios are read from disk by the simulation (see
# godot/scenes/menu/game_paths.gd), plus the license files for the player.
if [[ "$SYSTEM" == macos ]]; then
    DEST="$EXE/Contents/Resources"
else
    DEST="$OUT"
fi
mkdir -p "$DEST/data"
cp -R data/maps data/scenarios data/campaigns "$DEST/data/"
rm -rf "$DEST/data/maps/editor_selftest"
cp CREDITS.md LICENSE.txt target/export/THIRD_PARTY_LICENSES.txt "$DEST/"

mkdir -p build/release
NAME="MudAndSteel-$VERSION-$SYSTEM"
case "$SYSTEM" in
macos)
    bash tools/release/macos-sign.sh "$EXE"
    rm -f "build/release/$NAME.zip"
    ditto -c -k --sequesterRsrc --keepParent "$EXE" "build/release/$NAME.zip"
    ARCHIVE="build/release/$NAME.zip"
    ;;
linux)
    chmod +x "$EXE"
    tar -C "$OUT" -czf "build/release/$NAME.tar.gz" .
    ARCHIVE="build/release/$NAME.tar.gz"
    ;;
windows)
    rm -f "build/release/$NAME.zip"
    (cd "$OUT" && "$PY" -m zipfile -c "$ROOT/build/release/$NAME.zip" *)
    ARCHIVE="build/release/$NAME.zip"
    ;;
esac
echo "archive: $ARCHIVE ($(du -h "$ARCHIVE" | cut -f1))"
