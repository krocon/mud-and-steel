#!/usr/bin/env bash
# Does the ground texture stick to the ground while the camera moves? (prompt 071)
#
#   tools/ground-motion.sh [<dir>] [--png]
#
# Records the trailer shots `column_high` (farms, near the map centre) and
# `survey_ground` (test_large, near a corner, where a texture that depends on
# the camera slides the most) into <dir> (default build/ground-motion) and
# measures fixed windows of pure ground with tools/ground_flicker.py:
# "slide" (fine texture against the ground, px/frame, 0 = sticks) and the
# frame-to-frame match per wavelength band (1 = stable).
# --png writes lossless PNG frames instead of MJPEG (for the 2-4 px band,
# which MJPEG noise dominates); slower and about 6 MB per frame.
# Needs a window, the built bridge, ffmpeg and python3 with numpy.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
out="build/ground-motion"
png=0
for arg in "$@"; do
    case "$arg" in
        --png) png=1 ;;
        -*) echo "unknown argument: $arg" >&2; exit 2 ;;
        *) out="$arg" ;;
    esac
done
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# shot | first frame of the shot to measure | frames | windows (w x h + x + y), name=geometry
measure=(
    "column_high|120|30|grass=320x200+1300+700 road=128x128+940+620"
    "survey_ground|150|30|grass=320x200+1000+420"
)

for entry in "${measure[@]}"; do
    IFS='|' read -r shot first count windows <<<"$entry"
    log="$out/$shot.log"
    if [ "$png" = 1 ]; then
        mkdir -p "$out/$shot"
        rm -f "$out/$shot"/*.png
        target="$out/$shot/f.png"
    else
        target="$out/$shot.avi"
    fi
    "$godot" --path godot --always-on-top --fixed-fps 60 --write-movie "$target" \
        res://scenes/main.tscn -- "--trailer=$shot" >"$log" 2>&1 || { tail -n 20 "$log" >&2; exit 1; }
    start=$(sed -n 's/^trailer: record .*start_frame=\([0-9]*\).*/\1/p' "$log")
    [ -n "$start" ] || { echo "$shot: the shot never started, see $log" >&2; exit 1; }
    if [ "$png" = 1 ]; then
        # Movie Maker numbers PNG frames from 0 (f00000000.png).
        src="$out/$shot/f%08d.png"
    else
        src="$target"
    fi
    for win in $windows; do
        name="${win%%=*}"
        geo="${win#*=}"
        read -r w h x y <<<"$(sed 's/[x+]/ /g' <<<"$geo")"
        echo "== $shot $name"
        python3 tools/ground_flicker.py "$src" $((start + first)) "$count" "$x" "$y" "$w" "$h" | tail -n +2
    done
    if [ "$png" = 1 ]; then
        # Keep only the measured frames (all of them would be several GB).
        for f in "$out/$shot"/f*.png; do
            n=$((10#$(basename "$f" .png | tr -dc 0-9)))
            if [ "$n" -lt $((start + first)) ] || [ "$n" -ge $((start + first + count)) ]; then
                rm "$f"
            fi
        done
    fi
done
