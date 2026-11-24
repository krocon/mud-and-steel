"""Apartment blocks (prompt 078): four- to six-storey residential blocks,
built by the generator `apartment_block`.

Why a new generator: `buildings.house` builds every wall piece as a closed
box and every window from four boxes (glass, mullion, transom, sill), about
80 triangles per window; a block has 100 to 170 windows. `desert.desert_house`
is a room-block plan with palm beams, shutters and sand drifts. Neither
builds interiors for the damage levels. `apartment_block` reuses the shared
`House` base (parts, ruin top line, `finish`) and builds facades from
single faces:

- **Facade:** a strict grid of bays. The outer wall face is cut into
  piers and spandrels (one quad each, equal neighbours merged); window
  reveals are four quads to a recessed sash quad; the inner face of an
  intact wall is one quad per wall. Only where damage cuts a wall down
  (the breach of `Damaged`, the stumps of `Ruin`) are the pieces closed
  boxes with an inside face and a broken top.
- **Windows:** one quad per sash in the `Glass` slot whose UV carries the
  sash pattern (U = 102 + across, V = down; env.gdshader draws the white
  frame, transom and glazing bars), about 15 triangles per window with
  reveals and sill. Some windows have closed shutters (`Wood`).
- **Grime:** single `Plaster` quads 1 cm in front of the wall, U = 100
  (dirt streak under a sill) or 101 (soot above a burnt-out window); the
  shader darkens the wall pattern behind them, so they cost 2 triangles.
- **Damage:** `Damaged` tears one corner open over one or two storeys
  (walls with a jagged top, the roof slab and the floor slabs bitten off,
  partition walls and the far walls of the corner rooms visible), burns out
  and blasts windows, punches shell holes; `Ruin` keeps the lower one to
  two storeys as jagged wall stumps with a first-floor slab remnant,
  partition stumps and rubble heaps of the house's mass.

Conventions as `buildings.house`: length along Y, width along X; street
side with the entrances is +X, courtyard side with the balconies -X. The
box height is the highest point (stair heads of flat roofs, chimney tops of
saddle roofs). One variant = one damage level (`damage` 0 / 1 / 2) from the
same plan, so the three variants line up.
"""

import math

from mathutils import Vector

from .buildings import House, Opening
from .env import env_part, rng

SIDES = ("+x", "-x", "+y", "-y")
STREAK, SOOT, SASH = 100.0, 101.0, 102.0


def _sgn(side):
    return 1 if side[0] == "+" else -1


class Top:
    """Broken top of a wall: piecewise constant segments (u0, u1, z);
    outside them the wall keeps its full height `full`."""

    def __init__(self, segs, full):
        self.segs = segs
        self.full = full
        self.cuts = sorted({v for a, b, _z in segs for v in (a, b)})

    def __call__(self, u):
        for a, b, z in self.segs:
            if a - 1e-6 <= u <= b + 1e-6:
                return z
        return self.full

    def low(self, u0, u1):
        """Lowest top between u0 and u1."""
        zs = [self(u0), self(u1), self((u0 + u1) / 2)]
        zs += [z for a, b, z in self.segs if b > u0 and a < u1]
        return min(zs)


