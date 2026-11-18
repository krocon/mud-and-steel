#!/usr/bin/env python3
"""Motion of trees in the wind (prompt 086), from two stills one second apart.

    python3 -I tools/wind_shots.py <prefix> [<stills dir>] [<out dir>]

Reads `survey_wind_<view>_00.5.png` and `_01.5.png` (written by
`just trailer-shots --stills --only=survey_wind_mid,survey_wind_near`), writes
`<out>/<prefix>_<view>_a.png`, `_b.png` and `_diff.png` (absolute difference,
amplified 4x) and prints per view the share of pixels that changed by more
than 8 of 255 and the mean change. Needs ffmpeg (no Python image library).
"""
import os
import subprocess
import sys

VIEWS = ["mid", "near"]
THRESHOLD = 8


def gray(path):
    out = subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", path, "-f", "rawvideo", "-pix_fmt", "gray", "-"],
        check=True, capture_output=True,
    ).stdout
    return out


def main():
    prefix = sys.argv[1]
    stills = sys.argv[2] if len(sys.argv) > 2 else "build/trailer/stills"
    out = sys.argv[3] if len(sys.argv) > 3 else "build/wind_shots"
    os.makedirs(out, exist_ok=True)
    for view in VIEWS:
        a = os.path.join(stills, f"survey_wind_{view}_00.5.png")
        b = os.path.join(stills, f"survey_wind_{view}_01.5.png")
        if not (os.path.exists(a) and os.path.exists(b)):
            print(f"{view}: stills missing")
            continue
        base = os.path.join(out, f"{prefix}_{view}")
        subprocess.run(["cp", a, base + "_a.png"], check=True)
        subprocess.run(["cp", b, base + "_b.png"], check=True)
        subprocess.run(
            ["ffmpeg", "-loglevel", "error", "-y", "-i", a, "-i", b, "-filter_complex",
             "blend=all_mode=difference,lutrgb=r=val*4:g=val*4:b=val*4", base + "_diff.png"],
            check=True,
        )
        ga, gb = gray(a), gray(b)
        n = len(ga)
        diffs = [abs(x - y) for x, y in zip(ga, gb)]
        moved = sum(1 for d in diffs if d > THRESHOLD) / n
        mean = sum(diffs) / n
        print(f"{prefix} {view}: {moved * 100:.1f} % of pixels changed by > {THRESHOLD}, mean change {mean:.2f}")


if __name__ == "__main__":
    main()
