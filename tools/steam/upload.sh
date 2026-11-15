#!/usr/bin/env bash
# SteamPipe build of Mud and Steel (prompt 044, docs/RELEASE.md "Steam").
#
#   tools/steam/upload.sh            writes build/steam/*.vdf and prints the
#                                    steamcmd command (dry run, the default)
#   tools/steam/upload.sh --upload   also runs steamcmd: uploads the build
#
# Only the user runs this, by hand: CI never uploads. Needs the three exports
# in build/export/{windows,macos,linux}/ (from the release workflow's
# artifacts or tools/release/export.sh) and, for --upload, steamcmd and a
# Steam account with upload rights (it asks for the password and Steam Guard
# code itself; nothing is stored here).
#
# Settings from the environment (Steamworks partner site > App Admin):
#   STEAM_APP_ID        the game's app id (not 480, Valve's test app)
#   STEAM_DEPOT_WINDOWS, STEAM_DEPOT_MACOS, STEAM_DEPOT_LINUX   depot ids
#   STEAM_BRANCH        branch to set live after the upload, e.g. "beta"
#                       (a private branch with a password). Empty (default):
#                       nothing is set live; do it in the partner site.
#                       Steam never lets a tool set the default branch live.
#   STEAM_USER          Steam account for --upload
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/build/steam"
UPLOAD=0
[[ "${1:-}" == "--upload" ]] && UPLOAD=1

missing=0
for var in STEAM_APP_ID STEAM_DEPOT_WINDOWS STEAM_DEPOT_MACOS STEAM_DEPOT_LINUX; do
    if [[ -z "${!var:-}" ]]; then
        echo "error: $var is not set (see docs/RELEASE.md)" >&2
        missing=1
    fi
done
[[ $missing == 0 ]] || exit 1
if [[ "$STEAM_APP_ID" == 480 ]]; then
    echo "error: 480 is Valve's test app; use the game's own app id" >&2
    exit 1
fi
for sys in windows macos linux; do
    if [[ -z "$(ls -A "$ROOT/build/export/$sys" 2>/dev/null)" ]]; then
        echo "error: build/export/$sys/ is empty: export the game first" >&2
        exit 1
    fi
done
grep -q "APP_ID: u32 = $STEAM_APP_ID;" "$ROOT/crates/steam/src/lib.rs" ||
    echo "warning: steam::APP_ID in crates/steam/src/lib.rs is not $STEAM_APP_ID" >&2

VERSION="$(bash "$ROOT/tools/release/version.sh")"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
mkdir -p "$OUT/output"
for file in app_build depot_build_windows depot_build_macos depot_build_linux; do
    sed -e "s|@APP_ID@|$STEAM_APP_ID|" -e "s|@VERSION@|$VERSION|" -e "s|@COMMIT@|$COMMIT|" \
        -e "s|@ROOT@|$ROOT|" -e "s|@BRANCH@|${STEAM_BRANCH:-}|" \
        -e "s|@DEPOT_WINDOWS@|$STEAM_DEPOT_WINDOWS|" -e "s|@DEPOT_MACOS@|$STEAM_DEPOT_MACOS|" \
        -e "s|@DEPOT_LINUX@|$STEAM_DEPOT_LINUX|" \
        "$ROOT/tools/steam/$file.vdf.in" > "$OUT/$file.vdf"
done
CMD=(steamcmd +login "${STEAM_USER:-<steam account>}" +run_app_build "$OUT/app_build.vdf" +quit)
echo "wrote $OUT/app_build.vdf and the depot scripts (version $VERSION, $COMMIT, branch '${STEAM_BRANCH:-}')"
if [[ $UPLOAD == 0 ]]; then
    echo "dry run; to upload: tools/steam/upload.sh --upload   (runs: ${CMD[*]})"
    exit 0
fi
if [[ -z "${STEAM_USER:-}" ]]; then
    echo "error: STEAM_USER is not set" >&2
    exit 1
fi
"${CMD[@]}"
