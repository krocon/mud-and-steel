"""Desert village houses (prompt 059): flat-roofed mud-brick and plastered
houses of North African villages, built by the generator `desert_house`.

One data-driven generator builds all house types (`env_house_desert_*`):
the plan is a list of rectangular blocks (rooms) with their own height,
plus options for doors, windows, an outside stair, rooms on the roof,
barrel vaults, an arcade and a wooden balcony. Like `buildings.house`, one
variant is one damage level (`damage` 0 intact, 1 damaged, 2 ruin) and is
built from the same plan, so the three variants line up.

Conventions: length along Y, width along X, the main door / courtyard side
is +X. All plan coordinates in the blueprint are meters from the box's -X/-Y
corner (x = 0..width, y = 0..length). A block side on the box boundary is
set back by `skirt`, so that the battered wall base, the palm beam ends,
the drain spouts and the sand drifts stay inside the box; the base reaches
the box edge exactly, so `fit` does not distort the intact variant.
"""

import math

from .buildings import DOOR_H, House, Opening, _blast
from .env import env_part, rng

SIDES = ("+x", "-x", "+y", "-y")


def _sgn(side):
    return 1 if side[0] == "+" else -1


def _opp(side):
    return ("-" if side[0] == "+" else "+") + side[1]


class Top:
    """Top line of a damaged wall: `top(u)` -> z, with the u where it steps."""

    def __init__(self, fn, cuts):
        self.fn, self.cuts = fn, cuts

    def __call__(self, u):
        return self.fn(u)


class Block:
    """One room block: outer wall faces x0..x1, y0..y1 (model space)."""

    def __init__(self, idx, rect, h, base, level, kind):
        self.idx = idx
        self.x0, self.y0, self.x1, self.y1 = rect
        self.h, self.base, self.level, self.kind = h, base, level, kind
        self.gone = False

    def coord(self, side):
        return {"+x": self.x1, "-x": self.x0, "+y": self.y1, "-y": self.y0}[side]

    def span(self, side):
        return (self.y0, self.y1) if side[1] == "x" else (self.x0, self.x1)

    def contains(self, x, y):
        return self.x0 <= x <= self.x1 and self.y0 <= y <= self.y1


