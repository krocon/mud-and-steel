#!/usr/bin/env bash
# Flicker a zoom adds while the camera stands (prompt 090).
#
#   tools/shadow-flicker.sh [--off=<part,...>]
#
# Records the trailer shot `zoom_shadow_range` twice into
# build/shadow-flicker/: with the shadow distance in steps and gliding with
# the zoom (`--trailer-off=shadowsteps`), depth of field off both times, and
# measures both with tools/shadow_flicker.py. Further `--off=` parts apply
# to both runs (e.g. `--off=ssao`).
# Needs a window, the built bridge, ffmpeg and python3 with numpy.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
extra=""
for arg in "$@"; do
    case "$arg" in
        --off=*) extra=",${arg#--off=}" ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done
out="build/shadow-flicker"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
for run in steps glide; do
    off="dof$extra"
    [ "$run" = glide ] && off="$off,shadowsteps"
    log="$out/$run.log"
    # --quit-after: a shot that never starts must not leave its window open.
    "$godot" --path godot --always-on-top --fixed-fps 60 --quit-after 1800 --write-movie "$out/$run.avi" \
        res://scenes/main.tscn -- --trailer=zoom_shadow_range "--trailer-off=$off" >"$log" 2>&1 || { tail -n 20 "$log" >&2; exit 1; }
    start=$(sed -n 's/^trailer: record .*start_frame=\([0-9]*\).*/\1/p' "$log")
    [ -n "$start" ] || { echo "$run: the shot never started, see $log" >&2; exit 1; }
    echo "== $run"
    python3 tools/shadow_flicker.py "$out/$run.avi" "$start" 240 | tail -n +2
    rm "$out/$run.avi"
done
