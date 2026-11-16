#!/usr/bin/env bash
# Records the trailer shots of godot/scenes/tools/trailer_shots.gd (prompt 070).
#
#   tools/trailer-shots.sh [--stills] [--only=<shot>,<shot>…]
#
# Per shot: Godot's Movie Maker writes build/trailer/raw/<shot>.avi (1920 x 1080,
# 60 fps, picture and sound) including the run-up to the shot's start; ffmpeg
# then cuts the run-up off without re-encoding: build/trailer/<shot>.avi.
# With --stills it saves the shot's still pictures to build/trailer/stills/
# instead. Needs a window, the built bridge (`just build`) and, for movies,
# ffmpeg. The planning shots `survey_*` are only taken when named.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
script=godot/scenes/tools/trailer_shots.gd
out="$PWD/build/trailer"
stills=0
only=""
for arg in "$@"; do
    case "$arg" in
        --stills) stills=1 ;;
        --only=*) only="${arg#--only=}" ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

if [ -n "$only" ]; then
    shots="${only//,/ }"
else
    shots=$(sed -n 's/^	"\([a-z0-9_]*\)": {.*/\1/p' "$script" | grep -v '^survey_')
fi

mkdir -p "$out/raw" "$out/stills"
for shot in $shots; do
    if [ "$stills" = 1 ]; then
        "$godot" --path godot --always-on-top --disable-vsync --fixed-fps 60 res://scenes/main.tscn -- \
            "--trailer=$shot" "--trailer-stills=$out/stills" | grep '^trailer:' || true
        continue
    fi
    command -v ffmpeg >/dev/null || { echo "ffmpeg is needed to cut the recordings" >&2; exit 1; }
    log="$out/raw/$shot.log"
    "$godot" --path godot --always-on-top --fixed-fps 60 --write-movie "$out/raw/$shot.avi" \
        res://scenes/main.tscn -- "--trailer=$shot" >"$log" 2>&1 || { tail -n 20 "$log" >&2; exit 1; }
    line=$(grep '^trailer: record ' "$log") || { echo "$shot: the shot never started, see $log" >&2; exit 1; }
    start=$(sed -n 's/.*start_frame=\([0-9]*\).*/\1/p' <<<"$line")
    frames=$(sed -n 's/.* frames=\([0-9]*\).*/\1/p' <<<"$line")
    # Frame-exact cut: MJPEG and PCM have no dependencies between frames.
    ffmpeg -v error -y -ss "$(awk "BEGIN { printf \"%.6f\", $start / 60 }")" -i "$out/raw/$shot.avi" \
        -frames:v "$frames" -t "$(awk "BEGIN { printf \"%.6f\", $frames / 60 }")" -c copy "$out/$shot.avi"
    rm "$out/raw/$shot.avi"
    echo "trailer: $shot -> build/trailer/$shot.avi ($frames frames)"
done
