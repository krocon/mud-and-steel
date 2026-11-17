"""Picture series of explosions: a reference video against the game (prompt 072).

usage:
  explosion_series.py video <video> <start_s> <x,y,w,h> <until_s> <out.png> [--every]
      Tiles of the video from <start_s> on, cropped to x,y,w,h (pixels of the
      full frame): every frame of the first second, then four per second until
      <until_s> seconds after the start. With --every: four tiles per second of
      the whole video from <start_s> to <until_s> (overview, no crop: give 0,0,0,0).
  explosion_series.py game <dir> <kind> <x,y,w,h> <until_s> <out.png>
      The same tiles from the game's frames <dir>/<kind>_<ms>.png, written by
      `just explosion-series` (blast_shots.gd, `--explosion-series`).
  explosion_series.py stack <out.png> <a.png> <b.png> ...
      Puts the tile sheets under each other (video above, game below).
  explosion_series.py color <video> <t_s> <x,y,w,h>
      Mean sRGB colour of a window of one video frame (measurements).

Every tile carries its time in milliseconds after the start (a small built-in
digit font: this ffmpeg has no drawtext). Tile sheets are at most 1600 pixels
wide. Needs ffmpeg and numpy. The video is a reference for timing and look
only: its pictures stay in build/ and never enter the repository.
"""
import subprocess
import sys

import numpy as np

FPS = 30
TILE = 160
COLUMNS = 10

# 3 x 5 digit font.
DIGITS = {
    "0": "111101101101111", "1": "010110010010111", "2": "111001111100111",
    "3": "111001111001111", "4": "101101111001001", "5": "111100111001111",
    "6": "111100111101111", "7": "111001001001001", "8": "111101111101111",
    "9": "111101111001111", "s": "000011010001110", "m": "000111111101101",
    ".": "000000000000010", "-": "000000111000000",
}


def series_times(until: float) -> list[float]:
    """Every frame of the first second, then four per second."""
    times = [i / FPS for i in range(FPS)]
    t = 1.0
    while t <= until + 1e-6:
        times.append(t)
        t += 0.25
    return times


def label(tile: np.ndarray, text: str) -> None:
    scale = 3
    x0, y0 = 3, 3
    width = len(text) * 4 * scale + scale
    tile[y0 - 2:y0 + 5 * scale + 2, x0 - 2:x0 + width] = (tile[y0 - 2:y0 + 5 * scale + 2, x0 - 2:x0 + width] * 0.35).astype(np.uint8)
    for k, ch in enumerate(text):
        bits = DIGITS.get(ch)
        if bits is None:
            continue
        for i, b in enumerate(bits):
            if b == "1":
                r, c = divmod(i, 3)
                y = y0 + r * scale
                x = x0 + (k * 4 + c) * scale
                tile[y:y + scale, x:x + scale] = (255, 230, 60)


def sheet(tiles: list[tuple[np.ndarray, str]], out: str) -> None:
    rows = (len(tiles) + COLUMNS - 1) // COLUMNS
    h, w = tiles[0][0].shape[:2]
    canvas = np.zeros((rows * (h + 2), COLUMNS * (w + 2), 3), np.uint8)
    for n, (tile, text) in enumerate(tiles):
        tile = tile.copy()
        label(tile, text)
        r, c = divmod(n, COLUMNS)
        canvas[r * (h + 2):r * (h + 2) + h, c * (w + 2):c * (w + 2) + w] = tile
    write_png(canvas, out)


def write_png(image: np.ndarray, out: str) -> None:
    h, w = image.shape[:2]
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{w}x{h}",
                    "-i", "-", out], input=image.tobytes(), check=True)


def read_png(path: str, crop: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = crop
    vf = f"crop={w}:{h}:{x}:{y}," if w > 0 else ""
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-vf", f"{vf}scale={TILE}:{TILE}",
                          "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(TILE, TILE, 3)


def video_frames(video: str, start: float, seconds: float, crop: tuple[int, int, int, int],
                 size: tuple[int, int]) -> np.ndarray:
    x, y, w, h = crop
    vf = f"crop={w}:{h}:{x}:{y}," if w > 0 else ""
    raw = subprocess.run(["ffmpeg", "-v", "error", "-ss", f"{start:.4f}", "-i", video, "-t", f"{seconds:.4f}",
                          "-vf", f"{vf}scale={size[0]}:{size[1]}", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
                         check=True, capture_output=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, size[1], size[0], 3)


def crop_arg(text: str) -> tuple[int, int, int, int]:
    x, y, w, h = (int(v) for v in text.split(","))
    return x, y, w, h


def main() -> None:
    mode = sys.argv[1]
    if mode == "video":
        video, start, crop, until, out = sys.argv[2], float(sys.argv[3]), crop_arg(sys.argv[4]), float(sys.argv[5]), sys.argv[6]
        if "--every" in sys.argv:
            frames = video_frames(video, start, until - start, crop, (TILE, int(TILE * 16 / 9)))
            steps = int((until - start) * 4)
            tiles = [(frames[int(round(k * FPS / 4))], f"{start + k / 4:.2f}s") for k in range(steps)
                     if int(round(k * FPS / 4)) < len(frames)]
        else:
            frames = video_frames(video, start, until + 0.1, crop, (TILE, TILE))
            tiles = []
            for t in series_times(until):
                i = int(round(t * FPS))
                if i < len(frames):
                    tiles.append((frames[i], f"{int(round(t * 1000))}"))
        sheet(tiles, out)
    elif mode == "game":
        folder, kind, crop, until, out = sys.argv[2], sys.argv[3], crop_arg(sys.argv[4]), float(sys.argv[5]), sys.argv[6]
        tiles = []
        for t in series_times(until):
            ms = int(round(t * 1000))
            tiles.append((read_png(f"{folder}/{kind}_{ms:05d}.png", crop), f"{ms}"))
        sheet(tiles, out)
    elif mode == "stack":
        out, inputs = sys.argv[2], sys.argv[3:]
        args = ["ffmpeg", "-v", "error", "-y"]
        for path in inputs:
            args += ["-i", path]
        # Pad to the widest sheet so vstack accepts them.
        chains = [f"[{i}:v]pad=1640:ih:0:0:black[v{i}]" for i in range(len(inputs))]
        chains.append("".join(f"[v{i}]" for i in range(len(inputs))) + f"vstack=inputs={len(inputs)}")
        subprocess.run(args + ["-filter_complex", ";".join(chains), out], check=True)
    elif mode == "color":
        video, t, crop = sys.argv[2], float(sys.argv[3]), crop_arg(sys.argv[4])
        frame = video_frames(video, t, 1.0 / FPS, crop, (crop[2], crop[3]))[0].astype(np.float64)
        # Mean in linear light, reported as sRGB.
        lin = np.where(frame / 255 <= 0.04045, frame / 255 / 12.92, ((frame / 255 + 0.055) / 1.055) ** 2.4)
        m = lin.reshape(-1, 3).mean(axis=0)
        srgb = np.where(m <= 0.0031308, m * 12.92, 1.055 * m ** (1 / 2.4) - 0.055) * 255
        print("t=%.3f srgb=(%d, %d, %d) #%02x%02x%02x" % (t, *srgb.round(), *srgb.round().astype(int)))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
