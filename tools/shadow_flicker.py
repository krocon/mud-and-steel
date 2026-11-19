"""How much does the picture change while the camera stands? (prompt 090)

usage: shadow_flicker.py <video> <first_frame> <count>

For the shot `zoom_shadow_range`: the camera stands, only the values that
follow the zoom change. Per pair of consecutive frames the mean absolute
difference of the fine detail (luminance minus its 9 x 9 mean, so the slow
change of the haze drops out), in grey levels of 255:
- mean: the flicker over the whole shot,
- quiet pairs: the share of frame pairs that are (almost) the same,
- worst: the largest single jump (a step of the shadow distance).
Needs ffmpeg and numpy.
"""
import subprocess
import sys

import numpy as np

video, first, count = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
W, H = 1920, 1080
raw = subprocess.run(
    ["ffmpeg", "-v", "error", "-i", video, "-vf",
     f"select='between(n\\,{first}\\,{first + count - 1})'", "-vsync", "0",
     "-f", "rawvideo", "-pix_fmt", "gray", "-"],
    check=True, capture_output=True).stdout
frames = np.frombuffer(raw, np.uint8).reshape(-1, H, W).astype(np.float32)


def fine(img):
    k = 9
    c = np.cumsum(np.cumsum(np.pad(img, ((k, k), (k, k)), mode="edge"), axis=0), axis=1)
    box = (c[k:, k:] - c[:-k, k:] - c[k:, :-k] + c[:-k, :-k]) / (k * k)
    return img - box[: img.shape[0], : img.shape[1]]


diffs = []
prev = fine(frames[0])
for frame in frames[1:]:
    cur = fine(frame)
    diffs.append(float(np.abs(cur - prev).mean()))
    prev = cur
diffs = np.array(diffs)
print(f"{video}: frames {first}..{first + count - 1}")
print(f"  mean        {diffs.mean():.3f}")
print(f"  quiet pairs {100.0 * (diffs < 0.05).mean():.0f} %")
print(f"  worst       {diffs.max():.3f}")
