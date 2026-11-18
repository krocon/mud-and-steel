#!/usr/bin/env bash
# Scouts a trailer scene (prompt 082): plays the scene of a shot of
# godot/scenes/tools/trailer_shots.gd headless and without a camera and
# prints who fires, hits and is destroyed when and where.
#
#   tools/trailer-scout.sh <shot> <seconds> [seed] [seed] ...
#
# Without seeds: the shot's own seed, the whole log. With seeds: the scene
# once per seed, each log in build/trailer/scout/<shot>-<seed>.log, and one
# line per seed with the destroyed vehicles in order, and what is left.
set -euo pipefail
cd "$(dirname "$0")/.."
godot="${GODOT:-godot}"
shot="$1"
seconds="$2"
shift 2
out="$PWD/build/trailer/scout"
mkdir -p "$out"
run() { # run <log> [seed]
    "$godot" --path godot --headless --fixed-fps 60 res://scenes/main.tscn -- \
        "--trailer=$shot" "--trailer-scout=$seconds" ${2:+"--trailer-seed=$2"} >"$1" 2>&1 || true
}
if [ $# -eq 0 ]; then
    run "$out/$shot.log"
    grep '^trailer-scout\|^    ' "$out/$shot.log"
    exit 0
fi
for seed in "$@"; do
    log="$out/$shot-$seed.log"
    run "$log" "$seed"
    printf 'seed %3s: ' "$seed"
    grep 'destroyed\|turret\|first shot\|hit 1 de_tiger.*penetrated' "$log" | grep -v '_inf_\| s  {' \
        | sed -E 's/trailer-scout: +([0-9.]+) s /\1 /; s/ at \(.*//; s/destroyed //; s/ammunition explosion, turret [0-9]+ blown off/TURRET/; s/first shot by/first:/' \
        | tr '\n' ';'
    echo
    # The last report: who is left of every kind.
    awk '/^trailer-scout:.* s  \{/ { last = ""; next } /^    / { last = last $0 "\n" } END { printf "%s", last }' "$log" \
        | sed -E 's/\[([^]]*)\]/\1/; s/\([-0-9]+, [-0-9]+\) //g' | cut -c1-150
done