class DesertHouse(House):
    def __init__(self, model, name, p, box):
        super().__init__(model, name, p, box, "Plaster", p.get("plaster", 1))
        self.p = p
        self.d = rng(p.get("seed", 1) * 31 + 7)  # design randomness, same for every damage level
        self.hw, self.hl = box.hw, box.hl
        self.t = p.get("wall_t", 0.45)
        self.e = p.get("skirt", 0.3)
        self.tp = 0.4  # arcade front
        self.end_t = p.get("end_t", 0.7)
        self.parapet = p.get("parapet", 0.5)
        self.storey_h = p.get("storey_h", 3.0)
        self.roof_kind = p.get("roof_kind", 1)
        self.skirt_kind = p.get("skirt_kind", 1)
        c = p.get("coping", -1)
        self.coping = None if c is None or c < 0 else c
        self.win = p.get("window", [0.55, 0.65])
        self.sill = p.get("sill", 1.2)
        self.sill_up = p.get("sill_up", 0.9)
        self.beam_step = p.get("beam_step", 0.7)
        self.floor_beams = p.get("floor_beams", True)
        self.spring = p.get("vault_spring", 2.3)
        self.arches = p.get("arches", 4)
        self.ops = {}
        self.tops = {}
        self.stair = None
        self.ext = []  # exterior wall segments for decoration
        self._parse()

    # -- plan ---------------------------------------------------------------------

    def _parse(self):
        p, box = self.p, self.box
        W, L, e = box.width, box.length, self.e
        raw = p.get("blocks")
        if not raw:
            raise ValueError("desert_house needs blocks = [[x0, y0, x1, y1, h], ...]")
        vaults = set(p.get("vaults", []))
        arcade = p.get("arcade", -1)
        parapets = p.get("parapets", [])
        self.blocks = []
        for i, (x0, y0, x1, y1, h) in enumerate(raw):
            rect = (x0 - self.hw + (e if x0 < 0.01 else 0.0), y0 - self.hl + (e if y0 < 0.01 else 0.0),
                    x1 - self.hw - (e if x1 > W - 0.01 else 0.0), y1 - self.hl - (e if y1 > L - 0.01 else 0.0))
            kind = "vault" if i in vaults else "arcade" if i == arcade else "room"
            b = Block(i, rect, h, 0.0, 0, kind)
            b.parapet = parapets[i] if i < len(parapets) else self.parapet
            b.roof_z = None if kind == "vault" else h - b.parapet
            b.storeys = 2 if kind == "room" and b.roof_z > 5.0 else 1
            self.blocks.append(b)
        n0 = len(self.blocks)
        for j, (x0, y0, x1, y1, h) in enumerate(p.get("roof_rooms", [])):
            rect = (x0 - self.hw, y0 - self.hl, x1 - self.hw, y1 - self.hl)
            cx, cy = (rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2
            host = next((b for b in self.blocks[:n0] if b.contains(cx, cy) and b.roof_z), None)
            if host is None:
                raise ValueError(f"roof room {j}: no flat roof below")
            b = Block(n0 + j, rect, h, host.roof_z, 1, "room")
            k = n0 + j
            b.parapet = parapets[k] if k < len(parapets) else self.parapet
            b.roof_z = h - b.parapet
            b.storeys = 1
            b.gone = self.damage >= 2
            self.blocks.append(b)

    def thick(self, b, side):
        if b.kind == "vault" and side[1] == "x":
            return self.end_t
        if b.kind == "arcade" and side == "+x":
            return self.tp
        return self.t

    def wall_top(self, b, side):
        return self.spring if b.kind == "vault" else b.h

    def owns(self, b, o):
        if (b.kind == "arcade") != (o.kind == "arcade"):
            return o.kind == "arcade"
        if abs(b.h - o.h) > 1e-3:
            return b.h > o.h
        return b.idx < o.idx

    def raw_segments(self, b, side):
        """Pieces of a block side: (u0, u1, "ext" | "own" | "skip", neighbor)."""
        coord, opp = b.coord(side), _opp(side)
        u0, u1 = b.span(side)
        shared = []
        for o in self.blocks:
            if o is b or o.level != b.level or o.gone:
                continue
            if abs(o.coord(opp) - coord) > 0.02:
                continue
            a0, a1 = o.span(opp)
            lo, hi = max(u0, a0), min(u1, a1)
            if hi - lo < 0.05:
                continue
            shared.append((lo, hi, "own" if self.owns(b, o) else "skip", o))
        shared.sort(key=lambda s: s[0])
        segs, u = [], u0
        for lo, hi, k, o in shared:
            if lo > u + 0.01:
                segs.append((u, lo, "ext", None))
            segs.append((lo, hi, k, o))
            u = hi
        if u < u1 - 0.01:
            segs.append((u, u1, "ext", None))
        return segs

    def seg_kind(self, b, side, u):
        for a, c, k, _o in self.raw_segments(b, side):
            if a - 1e-4 <= u <= c + 1e-4:
                return k
        return None

    def built(self, b, side):
        return any(k != "skip" for _a, _c, k, _o in self.raw_segments(b, side))

    def segments(self, b, side):
        """Segments with the corner set-back of the ±y walls."""
        segs = self.raw_segments(b, side)
        if side[1] == "x":
            return segs
        y = b.coord(side) - _sgn(side) * 0.05
        lo = b.x0 + (self.thick(b, "-x") if self.seg_kind(b, "-x", y) not in (None, "skip") else 0.0)
        hi = b.x1 - (self.thick(b, "+x") if self.seg_kind(b, "+x", y) not in (None, "skip") else 0.0)
        out = []
        for a, c, k, o in segs:
            a, c = max(a, lo), min(c, hi)
            if c - a > 0.03:
                out.append((a, c, k, o))
        return out

    def inner(self, b):
        """Inner rectangle (inside the walls this block builds)."""
        return (b.x0 + (self.thick(b, "-x") if self.built(b, "-x") else 0.0),
                b.x1 - (self.thick(b, "+x") if self.built(b, "+x") else 0.0),
                b.y0 + (self.thick(b, "-y") if self.built(b, "-y") else 0.0),
                b.y1 - (self.thick(b, "+y") if self.built(b, "+y") else 0.0))

    def u_of(self, side, v):
        """Box coordinate along a side -> model u."""
        return v - (self.hl if side[1] == "x" else self.hw)

    # -- openings -------------------------------------------------------------------

    def add_op(self, b, side, o):
        self.ops.setdefault((b.idx, side), []).append(o)

    def openings(self):
        p = self.p
        ww, wh = self.win
        for entry in p.get("doors", []):
            i, side, v, w = entry[:4]
            st = entry[4] if len(entry) > 4 else 0
            dh = entry[5] if len(entry) > 5 else DOOR_H
            b = self.blocks[i]
            z0 = b.base + st * self.storey_h + (0.0 if st == 0 else 0.05)
            o = Opening(self.u_of(side, v), w, z0, z0 + dh, "door")
            o.wood = self.d.choice((0, 0, 2))
            o.storey = st
            self.add_op(b, side, o)
        for entry in p.get("windows", []):
            i, side, v = entry[:3]
            st = entry[3] if len(entry) > 3 else 0
            b = self.blocks[i]
            z0 = b.base + st * self.storey_h + (self.sill if st == 0 else self.sill_up)
            o = Opening(self.u_of(side, v), ww, z0, z0 + wh, "window")
            o.style = self.d.choices((0, 1, 2), weights=(3, 3, 2))[0]
            o.wood = self.d.choice((0, 2))
            o.storey = st
            self.add_op(b, side, o)

    def damage_plan(self):
        """Shell holes, blasted windows, broken parapets, ruin tops."""
        r = self.r
        live = [b for b in self.blocks if not b.gone]
        if self.damage == 1:
            wins = [o for k, v in self.ops.items() for o in v if o.kind == "window"]
            _blast(self, wins, 1 + (len(wins) > 4))
            # shell holes in long exterior walls of ground blocks
            cands = []
            for b in live:
                if b.level or b.kind == "arcade":
                    continue
                for side in SIDES:
                    for a, c, k, _o in self.segments(b, side):
                        if k == "ext" and c - a > 2.0:
                            cands.append((b, side, a, c))
            r.shuffle(cands)
            self.shell_holes = []
            for b, side, a, c in cands[:2]:
                ztop = (b.roof_z if b.kind != "vault" else self.spring) - 0.35
                for _try in range(6):
                    w = r.uniform(0.8, 1.4)
                    u = r.uniform(a + w / 2 + 0.3, c - w / 2 - 0.3)
                    z0 = r.uniform(0.5, 1.2)
                    z1 = min(z0 + r.uniform(0.8, 1.3), ztop)
                    if any(abs(u - o.u) < (w + o.w) / 2 + 0.25 for o in self.ops.get((b.idx, side), [])):
                        continue
                    o = Opening(u, w, z0, z1, "hole")
                    self.add_op(b, side, o)
                    self.shell_holes.append((b, side, u))
                    break
            # broken parapets
            for b in live:
                if b.kind == "vault":
                    continue
                for side in SIDES:
                    if r.random() < 0.45:
                        u0, u1 = b.span(side)
                        notches = []
                        for _n in range(r.choice((1, 1, 2))):
                            uc = r.uniform(u0 + 0.5, u1 - 0.5)
                            w = r.uniform(0.6, 1.5)
                            notches.append((uc - w / 2, uc + w / 2, b.roof_z + r.uniform(-0.05, 0.15)))
                        self.tops[(b.idx, side)] = Top(
                            lambda u, n=notches, z=b.h: min([zz for a, c, zz in n if a <= u <= c], default=z),
                            [v for a, c, _z in notches for v in (a, c)])
        elif self.damage >= 2:
            for b in live:
                zr = b.roof_z if b.kind != "vault" else self.spring
                for side in SIDES:
                    u0, u1 = b.span(side)
                    pts = self.ruin_top(b.base, zr, u0 - 1, u1 + 1)
                    self.tops[(b.idx, side)] = Top(lambda u, pts=pts: min(pts, key=lambda q: abs(q[0] - u))[1],
                                                   [(p[0] + q[0]) / 2 for p, q in zip(pts, pts[1:])])

    # -- primitives -----------------------------------------------------------------

    @staticmethod
    def _pt(axis, n, u, z):
        return (n, u, z) if axis == "x" else (u, n, z)

    def _box(self, mb, axis, n, u, z, dn, du, dz, mat, kind=None):
        if kind is not None:
            mb.style(kind=kind)
        size = (dn, du, dz) if axis == "x" else (du, dn, dz)
        mb.box(self._pt(axis, n, u, z), size, mat)

    def _wbox(self, mb, axis, n, u, za, zb, dn, du, mat, bottom=True, top=True):
        """Axis-aligned wall piece; hidden bottom / top faces can be left out."""
        size = (dn, du) if axis == "x" else (du, dn)
        cx, cy, _z = self._pt(axis, n, u, 0.0)
        hx, hy = size[0] / 2, size[1] / 2
        v = mb.verts([(cx + sx * hx, cy + sy * hy, z) for z in (za, zb) for sy in (-1, 1) for sx in (-1, 1)])
        quads = [(0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
        if bottom:
            quads.append((0, 2, 3, 1))
        if top:
            quads.append((4, 5, 7, 6))
        for q in quads:
            mb.face([v[i] for i in q], mat)

    def dwall(self, axis, coord, sgn, u0, u1, z0, z1, ops, t, top=None, coping=None):
        """Straight wall with openings (stacked openings allowed); `top`
        (a `Top`) clips the wall (broken parapets, ruins). Outer face at
        `coord`. Equal neighbouring pieces are merged to save triangles."""
        mb = self.walls
        nc = coord - sgn * t / 2
        ops = [o for o in ops if o.hi > u0 + 0.03 and o.lo < u1 - 0.03 and o.z0 < z1 - 0.05]
        cuts = {u0, u1} | {min(max(v, u0), u1) for o in ops for v in (o.lo, o.hi)}
        if top is not None:
            cuts |= {u for u in top.cuts if u0 < u < u1}
        cuts = sorted(cuts)
        rects, caps = [], []
        for ua, ub in zip(cuts, cuts[1:]):
            if ub - ua < 0.015:
                continue
            um = (ua + ub) / 2
            holes = sorted((o.z0, o.z1) for o in ops if o.lo < um < o.hi)
            fills, z = [], z0
            for a, b in holes:
                if a > z + 0.02:
                    fills.append((z, a))
                z = max(z, b)
            if z < z1 - 0.02:
                fills.append((z, z1))
            zt = z1 if top is None else min(z1, top(um))
            cap = coping is not None and zt >= z1 - 1e-6 and fills and fills[-1][1] >= z1 - 1e-6
            for a, b in fills:
                b = min(b, zt)
                if cap and b >= z1 - 1e-6:
                    b = z1 - 0.08
                if b - a >= 0.04:
                    rects.append((round(a, 4), round(b, 4), ua, ub))
            if cap:
                caps.append((0, 0, ua, ub))

        def merged(items):
            out = []
            for a, b, ua, ub in sorted(items):
                if out and out[-1][0] == a and out[-1][1] == b and abs(out[-1][3] - ua) < 1e-4:
                    out[-1] = (a, b, out[-1][2], ub)
                else:
                    out.append((a, b, ua, ub))
            return out

        mb.style(kind=self.wall_kind)
        for a, b, ua, ub in merged(rects):
            hid_top = coping is not None and abs(b - (z1 - 0.08)) < 1e-3 and any(
                c0 <= ua + 1e-4 and ub <= c1 + 1e-4 for _p, _q, c0, c1 in merged(caps))
            self._wbox(mb, axis, nc, (ua + ub) / 2, a, b, t, ub - ua, "Plaster", bottom=a > z0 + 1e-3,
                       top=not hid_top)
        for _a, _b, ua, ub in merged(caps):
            self._box(mb, axis, nc, (ua + ub) / 2, z1 - 0.04, t + 0.06, ub - ua, 0.08, "Plaster", coping)

    def fill(self, axis, coord, sgn, o, t, top, wall_u):
        """Doors, shutters, grilles and lintels of one opening."""
        if self.damage >= 2 or o.kind in ("hole", "gap"):
            return
        if top is not None and top(o.u) < o.z1 + 0.15:
            return
        mb = self.walls
        lint = o.z1 + 0.06
        self._box(mb, axis, coord - sgn * t / 2, o.u, lint, t + 0.02, o.w + 0.3, 0.12, "Wood", 2)
        if o.kind == "door":
            n = coord - sgn * t * 0.55
            self._box(mb, axis, n, o.u, (o.z0 + o.z1) / 2, 0.06, o.w, o.z1 - o.z0, "Wood", o.wood)
            # battens and plank seams
            for zz in (o.z1 - 0.45,):
                self._box(mb, axis, n + sgn * 0.035, o.u, zz, 0.02, o.w * 0.9, 0.1, "Wood", 2 if o.wood != 2 else 0)
            return
        h = o.z1 - o.z0
        if o.style == 1 and (o.lo - o.w / 2 - 0.05 < wall_u[0] or o.hi + o.w / 2 + 0.05 > wall_u[1]):
            o.style = 2
        if o.style == 0:  # closed shutters
            n = coord - sgn * t * 0.3
            for s in (-1, 1):
                self._box(mb, axis, n, o.u + s * o.w / 4, (o.z0 + o.z1) / 2, 0.05, o.w / 2 - 0.02, h - 0.02,
                          "Wood", o.wood)
            return
        # dark room behind the opening
        self._box(mb, axis, coord - sgn * t * 0.85, o.u, (o.z0 + o.z1) / 2, 0.04, o.w, h, "Wood", 3)
        if o.style == 1:  # shutters open against the wall
            n = coord + sgn * 0.02
            for s in (-1, 1):
                self._box(mb, axis, n, o.u + s * (o.w * 0.75 + 0.02), (o.z0 + o.z1) / 2, 0.035, o.w / 2, h,
                          "Wood", o.wood)
        else:  # simple grille
            n = coord - sgn * t * 0.3
            for k in (-1, 1):
                self._box(mb, axis, n, o.u + k * o.w / 6, (o.z0 + o.z1) / 2, 0.03, 0.03, h, "Metal", 3)
            self._box(mb, axis, n, o.u, (o.z0 + o.z1) / 2, 0.03, o.w, 0.03, "Metal", 3)

    # -- walls ----------------------------------------------------------------------

    def build_walls(self):
        for b in self.blocks:
            if b.gone:
                continue
            for side in SIDES:
                if b.kind == "arcade" and side == "+x":
                    self.arch_wall(b)
                    continue
                axis, sgn, coord = side[1], _sgn(side), b.coord(side)
                t = self.thick(b, side)
                top = self.tops.get((b.idx, side))
                z1 = self.wall_top(b, side)
                cop = self.coping if b.kind != "vault" else None
                for a, c, k, o in self.segments(b, side):
                    if k == "skip":
                        continue
                    ops = [op for op in self.ops.get((b.idx, side), []) if a - 0.01 <= op.u <= c + 0.01]
                    if k == "own":
                        ops += [op for op in self.ops.get((o.idx, _opp(side)), []) if a <= op.u <= c]
                    self.dwall(axis, coord, sgn, a, c, b.base, z1, ops, t, top, cop)
                    for op in ops:
                        self.fill(axis, coord, sgn, op, t, top, (a, c))
                    if b.kind == "vault" and axis == "x":
                        self.tympanum(b, side, top)
                    if k == "ext":
                        self.ext.append((b, side, a, c, ops))

    def arch_wall(self, b):
        """Arcade front (+x): piers and round arches up to the parapet."""
        mb = self.walls
        coord, tp = b.x1, self.tp
        u0, u1 = b.y0, b.y1
        n, pw = self.arches, 0.5
        a = (u1 - u0 - (n + 1) * pw) / n
        r = a / 2
        zs = b.roof_z - 0.3 - r
        top = self.tops.get((b.idx, "+x"))
        cop = self.coping
        for i in range(n + 1):
            pu = u0 + i * (pw + a)
            self.dwall("x", coord, 1, pu, pu + pw, b.base, b.h, [], tp, top, cop)
            if self.damage < 2:  # impost blocks
                for s in ((1,) if i == 0 else (-1,) if i == n else (-1, 1)):
                    uu = pu + pw / 2 + s * (pw / 2 + 0.04)
                    self._box(mb, "x", coord - tp / 2, uu, zs - 0.06, tp + 0.04, 0.08, 0.12, "Plaster", self.wall_kind)
        if self.damage >= 2:
            return
        K = 6
        for i in range(n):
            lo = u0 + pw + i * (pw + a)
            uc = lo + r
            for k in range(K):
                ua, ub = lo + a * k / K, lo + a * (k + 1) / K
                za = zs + math.sqrt(max(0.0, r * r - (ua - uc) ** 2))
                zb = zs + math.sqrt(max(0.0, r * r - (ub - uc) ** 2))
                zt = b.h if top is None else min(b.h, top((ua + ub) / 2))
                cap = cop is not None and zt >= b.h - 1e-6
                ze = zt - 0.08 if cap else zt
                if ze - max(za, zb) < 0.05:
                    continue
                mb.style(kind=self.wall_kind)
                mb.extrude([(coord, ua, za), (coord, ub, zb), (coord, ub, ze), (coord, ua, ze)], (-tp, 0, 0),
                           "Plaster")
                if cap:
                    self._box(mb, "x", coord - tp / 2, (ua + ub) / 2, b.h - 0.04, tp + 0.06, ub - ua, 0.08,
                              "Plaster", cop)
        self.ext.append((b, "+x", u0, u1, []))

    # -- vaults ---------------------------------------------------------------------

    def _arc(self, y0, y1, zs, rise, n):
        ym, a = (y0 + y1) / 2, (y1 - y0) / 2
        return [(ym - a * math.cos(math.pi * k / n), zs + rise * math.sin(math.pi * k / n)) for k in range(n + 1)]

    def vault(self, b):
        """Barrel vault, axis along X, crown at the block height (a ruin
        has lost it: the haunches would float over the broken walls)."""
        if self.roof is None or self.damage >= 2:
            return
        mb = self.roof
        n = 10
        vt = 0.3
        ix0, ix1, iy0, iy1 = self.inner(b)
        zs = self.spring
        rise = b.h - 0.04 - zs
        outer = self._arc(b.y0, b.y1, zs, rise, n)
        inner = self._arc(iy0, iy1, zs, rise - vt, n)
        xa, xb = b.x0 + 0.02, b.x1 - 0.02
        chunks = 1 if self.damage == 0 else 3
        holes = set()
        if self.damage == 1:
            c = self.r.randrange(chunks)
            k = self.r.randrange(3, n - 4)
            holes = {(c, k), (c, k + 1)}
            self.roof_holes.append((b, (xa + (xb - xa) * (c + 0.5) / chunks, (outer[k][0] + outer[k + 1][0]) / 2)))
        mb.style(kind=self.wall_kind)
        for c in range(chunks):
            x0 = xa + (xb - xa) * c / chunks
            x1 = xa + (xb - xa) * (c + 1) / chunks
            for k in range(n):
                if (c, k) in holes:
                    continue
                prof = [outer[k], outer[k + 1], inner[k + 1], inner[k]]
                mb.extrude_x(prof, x0, x1, "Plaster")

    def tympanum(self, b, side, top):
        """Arched top of a vault end wall, following the vault outline."""
        if self.damage >= 2:
            return
        mb = self.walls
        sgn, coord, t = _sgn(side), b.coord(side), self.end_t
        n = 10
        rise = b.h - 0.04 - self.spring
        arc = self._arc(b.y0, b.y1, self.spring, rise, n)
        xa, xb = (coord - t, coord) if sgn > 0 else (coord, coord + t)
        mb.style(kind=self.wall_kind)
        for (ya, za), (yb, zb) in zip(arc, arc[1:]):
            if max(za, zb) - self.spring < 0.02:
                continue
            mb.extrude_x([(ya, self.spring), (yb, self.spring), (yb, zb + 0.04), (ya, za + 0.04)], xa, xb,
                         "Plaster")

    # -- roofs ----------------------------------------------------------------------

    def flat(self, b):
        if self.roof is None or self.damage >= 2:
            return
        mb = self.roof
        x0, x1, y0, y1 = self.inner(b)
        z, t = b.roof_z, 0.25
        nx, ny = max(1, int((x1 - x0) / 1.6)), max(1, int((y1 - y0) / 1.6))
        holes = set()
        if self.damage == 1 and b.kind == "room" and self.r.random() < 0.8:
            for _k in range(4):
                i, j = self.r.randrange(nx), self.r.randrange(ny)
                xc = x0 + (x1 - x0) * (i + 0.5) / nx
                yc = y0 + (y1 - y0) * (j + 0.5) / ny
                if not any(o.level > b.level and o.contains(xc, yc) for o in self.blocks):
                    holes.add((i, j))
                    self.roof_holes.append((b, (xc, yc)))
                    break
        along_x = (x1 - x0) <= (y1 - y0)
        for i in range(nx):
            for j in range(ny):
                xa, xb = x0 + (x1 - x0) * i / nx, x0 + (x1 - x0) * (i + 1) / nx
                ya, yb = y0 + (y1 - y0) * j / ny, y0 + (y1 - y0) * (j + 1) / ny
                if (i, j) in holes:
                    # palm beams across the hole
                    mb.style(kind=2)
                    for f in (0.3, 0.7):
                        if along_x:
                            yy = ya + (yb - ya) * f
                            mb.cylinder((xa, yy, z - t - 0.02), (xb, yy, z - t - 0.02), 0.08, seg=5, mat="Bark")
                        else:
                            xx = xa + (xb - xa) * f
                            mb.cylinder((xx, ya, z - t - 0.02), (xx, yb, z - t - 0.02), 0.08, seg=5, mat="Bark")
                    continue
                mb.style(kind=self.roof_kind)
                mb.box(((xa + xb) / 2, (ya + yb) / 2, z - t / 2), (xb - xa, yb - ya, t), "Plaster")

    # -- floors, base, sand ------------------------------------------------------------

    def floors(self):
        fl = self.floor
        for b in self.blocks:
            if b.gone:
                continue
            x0, x1, y0, y1 = self.inner(b)
            if b.level == 0 and b.kind != "arcade":
                fl.style(kind=0)
                fl.box(((x0 + x1) / 2, (y0 + y1) / 2, 0.02), (x1 - x0, y1 - y0, 0.04), "Ground")
            if b.storeys > 1:
                z = b.base + self.storey_h
                fl.style(kind=0)
                if self.damage >= 2:
                    ya, yb = y0, y0 + (y1 - y0) * self.r.uniform(0.3, 0.5)
                else:
                    ya, yb = y0, y1
                fl.box(((x0 + x1) / 2, (ya + yb) / 2, z - 0.1), (x1 - x0, yb - ya, 0.2), "Wood")
            if b.kind == "arcade":
                fl.style(kind=1)
                xa, xb = b.x0, self.hw
                ya = -self.hl if b.y0 < -self.hl + self.e + 0.01 else b.y0
                yb = self.hl if b.y1 > self.hl - self.e - 0.01 else b.y1
                fl.box(((xa + xb) / 2, (ya + yb) / 2, 0.1), (xb - xa, yb - ya, 0.2), "Stone")

    def skirts(self):
        """Battered base along the exterior walls; breaks at doors."""
        fl, e = self.floor, self.e
        for b, side, a, c, ops in self.ext:
            if b.level or b.kind == "arcade":
                continue
            axis, sgn, coord = side[1], _sgn(side), b.coord(side)
            if axis == "x":
                if abs(a - b.y0) < 0.01 and self.seg_kind(b, "-y", coord - sgn * 0.05) == "ext":
                    a -= e
                if abs(c - b.y1) < 0.01 and self.seg_kind(b, "+y", coord - sgn * 0.05) == "ext":
                    c += e
            else:
                if self.seg_kind(b, "-x", coord - sgn * 0.05) not in (None, "skip"):
                    a = min(a, b.x0)
                if self.seg_kind(b, "+x", coord - sgn * 0.05) not in (None, "skip"):
                    c = max(c, b.x1)
            cuts = [(a, c)]
            for o in ops:
                if o.kind == "door" and o.z0 < 0.05:
                    nxt = []
                    for p0, p1 in cuts:
                        if o.hi < p0 or o.lo > p1:
                            nxt.append((p0, p1))
                            continue
                        if o.lo - p0 > 0.05:
                            nxt.append((p0, o.lo))
                        if p1 - o.hi > 0.05:
                            nxt.append((o.hi, p1))
                    cuts = nxt
                    # threshold stone
                    fl.style(kind=0)
                    self._box(fl, axis, coord + sgn * (e - 0.04) / 2, o.u, 0.06, e - 0.04, o.w + 0.2, 0.12, "Stone")
            fl.style(kind=self.skirt_kind)
            for p0, p1 in cuts:
                prof = [(-0.02, 0.0), (e, 0.0), (e, 0.06), (-0.02, 0.42)]
                pts = [self._pt(axis, coord + sgn * d, p0, z) for d, z in prof]
                dvec = (0, p1 - p0, 0) if axis == "x" else (p1 - p0, 0, 0)
                fl.extrude(pts, dvec, "Plaster")

    def drifts(self):
        """Low sand drifts at the wall bases (slot Ground, kind sand)."""
        fl = self.floor
        d = self.d
        for b, side, a, c, ops in self.ext:
            if b.level or b.kind == "arcade":
                continue
            axis, sgn, coord = side[1], _sgn(side), b.coord(side)
            half_u = self.hl if axis == "x" else self.hw
            room = (self.hw if axis == "x" else self.hl) - sgn * coord  # face to box edge
            for _k in range(d.choice((0, 1, 1, 2))):
                ru = d.uniform(0.8, 1.6)
                u = d.uniform(a + 0.2, c - 0.2)
                rp = d.uniform(0.35, 0.8)
                hz = d.uniform(0.3, 0.5)
                if any(o.kind == "door" and abs(u - o.u) < o.w / 2 + ru * 0.8 for o in ops):
                    continue
                if self.in_stair(*self._pt(axis, coord + sgn * 0.2, u, 0.0)[:2], 0.0):
                    continue
                rp = min(rp, (room - 0.02) / 1.15)
                ru = max(0.3, min(ru, (half_u - abs(u)) / 1.15))
                if rp < 0.12:
                    continue
                fl.style(kind=3)
                radii = (rp, ru, hz) if axis == "x" else (ru, rp, hz)
                fl.blob(self._pt(axis, coord, u, 0.0), radii, "Ground", seg=6, rings=3, rng=d, rough=0.12,
                        flat_bottom=0.0)
        # drifts in concave corners (L-shaped plans)
        seen = set()
        for b in self.blocks:
            if b.level or b.kind != "room":
                continue
            for o in self.blocks:
                if o is b or o.level or o.kind != "room":
                    continue
                if abs(o.x0 - b.x1) < 0.02 and b.y0 < o.y0 - 0.5:
                    x, y = b.x1, o.y0
                    if (round(x, 2), round(y, 2)) in seen:
                        continue
                    seen.add((round(x, 2), round(y, 2)))
                    r1 = min(1.2, self.hw - x - 0.05, y + self.hl - 0.05)
                    fl.style(kind=3)
                    fl.blob((x, y, 0.0), (r1 / 1.15, r1 / 1.15, 0.45), "Ground", seg=7, rings=4, rng=d, rough=0.12,
                            flat_bottom=0.0)

    # -- stair, balcony, details ---------------------------------------------------------

    def plan_stair(self):
        s = self.p.get("stair")
        if not s:
            return
        x0, y0, x1, y1 = s[0] - self.hw, s[1] - self.hl, s[2] - self.hw, s[3] - self.hl
        up = self.p.get("stair_up", "-y")
        host = max((b for b in self.blocks if b.level == 0 and b.roof_z and abs(b.x1 - x0) < 0.05
                    and b.y0 < y1 and b.y1 > y0), key=lambda b: b.roof_z)
        top = host.roof_z
        n = max(3, round(top / 0.23))
        rise = top / n
        length = y1 - y0
        tread = min(0.3, (length - self.p.get("stair_landing", 1.2)) / (n - 1))
        land = length - tread * (n - 1)

        def rng_y(d0, d1):
            return (y1 - d1, y1 - d0) if up == "-y" else (y0 + d0, y0 + d1)

        ly = rng_y(tread * (n - 1), length)
        self.stair = dict(x0=x0, x1=x1, y0=y0, y1=y1, n=n, rise=rise, tread=tread, top=top, up=up,
                          land=ly, rng_y=rng_y, length=length, passage=self.p.get("stair_passage", False))
        self.add_op(host, "+x", Opening((ly[0] + ly[1]) / 2, land - 0.3, top, host.h + 1.0, "gap"))

    def stair_z(self, y):
        s = self.stair
        d = (s["y1"] - y) if s["up"] == "-y" else (y - s["y0"])
        if d >= s["tread"] * (s["n"] - 1):
            return s["top"]
        return (int(max(d, 0.0) / s["tread"]) + 1) * s["rise"]

    def in_stair(self, x, y, z):
        s = self.stair
        if not s:
            return False
        return s["x0"] - 0.35 <= x <= s["x1"] + 0.05 and s["y0"] - 0.1 <= y <= s["y1"] + 0.1 and z <= self.stair_z(
            min(max(y, s["y0"]), s["y1"])) + 0.25

    def build_stair(self):
        s = self.stair
        if not s:
            return
        mb = self.walls
        mb.style(kind=self.wall_kind)
        xa, xb = s["x0"], s["x1"]
        xm, dx = (xa + xb) / 2, xb - xa
        keep = s["n"] - 1 if self.damage < 2 else int((s["n"] - 1) * 0.6)
        for i in range(keep):
            ya, yb = s["rng_y"](i * s["tread"], (i + 1) * s["tread"])
            z = (i + 1) * s["rise"]
            mb.box((xm, (ya + yb) / 2, z / 2), (dx, yb - ya, z), "Plaster")
        if self.damage >= 2:
            return
        ya, yb = s["land"]
        top = s["top"]
        if s["passage"]:
            # landing bridges over a passage to the door below; pier at the far end
            pa, pb = s["rng_y"](s["length"] - 0.3, s["length"])
            mb.box((xm, (pa + pb) / 2, top / 2), (dx, pb - pa, top), "Plaster")
            la, lb = (ya, pa) if pa > ya + 0.01 else (pb, yb)
            mb.box((xm, (la + lb) / 2, top - 0.3), (dx, lb - la, 0.6), "Plaster")
        else:
            mb.box((xm, (ya + yb) / 2, top / 2), (dx, yb - ya, top), "Plaster")
        # low curb on the open edge of the landing and the steps
        mb.box((xb - 0.08, (ya + yb) / 2, top + 0.2), (0.16, yb - ya, 0.4), "Plaster")

    def balcony(self):
        bal = self.p.get("balcony")
        if not bal or self.damage >= 2:
            return
        i, side, v, w = bal
        b = self.blocks[i]
        axis, sgn, coord = side[1], _sgn(side), b.coord(side)
        u = self.u_of(side, v)
        zf = b.base + self.storey_h
        dp = 0.85
        mb = self.walls
        mb.style(kind=0)
        self._box(mb, axis, coord + sgn * dp / 2, u, zf - 0.06, dp, w, 0.12, "Wood")
        for k in (-1, 0, 1):
            uu = u + k * (w / 2 - 0.15)
            a = self._pt(axis, coord, uu, zf - 0.75)
            c = self._pt(axis, coord + sgn * dp * 0.8, uu, zf - 0.1)
            mid = tuple((p + q) / 2 for p, q in zip(a, c))
            dvec = tuple(q - p for p, q in zip(a, c))
            side_v = (0, 1, 0) if axis == "x" else (1, 0, 0)
            ln = math.sqrt(sum(x * x for x in dvec))
            mb.oriented_box(mid, dvec, side_v, (ln, 0.1, 0.1), "Wood")
        # boarded parapet, posts, plank roof
        mb.style(kind=2)
        self._box(mb, axis, coord + sgn * (dp - 0.03), u, zf + 0.45, 0.04, w, 0.9, "Wood")
        for s in (-1, 1):
            self._box(mb, axis, coord + sgn * dp / 2, u + s * (w / 2 - 0.02), zf + 0.45, dp, 0.04, 0.9, "Wood")
            self._box(mb, axis, coord + sgn * (dp - 0.05), u + s * (w / 2 - 0.05), zf + 1.2, 0.09, 0.09, 2.4, "Wood")
        self._box(mb, axis, coord + sgn * (dp - 0.03), u, zf + 0.92, 0.08, w + 0.04, 0.06, "Wood", 0)
        if self.damage == 0:
            self._box(mb, axis, coord + sgn * (dp + 0.05) / 2, u, zf + 2.44, dp + 0.05, w + 0.2, 0.08, "Wood", 3)

    def details(self):
        """Palm beam ends, spalled plaster patches and drain spouts."""
        mb = self.walls
        d = self.d
        spout_cands = {}
        for b, side, a, c, ops in self.ext:
            axis, sgn, coord = side[1], _sgn(side), b.coord(side)
            top = self.tops.get((b.idx, side))
            z1 = self.wall_top(b, side)

            def blocked(u, z, du=0.12, dz=0.15, ops=ops):
                return any(o.lo - du < u < o.hi + du and o.z0 - dz < z < o.z1 + dz + 0.12 for o in ops)

            # palm beam ends where the roof (and floor) beams rest on this wall
            if b.kind == "room":
                x0, x1, y0, y1 = self.inner(b)
                along_x = (x1 - x0) <= (y1 - y0)
                if along_x == (axis == "x"):
                    levels = [b.roof_z - 0.36]
                    if b.storeys > 1 and self.floor_beams:
                        levels.append(b.base + self.storey_h - 0.3)
                    for z in levels:
                        nb = max(1, int((c - a - 0.5) / self.beam_step))
                        for k in range(nb):
                            u = a + 0.25 + (c - a - 0.5) * (k + 0.5) / nb + d.uniform(-0.06, 0.06)
                            zz = z + d.uniform(-0.03, 0.03)
                            rr = d.uniform(0.07, 0.095)
                            skip = d.random() < 0.08
                            if skip or blocked(u, zz) or (top is not None and top(u) < zz + 0.15):
                                continue
                            if self.in_stair(*self._pt(axis, coord + sgn * 0.1, u, zz)[:2], zz):
                                continue
                            if self.damage == 1 and self.r.random() < 0.15:
                                continue
                            mb.style(kind=2)
                            mb.cylinder(self._pt(axis, coord - sgn * 0.1, u, zz),
                                        self._pt(axis, coord + sgn * 0.2, u, zz), rr, seg=4, mat="Bark",
                                        cap0=False, phase=d.uniform(0, 1))
            # patches of spalled plaster showing the mud brick
            pk = []
            for _k in range(d.choice((0, 1, 1, 2, 2)) if not (b.kind == "arcade" and side == "+x") else 0):
                w, h = d.uniform(0.4, 1.1), d.uniform(0.3, 0.75)
                u = d.uniform(a + 0.2 + w / 2, max(a + 0.2 + w / 2, c - 0.2 - w / 2))
                zlo = b.base + 0.5
                z = d.uniform(zlo, max(zlo, z1 - 0.45 - h))
                mat = ("Stone", 3) if (self.wall_kind == 1 or d.random() < 0.6) else ("Plaster", 1)
                if c - a < w + 0.4 or z + h > z1 - 0.3:
                    continue
                if any(o.lo - 0.12 < u + w / 2 and u - w / 2 < o.hi + 0.12 and o.z0 - 0.15 < z + h and z < o.z1 + 0.25
                       for o in ops):
                    continue
                if any(abs(u - pu) < (w + pw) / 2 + 0.1 and abs(z - pz) < 1.0 for pu, pw, pz in pk):
                    continue
                if top is not None and top(u) < z + h + 0.1:
                    continue
                if self.in_stair(*self._pt(axis, coord + sgn * 0.1, u, z)[:2], z + h):
                    continue
                pk.append((u, w, z))
                self._box(mb, axis, coord + sgn * 0.008, u, z + h / 2, 0.03, w, h, *mat)
            if b.roof_z is not None and b.kind != "vault":
                spout_cands.setdefault(b.idx, []).append((b, side, a, c, ops, top))
        # drain spouts through the parapet
        for idx in sorted(spout_cands):
            cands = spout_cands[idx]
            for _k in range(d.choice((1, 1, 2))):
                b, side, a, c, ops, top = cands[d.randrange(len(cands))]
                axis, sgn, coord = side[1], _sgn(side), b.coord(side)
                u = d.uniform(a + 0.4, max(a + 0.4, c - 0.4))
                z = b.roof_z + 0.06
                if c - a < 0.8 or any(o.lo - 0.3 < u < o.hi + 0.3 for o in ops):
                    continue
                if top is not None and top(u) < z + 0.15:
                    continue
                if self.in_stair(*self._pt(axis, coord + sgn * 0.1, u, z)[:2], z):
                    continue
                mb.style(kind=0)
                self._box(mb, axis, coord + sgn * 0.06, u, z, 0.36, 0.13, 0.09, "Wood")
                self._box(mb, axis, coord + sgn * 0.08, u, z + 0.05, 0.3, 0.03, 0.06, "Wood")

    # -- debris -------------------------------------------------------------------------

    def heap(self, cx, cy, rx, ry, hz, mat, kind):
        hw, hl = self.hw, self.hl
        cx = max(-hw + 0.3, min(hw - 0.3, cx))
        cy = max(-hl + 0.3, min(hl - 0.3, cy))
        rx = max(0.2, min(rx, (hw - abs(cx)) / 1.22))
        ry = max(0.2, min(ry, (hl - abs(cy)) / 1.22))
        self.debris.style(kind=kind)
        self.debris.blob((cx, cy, 0.0), (rx, ry, hz), mat, seg=7, rings=4, rng=self.r, rough=0.18,
                         flat_bottom=0.0, smooth=False)

    def log(self, mb, a, b):
        """Palm log between two points, clamped into the box."""
        hw, hl = self.hw - 0.12, self.hl - 0.12
        a = (max(-hw, min(hw, a[0])), max(-hl, min(hl, a[1])), max(0.09, a[2]))
        b = (max(-hw, min(hw, b[0])), max(-hl, min(hl, b[1])), max(0.09, b[2]))
        mb.style(kind=2)
        mb.cylinder(a, b, 0.085, seg=5, mat="Bark")

    def _rubble_mat(self):
        r = self.r.random()
        if r < 0.45:
            return "Plaster", self.wall_kind
        if r < 0.8:
            return "Stone", 3
        return "Plaster", 1

    def debris_build(self):
        if self.debris is None:
            return
        r = self.r
        if self.damage == 1:
            for b, side, u in getattr(self, "shell_holes", []):
                axis, sgn, coord = side[1], _sgn(side), b.coord(side)
                x, y, _z = self._pt(axis, coord + sgn * 0.3, u, 0)
                rx, ry = (0.5, 0.9) if axis == "x" else (0.9, 0.5)
                self.heap(x, y, rx, ry, 0.4, *self._rubble_mat())
                x, y, _z = self._pt(axis, coord - sgn * 1.0, u, 0)
                self.heap(x, y, 0.7, 0.7, 0.45, *self._rubble_mat())
            for b, (x, y) in self.roof_holes:
                self.heap(x, y, 0.8, 0.8, 0.5, *self._rubble_mat())
                a = r.uniform(0, math.pi)
                self.log(self.debris, (x - math.cos(a) * 1.0, y - math.sin(a) * 1.0, 0.1),
                         (x + math.cos(a) * 1.0, y + math.sin(a) * 1.0, 0.9))
            return
        for b in self.blocks:
            if b.level:
                continue
            x0, x1, y0, y1 = self.inner(b)
            for _k in range(1 + int((b.x1 - b.x0) * (b.y1 - b.y0) > 25)):
                cx, cy = r.uniform(x0 + 0.5, x1 - 0.5), r.uniform(y0 + 0.5, y1 - 0.5)
                self.heap(cx, cy, r.uniform(0.9, 1.5), r.uniform(0.9, 1.5), r.uniform(0.6, 1.1), *self._rubble_mat())
            # a beam leaning from a wall stump into the room, one lying
            if b.kind != "arcade":
                along_x = (x1 - x0) <= (y1 - y0)
                if along_x:
                    yy = r.uniform(y0 + 0.4, y1 - 0.4)
                    top = self.tops.get((b.idx, "-x"))
                    zt = top(yy) if top else 1.0
                    self.log(self.roof, (x0 - 0.1, yy, zt - 0.05), (x1 - 0.6, yy + r.uniform(-0.6, 0.6), 0.1))
                else:
                    xx = r.uniform(x0 + 0.4, x1 - 0.4)
                    top = self.tops.get((b.idx, "-y"))
                    zt = top(xx) if top else 1.0
                    self.log(self.roof, (xx, y0 - 0.1, zt - 0.05), (xx + r.uniform(-0.6, 0.6), y1 - 0.6, 0.1))
                cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
                a = r.uniform(0, math.pi)
                self.log(self.debris, (cx - math.cos(a) * 1.2, cy - math.sin(a) * 1.2, 0.5),
                         (cx + math.cos(a) * 1.2, cy + math.sin(a) * 1.2, 0.2))
        # rubble spilling out of the walls
        outs = [(b, side, a, c) for b, side, a, c, _ops in self.ext if b.level == 0]
        r.shuffle(outs)
        for b, side, a, c in outs[:3]:
            axis, sgn, coord = side[1], _sgn(side), b.coord(side)
            u = r.uniform(a, c)
            x, y, _z = self._pt(axis, coord + sgn * 0.1, u, 0)
            rx, ry = (0.6, 1.2) if axis == "x" else (1.2, 0.6)
            self.heap(x, y, rx, ry, 0.55, *self._rubble_mat())

    # -- all --------------------------------------------------------------------------

    def build(self):
        self.roof_holes = []
        self.openings()
        self.plan_stair()
        self.damage_plan()
        self.build_walls()
        for b in self.blocks:
            if b.gone:
                continue
            if b.kind == "vault":
                self.vault(b)
            else:
                self.flat(b)
        self.floors()
        self.skirts()
        self.drifts()
        self.build_stair()
        self.balcony()
        self.details()
        if self.damage >= 1:
            self.buttress_and_debris()
        else:
            self.buttresses()

    def buttress_and_debris(self):
        self.buttresses()
        self.debris_build()

    def buttresses(self):
        """Sloped buttresses at the ends of the long walls of vaulted blocks."""
        mb = self.walls
        e = self.e
        for b, side, a, c, _ops in self.ext:
            if b.kind != "vault" or side[1] != "y" or b.level:
                continue
            sgn, coord = _sgn(side), b.coord(side)
            room = self.hl - sgn * coord - 0.01
            dep = min(e, room)
            mb.style(kind=self.wall_kind)
            for u in (b.x0 + 0.45, b.x1 - 0.45):
                if not (a - 0.5 <= u <= c + 0.5):
                    continue
                prof = [(-0.05, 0.0), (dep, 0.0), (dep, 0.35), (-0.05, self.spring * 0.9)]
                pts = [(u - 0.4, coord + sgn * dd, z) for dd, z in prof]
                mb.extrude(pts, (0.8, 0, 0), "Plaster")


@env_part("desert_house")
def desert_house(model, name, p, box):
    """North African village house (prompt 059), one variant = one damage level.

    Plan (meters from the box's -X/-Y corner, x = 0..width, y = 0..length):
    - `blocks`: [[x0, y0, x1, y1, h], ...] rooms on the ground; `h` = top of
      the parapet (vaulted block: vault crown). Adjacent blocks share one
      wall (built by the taller one). Blocks with a roof above 5 m get two
      storeys (`storey_h`, default 3.0).
    - `roof_rooms`: same, standing on the flat roof of the block below
      (stair heads, roof rooms); indices continue after `blocks`.
    - `doors`: [[block, side, u, width (, storey, height)], ...]; side
      "+x" | "-x" | "+y" | "-y", u = box coordinate along the side.
    - `windows`: [[block, side, u (, storey)], ...]; size `window` ([w, h],
      default [0.55, 0.65]), sill `sill` (1.2) / `sill_up` (0.9 above the
      upper floor). Shutters, open shutters or grille are chosen by seed.
    - `vaults`: block indices with a barrel vault (axis along X), spring
      height `vault_spring` (2.3), end wall thickness `end_t` (0.7).
    - `arcade`: block index of an arcade (round arches on +x, `arches`
      default 4) with a raised walk.
    - `stair`: [x0, y0, x1, y1] solid outside stair (not set back),
      `stair_up` ("-y" | "+y", direction of ascent), `stair_landing`
      (min. landing length, 1.2), `stair_passage` (door passage under the
      landing); the parapet above the landing is left open.
    - `balcony`: [block, side, u, width] wooden balcony on the upper floor.
    Look: `plaster` (Plaster kind of the walls, 0 lime / 1 adobe /
    2 whitewash / 3 ochre), `roof_kind` (1), `skirt_kind` (1), `coping`
    (Plaster kind of a parapet coping, -1 none), `parapet` (0.5) or
    `parapets` (per block), `wall_t` (0.45), `skirt` (0.3: set-back from the
    box edge and width of the battered base), `beam_step` (0.7, palm beam
    ends), `floor_beams` (beam ends at the upper floor, true). `seed`,
    `damage` (0 / 1 / 2).
    """
    h = DesertHouse(model, name, p, box)
    h.build()
    h.finish()
    model.part_props.setdefault(name, {})["door_height_m"] = DOOR_H
