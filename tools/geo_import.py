#!/usr/bin/env python3
"""Maps from real geodata (prompt 081): OpenStreetMap + a free elevation model.

Writes a map folder in the game's format (`map.toml`, `height.png`,
`terrain.png`, `objects.csv`; docs/MAPS.md) from a site description
(`assets-src/geo/<site>/site.json`) and two compact source files next to it:

  osm.json.gz   the OpenStreetMap extract, reduced to geometry and the few
                tags the importer reads (no names, addresses or ids)
  dem.json.gz   a crop of the elevation model in centimeters

Commands:

  extract SITE --osm RAW.json --dem RAW.tif
      reads a raw Overpass JSON answer and a Copernicus DEM GeoTIFF tile
      (both downloaded once, see docs/MAPS.md) and writes the two compact
      files next to SITE
  generate SITE [--out DIR] [--report DIR]
      writes the map (default data/maps/<name>/) and, with --report,
      pictures of the stages into DIR (not part of the repository)
  check SITE
      generates the map twice into temporary folders and compares the
      files byte for byte, then with the committed map (part of `just check`)

Python 3 standard library only. Every step is deterministic: integers where
it matters, floats only with + - * / and sqrt (exactly rounded by IEEE 754),
own series for sine, cosine and arctangent, fixed iteration orders. The
same sources give the same files on every machine.

Names, addresses, house numbers and every other attribute are dropped on
reading; the map files never carry a real name.
"""

import argparse
import gzip
import json
import math
import pathlib
import struct
import sys
import tempfile
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent

# Terrain ids of data/terrain.toml.
GRASS, DIRT, ROAD, SAND, MUD, SNOW, SHALLOW, DEEP, TALL_GRASS = range(9)

# Tags kept by `extract` (everything else, names first, is dropped).
KEEP_TAGS = (
    "building", "building:levels", "height", "highway", "tunnel", "bridge",
    "layer", "covered", "oneway", "area", "natural", "landuse", "leisure",
    "man_made", "floating", "water", "intermittent", "junction",
)
KEEP_AMENITY = ("place_of_worship", "townhall")

# --------------------------------------------------------------------------
# Deterministic math

PI = 3.141592653589793


def cos_sin(deg):
    """Cosine and sine of an angle in degrees from Taylor series."""
    deg = deg % 360.0
    if deg > 180.0:
        deg -= 360.0
    x = deg * PI / 180.0
    x2 = x * x
    c, s = 1.0, x
    tc, ts = 1.0, x
    for k in range(1, 18):
        tc = -tc * x2 / ((2 * k - 1) * (2 * k))
        ts = -ts * x2 / ((2 * k) * (2 * k + 1))
        c += tc
        s += ts
    return c, s


def atan2_deg(y, x):
    """atan2 in degrees (-180, 180] from a series (argument halved twice)."""
    if x == 0.0 and y == 0.0:
        return 0.0
    ax, ay = abs(x), abs(y)
    swap = ay > ax
    t = (ax / ay) if swap else (ay / ax)  # 0 .. 1
    # atan(t) = 2 atan(t / (1 + sqrt(1 + t^2))), twice: |t| <= 0.2
    for _ in range(2):
        t = t / (1.0 + math.sqrt(1.0 + t * t))
    t2 = t * t
    a, term = 0.0, t
    for k in range(12):
        a += term / (2 * k + 1)
        term = -term * t2
    a *= 4.0
    if swap:
        a = PI / 2 - a
    if x < 0:
        a = PI - a
    if y < 0:
        a = -a
    return a * 180.0 / PI


def rint(v):
    """Round half away from zero to an int (fixed, unlike round())."""
    return int(math.floor(v + 0.5)) if v >= 0 else -int(math.floor(-v + 0.5))


class Random:
    """xorshift64*: a small deterministic random stream."""

    def __init__(self, seed):
        self.s = (seed * 0x9E3779B97F4A7C15 + 0x632BE59BD9B4E019) & 0xFFFFFFFFFFFFFFFF or 1

    def next(self):
        s = self.s
        s ^= (s >> 12)
        s ^= (s << 25) & 0xFFFFFFFFFFFFFFFF
        s ^= (s >> 27)
        self.s = s
        return (s * 0x2545F4914F6CDD1D) & 0xFFFFFFFFFFFFFFFF

    def below(self, n):
        return (self.next() >> 11) % n

    def unit(self):
        return (self.next() >> 11) / float(1 << 53)


def parse_deg(text):
    """A decimal degree string to integer 1e-7 degrees (exact)."""
    neg = text.startswith("-")
    whole, _, frac = text.lstrip("-").partition(".")
    frac = (frac + "0000000")[:7]
    v = int(whole) * 10_000_000 + int(frac)
    return -v if neg else v


# --------------------------------------------------------------------------
# PNG

def _png_chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)


def write_png(path, width, height, rows_bytes, color_type, bit_depth, palette=None):
    """Writes a PNG from a list of raw row bytes (filter: Sub, fixed zlib level)."""
    bpp = {0: 1, 2: 3, 3: 1}[color_type] * (2 if bit_depth == 16 else 1)
    raw = bytearray()
    for row in rows_bytes:
        raw.append(1)  # Sub filter
        prev = bytes(bpp) + row[:-bpp] if len(row) > bpp else bytes(bpp)
        raw += bytes((a - b) & 0xFF for a, b in zip(row, prev[: len(row)]))
    out = b"\x89PNG\r\n\x1a\n"
    out += _png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, bit_depth, color_type, 0, 0, 0))
    if palette is not None:
        out += _png_chunk(b"PLTE", bytes(palette))
    out += _png_chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    out += _png_chunk(b"IEND", b"")
    pathlib.Path(path).write_bytes(out)


def write_rgb(path, width, height, rgb):
    """rgb: bytearray of width * height * 3."""
    rows = [bytes(rgb[j * width * 3:(j + 1) * width * 3]) for j in range(height)]
    write_png(path, width, height, rows, 2, 8)


# --------------------------------------------------------------------------
# Sources: extract

def _clean_tags(tags):
    out = {}
    for k in KEEP_TAGS:
        if k in tags:
            out[k] = tags[k]
    if tags.get("amenity") in KEEP_AMENITY:
        out["amenity"] = tags["amenity"]
    return out


def _pts(geometry):
    flat = []
    for g in geometry:
        if g is None:
            continue
        flat.append(parse_deg(g["lat"]))
        flat.append(parse_deg(g["lon"]))
    return flat


def _rings(members, role):
    """Joins the member ways of one role into closed rings (by end points)."""
    parts = [_pts(m.get("geometry") or []) for m in members if m.get("type") == "way" and m.get("role", "") == role]
    parts = [p for p in parts if len(p) >= 4]
    rings = []
    while parts:
        ring = parts.pop(0)
        changed = True
        while ring[:2] != ring[-2:] and changed:
            changed = False
            for k, p in enumerate(parts):
                if p[:2] == ring[-2:]:
                    ring += p[2:]
                elif p[-2:] == ring[-2:]:
                    rev = []
                    for i in range(len(p) - 2, -1, -2):
                        rev += p[i:i + 2]
                    ring += rev[2:]
                else:
                    continue
                parts.pop(k)
                changed = True
                break
        if ring[:2] == ring[-2:] and len(ring) >= 8:
            rings.append(ring)
    return rings


