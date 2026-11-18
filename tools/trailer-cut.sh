#!/usr/bin/env bash
# Cuts recorded trailer shots into one video (prompt 082).
#
#   tools/trailer-cut.sh <prefix> [out.mp4]
#
# Takes every shot of godot/scenes/tools/trailer_shots.gd whose name starts
# with <prefix>_ in the order of the table (the order of the edit), as
# recorded by tools/trailer-shots.sh (build/trailer/<shot>.avi), and joins
# them with hard cuts into build/trailer/<prefix>_final.mp4:
#
# - picture: 1920 x 1080, 60 fps, H.264 (yuv420p, video range: the
#   recordings are full-range MJPEG), no re-timing: every shot keeps its
#   frames;
# - sound: the game's own, faded out and in over 1.5 frames each at every
#   cut (3 frames in all, nothing overlaps, so picture and sound stay in
#   step and no cut clicks), then set to -14 LUFS integrated loudness
#   (ffmpeg's loudnorm with a measuring pass, then the measured rest as a
#   plain gain; peaks limited to -1.5 dB), AAC.
#
# One call rebuilds the video; nothing is done by hand. Needs ffmpeg,
# ffprobe and python3 (to read loudnorm's measurement).
set -euo pipefail
cd "$(dirname "$0")/.."

prefix="${1:?usage: tools/trailer-cut.sh <prefix> [out.mp4]}"
dir="$PWD/build/trailer"
out="${2:-$dir/${prefix}_final.mp4}"
script=godot/scenes/tools/trailer_shots.gd
fade=0.025
loudness=-14
limits="TP=-1.5:LRA=11"

# SHOTS="a b c" cuts just these, in this order (for trying a cut).
shots="${SHOTS:-$(sed -n 's/^	"\('"$prefix"'_[a-z0-9_]*\)": {.*/\1/p' "$script")}"
[ -n "$shots" ] || { echo "no shots named ${prefix}_* in $script" >&2; exit 1; }

inputs=()
vgraph=""
agraph=""
vlabels=""
audio=""
n=0
total=0
for shot in $shots; do
    file="$dir/$shot.avi"
    [ -f "$file" ] || { echo "$file is missing: just trailer-shots --only=$shot" >&2; exit 1; }
    frames=$(ffprobe -v error -select_streams v:0 -show_entries stream=nb_frames -of csv=p=0 "$file")
    seconds=$(awk "BEGIN { printf \"%.6f\", $frames / 60 }")
    fade_out=$(awk "BEGIN { printf \"%.6f\", $frames / 60 - $fade }")
    inputs+=(-i "$file")
    vgraph+="[$n:v]setpts=PTS-STARTPTS,fps=60,scale=in_range=full:out_range=tv,format=yuv420p[v$n];"
    agraph+="[$n:a]atrim=0:$seconds,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=$fade,afade=t=out:st=$fade_out:d=$fade[a$n];"
    vlabels+="[v$n]"
    audio+="[a$n]"
    total=$(awk "BEGIN { printf \"%.3f\", $total + $frames / 60 }")
    printf '  %-24s %6.2f s\n' "$shot" "$seconds"
    n=$((n + 1))
done
echo "trailer-cut: $n shots, $total s"

# The sound: joined, measured, set to the target loudness into a WAV file.
# With a wide loudness range (explosions over engines) loudnorm works
# dynamically and lands a little beside the target (-14.4 for -14), so the
# result is measured and the difference added as a plain gain.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
joined="${agraph}${audio}concat=n=$n:v=0:a=1"
measure=$(ffmpeg -hide_banner -nostdin "${inputs[@]}" \
    -filter_complex "$joined,loudnorm=I=$loudness:$limits:print_format=json[m]" \
    -map "[m]" -vn -f null - 2>&1 | python3 -c '
import json, re, sys
text = sys.stdin.read()
found = re.findall(r"\{[^{}]*\"input_i\"[^{}]*\}", text)
if not found:
    sys.exit("loudnorm printed no measurement:\n" + text[-2000:])
m = json.loads(found[-1])
print("measured_I=%s:measured_TP=%s:measured_LRA=%s:measured_thresh=%s:offset=%s" % (
    m["input_i"], m["input_tp"], m["input_lra"], m["input_thresh"], m["target_offset"]))
')
echo "trailer-cut: loudness before: $measure"
ffmpeg -hide_banner -nostdin -v error -y "${inputs[@]}" \
    -filter_complex "$joined,loudnorm=I=$loudness:$limits:$measure:linear=true,aresample=48000[a]" \
    -map "[a]" -vn -c:a pcm_s24le "$tmp/normalized.wav"
integrated() {
    ffmpeg -hide_banner -nostdin -i "$1" -af ebur128 -f null - 2>&1 \
        | awk '/Integrated loudness:/ { found = 1 } found && /I:/ { print $2; exit }'
}
got=$(integrated "$tmp/normalized.wav")
gain=$(awk "BEGIN { g = ($loudness) - ($got); if (g > 1.5) g = 1.5; if (g < -1.5) g = -1.5; printf \"%.2f\", g }")
echo "trailer-cut: loudness after loudnorm: $got LUFS, correction $gain dB"
# The rest is a plain gain; a limiter keeps the peaks at -1.5 dB.
ffmpeg -hide_banner -nostdin -v error -y -i "$tmp/normalized.wav" \
    -af "volume=${gain}dB,alimiter=limit=0.841:level=disabled:attack=1:release=50,aresample=48000" \
    -c:a pcm_s24le "$tmp/sound.wav"
echo "trailer-cut: loudness: $(integrated "$tmp/sound.wav") LUFS"

# The picture, joined and encoded with that sound.
mkdir -p "$(dirname "$out")"
ffmpeg -hide_banner -nostdin -v error -y "${inputs[@]}" -i "$tmp/sound.wav" \
    -filter_complex "${vgraph}${vlabels}concat=n=$n:v=1:a=0[v]" \
    -map "[v]" -map "$n:a" \
    -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -color_range tv -r 60 -profile:v high \
    -c:a aac -b:a 256k -ar 48000 -movflags +faststart "$out"

ffprobe -v error -show_entries format=duration,size -show_entries stream=codec_name,width,height,pix_fmt,r_frame_rate \
    -of compact "$out"
echo "trailer-cut: wrote $out"
