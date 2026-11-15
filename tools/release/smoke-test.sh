#!/usr/bin/env bash
# Smoke test of an exported game (prompt 044): starts it headless and plays
# the menu flow self-test (menu, settings, credits, lobbies, four matches,
# result; the same as `just selftest`). Exit code 0 = passed.
#
#   tools/release/smoke-test.sh macos|windows|linux [dir]
#       runs build/export/<system>/ (or the export unpacked in dir)
#   tools/release/smoke-test.sh pack
#       runs build/export/pack/MudAndSteel.pck with the Godot editor binary
#       ($GODOT) and the debug bridge: checks that the export contains every
#       resource, without export templates
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SYSTEM="${1:?usage: $0 macos|windows|linux|pack [dir]}"
DIR="${2:-$ROOT/build/export/$SYSTEM}"
ARGS=(--headless --fixed-fps 60 -- --self-test)

case "$SYSTEM" in
macos) EXE="$DIR/Mud and Steel.app/Contents/MacOS/Mud and Steel" ;;
windows) EXE="$DIR/MudAndSteel.exe" ;;
linux) EXE="$DIR/MudAndSteel.x86_64" ;;
pack)
    # The pack finds the debug bridge at res://../target/debug, relative to
    # the working directory, as when running from the repository.
    cd "$ROOT/godot"
    exec "${GODOT:-godot}" --main-pack "$DIR/MudAndSteel.pck" "${ARGS[@]}"
    ;;
*)
    echo "unknown system $SYSTEM" >&2
    exit 2
    ;;
esac

if [[ ! -f "$EXE" ]]; then
    echo "error: $EXE not found" >&2
    exit 1
fi
# Start from elsewhere: the game must not depend on the working directory.
cd "${TMPDIR:-/tmp}"
"$EXE" "${ARGS[@]}"
