#!/usr/bin/env python3
"""Maps from real geodata (prompt 081): OpenStreetMap + a free elevation model.

Writes a map folder in the game's format (`map.toml`, `height.png`,
`terrain.png`, `objects.csv`; docs/MAPS.md) from a site description
(`assets-src/geo/<site>/site.json`) and two compact source files next to it:

  osm.json.gz   the OpenStreetMap extract, reduced to geometry and the few
                tags the importer reads (no names, addresses or ids)
  dem.json.gz   a crop of the elevation model in centimeters

Commands:

  extract SITE --osm RAW.json [RAW2.json ...] --dem RAW.tif [RAW2.tif ...]
      reads raw Overpass JSON answers and Copernicus DEM GeoTIFF tiles
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
import subprocess
import sys
import tempfile
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent

# Terrain ids of data/terrain.toml.
GRASS, DIRT, ROAD, SAND, MUD, SNOW, SHALLOW, DEEP, TALL_GRASS = range(9)
FIELD_GREEN = 10

# Prompt 120: in the Mediterranean biome the building classes of a site take
# the Mediterranean family instead of the central European houses (and the
# desert stand-ins older sites used); see also `place_green` and
# `paint_areas`.
MED_BUILDINGS = {
    "env_house_town": "env_house_med_row_corner",
    "env_house_stone": "env_house_med_two",
    "env_house_farm": "env_house_med_casale",
    "env_house_farm_brick": "env_house_med_two",
    "env_house_mud": "env_house_med_small",
    "env_house_log": "env_house_med_small",
    "env_barn": "env_barn_med",
    "env_church": "env_church_med",
    "env_town_hall": "env_town_hall_med",
    "env_house_desert_tall": "env_house_med_row",
    "env_house_desert_large": "env_house_med_casale",
}


def mediterranean(site):
    return site["map"]["biome"] == "mediterranean"


def med_classes(site):
    """Prompt 120: the site's building classes with the Mediterranean family
    (each id once, first place kept)."""
    for name, ids in site["buildings"]["classes"].items():
        out = []
        for k in ids:
            m = MED_BUILDINGS.get(k, k)
            if m not in out:
                out.append(m)
        site["buildings"]["classes"][name] = out

# Tags kept by `extract` (everything else, names first, is dropped).
KEEP_TAGS = (
    "building", "building:levels", "height", "highway", "tunnel", "bridge",
    "layer", "covered", "oneway", "area", "natural", "landuse", "leisure",
    "man_made", "floating", "water", "intermittent", "junction", "start_date",
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

def _clean_tags(tags, extra=()):
    out = {}
    for k in KEEP_TAGS + tuple(extra):
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


def extract_osm(raw_paths, extra_tags=()):
    """One or more Overpass answers (prompt 112: a second, small query);
    elements in several answers are read once. `extra_tags` (prompt 113,
    `sources.extra_tags` of the site) are kept besides KEEP_TAGS."""
    # Coordinates stay decimal text, so 1e-7 degrees are exact.
    answers = [json.loads(pathlib.Path(p).read_text(encoding="utf-8"), parse_float=str) for p in raw_paths]
    data = answers[0]
    elements, seen = [], set()
    for a in answers:
        for e in a["elements"]:
            key = (e["type"], e.get("id"))
            if key not in seen:
                seen.add(key)
                elements.append(e)
    ways, relations = [], []
    for e in elements:
        tags = _clean_tags(e.get("tags", {}), extra_tags)
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
    size = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 11: 4, 12: 8, 16: 8}
    fmt = {1: "B", 2: "s", 3: "H", 4: "I", 5: "I", 11: "f", 12: "d", 16: "Q"}
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
        if typ == 5:
            cnt *= 2   # numerator, denominator
        tags[tag] = raw if typ == 2 else list(struct.unpack(bo + str(cnt) + fmt[typ], raw))
    return bo, tags


class DemTile:
    """One Copernicus DEM GeoTIFF tile (float32, deflate, tiled)."""

    def __init__(self, tif_path):
        buf = pathlib.Path(tif_path).read_bytes()
        bo, t = _tiff_tags(buf)
        self.width, self.height = t[256][0], t[257][0]
        if t[258][0] != 32 or t[259][0] not in (1, 8, 32946) or t[339][0] != 3:
            raise SystemExit("DEM: expected float32, deflate")
        self.predictor = t.get(317, [1])[0]
        self.tw, self.th = t[322][0], t[323][0]
        self.offsets, self.counts = t[324], t[325]
        sx, sy = t[33550][0], t[33550][1]
        self.lon0, self.lat0 = t[33922][3], t[33922][4]
        # Pixels per degree; above 50 degrees latitude Copernicus tiles
        # are twice as wide in longitude (prompt 114).
        self.px = rint(1.0 / sy)
        self.pxx = rint(1.0 / sx)
        self.buf, self.bo = buf, bo
        self.tiles_across = (self.width + self.tw - 1) // self.tw
        self.cache = {}

    def value(self, r, c):
        tw, th = self.tw, self.th
        key = (c // tw, r // th)
        if key not in self.cache:
            k = key[1] * self.tiles_across + key[0]
            raw = zlib.decompress(self.buf[self.offsets[k]:self.offsets[k] + self.counts[k]])
            rows = []
            for rr in range(th):
                row = bytearray(raw[rr * tw * 4:(rr + 1) * tw * 4])
                if self.predictor == 3:
                    for i in range(1, len(row)):
                        row[i] = (row[i] + row[i - 1]) & 0xFF
                    # byte planes (most significant first) to big-endian floats
                    planes = [row[p * tw:(p + 1) * tw] for p in range(4)]
                    be = bytearray(4 * tw)
                    for p in range(4):
                        be[p::4] = planes[p]
                    rows.append(struct.unpack(">%df" % tw, be))
                else:
                    rows.append(struct.unpack(self.bo + "%df" % tw, row))
            self.cache[key] = rows
        return self.cache[key][r % th][c % tw]


def extract_dem(tif_paths, bbox):
    """Crop of one or more neighbouring Copernicus DEM tiles (prompt 112:
    a site across a degree line needs two), in the pixel frame of the
    first tile."""
    tiles = [DemTile(p) for p in tif_paths]
    ref = tiles[0]
    px, pxx, lon0, lat0 = ref.px, ref.pxx, ref.lon0, ref.lat0
    # Pixel offset of every tile in the frame of the first.
    placed = []
    for t in tiles:
        if t.px != px or t.pxx != pxx:
            raise SystemExit("DEM: tiles with different pixel sizes")
        placed.append((t, rint((t.lon0 - lon0) * pxx), rint((lat0 - t.lat0) * px)))
    south, west, north, east = bbox
    c0 = int(math.floor((west / 1e7 - lon0) * pxx)) - 2
    c1 = int(math.floor((east / 1e7 - lon0) * pxx)) + 3
    r0 = int(math.floor((lat0 - north / 1e7) * px)) - 2
    r1 = int(math.floor((lat0 - south / 1e7) * px)) + 3
    c0 = max(c0, min(oc for _, oc, _ in placed))
    r0 = max(r0, min(orow for _, _, orow in placed))
    c1 = min(c1, max(oc + t.width for t, oc, _ in placed))
    r1 = min(r1, max(orow + t.height for t, _, orow in placed))
    values = []
    for r in range(r0, r1):
        for c in range(c0, c1):
            for t, oc, orow in placed:
                if 0 <= c - oc < t.width and 0 <= r - orow < t.height:
                    values.append(rint(t.value(r - orow, c - oc) * 100.0))
                    break
            else:
                raise SystemExit("DEM: pixel (%d, %d) in no tile" % (r, c))
    out = {
        "format": "geo_import dem v1",
        "lon0_deg": rint(lon0), "lat0_deg": rint(lat0), "px_per_deg": px,
        "col0": c0, "row0": r0, "cols": c1 - c0, "rows": r1 - r0,
        "pixel": "area", "unit": "cm", "values": values,
    }
    if pxx != px:
        out["px_per_deg_lon"] = pxx
    return out


# --------------------------------------------------------------------------
# Official terrain models in UTM tiles (prompt 114): DGM1 Schleswig-Holstein

# GRS 80 (ETRS89) and UTM.
GRS80_A = 6378137.0
GRS80_F = 1.0 / 298.257222101
UTM_K0 = 0.9996


def tm_forward(lat, lon, lon0, a=GRS80_A, f=GRS80_F, k0=UTM_K0, fe=500000.0, fn=0.0, lat0=0.0):
    """Transverse Mercator (Snyder 1987, eq. 8-9 to 8-10; series to the
    sixth order): geographic degrees to easting and northing in meters.
    Only + - * / sqrt and the own sine series. Checked against the
    example of EPSG Guidance Note 7-2 (British National Grid: 50.5 N,
    0.5 E gives 577 274.98 / 69 740.49, published 577 274.99 / 69 740.50)."""
    e2 = f * (2.0 - f)
    ep2 = e2 / (1.0 - e2)

    def meridian(deg):
        s2, s4, s6 = cos_sin(2.0 * deg)[1], cos_sin(4.0 * deg)[1], cos_sin(6.0 * deg)[1]
        return a * ((1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2 ** 3 / 256.0) * (deg * PI / 180.0)
                    - (3.0 * e2 / 8.0 + 3.0 * e2 * e2 / 32.0 + 45.0 * e2 ** 3 / 1024.0) * s2
                    + (15.0 * e2 * e2 / 256.0 + 45.0 * e2 ** 3 / 1024.0) * s4
                    - (35.0 * e2 ** 3 / 3072.0) * s6)

    c, s = cos_sin(lat)
    n = a / math.sqrt(1.0 - e2 * s * s)
    tn = s / c
    t = tn * tn
    cc = ep2 * c * c
    aa = (lon - lon0) * PI / 180.0 * c
    x = k0 * n * (aa + (1.0 - t + cc) * aa ** 3 / 6.0
                  + (5.0 - 18.0 * t + t * t + 72.0 * cc - 58.0 * ep2) * aa ** 5 / 120.0)
    y = k0 * (meridian(lat) - meridian(lat0) + n * tn * (
        aa * aa / 2.0 + (5.0 - t + 9.0 * cc + 4.0 * cc * cc) * aa ** 4 / 24.0
        + (61.0 - 58.0 * t + t * t + 600.0 * cc - 330.0 * ep2) * aa ** 6 / 720.0))
    return fe + x, fn + y


def frame_to_geo(g, x, y):
    """The exact inverse of `Grid.to_map`: map meters to degrees."""
    e = x * g.ca + y * g.sa
    n = x * g.sa - y * g.ca
    return g.lat7 / 1e7 + n / g.mlat, g.lon7 / 1e7 + e / g.mlon


def frame_to_utm(g, x, y, zone):
    lat, lon = frame_to_geo(g, x, y)
    return tm_forward(lat, lon, 6.0 * zone - 183.0)


def _cm(text):
    """A decimal height string ("12.34", "-0.5") to integer centimeters."""
    neg = text.startswith("-")
    whole, _, frac = text.lstrip("-+").partition(".")
    v = int(whole or "0") * 100 + int((frac + "00")[:2])
    return -v if neg else v


DGM_NODATA = -999999


def extract_dgm(xyz_paths, g, zone, margin_m, step_m=2):
    """Prompt 114: an official terrain model in 1 m tiles of ETRS89 / UTM
    (DGM1, `x y z` text, one point per 1 m pixel center), reduced to
    `step_m` cells (the mean of the pixels inside, in cm) over the map
    frame plus `margin_m`. Cells farther from the frame stay empty (no
    data), so the file compresses well."""
    corners = []
    for x in range(-rint(g.hx) - margin_m, rint(g.hx) + margin_m + 1, 50):
        for y in range(-rint(g.hy) - margin_m, rint(g.hy) + margin_m + 1, 50):
            corners.append(frame_to_utm(g, x, y, zone))
    e0 = int(math.floor(min(c[0] for c in corners) / step_m)) * step_m
    e1 = int(math.ceil(max(c[0] for c in corners) / step_m)) * step_m
    n0 = int(math.floor(min(c[1] for c in corners) / step_m)) * step_m
    n1 = int(math.ceil(max(c[1] for c in corners) / step_m)) * step_m
    cols, rows = (e1 - e0) // step_m, (n1 - n0) // step_m
    sums = [0] * (cols * rows)
    cnt = bytearray(cols * rows)
    points = 0
    for p in sorted(str(q) for q in xyz_paths):
        with open(p, "r", encoding="ascii") as fh:
            for line in fh:
                parts = line.split()
                if len(parts) != 3:
                    continue
                # Pixel centers lie on .5 m: the integer part is the pixel.
                ex = int(parts[0].partition(".")[0])
                ny = int(parts[1].partition(".")[0])
                c = (ex - e0) // step_m
                r = (n1 - 1 - ny) // step_m      # row 0 in the north
                if 0 <= c < cols and 0 <= r < rows:
                    z = _cm(parts[2])
                    if z <= -900000:            # -9999: no data
                        continue
                    sums[r * cols + c] += z
                    cnt[r * cols + c] += 1
                    points += 1
    # Cells outside the frame and its margin: no data. The frame test runs
    # on the inverse of a local affine fit (only for this mask; meters off
    # at most, far below the margin).
    xc, yc = frame_to_utm(g, 0.0, 0.0, zone)
    ex_, ey_ = frame_to_utm(g, 1000.0, 0.0, zone)
    fx_, fy_ = frame_to_utm(g, 0.0, 1000.0, zone)
    a11, a21 = (ex_ - xc) / 1000.0, (ey_ - yc) / 1000.0
    a12, a22 = (fx_ - xc) / 1000.0, (fy_ - yc) / 1000.0
    det = a11 * a22 - a12 * a21
    reach_x, reach_y = g.hx + margin_m, g.hy + margin_m
    values = []
    for r in range(rows):
        nn = n1 - step_m * r - step_m / 2.0 - yc
        for c in range(cols):
            ee = e0 + step_m * c + step_m / 2.0 - xc
            mx = (a22 * ee - a12 * nn) / det
            my = (-a21 * ee + a11 * nn) / det
            k = r * cols + c
            if cnt[k] and abs(mx) <= reach_x and abs(my) <= reach_y:
                values.append(rint(sums[k] / cnt[k]))
            else:
                values.append(DGM_NODATA)
    return {
        "format": "geo_import dem utm v1",
        "crs": "EPSG:%d" % (25800 + zone), "zone": zone,
        "e0": e0, "n1": n1, "step_m": step_m, "cols": cols, "rows": rows,
        "pixel": "area", "unit": "cm", "nodata": DGM_NODATA,
        "source_points": points, "values": values,
    }


def utm_lattice(g, proj, step):
    """Projected (UTM or RD) coordinates of the map at every `step`-th
    sample (plus the last row and column): exact transformation `proj(x, y)`
    on the lattice, bilinear between (prompt 114; the deviation is measured
    in `utm_lattice_error`)."""
    li = list(range(0, g.W - 1, step)) + [g.W - 1]
    lj = list(range(0, g.H - 1, step)) + [g.H - 1]
    lat = [[proj(g.x(i), g.y(j)) for i in li] for j in lj]
    return li, lj, lat


def utm_lattice_error(g, proj, li, lj, lat):
    """Largest distance (m) between the bilinear lattice and the exact
    transformation at the lattice cell centers, and between one affine
    fit at the center and the exact transformation over the frame."""
    worst = 0.0
    for b in range(len(lj) - 1):
        for a in range(len(li) - 1):
            x = (g.x(li[a]) + g.x(li[a + 1])) / 2.0
            y = (g.y(lj[b]) + g.y(lj[b + 1])) / 2.0
            ex, ny = proj(x, y)
            q = [lat[b][a], lat[b][a + 1], lat[b + 1][a], lat[b + 1][a + 1]]
            bx = sum(p[0] for p in q) / 4.0
            by = sum(p[1] for p in q) / 4.0
            worst = max(worst, math.sqrt((bx - ex) ** 2 + (by - ny) ** 2))
    xc, yc = proj(0.0, 0.0)
    ex_, ey_ = proj(1.0, 0.0)
    fx_, fy_ = proj(0.0, 1.0)
    affine = 0.0
    for x in (-g.hx, 0.0, g.hx):
        for y in (-g.hy, 0.0, g.hy):
            ex, ny = proj(x, y)
            ax = xc + (ex_ - xc) * x + (fx_ - xc) * y
            ay = yc + (ey_ - yc) * x + (fy_ - yc) * y
            affine = max(affine, math.sqrt((ax - ex) ** 2 + (ay - ny) ** 2))
    return worst, affine


def dgm_heights(g, dem, cfg, stats):
    """Prompt 114: ground heights from an official terrain model (no
    buildings or trees in it, so no subtraction, median or opening): every
    map sample is carried to UTM (lattice every 16 samples, bilinear
    between) and the 2 m cells are read bilinearly; cells without data
    count as `nodata_m`. A light box blur (`blur_radius`, `blur_passes`)
    only takes the 1 cm steps and single pixels out."""
    step = dem["step_m"]
    cols, rows, e0, n1 = dem["cols"], dem["rows"], dem["e0"], dem["n1"]
    if dem["format"] == "geo_import dem rd v1":
        # Prompt 116: the Dutch AHN in RD New.
        def proj(x, y):
            return frame_to_rd(g, x, y)
    else:
        zone = dem["zone"]

        def proj(x, y):
            return frame_to_utm(g, x, y, zone)
    nodata = dem["nodata"]
    fill = cfg.get("nodata_m", 0.0)
    v = [fill if x == nodata else x / 100.0 for x in dem["values"]]
    have = bytearray(0 if x == nodata else 1 for x in dem["values"])
    lattice = cfg.get("lattice_samples", 16)
    li, lj, lat = utm_lattice(g, proj, lattice)
    err, affine = utm_lattice_error(g, proj, li, lj, lat)
    stats["dgm"] = {"lattice_error_mm": rint(err * 1000.0), "affine_error_mm": rint(affine * 1000.0),
                    "cells": [cols, rows], "step_m": step}
    h = [fill] * g.N
    missing = 0
    for b in range(len(lj) - 1):
        j0, j1 = lj[b], lj[b + 1]
        for a in range(len(li) - 1):
            i0, i1 = li[a], li[a + 1]
            p00, p10, p01, p11 = lat[b][a], lat[b][a + 1], lat[b + 1][a], lat[b + 1][a + 1]
            last_j = j1 if b == len(lj) - 2 else j1 - 1
            last_i = i1 if a == len(li) - 2 else i1 - 1
            for j in range(j0, last_j + 1):
                ty = (j - j0) / (j1 - j0)
                le = p00[0] + (p01[0] - p00[0]) * ty
                ln = p00[1] + (p01[1] - p00[1]) * ty
                re = p10[0] + (p11[0] - p10[0]) * ty
                rn = p10[1] + (p11[1] - p10[1]) * ty
                base = j * g.W
                for i in range(i0, last_i + 1):
                    tx = (i - i0) / (i1 - i0)
                    ee = le + (re - le) * tx
                    nn = ln + (rn - ln) * tx
                    fc = (ee - e0) / step - 0.5
                    fr = (n1 - nn) / step - 0.5
                    ic, ir = int(math.floor(fc)), int(math.floor(fr))
                    if ic < 0 or ir < 0 or ic >= cols - 1 or ir >= rows - 1:
                        missing += 1
                        continue
                    tc, tr = fc - ic, fr - ir
                    q = ir * cols + ic
                    if have[q] and have[q + 1] and have[q + cols] and have[q + cols + 1]:
                        top = v[q] + (v[q + 1] - v[q]) * tc
                        bot = v[q + cols] + (v[q + cols + 1] - v[q + cols]) * tc
                        h[base + i] = top + (bot - top) * tr
                    else:
                        near = [v[k] for k in (q, q + 1, q + cols, q + cols + 1) if have[k]]
                        if near:
                            h[base + i] = sum(near) / len(near)
                        else:
                            missing += 1
    stats["dgm"]["samples_without_data"] = missing
    if cfg.get("blur_radius", 0) > 0:
        h = blur(g, h, cfg["blur_radius"], cfg.get("blur_passes", 1))
    return h


# --------------------------------------------------------------------------
# The Dutch terrain model AHN in RD New (prompt 116)

# Schreutelkamp and Strang van Hees (2001): ETRS89 degrees to RD New
# (EPSG:28992) as a polynomial in 0.36 * (degrees - reference), about
# 0.25 m off the official RDNAPTRANS (below the 2 m sample spacing).
RD_LAT0, RD_LON0, RD_X0, RD_Y0 = 52.15517440, 5.38720621, 155000.0, 463000.0
RD_R = ((0, 1, 190094.945), (1, 1, -11832.228), (2, 1, -114.221), (0, 3, -32.391), (1, 0, -0.705),
        (3, 1, -2.340), (1, 3, -0.608), (0, 2, -0.008), (2, 3, 0.148))
RD_S = ((1, 0, 309056.544), (0, 2, 3638.893), (2, 0, 73.077), (1, 2, -157.984), (3, 0, 59.788),
        (0, 1, 0.433), (2, 2, -6.439), (1, 1, -0.032), (0, 4, 0.092), (1, 4, -0.054))


def wgs_to_rd(lat, lon):
    """Degrees (ETRS89, as OSM) to RD New meters; only + - * (powers by
    repeated multiplication), so it is exactly reproducible. Checked: the
    reference point gives (155 000, 463 000)."""
    dp = 0.36 * (lat - RD_LAT0)
    dl = 0.36 * (lon - RD_LON0)
    pp = [1.0, dp, dp * dp, dp * dp * dp]
    pl = [1.0, dl, dl * dl, dl * dl * dl, dl * dl * dl * dl]
    x = RD_X0
    for p, q, c in RD_R:
        x += c * pp[p] * pl[q]
    y = RD_Y0
    for p, q, c in RD_S:
        y += c * pp[p] * pl[q]
    return x, y


def frame_to_rd(g, x, y):
    lat, lon = frame_to_geo(g, x, y)
    return wgs_to_rd(lat, lon)


class StripTiff:
    """A GeoTIFF of the AHN coverage service (float32, deflate, strips,
    floating point predictor; PDOK WCS `dtm_05m`), georeferenced in RD."""

    def __init__(self, path):
        buf = pathlib.Path(path).read_bytes()
        bo, t = _tiff_tags(buf)
        self.width, self.height = t[256][0], t[257][0]
        if t[258][0] != 32 or t[259][0] not in (1, 8, 32946) or t[339][0] != 3:
            raise SystemExit("AHN: expected float32, deflate (%s)" % path)
        if 322 in t:
            raise SystemExit("AHN: expected strips, not tiles (%s)" % path)
        self.predictor = t.get(317, [1])[0]
        self.rows_per_strip = t[278][0]
        self.offsets, self.counts = t[273], t[279]
        self.res = t[33550][0]
        if t[33550][1] != self.res:
            raise SystemExit("AHN: square pixels expected")
        self.x0, self.y0 = t[33922][3], t[33922][4]
        nd = t.get(42113, b"").rstrip(b"\x00").decode() or "nan"
        self.nodata = float(nd)
        self.compressed = t[259][0] != 1
        self.buf, self.bo = buf, bo

    def rows(self):
        """Yields (row, values) for every row, north first."""
        w = self.width
        for k, (off, n) in enumerate(zip(self.offsets, self.counts)):
            raw = self.buf[off:off + n]
            if self.compressed:
                raw = zlib.decompress(raw)
            for rr in range(len(raw) // (4 * w)):
                row = raw[rr * 4 * w:(rr + 1) * 4 * w]
                if self.predictor == 3:
                    acc = bytearray(len(row))
                    a = 0
                    for i, b in enumerate(row):
                        a = (a + b) & 0xFF
                        acc[i] = a
                    be = bytearray(4 * w)
                    for p in range(4):
                        be[p::4] = acc[p * w:(p + 1) * w]
                    vals = struct.unpack(">%df" % w, be)
                else:
                    vals = struct.unpack(self.bo + "%df" % w, row)
                yield k * self.rows_per_strip + rr, vals


def extract_ahn(tif_paths, g, margin_m, step_m=2):
    """Prompt 116: the Dutch terrain model AHN (DTM, 0.5 m, fetched from the
    PDOK coverage service in tiles at 1 m) reduced to `step_m` cells of RD
    New (the mean of the pixels inside, in cm) over the map frame plus
    `margin_m`. The DTM has no data under buildings and over water; those
    cells are filled ring by ring from their neighbours with data (the
    mean of the 8 neighbours filled before), so the ground runs on under
    the houses; water is shaped later from OSM. Cells farther from the
    frame stay empty."""
    corners = []
    for x in range(-rint(g.hx) - margin_m, rint(g.hx) + margin_m + 1, 50):
        for y in range(-rint(g.hy) - margin_m, rint(g.hy) + margin_m + 1, 50):
            corners.append(frame_to_rd(g, x, y))
    e0 = int(math.floor(min(c[0] for c in corners) / step_m)) * step_m
    e1 = int(math.ceil(max(c[0] for c in corners) / step_m)) * step_m
    n0 = int(math.floor(min(c[1] for c in corners) / step_m)) * step_m
    n1 = int(math.ceil(max(c[1] for c in corners) / step_m)) * step_m
    cols, rows = (e1 - e0) // step_m, (n1 - n0) // step_m
    sums = [0] * (cols * rows)
    cnt = [0] * (cols * rows)
    points = 0
    for p in sorted(str(q) for q in tif_paths):
        t = StripTiff(p)
        for r, vals in t.rows():
            ny = t.y0 - (r + 0.5) * t.res
            rr = int(math.floor((n1 - ny) / step_m))
            if not 0 <= rr < rows:
                continue
            base = rr * cols
            for c, z in enumerate(vals):
                if z != z or z == t.nodata or z > 1000.0 or z < -100.0:
                    continue
                ex = t.x0 + (c + 0.5) * t.res
                cc = int(math.floor((ex - e0) / step_m))
                if 0 <= cc < cols:
                    sums[base + cc] += rint(z * 100.0)
                    cnt[base + cc] += 1
                    points += 1
    # Inside the frame and its margin (inverse of a local affine fit, as
    # for the UTM tiles).
    xc, yc = frame_to_rd(g, 0.0, 0.0)
    ex_, ey_ = frame_to_rd(g, 1000.0, 0.0)
    fx_, fy_ = frame_to_rd(g, 0.0, 1000.0)
    a11, a21 = (ex_ - xc) / 1000.0, (ey_ - yc) / 1000.0
    a12, a22 = (fx_ - xc) / 1000.0, (fy_ - yc) / 1000.0
    det = a11 * a22 - a12 * a21
    reach_x, reach_y = g.hx + margin_m, g.hy + margin_m
    inside = bytearray(cols * rows)
    values = [DGM_NODATA] * (cols * rows)
    for r in range(rows):
        nn = n1 - step_m * r - step_m / 2.0 - yc
        for c in range(cols):
            ee = e0 + step_m * c + step_m / 2.0 - xc
            mx = (a22 * ee - a12 * nn) / det
            my = (-a21 * ee + a11 * nn) / det
            k = r * cols + c
            if abs(mx) <= reach_x and abs(my) <= reach_y:
                inside[k] = 1
                if cnt[k]:
                    values[k] = rint(sums[k] / cnt[k])
    # Fill the gaps ring by ring.
    def neighbours(k):
        r, c = k // cols, k % cols
        for dr in (-1, 0, 1):
            for dc in (-1, 0, 1):
                if (dr or dc) and 0 <= r + dr < rows and 0 <= c + dc < cols:
                    yield k + dr * cols + dc
    frontier = sorted({q for k in range(cols * rows) if inside[k] and values[k] == DGM_NODATA
                       for q in [k] if any(values[n] != DGM_NODATA for n in neighbours(k))})
    filled, rings = 0, 0
    while frontier:
        new = []
        for k in frontier:
            near = [values[n] for n in neighbours(k) if values[n] != DGM_NODATA]
            new.append((k, rint(sum(near) / len(near))))
        for k, v in new:
            values[k] = v
        filled += len(new)
        rings += 1
        nxt = set()
        for k, _ in new:
            for n in neighbours(k):
                if inside[n] and values[n] == DGM_NODATA:
                    nxt.add(n)
        frontier = sorted(nxt)
    return {
        "format": "geo_import dem rd v1",
        "crs": "EPSG:28992",
        "e0": e0, "n1": n1, "step_m": step_m, "cols": cols, "rows": rows,
        "pixel": "area", "unit": "cm", "nodata": DGM_NODATA,
        "source_points": points, "filled_cells": filled, "fill_rings": rings, "values": values,
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
    osm = extract_osm(args.osm, site["sources"].get("extra_tags", ()))
    osm["query"] = site["sources"]["osm_query"] + site["sources"].get("osm_query_extra", "")
    write_gz_json(site_path.parent / "osm.json.gz", osm)
    if args.dgm:
        # Prompt 114: an official terrain model in UTM tiles.
        src = site["sources"]
        dem = extract_dgm(args.dgm, Grid(site["frame"]), src["dgm_zone"], src.get("dgm_margin_m", 100))
    elif args.ahn:
        # Prompt 116: the Dutch AHN in RD New.
        dem = extract_ahn(args.ahn, Grid(site["frame"]), site["sources"].get("dgm_margin_m", 100))
    else:
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


def dilate_round(g, rows, n):
    """`n` steps of one sample, alternating square and cross: an octagon
    instead of a square (prompt 114: wide strips along a curved shore)."""
    full = (1 << g.W) - 1
    for step in range(n):
        if step % 2 == 0:
            rows = dilate(g, rows, 1)
        else:
            out = []
            for j in range(g.H):
                a = rows[j] | (rows[j] << 1) | (rows[j] >> 1)
                if j > 0:
                    a |= rows[j - 1]
                if j < g.H - 1:
                    a |= rows[j + 1]
                out.append(a & full)
            rows = out
    return rows


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
    if "clamp_m" in cfg:
        # Prompt 112: in a dense city the surface model is unreliable
        # (buildings over 30 m pixels, tall towers subtracted too deep); the
        # ground is held within the site's known range before the blur.
        lo, hi = cfg["clamp_m"]
        ground = [min(max(v, lo), hi) for v in ground]
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
            elif t.get("man_made") in ("pier", "breakwater", "quay"):
                # Multipolygon piers (prompt 112): their outer rings as land.
                if t.get("floating") != "yes":
                    self.land_add += [[g.pts(x)] for x in r["outer"]]
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


def start_year(t):
    """The year of `start_date` (OSM), or None."""
    text = t.get("start_date", "").strip().lstrip("~")
    if len(text) >= 4 and text[:4].isdigit():
        return int(text[:4])
    return None


def tagged_height(t, cfg):
    """Height (m) of a footprint from its tags for the building's choice,
    or None (the district estimates it). Prompt 112: a footprint built in
    or after `built_before_year` (OSM `start_date`) or taller than
    `max_tagged_height_m` counts as untagged, so a building of the
    district's estimate stands there (the state of 1944)."""
    v = building_height(t, None)
    if v is None:
        return None
    if "max_tagged_height_m" in cfg and v > cfg["max_tagged_height_m"]:
        return None
    if "built_before_year" in cfg:
        year = start_year(t)
        if year is not None and year >= cfg["built_before_year"]:
            return None
    return v


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
    # Prompt 116: a river site has no sea (no seeds); its water comes from
    # the OSM water areas (`lakes_min_m2`).
    if seeds and min(seeds) < 0:
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
    if "lakes_min_m2" in cfg:
        # Prompt 113: inland water (a lagoon behind a beach) is not reached
        # from the sea seeds; closed `natural=water` areas of at least
        # `lakes_min_m2` (not intermittent) are water too.
        lakes = 0
        skip_at = cfg.get("skip_water_at_m", [])
        for t, rings in f.areas:
            if skip_at:
                # Prompt 116: water dug after 1944, named by the mean of its
                # outer ring's points (within 10 m).
                pts = rings[0]
                cx = sum(p[0] for p in pts) / len(pts)
                cy = sum(p[1] for p in pts) / len(pts)
                if any(abs(cx - a) <= 10.0 and abs(cy - b) <= 10.0 for a, b in skip_at):
                    stats["water_skipped"] = stats.get("water_skipped", 0) + 1
                    continue
            if (t.get("natural") == "water" and t.get("intermittent", "no") == "no"
                    and area_of(rings) >= cfg["lakes_min_m2"]):
                for j, i0, i1 in polygon_spans(g, rings):
                    water[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
                lakes += 1
        stats["lakes"] = lakes
    if "dry_polygons_m" in cfg:
        # Prompt 116: water dug after 1944 (a side channel) is land again.
        dried = 0
        for d in cfg["dry_polygons_m"]:
            for j, i0, i1 in polygon_spans(g, [d["polygon_m"]]):
                for k in range(j * g.W + i0, j * g.W + i1 + 1):
                    if water[k]:
                        water[k] = 0
                        dried += 1
        stats["dried_m2"] = dried * 4
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


def land_strip(g, strip, h, terrain, water):
    """Prompt 116: a dam across narrow water where a road crosses it (the
    small bridges of the place are not in the game): within `half_m` of
    the line `pts_m` the water becomes grass at `height_m`."""
    (ax, ay), (bx, by) = strip["pts_m"]
    r = strip["half_m"]
    z = strip["height_m"]
    dx, dy = bx - ax, by - ay
    ll = dx * dx + dy * dy
    for j in range(g.H):
        y = g.y(j)
        if y < min(ay, by) - r - 2 or y > max(ay, by) + r + 2:
            continue
        for i in range(g.W):
            x = g.x(i)
            if x < min(ax, bx) - r - 2 or x > max(ax, bx) + r + 2:
                continue
            t = ((x - ax) * dx + (y - ay) * dy) / ll
            t = 0.0 if t < 0.0 else (1.0 if t > 1.0 else t)
            px, py = ax + t * dx - x, ay + t * dy - y
            if px * px + py * py <= r * r:
                k = j * g.W + i
                if water[k] or h[k] < z:
                    water[k] = 0
                    h[k] = z
                    terrain[k] = GRASS


def tidal_flats(g, f, cfg, water, stats):
    """Prompt 114: the tidal flats (OSM `wetland=tidalflat` over the sea)
    have no tides in the game. Water inside them within `mud_m` of the
    land becomes mud flat (land); the next `shallow_m` stay shallow
    water. Returns the mud and the shallow masks; `water` loses the mud."""
    t = cfg["tidal_flat"]
    tidal = bytearray(g.N)
    for tags, rings in f.areas:
        if tags.get("wetland") == "tidalflat":
            for j, i0, i1 in polygon_spans(g, rings):
                tidal[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    land = rows_not(g, to_rows(g, water))
    near_mud = to_mask(g, dilate_round(g, land, t["mud_m"] // 2))
    mud = bytearray(1 if (water[k] and tidal[k] and near_mud[k]) else 0 for k in range(g.N))
    for k in range(g.N):
        if mud[k]:
            water[k] = 0
    land = rows_not(g, to_rows(g, water))
    near_shallow = to_mask(g, dilate_round(g, land, t["shallow_m"] // 2))
    shallow = bytearray(1 if (water[k] and tidal[k] and near_shallow[k]) else 0 for k in range(g.N))
    stats["tidal_flat"] = {"mud_m2": count(mud) * 4, "shallow_m2": count(shallow) * 4}
    return mud, shallow


def shape_tidal_flats(g, cfg, h, terrain, mud, shallow, water):
    """Mud from `mud_top_m` at the shore down to `mud_edge_m` at its outer
    edge (by rings from the open water), the shallow seam beyond it
    `shallow_terrain`, its bed from -0.3 m down to the sea bed."""
    t = cfg["tidal_flat"]
    top, edge = t["mud_top_m"], t["mud_edge_m"]
    rings = t["mud_m"] // 2
    prev = rows_not(g, to_rows(g, water))
    level = 0
    seen = bytearray(g.N)
    while level < rings:
        nxt = erode(g, prev, 1)
        for k in set_bits(g, [a & ~b for a, b in zip(prev, nxt)]):
            if mud[k] and not seen[k]:
                seen[k] = 1
                h[k] = edge + (top - edge) * level / rings
        prev = nxt
        level += 1
    for k in range(g.N):
        if mud[k]:
            if not seen[k]:
                h[k] = top
            terrain[k] = MUD
        elif shallow[k]:
            terrain[k] = SHALLOW
            h[k] = max(h[k], -0.4)


# --------------------------------------------------------------------------
# Roads

def select_roads(g, f, cfg, stats, water=None):
    """The carrying network: classes of `cfg.classes`, without tunnels,
    steps, foot and cycle ways, service roads; simplified lines, the upper
    of two crossing roads dropped, short dead ends and tiny fragments
    removed. Returns [(pts, keys, half_width, terrain)].

    Prompt 113 (options): ways mostly inside `skip_polygons_m` (streets
    built later) are dropped; with `drop_bridges_over_water` so are bridges
    over the water (a strait the game cannot span); `extra` adds tracks and
    ramps of the site (map meters) after the simplification."""
    skip_polys = cfg.get("skip_polygons_m", [])
    minor_polys = cfg.get("minor_skip_polygons_m", [])
    minor_classes = cfg.get("minor_classes", ["residential", "living_street", "unclassified", "track"])
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
        bridge = t.get("bridge", "no") != "no" or layer > 0
        if skip_polys and 2 * sum(1 for x, y in pts if any(in_polygon(x, y, q) for q in skip_polys)) > len(pts):
            drop("built later (site polygons)", pts)
        elif (minor_polys and kind in minor_classes
              and 2 * sum(1 for x, y in pts if any(in_polygon(x, y, q) for q in minor_polys)) > len(pts)):
            # Prompt 116: the minor streets of a new town (the village of
            # 1944 had only its main roads there).
            drop("minor streets built later (site polygons)", pts)
        elif (bridge and cfg.get("drop_bridges_over_water") and water is not None
              and kind in classes and _water_share(g, pts, water) >= 0.3
              and not ("causeway_max_m" in cfg and length(pts) <= cfg["causeway_max_m"])):
            drop("bridges over water", pts)
        elif t.get("tunnel", "no") != "no" or t.get("covered", "no") != "no" or layer < 0:
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
    for n, e in enumerate(cfg.get("extra", [])):
        pts = [tuple(float(v) for v in p) for p in e["pts_m"]]
        kept.append({"pts": pts, "keys": [("extra", n, k) for k in range(len(pts))], "hw": e["width_m"] / 2.0,
                     "terrain": {"road": ROAD, "dirt_track": DIRT}[e.get("terrain", "dirt_track")], "bridge": False})
    stats["roads"] = {
        "osm_km_in_map": round(before_km, 1),
        "kept_ways": len(kept),
        "kept_km_in_map": round(sum(inside_length(g, w["pts"]) for w in kept) / 1000.0, 1),
        "points_before": points_before,
        "points_after": sum(len(w["pts"]) for w in kept),
        "dropped": {k: [n, round(km, 2)] for k, (n, km) in sorted(dropped.items())},
    }
    if cfg.get("extra"):
        stats["roads"]["extra_ways"] = len(cfg["extra"])
        stats["roads"]["extra_km"] = round(sum(length(w["pts"]) for w in kept[-len(cfg["extra"]):]) / 1000.0, 2)
    return kept


def build_causeways(g, roads, cfg, h, terrain, water, stats):
    """Prompt 116: where a kept road crosses water no wider than
    `causeway_max_m` along its line (a ditch, an old river arm; in the
    low country small bridges and dams with culverts), the water under the
    road and `causeway_side_m` beside it becomes land at the height of the
    banks, so the road runs on (the game has no small bridges from OSM)."""
    maxw = cfg["causeway_max_m"]
    side = cfg.get("causeway_side_m", 2.0)
    made, cells = 0, 0
    for w in roads:
        pts = w["pts"]
        dense = [pts[0]]
        for k in range(len(pts) - 1):
            (ax, ay), (bx, by) = pts[k], pts[k + 1]
            n = max(1, int(math.ceil(math.sqrt((bx - ax) ** 2 + (by - ay) ** 2) / 2.0)))
            for t in range(1, n + 1):
                dense.append((ax + (bx - ax) * t / n, ay + (by - ay) * t / n))
        idx = [g.index(x, y) for x, y in dense]
        wet = [1 if (q >= 0 and water[q]) else 0 for q in idx]
        k = 0
        while k < len(dense):
            if not wet[k]:
                k += 1
                continue
            a = k
            while k < len(dense) and wet[k]:
                k += 1
            b = k - 1
            # A short bridge way (a separate OSM way over the water) may
            # start or end in the water: its line runs on to the bank.
            line = list(dense[max(a - 1, 0):b + 2])
            for end in (0, 1):
                p0 = line[0] if end == 0 else line[-1]
                p1 = line[1] if end == 0 else line[-2]
                q0 = g.index(*p0)
                if q0 >= 0 and not water[q0]:
                    continue
                dx, dy = p0[0] - p1[0], p0[1] - p1[1]
                dl = math.sqrt(dx * dx + dy * dy) or 1.0
                found = None
                for st in range(1, 16):
                    c = (p0[0] + dx / dl * 2.0 * st, p0[1] + dy / dl * 2.0 * st)
                    q = g.index(*c)
                    if q < 0:
                        break
                    if not water[q]:
                        found = c
                        break
                if found is None:
                    line = None
                    break
                if end == 0:
                    line.insert(0, found)
                else:
                    line.append(found)
            if line is None:
                continue
            run = 0.0
            for m in range(len(line) - 1):
                run += math.sqrt((line[m + 1][0] - line[m][0]) ** 2 + (line[m + 1][1] - line[m][1]) ** 2)
            if run > maxw:
                continue
            za, zb = h[g.index(*line[0])], h[g.index(*line[-1])]
            r = w["hw"] + side
            for m in range(len(line)):
                x, y = line[m]
                z = za + (zb - za) * m / (len(line) - 1)
                i0 = max(0, int(math.floor((x - r + g.hx) / 2.0)))
                i1 = min(g.W - 1, int(math.ceil((x + r + g.hx) / 2.0)))
                j0 = max(0, int(math.floor((y - r + g.hy) / 2.0)))
                j1 = min(g.H - 1, int(math.ceil((y + r + g.hy) / 2.0)))
                for j in range(j0, j1 + 1):
                    for i in range(i0, i1 + 1):
                        q = j * g.W + i
                        if water[q] and (g.x(i) - x) ** 2 + (g.y(j) - y) ** 2 <= r * r:
                            water[q] = 0
                            h[q] = z
                            terrain[q] = GRASS
                            cells += 1
            made += 1
    stats["causeways"] = {"count": made, "m2": cells * 4}


def _water_share(g, pts, water):
    samples = line_samples(g, pts)
    return sum(1 for k in samples if water[k]) / len(samples) if samples else 0.0


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
# Long bridges over water (prompt 115)

# Length (m) of the bridge pieces a site may chain; keep in step with
# data/objects.toml (the map check rejects pieces whose ends do not meet).
BRIDGE_PIECES = {
    ("env_bridge_steel_arch", "Arch244"): 244.0,
    ("env_bridge_steel_arch", "Arch120"): 120.0,
    ("env_bridge_steel_approach", "Span60"): 60.0,
    ("env_bridge_steel_approach", "Span30"): 30.0,
    ("env_bridge_stone", "Arch1"): 20.0,
    ("env_bridge_stone", "Arch2"): 20.0,
    ("env_bridge_stone", "Arch3"): 30.0,
}


def long_bridge_layout(g, b):
    """Pieces of a long bridge of the site (`long_bridges` in site.json):
    [(kind, variant, x, y, heading)], its outer ends along the axis
    (s0, s1) and the axis line. The axis runs along map x (`axis` "x",
    heading 90) or y ("y", heading 180) at `line_m`, which must lie on a
    center line of the 4 m path cells (docs/MAPS.md, "Bridges"); the
    pieces follow each other from `start_m` in the positive direction."""
    axis = b["axis"]
    if axis not in ("x", "y"):
        raise SystemExit("long bridge %s: axis must be \"x\" or \"y\"" % b["name"])
    line = float(b["line_m"])
    half_other = g.hy if axis == "x" else g.hx
    # Path cell centers lie at -half + 1 + 4 k (2 m samples).
    k = (line + half_other - 1.0) / 4.0
    if abs(k - round(k)) > 1e-6:
        raise SystemExit("long bridge %s: line_m %.1f is off the 4 m path grid; use %.1f or %.1f"
                         % (b["name"], line, -half_other + 1 + 4 * math.floor(k), -half_other + 1 + 4 * math.ceil(k)))
    heading = 90 if axis == "x" else 180
    s = float(b["start_m"])
    pieces = []
    for kind, variant in b["pieces"]:
        length = BRIDGE_PIECES.get((kind, variant))
        if length is None:
            raise SystemExit("long bridge %s: unknown piece %s %s" % (b["name"], kind, variant))
        c = s + length / 2.0
        x, y = (c, line) if axis == "x" else (line, c)
        pieces.append((kind, variant, x, y, heading))
        s += length
    return pieces, (float(b["start_m"]), s), line


def build_long_bridges(g, site, h, terrain, water, reserved, stats):
    """Places the long bridges of the site: the pieces end to end (rows for
    objects.csv, written before everything else), an earth dam at each
    outer end up to `deck_m` (abutment `abutment_m` under the deck, then
    falling at `ramp_grade` until it meets the ground; crest
    `crest_half_m` either side of the axis, sides at `side_grade`) with
    a road on its crest. The ground under the bridge and the dams is kept
    free of buildings. No underpass: roads under the approach spans and
    across the dams end there (counted in the stats); a site routes them
    round the dam ends with `roads.extra`. Returns the rows."""
    rows = []
    report = []
    for b in site.get("long_bridges", []):
        pieces, (s0, s1), line = long_bridge_layout(g, b)
        for kind, variant, x, y, heading in pieces:
            rows.append("%s,%s,%d,%d,%d" % (kind, variant, rint(x * 1000.0), rint(y * 1000.0), heading))
        deck = float(b["deck_m"])
        grade = float(b.get("ramp_grade", 0.12))
        crest = float(b.get("crest_half_m", 10.0))
        side = float(b.get("side_grade", 1.0))
        abut = float(b.get("abutment_m", 8.0))
        road_half = float(b.get("road_half_m", 5.0))
        alongx = b["axis"] == "x"
        reach = (deck - min(h)) / grade + 2.0
        raised, road_cut, padded = 0, 0, 0
        for end, out in ((s0, -1.0), (s1, 1.0)):
            if b.get("level_high_ends"):
                # Prompt 116: where the bank is higher than the dam (a town
                # on a terrace), the deck ends on that ground; level a pad
                # beyond the end (`end_pad_m` long, `end_pad_half_m` either
                # side of the axis, reaching `end_pad_under_m` under the
                # deck) so the last cells are not a slope.
                ex, ey = (end, line) if alongx else (line, end)
                z = bilinear(g, h, ex, ey)
                if z > deck:
                    r = b.get("end_pad_half_m", crest)
                    run = b.get("end_pad_m", abut)
                    under = b.get("end_pad_under_m", abut)
                    lo, hi = sorted((end - out * under, end + out * run))
                    a0, a1 = (lo, hi) if alongx else (line - r, line + r)
                    b0, b1 = (line - r, line + r) if alongx else (lo, hi)
                    for j in range(max(0, int(math.floor((b0 + g.hy) / 2.0))), min(g.H - 1, int(math.ceil((b1 + g.hy) / 2.0))) + 1):
                        for i in range(max(0, int(math.floor((a0 + g.hx) / 2.0))), min(g.W - 1, int(math.ceil((a1 + g.hx) / 2.0))) + 1):
                            x, y = g.x(i), g.y(j)
                            along, across = (x, y) if alongx else (y, x)
                            if -under <= (along - end) * out <= run and abs(across - line) <= r:
                                idx = j * g.W + i
                                h[idx] = z
                                water[idx] = 0
                                if abs(across - line) <= road_half:
                                    terrain[idx] = ROAD
                                reserved[idx] = 1
                                padded += 1
            lo, hi = sorted((end - out * abut, end + out * reach))
            wide = crest + deck / side + 2.0
            a0, a1 = (lo, hi) if alongx else (line - wide, line + wide)
            b0, b1 = (line - wide, line + wide) if alongx else (lo, hi)
            i0, i1 = max(0, int(math.floor((a0 + g.hx) / 2.0))), min(g.W - 1, int(math.ceil((a1 + g.hx) / 2.0)))
            j0, j1 = max(0, int(math.floor((b0 + g.hy) / 2.0))), min(g.H - 1, int(math.ceil((b1 + g.hy) / 2.0)))
            for j in range(j0, j1 + 1):
                for i in range(i0, i1 + 1):
                    x, y = g.x(i), g.y(j)
                    along, across = (x, y) if alongx else (y, x)
                    t = (along - end) * out
                    if t < -abut:
                        continue
                    top = deck - max(t, 0.0) * grade - max(abs(across - line) - crest, 0.0) * side
                    idx = j * g.W + i
                    if top <= h[idx]:
                        continue
                    if terrain[idx] == ROAD and abs(across - line) > road_half:
                        road_cut += 1
                    h[idx] = top
                    water[idx] = 0
                    terrain[idx] = ROAD if abs(across - line) <= road_half else GRASS
                    reserved[idx] = 1
                    raised += 1
        # Under the bridge: no buildings, and count the roads that end there.
        for j in range(g.H):
            y = g.y(j)
            for i in range(g.W):
                x = g.x(i)
                along, across = (x, y) if alongx else (y, x)
                if s0 <= along <= s1 and abs(across - line) <= crest + 2.0:
                    idx = j * g.W + i
                    reserved[idx] = 1
                    if terrain[idx] == ROAD and not water[idx]:
                        road_cut += 1
        report.append({"name": b["name"], "pieces": len(pieces), "length_m": round(s1 - s0, 1),
                       "deck_m": deck, "dam_m2": raised * 4, "road_under_or_cut_m2": road_cut * 4})
        if b.get("level_high_ends"):
            report[-1]["end_pads_m2"] = padded * 4
    stats["long_bridges"] = report
    return rows


def drop_bridge_ways(g, f, site, stats):
    """OSM ways tagged as a bridge that run along a long bridge of the site
    (mostly within its corridor) are not painted: the bridge objects carry
    the road (prompt 115)."""
    corridors = []
    for b in site.get("long_bridges", []):
        _, (s0, s1), line = long_bridge_layout(g, b)
        corridors.append((b["axis"] == "x", s0 - 2.0, s1 + 2.0, line, float(b.get("crest_half_m", 10.0)) + 4.0))
    if not corridors:
        return
    keep, dropped = [], 0
    for t, pts, keys in f.highways:
        bridge = t.get("bridge", "no") != "no"
        inside = 0
        for x, y in pts:
            for alongx, a0, a1, line, half in corridors:
                along, across = (x, y) if alongx else (y, x)
                if a0 <= along <= a1 and abs(across - line) <= half:
                    inside += 1
                    break
        if bridge and 2 * inside >= len(pts):
            dropped += 1
        else:
            keep.append((t, pts, keys))
    f.highways = keep
    stats["long_bridge_ways_dropped"] = dropped


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
# Buildings (stage 2)

def catalog_sizes(ids):
    """Length, width and height (m) of neutral catalog entries."""
    sizes, current = {}, None
    for line in (ROOT / "data" / "catalog" / "neutral.toml").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("id ="):
            current = line.split('"')[1]
            if current in ids:
                sizes[current] = [0, 0, 0]
        elif current in sizes:
            for k, key in enumerate(("length_mm", "width_mm", "height_mm")):
                if line.startswith(key + " ="):
                    sizes[current][k] = int(line.split("=")[1].split("#")[0].strip().replace("_", "")) / 1000.0
    missing = [i for i in ids if i not in sizes]
    if missing:
        raise SystemExit("catalog: no size for %s" % ", ".join(missing))
    return sizes


def in_polygon(x, y, poly):
    inside = False
    n = len(poly)
    for k in range(n):
        (x1, y1), (x2, y2) = poly[k], poly[(k + 1) % n]
        if (y1 > y) != (y2 > y):
            if x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                inside = not inside
    return inside


def district_of(site, x, y):
    for d in site["districts"]:
        if in_polygon(x, y, d["polygon_m"]):
            return d
    return site["districts"][-1]


class Placer:
    """Occupancy of the map for objects: boxes and trunks on the 2 m grid,
    with the masks they must keep off (roads, water, reserved places)."""

    def __init__(self, g, road, water, reserved):
        self.g = g
        self.road = road
        self.water = water
        self.reserved = reserved
        self.occupied = bytearray(g.N)
        self.rows = []

    def box(self, cx, cy, heading, length, width, margin):
        """Samples within the box (grown by margin)."""
        return Placer.box_samples(self.g, cx, cy, heading, length, width, margin)

    @staticmethod
    def box_samples(g, cx, cy, heading, length, width, margin):
        c, s = cos_sin(heading)
        ux, uy = s, -c          # along the heading (length)
        vx, vy = c, s           # right (width)
        hl, hw = length / 2.0 + margin, width / 2.0 + margin
        ext = abs(ux) * hl + abs(vx) * hw, abs(uy) * hl + abs(vy) * hw
        i0 = max(0, int(math.floor((cx - ext[0] + g.hx) / 2.0)))
        i1 = min(g.W - 1, int(math.ceil((cx + ext[0] + g.hx) / 2.0)))
        j0 = max(0, int(math.floor((cy - ext[1] + g.hy) / 2.0)))
        j1 = min(g.H - 1, int(math.ceil((cy + ext[1] + g.hy) / 2.0)))
        out = []
        for j in range(j0, j1 + 1):
            dy = g.y(j) - cy
            for i in range(i0, i1 + 1):
                dx = g.x(i) - cx
                if abs(dx * ux + dy * uy) <= hl and abs(dx * vx + dy * vy) <= hw:
                    out.append(j * g.W + i)
        return out

    def inside_map(self, cx, cy, heading, length, width):
        c, s = cos_sin(heading)
        for a in (-0.5, 0.5):
            for b in (-0.5, 0.5):
                x = cx + s * length * a + c * width * b
                y = cy - c * length * a + s * width * b
                if not self.g.inside(x, y, -2.0):
                    return False
        return True

    def free(self, samples, road=True, occupied=True):
        for k in samples:
            if (occupied and self.occupied[k]) or self.water[k] or self.reserved[k] or (road and self.road[k]):
                return False
        return True

    def take(self, samples):
        for k in samples:
            self.occupied[k] = 1

    def row(self, kind, variant, x, y, heading):
        self.rows.append("%s,%s,%d,%d,%d" % (kind, variant, rint(x * 1000.0), rint(y * 1000.0), heading % 360))


def footprint_classes(t):
    b = t.get("building", "yes")
    if b in ("church", "chapel", "cathedral") or t.get("amenity") == "place_of_worship":
        return "church"
    if b in ("public", "civic", "government", "townhall") or t.get("amenity") == "townhall":
        return "public"
    if b in ("industrial", "warehouse", "hangar", "service", "storage_tank", "garages"):
        return "industrial"
    return None


def place_buildings(g, f, site, h, water, road, masks, placer, stats, min_area):
    """Every merged block of OSM footprints becomes catalog buildings: chosen
    by height (OSM, else the district's estimate), turned to the block's
    main edge direction, filled in rows; boxes that touch a road (with its
    edge), water, another building or a reserved place are shifted, taken
    smaller or dropped. Returns the placed buildings for the pads."""
    cfg = site["buildings"]
    W = g.W
    label = [0] * g.N
    owner_tags = []
    owner_edges = []
    owner_area = []
    count_osm = 0
    skip = masks["pitch"]
    for t, rings in f.buildings:
        if t.get("building") in ("roof", "construction") or t.get("layer", "0").startswith("-"):
            continue
        spans = polygon_spans(g, rings)
        if not spans:
            continue
        count_osm += 1
        owner_tags.append(t)
        owner_edges.append(rings[0])
        owner_area.append(area_of(rings))
        fid = len(owner_tags)
        for j, i0, i1 in spans:
            base = j * W
            for i in range(i0, i1 + 1):
                k = base + i
                if not water[k] and not skip[k]:
                    label[k] = fid
    # Blocks: 8-connected components of footprint samples.
    comp = [0] * g.N
    blocks = []
    for start in range(g.N):
        if not label[start] or comp[start]:
            continue
        cid = len(blocks) + 1
        comp[start] = cid
        stack = [start]
        cells = []
        members = set()
        while stack:
            k = stack.pop()
            cells.append(k)
            members.add(label[k])
            i, j = k % W, k // W
            for dj in (-1, 0, 1):
                jj = j + dj
                if jj < 0 or jj >= g.H:
                    continue
                for di in (-1, 0, 1):
                    ii = i + di
                    if 0 <= ii < W:
                        q = jj * W + ii
                        if label[q] and not comp[q]:
                            comp[q] = cid
                            stack.append(q)
        blocks.append((cells, sorted(members)))
    sizes = catalog_sizes(sorted({m for c in cfg["classes"].values() for m in c}))
    rnd = Random(cfg["seed"])
    min_cells = min_area // 4
    per_district = {}
    placed_list = []
    totals = {"blocks": 0, "small_blocks_dropped": 0, "placed": 0, "shifted": 0, "smaller": 0,
              "row_steps_without_room": 0, "blocks_without_building": 0}
    perimeter = cfg.get("layout") == "perimeter"
    # Prompt 116: a district may pick its own layout (town blocks in the
    # city, rows in the village).
    if perimeter or any(d.get("layout") == "perimeter" for d in site["districts"]):
        placer.blocks = bytearray(g.N)

    def base_class(height, district):
        if district.get("style"):
            return district["style"]
        if height >= 24:
            return "tower"
        if height >= 14:
            return "block"
        if height >= 9:
            return "town"
        return "villa"

    def variant_of(district=None):
        # Prompt 116: a district may set its own shares (a bombed quarter).
        d = district or {}
        ruin, damaged = d.get("ruin_share", cfg["ruin_share"]), d.get("damaged_share", cfg["damaged_share"])
        r = rnd.unit()
        return "Ruin" if r < ruin else ("Damaged" if r < ruin + damaged else "Intact")

    def perimeter_block(cells, cellset, frame, district, block_height, block_industrial, stat):
        """Prompt 112: closed street walls. Rows of catalog buildings stand
        along the four sides of the block, each at the block's real edge
        in its stretch (so skewed and triangular blocks are followed in
        steps), the street facade (+X) outwards; the inside stays free as
        a courtyard. The class comes from the footprint under the house
        (its height, church, public building or industry), else from the
        block. A passage of `passage_m` every `passage_every_m` lets
        infantry into the courtyard."""
        cxm, cym, ca_, sa_, heading, u0, u1, v0, v1 = frame
        nu = int((u1 - u0) / 2.0) + 2
        nv = int((v1 - v0) / 2.0) + 2
        vmin, vmax = [None] * nu, [None] * nu
        umin, umax = [None] * nv, [None] * nv
        for k in cells:
            px, py = g.x(k % W) - cxm, g.y(k // W) - cym
            u = px * ca_ + py * sa_
            v = -px * sa_ + py * ca_
            bu, bv = int((u - u0) / 2.0), int((v - v0) / 2.0)
            if vmin[bu] is None or v < vmin[bu]:
                vmin[bu] = v
            if vmax[bu] is None or v > vmax[bu]:
                vmax[bu] = v
            if umin[bv] is None or u < umin[bv]:
                umin[bv] = u
            if umax[bv] is None or u > umax[bv]:
                umax[bv] = u
        gap = cfg["building_gap_m"]
        placed_here = 0

        def edge(side, a, b):
            """The block's edge over the stretch a..b along the side
            (innermost value), or None where the block has a hole."""
            if side in ("v0", "v1"):
                bins, lo, hi = (vmin if side == "v0" else vmax), u0, nu
            else:
                bins, lo, hi = (umin if side == "u0" else umax), v0, nv
            vals = [bins[q] for q in range(max(0, int((a - lo) / 2.0)), min(hi, int((b - lo) / 2.0) + 1))]
            if not vals or None in vals:
                return None
            return max(vals) if side in ("v0", "u0") else min(vals)

        def to_xy(uu, vv):
            return cxm + uu * ca_ - vv * sa_, cym + uu * sa_ + vv * ca_

        def probe(side, pos):
            """Class choices at a stretch: from the footprint 5 m inside."""
            e = edge(side, pos, pos + 4.0)
            if e is None:
                return None
            inward = 5.0 if side in ("v0", "u0") else -5.0
            if side in ("v0", "v1"):
                x, y = to_xy(pos + 2.0, e + inward)
            else:
                x, y = to_xy(e + inward, pos + 2.0)
            k = g.index(x, y)
            fid = label[k] if k >= 0 else 0
            height, special = block_height, None
            if fid:
                t = owner_tags[fid - 1]
                v = tagged_height(t, cfg)
                if v is not None:
                    height = v
                special = footprint_classes(t)
                if special == "church" and owner_area[fid - 1] < 150:
                    special = None
                if special == "public" and owner_area[fid - 1] < cfg["public_min_m2"]:
                    special = None
            if special is None and block_industrial:
                special = "industrial"
            base = base_class(height, district)
            kinds = list(cfg["classes"][base])
            start = rnd.below(len(kinds))
            kinds = kinds[start:] + kinds[:start]
            out = [(m, special) for m in cfg["classes"][special]] if special else []
            return out + [(m, base) for m in kinds]

        sides = (("v1", 0), ("v0", 180), ("u0", 90), ("u1", 270))
        for side, turn in sides:
            along = side in ("v0", "v1")
            a0, a1 = (u0, u1) if along else (v0, v1)
            pos = a0
            since_passage = 0.0
            while pos < a1 - 4.0:
                choices = probe(side, pos)
                done = False
                for kind, which in (choices or []):
                    length, width, _ = sizes[kind]
                    if pos + length > a1 + cfg["overhang_m"]:
                        continue
                    e = edge(side, pos, pos + length)
                    if e is None:
                        continue
                    for inset in cfg["inward_shifts_m"]:
                        if side in ("v0", "u0"):
                            c = e + width / 2.0 + inset
                        else:
                            c = e - width / 2.0 - inset
                        x, y = to_xy(pos + length / 2.0, c) if along else to_xy(c, pos + length / 2.0)
                        hd = (heading + turn) % 360
                        if not placer.inside_map(x, y, hd, length, width):
                            continue
                        core = placer.box(x, y, hd, length, width, 0.0)
                        if not core or sum(1 for k in core if k in cellset) < cfg["coverage"] * len(core):
                            continue
                        if not placer.free(placer.box(x, y, hd, length, width, cfg["road_margin_m"]), occupied=False):
                            continue
                        if not placer.free(placer.box(x, y, hd, length, width, gap), road=False):
                            continue
                        zs = [bilinear(g, h, x + dx, y + dy) for dx, dy in
                              ((0, 0), (length / 2, width / 2), (-length / 2, width / 2), (length / 2, -width / 2), (-length / 2, -width / 2))]
                        if max(zs) - min(zs) > cfg["max_pad_drop_m"]:
                            continue
                        placer.take(core)
                        placer.row(kind, variant_of(district), x, y, hd)
                        placed_list.append((x, y, hd, length, width))
                        placed_here += 1
                        totals["placed"] += 1
                        stat["placed"] += 1
                        if inset:
                            totals["shifted"] += 1
                            stat["shifted"] += 1
                        pos += length + gap
                        since_passage += length + gap
                        if since_passage >= cfg["passage_every_m"]:
                            pos += cfg["passage_m"]
                            since_passage = 0.0
                        done = True
                        break
                    if done:
                        break
                if not done:
                    totals["row_steps_without_room"] += 1
                    stat["row_steps_without_room"] += 1
                    pos += cfg["step_m"]
        if placed_here:
            for k in cells:
                placer.blocks[k] = 1
        return placed_here

    for cid, (cells, members) in enumerate(blocks, start=1):
        if len(cells) < min_cells:
            totals["small_blocks_dropped"] += 1
            continue
        totals["blocks"] += 1
        # Main direction: length-weighted edge directions modulo 90 degrees.
        hist = [0.0] * 90
        tags = [owner_tags[m - 1] for m in members]
        for m in members:
            ring = owner_edges[m - 1]
            for k in range(len(ring) - 1):
                dx, dy = ring[k + 1][0] - ring[k][0], ring[k + 1][1] - ring[k][1]
                ll = math.sqrt(dx * dx + dy * dy)
                if ll > 0.5:
                    hist[int(atan2_deg(dy, dx) % 90.0) % 90] += ll
        smooth = [sum(hist[(a + d) % 90] for d in (-2, -1, 0, 1, 2)) for a in range(90)]
        angle = max(range(90), key=lambda a: (smooth[a], -a))
        pts = [(g.x(k % W), g.y(k // W)) for k in cells]
        cxm = sum(p[0] for p in pts) / len(pts)
        cym = sum(p[1] for p in pts) / len(pts)
        best = None
        for a in (angle, angle + 90):
            c, s = cos_sin(float(a))
            us = [(p[0] - cxm) * c + (p[1] - cym) * s for p in pts]
            vs = [-(p[0] - cxm) * s + (p[1] - cym) * c for p in pts]
            ext = (max(us) - min(us), max(vs) - min(vs))
            if best is None or ext[0] > best[1][0]:
                best = (a, ext, (min(us), max(us), min(vs), max(vs)))
        a, _, (u0, u1, v0, v1) = best
        ca_, sa_ = cos_sin(float(a))
        # Long axis direction (ca_, sa_) in map coordinates -> heading.
        heading = rint(atan2_deg(ca_, -sa_)) % 360
        district = district_of(site, cxm, cym)
        dname = district["name"]
        stat = per_district.setdefault(dname, {"blocks": 0, "placed": 0, "shifted": 0, "smaller": 0, "row_steps_without_room": 0})
        stat["blocks"] += 1
        if district.get("skip_blocks"):
            # Prompt 112: today's footprints on the waterfront are later
            # buildings; the quays get sheds (`sheds`) instead.
            stat["skipped_blocks"] = stat.get("skipped_blocks", 0) + 1
            continue
        # Height: tallest tagged footprint, else the district's estimate.
        tagged = [tagged_height(t, cfg) for t in tags]
        tagged = [v for v in tagged if v is not None]
        area = len(cells) * 4
        if tagged:
            height = max(tagged)
        else:
            levels = district["levels"][0] if area < cfg["large_block_m2"] else district["levels"][1]
            height = 3.2 * levels + 1.0
        special = [footprint_classes(t) for t in tags]
        industrial_ground = sum(1 for k in cells if masks["industrial"][k]) * 2 > len(cells)
        if "church" in special and area >= 150:
            cls = ["church"]
        elif "public" in special and area >= cfg["public_min_m2"]:
            cls = ["public"]
        elif "industrial" in special or industrial_ground:
            cls = ["industrial"]
        else:
            cls = []
        if district.get("style"):
            base = district["style"]
        elif height >= 24:
            base = "tower"
        elif height >= 14:
            base = "block"
        elif height >= 9:
            base = "town"
        else:
            base = "villa"
        cls.append(base)
        cellset = set(cells)
        # Rows across the depth of the block.
        depth = v1 - v0
        first = sizes[cfg["classes"][cls[-1]][0]]
        rows = max(1, min(4, int((depth + cfg["row_gap_m"]) / (first[1] + cfg["row_gap_m"]))))
        if rows == 1:
            centers = [(v0 + v1) / 2.0]
        else:
            centers = [v0 + first[1] / 2.0 + r * (depth - first[1]) / (rows - 1) for r in range(rows)]
        placed_here = 0
        special_left = cls[:-1]
        if district.get("layout", cfg.get("layout")) == "perimeter":
            centers = []
            placed_here = perimeter_block(cells, cellset, (cxm, cym, ca_, sa_, heading, u0, u1, v0, v1), district,
                                          height, industrial_ground, stat)
        for vc in centers:
            u = u0
            while u < u1 - 4.0:
                choices = []
                if special_left:
                    choices += [(m, special_left[0]) for m in cfg["classes"][special_left[0]]]
                choices += [(m, cls[-1]) for m in cfg["classes"][cls[-1]]]
                done = False
                for n, (kind, which) in enumerate(choices):
                    length, width, _ = sizes[kind]
                    if u + length > u1 + cfg["overhang_m"]:
                        continue
                    for shift in (0.0, 2.0, -2.0, 4.0, -4.0):
                        uc, vcs = u + length / 2.0, vc + shift
                        x = cxm + uc * ca_ - vcs * sa_
                        y = cym + uc * sa_ + vcs * ca_
                        if not placer.inside_map(x, y, heading, length, width):
                            continue
                        core = placer.box(x, y, heading, length, width, 0.0)
                        if not core or sum(1 for k in core if k in cellset) < cfg["coverage"] * len(core):
                            continue
                        if not placer.free(placer.box(x, y, heading, length, width, cfg["road_margin_m"])):
                            continue
                        zs = [bilinear(g, h, x + dx, y + dy) for dx, dy in
                              ((0, 0), (length / 2, width / 2), (-length / 2, width / 2), (length / 2, -width / 2), (-length / 2, -width / 2))]
                        if max(zs) - min(zs) > cfg["max_pad_drop_m"]:
                            continue
                        placer.take(placer.box(x, y, heading, length, width, 0.5))
                        placer.row(kind, variant_of(district), x, y, heading)
                        placed_list.append((x, y, heading, length, width))
                        placed_here += 1
                        totals["placed"] += 1
                        stat["placed"] += 1
                        if shift != 0.0:
                            totals["shifted"] += 1
                            stat["shifted"] += 1
                        if n > 0 and which == cls[-1] and not special_left:
                            totals["smaller"] += 1
                            stat["smaller"] += 1
                        if which in special_left:
                            special_left.remove(which)
                        u += length + cfg["gap_m"]
                        done = True
                        break
                    if done:
                        break
                if not done:
                    totals["row_steps_without_room"] += 1
                    stat["row_steps_without_room"] += 1
                    u += cfg["step_m"]
        if not placed_here:
            totals["blocks_without_building"] += 1
            stat["blocks_without_building"] = stat.get("blocks_without_building", 0) + 1
    totals["osm_footprints"] = count_osm
    totals["min_block_m2"] = min_area
    stats["buildings"] = {"totals": totals, "districts": per_district}
    return placed_list


def place_sheds(g, site, h, placer, stats):
    """Prompt 112: sheds and warehouses along lines of the site (quays and
    piers whose 1944 sheds are not in today's data), the length along the
    line; a shed keeps `road_margin_m` off roads, 1 m off water, other
    buildings and reserved places, and is shifted across the line where it
    does not fit. Returns the placed boxes."""
    cfg = site["buildings"]
    sizes = catalog_sizes(sorted({row["kind"] for row in site["sheds"]}))
    out = []
    skipped = 0
    for row in site["sheds"]:
        (ax, ay), (bx, by) = row["from_m"], row["to_m"]
        total = math.sqrt((bx - ax) ** 2 + (by - ay) ** 2)
        dx, dy = (bx - ax) / total, (by - ay) / total
        heading = rint(atan2_deg(dx, -dy)) % 360
        length, width, _ = sizes[row["kind"]]
        t = 0.0
        while t + length <= total + 0.01:
            done = False
            for shift in (0.0, 3.0, -3.0, 6.0, -6.0, 9.0, -9.0):
                x = ax + dx * (t + length / 2.0) - dy * shift
                y = ay + dy * (t + length / 2.0) + dx * shift
                if not placer.inside_map(x, y, heading, length, width):
                    continue
                core = placer.box(x, y, heading, length, width, 0.0)
                if not core or not placer.free(placer.box(x, y, heading, length, width, 1.0), road=False):
                    continue
                if any(placer.road[k] for k in placer.box(x, y, heading, length, width, cfg["road_margin_m"])):
                    continue
                placer.take(core)
                placer.row(row["kind"], "Intact", x, y, heading)
                out.append((x, y, heading, length, width))
                done = True
                break
            if done:
                t += length + row.get("gap_m", 6.0)
            else:
                skipped += 1
                t += cfg["step_m"]
    stats["buildings"]["sheds"] = {"placed": len(out), "steps_without_room": skipped}
    return out


def place_fortifications(g, site, placer, stats):
    """Prompt 113: field works at the places of the site's coast batteries
    (`fortifications.works`, positions from OSM or by hand, each with its
    source in site.json): a concrete pillbox (`fort_bunker_mg`, standing in
    for the battery's concrete) at the nearest free spot, a line of trench
    sections in front of it towards `facing_deg` (compass) and sandbag
    positions on both sides. Pieces that would touch a road, water, a
    building or a reserved place are left out."""
    cfg = site["fortifications"]
    sizes = catalog_sizes(["fort_bunker_mg", "fort_trench_section", "fort_sandbag_position"])
    turn = site["frame"]["x_axis_bearing_deg"] - 90
    counts = {"works": 0, "bunkers": 0, "trenches": 0, "sandbags": 0, "bunkers_without_room": 0}
    rings = [(0.0, 0.0)] + [(r * c, r * s_) for r in (3.0, 6.0, 9.0, 12.0, 15.0)
                            for c, s_ in (cos_sin(a * 45.0) for a in range(8))]

    def put(kind, variant, x, y, heading, margin, count_key):
        length, width, _ = sizes[kind]
        if not placer.inside_map(x, y, heading, length, width):
            return False
        if not placer.free(placer.box(x, y, heading, length, width, margin)):
            return False
        placer.take(placer.box(x, y, heading, length, width, 0.5))
        placer.row(kind, variant, x, y, heading)
        counts[count_key] += 1
        return True

    for w in cfg["works"]:
        x0, y0 = g.to_map(parse_deg(w["at_deg"][0]), parse_deg(w["at_deg"][1]))
        if not g.inside(x0, y0, -10.0):
            continue
        counts["works"] += 1
        facing = (w["facing_deg"] - turn) % 360
        fc, fs = cos_sin(facing)
        fx, fy = fs, -fc               # towards the facing (map)
        rx, ry = fc, fs                # to the right of it
        at = None
        for dx, dy in rings:
            if put("fort_bunker_mg", "Pillbox", x0 + dx, y0 + dy, facing, cfg["bunker_margin_m"], "bunkers"):
                at = (x0 + dx, y0 + dy)
                break
        if at is None:
            counts["bunkers_without_room"] += 1
            at = (x0, y0)
        bx, by = at
        n = cfg["trench_sections"]
        step = sizes["fort_trench_section"][0] + 0.5
        ahead = cfg["trench_ahead_m"]
        for k in range(n):
            off = (k - (n - 1) / 2.0) * step
            put("fort_trench_section", "Straight", bx + fx * ahead + rx * off, by + fy * ahead + ry * off,
                (facing + 90) % 360, 0.5, "trenches")
        side = (n * step) / 2.0 + 3.0
        for sgn in (-1.0, 1.0):
            put("fort_sandbag_position", "Position", bx + fx * (ahead - 2.0) + rx * side * sgn,
                by + fy * (ahead - 2.0) + ry * side * sgn, facing, 0.5, "sandbags")
    stats["fortifications"] = counts


def reserve_circle(g, mask, center, radius):
    cx, cy = center
    for j in range(max(0, int((cy - radius + g.hy) / 2.0)), min(g.H, int((cy + radius + g.hy) / 2.0) + 2)):
        for i in range(max(0, int((cx - radius + g.hx) / 2.0)), min(g.W, int((cx + radius + g.hx) / 2.0) + 2)):
            if (g.x(i) - cx) ** 2 + (g.y(j) - cy) ** 2 <= radius * radius:
                mask[j * g.W + i] = 1


def height_encoding(h):
    """Base and step (mm) of the 16-bit height file for these heights."""
    mm = [rint(v * 1000.0) for v in h]
    low, high = min(mm), max(mm)
    base = (low // 1000 - 1) * 1000
    step = 1
    while (high - base) // step > 65535:
        step += 1
    return base, step


def stored_mm(h, encoding):
    """Heights in mm as the map file will store them."""
    base, step = encoding
    return [base + step * rint((rint(v * 1000.0) - base) / step) for v in h]


# A box overlaps a path cell's square (4 m, 1 m beyond its samples) when it
# comes this close to one of the cell's samples (conservative: the
# simulation's separating-axis test blocks no more cells than this).
CELL_OVERLAP_M = 1.5


def pave_town(g, site, placed, terrain, water, masks, stats):
    """Paved ground (terrain `road`) within `pave_m` of the town's
    buildings, outside parks, woods, scrub and sports grounds; the
    districts with `garden: true` (villas) keep their grass."""
    cfg = site["buildings"]
    near = bytearray(g.N)
    for x, y, heading, length, width in placed:
        if district_of(site, x, y).get("garden"):
            continue
        for k in Placer.box_samples(g, x, y, heading, length, width, cfg["pave_m"]):
            near[k] = 1
    paved = 0
    for k in range(g.N):
        if near[k] and not water[k] and terrain[k] in (GRASS, TALL_GRASS) and not (
                masks["park"][k] or masks["wood"][k] or masks["scrub"][k] or masks["pitch"][k]):
            terrain[k] = ROAD
            paved += 1
    stats["buildings"]["paved_m2"] = paved * 4


def cell_steep(g, h, x, y, encoding):
    """Whether path cell (x, y) is too steep, as `sim::path` decides it
    (on the heights as stored)."""
    W = g.W
    i0, j0 = 2 * x, 2 * y
    base, step = encoding

    def H(i, j):
        return base + step * rint((rint(h[min(j, g.H - 1) * W + min(i, W - 1)] * 1000.0) - base) / step)

    for j in range(max(j0 - 1, 0), j0 + 2):
        for i in range(max(i0 - 1, 0), i0 + 2):
            gx = max(abs(H(i + 1, j) - H(i, j)), abs(H(i + 1, j + 1) - H(i, j + 1)))
            gy = max(abs(H(i, j + 1) - H(i, j)), abs(H(i + 1, j + 1) - H(i + 1, j)))
            if gx + gy > 1200:
                return True
    return False


def build_pads(g, placed, h, road, water, cfg, encoding):
    """Level the ground under every building (the box grown by
    `pad_margin_m`) at its center height where its corners differ by more
    than `pad_tolerance_m`. No cut or fill around it: the walls hide the
    step, which may leave path cells beside the house too steep. A pad is
    left out (the house stands on the slope) when it would cut a way: the
    open path cells around the house (`pad_window_m`) must stay connected
    among each other as before. Never within `pad_road_clear_m` of a road."""
    near_road = to_mask(g, dilate(g, to_rows(g, road), int(cfg["pad_road_clear_m"] / 2.0)))
    cw, ch = g.W // 2, g.H // 2
    core = bytearray(cw * ch)
    for x, y, heading, length, width in placed:
        for k in Placer.box_samples(g, x, y, heading, length, width, CELL_OVERLAP_M):
            i, j = (k % g.W) // 2, (k // g.W) // 2
            if i < cw and j < ch:
                core[j * cw + i] = 1
    flattened, skipped = 0, 0
    margin = cfg["pad_window_m"]
    for x, y, heading, length, width in placed:
        samples = Placer.box_samples(g, x, y, heading, length, width, cfg["pad_margin_m"])
        zs = [h[k] for k in samples]
        if not zs or max(zs) - min(zs) <= cfg["pad_tolerance_m"]:
            continue
        reach = length / 2.0 + width / 2.0 + margin
        x0 = max(0, int((x - reach + g.hx) / 4.0))
        x1 = min(cw - 1, int((x + reach + g.hx) / 4.0))
        y0 = max(0, int((y - reach + g.hy) / 4.0))
        y1 = min(ch - 1, int((y + reach + g.hy) / 4.0))
        ww, wh = x1 - x0 + 1, y1 - y0 + 1

        def window_open():
            return bytearray(0 if core[(y0 + b) * cw + x0 + a] or cell_steep(g, h, x0 + a, y0 + b, encoding) else 1
                             for b in range(wh) for a in range(ww))

        before = window_open()
        z = bilinear(g, h, x, y)
        old = [(k, h[k]) for k in samples if not near_road[k] and not water[k]]
        for k, _ in old:
            h[k] = z
        after = window_open()
        lb, _ = cell_components(ww, wh, before)
        la, _ = cell_components(ww, wh, after)
        border = [b * ww + a for b in range(wh) for a in range(ww) if a in (0, ww - 1) or b in (0, wh - 1)]
        joined = {}
        cut = False
        for c in border:
            if not lb[c]:
                continue
            if not la[c]:
                cut = True
                break
            if joined.setdefault(lb[c], la[c]) != la[c]:
                cut = True
                break
        if cut:
            for k, v in old:
                h[k] = v
            skipped += 1
        else:
            flattened += 1
    return flattened, skipped


def path_cells(g, h, terrain, encoding):
    """Like the simulation's path grid (`sim::path`, 4 m cells of 2 x 2
    samples): per cell whether infantry may enter it, ignoring objects
    (no impassable terrain in it, no grade above 60 % by |dx| + |dy| over
    the height patches it overlaps), on the heights as the file stores them."""
    cw, ch = g.W // 2, g.H // 2
    W, H = g.W, g.H
    hm = stored_mm(h, encoding)
    # Patch (i, j) spans samples i..i+1, j..j+1 (clamped at the far edges).
    bad = bytearray(g.N)
    for j in range(H):
        r0 = hm[j * W:(j + 1) * W]
        r1 = hm[min(j + 1, H - 1) * W:(min(j + 1, H - 1) + 1) * W]
        base = j * W
        for i in range(W):
            i1 = i + 1 if i + 1 < W else i
            a, b, c, d = r0[i], r0[i1], r1[i], r1[i1]
            gx = max(abs(b - a), abs(d - c))
            gy = max(abs(c - a), abs(d - b))
            if gx + gy > 1200:
                bad[base + i] = 1
    # A cell (x, y) looks at the patches x*2-1 .. x*2+1, y*2-1 .. y*2+1.
    near = to_mask(g, dilate(g, to_rows(g, bad), 1))
    open_ = bytearray(cw * ch)
    for y in range(ch):
        j0 = 2 * y
        for x in range(cw):
            i0 = 2 * x
            k = j0 * W + i0
            if near[k] or DEEP in (terrain[k], terrain[k + 1], terrain[k + W], terrain[k + W + 1]):
                continue
            open_[y * cw + x] = 1
    return cw, ch, open_


def cell_components(cw, ch, open_):
    label = [0] * (cw * ch)
    sizes = [0]
    for s in range(cw * ch):
        if not open_[s] or label[s]:
            continue
        n = len(sizes)
        label[s] = n
        stack = [s]
        size = 0
        while stack:
            k = stack.pop()
            size += 1
            x, y = k % cw, k // cw
            for q in ((k - 1) if x > 0 else -1, (k + 1) if x < cw - 1 else -1,
                      (k - cw) if y > 0 else -1, (k + cw) if y < ch - 1 else -1):
                if q >= 0 and open_[q] and not label[q]:
                    label[q] = n
                    stack.append(q)
        sizes.append(size)
    return label, sizes


def keep_ways_open(g, site, h, h_before, terrain, placed, cfg, stats, encoding):
    """Pads and buildings must not wall off ground that was connected
    before them: where a piece of ground (at least `min_part_cells` path
    cells) is cut off from the larger part it belonged to, the buildings at
    its edge first lose their pad, in a later pass the building itself.
    Returns the indices of the removed buildings."""
    cw, ch, ground = path_cells(g, h_before, terrain, encoding)
    label0, _ = cell_components(cw, ch, ground)

    def cells(n, margin):
        x, y, heading, length, width = placed[n]
        out = set()
        for k in Placer.box_samples(g, x, y, heading, length, width, margin):
            i, j = k % g.W, k // g.W
            if i // 2 < cw and j // 2 < ch:
                out.add((j // 2) * cw + i // 2)
        return out

    near = [cells(n, cfg["open_check_m"]) for n in range(len(placed))]
    core = [cells(n, CELL_OVERLAP_M) for n in range(len(placed))]
    pads = [Placer.box_samples(g, *p[:3], p[3], p[4], cfg["pad_margin_m"]) for p in placed]
    removed, unpadded = set(), set()
    for _ in range(cfg["open_passes"]):
        _, _, now = path_cells(g, h, terrain, encoding)
        for n in range(len(placed)):
            if n not in removed:
                for c in core[n]:
                    now[c] = 0
        label1, sizes1 = cell_components(cw, ch, now)
        # Per original piece of ground, its parts now; all but the largest
        # big part are cut off.
        groups = {}
        for c in range(cw * ch):
            if label1[c] and label0[c] and sizes1[label1[c]] >= cfg["min_part_cells"]:
                groups.setdefault(label0[c], set()).add(label1[c])
        cut_off = set()
        for parts in groups.values():
            if len(parts) > 1:
                largest = max(parts, key=lambda k: (sizes1[k], -k))
                cut_off.update(p for p in parts if p != largest)
        if not cut_off:
            break
        for n in range(len(placed)):
            if n in removed or not any(label1[c] in cut_off for c in near[n]):
                continue
            if n in unpadded:
                removed.add(n)
            else:
                unpadded.add(n)
                for k in pads[n]:
                    h[k] = h_before[k]
    totals = stats["buildings"]["totals"]
    totals["pads_undone_to_keep_ways_open"] = len(unpadded - removed)
    totals["removed_to_keep_ways_open"] = len(removed)
    for n in sorted(removed):
        d = stats["buildings"]["districts"][district_of(site, placed[n][0], placed[n][1])["name"]]
        d["placed"] -= 1
        d["removed"] = d.get("removed", 0) + 1
    return removed


# --------------------------------------------------------------------------
# Green: woods, parks, tree rows, scrub

def place_green(g, f, site, masks, placer, terrain, stats):
    cfg = site["green"]
    rnd = Random(cfg["seed"])
    W = g.W
    counts = {"trees": 0, "palms": 0, "bushes": 0, "hedges": 0}
    med = mediterranean(site)
    if med:
        counts["olives"] = 0

    def tree(x, y, kind, variant):
        k = g.index(x, y)
        if k < 0 or not g.inside(x, y, -1.0):
            return False
        trunk = placer.box(x, y, 0, 2.0 * cfg["trunk_clear_m"], 2.0 * cfg["trunk_clear_m"], 0.0)
        if not placer.free(trunk):
            return False
        placer.take(placer.box(x, y, 0, 1.0, 1.0, 0.0))
        placer.row(kind, variant, x, y, rnd.below(360))
        counts["palms" if kind == "env_tree_palm" else "trees"] += 1
        return True

    def bush(x, y):
        k = g.index(x, y)
        if k < 0 or not g.inside(x, y, -1.0) or placer.occupied[k] or placer.water[k] or placer.road[k] or placer.reserved[k]:
            return
        if med:
            placer.row("env_bush_macchia", ["Holm", "Strawberry", "Broom"][rnd.below(3)], x, y, rnd.below(360))
        else:
            placer.row("env_bush", ["Round", "Wide", "Hazel"][rnd.below(3)], x, y, rnd.below(360))
        counts["bushes"] += 1

    def scatter(mask, spacing, pick, bush_share):
        """Jittered grid over the samples of `mask`."""
        step = int(spacing / 2.0)
        for j in range(0, g.H, step):
            for i in range(0, W, step):
                k = j * W + i
                if not mask[k]:
                    continue
                x = g.x(i) + (rnd.unit() - 0.5) * spacing * 0.8
                y = g.y(j) + (rnd.unit() - 0.5) * spacing * 0.8
                kk = g.index(x, y)
                if kk < 0 or not mask[kk]:
                    continue
                if rnd.unit() < bush_share:
                    bush(x, y)
                else:
                    kind, variant = pick(x, y)
                    tree(x, y, kind, variant)

    def med_pick(x, y):
        # Prompt 120: stone pines, some olives and cypresses.
        r = rnd.unit()
        if r < 0.45:
            return "env_tree_stone_pine", "StonePine"
        if r < 0.6:
            return "env_tree_stone_pine", ["StonePineYoung", "StonePineLeaning"][rnd.below(2)]
        if r < 0.8:
            return "env_tree_olive", ["Olive", "OliveOld"][rnd.below(2)]
        return "env_tree_cypress", ["Cypress", "CypressShort"][rnd.below(2)]

    def wood_pick(x, y):
        if med:
            return med_pick(x, y)
        r = rnd.unit()
        if r < 0.55:
            return "env_tree_conifer", "Pine"
        if r < 0.85:
            return "env_tree_deciduous", "Oak"
        return "env_tree_deciduous", "OakYoung"

    def park_pick(x, y):
        r = rnd.unit()
        if r < cfg["park_palm_share"]:
            return "env_tree_palm", ["Palm", "PalmShort", "PalmLeaning"][rnd.below(3)]
        if med:
            return med_pick(x, y)
        if r < 0.8:
            return "env_tree_deciduous", ["Oak", "Beech", "OakYoung"][rnd.below(3)]
        return "env_tree_conifer", "Pine"

    scatter(masks["wood"], cfg["wood_spacing_m"], wood_pick, 0.12)
    if med:
        # Prompt 120: olive groves in rows (a regular grid of
        # `olive_spacing_m`, default 8 m, a little irregular), a young tree
        # here and there.
        sp = cfg.get("olive_spacing_m", 8)
        step = max(1, int(sp / 2.0))
        orchard = masks["orchard"]
        for j in range(0, g.H, step):
            for i in range(0, W, step):
                if not orchard[j * W + i]:
                    continue
                x = g.x(i) + (rnd.unit() - 0.5) * 0.8
                y = g.y(j) + (rnd.unit() - 0.5) * 0.8
                v = ["Olive", "Olive", "OliveOld", "OliveYoung"][rnd.below(4)]
                if tree(x, y, "env_tree_olive", v):
                    counts["olives"] += 1
    scatter(masks["park"], cfg["park_spacing_m"], park_pick, 0.35)
    if cfg["scrub_spacing_m"]:
        # Prompt 113: 0 keeps scrub free of objects (tall grass only, no
        # trees on open ridges).
        scrub_bushes = bytearray(masks["scrub"])
        scatter(scrub_bushes, cfg["scrub_spacing_m"], wood_pick, 0.7)
    # Tree rows: a tree every `row_spacing_m` where there is room.
    for t, pts in f.lines:
        for kk in range(len(pts) - 1):
            (ax, ay), (bx, by) = pts[kk], pts[kk + 1]
            seg = math.sqrt((bx - ax) ** 2 + (by - ay) ** 2)
            n = max(1, int(seg / cfg["row_spacing_m"]))
            for s in range(n):
                x = ax + (bx - ax) * (s + 0.5) / n
                y = ay + (by - ay) * (s + 0.5) / n
                if "paved" in masks and masks["paved"][max(g.index(x, y), 0)]:
                    continue
                if g.inside(x, y, -2.0):
                    if y > cfg["palm_below_y_m"]:
                        tree(x, y, "env_tree_palm", "Palm")
                    elif med:
                        # Prompt 120: cypresses line the lanes.
                        tree(x, y, "env_tree_cypress", "Cypress")
                    else:
                        tree(x, y, "env_tree_deciduous", "Oak")
    # Hedges along the edges of gardens and parks (not at roads), in runs
    # with gaps.
    for t, rings in f.areas:
        if t.get("leisure") not in ("garden", "park") or area_of(rings) < site["areas"]["min_area_m2"]:
            continue
        ring = rings[0]
        walked = 0.0
        for kk in range(len(ring) - 1):
            (ax, ay), (bx, by) = ring[kk], ring[kk + 1]
            seg = math.sqrt((bx - ax) ** 2 + (by - ay) ** 2)
            if seg < 3.0:
                continue
            heading = rint(atan2_deg(bx - ax, -(by - ay))) % 360
            n = int(seg / 3.0)
            for s in range(n):
                walked += 3.0
                if walked % cfg["hedge_run_m"] > cfg["hedge_run_m"] - cfg["hedge_gap_m"]:
                    continue
                x = ax + (bx - ax) * (s + 0.5) / n
                y = ay + (by - ay) * (s + 0.5) / n
                if not g.inside(x, y, -2.0):
                    continue
                if "paved" in masks and masks["paved"][g.index(x, y)]:
                    continue
                samples = placer.box(x, y, heading, 3.0, 1.2, 2.0)
                if placer.free(samples):
                    placer.take(placer.box(x, y, heading, 3.0, 1.2, 0.0))
                    # Prompt 120: field-stone garden walls instead of hedges.
                    placer.row("env_wall_field" if med else "env_hedge", "Segment", x, y, heading)
                    counts["hedges"] += 1
    # Courtyards of the perimeter blocks (prompt 112): a few trees and
    # bushes, at least 4 m from the houses.
    if cfg.get("courtyard_spacing_m") and getattr(placer, "blocks", None):
        paved = masks.get("paved") or bytearray(g.N)
        free = bytearray(1 if placer.blocks[k] and not placer.occupied[k] and not paved[k] else 0 for k in range(g.N))
        court = to_mask(g, erode(g, to_rows(g, free), 2))

        def court_pick(x, y):
            if med:
                return ("env_tree_olive", "Olive") if rnd.unit() < 0.6 else ("env_tree_cypress", "CypressShort")
            r = rnd.unit()
            if r < 0.7:
                return "env_tree_deciduous", ["Oak", "Beech", "OakYoung"][rnd.below(3)]
            return "env_tree_birch", "Birch"
        scatter(court, cfg["courtyard_spacing_m"], court_pick, cfg.get("courtyard_bush_share", 0.4))
    stats["green"] = counts


# --------------------------------------------------------------------------
# Terrain from land use

def paint_areas(g, f, cfg, terrain, water, road, med=False):
    """Large areas only (`min_area_m2`): beaches sand, scrub and heath tall
    grass, pedestrian squares road. Returns the masks the objects need.
    Prompt 120, `med` (Mediterranean biome): orchards are olive groves of
    their own (mask "orchard"), vineyards green field rows."""
    masks = {}
    names = ("wood", "park", "scrub", "pitch", "beach", "industrial") + (("orchard", "vineyard") if med else ())
    for name in names:
        masks[name] = bytearray(g.N)
    small = cfg["min_area_m2"]
    park_landuse = ("recreation_ground", "cemetery", "village_green") + (() if med else ("orchard",))
    sorts = {
        "wood": lambda t: t.get("natural") == "wood" or t.get("landuse") == "forest",
        "park": lambda t: t.get("leisure") in ("park", "garden") or t.get("landuse") in park_landuse,
        "scrub": lambda t: t.get("natural") in ("scrub", "heath"),
        "pitch": lambda t: t.get("leisure") in ("pitch", "stadium", "track", "sports_centre", "golf_course"),
        "beach": lambda t: t.get("natural") == "beach",
        "industrial": lambda t: t.get("landuse") in ("industrial", "port", "harbour", "railway") or t.get("leisure") == "marina",
    }
    if med:
        sorts["orchard"] = lambda t: t.get("landuse") == "orchard"
        sorts["vineyard"] = lambda t: t.get("landuse") == "vineyard"
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
        elif med and masks["vineyard"][k]:
            # Vine rows: the green field draws rows along the field's long
            # axis (prompt 088).
            terrain[k] = FIELD_GREEN
    if cfg.get("open_sand"):
        # Prompt 114: open sand (OSM `natural=sand`, `natural=dune`) is
        # sand, over heath and scrub.
        n = 0
        for t, rings in f.areas:
            if t.get("natural") in ("sand", "dune") and area_of(rings) >= small:
                n += 1
                for j, i0, i1 in polygon_spans(g, rings):
                    for k in range(j * g.W + i0, j * g.W + i1 + 1):
                        if not water[k] and not road[k]:
                            terrain[k] = SAND
                            masks["scrub"][k] = 0
        kept["open sand"] = n
    return masks, kept


def drop_areas(f, cfg, stats):
    """Prompt 112: areas raised above the ground (`layer` > 0: a park on a
    viaduct, a park on a pier) and areas made in or after
    `built_before_year` (OSM `start_date`) are not read."""
    if not cfg.get("skip_raised") and "built_before_year" not in cfg:
        return
    keep, raised, late = [], 0, 0
    for t, rings in f.areas:
        try:
            layer = int(t.get("layer", "0"))
        except ValueError:
            layer = 0
        year = start_year(t)
        if cfg.get("skip_raised") and layer > 0:
            raised += 1
        elif "built_before_year" in cfg and year is not None and year >= cfg["built_before_year"]:
            late += 1
        else:
            keep.append((t, rings))
    f.areas = keep
    stats["areas_dropped"] = {"raised": raised, "built_later": late}


def skip_footprints(f, pairs, stats):
    """Prompt 113: footprints with one of the tag pairs `[key, value]` are
    not read (coast batteries and bunkers: `fortifications` stand there
    instead; ruins of later works)."""
    keep, n = [], 0
    for t, rings in f.buildings:
        if any(t.get(k) == v for k, v in pairs):
            n += 1
        else:
            keep.append((t, rings))
    f.buildings = keep
    stats["footprints_skipped_by_tags"] = n


def drop_postwar(g, f, site, stats):
    """Prompt 116: footprints with an OSM `start_date` from
    `buildings.drop_from_year` on are not read where their district says
    `drop_postwar` (the village and the polder of 1944; the town keeps
    them as stand-ins on the old street plan)."""
    year = site["buildings"]["drop_from_year"]
    keep, n = [], 0
    for t, rings in f.buildings:
        y = start_year(t)
        pts = rings[0]
        cx = sum(p[0] for p in pts) / len(pts)
        cy = sum(p[1] for p in pts) / len(pts)
        if y is not None and y >= year and district_of(site, cx, cy).get("drop_postwar"):
            n += 1
        else:
            keep.append((t, rings))
    f.buildings = keep
    stats["footprints_postwar_dropped"] = n


def core_landmarks(g, site, folder, placer, stats):
    """Prompt 118: landmarks of the core (`landmarks` in the core file:
    catalog kind, variant, map position, heading) stand before the town's
    buildings, which keep off them (1 m)."""
    data = json.loads((folder / site["core"]["file"]).read_text(encoding="utf-8"))
    marks = data.get("landmarks", [])
    if not marks:
        return
    sizes = catalog_sizes(sorted({m["kind"] for m in marks}))
    for m in marks:
        length, width, _ = sizes[m["kind"]]
        x, y = float(m["at_m"][0]), float(m["at_m"][1])
        placer.take(placer.box(x, y, m["heading"], length, width, 1.0))
        placer.row(m["kind"], m["variant"], x, y, m["heading"])
    stats.setdefault("core", {})["landmarks"] = len(marks)


def core_streets(g, site, folder, terrain, water, road, stats):
    """Prompt 118: the streets of the old core as paved ground. The old map
    shows streets as the open ground between its blocks; every sample of
    the core that is not an old block, pond, park or cleared ground and
    that a closing of the old blocks fills (gaps up to `street_max_m`
    wide) becomes road (terrain and the mask buildings keep off); open
    yards around single buildings stay as they are. Today's surviving axes (kept by `apply_core`)
    still level their beds; the rest stays on the ground as it is."""
    cfg = site["core"]
    data = json.loads((folder / cfg["file"]).read_text(encoding="utf-8"))
    poly = [tuple(p) for p in cfg["polygon_m"]]
    old = bytearray(g.N)
    rings = [g.pts(b["pts"]) for b in data["blocks"]]
    rings += [[tuple(float(v) for v in p) for p in b["pts_m"]] for b in data.get("buildings", [])]
    for ring in rings:
        for j, i0, i1 in polygon_spans(g, [ring]):
            old[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    other = bytearray(g.N)
    for key in ("water", "parks"):
        for b in data.get(key, []):
            for j, i0, i1 in polygon_spans(g, [g.pts(b["pts"])]):
                other[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    for cp in cfg.get("cleared_polygons_m", []):
        for j, i0, i1 in polygon_spans(g, [[tuple(p) for p in cp]]):
            other[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    r = rint(cfg.get("street_max_m", 16.0) / 4.0)
    rows = erode(g, dilate(g, to_rows(g, old), r), r)
    # A second, wider closing fills the crossings the first one leaves.
    near = to_mask(g, erode(g, dilate(g, rows, 2 * r), 2 * r))
    n = 0
    for j, i0, i1 in polygon_spans(g, [poly + [poly[0]]]):
        for k in range(j * g.W + i0, j * g.W + i1 + 1):
            if near[k] and not old[k] and not other[k] and not water[k]:
                if not road[k]:
                    n += 1
                road[k] = 1
                terrain[k] = ROAD
    stats.setdefault("core", {})["street_m2"] = n * 4


def apply_core(g, f, site, folder, stats):
    """Prompt 118: a core traced by hand from historical sources replaces
    today's data near the bridge (the quarter was destroyed in 1944 and
    rebuilt on another plan). `site.core.file` (next to `site.json`)
    holds the old blocks, ponds and parks (degrees * 1e7, traced from the
    old topographic map by `tools/geo_trace_core.py`) and, added by hand,
    `streets` (map meters, `highway` class) and `buildings` (tags and a
    ring in map meters: the church). Inside `core.polygon_m` today's
    footprints, areas and small ponds are not read (except footprints with
    an `amenity` of `keep_amenity`); today's streets mostly inside are
    kept only where the old map shows open ground (at most
    `street_block_share` of their length over the old blocks and, if set,
    at least `street_near_share` within `street_near_m` of them), so the
    axes that survived stay as they are measured today; streets mostly in
    `cleared_polygons_m` (ground rebuilt after the map, the bridge ramp)
    are not read."""
    cfg = site["core"]
    data = json.loads((folder / cfg["file"]).read_text(encoding="utf-8"))
    poly = [tuple(p) for p in cfg["polygon_m"]]

    def mean_inside(pts):
        n = len(pts)
        return in_polygon(sum(p[0] for p in pts) / n, sum(p[1] for p in pts) / n, poly)

    def share_inside(pts):
        return sum(1 for x, y in pts if in_polygon(x, y, poly)) / float(len(pts))

    blocks = [g.pts(b["pts"]) for b in data["blocks"]]
    old = bytearray(g.N)
    for ring in blocks:
        for j, i0, i1 in polygon_spans(g, [ring]):
            old[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    near = to_mask(g, dilate_round(g, to_rows(g, old), rint(cfg.get("street_near_m", 12.0) / 2.0)))
    keep_amenity = cfg.get("keep_amenity", [])
    st = {"footprints_dropped": 0, "footprints_kept": 0, "areas_dropped": 0, "streets_dropped": 0,
          "streets_dropped_km": 0.0, "streets_kept": 0}
    keep = []
    for t, rings in f.buildings:
        if mean_inside(rings[0]):
            if t.get("amenity") in keep_amenity:
                st["footprints_kept"] += 1
                keep.append((t, rings))
            else:
                st["footprints_dropped"] += 1
        else:
            keep.append((t, rings))
    f.buildings = keep
    keep = []
    for t, rings in f.areas:
        small = area_of(rings) < cfg.get("water_keep_m2", 200000)
        if mean_inside(rings[0]) and (small or t.get("natural") != "water"):
            st["areas_dropped"] += 1
        else:
            keep.append((t, rings))
    f.areas = keep
    keep = []
    share_max = cfg.get("street_block_share", 0.25)
    cleared = [[tuple(p) for p in cp] for cp in cfg.get("cleared_polygons_m", [])]
    for t, pts, keys in f.highways:
        samples = line_samples(g, pts)
        if cleared and 2 * sum(1 for x, y in pts if any(in_polygon(x, y, cp) for cp in cleared)) > len(pts):
            st["streets_dropped"] += 1
            st["streets_dropped_km"] += length(pts) / 1000.0
            continue
        if samples and share_inside(pts) > 0.5:
            over = sum(1 for k in samples if old[k]) / float(len(samples))
            by = sum(1 for k in samples if near[k]) / float(len(samples))
            if over > share_max or by < cfg.get("street_near_share", 0.0):
                st["streets_dropped"] += 1
                st["streets_dropped_km"] += length(pts) / 1000.0
                continue
            st["streets_kept"] += 1
        keep.append((t, pts, keys))
    f.highways = keep
    for ring in blocks:
        f.buildings.append(({"building": "yes"}, [ring]))
    for b in data.get("buildings", []):
        f.buildings.append((dict(b["tags"]), [[tuple(float(v) for v in p) for p in b["pts_m"]]]))
    for w in data.get("water", []):
        f.areas.append(({"natural": "water"}, [g.pts(w["pts"])]))
    for w in data.get("parks", []):
        f.areas.append(({"leisure": "park"}, [g.pts(w["pts"])]))
    for s in data.get("streets", []):
        pts = [tuple(float(v) for v in p) for p in s["pts_m"]]
        keys = [("core", rint(x * 10.0), rint(y * 10.0)) for x, y in pts]
        f.highways.append(({"highway": s["highway"]}, pts, keys))
    st["streets_dropped_km"] = round(st["streets_dropped_km"], 2)
    st["blocks"] = len(blocks)
    st["streets_added"] = len(data.get("streets", []))
    st["buildings_added"] = len(data.get("buildings", []))
    stats["core"] = st


def pave_districts(g, site, terrain, water, masks, stats):
    """Prompt 112: districts with `paved: true` (a waterfront of quays,
    sheds and tracks) are paved ground (`road`) in full, without parks,
    woods, scrub or pitches."""
    paved = bytearray(g.N)
    for d in site["districts"]:
        if not d.get("paved"):
            continue
        ring = [tuple(p) for p in d["polygon_m"]] + [tuple(d["polygon_m"][0])]
        for j, i0, i1 in polygon_spans(g, [ring]):
            paved[j * g.W + i0:j * g.W + i1 + 1] = b"\x01" * (i1 - i0 + 1)
    n = 0
    for k in range(g.N):
        if paved[k] and not water[k]:
            if terrain[k] != ROAD:
                n += 1
            terrain[k] = ROAD
            for name in ("park", "wood", "scrub", "pitch"):
                masks[name][k] = 0
    masks["paved"] = paved
    stats["paved_districts_m2"] = n * 4


# --------------------------------------------------------------------------
# Output


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
    base, step = height_encoding(h)
    rows = []
    for j in range(g.H):
        row = bytearray()
        for v in mm[j * g.W:(j + 1) * g.W]:
            row += struct.pack(">H", min(65535, max(0, rint((v - base) / step))))
        rows.append(bytes(row))
    write_png(out / "height.png", g.W, g.H, rows, 0, 16)
    palette = [c for rgb in terrain_palette() for c in rgb]
    write_png(out / "terrain.png", g.W, g.H, [bytes(terrain[j * g.W:(j + 1) * g.W]) for j in range(g.H)], 3, 8, palette)
    # Prompt 116: name the Dutch terrain model where a site uses it (the
    # other sites keep their header, so they regenerate bit for bit).
    dem_name = "AHN (CC0)" if site["elevation"].get("source") == "ahn" else "Copernicus DEM"
    lines = ["# Generated by tools/geo_import.py from OpenStreetMap (ODbL) and %s data" % dem_name,
             "# (prompt 081, docs/MAPS.md, docs/SOURCES.md); regenerate with `just gen-geo %s`." % site["name"],
             "kind,variant,x_mm,y_mm,heading_deg"]
    (out / "objects.csv").write_text("\n".join(lines + rows_csv) + "\n", encoding="utf-8")
    m = site["map"]
    toml = [
        "# Generated by tools/geo_import.py (prompt 081) from OpenStreetMap and %s data;" % dem_name,
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


def draw_objects(g, rgb, rows_csv, placer):
    """Buildings as light boxes with a dark rim, trees as dark dots."""
    kinds = sorted({r.split(",")[0] for r in rows_csv if not r.startswith("env_tree") and not r.startswith("env_bush")})
    sizes = catalog_sizes(kinds)
    for r in rows_csv:
        kind, variant, x, y, heading = r.split(",")
        x, y, heading = int(x) / 1000.0, int(y) / 1000.0, int(heading)
        if kind.startswith("env_tree"):
            col = b"\x2e\x55\x22" if kind != "env_tree_palm" else b"\x6a\x8a\x2a"
            for k in placer.box(x, y, 0, 3.0, 3.0, 0.0):
                rgb[3 * k:3 * k + 3] = col
        elif kind in ("env_bush", "env_bush_macchia"):
            k = g.index(x, y)
            if k >= 0:
                rgb[3 * k:3 * k + 3] = b"\x50\x70\x30"
        else:
            length, width, _ = sizes[kind]
            rim = set(placer.box(x, y, heading, length, width, 0.0)) - set(placer.box(x, y, heading, length - 3.0, width - 3.0, 0.0))
            fill = b"\xee\xec\xe6" if variant == "Intact" else (b"\xb8\xa8\x98" if variant == "Damaged" else b"\x80\x70\x60")
            if kind == "env_hedge":
                fill = b"\x30\x48\x20"
            for k in placer.box(x, y, heading, length, width, 0.0):
                rgb[3 * k:3 * k + 3] = b"\x60\x60\x60" if k in rim and kind != "env_hedge" else fill


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

    if mediterranean(site):
        med_classes(site)
    f = Features(g, osm)
    drop_areas(f, site["areas"], stats)
    if site["buildings"].get("skip_tags"):
        skip_footprints(f, site["buildings"]["skip_tags"], stats)
    if "drop_from_year" in site["buildings"]:
        drop_postwar(g, f, site, stats)
    if site.get("core"):
        apply_core(g, f, site, site_path.parent, stats)
    lap("features")
    if dem["format"] in ("geo_import dem utm v1", "geo_import dem rd v1"):
        h = dgm_heights(g, dem, site["elevation"], stats)
    else:
        roof = roof_raster(g, f, site["elevation"])
        h = dem_heights(g, dem, site["elevation"], roof)
    if "offset_m" in site["elevation"]:
        # Prompt 116: a river site puts its water level at 0 (heights over
        # the river instead of over the sea).
        off = site["elevation"]["offset_m"]
        h = [v + off for v in h]
    for d in site["coast"].get("dry_polygons_m", []):
        # Ground built up after 1944 inside (an island, the banks of a new
        # channel) is held at the old floodplain's height.
        if "max_m" in d:
            for j, i0, i1 in polygon_spans(g, [d["polygon_m"]]):
                for k in range(j * g.W + i0, j * g.W + i1 + 1):
                    if h[k] > d["max_m"]:
                        h[k] = d["max_m"]
    lap("elevation")
    terrain = bytearray(g.N)
    water = build_coast(g, f, site["coast"], stats)
    tidal = tidal_flats(g, f, site["coast"], water, stats) if "tidal_flat" in site["coast"] else None
    lap("coast")
    shape_sea(g, f, site["coast"], h, terrain, water)
    if tidal:
        shape_tidal_flats(g, site["coast"], h, terrain, tidal[0], tidal[1], water)
    lap("sea")
    for strip in site["coast"].get("land_strips_m", []):
        land_strip(g, strip, h, terrain, water)
    flatten_pads(g, site["pads"], h, water=water)
    lap("pads")
    if site.get("long_bridges"):
        drop_bridge_ways(g, f, site, stats)
    roads = select_roads(g, f, site["roads"], stats, water)
    if "causeway_max_m" in site["roads"]:
        build_causeways(g, roads, site["roads"], h, terrain, water, stats)
    road = build_roads(g, roads, site["roads"], h, terrain, water, stats)
    if site.get("core"):
        core_streets(g, site, site_path.parent, terrain, water, road, stats)
    lap("roads")
    masks, kept = paint_areas(g, f, site["areas"], terrain, water, road, med=mediterranean(site))
    stats["areas"] = kept
    if any(d.get("paved") for d in site["districts"]):
        pave_districts(g, site, terrain, water, masks, stats)
    lap("areas")
    h1, terrain1 = list(h), bytearray(terrain)
    reserved = bytearray(g.N)
    for p in site["pads"]:
        reserve_circle(g, reserved, p["center_m"], p.get("reserve_m", p["radius_m"]))
    for name, pos in site["markers"]:
        if name.startswith("supply"):
            reserve_circle(g, reserved, pos, site["buildings"]["supply_free_m"])
    bridge_rows = build_long_bridges(g, site, h, terrain, water, reserved, stats) if site.get("long_bridges") else []
    # The smallest block that gets a building rises until no more than
    # `max_buildings` stand (prompt 081: about 1 500).
    for min_area in site["buildings"]["min_area_steps_m2"]:
        placer = Placer(g, road, water, reserved)
        if site.get("core"):
            core_landmarks(g, site, site_path.parent, placer, stats)
        placed = place_buildings(g, f, site, h, water, road, masks, placer, stats, min_area)
        if not quiet:
            print("  blocks from %d m2: %d buildings" % (min_area, len(placed)), file=sys.stderr)
        if len(placed) <= site["buildings"]["max_buildings"]:
            break
    if site.get("sheds"):
        placed = placed + place_sheds(g, site, h, placer, stats)
    lap("buildings")
    h_before = list(h)
    encoding = height_encoding(h)
    stats["buildings"]["pads_levelled"], stats["buildings"]["pads_left_out"] = build_pads(
        g, placed, h, road, water, site["buildings"], encoding)
    lap("pads")
    removed = keep_ways_open(g, site, h, h_before, terrain, placed, site["buildings"], stats, encoding)
    if height_encoding(h) != encoding:
        raise SystemExit("height range changed by the pads")
    if removed:
        placer.rows = [r for n, r in enumerate(placer.rows) if n not in removed]
        placed = [p for n, p in enumerate(placed) if n not in removed]
        placer.occupied = bytearray(g.N)
        for x, y, heading, length, width in placed:
            placer.take(placer.box(x, y, heading, length, width, 0.5))
    stats["buildings"]["totals"]["placed"] = len(placed)
    pave_town(g, site, placed, terrain, water, masks, stats)
    if site.get("fortifications"):
        place_fortifications(g, site, placer, stats)
    lap("open ways")
    place_green(g, f, site, masks, placer, terrain, stats)
    lap("green")
    # Bridges first (docs/MAPS.md, "Objects").
    rows_csv = bridge_rows + placer.rows
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
        rgb = shade_rgb(g, h1, lambda k: pal[terrain1[k]])
        write_rgb(rep / "stage1.png", g.W, g.H, rgb)
        grid_overlay(g, rgb)
        mark_points(g, rgb, [(s["position_m"], b"\xff\xff\xff") for s in site["starts"]]
                    + [(p, b"\xff\xe0\x00" if n.startswith("supply") else b"\x40\xc0\xff") for n, p in site["markers"]])
        write_rgb(rep / "stage1_grid.png", g.W, g.H, rgb)
        rgb = shade_rgb(g, h, lambda k: pal[terrain[k]])
        draw_objects(g, rgb, rows_csv, placer)
        write_rgb(rep / "stage2.png", g.W, g.H, rgb)
        lap("report")
    stats["time_s"] = timing
    return stats


def dem_bilinear(g, dem):
    """Copernicus heights at every sample, bilinear, without any filter
    (prompt 114: the comparison with the official terrain model)."""
    cols, rows = dem["cols"], dem["rows"]
    v = [x / 100.0 for x in dem["values"]]
    px = dem["px_per_deg"]
    pxx = dem.get("px_per_deg_lon", px)
    c0 = ((g.lon7 / 1e7 - dem["lon0_deg"]) * pxx) - dem["col0"] - 0.5
    r0 = ((dem["lat0_deg"] - g.lat7 / 1e7) * px) - dem["row0"] - 0.5
    cx, cy = g.ca / g.mlon * pxx, g.sa / g.mlon * pxx
    rx, ry = -g.sa / g.mlat * px, g.ca / g.mlat * px
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
            a = v[q] + (v[q + 1] - v[q]) * tc
            b = v[q + cols] + (v[q + cols + 1] - v[q + cols]) * tc
            h[j * g.W + i] = a + (b - a) * tr
    return h


def _relief_stats(g, h, land):
    """Mean slope (%) and the share of land steeper than 60 % (2 m)."""
    total, steep, n = 0.0, 0, 0
    for j in range(1, g.H - 1):
        for i in range(1, g.W - 1):
            k = j * g.W + i
            if not land[k]:
                continue
            dx = (h[k + 1] - h[k - 1]) / 4.0
            dy = (h[k + g.W] - h[k - g.W]) / 4.0
            s = math.sqrt(dx * dx + dy * dy)
            total += s
            steep += s > 0.6
            n += 1
    return {"mean_slope_pct": round(100.0 * total / max(n, 1), 1),
            "steeper_than_60_pct_share": round(100.0 * steep / max(n, 1), 2)}


def _draw_line(rgb, w, hgt, a, b, color):
    (x0, y0), (x1, y1) = a, b
    n = max(1, int(max(abs(x1 - x0), abs(y1 - y0))))
    for s in range(n + 1):
        x = rint(x0 + (x1 - x0) * s / n)
        y = rint(y0 + (y1 - y0) * s / n)
        for dy in (0, 1):
            if 0 <= x < w and 0 <= y + dy < hgt:
                o = 3 * ((y + dy) * w + x)
                rgb[o:o + 3] = color


def cmd_compare_dem(args):
    """Prompt 114: the official terrain model against the Copernicus DEM
    on the same frame: both reliefs as pictures (same colors), a profile
    across the island at `--profile-x` (map meters) as picture and CSV,
    mean slope and steep share on the land of the map."""
    site_path = pathlib.Path(args.site)
    site = json.loads(site_path.read_text(encoding="utf-8"))
    g = Grid(site["frame"])
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    stats = {}
    dgm = dgm_heights(g, read_gz_json(site_path.parent / "dem.json.gz"), site["elevation"], stats)
    bbox = [parse_deg(v) for v in site["sources"]["bbox_deg"]]
    cop = dem_bilinear(g, extract_dem(args.dem, bbox))
    with tempfile.TemporaryDirectory() as tmp:
        generate(site_path, tmp, None, quiet=True)
        terrain = png_content((pathlib.Path(tmp) / "terrain.png").read_bytes())[1]
    # One filter byte (Sub, as `write_png` writes) per row, then one byte
    # per sample.
    land = bytearray(g.N)
    for j in range(g.H):
        row = terrain[j * (g.W + 1) + 1:(j + 1) * (g.W + 1)]
        prev = 0
        for i in range(g.W):
            prev = (row[i] + prev) & 0xFF
            land[j * g.W + i] = prev not in (SHALLOW, DEEP, MUD)
    hi = max(max(dgm), max(cop))

    def colors(h):
        def c(k):
            if not land[k]:
                return (70, 100, 140)
            t = min(max(h[k] / hi, 0.0), 1.0)
            return (int(110 + 130 * t), int(150 - 30 * t), int(80 + 40 * t))
        return c
    x0 = float(args.profile_x)
    i0 = rint((x0 + g.hx) / 2.0)
    pics = []
    for h in (cop, dgm):
        rgb = shade_rgb(g, h, colors(h))
        for j in range(g.H):
            o = 3 * (j * g.W + i0)
            rgb[o:o + 3] = b"\xff\xff\xff"
        pics.append(rgb)
    gap = 12
    both = pics[0] + bytearray(b"\xff\xff\xff" * (g.W * gap)) + pics[1]
    write_rgb(out / "dem_compare.png", g.W, 2 * g.H + gap, both)
    # Profile along y at x0.
    lines = ["y_m,copernicus_m,dgm1_m,land"]
    prof = []
    for j in range(g.H):
        k = j * g.W + i0
        prof.append((g.y(j), cop[k], dgm[k], land[k]))
        lines.append("%d,%.2f,%.2f,%d" % (rint(g.y(j)), cop[k], dgm[k], land[k]))
    (out / "dem_profile.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")
    pw, ph, top = 1600, 520, 60.0
    rgb = bytearray(b"\xff\xff\xff" * (pw * ph))
    lo = -5.0

    def px(y, v):
        return ((y + g.hy) / (2.0 * g.hy) * (pw - 1), (ph - 1) - (v - lo) / (top - lo) * (ph - 1))
    for level in range(-5, 61, 5):
        _, yy = px(0.0, level)
        _draw_line(rgb, pw, ph, (0, yy), (pw - 1, yy), b"\x40\x40\x40" if level == 0 else b"\xdd\xdd\xdd")
    for ym in range(-rint(g.hy), rint(g.hy) + 1, 200):
        xx, _ = px(ym, 0.0)
        _draw_line(rgb, pw, ph, (xx, 0), (xx, ph - 1), b"\xdd\xdd\xdd")
    for col, idx in ((b"\xe0\x80\x20", 1), (b"\x20\x40\x90", 2)):
        for a, b in zip(prof, prof[1:]):
            _draw_line(rgb, pw, ph, px(a[0], a[idx]), px(b[0], b[idx]), col)
    write_rgb(out / "dem_profile.png", pw, ph, rgb)
    result = {"copernicus": _relief_stats(g, cop, land), "dgm1": _relief_stats(g, dgm, land),
              "profile_x_m": x0, "dgm": stats["dgm"]}
    print(json.dumps(result, indent=1))


def cmd_generate(args):
    site = json.loads(pathlib.Path(args.site).read_text(encoding="utf-8"))
    out = args.out or str(ROOT / "data" / "maps" / site["name"])
    stats = generate(args.site, out, args.report, quiet=args.quiet)
    if not args.quiet:
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
        # Two independent processes at once (same sources, same files).
        procs = [subprocess.Popen([sys.executable, "-I", str(pathlib.Path(__file__).resolve()), "generate",
                                   str(args.site), "--out", str(pathlib.Path(tmp) / str(k)), "--quiet"],
                                  stdout=subprocess.DEVNULL) for k in range(2)]
        if any(p.wait() != 0 for p in procs):
            raise SystemExit("geo check: generating failed")
        runs = [{f: (pathlib.Path(tmp) / str(k) / f).read_bytes() for f in files} for k in range(2)]
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
    p.add_argument("--osm", required=True, nargs="+", help="one Overpass answer, or several")
    dem = p.add_mutually_exclusive_group(required=True)
    dem.add_argument("--dem", nargs="+", help="one Copernicus tile, or neighbouring tiles")
    dem.add_argument("--dgm", nargs="+", help="official terrain model tiles in UTM (x y z text, prompt 114)")
    dem.add_argument("--ahn", nargs="+", help="AHN DTM GeoTIFF tiles in RD New (PDOK coverage service, prompt 116)")
    p = sub.add_parser("generate")
    p.add_argument("site")
    p.add_argument("--out")
    p.add_argument("--report")
    p.add_argument("--quiet", action="store_true")
    p = sub.add_parser("check")
    p.add_argument("site")
    p = sub.add_parser("compare-dem")
    p.add_argument("site")
    p.add_argument("--dem", required=True, nargs="+", help="Copernicus tiles over the site")
    p.add_argument("--out", required=True)
    p.add_argument("--profile-x", default="0", help="x (map meters) of the profile across the map")
    args = parser.parse_args()
    {"extract": cmd_extract, "generate": cmd_generate, "check": cmd_check,
     "compare-dem": cmd_compare_dem}[args.cmd](args)


if __name__ == "__main__":
    main()
