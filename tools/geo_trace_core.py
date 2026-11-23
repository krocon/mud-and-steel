#!/usr/bin/env python3
"""Trace the core of a geodata site from a historical topographic map
(prompt 118): a one-time aid, not part of `geo_import.py generate`.

The Kadaster's historical topographic maps (Topotijdreis tile service, RD
New tiling, level 11 = 1.59 m per pixel) show built-up blocks in salmon,
single buildings in red, water in grey-blue and streets as the white space
between blocks. This script warps the tiles into the site's map frame,
classifies the colours and writes a first draft of the core file:

  blocks   outlines of the built-up blocks and buildings (rings)
  water    ponds
  parks    green areas

in degrees * 1e7 like `osm.json.gz`, so `geo_import.py` reads them as
footprints and areas. The streets are not traced here: the importer keeps
today's street axes where the old map shows open ground and drops those
that run through its blocks; streets the old map has and today's data
lacks, the bridge ramp of 1935 and the church are added by hand in the
site's `core_1944.json` (`streets`, `buildings`), which this script keeps
when it writes the draft again.

    python3 -I tools/geo_trace_core.py SITE.json TILES_DIR OUT.json [--report DIR]

TILES_DIR holds the raw tiles `t<year>_<row>_<col>.jpg` (downloaded once
with any HTTP client into build/, never into the repository). Needs numpy
and ImageMagick (`magick`) on the developer's machine; the result is plain
JSON, so the importer stays standard-library only.
"""

import argparse
import json
import pathlib
import subprocess
import sys

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import geo_import as G  # noqa: E402

TILE_ORIGIN = (-30515500.0, 31112399.999999993)
TILE_RES = 1.5875031750063502  # level 11


