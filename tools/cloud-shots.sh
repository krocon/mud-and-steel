#!/usr/bin/env bash
# Cloud shadows in pictures (prompt 087).
#
#   tools/cloud-shots.sh [<prefix>] [--off]
#
# Saves the stills of the trailer shots `survey_clouds_*` (meadow, wood's
# edge with a column, a village of test_large; clear by day, overcast,
# evening, and a series every 2 s at a fixed camera) as
# build/cloud_shots/<prefix>_<shot>_<second>.png. --off takes them without
# cloud shadows (the "before" pictures). Needs a window and the built bridge.
set -euo pipefail
cd "$(dirname "$0")/.."
godot="${GODOT:-godot}"
prefix="${1:-after}"
off=""
[ "${2:-}" = "--off" ] && off="--trailer-off=clouds"
out="$PWD/build/cloud_shots"
tmp="$out/raw"
mkdir -p "$out"
for shot in meadow near far overcast evening series; do
    rm -rf "$tmp" && mkdir -p "$tmp"
    "$godot" --path godot --always-on-top --disable-vsync --fixed-fps 60 res://scenes/main.tscn -- \
        "--trailer=survey_clouds_$shot" "--trailer-stills=$tmp" $off | grep '^trailer: still' || true
    for f in "$tmp"/*.png; do
        [ -e "$f" ] || continue
        name=$(basename "$f")
        mv "$f" "$out/${prefix}_${name#survey_clouds_}"
    done
done
rm -rf "$tmp"