def extract_osm(raw_path):
    # Coordinates stay decimal text, so 1e-7 degrees are exact.
    data = json.loads(pathlib.Path(raw_path).read_text(encoding="utf-8"), parse_float=str)
    ways, relations = [], []
    for e in data["elements"]:
        tags = _clean_tags(e.get("tags", {}))
        if not tags:
            continue
        if e["type"] == "way":
            pts = _pts(e.get("geometry") or [])
            if len(pts) >= 4:
                ways.append({"tags": tags, "pts": pts})
        elif e["type"] == "relation":
            members = e.get("members", [])
            outer = _rings(members, "outer")
            if outer:
                relations.append({"tags": tags, "outer": outer, "inner": _rings(members, "inner")})
    ways.sort(key=lambda w: (json.dumps(w["tags"], sort_keys=True), w["pts"]))
    relations.sort(key=lambda r: (json.dumps(r["tags"], sort_keys=True), r["outer"]))
    stamp = data.get("osm3s", {}).get("timestamp_osm_base", "")
    return {"format": "geo_import osm v1", "osm_base": stamp, "ways": ways, "relations": relations}


def _tiff_tags(buf):
    bo = "<" if buf[:2] == b"II" else ">"
    off = struct.unpack(bo + "I", buf[4:8])[0]
    n = struct.unpack(bo + "H", buf[off:off + 2])[0]
    size = {1: 1, 2: 1, 3: 2, 4: 4, 12: 8, 16: 8}
    fmt = {1: "B", 2: "s", 3: "H", 4: "I", 12: "d", 16: "Q"}
    tags = {}
    for i in range(n):
        base = off + 2 + 12 * i
        tag, typ, cnt = struct.unpack(bo + "HHI", buf[base:base + 8])
        nbytes = size[typ] * cnt
        if nbytes <= 4:
            raw = buf[base + 8:base + 8 + nbytes]
        else:
            at = struct.unpack(bo + "I", buf[base + 8:base + 12])[0]
            raw = buf[at:at + nbytes]
        tags[tag] = raw if typ == 2 else list(struct.unpack(bo + str(cnt) + fmt[typ], raw))
    return bo, tags