def warp(g, tiles_dir, year, box, mpp=1.0):
    """The topo map resampled into the map frame (nearest pixel)."""
    x0, y0, x1, y1 = box
    c = G.frame_to_rd(g, 0.0, 0.0)
    ex = G.frame_to_rd(g, 1000.0, 0.0)
    ey = G.frame_to_rd(g, 0.0, 1000.0)
    ax = ((ex[0] - c[0]) / 1000, (ex[1] - c[1]) / 1000)
    ay = ((ey[0] - c[0]) / 1000, (ey[1] - c[1]) / 1000)
    w, h = int((x1 - x0) / mpp), int((y1 - y0) / mpp)
    X, Y = np.meshgrid(x0 + (np.arange(w) + 0.5) * mpp, y0 + (np.arange(h) + 0.5) * mpp)
    E = c[0] + ax[0] * X + ay[0] * Y
    N = c[1] + ax[1] * X + ay[1] * Y
    col = (E - TILE_ORIGIN[0]) / TILE_RES
    row = (TILE_ORIGIN[1] - N) / TILE_RES
    c0, c1 = int(col.min() // 256), int(col.max() // 256)
    r0, r1 = int(row.min() // 256), int(row.max() // 256)
    mos = np.zeros(((r1 - r0 + 1) * 256, (c1 - c0 + 1) * 256, 3), np.uint8)
    for r in range(r0, r1 + 1):
        for cc in range(c0, c1 + 1):
            f = pathlib.Path(tiles_dir) / ("t%s_%d_%d.jpg" % (year, r, cc))
            if not f.exists():
                raise SystemExit("missing tile %s (level 11, row %d, col %d)" % (f, r, cc))
            raw = subprocess.run(["magick", str(f), "-depth", "8", "rgb:-"], capture_output=True, check=True).stdout
            mos[(r - r0) * 256:(r - r0 + 1) * 256, (cc - c0) * 256:(cc - c0 + 1) * 256] = \
                np.frombuffer(raw, np.uint8).reshape(256, 256, 3)
    pi = np.clip((row - r0 * 256).astype(int), 0, mos.shape[0] - 1)
    pj = np.clip((col - c0 * 256).astype(int), 0, mos.shape[1] - 1)
    return mos[pi, pj]


def classify(img):
    """0 open (white, dark lines and text), 1 block (salmon), 2 building
    (red), 3 water, 4 green."""
    R, Gc, B = (img[..., k].astype(int) for k in range(3))
    red = (R > 140) & (R - Gc > 70) & (R - B > 60)
    salmon = (R > 170) & (R - Gc > 30) & (Gc - B > -8) & (R - B > 45) & ~red
    dark = (R + Gc + B < 330) & ~red
    water = (abs(R - Gc) < 25) & (Gc - R > 5) & (Gc > 170) & (B > 160) & (Gc - B < 25) & (R < 200)
    green = (Gc - R > 15) & (Gc - B > 15) & ~water
    cls = np.zeros(R.shape, np.int8)
    cls[salmon] = 1
    cls[red] = 2
    cls[water] = 3
    cls[green] = 4
    # Dark pixels are street casings and text: open ground; text inside
    # blocks is closed again by the caller.
    cls[dark] = 0
    return cls


def shift(m, dy, dx):
    out = np.zeros_like(m)
    h, w = m.shape
    out[max(dy, 0):h + min(dy, 0), max(dx, 0):w + min(dx, 0)] = m[max(-dy, 0):h + min(-dy, 0), max(-dx, 0):w + min(-dx, 0)]
    return out


def dilate(m, r):
    out = m.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dy * dy + dx * dx <= r * r:
                out |= shift(m, dy, dx)
    return out


def erode(m, r):
    return ~dilate(~m, r)


NB = ((-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1))


def ring_of(mask):
    """Outer boundary of a single component (Moore tracing), pixel corners."""
    ys, xs = np.nonzero(mask)
    start = (ys.min(), xs[ys == ys.min()].min())
    h, w = mask.shape

    def on(p):
        return 0 <= p[0] < h and 0 <= p[1] < w and mask[p]
    ring = [start]
    cur, back = start, 6  # came from the west
    for _ in range(4 * mask.sum() + 10):
        for k in range(8):
            d = (back + 1 + k) % 8
            q = (cur[0] + NB[d][0], cur[1] + NB[d][1])
            if on(q):
                back = (d + 4) % 8
                cur = q
                break
        else:
            break
        if cur == start:
            break
        ring.append(cur)
    return ring


def label(m):
    """Connected components (4-neighbours) by flood fill; returns a list of masks' pixel lists."""
    lab = np.zeros(m.shape, np.int32)
    comps = []
    h, w = m.shape
    for y0, x0 in zip(*np.nonzero(m)):
        if lab[y0, x0]:
            continue
        k = len(comps) + 1
        stack = [(y0, x0)]
        lab[y0, x0] = k
        px = []
        while stack:
            y, x = stack.pop()
            px.append((y, x))
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                yy, xx = y + dy, x + dx
                if 0 <= yy < h and 0 <= xx < w and m[yy, xx] and not lab[yy, xx]:
                    lab[yy, xx] = k
                    stack.append((yy, xx))
        comps.append(px)
    return comps


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("site")
    ap.add_argument("tiles")
    ap.add_argument("out")
    ap.add_argument("--report")
    a = ap.parse_args()
    site = json.loads(pathlib.Path(a.site).read_text())
    core = site["core"]
    g = G.Grid(site["frame"])
    poly = core["polygon_m"]
    x0 = min(p[0] for p in poly) - 20
    x1 = max(p[0] for p in poly) + 20
    y0 = min(p[1] for p in poly) - 20
    y1 = max(p[1] for p in poly) + 20
    img = warp(g, a.tiles, core.get("year", "1944"), (x0, y0, x1, y1))
    cls = classify(img)
    h, w = cls.shape
    X, Y = np.meshgrid(x0 + np.arange(w) + 0.5, y0 + np.arange(h) + 0.5)
    inside = np.array([[G.in_polygon(X[j, i], Y[j, i], poly) for i in range(0, w, 4)] for j in range(0, h, 4)])
    inside = np.repeat(np.repeat(inside, 4, 0), 4, 1)[:h, :w]
    blocks = (cls == 1) | (cls == 2)
    # Text and black dots inside blocks: close small holes; the street
    # casing lines stay free (they were read as open ground).
    blocks = erode(dilate(blocks, 2), 2)
    blocks = dilate(erode(blocks, 1), 1)
    water = dilate(erode(cls == 3, 2), 2)
    # Ground cleared after the map's edition (the bridge ramp of 1935).
    for cp in core.get("cleared_polygons_m", []):
        cut = np.array([[G.in_polygon(X[j, i], Y[j, i], cp) for i in range(w)] for j in range(h)])
        inside &= ~cut

    def to_deg(px):
        x, y = x0 + px[1] + 0.5, y0 + px[0] + 0.5
        lat, lon = G.frame_to_geo(g, x, y)
        return [int(round(lat * 1e7)), int(round(lon * 1e7))]

    out = {"format": "geo_import core v1", "blocks": [], "water": [], "parks": []}
    min_px = core.get("block_min_m2", 60)
    for px in label(blocks & inside):
        if len(px) < min_px:
            continue
        m = np.zeros((h, w), bool)
        ys, xs = zip(*px)
        m[ys, xs] = True
        ring = ring_of(m)
        pts = G.simplify([(p[1] * 1.0, p[0] * 1.0) for p in ring] + [(ring[0][1] * 1.0, ring[0][0] * 1.0)], 1.5)
        if len(pts) < 4:
            continue
        reds = sum(1 for y, x in px if cls[y, x] == 2)
        flat = []
        for xx, yy in pts:
            flat += to_deg((yy, xx))
        out["blocks"].append({"kind": "building" if reds * 2 > len(px) else "block", "m2": len(px), "pts": flat})
    for px in label(water & inside):
        if len(px) < 400:
            continue
        m = np.zeros((h, w), bool)
        ys, xs = zip(*px)
        m[ys, xs] = True
        ring = ring_of(m)
        pts = G.simplify([(p[1] * 1.0, p[0] * 1.0) for p in ring] + [(ring[0][1] * 1.0, ring[0][0] * 1.0)], 2.0)
        flat = []
        for xx, yy in pts:
            flat += to_deg((yy, xx))
        out["water"].append({"m2": len(px), "pts": flat})
    parks = dilate(erode(cls == 4, 2), 2)
    for px in label(parks & inside):
        if len(px) < 300:
            continue
        m = np.zeros((h, w), bool)
        ys, xs = zip(*px)
        m[ys, xs] = True
        ring = ring_of(m)
        pts = G.simplify([(p[1] * 1.0, p[0] * 1.0) for p in ring] + [(ring[0][1] * 1.0, ring[0][0] * 1.0)], 2.0)
        flat = []
        for xx, yy in pts:
            flat += to_deg((yy, xx))
        out["parks"].append({"m2": len(px), "pts": flat})
    # Keep what was added by hand.
    old = json.loads(pathlib.Path(a.out).read_text()) if pathlib.Path(a.out).exists() else {}
    for key in ("note", "streets", "buildings", "landmarks"):
        if key in old:
            out[key] = old[key]
    pathlib.Path(a.out).write_text(json.dumps(out, indent=1, sort_keys=True))
    print("core draft: %d blocks, %d ponds, %d parks" % (len(out["blocks"]), len(out["water"]), len(out["parks"])))
    if a.report:
        rep = pathlib.Path(a.report)
        rep.mkdir(parents=True, exist_ok=True)
        pic = img.copy()
        pic[blocks & inside] = (pic[blocks & inside] // 2 + np.array([0, 100, 0], np.uint8))
        subprocess.run(["magick", "-size", "%dx%d" % (w, h), "-depth", "8", "rgb:-", str(rep / "core_trace.png")],
                       input=pic.tobytes(), check=True)


if __name__ == "__main__":
    main()
