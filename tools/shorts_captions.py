#!/usr/bin/env python3
"""Renders vertical Shorts captions from TOML into transparent PNG overlays (prompt 238).

Uses ImageMagick (`magick`) so `tools/trailer-cut.sh` can overlay bold outlined
hook captions with plain ffmpeg `overlay`, without requiring libass or freetype
inside ffmpeg. Compatible with Python 3.8+ (no third-party Python dependencies).

Input TOML:

    font = "Helvetica-Bold"   # optional ImageMagick font name
    font_size = 92            # optional (at 1080x1920, scaled automatically)
    color = "#ff2b3a"         # optional hex RGB, default white
    outline_color = "#000000"
    outline_px = 14           # optional stroke width

    [[caption]]
    start_s = 0.6
    end_s = 2.6
    text = "Short caption\\nsecond line"
    y_pct = 24                # optional vertical center (0 = top, 100 = bottom)
    color = "#ff2b3a"         # optional per-caption override
    font_size = 104           # optional per-caption override

Usage:
    python3 tools/shorts_captions.py <input.toml> <out_dir> <width> <height>

Writes `<out_dir>/cap_00.png`, ... and prints one TSV line per caption:
    <start_s>\t<end_s>\t<png_path>
"""

from __future__ import annotations

import ast
import pathlib
import shutil
import subprocess
import sys

try:
    import tomllib  # type: ignore[import-not-found]
except ModuleNotFoundError:
    tomllib = None


def _parse_simple_toml(text: str) -> dict:
    root: dict = {"caption": []}
    target = root
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line == "[[caption]]":
            entry: dict = {}
            root["caption"].append(entry)
            target = entry
            continue
        if "=" not in line:
            continue
        k, v = line.split("=", 1)
        key = k.strip()
        val = v.strip()
        in_str = False
        esc = False
        for idx, ch in enumerate(val):
            if esc:
                esc = False
                continue
            if ch == "\\" and in_str:
                esc = True
            elif ch == '"':
                in_str = not in_str
            elif ch == "#" and not in_str:
                val = val[:idx].strip()
                break
        if val.startswith('"') and val.endswith('"'):
            target[key] = ast.literal_eval(val)
        elif val in ("true", "false"):
            target[key] = val == "true"
        else:
            target[key] = float(val) if "." in val else int(val)
    return root


def _load_toml(path: pathlib.Path) -> dict:
    text = path.read_text(encoding="utf-8")
    if tomllib is not None:
        return tomllib.loads(text)
    return _parse_simple_toml(text)


def main() -> int:
    if len(sys.argv) < 5:
        print(
            "usage: shorts_captions.py <input.toml> <out_dir> <width> <height>",
            file=sys.stderr,
        )
        return 2
    magick = shutil.which("magick") or shutil.which("convert")
    if not magick:
        print("shorts_captions.py: ImageMagick (magick/convert) is required", file=sys.stderr)
        return 1

    src = pathlib.Path(sys.argv[1])
    out_dir = pathlib.Path(sys.argv[2])
    width = int(sys.argv[3])
    height = int(sys.argv[4])
    out_dir.mkdir(parents=True, exist_ok=True)

    data = _load_toml(src)
    scale = height / 1920.0
    font = str(data.get("font", "Helvetica-Bold"))
    base_size = float(data.get("font_size", 92))
    base_color = str(data.get("color", "#FFFFFF"))
    outline_color = str(data.get("outline_color", "#000000"))
    outline_px = max(4, round(float(data.get("outline_px", 14)) * scale))

    for idx, item in enumerate(data.get("caption", [])):
        start_s = float(item["start_s"])
        end_s = float(item["end_s"])
        text = str(item["text"]).replace("\r\n", "\n")
        color = str(item.get("color", base_color))
        font_size = round(float(item.get("font_size", base_size)) * scale)
        y_pct = float(item.get("y_pct", 24.0))
        y_off = round((y_pct - 50.0) / 100.0 * height)
        png = out_dir / f"cap_{idx:02d}.png"
        # Draw thick outline stroke first, then crisp fill on top so stroke never eats glyphs.
        cmd = [
            magick,
            "-size",
            f"{width}x{height}",
            "xc:none",
            "-gravity",
            "Center",
            "-font",
            font,
            "-pointsize",
            str(font_size),
            "-interline-spacing",
            str(round(8 * scale)),
            "-stroke",
            outline_color,
            "-strokewidth",
            str(outline_px),
            "-fill",
            outline_color,
            "-annotate",
            f"+0{y_off:+d}",
            text,
            "-stroke",
            "none",
            "-fill",
            color,
            "-annotate",
            f"+0{y_off:+d}",
            text,
            str(png),
        ]
        subprocess.run(cmd, check=True)
        print(f"{start_s:.3f}\t{end_s:.3f}\t{png}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
