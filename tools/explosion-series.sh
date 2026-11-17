#!/usr/bin/env bash
# Explosion series, game against the reference video (prompt 072).
#
#   tools/explosion-series.sh [out_dir]       (default build/explosion-ref/now)
#
# Runs blast_shots.gd with --explosion-series (every explosion kind once,
# 30 fps fixed), then builds per kind a tile sheet of the game's pictures
# (tools/explosion_series.py) and, if the reference video is there, puts the
# video's sheet of the matching explosion above it: <out_dir>/compare_<kind>.png.
# The video (.temp/rts-explosionen.mp4, not in the repository) is a reference
# for timing and look only; its pictures stay in build/.
# Needs a window, the built bridge (`just build`), ffmpeg and numpy.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
out="${1:-build/explosion-ref/now}"
video=".temp/rts-explosionen.mp4"
mkdir -p "$out/frames"
out="$(cd "$out" && pwd)"

# Per game kind: the video explosion it is compared with (start s, crop
# x,y,w,h in pixels of the 2160 x 3840 frame, seconds until the next cut).
# medium: the 6.2 s impact is bigger than a 9 cm shell; the game crop is
# scaled by the caliber instead (blast_shots.gd SERIES_KINDS). ammo: the
# video has none; compared with the vehicle explosion.
ref() {
    case "$1" in
        small) echo "29.450 850,1050,1000,1000 1.45" ;;
        medium) echo "6.233 580,900,1400,1400 4.0" ;;
        big) echo "19.633 500,700,1600,1600 1.75" ;;
        vehicle | ammo) echo "6.933 0,1000,1400,1400 3.4" ;;
    esac
}

rm -f "$out"/frames/*.png
"$godot" --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- \
    "--blast-shots=$out/frames" --explosion-series | grep '^explosion-series' || true

while read -r kind crop; do
    python3 -I tools/explosion_series.py game "$out/frames" "$kind" "$crop" 10.0 "$out/game_$kind.png" </dev/null
    if [ -f "$video" ]; then
        read -r start vcrop until <<<"$(ref "$kind")"
        [ -f "$out/../video_$kind.png" ] ||
            python3 -I tools/explosion_series.py video "$video" "$start" "$vcrop" "$until" "$out/../video_$kind.png" </dev/null
        python3 -I tools/explosion_series.py stack "$out/compare_$kind.png" "$out/../video_$kind.png" "$out/game_$kind.png" </dev/null
    fi
    echo "explosion-series: $out/compare_$kind.png"
done <"$out/frames/series.txt"
