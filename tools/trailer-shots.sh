#!/usr/bin/env bash
# Records the trailer shots of godot/scenes/tools/trailer_shots.gd (prompts 070, 238).
#
#   tools/trailer-shots.sh [--stills] [--only=<shot>,<shot>…] [--size=WxH]
#
# Per shot: Godot's Movie Maker writes build/trailer/raw/<shot>.avi (default
# 1920 x 1080 or per-shot portrait 1080 x 1920, 60 fps, picture and sound)
# including the run-up to the shot's start; ffmpeg then cuts the run-up off
# without re-encoding: build/trailer/<shot>.avi.
# With --stills it saves the shot's still pictures to build/trailer/stills/
# instead. Needs a window, the built bridge (`just build`) and, for movies,
# ffmpeg. The planning shots `survey_*` are only taken when named.
set -euo pipefail
cd "$(dirname "$0")/.."

godot="${GODOT:-godot}"
script=godot/scenes/tools/trailer_shots.gd
out="$PWD/build/trailer"
override_cfg="godot/override.cfg"
had_override=0
[ -f "$override_cfg" ] && had_override=1

cleanup_override() {
    if [ "$had_override" = 0 ] && [ -f "$override_cfg" ]; then
        rm -f "$override_cfg"
    fi
}
trap cleanup_override EXIT

stills=0
only=""
size_override=""
for arg in "$@"; do
    case "$arg" in
        --stills) stills=1 ;;
        --only=*) only="${arg#--only=}" ;;
        --size=*) size_override="${arg#--size=}" ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

if [ -n "$only" ]; then
    shots="${only//,/ }"
else
    shots=$(sed -n 's/^	"\([a-z0-9_]*\)": {.*/\1/p' "$script" | grep -v '^survey_')
fi

shot_resolution() {
    local name="$1"
    if [ -n "$size_override" ]; then
        echo "$size_override"
        return
    fi
    python3 -I - "$script" "$name" <<'PY'
import pathlib, re, sys
text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
name = re.escape(sys.argv[2])
m = re.search(rf'^\t"{name}": \{{(.*?)^\t\}},', text, re.S | re.M)
if not m:
    sys.exit(0)
body = m.group(1)
sm = re.search(r'"size"\s*:\s*Vector2i\(\s*(\d+)\s*,\s*(\d+)\s*\)', body)
if sm:
    print(f"{sm.group(1)}x{sm.group(2)}")
elif re.search(r'"aspect"\s*:\s*"9:16"', body) or re.search(r'"portrait"\s*:\s*true', body):
    print("1080x1920")
PY
}

mkdir -p "$out/raw" "$out/stills"
for shot in $shots; do
    res=$(shot_resolution "$shot")
    res_args=()
    cleanup_override
    if [ -n "$res" ] && [ "$res" != "1920x1080" ]; then
        w="${res%x*}"
        h="${res#*x}"
        res_args=(--resolution "$res")
        # Godot's MovieWriter locks AVI dimensions to viewport_{width,height}.movie
        # from project settings before applying --resolution; override.cfg sets
        # the per-shot portrait/custom size cleanly without touching project.godot.
        if [ "$stills" = 0 ] && [ "$had_override" = 0 ]; then
            cat >"$override_cfg" <<EOF
[display]
window/size/viewport_width.movie=$w
window/size/viewport_height.movie=$h
EOF
        fi
    fi
    if [ "$stills" = 1 ]; then
        "$godot" --path godot --always-on-top --disable-vsync --fixed-fps 60 "${res_args[@]}" \
            res://scenes/main.tscn -- "--trailer=$shot" "--trailer-stills=$out/stills" \
            | grep '^trailer:' || true
        continue
    fi
    command -v ffmpeg >/dev/null || { echo "ffmpeg is needed to cut the recordings" >&2; exit 1; }
    log="$out/raw/$shot.log"
    "$godot" --path godot --always-on-top --fixed-fps 60 "${res_args[@]}" \
        --write-movie "$out/raw/$shot.avi" \
        res://scenes/main.tscn -- "--trailer=$shot" >"$log" 2>&1 || { tail -n 20 "$log" >&2; exit 1; }
    cleanup_override
    line=$(grep '^trailer: record ' "$log") || { echo "$shot: the shot never started, see $log" >&2; exit 1; }
    start=$(sed -n 's/.*start_frame=\([0-9]*\).*/\1/p' <<<"$line")
    frames=$(sed -n 's/.* frames=\([0-9]*\).*/\1/p' <<<"$line")
    # Frame-exact cut: MJPEG and PCM have no dependencies between frames.
    ffmpeg -v error -y -ss "$(awk "BEGIN { printf \"%.6f\", $start / 60 }")" -i "$out/raw/$shot.avi" \
        -frames:v "$frames" -t "$(awk "BEGIN { printf \"%.6f\", $frames / 60 }")" -c copy "$out/$shot.avi"
    rm "$out/raw/$shot.avi"
    echo "trailer: $shot -> build/trailer/$shot.avi ($frames frames${res:+, $res})"
done
