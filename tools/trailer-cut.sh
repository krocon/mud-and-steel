#!/usr/bin/env bash
# Cuts recorded trailer shots into one video (prompts 082, 238).
#
#   tools/trailer-cut.sh <prefix> [out.mp4]
#
# Takes every shot of godot/scenes/tools/trailer_shots.gd whose name starts
# with <prefix>_ in the order of the table (the order of the edit), as
# recorded by tools/trailer-shots.sh (build/trailer/<shot>.avi), and joins
# them with hard cuts into build/trailer/<prefix>_final.mp4:
#
# - picture: native resolution of the recorded shots (e.g. 1920 x 1080 or
#   portrait 1080 x 1920, optionally scaled via SCALE=WxH), 60 fps, H.264
#   (yuv420p, video range: the recordings are full-range MJPEG), no re-timing;
# - sound: the game's own, faded out and in over 1.5 frames each at every
#   cut (3 frames in all, nothing overlaps, so picture and sound stay in
#   step and no cut clicks), then set to -14 LUFS integrated loudness
#   (ffmpeg's loudnorm with a measuring pass, then the measured rest as a
#   plain gain; peaks limited to -1.5 dB), AAC;
# - optional Shorts extras (default off):
#     SCALE=1440x2560           upscale/resize with lanczos
#     CAPTIONS=file.toml|.ass|.srt  burn in hook captions (TOML via shorts_captions.py)
#     VOICEOVER=file.wav|.mp3   mix local commentary with gentle game-audio ducking
#     VOICEOVER_START_S=0.0     delay of voiceover in seconds
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
expected_res=""
for shot in $shots; do
    file="$dir/$shot.avi"
    [ -f "$file" ] || { echo "$file is missing: just trailer-shots --only=$shot" >&2; exit 1; }
    info=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height,nb_frames -of csv=p=0 "$file")
    IFS=, read -r w h frames <<<"$info"
    res="${w}x${h}"
    if [ -z "$expected_res" ]; then
        expected_res="$res"
    elif [ "$res" != "$expected_res" ]; then
        echo "trailer-cut: resolution mismatch in $shot ($res != $expected_res)" >&2
        exit 1
    fi
    seconds=$(awk "BEGIN { printf \"%.6f\", $frames / 60 }")
    fade_out=$(awk "BEGIN { printf \"%.6f\", $frames / 60 - $fade }")
    inputs+=(-i "$file")
    vgraph+="[$n:v]setpts=PTS-STARTPTS,fps=60,scale=in_range=full:out_range=tv,format=yuv420p[v$n];"
    agraph+="[$n:a]atrim=0:$seconds,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=$fade,afade=t=out:st=$fade_out:d=$fade[a$n];"
    vlabels+="[v$n]"
    audio+="[a$n]"
    total=$(awk "BEGIN { printf \"%.3f\", $total + $frames / 60 }")
    printf '  %-24s %6.2f s  (%s)\n' "$shot" "$seconds" "$res"
    n=$((n + 1))
done
echo "trailer-cut: $n shots, $total s, native $expected_res"

out_w="${expected_res%x*}"
out_h="${expected_res#*x}"
scale_filter=""
if [ -n "${SCALE:-}" ]; then
    out_w="${SCALE%x*}"
    out_h="${SCALE#*x}"
    scale_filter=",scale=${out_w}:${out_h}:flags=lanczos,setsar=1"
fi

# The sound: joined, optional voiceover ducking, measured, set to target loudness.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
joined="${agraph}${audio}concat=n=$n:v=0:a=1"

ffmpeg -hide_banner -nostdin -v error -y "${inputs[@]}" \
    -filter_complex "${joined},aresample=48000[game]" \
    -map "[game]" -vn -c:a pcm_s24le "$tmp/raw_mix.wav"

if [ -n "${VOICEOVER:-}" ]; then
    [ -f "$VOICEOVER" ] || { echo "VOICEOVER file not found: $VOICEOVER" >&2; exit 1; }
    vo_ms=$(awk "BEGIN { printf \"%d\", ${VOICEOVER_START_S:-0.0} * 1000 }")
    ffmpeg -hide_banner -nostdin -v error -y -i "$tmp/raw_mix.wav" -i "$VOICEOVER" \
        -filter_complex "[1:a]aresample=48000,aformat=channel_layouts=stereo,adelay=${vo_ms}|${vo_ms}[vo];[vo]asplit=2[vo_sc][vo_mix];[0:a][vo_sc]sidechaincompress=threshold=0.03:ratio=3.5:attack=15:release=250:makeup=1[ducked];[ducked][vo_mix]amix=inputs=2:duration=first:normalize=0[mixed]" \
        -map "[mixed]" -vn -c:a pcm_s24le "$tmp/with_vo.wav"
    mv "$tmp/with_vo.wav" "$tmp/raw_mix.wav"
    echo "trailer-cut: mixed voiceover $VOICEOVER (start ${VOICEOVER_START_S:-0.0} s)"
fi

measure=$(ffmpeg -hide_banner -nostdin -i "$tmp/raw_mix.wav" \
    -af "loudnorm=I=$loudness:$limits:print_format=json" \
    -vn -f null - 2>&1 | python3 -c '
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
ffmpeg -hide_banner -nostdin -v error -y -i "$tmp/raw_mix.wav" \
    -af "loudnorm=I=$loudness:$limits:$measure:linear=true,aresample=48000" \
    -vn -c:a pcm_s24le "$tmp/normalized.wav"
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

# Optional captions (TOML rendered via ImageMagick PNG overlays, works without libass).
cap_inputs=()
cap_graph=""
v_final="[vcat]"
if [ -n "${CAPTIONS:-}" ]; then
    [ -f "$CAPTIONS" ] || { echo "CAPTIONS file not found: $CAPTIONS" >&2; exit 1; }
    cap_idx=$((n + 1))
    cur_v="[vcat]"
    while IFS=$'\t' read -r c_start c_end c_png; do
        [ -n "$c_png" ] || continue
        cap_inputs+=(-loop 1 -i "$c_png")
        next_v="[vcap$cap_idx]"
        cap_graph+=";${cur_v}[${cap_idx}:v]overlay=0:0:shortest=1:enable='between(t,${c_start},${c_end})'${next_v}"
        cur_v="$next_v"
        cap_idx=$((cap_idx + 1))
    done < <(python3 -I tools/shorts_captions.py "$CAPTIONS" "$tmp/caps" "$out_w" "$out_h")
    v_final="$cur_v"
    echo "trailer-cut: overlaying captions from $CAPTIONS"
fi

# The picture, joined and encoded with that sound.
mkdir -p "$(dirname "$out")"
ffmpeg -hide_banner -nostdin -v error -y "${inputs[@]}" -i "$tmp/sound.wav" ${cap_inputs[@]+"${cap_inputs[@]}"} \
    -filter_complex "${vgraph}${vlabels}concat=n=$n:v=1:a=0${scale_filter}[vcat]${cap_graph}" \
    -map "$v_final" -map "$n:a" \
    -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -color_range tv -r 60 -profile:v high \
    -c:a aac -b:a 256k -ar 48000 -movflags +faststart "$out"

ffprobe -v error -show_entries format=duration,size -show_entries stream=codec_name,width,height,pix_fmt,r_frame_rate,sample_aspect_ratio \
    -of compact "$out"
echo "trailer-cut: wrote $out"
