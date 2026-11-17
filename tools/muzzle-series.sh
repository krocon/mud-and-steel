#!/usr/bin/env bash
# Muzzle blast series (prompt 077).
#
#   tools/muzzle-series.sh [out_dir] [--only=<run>,<run>]   (default build/muzzle/now)
#
# Runs effects_bench.gd with --muzzle-series (30 fps fixed): every gun class,
# distance, daytime and biome of MUZZLE_RUNS fires once; every frame of the
# first half second, then four pictures a second to 4 s. Builds per run a
# tile sheet around the muzzle (tools/explosion_series.py):
# <out_dir>/sheet_<run>.png. If the reference picture is there
# (.temp/blitz-aus-rohr.png, not in the repository), it goes beside the
# game's reference view at 33, 67, 100 and 200 ms:
# <out_dir>/compare_<ms>.png. The reference is for effect and size only;
# the comparison pictures stay in build/.
# Needs a window, the built bridge (`just build`), ffmpeg and numpy.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
out="build/muzzle/now"
only=()
for arg in "$@"; do
    case "$arg" in
        --only=*) only=("$arg") ;;
        *) out="$arg" ;;
    esac
done
reference=".temp/blitz-aus-rohr.png"
mkdir -p "$out/frames"
out="$(cd "$out" && pwd)"

rm -f "$out"/frames/*.png "$out"/frames/series.txt
"$godot" --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- \
    "--muzzle-series=$out/frames" ${only[@]+"${only[@]}"} | grep '^muzzle-series' || true

while read -r run crop; do
    python3 -I tools/explosion_series.py game "$out/frames" "$run" "$crop" 4.0 "$out/sheet_$run.png" 0.5 </dev/null
    echo "muzzle-series: $out/sheet_$run.png"
done <"$out/frames/series.txt"

if [ -f "$reference" ] && [ -f "$out/frames/reference_00033.png" ]; then
    for ms in 00033 00067 00100 00200; do
        ffmpeg -v error -y -i "$reference" -i "$out/frames/reference_$ms.png" -filter_complex \
            "[0:v]scale=-2:450[a];[1:v]scale=-2:450[b];[a][b]hstack=inputs=2" "$out/compare_$ms.png" </dev/null
        echo "muzzle-series: $out/compare_$ms.png"
    done
fi