def extract_dem(tif_path, bbox):
    """Crop of a Copernicus DEM GeoTIFF tile (float32, deflate, tiled)."""
    buf = pathlib.Path(tif_path).read_bytes()
    bo, t = _tiff_tags(buf)
    width, height = t[256][0], t[257][0]
    if t[258][0] != 32 or t[259][0] not in (1, 8, 32946) or t[339][0] != 3:
        raise SystemExit("DEM: expected float32, deflate")
    predictor = t.get(317, [1])[0]
    tw, th = t[322][0], t[323][0]
    offsets, counts = t[324], t[325]
    sx, sy = t[33550][0], t[33550][1]
    lon0, lat0 = t[33922][3], t[33922][4]
    px = rint(1.0 / sx)
    if rint(1.0 / sy) != px:
        raise SystemExit("DEM: expected square pixels in degrees")
    south, west, north, east = bbox
    c0 = int(math.floor((west / 1e7 - lon0) * px)) - 2
    c1 = int(math.floor((east / 1e7 - lon0) * px)) + 3
    r0 = int(math.floor((lat0 - north / 1e7) * px)) - 2
    r1 = int(math.floor((lat0 - south / 1e7) * px)) + 3
    c0, r0, c1, r1 = max(c0, 0), max(r0, 0), min(c1, width), min(r1, height)
    tiles_across = (width + tw - 1) // tw
    cache = {}

    def tile(ti, tj):
        key = (ti, tj)
        if key not in cache:
            k = tj * tiles_across + ti
            raw = zlib.decompress(buf[offsets[k]:offsets[k] + counts[k]])
            rows = []
            for r in range(th):
                row = bytearray(raw[r * tw * 4:(r + 1) * tw * 4])
                if predictor == 3:
                    for i in range(1, len(row)):
                        row[i] = (row[i] + row[i - 1]) & 0xFF
                    # byte planes (most significant first) to big-endian floats
                    planes = [row[p * tw:(p + 1) * tw] for p in range(4)]
                    be = bytearray(4 * tw)
                    for p in range(4):
                        be[p::4] = planes[p]
                    rows.append(struct.unpack(">%df" % tw, be))
                else:
                    rows.append(struct.unpack(bo + "%df" % tw, row))
            cache[key] = rows
        return cache[key]

    values = []
    for r in range(r0, r1):
        for c in range(c0, c1):
            v = tile(c // tw, r // th)[r % th][c % tw]
            values.append(rint(v * 100.0))
    return {
        "format": "geo_import dem v1",
        "lon0_deg": rint(lon0), "lat0_deg": rint(lat0), "px_per_deg": px,
        "col0": c0, "row0": r0, "cols": c1 - c0, "rows": r1 - r0,
        "pixel": "area", "unit": "cm", "values": values,
    }


def write_gz_json(path, obj):
    text = json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()
    with open(path, "wb") as f:
        with gzip.GzipFile(filename="", mode="wb", fileobj=f, mtime=0, compresslevel=9) as g:
            g.write(text)


def read_gz_json(path):
    with gzip.open(path, "rb") as g:
        return json.loads(g.read().decode())


def cmd_extract(args):
    site_path = pathlib.Path(args.site)
    site = json.loads(site_path.read_text(encoding="utf-8"))
    bbox = [parse_deg(v) for v in site["sources"]["bbox_deg"]]
    osm = extract_osm(args.osm)
    osm["query"] = site["sources"]["osm_query"]
    write_gz_json(site_path.parent / "osm.json.gz", osm)
    dem = extract_dem(args.dem, bbox)
    write_gz_json(site_path.parent / "dem.json.gz", dem)
    print("extract: %d ways, %d relations (OSM base %s); DEM %d x %d px"
          % (len(osm["ways"]), len(osm["relations"]), osm["osm_base"], dem["cols"], dem["rows"]))


# --------------------------------------------------------------------------
# Frame and grid

class Grid:
    """Map frame: an equirectangular projection around the site center,
    turned so the map's +x axis points to `x_axis_bearing_deg`; samples
    every 2 m, centered on the origin (docs/MAPS.md, "Coordinates")."""

    def __init__(self, frame):
        self.lat7 = parse_deg(frame["center_deg"][0])
        self.lon7 = parse_deg(frame["center_deg"][1])
        phi = self.lat7 / 1e7
        cs = [cos_sin(k * phi)[0] for k in range(6)]
        # Meters per degree on the WGS 84 ellipsoid (series).
        self.mlat = 111132.954 - 559.822 * cs[2] + 1.175 * cs[4]
        self.mlon = 111412.84 * cs[1] - 93.5 * cs[3] + 0.118 * cs[5]
        self.ca, self.sa = cos_sin(90.0 - frame["x_axis_bearing_deg"])
        self.sx, self.sy = frame["size_m"]
        if self.sx % 2 or self.sy % 2:
            raise SystemExit("size_m must be even meters")
        self.W = self.sx // 2 + 1
        self.H = self.sy // 2 + 1
        self.N = self.W * self.H
        self.hx = self.sx / 2.0
        self.hy = self.sy / 2.0

    def to_map(self, lat7, lon7):
        e = (lon7 - self.lon7) * 1e-7 * self.mlon
        n = (lat7 - self.lat7) * 1e-7 * self.mlat
        return (e * self.ca + n * self.sa, e * self.sa - n * self.ca)

    def pts(self, flat):
        return [self.to_map(flat[k], flat[k + 1]) for k in range(0, len(flat), 2)]

    def x(self, i):
        return -self.hx + 2.0 * i

    def y(self, j):
        return -self.hy + 2.0 * j

    def index(self, x, y):
        """Nearest sample, or -1 outside the map."""
        i = rint((x + self.hx) / 2.0)
        j = rint((y + self.hy) / 2.0)
        if 0 <= i < self.W and 0 <= j < self.H:
            return j * self.W + i
        return -1

    def inside(self, x, y, margin=0.0):
        return -self.hx - margin <= x <= self.hx + margin and -self.hy - margin <= y <= self.hy + margin


# --------------------------------------------------------------------------
# Raster helpers

def polygon_spans(g, rings):
    """Samples inside the rings (even-odd) as (row, i0, i1) spans."""
    edges = []
    ymin, ymax = 1e18, -1e18
    for ring in rings:
        for k in range(len(ring) - 1):
            (x1, y1), (x2, y2) = ring[k], ring[k + 1]
            if y1 != y2:
                edges.append((x1, y1, x2, y2))
            ymin = min(ymin, y1)
            ymax = max(ymax, y1)
    if not edges:
        return []
    j0 = max(0, int(math.ceil((ymin + g.hy) / 2.0)))
    j1 = min(g.H - 1, int(math.floor((ymax + g.hy) / 2.0)))
    spans = []
    for j in range(j0, j1 + 1):
        yy = g.y(j)
        xs = []
        for x1, y1, x2, y2 in edges:
            if (y1 <= yy < y2) or (y2 <= yy < y1):
                xs.append(x1 + (yy - y1) * (x2 - x1) / (y2 - y1))
        xs.sort()
        for k in range(0, len(xs) - 1, 2):
            i0 = max(0, int(math.ceil((xs[k] + g.hx) / 2.0)))
            i1 = min(g.W - 1, int(math.floor((xs[k + 1] + g.hx) / 2.0)))
            if i0 <= i1:
                spans.append((j, i0, i1))
    return spans


def ring_area(ring):
    a = 0.0
    for k in range(len(ring) - 1):
        a += ring[k][0] * ring[k + 1][1] - ring[k + 1][0] * ring[k][1]
    return abs(a) / 2.0


def stamp_line(g, pts, mask):
    """Marks the samples nearest to a polyline (an 8-connected line)."""
    for idx in line_samples(g, pts):
        mask[idx] = 1


def line_samples(g, pts):
    out = []
    for k in range(len(pts) - 1):
        (ax, ay), (bx, by) = pts[k], pts[k + 1]
        length = math.sqrt((bx - ax) ** 2 + (by - ay) ** 2)
        n = int(length / 0.5) + 1
        for s in range(n + 1):
            t = s / n
            idx = g.index(ax + (bx - ax) * t, ay + (by - ay) * t)
            if idx >= 0:
                out.append(idx)
    return out


def flood(g, wall, seeds):
    """4-connected fill of the samples that are not walls, from seeds."""
    W, H = g.W, g.H
    out = bytearray(g.N)
    stack = list(seeds)
    while stack:
        k = stack.pop()
        if out[k] or wall[k]:
            continue
        j = k // W
        row0 = j * W
        left = k
        while left > row0 and not out[left - 1] and not wall[left - 1]:
            left -= 1
        right = k
        while right < row0 + W - 1 and not out[right + 1] and not wall[right + 1]:
            right += 1
        out[left:right + 1] = b"\x01" * (right - left + 1)
        for nj in (j - 1, j + 1):
            if 0 <= nj < H:
                off = (nj - j) * W
                run = False
                for m in range(left + off, right + off + 1):
                    if not out[m] and not wall[m]:
                        if not run:
                            stack.append(m)
                            run = True
                    else:
                        run = False
    return out


_T01 = bytes.maketrans(b"\x00\x01", b"01")
_T10 = bytes.maketrans(b"01", b"\x00\x01")


def to_rows(g, mask):
    """A 0/1 bytearray as one int per row (bit i = sample i)."""
    W = g.W
    return [int(bytes(mask[j * W:(j + 1) * W]).translate(_T01)[::-1], 2) for j in range(g.H)]


def to_mask(g, rows):
    out = bytearray()
    for v in rows:
        out += bin(v)[2:].zfill(g.W)[::-1].encode().translate(_T10)
    return out


def dilate(g, rows, r):
    full = (1 << g.W) - 1
    h = []
    for v in rows:
        a = v
        for k in range(1, r + 1):
            a |= (v << k) | (v >> k)
        h.append(a & full)
    out = []
    for j in range(g.H):
        a = 0
        for m in range(max(0, j - r), min(g.H, j + r + 1)):
            a |= h[m]
        out.append(a)
    return out


def erode(g, rows, r):
    """Square erosion; outside the map counts as set."""
    W = g.W
    full = (1 << W) - 1
    h = []
    for v in rows:
        a = v
        for k in range(1, r + 1):
            a &= ((v << k) | ((1 << k) - 1)) & (v >> k | (((1 << k) - 1) << (W - k)))
        h.append(a & full)
    out = []
    for j in range(g.H):
        a = full
        for m in range(max(0, j - r), min(g.H, j + r + 1)):
            a &= h[m]
        out.append(a)
    return out


def rows_not(g, rows):
    full = (1 << g.W) - 1
    return [full & ~v for v in rows]


def set_bits(g, rows):
    """Sample indices of the set bits, row by row."""
    out = []
    for j, v in enumerate(rows):
        base = j * g.W
        while v:
            low = v & -v
            out.append(base + low.bit_length() - 1)
            v ^= low
    return out


def count(mask):
    return mask.count(1)


def blur(g, h, r, passes, only=None):
    """Box blur (separable running sums), `passes` times; `only`: mask of
    samples that may change (others keep their value but still count)."""
    W, H = g.W, g.H
    n = 2 * r + 1
    for _ in range(passes):
        tmp = [0.0] * g.N
        for j in range(H):
            row = h[j * W:(j + 1) * W]
            ext = [row[0]] * r + row + [row[-1]] * r
            acc = sum(ext[0:n])
            base = j * W
            for i in range(W):
                tmp[base + i] = acc / n
                acc += ext[i + n] - ext[i] if i + n < len(ext) else 0.0
        out = [0.0] * g.N
        for i in range(W):
            col = tmp[i::W]
            ext = [col[0]] * r + col + [col[-1]] * r
            acc = sum(ext[0:n])
            for j in range(H):
                out[j * W + i] = acc / n
                acc += ext[j + n] - ext[j] if j + n < len(ext) else 0.0
        if only is None:
            h = out
        else:
            h = [out[k] if only[k] else h[k] for k in range(g.N)]
    return h


def bilinear(g, h, x, y):
    fx = min(max((x + g.hx) / 2.0, 0.0), g.W - 1.0)
    fy = min(max((y + g.hy) / 2.0, 0.0), g.H - 1.0)
    i, j = min(int(fx), g.W - 2), min(int(fy), g.H - 2)
    tx, ty = fx - i, fy - j
    k = j * g.W + i
    a = h[k] + (h[k + 1] - h[k]) * tx
    b = h[k + g.W] + (h[k + g.W + 1] - h[k + g.W]) * tx
    return a + (b - a) * ty


def simplify(pts, tol):
    """Douglas-Peucker."""
    if len(pts) < 3:
        return list(pts)
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        a, b = stack.pop()
        (ax, ay), (bx, by) = pts[a], pts[b]
        dx, dy = bx - ax, by - ay
        ll = dx * dx + dy * dy
        best, at = -1.0, -1
        for k in range(a + 1, b):
            px, py = pts[k]
            if ll == 0.0:
                d = (px - ax) ** 2 + (py - ay) ** 2
            else:
                t = ((px - ax) * dx + (py - ay) * dy) / ll
                t = min(max(t, 0.0), 1.0)
                d = (ax + t * dx - px) ** 2 + (ay + t * dy - py) ** 2
            if d > best:
                best, at = d, k
        if at >= 0 and best > tol * tol:
            keep[at] = True
            stack.append((a, at))
            stack.append((at, b))
    return [p for p, k in zip(pts, keep) if k]


def length(pts):
    return sum(math.sqrt((pts[k + 1][0] - pts[k][0]) ** 2 + (pts[k + 1][1] - pts[k][1]) ** 2)
               for k in range(len(pts) - 1))


def inside_length(g, pts):
    """Length of a line inside the map (sampled every 2 m)."""
    total = 0.0
    for k in range(len(pts) - 1):
        (ax, ay), (bx, by) = pts[k], pts[k + 1]
        seg = math.sqrt((bx - ax) ** 2 + (by - ay) ** 2)
        n = max(1, int(seg / 2.0))
        inside = sum(1 for s in range(n) if g.inside(ax + (bx - ax) * (s + 0.5) / n, ay + (by - ay) * (s + 0.5) / n))
        total += seg * inside / n
    return total


def seg_cross(a, b, c, d):
    """Whether segments ab and cd cross properly (not at end points)."""
    def orient(p, q, r):
        v = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        return (v > 1e-9) - (v < -1e-9)
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    return o1 * o2 < 0 and o3 * o4 < 0


# --------------------------------------------------------------------------
# Elevation

def dem_heights(g, dem, cfg, roof):
    """Ground heights (m) at every sample from the elevation model, which is
    a surface model (roofs, tree tops): per pixel the mean height of the
    OSM buildings over the ground (`roof`, per sample) is taken off, then a
    3 x 3 median against spikes and a grey opening against what is left of
    buildings and trees; bilinear to 2 m, box blurred."""
    cols, rows = dem["cols"], dem["rows"]
    v = [x / 100.0 for x in dem["values"]]
    px = dem["px_per_deg"]
    lon_c, lat_c = g.lon7 / 1e7, g.lat7 / 1e7
    c0 = ((lon_c - dem["lon0_deg"]) * px) - dem["col0"] - 0.5
    r0 = ((dem["lat0_deg"] - lat_c) * px) - dem["row0"] - 0.5
    # col = c0 + (x ca + y sa) / mlon * px ; row = r0 - (x sa - y ca) / mlat * px
    cx, cy = g.ca / g.mlon * px, g.sa / g.mlon * px
    rx, ry = -g.sa / g.mlat * px, g.ca / g.mlat * px
    raise_sum = [0.0] * (cols * rows)
    raise_n = [0] * (cols * rows)
    for j in range(g.H):
        y = g.y(j)
        base = j * g.W
        for i in range(g.W):
            x = g.x(i)
            ci = rint(c0 + cx * x + cy * y)
            ri = rint(r0 + rx * x + ry * y)
            if 0 <= ci < cols and 0 <= ri < rows:
                q = ri * cols + ci
                raise_sum[q] += roof[base + i]
                raise_n[q] += 1
    factor = cfg["roof_factor"]
    for q in range(cols * rows):
        if raise_n[q]:
            v[q] -= factor * raise_sum[q] / raise_n[q]
    med = list(v)
    for r in range(1, rows - 1):
        for c in range(1, cols - 1):
            w = sorted(v[(r + dr) * cols + c + dc] for dr in (-1, 0, 1) for dc in (-1, 0, 1))
            med[r * cols + c] = w[4]
    k = cfg["opening_px"]

    def window(src, fn):
        out = list(src)
        for r in range(rows):
            for c in range(cols):
                out[r * cols + c] = fn(src[rr * cols + cc]
                                       for rr in range(max(0, r - k), min(rows, r + k + 1))
                                       for cc in range(max(0, c - k), min(cols, c + k + 1)))
        return out

    opened = window(window(med, min), max)
    # Keep the surface where it lies within `opening_keep_m` of the opened
    # ground (terrain), take the opened ground where it stands above
    # (buildings, trees).
    keep = cfg["opening_keep_m"]
    ground = [min(a, b + keep) for a, b in zip(med, opened)]
    h = [0.0] * g.N
    for j in range(g.H):
        y = g.y(j)
        for i in range(g.W):
            x = g.x(i)
            fc = min(max(c0 + cx * x + cy * y, 0.0), cols - 1.001)
            fr = min(max(r0 + rx * x + ry * y, 0.0), rows - 1.001)
            ic, ir = int(fc), int(fr)
            tc, tr = fc - ic, fr - ir
            q = ir * cols + ic
            a = ground[q] + (ground[q + 1] - ground[q]) * tc
            b = ground[q + cols] + (ground[q + cols + 1] - ground[q + cols]) * tc
            h[j * g.W + i] = a + (b - a) * tr
    return blur(g, h, cfg["blur_radius"], cfg["blur_passes"])


# --------------------------------------------------------------------------
# Features from the OSM extract

class Features:
    """The OSM extract in map coordinates, sorted into what the importer uses."""

    def __init__(self, g, osm):
        self.coast = []        # polylines
        self.highways = []     # (tags, pts, keys) with keys = original node coordinates
        self.squares = []      # rings of pedestrian areas
        self.buildings = []    # (tags, rings)
        self.areas = []        # (tags, rings): landuse, leisure, natural areas
        self.lines = []        # (tags, pts): tree rows
        self.land_add = []     # rings of solid piers, breakwaters and quays
        for w in osm["ways"]:
            t = w["tags"]
            pts = g.pts(w["pts"])
            closed = w["pts"][:2] == w["pts"][-2:] and len(pts) >= 4
            if t.get("natural") == "coastline":
                self.coast.append(pts)
            elif "building" in t:
                if closed:
                    self.buildings.append((t, [pts]))
            elif "highway" in t:
                if t["highway"] == "pedestrian" and t.get("area") == "yes" and closed:
                    self.squares.append([pts])
                else:
                    keys = [(w["pts"][k], w["pts"][k + 1]) for k in range(0, len(w["pts"]), 2)]
                    self.highways.append((t, pts, keys))
            elif t.get("man_made") in ("pier", "breakwater", "quay"):
                if closed and t.get("floating") != "yes":
                    self.land_add.append([pts])
            elif t.get("natural") == "tree_row":
                self.lines.append((t, pts))
            elif closed:
                self.areas.append((t, [pts]))
        for r in osm["relations"]:
            t = r["tags"]
            rings = [g.pts(x) for x in r["outer"]] + [g.pts(x) for x in r["inner"]]
            if "building" in t:
                self.buildings.append((t, rings))
            elif t.get("natural") == "coastline":
                continue
            else:
                self.areas.append((t, rings))


def building_height(t, default):
    """Height (m) from `height` or `building:levels` (3.2 m each), else the default."""
    def number(text):
        num = ""
        for ch in text.split(";")[0].strip():
            if ch.isdigit() or ch == ".":
                num += ch
            else:
                break
        try:
            return float(num)
        except ValueError:
            return None
    if "height" in t:
        v = number(t["height"])
        if v is not None and 2.0 <= v <= 300.0:
            return v
    if "building:levels" in t:
        v = number(t["building:levels"])
        if v is not None and 1.0 <= v <= 80.0:
            return 3.2 * v + 1.0
    return default


def roof_raster(g, f, cfg):
    """Height of the OSM buildings over the ground per sample (0 = none)."""
    roof = [0.0] * g.N
    for t, rings in f.buildings:
        if t.get("building") in ("roof", "construction") or t.get("layer", "0").startswith("-"):
            continue
        hb = building_height(t, cfg["default_building_m"])
        for j, i0, i1 in polygon_spans(g, rings):
            base = j * g.W
            for i in range(i0, i1 + 1):
                if roof[base + i] < hb:
                    roof[base + i] = hb
    return roof


def area_of(rings):
    return ring_area(rings[0]) - sum(ring_area(r) for r in rings[1:])


# --------------------------------------------------------------------------
# Coast

def build_coast(g, f, cfg, stats):
    """Sea mask: coast lines as walls, flood fill from the sea seeds, solid
    piers and quays added as land, then simplified: water narrower than
    `min_width_m` closed, land narrower than it (moles, piers) opened,
    basins cut off by that closed, a majority pass over the edge."""
    wall = bytearray(g.N)
    coast_points = 0
    for pts in f.coast:
        stamp_line(g, pts, wall)
        coast_points += sum(1 for p in pts if g.inside(*p))
    seeds = [g.index(x, y) for x, y in cfg["sea_seeds_m"]]
    if min(seeds) < 0:
        raise SystemExit("a sea seed lies outside the map")
    water = flood(g, wall, seeds)
    raw_water = bytes(water)
    for rings in f.land_add:
        for j, i0, i1 in polygon_spans(g, rings):
            water[j * g.W + i0:j * g.W + i1 + 1] = bytes(i1 - i0 + 1)
    r = max(1, cfg["min_width_m"] // 4)
    rows = to_rows(g, water)
    rows = dilate(g, erode(g, rows, r), r)          # narrow water closed
    land = rows_not(g, rows)
    opened = dilate(g, erode(g, land, r), r)        # narrow land opened
    rows = rows_not(g, opened)
    water = to_mask(g, rows)
    # Basins cut off from the sea become land.
    water = flood(g, bytearray(1 - v for v in water), seeds)
    # Majority pass along the edge.
    rows = to_rows(g, water)
    band = to_mask(g, [a & b for a, b in zip(dilate(g, rows, 1), dilate(g, rows_not(g, rows), 1))])
    W = g.W
    flip = []
    for k in range(g.N):
        if not band[k]:
            continue
        i, j = k % W, k // W
        if i == 0 or j == 0 or i == W - 1 or j == g.H - 1:
            continue
        n = (water[k - W - 1] + water[k - W] + water[k - W + 1] + water[k - 1] + water[k + 1]
             + water[k + W - 1] + water[k + W] + water[k + W + 1])
        if water[k] and n <= 2:
            flip.append((k, 0))
        elif not water[k] and n >= 6:
            flip.append((k, 1))
    for k, v in flip:
        water[k] = v
    water = flood(g, bytearray(1 - v for v in water), seeds)
    edge = to_mask(g, [a & b for a, b in zip(to_rows(g, water), dilate(g, rows_not(g, to_rows(g, water)), 1))])
    stats["coast"] = {
        "osm_coast_points": coast_points,
        "water_samples_before": raw_water.count(1),
        "water_samples_after": count(water),
        "coast_samples_after": count(edge),
    }
    return water


def shape_sea(g, f, cfg, h, terrain, water):
    """Sea bed by rings from the shore, a shallow seam at beaches, land at
    least a little above the water (quays: an edge)."""
    beach = bytearray(g.N)
    for t, rings in f.areas:
        if t.get("natural") == "beach":
            for j, i0, i1 in polygon_spans(g, rings):
                beach[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    beach_near = to_mask(g, dilate(g, to_rows(g, beach), cfg["beach_reach_m"] // 2))
    rows = to_rows(g, water)
    shallow_rings = cfg["shallow_rings"]
    deepest = cfg["bed_m"][2]
    for k in range(g.N):
        if water[k]:
            h[k] = deepest
            terrain[k] = DEEP
    prev = rows
    for level in range(1, 40):
        nxt = erode(g, prev, 1)
        bed = max(cfg["bed_m"][0] - cfg["bed_m"][1] * (level - 1), deepest)
        for k in set_bits(g, [a & ~b for a, b in zip(prev, nxt)]):
            h[k] = bed
            terrain[k] = SHALLOW if (beach_near[k] and level <= shallow_rings) else DEEP
        prev = nxt
    # Land rings from the water.
    prev = rows_not(g, rows)
    for level in range(1, 9):
        nxt = erode(g, prev, 1)
        for k in set_bits(g, [a & ~b for a, b in zip(prev, nxt)]):
            if beach_near[k]:
                h[k] = max(min(h[k], cfg["beach_m"][0] + cfg["beach_m"][1] * level), cfg["beach_m"][0])
                terrain[k] = SAND
            elif level <= 2:
                h[k] = max(h[k], cfg["quay_m"])
        prev = nxt
    for k in range(g.N):
        if not water[k] and h[k] < cfg["land_min_m"]:
            h[k] = cfg["land_min_m"]
    for k in range(g.N):
        if beach[k] and not water[k]:
            terrain[k] = SAND


# --------------------------------------------------------------------------
# Roads

def select_roads(g, f, cfg, stats):
    """The carrying network: classes of `cfg.classes`, without tunnels,
    steps, foot and cycle ways, service roads; simplified lines, the upper
    of two crossing roads dropped, short dead ends and tiny fragments
    removed. Returns [(pts, keys, half_width, terrain)]."""
    classes = cfg["classes"]
    dropped = {}
    kept = []

    def drop(reason, pts):
        n, km = dropped.get(reason, (0, 0.0))
        dropped[reason] = (n + 1, km + inside_length(g, pts) / 1000.0)

    margin = 60.0
    before_km = 0.0
    for t, pts, keys in f.highways:
        if not any(g.inside(x, y, margin) for x, y in pts):
            continue
        kind = t["highway"]
        before_km += inside_length(g, pts) / 1000.0
        layer = t.get("layer", "0")
        try:
            layer = int(layer)
        except ValueError:
            layer = 0
        if t.get("tunnel", "no") != "no" or t.get("covered", "no") != "no" or layer < 0:
            drop("tunnels and covered ways", pts)
        elif kind == "steps":
            drop("steps", pts)
        elif kind in ("footway", "path", "pedestrian", "cycleway", "corridor", "elevator", "platform", "bridleway"):
            drop("foot and cycle ways, alleys", pts)
        elif kind in ("service", "services"):
            drop("service roads, drives, parking aisles", pts)
        elif kind == "raceway":
            drop("race circuit lines (on the streets)", pts)
        elif kind not in classes:
            drop("other (%s)" % kind, pts)
        else:
            width, terr = classes[kind]
            bridge = t.get("bridge", "no") != "no" or layer > 0
            kept.append({"pts": pts, "keys": keys, "hw": width / 2.0,
                         "terrain": {"road": ROAD, "dirt_track": DIRT}[terr], "bridge": bridge})
    # Upper of two crossing roads.
    bucket = {}
    for n, w in enumerate(kept):
        if w["bridge"]:
            continue
        for k in range(len(w["pts"]) - 1):
            (ax, ay), (bx, by) = w["pts"][k], w["pts"][k + 1]
            for bxk in range(int(math.floor(min(ax, bx) / 32)), int(math.floor(max(ax, bx) / 32)) + 1):
                for byk in range(int(math.floor(min(ay, by) / 32)), int(math.floor(max(ay, by) / 32)) + 1):
                    bucket.setdefault((bxk, byk), []).append((n, k))
    keep = []
    for n, w in enumerate(kept):
        crosses = False
        if w["bridge"]:
            for k in range(len(w["pts"]) - 1):
                a, b = w["pts"][k], w["pts"][k + 1]
                seen = set()
                for bxk in range(int(math.floor(min(a[0], b[0]) / 32)), int(math.floor(max(a[0], b[0]) / 32)) + 1):
                    for byk in range(int(math.floor(min(a[1], b[1]) / 32)), int(math.floor(max(a[1], b[1]) / 32)) + 1):
                        for m, q in bucket.get((bxk, byk), ()):
                            if (m, q) in seen:
                                continue
                            seen.add((m, q))
                            o = kept[m]["pts"]
                            if seg_cross(a, b, o[q], o[q + 1]):
                                crosses = True
        if crosses:
            drop("bridges over other roads (the lower road stays)", w["pts"])
        else:
            keep.append(w)
    kept = keep
    # Dead ends shorter than `stub_m`, twice; then fragments.
    for _ in range(2):
        degree = {}
        for w in kept:
            for key in set(w["keys"]):
                degree[key] = degree.get(key, 0) + 1
        keep = []
        for w in kept:
            ends = (degree[w["keys"][0]], degree[w["keys"][-1]])
            if min(ends) == 1 and length(w["pts"]) < cfg["stub_m"]:
                drop("short dead ends", w["pts"])
            else:
                keep.append(w)
        kept = keep
    parent = list(range(len(kept)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    owner = {}
    for n, w in enumerate(kept):
        for key in w["keys"]:
            if key in owner:
                ra, rb = find(owner[key]), find(n)
                if ra != rb:
                    parent[max(ra, rb)] = min(ra, rb)
            else:
                owner[key] = n
    total = {}
    for n, w in enumerate(kept):
        total[find(n)] = total.get(find(n), 0.0) + length(w["pts"])
    keep = []
    for n, w in enumerate(kept):
        if total[find(n)] < cfg["fragment_m"]:
            drop("isolated fragments", w["pts"])
        else:
            keep.append(w)
    kept = keep
    points_before = sum(len(w["pts"]) for w in kept)
    for w in kept:
        w["pts"] = simplify(w["pts"], cfg["simplify_m"])
    stats["roads"] = {
        "osm_km_in_map": round(before_km, 1),
        "kept_ways": len(kept),
        "kept_km_in_map": round(sum(inside_length(g, w["pts"]) for w in kept) / 1000.0, 1),
        "points_before": points_before,
        "points_after": sum(len(w["pts"]) for w in kept),
        "dropped": {k: [n, round(km, 2)] for k, (n, km) in sorted(dropped.items())},
    }
    return kept


def build_roads(g, roads, cfg, h, terrain, water, stats):
    """Roads as terrain on the ground: the bed level across (height of the
    nearest center line, which follows the smoothed ground along the road),
    banks of `bank_grade` beside it; parallel carriageways and junctions
    closed into one surface. Returns the road mask."""
    W = g.W
    best_d = {}
    best = {}
    reach = cfg["bank_reach_m"]
    for w in roads:
        pts = w["pts"]
        dense = [pts[0]]
        for k in range(len(pts) - 1):
            (ax, ay), (bx, by) = pts[k], pts[k + 1]
            n = max(1, int(math.ceil(math.sqrt((bx - ax) ** 2 + (by - ay) ** 2) / 4.0)))
            for s in range(1, n + 1):
                dense.append((ax + (bx - ax) * s / n, ay + (by - ay) * s / n))
        z = [bilinear(g, h, x, y) for x, y in dense]
        span = cfg["along_smooth_points"]
        for _ in range(2):
            z = [sum(z[max(0, k - span):k + span + 1]) / (min(len(z), k + span + 1) - max(0, k - span))
                 for k in range(len(z))]
        hw = w["hw"]
        r = hw + reach
        for k in range(len(dense) - 1):
            (ax, ay), (bx, by) = dense[k], dense[k + 1]
            dx, dy = bx - ax, by - ay
            ll = dx * dx + dy * dy
            i0 = max(0, int(math.floor((min(ax, bx) - r + g.hx) / 2.0)))
            i1 = min(W - 1, int(math.ceil((max(ax, bx) + r + g.hx) / 2.0)))
            j0 = max(0, int(math.floor((min(ay, by) - r + g.hy) / 2.0)))
            j1 = min(g.H - 1, int(math.ceil((max(ay, by) + r + g.hy) / 2.0)))
            for j in range(j0, j1 + 1):
                y = g.y(j)
                for i in range(i0, i1 + 1):
                    x = g.x(i)
                    t = ((x - ax) * dx + (y - ay) * dy) / ll if ll > 0 else 0.0
                    t = 0.0 if t < 0.0 else (1.0 if t > 1.0 else t)
                    px, py = ax + t * dx - x, ay + t * dy - y
                    d = math.sqrt(px * px + py * py)
                    if d > r:
                        continue
                    idx = j * W + i
                    # Rank by the distance to the edge, so a wide road wins
                    # over a narrow one next to it.
                    e = d - hw
                    if e < best_d.get(idx, 1e18):
                        best_d[idx] = e
                        best[idx] = (z[k] + (z[k + 1] - z[k]) * t, hw, w["terrain"])
    road = bytearray(g.N)
    for idx, e in best_d.items():
        if e <= 0.0 and not water[idx]:
            road[idx] = 1
    gap = cfg["merge_gap_m"] // 4
    closed = to_mask(g, erode(g, dilate(g, to_rows(g, road), gap), gap))
    merged = 0
    for idx in range(g.N):
        if closed[idx] and not road[idx] and idx in best_d and not water[idx]:
            road[idx] = 1
            best_d[idx] = 0.0
            merged += 1
    stats["roads"]["merged_m2"] = merged * 4
    stats["roads"]["road_m2"] = count(road) * 4
    # The bed: surface heights smoothed over road samples only.
    zr = {idx: best[idx][0] for idx in best_d if road[idx]}
    for _ in range(cfg["bed_smooth_passes"]):
        nz = {}
        for idx, v in zr.items():
            i, j = idx % W, idx // W
            acc, n = 0.0, 0
            for dj in (-2, -1, 0, 1, 2):
                jj = j + dj
                if 0 <= jj < g.H:
                    for di in (-2, -1, 0, 1, 2):
                        ii = i + di
                        if 0 <= ii < W:
                            q = jj * W + ii
                            if q in zr:
                                acc += zr[q]
                                n += 1
            nz[idx] = acc / n
        zr = nz
    grade = cfg["bank_grade"]
    for idx in sorted(best_d):
        if water[idx]:
            continue
        if road[idx]:
            h[idx] = zr[idx]
            terrain[idx] = best[idx][2]
        else:
            # Bank: the ground pulled into a cone of `bank_grade` around the
            # bed; where the slope is steeper than the cone, the last 4 m of
            # the reach blend back to the ground (a wall away from the road).
            z, hw, _ = best[idx]
            e = best_d[idx]
            lim = grade * e
            v = min(max(h[idx], z - lim), z + lim)
            fade = (reach - e) / 4.0
            if fade < 1.0:
                v = h[idx] + (v - h[idx]) * max(fade, 0.0)
            h[idx] = v
    return road


# --------------------------------------------------------------------------
# Pads (starts and places): flat ground

def flatten_pads(g, pads, h, terrain=None, water=None):
    """Every pad: flat at its height (the mean ground inside, or `height_m`)
    within `radius_m`, blended over `blend_m` (smoothstep)."""
    for p in pads:
        cx, cy = p["center_m"]
        r, b = float(p["radius_m"]), float(p.get("blend_m", 30))
        i0 = max(0, int((cx - r - b + g.hx) / 2.0))
        i1 = min(g.W - 1, int((cx + r + b + g.hx) / 2.0) + 1)
        j0 = max(0, int((cy - r - b + g.hy) / 2.0))
        j1 = min(g.H - 1, int((cy + r + b + g.hy) / 2.0) + 1)
        if "height_m" in p:
            level = float(p["height_m"])
        else:
            acc, n = 0.0, 0
            for j in range(j0, j1 + 1):
                for i in range(i0, i1 + 1):
                    dx, dy = g.x(i) - cx, g.y(j) - cy
                    if dx * dx + dy * dy <= r * r:
                        acc += h[j * g.W + i]
                        n += 1
            level = acc / max(n, 1)
        p["level_m"] = level
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                k = j * g.W + i
                if water is not None and water[k]:
                    continue
                dx, dy = g.x(i) - cx, g.y(j) - cy
                d = math.sqrt(dx * dx + dy * dy)
                if d <= r:
                    h[k] = level
                elif d < r + b:
                    t = (d - r) / b
                    s = t * t * (3.0 - 2.0 * t)
                    h[k] = level + (h[k] - level) * s


# --------------------------------------------------------------------------
# Terrain from land use

def paint_areas(g, f, cfg, terrain, water, road):
    """Large areas only (`min_area_m2`): beaches sand, scrub and heath tall
    grass, pedestrian squares road. Returns the masks the objects need."""
    masks = {}
    for name in ("wood", "park", "scrub", "pitch", "beach", "industrial"):
        masks[name] = bytearray(g.N)
    small = cfg["min_area_m2"]
    sorts = {
        "wood": lambda t: t.get("natural") == "wood" or t.get("landuse") == "forest",
        "park": lambda t: t.get("leisure") in ("park", "garden") or t.get("landuse") in ("recreation_ground", "cemetery", "orchard", "village_green"),
        "scrub": lambda t: t.get("natural") in ("scrub", "heath"),
        "pitch": lambda t: t.get("leisure") in ("pitch", "stadium", "track", "sports_centre", "golf_course"),
        "beach": lambda t: t.get("natural") == "beach",
        "industrial": lambda t: t.get("landuse") in ("industrial", "port", "harbour", "railway") or t.get("leisure") == "marina",
    }
    kept = {}
    for t, rings in f.areas:
        for name, test in sorts.items():
            if test(t):
                a = area_of(rings)
                key = name if a >= small else "small areas (dropped)"
                kept[key] = kept.get(key, 0) + 1
                if a < small:
                    continue
                m = masks[name]
                for j, i0, i1 in polygon_spans(g, rings):
                    m[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    for rings in f.squares:
        if area_of(rings) >= cfg["square_min_m2"]:
            for j, i0, i1 in polygon_spans(g, rings):
                for k in range(j * g.W + i0, j * g.W + i1 + 1):
                    if not water[k]:
                        terrain[k] = ROAD
            kept["squares"] = kept.get("squares", 0) + 1
    for k in range(g.N):
        if water[k] or road[k] or terrain[k] != GRASS:
            continue
        if masks["scrub"][k]:
            terrain[k] = TALL_GRASS
    return masks, kept


# --------------------------------------------------------------------------
# Output

TERRAIN_COLORS = None


def terrain_palette():
    """debug_color of data/terrain.toml (the PNG palette, as the Rust tools)."""
    colors = []
    for line in (ROOT / "data" / "terrain.toml").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("debug_color"):
            v = int(line.split("=")[1].split("#")[0].strip(), 16)
            colors.append(((v >> 16) & 255, (v >> 8) & 255, v & 255))
    return colors


def write_map(g, site, out, h, terrain, rows_csv, stats):
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    mm = [rint(v * 1000.0) for v in h]
    low, high = min(mm), max(mm)
    base = (low // 1000 - 1) * 1000
    step = 1
    while (high - base) // step > 65535:
        step += 1
    rows = []
    for j in range(g.H):
        row = bytearray()
        for v in mm[j * g.W:(j + 1) * g.W]:
            row += struct.pack(">H", min(65535, max(0, rint((v - base) / step))))
        rows.append(bytes(row))
    write_png(out / "height.png", g.W, g.H, rows, 0, 16)
    palette = [c for rgb in terrain_palette() for c in rgb]
    write_png(out / "terrain.png", g.W, g.H, [bytes(terrain[j * g.W:(j + 1) * g.W]) for j in range(g.H)], 3, 8, palette)
    lines = ["# Generated by tools/geo_import.py from OpenStreetMap (ODbL) and Copernicus DEM data",
             "# (prompt 081, docs/MAPS.md, docs/SOURCES.md); regenerate with `just gen-geo %s`." % site["name"],
             "kind,variant,x_mm,y_mm,heading_deg"]
    (out / "objects.csv").write_text("\n".join(lines + rows_csv) + "\n", encoding="utf-8")
    m = site["map"]
    toml = [
        "# Generated by tools/geo_import.py (prompt 081) from OpenStreetMap and Copernicus DEM data;",
        "# regenerate with `just gen-geo %s` (docs/MAPS.md, \"Maps from real geodata\")." % site["name"],
        'name = "%s"' % site["name"],
        'generator = "tools/geo_import.py generate assets-src/geo/%s/site.json"' % site["name"],
        "cell_size_mm = 2000",
        "samples = [%d, %d]" % (g.W, g.H),
        "height_base_mm = %d" % base,
        "height_step_mm = %d" % step,
        'biome = "%s"' % m["biome"],
        "water_flow_deg = %d" % m.get("water_flow_deg", 180),
    ]
    for key in ("weather", "daytime"):
        if key in m:
            toml.append('%s = "%s"' % (key, m[key]))
    for s in site["starts"]:
        toml += ["", "[[start]]", "player = %d" % s["player"], "team = %d" % s["team"],
                 "position_m = [%d, %d]" % tuple(s["position_m"])]
    for name, (x, y) in site["markers"]:
        toml += ["", "[[marker]]", 'name = "%s"' % name, "position_m = [%d, %d]" % (x, y)]
    (out / "map.toml").write_text("\n".join(toml) + "\n", encoding="utf-8")
    stats["map"] = {"samples": [g.W, g.H], "heights_m": [low / 1000.0, high / 1000.0],
                    "height_step_mm": step, "objects": len(rows_csv)}


# --------------------------------------------------------------------------
# Report pictures (presentation only, not part of the map)

def shade_rgb(g, h, color_of):
    """Hill-shaded picture: color per sample times light from the north-west."""
    W, H = g.W, g.H
    rgb = bytearray(W * H * 3)
    lx, ly, lz = -0.5, -0.5, 0.7071
    for j in range(H):
        for i in range(W):
            k = j * W + i
            dx = (h[k + 1 if i < W - 1 else k] - h[k - 1 if i > 0 else k]) / 4.0
            dy = (h[k + W if j < H - 1 else k] - h[k - W if j > 0 else k]) / 4.0
            n = math.sqrt(dx * dx + dy * dy + 1.0)
            light = (-dx * lx - dy * ly + lz) / n
            s = 0.45 + 0.75 * max(0.0, light)
            r, gg, b = color_of(k)
            o = 3 * k
            rgb[o] = min(255, int(r * s))
            rgb[o + 1] = min(255, int(gg * s))
            rgb[o + 2] = min(255, int(b * s))
    return rgb


def grid_overlay(g, rgb, step=100):
    """Coordinate lines every `step` m (every fifth darker, the axes red)
    and the starts and markers of the site."""
    W = g.W
    for j in range(g.H):
        y = rint(g.y(j))
        for i in range(W):
            x = rint(g.x(i))
            on_x, on_y = x % step == 0, y % step == 0
            if not (on_x or on_y):
                continue
            o = 3 * (j * W + i)
            if (on_x and x == 0) or (on_y and y == 0):
                rgb[o:o + 3] = b"\xd0\x20\x20"
            elif (on_x and x % (5 * step) == 0) or (on_y and y % (5 * step) == 0):
                rgb[o:o + 3] = b"\x10\x10\x10"
            else:
                rgb[o] //= 2
                rgb[o + 1] //= 2
                rgb[o + 2] //= 2


def mark_points(g, rgb, points):
    for (x, y), color in points:
        i0, j0 = rint((x + g.hx) / 2.0), rint((y + g.hy) / 2.0)
        for dj in range(-4, 5):
            for di in range(-4, 5):
                i, j = i0 + di, j0 + dj
                if 0 <= i < g.W and 0 <= j < g.H:
                    o = 3 * (j * g.W + i)
                    rgb[o:o + 3] = color if max(abs(di), abs(dj)) < 4 else b"\x00\x00\x00"


def report_geodata(g, f, water_raw, path):
    """The OSM data before simplification: sea, every way, every building."""
    W, H = g.W, g.H
    rgb = bytearray(b"\xf2\xef\xe6" * (W * H))
    for k in range(g.N):
        if water_raw[k]:
            rgb[3 * k:3 * k + 3] = b"\x9c\xc0\xd9"
    for t, rings in f.areas:
        col = None
        if t.get("natural") in ("wood",) or t.get("landuse") == "forest":
            col = b"\xa8\xc8\x98"
        elif t.get("leisure") in ("park", "garden"):
            col = b"\xc8\xe0\xb0"
        if col:
            for j, i0, i1 in polygon_spans(g, rings):
                rgb[3 * (j * W + i0):3 * (j * W + i1 + 1)] = col * (i1 - i0 + 1)
    for t, rings in f.buildings:
        for j, i0, i1 in polygon_spans(g, rings):
            rgb[3 * (j * W + i0):3 * (j * W + i1 + 1)] = b"\x80\x78\x70" * (i1 - i0 + 1)
    for t, pts, _ in f.highways:
        major = t["highway"] in ("motorway", "trunk", "primary", "secondary", "tertiary")
        col = b"\xc0\x40\x30" if major else (b"\x50\x50\x50" if t["highway"] in ("residential", "unclassified", "living_street") else b"\xb0\xa0\xa0")
        for k in line_samples(g, pts):
            rgb[3 * k:3 * k + 3] = col
    write_rgb(path, W, H, rgb)


# --------------------------------------------------------------------------
# The whole run

def generate(site_path, out_dir, report_dir, quiet=False):
    t0 = time.time()
    site_path = pathlib.Path(site_path)
    site = json.loads(site_path.read_text(encoding="utf-8"))
    osm = read_gz_json(site_path.parent / "osm.json.gz")
    dem = read_gz_json(site_path.parent / "dem.json.gz")
    g = Grid(site["frame"])
    stats = {}
    timing = []

    def lap(name):
        timing.append((name, round(time.time() - t0, 1)))
        if not quiet:
            print("  %-10s %6.1f s" % (name, time.time() - t0), file=sys.stderr)

    f = Features(g, osm)
    lap("features")
    roof = roof_raster(g, f, site["elevation"])
    h = dem_heights(g, dem, site["elevation"], roof)
    lap("elevation")
    terrain = bytearray(g.N)
    water = build_coast(g, f, site["coast"], stats)
    lap("coast")
    shape_sea(g, f, site["coast"], h, terrain, water)
    lap("sea")
    flatten_pads(g, site["pads"], h, water=water)
    lap("pads")
    roads = select_roads(g, f, site["roads"], stats)
    road = build_roads(g, roads, site["roads"], h, terrain, water, stats)
    lap("roads")
    masks, kept = paint_areas(g, f, site["areas"], terrain, water, road)
    stats["areas"] = kept
    lap("areas")
    rows_csv = []
    write_map(g, site, out_dir, h, terrain, rows_csv, stats)
    lap("write")
    if report_dir:
        rep = pathlib.Path(report_dir)
        rep.mkdir(parents=True, exist_ok=True)
        wall = bytearray(g.N)
        for pts in f.coast:
            stamp_line(g, pts, wall)
        seeds = [g.index(x, y) for x, y in site["coast"]["sea_seeds_m"]]
        report_geodata(g, f, flood(g, wall, seeds), rep / "geodata.png")
        pal = terrain_palette()
        rgb = shade_rgb(g, h, lambda k: pal[terrain[k]])
        write_rgb(rep / "stage1.png", g.W, g.H, rgb)
        grid_overlay(g, rgb)
        mark_points(g, rgb, [(s["position_m"], b"\xff\xff\xff") for s in site["starts"]]
                    + [(p, b"\xff\xe0\x00" if n.startswith("supply") else b"\x40\xc0\xff") for n, p in site["markers"]])
        write_rgb(rep / "stage1_grid.png", g.W, g.H, rgb)
        lap("report")
    stats["time_s"] = timing
    return stats


def cmd_generate(args):
    site = json.loads(pathlib.Path(args.site).read_text(encoding="utf-8"))
    out = args.out or str(ROOT / "data" / "maps" / site["name"])
    stats = generate(args.site, out, args.report)
    print(json.dumps(stats, indent=1))
    print("wrote %s" % out)


def png_content(data):
    """Header and decompressed image data of a PNG (independent of the zlib
    build that compressed it)."""
    pos, header, idat = 8, b"", b""
    while pos < len(data):
        n = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        if kind in (b"IHDR", b"PLTE"):
            header += data[pos + 8:pos + 8 + n]
        elif kind == b"IDAT":
            idat += data[pos + 8:pos + 8 + n]
        pos += 12 + n
    return header, zlib.decompress(idat)


def cmd_check(args):
    """Generates the map twice from the same sources and compares the files
    byte for byte, then compares the content with the committed map."""
    site = json.loads(pathlib.Path(args.site).read_text(encoding="utf-8"))
    files = ("map.toml", "height.png", "terrain.png", "objects.csv")
    with tempfile.TemporaryDirectory() as tmp:
        runs = []
        for k in range(2):
            out = pathlib.Path(tmp) / str(k)
            generate(args.site, out, None, quiet=True)
            runs.append({f: (out / f).read_bytes() for f in files})
    failed = [f for f in files if runs[0][f] != runs[1][f]]
    if failed:
        raise SystemExit("geo check: two runs differ in %s" % ", ".join(failed))
    committed = ROOT / "data" / "maps" / site["name"]
    for f in files:
        have = (committed / f).read_bytes()
        same = (png_content(have) == png_content(runs[0][f])) if f.endswith(".png") else have == runs[0][f]
        if not same:
            failed.append(f)
    if failed:
        raise SystemExit("geo check: data/maps/%s differs from the sources in %s (run `just gen-geo %s`)"
                         % (site["name"], ", ".join(failed), site["name"]))
    print("geo check: %s generated twice, identical, matches data/maps/%s" % (site["name"], site["name"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("extract")
    p.add_argument("site")
    p.add_argument("--osm", required=True)
    p.add_argument("--dem", required=True)
    p = sub.add_parser("generate")
    p.add_argument("site")
    p.add_argument("--out")
    p.add_argument("--report")
    p = sub.add_parser("check")
    p.add_argument("site")
    args = parser.parse_args()
    {"extract": cmd_extract, "generate": cmd_generate, "check": cmd_check}[args.cmd](args)


if __name__ == "__main__":
    main()
