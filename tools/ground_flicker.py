"""Does the ground texture stick to the ground while the camera moves?

usage: ground_flicker.py <video or png pattern> <first_frame> <count> <x> <y> <w> <h>

The window (pixels, 1920 x 1080 frames) must show only ground for all frames.
Per pair of consecutive frames: the ground motion from the coarse structure
(shadows, blotches of 24-120 px), then
- slide: how far the fine texture (wavelengths below 8 px) moved against the
  ground, in pixels per frame (0 = it sticks to the ground),
- ncc per band: how well frame n+1 matches frame n after undoing the ground
  motion (1 = stable, 0 = unrelated).
Needs ffmpeg and numpy.
"""
import subprocess
import sys

import numpy as np

video, first, count, x, y, w, h = sys.argv[1], *map(int, sys.argv[2:8])
W, H = 1920, 1080
raw = subprocess.run(
    ["ffmpeg", "-v", "error", "-i", video, "-vf",
     f"select='between(n\\,{first}\\,{first + count - 1})'", "-vsync", "0",
     "-f", "rawvideo", "-pix_fmt", "gray", "-"],
    check=True, capture_output=True).stdout
frames = np.frombuffer(raw, np.uint8).reshape(-1, H, W)[:, y:y + h, x:x + w].astype(np.float64)

fy = np.fft.fftfreq(h)[:, None]
fx = np.fft.fftfreq(w)[None, :]
fr = np.hypot(fx, fy)
win = np.hanning(h)[:, None] * np.hanning(w)[None, :]
coarse = np.exp(-(fr * 24.0) ** 2) * (fr > 1 / 120.0)
fine = fr > 1 / 8.0
bands = {"16-32 px": (fr > 1 / 32) & (fr <= 1 / 16), "8-16 px": (fr > 1 / 16) & (fr <= 1 / 8),
         "4-8 px": (fr > 1 / 8) & (fr <= 1 / 4), "2-4 px": fr > 1 / 4}


def spectrum(img, band):
    return np.fft.fft2((img - img.mean()) * win) * band


def shift_of(a, b, band):
    """Shift of b against a in pixels, from the phase correlation in `band`."""
    cross = spectrum(a, band) * np.conj(spectrum(b, band))
    corr = np.fft.ifft2(cross / (np.abs(cross) + 1e-9) * band).real
    iy, ix = np.unravel_index(np.argmax(corr), corr.shape)

    def refine(c, i, n):
        left, mid, right = c[(i - 1) % n], c[i], c[(i + 1) % n]
        d = left - 2 * mid + right
        i = i + (0.5 * (left - right) / d if d != 0 else 0.0)
        return i - n if i > n / 2 else i

    return refine(corr[iy, :], ix, w), refine(corr[:, ix], iy, h)


def ncc(a, b, band, sx, sy):
    pa = np.fft.ifft2(spectrum(a, band)).real
    pb = np.fft.ifft2(spectrum(b, band) * np.exp(-2j * np.pi * (fx * sx + fy * sy))).real
    return (pa * pb).sum() / np.sqrt((pa * pa).sum() * (pb * pb).sum())


ground, slide, match = [], [], {k: [] for k in bands}
for a, b in zip(frames[:-1], frames[1:]):
    gx, gy = shift_of(a, b, coarse)
    tx, ty = shift_of(a, b, fine)
    ground.append(np.hypot(gx, gy))
    slide.append(np.hypot(tx - gx, ty - gy))
    for k, band in bands.items():
        match[k].append(ncc(a, b, band, gx, gy))

print(f"{video}: frames {first}..{first + count - 1}, window {w}x{h}+{x}+{y}")
print(f"  ground motion  {np.mean(ground):.2f} px/frame")
print(f"  slide          {np.median(slide):.2f} px/frame (fine texture against the ground, median)")
for k, v in match.items():
    print(f"  ncc {k:8s}   {np.mean(v):.3f}")