class Block(House):
    def __init__(self, model, name, p, box):
        wall = p.get("wall", "Plaster")
        super().__init__(model, name, p, box, wall, p.get("wall_kind", 0))
        self.p = p
        self.d = rng(p.get("seed", 1) * 31 + 7)  # design: the same for every damage level
        hl, hw, H = box.hl, box.hw, box.height
        self.H = H
        self.storeys = p.get("storeys", 5)
        self.sh = p.get("storey_h", 2.9)
        self.gh = p.get("ground_h", self.sh)
        self.plinth = p.get("plinth", 0.3)
        self.t = p.get("wall_t", 0.4)
        self.win = p.get("window", [1.1, 1.4])
        self.win_ground = p.get("window_ground", self.win)
        self.sill = p.get("sill", 0.9)
        self.bays_long = p.get("bays_long", 10)
        self.bays_short = p.get("bays_short", 2)
        self.bay_margin = p.get("bay_margin", 1.2)
        self.entrances = p.get("entrances", [])
        self.door = p.get("door", [1.4, 2.4])
        self.door_ground = p.get("door_at_ground", False)
        self.canopy = p.get("canopy", 1.2)
        self.bal_bays = p.get("balconies", [])
        self.bal_depth = p.get("balcony_depth", 1.2)
        self.passage = p.get("passage", None)
        self.roof_type = p.get("roof", "flat")
        self.parapet = p.get("parapet", 0.9)
        self.roof_kind = p.get("roof_kind", 0)
        self.eave = p.get("eave", 0.4)
        self.ledge = p.get("apron", 0.3)
        self.tanks = p.get("tanks", 2)
        self.chimneys = p.get("chimneys", 4)
        self.vents = p.get("vents", 3)
        self.stair_head = p.get("stair_head", [2.8, 3.6])
        self.streaks = p.get("streaks", 0.6)
        self.shutters = p.get("shutters", 0.12)
        self.cornice = p.get("cornice", False)
        self.band = p.get("string_course", False)
        self.plinth_band = p.get("plinth_band", [0.5, 0])  # [height, Stone kind]
        self.slab_mat = p.get("slab", ["Stone", 2])
        self.inner_kind = p.get("inner_kind", 0)
        self.burnt = p.get("burnt", 8)
        # Prompt 202: fire escapes, high stoops, a projecting top cornice.
        self.fire_escape = p.get("fire_escape", [])
        self.stoop = p.get("stoop", 0.0)
        self.top_cornice = p.get("top_cornice", 0.0)
        self.front = p.get("front", 0.0)   # the street wall stands this far behind the box edge
        self.reveal = 0.15
        flat = self.roof_type == "flat"
        if flat:
            self.x0 = -hw + (self.bal_depth if self.bal_bays else self.ledge)
            self.x1 = hw - max(self.canopy if self.entrances and self.canopy > 0 else self.ledge, self.front)
            self.y0, self.y1 = -hl + self.ledge, hl - self.ledge
        else:
            self.x0, self.x1 = -hw + self.eave, hw - self.eave
            self.y0, self.y1 = -hl + 0.12, hl - 0.12
        self.roof_z = self.level(self.storeys)
        self.z1 = self.roof_z + self.parapet if flat else self.roof_z
        self.ridge = H - p.get("chimney_above", 0.6)
        self.ops = {s: [] for s in SIDES}
        self.tops = {}
        self.R = None  # breach rect (x0, x1, y0, y1) and corner signs
        self.bays = {}

    # -- plan -----------------------------------------------------------------------

    def level(self, s):
        if s <= 0:
            return self.plinth
        return self.plinth + self.gh + (s - 1) * self.sh

    def coord(self, side):
        return {"+x": self.x1, "-x": self.x0, "+y": self.y1, "-y": self.y0}[side]

    def span(self, side):
        return (self.y0, self.y1) if side[1] == "x" else (self.x0, self.x1)

    def centers(self, side):
        n = self.bays_long if side[1] == "x" else self.bays_short
        u0, u1 = self.span(side)
        m = self.bay_margin
        step = (u1 - u0 - 2 * m) / max(n, 1)
        return [u0 + m + step * (i + 0.5) for i in range(n)], step

    def openings(self):
        d = self.d
        for side in SIDES:
            us, step = self.centers(side)
            self.bays[side] = (us, step)
            for i, u in enumerate(us):
                entrance = side == "+x" and i in self.entrances
                passage = self.passage is not None and side in ("+x", "-x") and i == self.passage[0]
                for s in range(self.storeys):
                    lev, top = self.level(s), self.level(s + 1)
                    ww, wh = self.win_ground if s == 0 else self.win
                    if passage and s == 0:
                        o = Opening(u, self.passage[1], 0.0, self.passage[2], "passage")
                    elif entrance and s == 0:
                        z0 = 0.0 if self.door_ground else lev
                        o = Opening(u, self.door[0], z0, z0 + self.door[1], "door")
                    elif entrance:
                        # stairwell: smaller windows at the half landings
                        if s >= self.storeys - 1:
                            continue
                        z0 = lev + (top - lev) / 2 + 0.75
                        o = Opening(u, ww * 0.8, z0, z0 + 1.0, "window")
                    elif side == "-x" and i in self.bal_bays and s >= 1:
                        o = Opening(u, ww, lev + 0.02, lev + self.sill + wh, "balcony_door")
                    else:
                        z0 = lev + self.sill
                        o = Opening(u, ww, z0, min(z0 + wh, top - 0.3), "window")
                    o.storey = s
                    o.bay = i
                    o.shut = o.kind == "window" and d.random() < self.shutters
                    o.wood = d.choice((0, 2, 2))
                    o.streak = d.random() < self.streaks
                    o.streak_h = d.uniform(0.5, 1.0)
                    self.ops[side].append(o)

    # -- damage ---------------------------------------------------------------------

    def _breach_top(self, a, b, corner_at_b, zb, full):
        """Segments descending from full height at the far end to `zb` at the corner."""
        r = self.r
        segs = []
        n = max(2, int(abs(b - a) / 1.1))
        for k in range(n):
            ua, ub = a + (b - a) * k / n, a + (b - a) * (k + 1) / n
            t = (k + 0.5) / n
            t = t if corner_at_b else 1.0 - t
            f = min(1.0, t / 0.55)
            z = full - (full - zb) * (f * f * (3 - 2 * f)) + r.uniform(-0.45, 0.35)
            z = max(zb - 0.3, min(full, z))
            segs.append((min(ua, ub), max(ua, ub), z))
        return segs

    def damage_plan(self):
        d, r = self.d, self.r
        if self.damage == 1:
            sx, sy = d.choice((1, -1)), d.choice((1, -1))
            if self.roof_type != "flat":
                sx = 1  # the street side under the eaves
            k = d.choice((1, 2))
            zb = self.level(self.storeys - k) + d.uniform(0.3, 0.8)
            ln = d.uniform(6.5, 9.0)
            xo = self.x1 if sx > 0 else self.x0
            yo = self.y1 if sy > 0 else self.y0
            xa = 0.0
            ya = yo - sy * ln
            self.R = (min(xa, xo), max(xa, xo), min(ya, yo), max(ya, yo), sx, sy, k, zb)
            full = self.z1
            side_x = "+x" if sx > 0 else "-x"
            side_y = "+y" if sy > 0 else "-y"
            segs = self._breach_top(ya, yo, True, zb, full)
            self.tops[side_x] = Top(segs, full)
            segs = self._breach_top(xa, xo, True, zb, full)
            self.tops[side_y] = Top(segs, full)
            # burnt-out windows: around the breach, a few elsewhere
            wins = [(s, o) for s in SIDES for o in self.ops[s] if o.kind in ("window", "balcony_door")]
            near = [(s, o) for s, o in wins if self.near_breach(s, o)]
            far = [(s, o) for s, o in wins if not self.near_breach(s, o)]
            r.shuffle(near)
            r.shuffle(far)
            for s, o in near[: self.burnt // 2] + far[: self.burnt - self.burnt // 2]:
                o.kind = "burnt"
            # blasted windows
            far = [(s, o) for s, o in far if o.kind == "window"]
            for s, o in far[:3]:
                _us, step = self.bays[s]
                o.kind = "hole"
                o.w = min(o.w * r.uniform(1.4, 1.7), step - 0.5)
                o.z0 -= r.uniform(0.2, 0.4)
                o.z1 += r.uniform(0.1, 0.25)
            # shell holes in the piers
            for _k in range(3):
                s = r.choice(("+x", "-x", "+x", "-y", "+y"))
                us, step = self.bays[s]
                if len(us) < 2:
                    continue
                i = r.randrange(len(us) - 1)
                st = r.randrange(0, self.storeys - k)
                lev = self.level(st)
                w = min(r.uniform(1.0, 1.5), step - self.win[0] - 0.3)
                if w < 0.6:
                    continue
                o = Opening((us[i] + us[i + 1]) / 2 + r.uniform(-0.1, 0.1), w, lev + r.uniform(0.5, 0.9),
                            lev + r.uniform(1.6, 2.1), "hole")
                o.storey, o.bay, o.shut, o.streak = st, -1, False, False
                self.ops[s].append(o)
        elif self.damage >= 2:
            # Stumps one to two storeys high. The top steps at the bays
            # (between the windows), so the stumps read as walls with
            # window holes, broken by a few notches; corners stand higher.
            lo = self.level(0) + 0.5 * (self.level(1) - self.level(0))
            hi = self.level(2) - 0.4
            for side in SIDES:
                u0, u1 = self.span(side)
                us, step = self.bays[side] if self.bays[side][0] else ([], u1 - u0)
                edges = [u0] + [(a + b) / 2 for a, b in zip(us, us[1:])] + [u1]
                segs = []
                z = r.uniform(lo, hi)
                k = 0
                while k < len(edges) - 1:
                    n = r.choice((1, 1, 2))
                    a, b = edges[k], edges[min(k + n, len(edges) - 1)]
                    k += n
                    zz = z
                    if min(a - u0, u1 - b) < 0.5:
                        zz = max(zz, r.uniform(self.level(1) + 0.4, self.level(2) + 0.5))
                    # a broken notch inside the piece
                    if b - a > 2.5 and r.random() < 0.6:
                        m = r.uniform(a + 0.8, b - 0.8)
                        w = r.uniform(0.6, 1.2)
                        notch = max(self.plinth + 0.5, zz - r.uniform(0.8, 1.8))
                        segs += [(a, m - w / 2, zz), (m - w / 2, m + w / 2, notch), (m + w / 2, b, zz)]
                    else:
                        segs.append((a, b, zz))
                    z = max(lo, min(hi, z + r.uniform(-1.4, 1.4)))
                self.tops[side] = Top(segs, self.z1)

    def near_breach(self, side, o):
        if self.R is None:
            return False
        x0, x1, y0, y1, sx, sy, k, zb = self.R
        if o.storey < self.storeys - k - 1:
            return False
        if side[1] == "x":
            return _sgn(side) == sx and y0 - 4.0 < o.u < y1 + 4.0
        return _sgn(side) == sy and x0 - 4.0 < o.u < x1 + 4.0

    def in_R(self, x, y, pad=0.0):
        if self.R is None:
            return False
        x0, x1, y0, y1 = self.R[:4]
        return x0 - pad <= x <= x1 + pad and y0 - pad <= y <= y1 + pad

    # -- primitives -----------------------------------------------------------------

    @staticmethod
    def P(axis, n, u, z):
        return (n, u, z) if axis == "x" else (u, n, z)

    def quad(self, mb, pts, mat, normal, uvs=None):
        """One quad; the corner order is fixed so that it faces `normal`."""
        a, b, c = Vector(pts[0]), Vector(pts[1]), Vector(pts[2])
        if (b - a).cross(c - a).dot(Vector(normal)) < 0:
            pts = list(reversed(pts))
            uvs = list(reversed(uvs)) if uvs else None
        if uvs:
            return mb.quad_uv(pts, uvs, mat, smooth=False)
        return mb.face(mb.verts(pts), mat)

    def wquad(self, mb, axis, n, ua, ub, za, zb, mat, normal, uvs=None):
        """Quad in a wall plane (x = n or y = n)."""
        P = self.P
        return self.quad(mb, [P(axis, n, ua, za), P(axis, n, ub, za), P(axis, n, ub, zb), P(axis, n, ua, zb)],
                         mat, normal, uvs)

    @staticmethod
    def nvec(axis, s):
        return (s, 0, 0) if axis == "x" else (0, s, 0)

    @staticmethod
    def uvec(axis, s):
        return (0, s, 0) if axis == "x" else (s, 0, 0)

    def card(self, mb, axis, n, ua, ub, za, zb, code, normal):
        """Stain card (dirt streak, soot): U = code + across, V = 1 at the top."""
        mb.style(kind=self.wall_kind)
        self.wquad(mb, axis, n, ua, ub, za, zb, self.wall_mat, normal,
                   uvs=[(code + 0.001, 0.0), (code + 0.999, 0.0), (code + 0.999, 1.0), (code + 0.001, 1.0)])

    # -- facades --------------------------------------------------------------------

    def facade(self, side):
        axis, sgn = side[1], _sgn(side)
        c = self.coord(side)
        t = self.t
        ci = c - sgn * t
        u0, u1 = self.span(side)
        z0, z1 = 0.0, self.z1
        top = self.tops.get(side)
        ops = [o for o in self.ops[side] if o.hi > u0 + 0.03 and o.lo < u1 - 0.03]
        mb = self.walls
        out_n = self.nvec(axis, sgn)
        in_n = self.nvec(axis, -sgn)
        cuts = {u0, u1} | {min(max(v, u0), u1) for o in ops for v in (o.lo, o.hi)}
        if top is not None:
            cuts |= {u for u in top.cuts if u0 < u < u1}
        cuts = sorted(cuts)
        flat_rects, box_rects, inner = [], [], []
        for ua, ub in zip(cuts, cuts[1:]):
            if ub - ua < 0.015:
                continue
            um = (ua + ub) / 2
            zt = z1 if top is None else min(z1, top(um))
            holes = sorted((o.z0, o.z1) for o in ops if o.lo < um < o.hi)
            fills, z = [], z0
            for a, b in holes:
                if a > z + 0.02:
                    fills.append((z, a))
                z = max(z, b)
            if z < z1 - 0.02:
                fills.append((z, z1))
            clipped = zt < z1 - 1e-3
            for a, b in fills:
                b = min(b, zt)
                if b - a < 0.04:
                    continue
                if clipped:
                    box_rects.append((round(a, 4), round(b, 4), ua, ub))
                    continue
                # split near the ground: the AO bake is per vertex, the
                # ground occlusion should stay at the foot of the wall
                zs = [a] + [z for z in (1.5,) if a + 0.3 < z < b - 0.3] + [b]
                for za, zb in zip(zs, zs[1:]):
                    flat_rects.append((round(za, 4), round(zb, 4), ua, ub))
            if not clipped:
                # inner face: cut only by openings through the building
                thr = sorted((o.z0, o.z1) for o in ops if o.lo < um < o.hi and o.kind == "passage")
                z = z0
                for a, b in thr:
                    if a > z + 0.02:
                        inner.append((z, a, ua, ub))
                    z = max(z, b)
                if z < z1 - 0.02:
                    inner.append((round(z, 4), z1, ua, ub))

        def merged(items):
            res = []
            for a, b, ua, ub in sorted(items):
                if res and res[-1][0] == a and res[-1][1] == b and abs(res[-1][3] - ua) < 1e-4:
                    res[-1] = (a, b, res[-1][2], ub)
                else:
                    res.append((a, b, ua, ub))
            return res

        mb.style(kind=self.wall_kind)
        for a, b, ua, ub in merged(flat_rects):
            self.wquad(mb, axis, c, ua, ub, a, b, self.wall_mat, out_n)
        ia, ib = (u0 + t, u1 - t) if axis == "y" else (u0, u1)
        for a, b, ua, ub in merged(inner):
            ua, ub = max(ua, ia), min(ub, ib)
            if ub - ua > 0.02:
                self.wquad(mb, axis, ci, ua, ub, a, b, self.wall_mat, in_n)
        for a, b, ua, ub in merged(box_rects):
            P = self.P
            pts = [P(axis, c - sgn * t / 2, (ua + ub) / 2, (a + b) / 2)]
            size = (t, ub - ua, b - a) if axis == "x" else (ub - ua, t, b - a)
            mb.box(pts[0], size, self.wall_mat)
        for o in ops:
            self.opening(side, o, top)

    def op_clipped(self, top, o):
        return top is not None and top.low(o.lo, o.hi) < o.z1 + 0.1

    def opening(self, side, o, top):
        """Reveals, sash, door, sill and stains of one opening."""
        axis, sgn = side[1], _sgn(side)
        c = self.coord(side)
        t = self.t
        mb = self.walls
        if self.op_clipped(top, o) or self.damage >= 2:
            # The broken wall is built of closed boxes that frame the
            # opening; the stumps keep their holes without sashes.
            return
        through = o.kind in ("hole", "burnt", "passage")
        depth = t if through else (0.22 if o.kind in ("door",) else self.reveal)
        if top is None or top(o.u) >= self.z1 - 1e-3:
            self.reveals(side, o, depth)
        n_in = c - sgn * depth
        out_n = self.nvec(axis, sgn)
        if o.kind in ("window", "balcony_door"):
            if o.shut:
                mb.style(kind=o.wood)
                self.wquad(mb, axis, n_in + sgn * 0.08, o.lo, o.hi, o.z0, o.z1, "Wood", out_n)
            else:
                mb.style(kind=0)
                self.wquad(mb, axis, n_in, o.lo, o.hi, o.z0, o.z1, "Glass", out_n,
                           uvs=[(SASH + 0.001, 0.0), (SASH + 0.999, 0.0), (SASH + 0.999, 1.0), (SASH + 0.001, 1.0)])
        elif o.kind == "door":
            mb.style(kind=2)
            self.wquad(mb, axis, n_in, o.lo, o.hi, o.z0, o.z1, "Wood", out_n)
        elif o.kind in ("hole", "burnt"):
            mb.style(kind=3)  # dark, charred room behind
            self.wquad(mb, axis, c - sgn * (t + 0.8), o.lo - 0.3, o.hi + 0.3, o.z0 - 0.2, o.z1 + 0.2, "Wood", out_n)
        if o.kind in ("window", "burnt") and o.z0 > 0.3:
            # sill: front and top
            sm, sk = self.slab_mat
            mb.style(kind=sk)
            P = self.P
            za, zb = o.z0 - 0.06, o.z0
            nf = c + sgn * 0.06
            self.wquad(mb, axis, nf, o.lo - 0.06, o.hi + 0.06, za, zb, sm, out_n)
            self.quad(mb, [P(axis, c, o.lo - 0.06, zb), P(axis, c, o.hi + 0.06, zb), P(axis, nf, o.hi + 0.06, zb),
                           P(axis, nf, o.lo - 0.06, zb)], sm, (0, 0, 1))
        # grime under the sill, soot over burnt windows
        nc = c + sgn * 0.012
        if o.kind == "window" and o.streak and o.z0 > 1.0:
            below = max((p.z1 for p in self.ops[side] if p is not o and p.lo < o.u < p.hi and p.z1 <= o.z0),
                        default=0.0)
            h = min(o.streak_h, o.z0 - 0.07 - below - 0.1)
            if h > 0.25:
                self.card(mb, axis, nc, o.lo + 0.05, o.hi - 0.05, o.z0 - 0.07 - h, o.z0 - 0.07, STREAK, out_n)
        if o.kind == "burnt":
            above = min((p.z0 for p in self.ops[side] if p is not o and p.lo < o.u < p.hi and p.z0 >= o.z1),
                        default=self.z1)
            h = min(2.2, above - o.z1 - 0.04)
            if top is not None:
                h = min(h, top.low(o.lo, o.hi) - o.z1 - 0.05)
            if h > 0.3:
                w = o.w * 0.6
                self.card(mb, axis, nc, o.lo - w, o.hi + w, o.z1, o.z1 + h, SOOT, out_n)

    def reveals(self, side, o, depth):
        axis, sgn = side[1], _sgn(side)
        c = self.coord(side)
        n2 = c - sgn * depth
        mb = self.walls
        mb.style(kind=self.wall_kind)
        P = self.P
        u = self.uvec(axis, 1)
        for uu, s in ((o.lo, 1), (o.hi, -1)):
            nrm = tuple(x * s for x in u)
            self.quad(mb, [P(axis, c, uu, o.z0), P(axis, n2, uu, o.z0), P(axis, n2, uu, o.z1), P(axis, c, uu, o.z1)],
                      self.wall_mat, nrm)
        self.quad(mb, [P(axis, c, o.lo, o.z1), P(axis, c, o.hi, o.z1), P(axis, n2, o.hi, o.z1), P(axis, n2, o.lo, o.z1)],
                  self.wall_mat, (0, 0, -1))
        if o.z0 > 0.02:
            self.quad(mb, [P(axis, c, o.lo, o.z0), P(axis, c, o.hi, o.z0), P(axis, n2, o.hi, o.z0),
                           P(axis, n2, o.lo, o.z0)], self.wall_mat, (0, 0, 1))

    def spans_at(self, side, z, ops_cut=True):
        """Parts of a side whose wall still reaches `z` (cornices, copings,
        bands), cut by openings that reach it."""
        u0, u1 = self.span(side)
        top = self.tops.get(side)
        cuts = [u0, u1]
        if top is not None:
            cuts += [u for u in top.cuts if u0 < u < u1]
        blocked = [(o.lo, o.hi) for o in self.ops[side] if ops_cut and o.z0 <= z <= o.z1]
        cuts += [v for a, b in blocked for v in (a, b) if u0 < v < u1]
        cuts = sorted(set(cuts))
        res = []
        for ua, ub in zip(cuts, cuts[1:]):
            um = (ua + ub) / 2
            if top is not None and top(um) < z:
                continue
            if any(a < um < b for a, b in blocked):
                continue
            if res and abs(res[-1][1] - ua) < 1e-4:
                res[-1] = (res[-1][0], ub)
            else:
                res.append((ua, ub))
        return [(a, b) for a, b in res if b - a > 0.1]

    def band_box(self, side, za, zb, out, mat, kind, ops_cut=True, part=None):
        axis, sgn = side[1], _sgn(side)
        c = self.coord(side)
        mb = part or self.walls
        mb.style(kind=kind)
        P = self.P
        for ua, ub in self.spans_at(side, zb - 0.01, ops_cut):
            if axis == "y":  # run the band round the corners
                ua, ub = (ua - out if ua <= self.x0 + 1e-3 else ua), (ub + out if ub >= self.x1 - 1e-3 else ub)
            nf = c + sgn * out
            self.wquad(mb, axis, nf, ua, ub, za, zb, mat, self.nvec(axis, sgn))
            self.quad(mb, [P(axis, c, ua, zb), P(axis, c, ub, zb), P(axis, nf, ub, zb), P(axis, nf, ua, zb)], mat,
                      (0, 0, 1))
            if za > 0.02:
                self.quad(mb, [P(axis, c, ua, za), P(axis, c, ub, za), P(axis, nf, ub, za), P(axis, nf, ua, za)], mat,
                          (0, 0, -1))
            if axis == "x":
                for uu, s in ((ua, -1), (ub, 1)):
                    self.quad(mb, [P(axis, c, uu, za), P(axis, nf, uu, za), P(axis, nf, uu, zb), P(axis, c, uu, zb)],
                              mat, self.uvec(axis, s))

    # -- details on the walls ---------------------------------------------------------

    def canopies(self):
        if self.roof_type != "flat" or self.canopy <= 0 or self.damage >= 2:
            return
        mb = self.walls
        sm, sk = self.slab_mat
        for o in self.ops["+x"]:
            if o.kind != "door":
                continue
            mb.style(kind=sk)
            z = o.z1 + 0.3
            w = o.w + 1.6
            mb.box((self.x1 + self.canopy / 2, o.u, z + 0.07), (self.canopy, w, 0.14), sm)
            # two steps up to the raised ground floor
            if o.z0 > 0.1:
                mb.box((self.x1 + 0.35, o.u, o.z0 / 2), (0.7, o.w + 0.6, o.z0), sm)

    def balconies(self):
        if not self.bal_bays or self.damage >= 2:
            return
        mb = self.walls
        sm, sk = self.slab_mat
        x = self.x0
        dep = self.bal_depth
        us, step = self.bays["-x"]
        for o in self.ops["-x"]:
            if o.kind not in ("balcony_door", "burnt") or o.bay not in self.bal_bays or o.storey < 1:
                continue
            if self.in_R(x - 0.5, o.u, 0.5) and self.tops.get("-x") and self.tops["-x"](o.u) < o.z1:
                continue
            w = min(step - 0.3, o.w + 1.8)
            lev = self.level(o.storey)
            mb.style(kind=sk)
            mb.box((x - dep / 2, o.u, lev - 0.07), (dep, w, 0.16), sm)
            # parapet: front plate and two side plates (both faces)
            mb.style(kind=self.wall_kind)
            mb.box((x - dep + 0.05, o.u, lev + 0.5), (0.1, w, 0.86), self.wall_mat)
            for s in (-1, 1):
                uu = o.u + s * (w / 2 - 0.04)
                for nrm in (1, -1):
                    self.quad(mb, [(x, uu, lev + 0.01), (x - dep + 0.1, uu, lev + 0.01),
                                   (x - dep + 0.1, uu, lev + 0.93), (x, uu, lev + 0.93)], self.wall_mat, (0, nrm, 0))

    def fire_escapes(self):
        """Prompt 202: iron fire escapes on the street side (+x): a grated
        platform with a railing before the windows of the given bays on
        every upper floor, ladders between them and a drop ladder below."""
        if not self.fire_escape or self.damage >= 2:
            return
        mb = self.walls
        x = self.x1
        dep = min(0.95, max(self.ledge, self.front, self.canopy if self.entrances else 0.0) - 0.02)
        _, step = self.bays["+x"]
        mb.style(kind=3)
        for bay in self.fire_escape:
            wins = sorted((o for o in self.ops["+x"] if o.bay == bay and o.storey >= 1 and o.kind != "door"),
                          key=lambda o: o.storey)
            w = min(step * 1.6, 3.2)
            for i, o in enumerate(wins):
                if self.tops.get("+x") and self.tops["+x"](o.u) < o.z1:
                    continue
                lev = self.level(o.storey) + 0.25
                mb.box((x + dep / 2, o.u, lev), (dep, w, 0.05), "Metal")
                mb.box((x + dep - 0.02, o.u, lev + 0.95), (0.03, w, 0.04), "Metal")       # top rail
                mb.box((x + dep - 0.02, o.u, lev + 0.50), (0.03, w, 0.03), "Metal")
                for k in (-1, 1):
                    mb.box((x + dep / 2, o.u + k * (w / 2 - 0.02), lev + 0.95), (dep, 0.03, 0.04), "Metal")
                    mb.box((x + dep - 0.02, o.u + k * (w / 2 - 0.02), lev + 0.48), (0.03, 0.03, 0.96), "Metal")
                # ladder down to the platform below (or the drop ladder under the lowest one)
                below = (self.level(wins[i - 1].storey) + 0.25) if i else lev - self.sh * 0.75
                side = 1 if i % 2 else -1
                ya, yb = o.u + side * (w / 2 - 0.25), o.u + side * (w / 2 - 0.25 - (0.0 if not i else 0.9))
                for off in (-0.2, 0.2):
                    mb.oriented_box(((x + dep * 0.55), (ya + yb) / 2 + off * 0.0 + off, (lev + below) / 2),
                                    (0.0, yb - ya, below - lev) if i else (0.0, 0.0, -1.0), (1.0, 0.0, 0.0),
                                    (math.hypot(yb - ya, lev - below), 0.04, 0.04), "Metal")

    def stoops(self):
        """Prompt 202: a high stoop in front of every door on the street side
        (+x): steps from the pavement up to the raised ground floor between
        two cheek walls."""
        if self.stoop <= 0 or self.damage >= 2:
            return
        mb = self.walls
        sm, sk = self.slab_mat
        for o in self.ops["+x"]:
            if o.kind != "door" or o.z0 < 0.3:
                continue
            n = max(3, int(o.z0 / 0.2))
            run = self.stoop / n
            mb.style(kind=sk)
            for i in range(n):
                z = o.z0 * (n - i) / n
                mb.box((self.x1 + run * (i + 0.5), o.u, z / 2), (run, o.w + 0.3, z), sm)
            for k in (-1, 1):
                mb.box((self.x1 + self.stoop / 2, o.u + k * (o.w / 2 + 0.28), (o.z0 + 0.5) / 2),
                       (self.stoop, 0.26, o.z0 + 0.5), sm, bevel=0.02)

    # -- roofs --------------------------------------------------------------------------

    def flat_roof(self):
        if self.damage >= 2 or self.roof is None:
            return
        mb = self.roof
        t = self.t
        x0, x1, y0, y1 = self.x0 + t, self.x1 - t, self.y0 + t, self.y1 - t
        z = self.roof_z
        mb.style(kind=self.roof_kind)
        rects = [(x0, x1, y0, y1)]
        if self.R is not None:
            rx0, rx1, ry0, ry1 = self.R[:4]
            rects = self.minus(x0, x1, y0, y1, rx0, rx1, ry0, ry1)
            self.hanging(mb, z)
        for a, b, c, d in rects:
            self.slab(mb, a, b, c, d, z, 0.3, "Plaster")
        # coping on the parapets
        sm, sk = self.slab_mat
        for side in SIDES:
            self.band_box(side, self.z1 - 0.08, self.z1, 0.04, sm, sk, ops_cut=False)

    def slab(self, mb, x0, x1, y0, y1, z, t, mat, cell=2.5):
        """Slab with its top face in cells (the AO bake is per vertex: a
        single quad would take the corner shade of the parapets)."""
        nx, ny = max(1, round((x1 - x0) / cell)), max(1, round((y1 - y0) / cell))
        xs = [x0 + (x1 - x0) * i / nx for i in range(nx + 1)]
        ys = [y0 + (y1 - y0) * j / ny for j in range(ny + 1)]
        for xa, xb in zip(xs, xs[1:]):
            for ya, yb in zip(ys, ys[1:]):
                self.quad(mb, [(xa, ya, z), (xb, ya, z), (xb, yb, z), (xa, yb, z)], mat, (0, 0, 1))
        zb = z - t
        self.quad(mb, [(x0, y0, zb), (x1, y0, zb), (x1, y1, zb), (x0, y1, zb)], mat, (0, 0, -1))
        for (xa, ya), (xb, yb), n in (((x0, y0), (x1, y0), (0, -1, 0)), ((x1, y1), (x0, y1), (0, 1, 0)),
                                      ((x0, y1), (x0, y0), (-1, 0, 0)), ((x1, y0), (x1, y1), (1, 0, 0))):
            self.quad(mb, [(xa, ya, zb), (xb, yb, zb), (xb, yb, z), (xa, ya, z)], mat, n)

    @staticmethod
    def minus(x0, x1, y0, y1, rx0, rx1, ry0, ry1):
        """Rectangle minus a rectangle touching one of its corners: up to two rectangles."""
        res = []
        ry0, ry1 = max(ry0, y0), min(ry1, y1)
        rx0, rx1 = max(rx0, x0), min(rx1, x1)
        # full-width strip outside the y range of R, and the part beside R
        if ry0 > y0 + 0.05:
            res.append((x0, x1, y0, ry0))
        if ry1 < y1 - 0.05:
            res.append((x0, x1, ry1, y1))
        if rx0 > x0 + 0.05:
            res.append((x0, rx0, ry0, ry1))
        if rx1 < x1 - 0.05:
            res.append((rx1, x1, ry0, ry1))
        return res

    def hanging(self, mb, z):
        """A broken slab hanging from the edge of the breach into the room."""
        x0, x1, y0, y1, sx, sy, k, zb = self.R
        r = self.r
        ye = y0 if sy > 0 else y1
        xm = (x0 + x1) / 2
        ln = r.uniform(1.6, 2.4)
        a = r.uniform(0.5, 0.9)
        sm, sk = self.slab_mat
        mb.style(kind=sk)
        c = Vector((xm, ye + sy * math.cos(a) * ln / 2, z - 0.15 - math.sin(a) * ln / 2))
        mb.oriented_box(c, (0, sy * math.cos(a), -math.sin(a)), (1, 0, 0), (ln, (x1 - x0) * 0.6, 0.22), sm)

    def gable_roof(self):
        if self.damage >= 2 or self.roof is None:
            return
        mb = self.roof
        box = self.box
        x0, x1 = -box.hw, box.hw
        y0, y1 = -box.hl, box.hl
        z_wall = self.z1
        rise = self.ridge - z_wall
        z_e = z_wall - rise / (box.hw - self.eave) * self.eave
        half = box.hw
        slope = math.hypot(half, self.ridge - z_e)
        rz = self.ridge - z_e
        strips = max(6, int(box.length / 2.5))
        sl = (y1 - y0) / strips
        slab = 0.18
        for sd in (1, -1):
            dvec = Vector((-sd * half, 0, rz)).normalized()
            n = Vector((sd * rz, 0, half)).normalized()
            for i in range(strips):
                yc = y0 + sl * (i + 0.5)
                mid = Vector((sd * half / 2, yc, z_e + rz / 2))
                if self.R is not None and sd == self.R[4] and self.R[2] - 0.5 < yc < self.R[3] + 0.5:
                    if abs(yc - (self.R[2] if self.R[5] > 0 else self.R[3])) < sl * 1.2:
                        mb.style(kind=0)  # rafters left at the edge of the hole
                        for kk in (-0.3, 0.3):
                            mb.oriented_box(mid + Vector((0, kk * sl, 0)) - n * 0.12, dvec, (0, 1, 0),
                                            (slope * 0.96, 0.12, 0.16), "Wood")
                    continue
                mb.style(kind=self.roof_kind)
                mb.oriented_box(mid - n * (slab / 2) + n * (slab / 2), dvec, (0, 1, 0), (slope, sl * 1.001, slab), "Roof")
        if self.R is None:
            mb.style(kind=self.roof_kind)
            mb.oriented_box((0, 0, self.ridge + 0.02), (0, 1, 0), (1, 0, 0), (y1 - y0, 0.3, 0.12), "Roof")
        # fire-wall gables, as halves so that the breach can take one
        wm = self.walls
        wm.style(kind=self.wall_kind)
        t = self.t
        for sy, yy in ((1, self.y1), (-1, self.y0)):
            for sx in (1, -1):
                if self.R is not None and self.R[4] == sx and self.R[5] == sy:
                    continue
                xo = self.x1 if sx > 0 else self.x0
                zt = z_wall + rise * 0.0
                tri = [(0.0, zt), (xo, zt), (0.0, self.ridge - 0.12)]
                ya = yy - sy * t
                wm.extrude([(x, min(ya, yy), z) for x, z in tri], (0, t, 0), self.wall_mat)
        # eave cornice
        if self.cornice:
            sm, sk = self.slab_mat
            for side in ("+x", "-x"):
                self.band_box(side, z_wall - 0.35, z_wall, self.eave - 0.05, self.wall_mat, self.wall_kind,
                              ops_cut=False)

    def roof_props(self):
        """Stair heads, water tanks, chimneys and vents (flat roofs); chimneys
        on the ridge (saddle roofs). Placed by the design seed, so damaged
        variants keep what survived in the same places."""
        if self.damage >= 2 or self.roof is None:
            return
        mb = self.roof
        d = self.d
        t = self.t
        z = self.roof_z
        if self.roof_type != "flat":
            n = self.chimneys
            for i in range(n):
                y = self.y0 + 2.0 + (self.y1 - self.y0 - 4.0) * (i + 0.5) / n + d.uniform(-0.6, 0.6)
                x = d.choice((-0.35, 0.35))
                zb = self.ridge - 1.6
                if self.in_R(x, y, 0.8):
                    continue
                mb.style(kind=3)
                mb.box((x, y, (zb + self.H - 0.1) / 2), (0.55, 1.1, self.H - 0.1 - zb), "Stone")
                mb.style(kind=2)
                mb.box((x, y, self.H - 0.05), (0.7, 1.25, 0.1), "Stone")
            return
        x0, x1, y0, y1 = self.x0 + t + 0.3, self.x1 - t - 0.3, self.y0 + t + 0.3, self.y1 - t - 0.3
        taken = []

        def free(cx, cy, hx, hy):
            if cx - hx < x0 or cx + hx > x1 or cy - hy < y0 or cy + hy > y1:
                return False
            if self.in_R(cx, cy, max(hx, hy) + 0.3):
                return None
            return not any(abs(cx - a) < hx + ha + 0.6 and abs(cy - b) < hy + hb + 0.6 for a, b, ha, hb in taken)

        sw, sd_ = self.stair_head
        sm, sk = self.slab_mat
        for o in self.ops["+x"]:
            if o.kind != "door" and not (o.kind == "burnt" and o.storey == 0):
                continue
            cx, cy = x1 - sd_ / 2 - 0.4, o.u
            taken.append((cx, cy, sd_ / 2, sw / 2))
            if self.in_R(cx, cy, 1.0):
                continue
            top = self.H - 0.12
            mb.style(kind=self.wall_kind)
            mb.box((cx, cy, (z + top) / 2), (sd_, sw, top - z), self.wall_mat)
            mb.style(kind=sk)
            mb.box((cx, cy, self.H - 0.06), (sd_ + 0.2, sw + 0.2, 0.12), sm)
            mb.style(kind=2)
            self.wquad(mb, "y", cy - sw / 2 - 0.02, cx - 0.45, cx + 0.45, z, z + 2.0, "Wood", (0, -1, 0))
        tries = 0
        placed = {"tank": 0, "chimney": 0, "vent": 0}
        want = {"tank": self.tanks, "chimney": self.chimneys, "vent": self.vents}
        while tries < 400 and any(placed[k] < want[k] for k in want):
            tries += 1
            kind = d.choice([k for k in want if placed[k] < want[k]])
            if kind == "tank":
                r = d.uniform(0.5, 0.65)
                hx = hy = r + 0.05
                # tanks stand near the stair heads (the risers)
            elif kind == "chimney":
                hx, hy = 0.3, 0.4
            else:
                hx = hy = 0.3
            cx, cy = d.uniform(x0 + hx, x1 - hx), d.uniform(y0 + hy, y1 - hy)
            if kind == "chimney":  # chimneys stand at the walls
                cx = d.choice((x0 + hx, x1 - hx))
            f = free(cx, cy, hx, hy)
            if f is False:
                continue
            taken.append((cx, cy, hx, hy))
            placed[kind] += 1
            if f is None:
                continue  # it stood where the breach is
            if kind == "tank":
                h = d.uniform(1.1, 1.5)
                zs = z + 0.45
                mb.style(kind=sk)
                for s in (-1, 1):
                    mb.box((cx + s * r * 0.5, cy, z + 0.22), (0.22, 2 * r + 0.1, 0.45), sm)
                mb.style(kind=3)
                mb.cylinder((cx, cy, zs), (cx, cy, zs + h), r, seg=10, mat="Metal")
            elif kind == "chimney":
                h = d.uniform(1.0, 1.6)
                mb.style(kind=self.wall_kind)
                mb.box((cx, cy, z + h / 2), (2 * hx, 2 * hy, h), self.wall_mat)
                mb.style(kind=sk)
                mb.box((cx, cy, z + h + 0.05), (2 * hx + 0.12, 2 * hy + 0.12, 0.1), sm)
            else:
                h = d.uniform(0.5, 0.8)
                mb.style(kind=1)
                mb.box((cx, cy, z + h / 2), (0.35, 0.35, h), "Metal")
                mb.box((cx, cy, z + h + 0.08), (0.6, 0.6, 0.1), "Metal")

    # -- interior (damage) ----------------------------------------------------------------

    def interior(self):
        fl = self.floor
        sm, sk = self.slab_mat
        t = self.t
        ix0, ix1, iy0, iy1 = self.x0 + t, self.x1 - t, self.y0 + t, self.y1 - t
        r = self.r
        if self.damage < 2:
            fl.style(kind=sk)
            fl.box(((ix0 + ix1) / 2, (iy0 + iy1) / 2, self.plinth - 0.05), (ix1 - ix0, iy1 - iy0, 0.1), sm)
        if self.damage == 1 and self.R is not None:
            x0, x1, y0, y1, sx, sy, k, zb = self.R
            x0, x1, y0, y1 = max(x0, ix0), min(x1, ix1), max(y0, iy0), min(y1, iy1)
            low = self.storeys - k
            for s in range(low, self.storeys):
                z = self.level(s)
                fl.style(kind=sk)
                if s == low:
                    fl.box(((x0 + x1) / 2, (y0 + y1) / 2, z - 0.12), (x1 - x0, y1 - y0, 0.24), sm)
                else:
                    bx = r.uniform(1.8, 3.0)
                    by = r.uniform(2.0, 3.5)
                    cxo = x1 if sx > 0 else x0
                    cyo = y1 if sy > 0 else y0
                    for a, b, c, dd in self.minus(x0, x1, y0, y1, *sorted((cxo, cxo - sx * bx)),
                                                  *sorted((cyo, cyo - sy * by))):
                        fl.box(((a + b) / 2, (c + dd) / 2, z - 0.12), (b - a, dd - c, 0.24), sm)
            # partitions: the walls closing the corner rooms, one between them
            zl = self.level(low) - 0.02
            ztop = self.roof_z - 0.3
            fl.style(kind=self.inner_kind)
            yb = y0 if sy > 0 else y1
            fl.box(((x0 + x1) / 2, yb, (zl + ztop) / 2), (x1 - x0, 0.18, ztop - zl), "Plaster")
            xb = x0 if sx > 0 else x1
            fl.box((xb, (y0 + y1) / 2, (zl + ztop) / 2), (0.18, y1 - y0, ztop - zl), "Plaster")
            ym = (y0 + y1) / 2 + r.uniform(-0.8, 0.8)
            for s in range(low, self.storeys):
                za = self.level(s)
                zt = self.level(s + 1) - 0.25 if s == low else za + r.uniform(0.8, 1.8)
                ln = (x1 - x0) * (1.0 if s == low else r.uniform(0.4, 0.7))
                xc = xb + sx * ln / 2
                fl.box((xc, ym, (za + zt) / 2), (ln, 0.15, zt - za), "Plaster")
            # rubble on the lowest open floor
            db = self.debris
            for _k in range(2):
                cx, cy = r.uniform(x0 + 1.0, x1 - 1.0), r.uniform(y0 + 1.0, y1 - 1.0)
                db.style(kind=self.wall_kind)
                db.blob((cx, cy, self.level(low)), (r.uniform(0.9, 1.5), r.uniform(0.9, 1.5), r.uniform(0.4, 0.7)),
                        self.wall_mat, seg=7, rings=4, rng=r, rough=0.25, flat_bottom=self.level(low), smooth=False)
        elif self.damage >= 2:
            # ground floor, first-floor remnants, partition stumps
            fl.style(kind=sk)
            fl.box(((ix0 + ix1) / 2, (iy0 + iy1) / 2, self.plinth - 0.05), (ix1 - ix0, iy1 - iy0, 0.1), sm)
            z1 = self.level(1)
            # first-floor slab remnants along the walls, one slab leaning
            for _k in range(3):
                ya = r.uniform(iy0, iy1 - 3.0)
                yb = min(iy1, ya + r.uniform(1.8, 3.2))
                sx = r.choice((-1, 1))
                xa, xb = (ix1 - r.uniform(1.2, 2.4), ix1) if sx > 0 else (ix0, ix0 + r.uniform(1.2, 2.4))
                fl.style(kind=sk)
                fl.box(((xa + xb) / 2, (ya + yb) / 2, z1 - 0.12), (xb - xa, yb - ya, 0.24), sm)
            yc = r.uniform(iy0 + 3.0, iy1 - 3.0)
            a = r.uniform(0.35, 0.6)
            ln = min(4.0, (ix1 - ix0) * 0.6)
            fl.oriented_box((ix0 + ln * math.cos(a) / 2 + 0.2, yc, z1 - ln * math.sin(a) / 2),
                            (math.cos(a), 0, -math.sin(a)), (0, 1, 0), (ln, 2.5, 0.24), sm)
            n = max(2, int((iy1 - iy0) / 6.0))
            fl.style(kind=self.inner_kind)
            for i in range(n):
                y = iy0 + (iy1 - iy0) * (i + 0.5) / n + r.uniform(-0.8, 0.8)
                h = r.uniform(0.8, z1 + 0.8)
                fl.box(((ix0 + ix1) / 2, y, self.plinth + h / 2), (ix1 - ix0, 0.18, h), "Plaster")

    def rubble_heaps(self):
        db = self.debris
        if db is None:
            return
        r = self.r
        hw, hl = self.box.hw, self.box.hl
        # broken masonry: dusty spoil, brick, the house's plaster
        mats = [("Ground", 1), ("Stone", 0), ("Stone", 3), (self.wall_mat, self.wall_kind), ("Stone", 0)]

        def heap(cx, cy, rx, ry, hz):
            cx = max(-hw + 0.3, min(hw - 0.3, cx))
            cy = max(-hl + 0.3, min(hl - 0.3, cy))
            rx = max(0.3, min(rx, (hw - abs(cx)) / 1.25))
            ry = max(0.3, min(ry, (hl - abs(cy)) / 1.25))
            m, k = mats[r.randrange(len(mats))]
            db.style(kind=k)
            db.blob((cx, cy, 0.0), (rx, ry, hz), m, seg=9, rings=6, rng=r, rough=0.3, flat_bottom=0.0, smooth=False)

        if self.damage == 1 and self.R is not None:
            x0, x1, y0, y1, sx, sy, k, zb = self.R
            xo = (self.x1 if sx > 0 else self.x0) + sx * 0.4
            for _k in range(3):
                heap(xo, r.uniform(y0, y1), r.uniform(0.8, 1.4), r.uniform(1.0, 2.0), r.uniform(0.5, 0.9))
            sm, sk = self.slab_mat
            db.style(kind=sk)
            c = (max(-hw + 0.6, min(hw - 0.6, xo)), (y0 + y1) / 2 + r.uniform(-1, 1), 0.5)
            db.oriented_box(c, (0.25 * sx, 1, 0.35), (1, 0, 0), (2.4, 0.9, 0.2), sm)
            # rubble under the shell holes
            for side in SIDES:
                for o in self.ops[side]:
                    if o.kind != "hole" or o.bay != -1:
                        continue
                    axis, sgn = side[1], _sgn(side)
                    px, py, _z = self.P(axis, self.coord(side) + sgn * 0.3, o.u, 0.0)
                    heap(px, py, 0.6, 0.6, 0.35)
        elif self.damage >= 2:
            t = self.t
            ix0, ix1, iy0, iy1 = self.x0 + t, self.x1 - t, self.y0 + t, self.y1 - t
            n = max(3, int((iy1 - iy0) / 6.5))
            hz = 0.15 * self.H
            for i in range(n):
                cy = iy0 + (iy1 - iy0) * (i + 0.5) / n + r.uniform(-1.5, 1.5)
                heap(r.uniform(ix0 + 2, ix1 - 2), cy, (ix1 - ix0) * r.uniform(0.32, 0.45), r.uniform(3.0, 4.5),
                     hz * r.uniform(0.7, 1.1))
            for side in ("+x", "-x", "+y", "-y"):
                axis, sgn = side[1], _sgn(side)
                u0, u1 = self.span(side)
                for _k in range(2 if axis == "x" else 1):
                    u = r.uniform(u0 + 2, u1 - 2)
                    px, py, _z = self.P(axis, self.coord(side) + sgn * 0.2, u, 0.0)
                    rx, ry = (1.4, 2.5) if axis == "x" else (2.5, 1.4)
                    heap(px, py, rx, ry, r.uniform(1.0, 1.8))
            sm, sk = self.slab_mat
            db.style(kind=sk)
            for _k in range(3):
                a = r.uniform(0, math.pi)
                c = (r.uniform(ix0 + 2, ix1 - 2), r.uniform(iy0 + 3, iy1 - 3), hz * 0.6)
                db.oriented_box(c, (math.cos(a), math.sin(a), r.uniform(-0.4, 0.4)), (0, 0, 1),
                                (r.uniform(2.0, 3.5), r.uniform(1.2, 2.2), 0.22), sm)

    # -- passage ---------------------------------------------------------------------------

    def passage_tunnel(self):
        if self.passage is None or self.damage >= 2:
            return
        o = next((o for o in self.ops["+x"] if o.kind == "passage"), None)
        if o is None:
            return
        mb = self.walls
        xa, xb = self.x0 + self.t, self.x1 - self.t
        mb.style(kind=self.wall_kind)
        for uu, s in ((o.lo, 1), (o.hi, -1)):
            self.quad(mb, [(xa, uu, 0.0), (xb, uu, 0.0), (xb, uu, o.z1), (xa, uu, o.z1)], self.wall_mat, (0, s, 0))
        self.quad(mb, [(xa, o.lo, o.z1), (xb, o.lo, o.z1), (xb, o.hi, o.z1), (xa, o.hi, o.z1)], self.wall_mat,
                  (0, 0, -1))
        self.floor.style(kind=0)
        self.quad(self.floor, [(self.x0, o.lo, 0.02), (self.x1, o.lo, 0.02), (self.x1, o.hi, 0.02),
                               (self.x0, o.hi, 0.02)], "Stone", (0, 0, 1))

    # -- all ---------------------------------------------------------------------------------

    def build(self):
        self.openings()
        self.damage_plan()
        for side in SIDES:
            self.facade(side)
        hb, kb = self.plinth_band
        for side in SIDES:
            self.band_box(side, 0.0, max(hb, 0.05), 0.08, "Stone", kb)
            if self.band:
                z = self.level(1) - 0.05
                self.band_box(side, z - 0.2, z, 0.1, self.wall_mat, self.wall_kind)
        if self.top_cornice > 0 and self.roof_type == "flat" and self.damage < 2:
            sm, sk = self.slab_mat
            for side in SIDES:
                self.band_box(side, self.z1 - 0.55, self.z1 - 0.05, self.top_cornice if side == "+x" else 0.08, sm, sk)
        self.canopies()
        self.balconies()
        self.fire_escapes()
        self.stoops()
        self.passage_tunnel()
        if self.roof_type == "flat":
            self.flat_roof()
        else:
            self.gable_roof()
        self.roof_props()
        self.interior()
        self.rubble_heaps()


@env_part("apartment_block")
def apartment_block(model, name, p, box):
    """Apartment block of four to six storeys (prompt 078), one variant =
    one damage level (`damage` 0 / 1 / 2).

    Storeys: `storeys` (5), `storey_h` (2.9), `ground_h` (ground floor,
    default storey_h), `plinth` (raised ground floor, 0.3). Walls: `wall`
    ("Plaster"), `wall_kind`, `wall_t` (0.4), `inner_kind` (Plaster kind
    of the partitions), `slab` (["Stone", 2]: slot and kind of slabs,
    sills, canopies, coping). Bays: `bays_long` / `bays_short` (window
    columns per long / short side, 0 = blind fire wall), `bay_margin`
    (1.2), `window` ([w, h] 1.1 x 1.4), `window_ground`, `sill` (0.9),
    `shutters` (share of closed windows, 0.12), `streaks` (share of windows
    with a dirt streak, 0.6). Street side +x: `entrances` (bay indices; a
    door below, stairwell windows at the half landings above), `door`
    ([w, h]), `door_at_ground` (door from the ground instead of the raised
    floor), `canopy` (depth of the concrete canopy, 1.2; 0 = none),
    `stoop` (prompt 202: depth of a high stoop with cheek walls up to a
    raised door; needs `front` at least as deep and `canopy` 0),
    `fire_escape` (prompt 202: bay indices with iron fire escapes on the
    upper floors; they are as deep as `front`, at most 0.95), `front`
    (prompt 202: the street wall stands this far behind the box edge, room
    for stoops and fire escapes; flat roofs),
    `top_cornice` (prompt 202: projection of a cornice under the parapet of
    a flat roof on the street side),
    `passage` ([bay, width, height]: carriage passage through the ground
    floor). Courtyard side -x: `balconies` (bay indices, stacked from the
    first floor), `balcony_depth` (1.2). Roof: `roof` ("flat" | "gable");
    flat: `parapet` (0.9), `roof_kind`, `stair_head` ([along, across] in m,
    over every entrance, top = box height), `tanks`, `chimneys`, `vents`,
    `apron` (set-back of the walls from the box, 0.3); gable: `eave`
    (0.4), `chimney_above` (chimney tops above the ridge, 0.6), `chimneys`,
    `roof_kind`, `cornice`. Bands: `string_course` (above the ground
    floor), `plinth_band` ([height, Stone kind]). Damage: `burnt` (burnt-out
    windows when damaged, 8). `seed`, `damage`.
    """
    b = Block(model, name, p, box)
    b.build()
    b.finish()
    model.part_props.setdefault(name, {})["door_height_m"] = b.door[1]
