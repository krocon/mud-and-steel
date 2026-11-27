"""Buildings and bridges (prompt 007, part B).

Buildings are modular variants: an empty `<Variant>` with the meshes
`<Variant>_Walls` (walls, frames, windows, doors), `<Variant>_Roof`,
`<Variant>_Floor` (plinth, ground floor and upper floor slabs) and, when
damaged, `<Variant>_Debris`. The damage level (`damage` 0 intact,
1 damaged, 2 ruin) is a generator parameter, so the same plan builds all
three variants; prompt 020 can switch or combine these parts.

Plan conventions: the ridge runs along Y (catalog length), the box width
(X) includes the eaves overhang, the box height is the ridge (or the top of
the parapet / spire). Doors are 2.0 m high, window sills 0.9 m.
"""

import math

from mathutils import Vector

from .env import env_part, rng

DOOR_H = 2.0


def _sash_quad(mb, pts, normal, u_base=102.0):
    """Places a single explicit-UV window quad guaranteed to face `normal`
    (u_base=102: sash_window, u_base=103: white cross_window in env.gdshader)."""
    uvs = [(u_base + 0.001, 0.0), (u_base + 0.999, 0.0), (u_base + 0.999, 1.0), (u_base + 0.001, 1.0)]
    a, b, c = Vector(pts[0]), Vector(pts[1]), Vector(pts[2])
    if (b - a).cross(c - a).dot(Vector(normal)) < 0:
        pts = list(reversed(pts))
        uvs = list(reversed(uvs))
    mb.style(kind=0)
    mb.quad_uv(pts, uvs, "Glass", smooth=False)


def _shutter_quad(mb, pts, normal, w, h):
    """Single outward-facing 2-tri wooden shutter quad (prompt 215, saves 10 tris/shutter)."""
    uvs = [(0.0, h), (w, h), (w, 0.0), (0.0, 0.0)]
    a, b, c = Vector(pts[0]), Vector(pts[1]), Vector(pts[2])
    if (b - a).cross(c - a).dot(Vector(normal)) < 0:
        pts = list(reversed(pts))
        uvs = list(reversed(uvs))
    mb.style(kind=0)
    mb.quad_uv(pts, uvs, "Wood", smooth=False)


class Opening:
    def __init__(self, u, w, z0, z1, kind):
        self.u, self.w, self.z0, self.z1, self.kind = u, w, z0, z1, kind

    @property
    def lo(self):
        return self.u - self.w / 2

    @property
    def hi(self):
        return self.u + self.w / 2


class House:
    """Builds the parts of one building variant."""

    def __init__(self, model, name, p, box, wall_mat, wall_kind):
        self.model = model
        self.name = name
        self.box = box
        self.r = rng(p.get("seed", 1))
        self.damage = p.get("damage", 0)
        self.wall_mat = wall_mat
        self.wall_kind = wall_kind
        model.group(name)
        model.part_props[name] = {"damage": ["intact", "damaged", "ruin"][self.damage]}
        self.walls = model.mb(f"{name}_Walls", parent=name)
        self.roof = model.mb(f"{name}_Roof", parent=name) if self.damage < 2 or p.get("ruin_roof", True) else None
        self.floor = model.mb(f"{name}_Floor", parent=name)
        self.debris = model.mb(f"{name}_Debris", parent=name) if self.damage else None
        self.frame_mat = "Wood"
        # Prompt 120: open wooden shutters beside every window.
        self.shutters = p.get("shutters", False)
        # Prompt 215: default intact/damaged windows to 1-quad sash_window
        # (env.gdshader UV.x = 102..103); opt out with `sash_windows = false`.
        self.sash_windows = bool(p.get("sash_windows", True))
        # Prompt 224: Dutch/Flemish white cross-mullion windows (UV.x = 103),
        # dressed lintels/sills independent of quoins, light eave cornice trim.
        self.cross_windows = bool(p.get("cross_windows", False))
        self.quoins = bool(p.get("quoins", False))
        self.lintels = bool(p.get("lintels", False))
        self.eave_trim = p.get("eave_trim", False)
        self.gable_trim = p.get("gable_trim", False)
        # Prompt 184: dressed stone window frames, segmental arch lintels, cornices.
        self.window_frame = bool(p.get("window_frame", False))
        self.window_arch = bool(p.get("window_arch", False))
        self.cornice = bool(p.get("cornice", False))

    # -- walls --------------------------------------------------------------------

    def ruin_top(self, z0, z1, u0, u1):
        """Jagged top line of a ruined wall: list of (u, z) breakpoints."""
        pts = []
        u = u0
        h = self.r.uniform(0.4, 0.9)
        while u < u1:
            pts.append((u, z0 + (z1 - z0) * h))
            u += self.r.uniform(0.7, 1.3)
            h = min(0.95, max(0.15, h + self.r.uniform(-0.3, 0.3)))
        return pts

    def wall(self, axis, coord, side, u0, u1, z0, z1, openings, t, top=None, frame=False):
        """Straight wall. axis "x": wall plane x = coord, runs along Y (u = y);
        axis "y": plane y = coord, runs along X. `side` = outward sign.
        `top` = ruin top line (list of (u, z)), clips every piece."""
        mb = self.walls
        mb.style(kind=self.wall_kind)
        c = coord - side * t / 2

        def piece(ua, ub, za, zb, mat=None, depth=None, inset=0.0, kind=None):
            if ub - ua < 0.02 or zb - za < 0.02:
                return
            # clip against the ruin top, split into ~1 m columns
            cols = [(ua, ub)]
            if top is not None:
                n = max(1, int((ub - ua) / 0.9 + 0.5))
                cols = [(ua + (ub - ua) * i / n, ua + (ub - ua) * (i + 1) / n) for i in range(n)]
            for ca, cb in cols:
                zt = zb
                if top is not None:
                    um = (ca + cb) / 2
                    zt = min(zb, min((z for u, z in top if abs(u - um) < 0.9), default=zb))
                if zt - za < 0.05:
                    continue
                d = depth if depth is not None else t
                cc = c - side * inset
                if kind is not None:
                    mb.style(kind=kind)
                center = (cc, (ca + cb) / 2, (za + zt) / 2) if axis == "x" else ((ca + cb) / 2, cc, (za + zt) / 2)
                size = (d, cb - ca, zt - za) if axis == "x" else (cb - ca, d, zt - za)
                mb.box(center, size, mat or self.wall_mat)
                mb.style(kind=self.wall_kind)

        ops = sorted(openings, key=lambda o: o.u)
        u = u0
        for o in ops:
            piece(u, o.lo, z0, z1)
            piece(o.lo, o.hi, z0, o.z0)
            piece(o.lo, o.hi, o.z1, z1)
            u = o.hi
            if top is not None and any(abs(tu - o.u) < 0.6 and tz < o.z1 for tu, tz in top):
                continue
            if o.kind in ("window", "double_window") and self.damage < 2:
                is_double = o.kind == "double_window"
                if self.sash_windows:
                    # Single explicit-UV quad with U = 102..103 (sash_window in env.gdshader).
                    gc = coord - side * (t * 0.35)
                    u_base = 103.0 if self.cross_windows else 102.0
                    panes = (
                        [(o.lo, o.u - 0.06), (o.u + 0.06, o.hi)]
                        if is_double
                        else [(o.lo, o.hi)]
                    )
                    for pa, pb in panes:
                        if axis == "x":
                            pts = [(gc, pa, o.z1), (gc, pb, o.z1), (gc, pb, o.z0), (gc, pa, o.z0)]
                            nrm = (side, 0, 0)
                        else:
                            pts = [(pa, gc, o.z1), (pb, gc, o.z1), (pb, gc, o.z0), (pa, gc, o.z0)]
                            nrm = (0, side, 0)
                        _sash_quad(mb, pts, nrm, u_base)
                    mb.style(kind=self.wall_kind)
                else:
                    piece(o.lo, o.hi, o.z0, o.z1, "Glass", depth=0.03, inset=t * 0.35)
                    fk = 2 if self.frame_mat == "Wood" else 0
                    piece(o.u - 0.03, o.u + 0.03, o.z0, o.z1, "Wood", depth=0.06, inset=t * 0.3, kind=fk)
                    piece(o.lo, o.hi, (o.z0 + o.z1) / 2 + 0.1, (o.z0 + o.z1) / 2 + 0.16, "Wood", depth=0.06,
                          inset=t * 0.3, kind=fk)
                if is_double:
                    # Central stone mullion dividing the double window.
                    piece(o.u - 0.065, o.u + 0.065, o.z0, o.z1, "Stone", depth=t * 0.85, inset=t * 0.04, kind=1)
                stone_trim = self.quoins or self.lintels or self.window_frame
                sill_mat = "Stone" if stone_trim else self.wall_mat
                sill_kind = 1 if stone_trim else self.wall_kind
                piece(o.lo - 0.06, o.hi + 0.06, o.z0 - 0.08, o.z0, sill_mat, depth=t + 0.08, inset=-0.05, kind=sill_kind)
                if self.window_frame:
                    # Dressed stone side jambs & lintel band framing the opening (prompt 184).
                    piece(o.lo - 0.10, o.lo + 0.01, o.z0, o.z1, "Stone", depth=t + 0.05, inset=-0.03, kind=1)
                    piece(o.hi - 0.01, o.hi + 0.10, o.z0, o.z1, "Stone", depth=t + 0.05, inset=-0.03, kind=1)
                    if self.window_arch:
                        # Flat segmental brick arch lintel above the window.
                        piece(o.lo - 0.12, o.hi + 0.12, o.z1, o.z1 + 0.18, "Stone", depth=t + 0.06, inset=-0.035, kind=3)
                    else:
                        piece(o.lo - 0.10, o.hi + 0.10, o.z1, o.z1 + 0.15, "Stone", depth=t + 0.05, inset=-0.03, kind=1)
                elif stone_trim:
                    # Crisp stone lintel cap above each window.
                    piece(o.lo - 0.10, o.hi + 0.10, o.z1, o.z1 + 0.16, "Stone", depth=t + 0.06, inset=-0.04, kind=1)
                if self.shutters:
                    # Folded back against the wall, half the window wide each (2-tri quads, prompt 215).
                    sc = coord + side * 0.03
                    sw = o.w / 2
                    sh = o.z1 - o.z0
                    nrm = (side, 0, 0) if axis == "x" else (0, side, 0)
                    for ua, ub in ((o.lo - sw - 0.04, o.lo - 0.04), (o.hi + 0.04, o.hi + sw + 0.04)):
                        if axis == "x":
                            spts = [(sc, ua, o.z1), (sc, ub, o.z1), (sc, ub, o.z0), (sc, ua, o.z0)]
                        else:
                            spts = [(ua, sc, o.z1), (ub, sc, o.z1), (ub, sc, o.z0), (ua, sc, o.z0)]
                        _shutter_quad(mb, spts, nrm, sw, sh)
                    mb.style(kind=self.wall_kind)
            elif o.kind == "shop" and self.damage < 2:
                # Prompt 120: shop front, glazing over a wooden base and a
                # fascia board above.
                piece(o.lo, o.hi, o.z0 + 0.6, o.z1, "Glass", depth=0.03, inset=t * 0.35)
                piece(o.lo, o.hi, o.z0, o.z0 + 0.6, "Wood", depth=0.06, inset=t * 0.4, kind=2)
                piece(o.lo - 0.1, o.hi + 0.1, o.z1, o.z1 + 0.35, "Wood", depth=0.06, inset=-0.05, kind=2)
            elif o.kind in ("door", "barn_door"):
                piece(o.lo, o.hi, o.z0, o.z1, "Wood", depth=0.06, inset=t * 0.6, kind=2)
                if (self.quoins or self.lintels) and o.kind == "door":
                    piece(o.lo - 0.12, o.hi + 0.12, o.z1, o.z1 + 0.18, "Stone", depth=t + 0.06, inset=-0.04, kind=1)
            elif o.kind == "arch_window" and self.damage < 2:
                piece(o.lo, o.hi, o.z0, o.z1, "Glass", depth=0.03, inset=t * 0.4)
            elif o.kind == "louver":
                for k in range(3):
                    zz = o.z0 + (o.z1 - o.z0) * (k + 0.5) / 3
                    piece(o.lo, o.hi, zz - 0.04, zz + 0.04, "Wood", depth=0.04, inset=t * 0.5, kind=2)
        piece(u, u1, z0, z1)
        if frame and top is None:
            self.timber_frame(axis, coord, side, u0, u1, z0, z1, ops)

    def timber_frame(self, axis, coord, side, u0, u1, z0, z1, ops):
        """Half-timbering on the outside of a wall: sill, rails, posts, braces."""
        mb = self.walls
        mb.style(kind=2)
        c = coord + side * 0.015
        b = 0.16

        def beam(a, bb, rot=None):
            a, bb = Vector(a), Vector(bb)
            d = bb - a
            n = Vector((1, 0, 0)) if axis == "x" else Vector((0, 1, 0))
            mb.oriented_box((a + bb) / 2, d, d.cross(n), (d.length, b, 0.05), "Wood")

        def P(u, z):
            return (c, u, z) if axis == "x" else (u, c, z)

        beam(P(u0, z0 + b / 2), P(u1, z0 + b / 2))
        beam(P(u0, z1 - b / 2), P(u1, z1 - b / 2))
        mid = z0 + 0.9 - 0.08
        posts = sorted({u0 + b / 2, u1 - b / 2} | {o.lo - b / 2 for o in ops} | {o.hi + b / 2 for o in ops})
        for pu in posts:
            beam(P(pu, z0 + b), P(pu, z1 - b))
        spans = list(zip(posts, posts[1:]))
        for ua, ub in spans:
            inside = any(o.lo - 0.2 < (ua + ub) / 2 < o.hi + 0.2 for o in ops)
            if inside:
                continue
            beam(P(ua, mid), P(ub, mid))
            if ub - ua > 0.9:
                beam(P(ua + b / 2, z0 + b), P(ub - b / 2, mid))

    def gable(self, axis, coord, side, u0, u1, z0, z1, t, frame=False):
        """Triangular gable wall above the eaves."""
        if self.damage >= 2:
            return
        mb = self.walls
        mb.style(kind=self.wall_kind)
        c = coord - side * t / 2
        um = (u0 + u1) / 2
        tri = [(u0, z0), (u1, z0), (um, z1)]
        if axis == "y":
            mb.extrude([(u, c - t / 2, z) for u, z in tri], (0, t, 0), self.wall_mat)
        else:
            mb.extrude([(c - t / 2, u, z) for u, z in tri], (t, 0, 0), self.wall_mat)
        if frame:
            mb.style(kind=2)
            cc = coord + side * 0.015
            n = Vector((1, 0, 0)) if axis == "x" else Vector((0, 1, 0))
            for a, b in (((u0, z0 + 0.08), (um, z1 - 0.05)), ((u1, z0 + 0.08), (um, z1 - 0.05)),
                         ((um, z0), (um, z1)), ((u0 + (um - u0) * 0.5, z0), (u0 + (um - u0) * 0.5, (z0 + z1) / 2)),
                         ((u1 - (u1 - um) * 0.5, z0), (u1 - (u1 - um) * 0.5, (z0 + z1) / 2))):
                pa = Vector((cc, a[0], a[1])) if axis == "x" else Vector((a[0], cc, a[1]))
                pb = Vector((cc, b[0], b[1])) if axis == "x" else Vector((b[0], cc, b[1]))
                d = pb - pa
                mb.oriented_box((pa + pb) / 2, d, d.cross(n), (d.length, 0.15, 0.05), "Wood")

    def stepped_gable(self, axis, coord, side, u0, u1, z0, z1, t, steps=5, z_ground=None):
        """Prompt 224: Dutch/Flemish crow-stepped gable wall rising proud of the
        roof planes with dressed stone coping caps on each step. Preserves a
        solidly grounded jagged partial stepped-gable silhouette on `+Y` in
        `Ruin` (`damage >= 2`)."""
        mb = self.walls
        steps = max(3, int(steps))
        c = coord - side * t / 2
        um = (u0 + u1) / 2
        half = (u1 - u0) / 2
        rise = z1 - z0
        if rise <= 0.1:
            return
        # In Ruin only keep the iconic jagged stepped gable on +Y (anchored all
        # the way down to z_ground so nothing ever floats above a ruined wall!).
        if self.damage >= 2:
            if side < 0:
                return
            keep = max(2, int(round(steps * 0.72)))
            tier_h = rise / steps
            base_z = z_ground if z_ground is not None else max(0.0, z0 - 2.5)
            # Solid ruined gable wall backing up to z0 across the surviving width.
            mb.style(kind=self.wall_kind)
            lo_base = um - half * 0.85
            hi_base = um + half * 0.92
            if axis == "y":
                mb.box(((lo_base + hi_base) / 2, c, (base_z + z0) / 2), (hi_base - lo_base, t, z0 - base_z), self.wall_mat)
            else:
                mb.box((c, (lo_base + hi_base) / 2, (base_z + z0) / 2), (t, hi_base - lo_base, z0 - base_z), self.wall_mat)
            for k in range(keep):
                za = z0 + k * tier_h
                zb = za + tier_h
                frac = 1.0 - float(k) / float(steps)
                hw = max(0.36, half * frac)
                lo = max(lo_base, um - hw)
                hi = min(hi_base, um + hw)
                if k >= keep - 2:
                    lo += hw * (0.28 + 0.18 * (k - (keep - 2)))
                if hi - lo < 0.35:
                    continue
                if axis == "y":
                    mb.box(((lo + hi) / 2, c, (za + zb) / 2), (hi - lo, t, zb - za), self.wall_mat)
                else:
                    mb.box((c, (lo + hi) / 2, (za + zb) / 2), (t, hi - lo, zb - za), self.wall_mat)
            return
        tier_h = rise / steps
        for k in range(steps):
            za = z0 + k * tier_h
            zb = za + tier_h
            frac = 1.0 - float(k) / float(steps)
            hw = max(0.36, half * frac)
            lo = um - hw
            hi = um + hw
            if self.damage == 1 and k == steps - 1 and side > 0:
                hi = um + hw * 0.35
            mb.style(kind=self.wall_kind)
            if axis == "y":
                mb.box(((lo + hi) / 2, c, (za + zb) / 2), (hi - lo, t, zb - za), self.wall_mat)
            else:
                mb.box((c, (lo + hi) / 2, (za + zb) / 2), (t, hi - lo, zb - za), self.wall_mat)
            # Crisp stone coping cap on top of each exposed step (intact & damaged).
            cap_h = 0.075
            cap_t = t + 0.07
            mb.style(kind=1)
            if k == steps - 1:
                if axis == "y":
                    mb.box(((lo + hi) / 2, c, zb + cap_h / 2), (hi - lo + 0.08, cap_t, cap_h), "Stone")
                else:
                    mb.box((c, (lo + hi) / 2, zb + cap_h / 2), (cap_t, hi - lo + 0.08, cap_h), "Stone")
            else:
                next_hw = max(0.36, half * (1.0 - float(k + 1) / float(steps)))
                step_w = max(0.08, hw - next_hw)
                for sx in (-1, 1):
                    sc = um + sx * (next_hw + step_w / 2)
                    if axis == "y":
                        mb.box((sc, c, zb + cap_h / 2), (step_w + 0.06, cap_t, cap_h), "Stone")
                    else:
                        mb.box((c, sc, zb + cap_h / 2), (cap_t, step_w + 0.06, cap_h), "Stone")

    def rect_walls(self, x0, x1, y0, y1, z0, z1, openings, t, frame=False, ruin=None):
        """Four walls of a rectangle. `openings` = dict side -> [Opening],
        sides "+x", "-x" (long walls, u = y) and "+y", "-y" (u = x)."""
        tops = {}
        for side in ("+x", "-x", "+y", "-y"):
            if ruin is not None:
                u0, u1 = (y0, y1) if side[1] == "x" else (x0, x1)
                tops[side] = self.ruin_top(z0, z1, u0 - 1, u1 + 1)
        self.wall("x", x1, 1, y0, y1, z0, z1, openings.get("+x", []), t, tops.get("+x"), frame)
        self.wall("x", x0, -1, y0, y1, z0, z1, openings.get("-x", []), t, tops.get("-x"), frame)
        self.wall("y", y1, 1, x0 + t, x1 - t, z0, z1, openings.get("+y", []), t, tops.get("+y"), frame)
        self.wall("y", y0, -1, x0 + t, x1 - t, z0, z1, openings.get("-y", []), t, tops.get("-y"), frame)

    # -- roofs ----------------------------------------------------------------------

    def gable_roof(self, x0, x1, y0, y1, z_eave, z_ridge, kind, strips=8, slab=0.18):
        """Two roof planes from x0 / x1 (eaves) to the ridge at x = (x0+x1)/2.

        Prompt 240: damaged roofs (`damage == 1`) use a 2D strip x row grid with
        ragged blast holes, exposed dark rafters & battens, a dark charred attic
        backing below the holes, and a partially broken ridge cap.
        """
        if self.roof is None:
            return
        mb = self.roof
        xm = (x0 + x1) / 2
        half = (x1 - x0) / 2
        rise = z_ridge - z_eave
        slope = math.hypot(half, rise)
        if self.damage >= 2:
            return slope
        sl = (y1 - y0) / strips
        if self.damage == 0:
            for sd in (1, -1):
                d = Vector((-sd * half, 0, rise)).normalized()
                n = Vector((sd * rise, 0, half)).normalized()
                for i in range(strips):
                    yc = y0 + sl * (i + 0.5)
                    mid = Vector((xm + sd * half / 2, yc, z_eave + rise / 2))
                    mb.style(kind=kind)
                    mb.oriented_box(mid + n * (slab / 2), d, Vector((0, 1, 0)), (slope, sl * 1.001, slab), "Roof")
            mb.style(kind=kind)
            mb.oriented_box((xm, (y0 + y1) / 2, z_ridge + 0.02), (0, 1, 0), (1, 0, 0), (y1 - y0, 0.3, 0.12), "Roof")
            return slope

        # Damaged (damage == 1): 2D grid (strips along Y x 3 rows from eave to ridge)
        # with irregular shell-hole clusters, exposed broken rafters/battens and dark attic.
        rows = 3
        holes = set()
        # Primary shell hole cluster on +1 slope, secondary cluster on -1 slope.
        s1 = self.r.randrange(0, max(1, strips - 1))
        r1 = self.r.randrange(0, rows)
        holes.add((1, s1, r1))
        holes.add((1, min(s1 + 1, strips - 1), r1))
        holes.add((1, s1, min(r1 + 1, rows - 1) if r1 < rows - 1 else max(0, r1 - 1)))
        if strips >= 5:
            s2 = (s1 + max(2, strips // 2)) % strips
            holes.add((1, s2, 1))
            holes.add((1, min(s2 + 1, strips - 1), 2))
        s3 = (s1 + 1) % strips
        holes.add((-1, s3, 1))
        holes.add((-1, s3, 2))
        if strips >= 4:
            holes.add((-1, (s3 + 1) % strips, 1))

        # Dark charred attic floor directly beneath the roof so holes read deep & burned.
        attic_w = max(0.6, (x1 - x0) - 0.9)
        attic_l = max(0.6, (y1 - y0) - 0.9)
        mb.style(kind=3)
        mb.box((xm, (y0 + y1) / 2, z_eave + max(0.12, rise * 0.12)), (attic_w, attic_l, 0.14), "Wood")

        row_slope = slope / rows
        for sd in (1, -1):
            d = Vector((-sd * half, 0, rise)).normalized()
            n = Vector((sd * rise, 0, half)).normalized()
            eave_x = xm + sd * half
            for i in range(strips):
                yc = y0 + sl * (i + 0.5)
                strip_has_hole = any((sd, i, r) in holes for r in range(rows))
                for r in range(rows):
                    u_mid = (r + 0.5) / rows
                    mid = Vector((eave_x - sd * half * u_mid, yc, z_eave + rise * u_mid))
                    if (sd, i, r) in holes:
                        # Exposed charred horizontal roof battens spanning the hole.
                        mb.style(kind=3)
                        for b_off in (-0.25, 0.25):
                            bu = (r + 0.5 + b_off) / rows
                            bmid = Vector((eave_x - sd * half * bu, yc, z_eave + rise * bu)) - n * 0.06
                            mb.oriented_box(bmid, d, Vector((0, 1, 0)), (0.09, sl * 1.04, 0.07), "Wood")
                        continue
                    mb.style(kind=kind)
                    # Slightly ragged inset on tiles adjacent to a hole.
                    adj = any((sd, i + di, r + dr) in holes for di, dr in ((-1, 0), (1, 0), (0, -1), (0, 1)))
                    len_s = row_slope * (0.94 if adj else 1.005)
                    wid_s = sl * (0.94 if adj else 1.004)
                    mb.oriented_box(mid + n * (slab / 2), d, Vector((0, 1, 0)), (len_s, wid_s, slab), "Roof")
                if strip_has_hole:
                    # Two dark/charred rafters running up the slope under the hole (one partly snapped).
                    mb.style(kind=3)
                    for k_idx, k in enumerate((-0.28, 0.28)):
                        frac = 0.72 if (k_idx == 1 and (i + sd) % 2 == 0) else 0.96
                        rmid = Vector((eave_x - sd * half * (frac * 0.5), yc + k * sl, z_eave + rise * (frac * 0.5))) - n * 0.09
                        mb.oriented_box(rmid, d, Vector((0, 1, 0)), (slope * frac, 0.11, 0.15), "Wood")
        # Broken ridge cap: draw only over strips where the top row survives on at least one slope.
        mb.style(kind=kind)
        for i in range(strips):
            if (1, i, rows - 1) in holes and (-1, i, rows - 1) in holes:
                continue
            yc = y0 + sl * (i + 0.5)
            mb.oriented_box((xm, yc, z_ridge + 0.02), (0, 1, 0), (1, 0, 0), (sl * 0.98, 0.28, 0.11), "Roof")
        return slope

    def hip_roof(self, x0, x1, y0, y1, z_eave, z_ridge, kind, strips=8, slab=0.18):
        """Prompt 184: four-sided hip roof (Walmdach) with equal slope on all
        four sides, ridge and hip cap tiles, and shell-hole damage on `damage == 1`."""
        if self.roof is None or self.damage >= 2:
            return
        mb = self.roof
        xm = (x0 + x1) / 2
        half = (x1 - x0) / 2
        rise = z_ridge - z_eave
        # Equal pitch on all four faces: hip ends inset by `half` along Y.
        hip_in = min(half, (y1 - y0) * 0.45)
        y_r0 = y0 + hip_in
        y_r1 = y1 - hip_in

        def slope_poly(pts):
            # Extrude polygon along its outward normal by `slab`.
            a, b, c = Vector(pts[0]), Vector(pts[1]), Vector(pts[2])
            n = (b - a).cross(c - a)
            if n.z < 0:
                pts = list(reversed(pts))
                n = -n
            n.normalize()
            mb.style(kind=kind)
            mb.extrude(pts, n * slab, "Roof")

        def ridge_caps(broken_mask=None):
            mb.style(kind=kind)
            # Horizontal ridge cap
            if y_r1 - y_r0 > 0.15:
                if broken_mask is None:
                    mb.oriented_box((xm, (y_r0 + y_r1) / 2, z_ridge + 0.05),
                                    (0, 1, 0), (1, 0, 0), (y_r1 - y_r0 + 0.12, 0.28, 0.12), "Roof")
                else:
                    n_seg = max(2, int((y_r1 - y_r0) / 1.0))
                    seg_l = (y_r1 - y_r0) / n_seg
                    for k in range(n_seg):
                        if k in broken_mask:
                            continue
                        mb.oriented_box((xm, y_r0 + seg_l * (k + 0.5), z_ridge + 0.05),
                                        (0, 1, 0), (1, 0, 0), (seg_l * 0.96, 0.28, 0.12), "Roof")
            # Four diagonal hip ridge caps
            for cx, cy, ry in ((x1, y0, y_r0), (x0, y0, y_r0), (x1, y1, y_r1), (x0, y1, y_r1)):
                p0 = Vector((cx, cy, z_eave + 0.04))
                p1 = Vector((xm, ry, z_ridge + 0.04))
                d = p1 - p0
                side_v = Vector((-d.y, d.x, 0.0))
                mb.oriented_box((p0 + p1) * 0.5, d, side_v, (d.length, 0.24, 0.12), "Roof")

        if self.damage == 0:
            # +X and -X trapezoidal main slopes
            slope_poly([(x1, y0, z_eave), (x1, y1, z_eave), (xm, y_r1, z_ridge), (xm, y_r0, z_ridge)])
            slope_poly([(x0, y1, z_eave), (x0, y0, z_eave), (xm, y_r0, z_ridge), (xm, y_r1, z_ridge)])
            # -Y and +Y triangular hip slopes
            slope_poly([(x0, y0, z_eave), (x1, y0, z_eave), (xm, y_r0, z_ridge)])
            slope_poly([(x1, y1, z_eave), (x0, y1, z_eave), (xm, y_r1, z_ridge)])
            ridge_caps()
            return

        # Damaged (damage == 1): dark charred attic floor + subdivided +X/-X hip
        # slopes with shell holes and exposed charred rafters & battens.
        mb.style(kind=3)
        mb.box((xm, (y0 + y1) / 2, z_eave + max(0.25, rise * 0.16)),
               (max(0.8, (x1 - x0) - 1.2), max(0.8, (y1 - y0) - 1.2), 0.14), "Wood")
        rows = 3
        holes = set()
        s_mid = max(1, strips // 2 - 1)
        holes.update({(1, s_mid, 1), (1, min(s_mid + 1, strips - 1), 1),
                      (1, s_mid, 2), (1, min(s_mid + 1, strips - 1), 2)})
        if strips >= 6:
            holes.add((-1, (s_mid + 2) % strips, 1))
            holes.add((-1, (s_mid + 2) % strips, 2))

        for sd in (1, -1):
            eave_x = xm + sd * half
            d = Vector((-sd * half, 0, rise)).normalized()
            n = Vector((sd * rise, 0, half)).normalized()
            for r_idx in range(rows):
                u0 = r_idx / rows
                u1 = (r_idx + 1) / rows
                xa = eave_x - sd * half * u0
                xb = eave_x - sd * half * u1
                za = z_eave + rise * u0
                zb = z_eave + rise * u1
                # Hip bevel bounds at row bottom (u0) and top (u1).
                ya0, ya1 = y0 + hip_in * u0, y1 - hip_in * u0
                yb0, yb1 = y0 + hip_in * u1, y1 - hip_in * u1
                for i in range(strips):
                    f0 = i / strips
                    f1 = (i + 1) / strips
                    p00 = (xa, ya0 + (ya1 - ya0) * f0, za)
                    p01 = (xa, ya0 + (ya1 - ya0) * f1, za)
                    p11 = (xb, yb0 + (yb1 - yb0) * f1, zb)
                    p10 = (xb, yb0 + (yb1 - yb0) * f0, zb)
                    if (sd, i, r_idx) in holes:
                        # Exposed charred horizontal roof battens spanning the hole.
                        mb.style(kind=3)
                        for b_off in (0.3, 0.7):
                            bx = xa + (xb - xa) * b_off
                            bz = za + (zb - za) * b_off
                            bya = p00[1] + (p10[1] - p00[1]) * b_off
                            byb = p01[1] + (p11[1] - p01[1]) * b_off
                            bmid = Vector((bx, (bya + byb) * 0.5, bz)) - n * 0.04
                            mb.oriented_box(bmid, d, Vector((0, 1, 0)), (0.09, max(0.2, byb - bya + 0.08), 0.07), "Wood")
                        continue
                    slope_poly([p00, p01, p11, p10])
            # Charred rafters running up any strip containing a blast hole.
            mb.style(kind=3)
            for i in range(strips):
                if not any((sd, i, r_idx) in holes for r_idx in range(rows)):
                    continue
                for kf, frac in ((0.25, 0.94), (0.72, 0.78)):
                    ft = (i + kf) / strips
                    y_bot = y0 + (y1 - y0) * ft
                    y_top = y_r0 + (y_r1 - y_r0) * ft
                    p_bot = Vector((eave_x, y_bot, z_eave)) - n * 0.08
                    p_top = Vector((xm, y_top, z_ridge)) - n * 0.08
                    p_end = p_bot + (p_top - p_bot) * frac
                    rd = p_end - p_bot
                    mb.oriented_box((p_bot + p_end) * 0.5, rd, Vector((0, 1, 0)), (rd.length, 0.11, 0.14), "Wood")

        # Keep both triangular hip ends intact on `Damaged`.
        slope_poly([(x0, y0, z_eave), (x1, y0, z_eave), (xm, y_r0, z_ridge)])
        slope_poly([(x1, y1, z_eave), (x0, y1, z_eave), (xm, y_r1, z_ridge)])
        ridge_caps(broken_mask={1})

    def cornice_band(self, x0, x1, y0, y1, z, h=0.22, proj=0.09, ruin=False):
        """Prompt 184: horizontal dressed stone cornice wrapping all four walls."""
        mb = self.walls
        mb.style(kind=1)
        if not ruin:
            mb.box(((x0 + x1) / 2, (y0 + y1) / 2, z), (x1 - x0 + 2 * proj, y1 - y0 + 2 * proj, h), "Stone")
            return
        # In Ruin only keep the cornice on the surviving -Y / -X half.
        ym = y0 + (y1 - y0) * 0.58
        mb.box(((x0 + x1) / 2, (y0 + ym) / 2, z), (x1 - x0 + 2 * proj, ym - y0 + proj, h), "Stone")

    def arcade_front(self, side, x0, x1, y0, y1, z_ground, z1, count, arch_w, spring, depth, t, ruin=False):
        """Prompt 184: open ground-floor round-arch arcade (Laubengang) along
        `side` ("+x" or "-x") with piers, impost caps, brick voussoir bands,
        and a recessed loggia with floor, back wall and end walls."""
        sgn = 1 if side == "+x" else -1
        x_front = x1 if sgn > 0 else x0
        x_back = x_front - sgn * depth
        span_len = y1 - y0
        count = max(1, int(count))
        # Divide remaining width: compact inner pier(s), sturdy outer corner piers.
        inner_pw = 1.20 if count >= 2 else 0.0
        rem = span_len - count * arch_w - (count - 1) * inner_pw
        if rem < 1.2:
            # Clamp arch width gracefully if building is narrower than requested.
            arch_w = max(1.6, (span_len - 1.4 - (count - 1) * inner_pw) / count)
            rem = span_len - count * arch_w - (count - 1) * inner_pw
        outer_pw = rem / 2
        r = arch_w / 2
        z_spring = z_ground + spring

        # Loggia floor slab inside the open gallery (in _Floor).
        self.floor.style(kind=1)
        loggia_x_mid = (x_front + x_back) / 2
        self.floor.box((loggia_x_mid, (y0 + y1) / 2, 0.07), (depth + 0.06, span_len, 0.14), "Stone")

        mb = self.walls
        # Compute arch intervals [lo, hi] along Y.
        arches = []
        cursor = y0 + outer_pw
        for i in range(count):
            arches.append((cursor, cursor + arch_w))
            cursor += arch_w + inner_pw

        # Build piers along the arcade face.
        piers = [(y0, arches[0][0])]
        for i in range(count - 1):
            piers.append((arches[i][1], arches[i + 1][0]))
        piers.append((arches[-1][1], y1))

        x_inner = x_front - sgn * t
        for idx, (pa, pb) in enumerate(piers):
            # In Ruin collapse the far +Y corner pier (leaving the -Y arch intact).
            pier_top = z_ground + (z1 - z_ground) * 0.34 if (ruin and idx == len(piers) - 1) else z1
            pw = pb - pa
            pm = (pa + pb) / 2
            mb.style(kind=self.wall_kind)
            mb.box((x_front - sgn * t / 2, pm, (z_ground + pier_top) / 2), (t, pw, pier_top - z_ground), self.wall_mat)
            # Stone/brick plinth base on each pier.
            mb.style(kind=3)
            mb.box((x_front - sgn * t / 2, pm, z_ground + 0.22), (t + 0.06, pw + 0.06, 0.44), "Stone")
            # Stone impost cap (Kämpferplatte) at spring height on intact piers.
            if not (ruin and idx == len(piers) - 1) and pier_top >= z_spring + 0.1:
                mb.style(kind=1)
                mb.box((x_front - sgn * t / 2, pm, z_spring - 0.06), (t + 0.09, pw + 0.12, 0.14), "Stone")

        # Round arch spandrels + proud brick voussoir bands (K = 6 segments per arch, like desert.py).
        K = 6
        for a_idx, (lo, hi) in enumerate(arches):
            # In Ruin collapse the +Y arch, keeping the -Y arch standing!
            if ruin and a_idx == count - 1:
                continue
            uc = (lo + hi) / 2
            for k in range(K):
                # Uniform angle sampling produces cleaner silhouettes near the springline.
                ang0 = math.pi * (1.0 - k / K)
                ang1 = math.pi * (1.0 - (k + 1) / K)
                ua = uc + r * math.cos(ang0)
                ub = uc + r * math.cos(ang1)
                za = z_spring + r * math.sin(ang0)
                zb = z_spring + r * math.sin(ang1)
                if z1 - min(za, zb) < 0.05:
                    continue
                # Wall spandrel above the arch curve up to z1.
                mb.style(kind=self.wall_kind)
                prof = [(ua, za), (ub, zb), (ub, z1), (ua, z1)]
                mb.extrude_x(prof, min(x_inner, x_front), max(x_inner, x_front), self.wall_mat)
                # Proud brick voussoir ring on the street face (reusing bridge band idea).
                band_h = 0.30
                vza = min(z1 - 0.02, za + band_h * (0.55 + 0.45 * math.sin(ang0)))
                vzb = min(z1 - 0.02, zb + band_h * (0.55 + 0.45 * math.sin(ang1)))
                mb.style(kind=3)
                vprof = [(ua, za), (ub, zb), (ub, vzb), (ua, vza)]
                if sgn > 0:
                    mb.extrude_x(vprof, x_front - 0.02, x_front + 0.045, "Stone")
                else:
                    mb.extrude_x(vprof, x_front - 0.045, x_front + 0.02, "Stone")

        # Recessed loggia back wall (with centered wooden door) and closed end walls.
        back_ops = [Opening((y0 + y1) / 2, 1.3, z_ground + 0.14, z_ground + 0.14 + DOOR_H, "door")]
        back_top = self.ruin_top(z_ground, z1, y0, y1) if ruin else None
        self.wall("x", x_back, sgn, y0 + t, y1 - t, z_ground, z1, back_ops, t, top=back_top)
        # End walls sealing the ±Y ends of the loggia gallery flush with the side facade.
        for ey, esgn in ((y0, -1), (y1, 1)):
            if ruin and esgn > 0:
                continue
            mb.style(kind=self.wall_kind)
            ex0 = min(x_back, x_front) - (t if sgn > 0 else 0.0)
            ex1 = max(x_back, x_front) + (t if sgn < 0 else 0.0)
            mb.box(((ex0 + ex1) / 2, ey - esgn * t / 2, (z_ground + z1) / 2),
                   (ex1 - ex0, t, z1 - z_ground), self.wall_mat)

    def flat_roof(self, x0, x1, y0, y1, z, kind_mat="Plaster", kind=1, beams=True, t=0.25):
        if self.roof is None:
            return
        mb = self.roof
        nx, ny = max(1, int((x1 - x0) / 1.5)), max(1, int((y1 - y0) / 1.5))
        holes = set()
        if self.damage == 1:
            holes = {(self.r.randrange(nx), self.r.randrange(ny)) for _ in range(2)}
        elif self.damage >= 2:
            holes = {(i, j) for i in range(nx) for j in range(ny) if self.r.random() < 0.8}
        mb.style(kind=kind)
        for i in range(nx):
            for j in range(ny):
                if (i, j) in holes:
                    continue
                xa, xb = x0 + (x1 - x0) * i / nx, x0 + (x1 - x0) * (i + 1) / nx
                ya, yb = y0 + (y1 - y0) * j / ny, y0 + (y1 - y0) * (j + 1) / ny
                mb.box(((xa + xb) / 2, (ya + yb) / 2, z - t / 2), (xb - xa, yb - ya, t), kind_mat)
        if beams:
            mb.style(kind=2)
            n = max(2, int((y1 - y0) / 0.9))
            for k in range(n):
                y = y0 + (y1 - y0) * (k + 0.5) / n
                if self.damage >= 2 and self.r.random() < 0.6:
                    continue
                mb.cylinder((x0 - 0.35, y, z - t - 0.08), (x1 + 0.35, y, z - t - 0.08), 0.09, seg=5, mat="Wood")

    # -- floors and debris ----------------------------------------------------------------

    def plinth(self, x0, x1, y0, y1, h, kind=0):
        self.floor.style(kind=kind)
        self.floor.box(((x0 + x1) / 2, (y0 + y1) / 2, h / 2), (x1 - x0 + 0.06, y1 - y0 + 0.06, h), "Stone")

    def slab(self, x0, x1, y0, y1, z, t=0.2, partial=None):
        self.floor.style(kind=0)
        if partial is None:
            self.floor.box(((x0 + x1) / 2, (y0 + y1) / 2, z + t / 2), (x1 - x0, y1 - y0, t), "Wood")
            return
        # ruin: only a part of the upper floor survives
        ya = y0 + (y1 - y0) * partial[0]
        yb = y0 + (y1 - y0) * partial[1]
        self.floor.box(((x0 + x1) / 2, (ya + yb) / 2, z + t / 2), (x1 - x0, yb - ya, t), "Wood")

    def rubble(self, x0, x1, y0, y1, piles, mats=("Stone",), height=1.0, size=1.0):
        """Rubble piles and loose beams with centers in x0..x1, y0..y1;
        piles stay inside the building box (prompt 240: richer brick/tile/beam heaps)."""
        if self.debris is None:
            return
        mb = self.debris
        r = self.r
        hw, hl = self.box.hw, self.box.hl
        use_roof_tiles = self.damage >= 2 and hasattr(self, "roof_kind")
        # Low broad central rubble mound in full ruins so collapsed houses read as rich heaps.
        if self.damage >= 2 and piles >= 2:
            cx_m, cy_m = (x0 + x1) * 0.5, (y0 + y1) * 0.5
            rx_m = max(0.4, min((x1 - x0) * 0.42, (hw - abs(cx_m)) / 1.25))
            ry_m = max(0.4, min((y1 - y0) * 0.42, (hl - abs(cy_m)) / 1.25))
            mb.style(kind=self.wall_kind if mats[0] == self.wall_mat else 0)
            mb.blob((cx_m, cy_m, 0.0), (rx_m, ry_m, height * 0.62), mats[0], seg=7, rings=3, rng=r,
                    rough=0.28, flat_bottom=0.0, smooth=False)
        for i in range(piles):
            cx, cy = r.uniform(x0, x1), r.uniform(y0, y1)
            rx, ry = r.uniform(0.9, 1.9) * size, r.uniform(0.9, 1.9) * size
            rx = max(0.2, min(rx, (hw - abs(cx)) / 1.35))
            ry = max(0.2, min(ry, (hl - abs(cy)) / 1.35))
            # Mix fallen roof-tile debris with masonry rubble in ruins.
            if use_roof_tiles and i % 2 == 1:
                mb.style(kind=self.roof_kind)
                mat_i = "Roof"
            else:
                mat_i = mats[i % len(mats)]
                mb.style(kind=self.wall_kind if mat_i == self.wall_mat else 0)
            mb.blob((cx, cy, 0.0), (rx, ry, height * r.uniform(0.55, 1.05)), mat_i, seg=6, rings=3, rng=r,
                    rough=0.32, flat_bottom=0.0, smooth=False)
        beam_count = piles * 3 if self.damage >= 2 else piles * 2
        for i in range(beam_count):
            cx, cy = r.uniform(x0, x1), r.uniform(y0, y1)
            a = r.uniform(0, math.pi)
            ln = r.uniform(1.6, 3.4)
            d = Vector((math.cos(a), math.sin(a), 0.0))
            room = min((hw - abs(cx)) / max(abs(d.x), 1e-3), (hl - abs(cy)) / max(abs(d.y), 1e-3)) * 2
            ln = max(0.4, min(ln, room - 0.25))
            tilt = r.uniform(0.05, 0.35) if (self.damage >= 2 and i % 3 == 0) else r.uniform(-0.1, 0.22)
            z_b = 0.28 + (0.28 if tilt > 0.2 else 0.0) if self.damage >= 2 else 0.15
            mb.style(kind=3)
            mb.oriented_box((cx, cy, z_b), (d.x, d.y, tilt), (0, 0, 1),
                            (ln, 0.15, 0.15), "Wood")

    def corner_quoins(self, x0, x1, y0, y1, z0, z1, ruin=False):
        """Prompt 219: alternating dressed stone corner quoins (Eckquader) on all
        four wall corners from z0 to z1."""
        mb = self.walls
        mb.style(kind=1)
        course = 0.48
        z_top = z0 + (z1 - z0) * 0.55 if ruin else z1
        n = max(1, int((z_top - z0) / course))
        ch = (z_top - z0) / n
        for k in range(n):
            za = z0 + k * ch
            # Alternate long/short arms along X and Y on odd/even courses.
            ax = 0.46 if k % 2 == 0 else 0.28
            ay = 0.28 if k % 2 == 0 else 0.46
            for sx, cx in ((-1, x0), (1, x1)):
                for sy, cy in ((-1, y0), (1, y1)):
                    mb.box((cx - sx * (ax / 2 - 0.03), cy - sy * (ay / 2 - 0.03), za + ch / 2),
                           (ax + 0.04, ay + 0.04, ch - 0.015), "Stone")

    def add_gable_trim(self, x0, x1, y0, y1, z_eave, z_ridge, style=True):
        """Prompt 219: clean verge trim (Ortgang) along both gable ends and
        eave fascia along the long walls."""
        if self.roof is None or self.damage >= 2:
            return
        mb = self.roof
        mat = "Wood" if style == "wood" else "Stone"
        kind = 2 if style == "wood" else 1
        mb.style(kind=kind)
        xm = (x0 + x1) / 2
        half = (x1 - x0) / 2
        rise = z_ridge - z_eave
        slope = math.hypot(half, rise)
        for y, sgn_y in ((y0, -1), (y1, 1)):
            for sd in (1, -1):
                d = Vector((-sd * half, 0, rise)).normalized()
                mid = Vector((xm + sd * half / 2, y + sgn_y * 0.04, z_eave + rise / 2 - 0.02))
                mb.oriented_box(mid, d, Vector((0, 1, 0)), (slope + 0.14, 0.14, 0.22), mat)
        # Subtle eave fascia along both long sides.
        for x, sd in ((x0, -1), (x1, 1)):
            mb.box((x + sd * 0.03, (y0 + y1) / 2, z_eave - 0.04), (0.08, y1 - y0 + 0.06, 0.16), mat)

    def add_dormers(self, x0, x1, y0, y1, z_wall_top, z_ridge, eave, roof_kind, count=1):
        """Prompt 219: small gabled roof dormer(s) on the front (+x) roof slope."""
        if self.damage >= 1 or self.roof is None or count <= 0:
            return
        xm = (x0 + x1) / 2
        half_wall = (x1 - x0) / 2
        rise = z_ridge - z_wall_top
        slope = rise / max(half_wall, 0.1)
        dw = 1.25
        dh = 1.15
        # Sit partway up the front (+x) slope so the dormer fits cleanly under the ridge.
        x_front = x1 - 0.35
        z_base = z_wall_top + (x1 - x_front) * slope
        z_top = z_base + dh
        x_back = max(xm + 0.15, x1 - (z_top - z_wall_top) / max(slope, 0.2))
        if z_top + 0.45 >= z_ridge or x_front - x_back < 0.4:
            return
        ys = _spread(y0 + 1.2, y1 - 1.2, count, dw)
        for dy in ys:
            # Cheek walls and front face.
            self.walls.style(kind=self.wall_kind)
            depth = x_front - x_back
            self.walls.box(((x_front + x_back) / 2, dy, (z_base + z_top) / 2),
                           (depth, dw, dh), self.wall_mat)
            # Mini gable triangle above the front face.
            d_ridge = z_top + dw * 0.38
            tri = [(x_front - depth, dy - dw / 2, z_top), (x_front - depth, dy + dw / 2, z_top),
                   (x_front - depth, dy, d_ridge)]
            self.walls.extrude(tri, (depth, 0, 0), self.wall_mat)
            # Crisp sash window quad on front (+X) face.
            wx = x_front + 0.02
            wlo, whi = dy - dw * 0.32, dy + dw * 0.32
            wz0, wz1 = z_base + 0.14, z_top - 0.10
            _sash_quad(self.walls, [(wx, wlo, wz1), (wx, whi, wz1), (wx, whi, wz0), (wx, wlo, wz0)], (1, 0, 0))
            # Pitched mini roof planes matching the house roof_kind.
            self.roof.style(kind=roof_kind)
            r_half = dw / 2 + 0.14
            r_rise = d_ridge - z_top + 0.06
            r_slope = math.hypot(r_half, r_rise)
            x_rback = max(xm + 0.05, x1 - (d_ridge - z_wall_top) / max(slope, 0.2))
            r_len = (x_front + 0.16) - x_rback
            xc = (x_front + 0.16 + x_rback) / 2
            for sd in (1, -1):
                d = Vector((0, -sd * r_half, r_rise)).normalized()
                n = Vector((0, sd * r_rise, r_half)).normalized()
                mid = Vector((xc, dy + sd * r_half / 2, z_top + r_rise / 2)) + n * 0.04
                self.roof.oriented_box(mid, d, Vector((1, 0, 0)), (r_slope, r_len, 0.10), "Roof")

    def finish(self):
        if self.debris is not None and not self.debris.bm.faces:
            self.model.drop(f"{self.name}_Debris")
        if self.roof is not None and not self.roof.bm.faces:
            self.model.drop(f"{self.name}_Roof")


def _spread(u0, u1, n, w):
    """n opening centers evenly spread between u0 and u1."""
    if n <= 0:
        return []
    step = (u1 - u0) / n
    return [u0 + step * (i + 0.5) for i in range(n)]


def _blast(house, openings, count):
    """Damaged: turn some windows into larger shell holes."""
    wins = [o for o in openings if o.kind == "window"]
    house.r.shuffle(wins)
    for o in wins[:count]:
        o.kind = "hole"
        o.w = min(o.w * house.r.uniform(1.6, 2.2), 2.4)
        o.z0 -= house.r.uniform(0.2, 0.6)
        o.z1 += house.r.uniform(0.2, 0.5)


@env_part("house")
def house(model, name, p, box):
    """Gabled, hipped or flat-roofed house, one variant = one damage level.

    Parameters: `seed`; `damage` (0 intact, 1 damaged, 2 ruin); `plinth`
    (m, default 0.4); `storeys` (full storeys, default 1); `storey_h`
    (default 2.8); `storey_hs` (optional per-storey height list, e.g.
    `[3.95, 3.15]`); `knee` (attic knee wall above the top floor, default 0);
    `wall` ("Plaster", "Stone" or "Wood", default "Plaster"); `wall_kind`;
    `wall_t` (default 0.4); `frame` (half-timbering, default false);
    `roof` ("gable", "hip" or "flat"); `roof_kind` (Roof kind, default 0 clay tile);
    `eave` (overhang at eaves, default 0.45 gable / 0.35 hip); `verge` (overhang
    at gables, default 0.3); `windows_long` / `windows_short` per storey
    (default 4 / 2); `windows` (optional per-side dict `{"+x": 5, "-x": 2, ...}`);
    `double_mid` (optional list of sides whose center window is a double window);
    `window` ([w, h], default [0.9, 1.2]); `window_frame` (dressed stone frames,
    default false); `window_arch` (segmental brick arch lintels, default false);
    `cornice` (wrapping stone cornice bands at storey lines & eaves, default
    false); `arcade` (optional `[side, count, width, spring]` ground-floor open
    round-arch loggia on `+x` or `-x`); `arcade_depth` (loggia depth in m,
    default 3.0); `door` (["+x"|"-x"|"+y"|"-y", width], default ["-x", 1.0],
    or `false`/omitted when `arcade` provides the entry); `barn_doors`
    (barn door [w, h] on both long walls, default none); `parapet` (flat
    roof, default 0.5); `chimney` (default false); `attic_windows`
    (windows per gable in the attic, default 1). Prompt 120: `shutters`
    (open shutters beside the windows, default false); `shop` ([w, h]: a
    shop front instead of the door's neighbours on the ground floor of the
    door side, default none); `outside_stair` (masonry stair along the door
    side up to a door on the first floor, default false); `ridge_along_x`
    is not supported (the ridge runs along Y). Prompt 202: `door_gable`
    (width of a pointed gable over the door in a long wall, running back
    into the roof; thatched houses of the coast; default none).
    """
    wall_mat = p.get("wall", "Plaster")
    h = House(model, name, p, box, wall_mat, p.get("wall_kind", 0))
    if wall_mat == "Wood":
        h.frame_mat = "Wood"
    H = box.height
    plinth = p.get("plinth", 0.4)
    storey_hs = p.get("storey_hs")
    if storey_hs:
        heights = [float(v) for v in storey_hs]
        storeys = len(heights)
        sh = heights[0]
    else:
        storeys = p.get("storeys", 1)
        sh = p.get("storey_h", 2.8)
        heights = [sh] * storeys
    knee = p.get("knee", 0.0)
    t = p.get("wall_t", 0.4)
    roof = p.get("roof", "gable")
    pitched = roof in ("gable", "hip")
    stepped = int(p.get("stepped_gable", 0))
    gable_chimneys = bool(p.get("gable_chimneys", False))
    eave = (p.get("eave", 0.35) if roof == "hip"
            else p.get("eave", 0.30 if stepped else 0.45) if roof == "gable"
            else 0.0)
    verge = (eave if roof == "hip"
             else 0.0 if stepped
             else (p.get("verge", 0.3) if roof == "gable" else 0.0))
    x0, x1 = -box.hw + eave, box.hw - eave
    y0, y1 = -box.hl + verge, box.hl - verge
    ww, wh = p.get("window", [0.9, 1.2])
    arcade = p.get("arcade")
    arcade_side = arcade[0] if arcade else None
    arcade_depth = float(p.get("arcade_depth", 3.0))
    door_cfg = p.get("door", False if arcade else ["-x", 1.0])
    door_side, door_w = (door_cfg[0], door_cfg[1]) if door_cfg else (None, 0.0)
    win_per_side = p.get("windows") or {}
    double_mid = set(p.get("double_mid", []))
    barn = p.get("barn_doors")
    frame = p.get("frame", False)
    shop = p.get("shop")
    ruin = h.damage >= 2
    # Prompt 120: outside stair along the door side, landing near +Y.
    stair_u = None
    if p.get("outside_stair", False) and storeys >= 2:
        stair_u = y1 - t - 0.9
    z_wall_top = plinth + sum(heights) + knee if pitched else H
    if arcade_side == "+x":
        h.plinth(x0, x1 - arcade_depth, y0, y1, plinth, kind=int(p.get("plinth_kind", 0)))
    elif arcade_side == "-x":
        h.plinth(x0 + arcade_depth, x1, y0, y1, plinth, kind=int(p.get("plinth_kind", 0)))
    else:
        h.plinth(x0, x1, y0, y1, plinth, kind=int(p.get("plinth_kind", 0)))
    door_u_ground = None
    all_ops = []
    z_cur = plinth
    for s in range(storeys):
        s_h = heights[s]
        z0 = z_cur
        z1 = z0 + s_h if s < storeys - 1 or pitched else H
        if s == storeys - 1 and pitched:
            z1 = z_wall_top
        z_cur += s_h
        ops = {}
        sill = z0 + p.get("sill", 0.9)
        for side, (u0, u1), n_def in (("+x", (y0, y1), p.get("windows_long", 4)),
                                      ("-x", (y0, y1), p.get("windows_long", 4)),
                                      ("+y", (x0, x1), p.get("windows_short", 2)),
                                      ("-y", (x0, x1), p.get("windows_short", 2))):
            n = int(win_per_side.get(side, n_def))
            # On ground floor with an arcade, side windows shift behind the open loggia.
            span_u0, span_u1 = u0 + t, u1 - t
            if s == 0 and arcade_side == "+x" and side in ("+y", "-y"):
                span_u1 = min(span_u1, x1 - arcade_depth - 0.3)
            elif s == 0 and arcade_side == "-x" and side in ("+y", "-y"):
                span_u0 = max(span_u0, x0 + arcade_depth + 0.3)
            if s == 0 and side == arcade_side:
                ops[side] = []
                continue
            us = _spread(span_u0, span_u1, n, ww)
            lst = []
            door_u = None
            if s == 0 and side == door_side:
                door_u = (u0 + u1) / 2 if not us else us[len(us) // 2]
                door_u_ground = door_u
                lst.append(Opening(door_u, door_w, z0 + 0.0, z0 + DOOR_H, "door"))
                if shop:
                    # The shop front takes the larger side of the door.
                    sw, shh = shop
                    left = door_u - (u0 + t) > (u1 - t) - door_u
                    su = door_u - (door_w / 2 + 0.4 + sw / 2) if left else door_u + (door_w / 2 + 0.4 + sw / 2)
                    lst.append(Opening(su, sw, z0, z0 + shh, "shop"))
                    us = [u for u in us if abs(u - su) > (sw + ww) / 2 + 0.2]
            if s == 1 and side == door_side and stair_u is not None:
                lst.append(Opening(stair_u, 0.9, z0, z0 + DOOR_H, "door"))
                us = [u for u in us if abs(u - stair_u) > (0.9 + ww) / 2 + 0.2]
            if s == 0 and barn and side in ("+x", "-x"):
                bw, bh = barn
                lst = [Opening((u0 + u1) / 2, bw, z0, z0 + bh, "barn_door")]
                us = [u for u in (u0 + (u1 - u0) * 0.15, u1 - (u1 - u0) * 0.15)]
                for u in us:
                    lst.append(Opening(u, 0.6, z0 + 2.4, z0 + 3.0, "louver"))
                ops[side] = lst
                continue
            mid_idx = len(us) // 2 if (side in double_mid and len(us) % 2 == 1) else -1
            for idx_u, u in enumerate(us):
                if door_u is not None and abs(u - door_u) < (door_w + ww) / 2 + 0.2:
                    continue
                if idx_u == mid_idx:
                    lst.append(Opening(u, ww * 1.85, sill, min(sill + wh, z1 - 0.3), "double_window"))
                else:
                    lst.append(Opening(u, ww, sill, min(sill + wh, z1 - 0.3), "window"))
            ops[side] = lst
            all_ops += lst
        if h.damage == 1:
            _blast(h, [o for v in ops.values() for o in v], 2 if s == storeys - 1 else 1)
        if s == 0 and arcade_side in ("+x", "-x"):
            # Build the three enclosed ground-floor walls behind the arcade,
            # then the open round-arch arcade front + recessed loggia.
            gx0 = x0 + arcade_depth if arcade_side == "-x" else x0
            gx1 = x1 - arcade_depth if arcade_side == "+x" else x1
            tops = {}
            if ruin:
                for side_k in ("+x", "-x", "+y", "-y"):
                    u_a, u_b = (y0, y1) if side_k[1] == "x" else (gx0, gx1)
                    tops[side_k] = h.ruin_top(z0, z1, u_a - 1, u_b + 1)
            if arcade_side == "+x":
                h.wall("x", gx0, -1, y0, y1, z0, z1, ops.get("-x", []), t, tops.get("-x"), frame)
            else:
                h.wall("x", gx1, 1, y0, y1, z0, z1, ops.get("+x", []), t, tops.get("+x"), frame)
            h.wall("y", y1, 1, gx0 + t, gx1 - t, z0, z1, ops.get("+y", []), t, tops.get("+y"), frame)
            h.wall("y", y0, -1, gx0 + t, gx1 - t, z0, z1, ops.get("-y", []), t, tops.get("-y"), frame)
            h.arcade_front(arcade_side, x0, x1, y0, y1, 0.0, z1, arcade[1], arcade[2], arcade[3],
                           arcade_depth, t, ruin=ruin)
        else:
            h.rect_walls(x0, x1, y0, y1, z0, z1, ops, t, frame=frame, ruin=(True if ruin else None))
        if s > 0:
            h.slab(x0 + t, x1 - t, y0 + t, y1 - t, z0, partial=(0.0, 0.45) if ruin else None)
            if h.cornice:
                h.cornice_band(x0, x1, y0, y1, z0, ruin=ruin)
    if h.cornice:
        h.cornice_band(x0, x1, y0, y1, z_wall_top - 0.14, h=0.24, proj=0.11, ruin=ruin)
    if h.quoins:
        h.corner_quoins(x0, x1, y0, y1, plinth, z_wall_top, ruin=ruin)
    if storeys and pitched and (knee > 0 or storeys >= 1):
        top_floor = plinth + sum(heights)
        if knee > 0:
            h.slab(x0 + t, x1 - t, y0 + t, y1 - t, top_floor - 0.0, partial=(0.3, 0.7) if ruin else None)
    if arcade_side == "+x":
        h.slab(x0 + t, x1 - arcade_depth, y0 + t, y1 - t, plinth - 0.02, t=0.05)
    elif arcade_side == "-x":
        h.slab(x0 + arcade_depth, x1 - t, y0 + t, y1 - t, plinth - 0.02, t=0.05)
    else:
        h.slab(x0 + t, x1 - t, y0 + t, y1 - t, plinth - 0.02, t=0.05)
    if roof == "hip":
        z_ridge = H - 0.14
        rise = z_ridge - z_wall_top
        z_eave = z_wall_top - (rise / max(box.hw - eave, 0.5)) * eave
        h.roof_kind = p.get("roof_kind", 0)
        h.hip_roof(-box.hw, box.hw, -box.hl, box.hl, z_eave, z_ridge, h.roof_kind,
                   strips=max(5, int(box.length / 2.4)))
    elif roof == "gable":
        # Reserve top headroom when stepped gables carry twin gable-end chimneys
        # so the finished geometry stays strictly <= box height H (fit=false safe).
        chimney_head = 0.95 if gable_chimneys else 0.0
        gable_over = 0.36 if stepped else 0.0
        z_ridge = H - 0.14 - chimney_head - gable_over
        rise = z_ridge - z_wall_top
        half = box.hw - 0.0
        # roof planes run out to the eaves: the slope continues past the wall top
        z_eave = z_wall_top - (rise / (box.hw - eave)) * eave
        z_gable_top = z_ridge + gable_over if stepped else z_ridge - 0.1
        for side, y in (("+y", y1), ("-y", y0)):
            if stepped:
                h.stepped_gable("y", y, 1 if side == "+y" else -1, x0, x1, z_wall_top, z_gable_top, t,
                                steps=stepped, z_ground=plinth)
            else:
                h.gable("y", y, 1 if side == "+y" else -1, x0, x1, z_wall_top, z_ridge - 0.1, t, frame=frame)
            # attic windows in the gable
            n = p.get("attic_windows", 1)
            if not ruin and n:
                for u in _spread(x0 + 1.0, x1 - 1.0, n, ww):
                    zz = z_wall_top + 0.4
                    if zz + wh * 0.8 < z_wall_top + rise * 0.6:
                        h.walls.style(kind=0)
                        c = y + (0.03 if side == "+y" else -0.03)
                        if h.sash_windows:
                            aw, ah = ww * 0.8, wh * 0.8
                            pts = [(u - aw / 2, c, zz + ah), (u + aw / 2, c, zz + ah),
                                   (u + aw / 2, c, zz), (u - aw / 2, c, zz)]
                            _sash_quad(h.walls, pts, (0, 1 if side == "+y" else -1, 0),
                                       103.0 if h.cross_windows else 102.0)
                        else:
                            h.walls.box((u, c, zz + wh * 0.4), (ww * 0.8, 0.02, wh * 0.8), "Glass")
        h.roof_kind = p.get("roof_kind", 0)
        ry0 = y0 + t * 0.45 if stepped else -box.hl
        ry1 = y1 - t * 0.45 if stepped else box.hl
        h.gable_roof(-box.hw, box.hw, ry0, ry1, z_eave, z_ridge, h.roof_kind,
                     strips=max(4, int(box.length / 1.8)))
        if h.gable_trim and not stepped:
            h.add_gable_trim(-box.hw, box.hw, -box.hl, box.hl, z_eave, z_ridge, h.gable_trim)
        if h.eave_trim and not ruin and h.roof is not None:
            # Crisp light stone/concrete eave fascia along both long sides (prompt 224).
            h.roof.style(kind=1)
            for x, sd in ((-box.hw, -1), (box.hw, 1)):
                h.roof.box((x + sd * 0.02, (ry0 + ry1) / 2, z_eave - 0.03), (0.09, ry1 - ry0 + 0.04, 0.16), "Stone")
        if gable_chimneys and not ruin and h.roof is not None:
            # Twin masonry chimneys perched atop both gable peaks (strictly <= H).
            ck = 3 if wall_mat != "Stone" else p.get("wall_kind", 3)
            cx = (x0 + x1) / 2
            ch_top = H - 0.24
            ch_base = z_gable_top - 0.35
            ch_h = max(0.6, ch_top - ch_base)
            for idx, cy in enumerate((y0 + t * 0.5, y1 - t * 0.5)):
                if h.damage == 1 and idx == 1:
                    # Half-broken rear chimney on damaged variant.
                    h.roof.style(kind=ck)
                    h.roof.box((cx, cy, ch_base + ch_h * 0.3), (0.58, max(t + 0.06, 0.52), ch_h * 0.6), "Stone")
                    continue
                h.roof.style(kind=ck)
                h.roof.box((cx, cy, ch_base + ch_h / 2), (0.62, max(t + 0.08, 0.56), ch_h), "Stone")
                h.roof.style(kind=1)
                h.roof.box((cx, cy, ch_top + 0.04), (0.74, max(t + 0.18, 0.66), 0.10), "Stone")
                h.roof.style(kind=3)
                for dx in (-0.16, 0.16):
                    h.roof.cylinder((cx + dx, cy, ch_top + 0.08), (cx + dx, cy, min(H - 0.02, ch_top + 0.22)),
                                    0.075, 0.065, seg=6, mat="Stone")
        if p.get("door_step", False) and door_u_ground is not None and door_side in ("+x", "-x"):
            sgn = 1 if door_side == "+x" else -1
            xw = x1 if sgn > 0 else x0
            step_d = min(0.42, max(0.18, box.hw - abs(xw) - 0.02))
            h.floor.style(kind=2)
            h.floor.box((xw + sgn * (step_d / 2), door_u_ground, min(0.24, plinth * 0.45) / 2),
                        (step_d, door_w + 0.45, min(0.24, plinth * 0.45)), "Stone")
        dormer_n = int(p.get("dormer", 0)) if not isinstance(p.get("dormer", 0), bool) else (1 if p.get("dormer") else 0)
        if dormer_n > 0:
            h.add_dormers(x0, x1, y0, y1, z_wall_top, z_ridge, eave, h.roof_kind, dormer_n)
        dg = p.get("door_gable")
        if dg and door_side in ("+x", "-x") and h.damage == 0 and h.roof is not None:
            # Prompt 202: a pointed gable over the door of a thatched house
            # (burning thatch slides off beside the door, not before it):
            # a wall triangle on the long wall and two small roof planes
            # running back into the main roof.
            us = _spread(y0 + t, y1 - t, p.get("windows_long", 4), ww)
            du = (y0 + y1) / 2 if not us else us[len(us) // 2]
            sgn = 1 if door_side == "+x" else -1
            xw = x1 if sgn > 0 else x0
            slope = rise / (box.hw - eave)
            zt = min(z_wall_top + dg * 0.75, z_ridge - 0.5)
            xo = xw + sgn * 0.12                     # the gable wall stands a little proud
            xr = xw - sgn * (zt - z_wall_top) / slope  # where its ridge meets the main roof
            h.walls.style(kind=h.wall_kind)
            for a, b in (((xo, du - dg / 2, z_wall_top - 0.3), (xo, du + dg / 2, z_wall_top - 0.3)),):
                tri = [a, b, (xo, du, zt)]
                h.walls.face(h.walls.verts(tri), h.wall_mat)
                h.walls.face(h.walls.verts(tri[::-1]), h.wall_mat)
            h.roof.style(kind=h.roof_kind)
            for k in (-1, 1):
                pts = [(xo + sgn * 0.15, du + k * (dg / 2 + 0.15), z_wall_top - 0.42), (xo + sgn * 0.15, du, zt + 0.1),
                       (xr, du, zt + 0.1), (xw - sgn * 0.0, du + k * (dg / 2 + 0.15), z_wall_top - 0.3)]
                h.roof.face(h.roof.verts(pts), "Roof")
                h.roof.face(h.roof.verts(pts[::-1]), "Roof")
    else:
        par = p.get("parapet", 0.5)
        h.flat_roof(x0 + t, x1 - t, y0 + t, y1 - t, H - par, kind_mat=wall_mat, kind=p.get("wall_kind", 1))
    if p.get("chimney", False) and not ruin:
        ck = 3 if wall_mat != "Stone" else p.get("wall_kind", 0)
        h.roof.style(kind=ck)
        if p.get("chimney_cap", False):
            # Masonry ridge chimney near the gable end with stepped stone crown & pots
            # (kept strictly <= H so fit=false Damaged variants never exceed Intact height).
            cx = (x0 + x1) / 2
            cy = y1 - 1.25
            ch_top = H - 0.38
            h.roof.box((cx, cy, ch_top - 0.85), (0.62, 0.72, 1.7), "Stone")
            h.roof.style(kind=1)
            h.roof.box((cx, cy, ch_top + 0.04), (0.74, 0.84, 0.12), "Stone")
            h.roof.style(kind=3)
            for dy in (-0.18, 0.18):
                h.roof.cylinder((cx, cy + dy, ch_top + 0.08), (cx, cy + dy, ch_top + 0.34), 0.09, 0.075, seg=6, mat="Stone")
        else:
            top = H + 0.0 - 0.15
            h.roof.box((x0 + (x1 - x0) * 0.35, y0 + (y1 - y0) * 0.7, top - 0.9), (0.5, 0.6, 1.8 - 0.0), "Stone")
    if stair_u is not None:
        _outside_stair(h, door_side, x0, x1, stair_u, plinth + sh, box, ruin)
    if h.damage == 1:
        h.rubble(x1 + 0.05, box.hw - 0.2, y0, y1, 2, mats=(wall_mat if wall_mat != "Wood" else "Stone",), height=0.6,
                 size=0.5)
    elif ruin:
        h.rubble(x0 + 1, x1 - 1, y0 + 1, y1 - 1, 4, mats=(wall_mat if wall_mat != "Wood" else "Stone",),
                 height=1.4)
        h.rubble(x1 + 0.05, box.hw - 0.2, y0, y1, 2, mats=(wall_mat if wall_mat != "Wood" else "Stone",), height=0.8,
                 size=0.6)
    h.finish()
    model.part_props.setdefault(name, {})["door_height_m"] = DOOR_H


def _outside_stair(h, door_side, x0, x1, landing_u, z_top, box, ruin):
    """Prompt 120: a masonry stair (solid steps on a wall) along the door
    side (+x or -x), rising towards -Y to a landing at `landing_u` in front
    of the first-floor door. Only along the long walls; it stays inside the
    eaves overhang of the box."""
    if door_side not in ("+x", "-x"):
        return
    sgn = 1 if door_side == "+x" else -1
    face = x1 if sgn > 0 else x0
    depth = max(0.6, min(1.0, box.hw - abs(face) - 0.02))
    xc = face + sgn * depth / 2
    mb = h.floor
    mb.style(kind=0)
    landing = 1.1
    top = z_top * (0.55 if ruin else 1.0)
    mb.box((xc, landing_u, top / 2), (depth, landing, top), "Stone")
    rise = 0.18
    n = max(1, int(top / rise))
    run = 0.28
    for k in range(n):
        zk = top - (k + 1) * rise
        if zk <= 0.0:
            break
        u = landing_u - landing / 2 - (k + 0.5) * run
        mb.box((xc, u, zk / 2), (depth, run, zk), "Stone")
    if not ruin:
        # Parapet along the outer edge of the landing and the flight.
        h.walls.style(kind=h.wall_kind)
        u0 = landing_u - landing / 2 - n * run
        h.walls.box((face + sgn * (depth - 0.08), (u0 + landing_u + landing / 2) / 2, top + 0.45),
                    (0.16, landing_u + landing / 2 - u0, 0.9), h.wall_mat)


@env_part("church")
def church(model, name, p, box):
    """Village church: tower with spire at -Y, nave with tall windows, lower
    choir at +Y. Parameters: `seed`; `damage`; `tower` (tower side in m,
    default 5); `tower_h` (top of the tower walls, default 17); `choir`
    (choir length, default 3). Prompt 120: `campanile` (low pyramid roof
    and arched belfry instead of the spire, default false); `wall`
    ("Stone" or "Plaster", default "Stone"); `nave_top` (default 7.5) and
    `ridge` (default 13.5) of the nave. Prompt 202: `windows` (nave windows
    per side, default 4), `window` ([w, h], default [1.4, 3.4]),
    `window_sill` (above the plinth, default 2.2), `choir_top` and
    `choir_ridge` (default 6 and 9), `belfry` ([w, h] of the louvred
    openings, default [1.0, 1.8]), `crown` (an open crown of four flying
    ribs from corner pinnacles to a lantern with a short spire instead of
    the spire; a cathedral of the castle rock, default false)."""
    wall_mat = p.get("wall", "Stone")
    wall_kind = int(p.get("wall_kind", 1 if wall_mat == "Stone" else 0))
    h = House(model, name, p, box, wall_mat, wall_kind)
    H = box.height
    ruin = h.damage >= 2
    tw = p.get("tower", 5.0)
    th = p.get("tower_h", 17.0)
    choir = p.get("choir", 3.0)
    t = 0.8
    eave = 0.4
    yt0, yt1 = -box.hl, -box.hl + tw
    yn0, yn1 = yt1 - 0.3, box.hl - choir
    nx0, nx1 = -box.hw + eave, box.hw - eave
    plinth = 0.5
    nave_top = p.get("nave_top", 7.5)
    ridge = p.get("ridge", 13.5)
    campanile = p.get("campanile", False)
    crown = p.get("crown", False)
    h.plinth(nx0, nx1, yn0, yn1, plinth)
    nwin = p.get("windows", 4)
    ww, wh = p.get("window", [1.4, 3.4])
    wz = p.get("window_sill", 2.2)
    wins = {side: [Opening(u, ww, plinth + wz, plinth + wz + wh, "arch_window")
                   for u in _spread(yn0 + 1, yn1 - 1, nwin, ww)]
            for side in ("+x", "-x")}
    # the side door takes the place of the two middle windows (of four) on +x
    wins["+x"] = ([Opening((yn0 + yn1) / 2 + 2.5, 1.6, plinth, plinth + 3.0, "door")] + wins["+x"][:1] + wins["+x"][3:]
                  if nwin == 4 else
                  [Opening((yn0 + yn1) / 2, 1.8, plinth, plinth + 3.2, "door")]
                  + [o for o in wins["+x"] if abs(o.u - (yn0 + yn1) / 2) > ww / 2 + 1.2])
    if h.damage == 1:
        _blast(h, wins["-x"], 1)
    h.rect_walls(nx0, nx1, yn0, yn1, plinth, nave_top, wins, t, ruin=(True if ruin else None))
    h.gable("y", yn1, 1, nx0, nx1, nave_top, ridge - 0.1, t)
    z_nave_eave = nave_top - eave * (ridge - nave_top) / (box.hw - eave)
    h.gable_roof(-box.hw, box.hw, yn0, yn1 + 0.2, z_nave_eave, ridge, 1, strips=8)
    if h.gable_trim:
        h.add_gable_trim(-box.hw, box.hw, yn0, yn1 + 0.2, z_nave_eave, ridge, h.gable_trim)
    h.slab(nx0 + t, nx1 - t, yn0 + t, yn1 - t, plinth - 0.02, t=0.05)
    # Prompt 219: stepped stone buttresses along nave flanks.
    buttresses = bool(p.get("buttresses", False))
    cornice = bool(p.get("cornice", False))
    if buttresses:
        h.walls.style(kind=h.wall_kind)
        nb = 3
        for i in range(nb):
            by = yn0 + (yn1 - yn0) * (i + 0.5) / nb
            bh_n = (nave_top - plinth) * (0.5 if ruin else 0.85)
            for sgn, bx in ((-1, nx0), (1, nx1)):
                h.walls.box((bx + sgn * 0.28, by, plinth + bh_n * 0.5), (0.56, 0.52, bh_n), "Stone")
                if not ruin:
                    h.walls.box((bx + sgn * 0.36, by, plinth + bh_n * 0.28), (0.72, 0.62, bh_n * 0.56), "Stone")
    # choir
    cx = (nx1 - nx0) * 0.33
    h.plinth(-cx, cx, yn1 - 0.2, box.hl - 0.2, plinth)
    ct, cr = p.get("choir_top", 6.0), p.get("choir_ridge", 9.0)
    h.rect_walls(-cx, cx, yn1 - 0.2, box.hl - 0.2, plinth, ct,
                 {"+y": [Opening(0, 1.0, plinth + 2.0, plinth + min(4.5, ct - 1.0), "arch_window")]}, 0.6,
                 ruin=(True if ruin else None))
    if not ruin:
        h.gable("y", box.hl - 0.2, 1, -cx, cx, ct, cr - 0.1, 0.6)
        z_choir_eave = ct - 0.3 * (cr - ct) / cx
        h.gable_roof(-cx - 0.3, cx + 0.3, yn1 - 0.2, box.hl, z_choir_eave, cr, 1, strips=2)
        if h.gable_trim:
            h.add_gable_trim(-cx - 0.3, cx + 0.3, yn1 - 0.2, box.hl, z_choir_eave, cr, h.gable_trim)
    # tower: walls, belfry openings, spire
    tx0, tx1 = -tw / 2, tw / 2
    tower = model.mb(f"{name}_Tower", parent=name)
    walls = h.walls
    h.walls = tower
    tower.style(kind=h.wall_kind)
    h.plinth(tx0, tx1, yt0, yt1, plinth)
    tops = th if not ruin else th * 0.55
    bw, bh = p.get("belfry", [1.0, 1.8])
    # u is x on the ±y walls and y on the ±x walls: the belfry openings sit in
    # the middle of the tower on every side (prompt 202: before, those of the
    # ±x walls sat at y = 0, in the nave, and stretched the tower wall there).
    ytc = (yt0 + yt1) / 2
    ops = {s: ([Opening(ytc if s[1] == "x" else 0.0, bw, tops - 1.2 - bh, tops - 1.2, "louver")] if not ruin else [])
           for s in ("+x", "-x", "+y", "-y")}
    if campanile and not ruin:
        # Two open arches per side in the belfry (u: x on the ±y walls, y
        # on the ±x walls).
        yc = (yt0 + yt1) / 2
        for side, uc in (("+x", yc), ("-x", yc), ("+y", 0.0), ("-y", 0.0)):
            ops[side] = [Opening(uc + d, 1.0, tops - 3.2, tops - 0.8, "hole") for d in (-tw / 4, tw / 4)]
    ops["-y"] = ops["-y"] + [Opening(0, 1.4, plinth, plinth + 2.6, "door")]
    # On the ±x walls u is y: the openings sit in the middle of the tower.
    # (Before prompt 202 the towers with a spire put them at y = 0, beyond
    # the tower, which stretched that wall into the nave.)
    u_x = (yt0 + yt1) / 2
    ops["+x"] = ops["+x"] + [Opening(u_x, 0.4, 6, 7.2, "window")]
    if h.damage == 1:
        ops["+x"].append(Opening(u_x - 0.8, 1.6, 9.0, 11.0, "hole"))
    h.rect_walls(tx0, tx1, yt0, yt1, plinth, tops, ops, 0.9, ruin=(True if ruin else None))
    if buttresses:
        # Stepped corner buttresses on the bell tower (Normandy church landmark).
        tower.style(kind=1)
        bh1 = (tops - plinth) * 0.42
        bh2 = (tops - plinth) * 0.78
        for sx, tx in ((-1, tx0), (1, tx1)):
            for sy, ty in ((-1, yt0), (1, yt1)):
                # Only outer corners (don't block the nave junction at +y).
                if sy > 0:
                    tower.box((tx + sx * 0.28, ty - 0.4, plinth + bh2 * 0.5), (0.56, 0.65, bh2), "Stone")
                else:
                    tower.box((tx + sx * 0.25, ty + sy * 0.25, plinth + bh2 * 0.5), (0.72, 0.72, bh2), "Stone")
                    tower.box((tx + sx * 0.36, ty + sy * 0.36, plinth + bh1 * 0.5), (0.94, 0.94, bh1), "Stone")
    if cornice and not ruin:
        # Stone string courses below the belfry and projecting eave cornice under the spire.
        tower.style(kind=1)
        yc = (yt0 + yt1) / 2
        tower.box((0, yc, tops - 1.2 - bh - 0.35), (tw + 0.32, tw + 0.32, 0.24), "Stone")
        tower.box((0, yc, tops - 0.14), (tw + 0.48, tw + 0.48, 0.28), "Stone")
    h.walls = walls
    if h.damage == 0 and campanile:
        # Low pyramid roof of tiles on a cornice.
        tower.style(kind=1)
        tower.box((0, (yt0 + yt1) / 2, th + 0.15), (tw + 0.4, tw + 0.4, 0.3), "Stone")
        tower.style(kind=0)
        base = tower.verts([Vector((tx0 - 0.3, yt0 - 0.3, th + 0.3)), Vector((tx1 + 0.3, yt0 - 0.3, th + 0.3)),
                            Vector((tx1 + 0.3, yt1 + 0.3, th + 0.3)), Vector((tx0 - 0.3, yt1 + 0.3, th + 0.3))])
        apex = tower.verts([(0, (yt0 + yt1) / 2, H)])[0]
        for i in range(4):
            tower.face([base[i], base[(i + 1) % 4], apex], "Roof")
        tower.face(list(reversed(base)), "Roof")
    elif h.damage == 0 and crown:
        # Open crown: parapet, corner pinnacles, four ribs rising to a
        # lantern, a short spire on it.
        yc = (yt0 + yt1) / 2
        tower.style(kind=1)
        tower.box((0, yc, th + 0.4), (tw + 0.4, tw + 0.4, 0.8), "Stone")
        lz = th + (H - th) * 0.55
        corners = [(tx0, yt0), (tx1, yt0), (tx1, yt1), (tx0, yt1)]
        for cx_, cy_ in corners:
            tower.box((cx_, cy_, th + 1.6), (0.7, 0.7, 3.2), "Stone")
            tower.cylinder((cx_, cy_, th + 3.2), (cx_, cy_, th + 5.0), 0.35, 0.02, seg=4, mat="Stone")
            pts = []
            for k in range(6):
                t = k / 5
                pts.append(Vector((cx_ * (1 - t) * 0.95, yc + (cy_ - yc) * (1 - t) * 0.95,
                                   th + 2.6 + (lz - th - 2.6) * math.sin(t * math.pi / 2))))
            for a_, b_ in zip(pts, pts[1:]):
                d_ = b_ - a_
                tower.oriented_box((a_ + b_) / 2, d_, (0, 0, 1) if abs(d_.normalized().z) < 0.9 else (1, 0, 0),
                                   (d_.length + 0.2, 0.45, 0.45), "Stone")
        tower.box((0, yc, lz + 0.9), (1.6, 1.6, 1.8), "Stone")
        tower.cylinder((0, yc, lz + 1.8), (0, yc, H), 0.75, 0.03, seg=8, mat="Stone")
    elif h.damage == 0:
        tower.style(kind=1)
        sp = [Vector((tx0 - 0.15, yt0 - 0.15, th)), Vector((tx1 + 0.15, yt0 - 0.15, th)),
              Vector((tx1 + 0.15, yt1 + 0.15, th)), Vector((tx0 - 0.15, yt1 + 0.15, th))]
        apex = tower.verts([(0, (yt0 + yt1) / 2, H - 0.6)])[0]
        base = tower.verts(sp)
        for i in range(4):
            tower.face([base[i], base[(i + 1) % 4], apex], "Roof")
        tower.face(list(reversed(base)), "Roof")
        tower.style(kind=0)
        tower.cylinder((0, (yt0 + yt1) / 2, H - 0.7), (0, (yt0 + yt1) / 2, H), 0.04, seg=4, mat="Metal")
        tower.box((0, (yt0 + yt1) / 2, H - 0.25), (0.5, 0.04, 0.04), "Metal")
    elif h.damage == 1:
        tower.style(kind=3)
        for i in range(4):
            a = i * math.pi / 2
            tower.oriented_box((math.cos(a) * 1.2, (yt0 + yt1) / 2 + math.sin(a) * 1.2, th + 1.4),
                               (-math.cos(a), -math.sin(a), 1.4), (0, 0, 1) if i % 2 else (1, 0, 0),
                               (3.6, 0.15, 0.15), "Wood")
    if h.damage:
        h.rubble(nx1 + 0.1, box.hw, yn0, yn1, 2, height=0.7)
        if ruin:
            h.rubble(nx0 + 1, nx1 - 1, yn0 + 1, yn1 - 1, 4, height=1.6)
            h.rubble(tx0 - 1, tx1 + 1, yt0 + 0.5, yt1, 2, height=1.8)
    h.finish()


@env_part("memorial_arch")
def memorial_arch(model, name, p, box):
    """Free-standing memorial arch (prompt 202): two massive piers joined by
    a round arch, an entablature and an attic over it; the passage runs
    along X (the long front faces +X and -X, the width of the monument is
    the box length along Y). Parameters: `span` (clear width between the
    piers, default 0.53 x length), `opening` (height of the arch opening,
    default 0.61 x height), `attic` (height of the attic, default 0.14 x
    height), `damage` (1: the attic shot away; 2: the piers stand as stumps,
    the arch has fallen)."""
    h = House(model, name, p, box, "Stone", 1)
    mb = h.walls
    L, W, H = box.length, box.width, box.height
    span = p.get("span", L * 0.53)
    op = p.get("opening", H * 0.61)
    attic = p.get("attic", H * 0.14)
    ruin = h.damage >= 2
    pw = (L - span) / 2 - 0.4          # pier width (the base and cornices project 0.4)
    d = W - 0.8
    r = span / 2
    spring = op - r
    body = H - attic
    mb.style(kind=1)
    for k in (-1, 1):
        yc = k * (span / 2 + pw / 2)
        top = body if not ruin else (op * 0.55 if k < 0 else op * 0.30)
        mb.box((0, yc, 0.6), (W, pw + 0.8, 1.2), "Stone", bevel=0.05)             # base
        mb.box((0, yc, (1.2 + top) / 2), (d, pw, top - 1.2), "Stone", bevel=0.04)
        if not ruin:
            mb.box((0, k * (span / 2 + 0.25), spring - 0.2), (d + 0.3, 0.5 + 0.3, 0.4), "Stone")   # impost
    if not ruin:
        n = 12
        for i in range(n):
            a = math.pi * (i + 0.5) / n
            c = Vector((0, math.cos(a) * (r + 0.55), spring + math.sin(a) * (r + 0.55)))
            mb.oriented_box(c, (0, -math.sin(a), math.cos(a)), (1, 0, 0),
                            (2 * (r + 1.1) * math.sin(math.pi / (2 * n)) + 0.06, d, 1.1), "Stone")
        crown = spring + r + 1.0
        mb.box((0, 0, (crown + body) / 2 - 0.2), (d, span + 0.2, body - crown + 0.5), "Stone")
        for k in (-1, 1):                                                           # spandrels
            z0s, z1s = spring + r * 0.45, crown + 0.2
            mb.box((0, k * (r * 0.85 + 0.3), (z0s + z1s) / 2), (d - 0.02, r * 0.6, z1s - z0s), "Stone")
        mb.box((0, 0, body - 0.35), (W, L, 0.7), "Stone", bevel=0.05)              # entablature
        if h.damage == 0:
            mb.box((0, 0, body + (attic - 0.4) / 2 + 0.35), (d, L - 1.0, attic - 0.7), "Stone", bevel=0.04)
            mb.box((0, 0, H - 0.2), (d + 0.5, L - 0.5, 0.4), "Stone", bevel=0.05)  # top cornice
        else:   # damaged: the attic is shot away but for a stump at one end
            mb.box((0, -L * 0.28, body + attic * 0.3 + 0.35), (d, L * 0.3, attic * 0.6), "Stone")
    h.floor.style(kind=1)
    h.floor.box((0, 0, 0.05), (W, L, 0.1), "Stone")
    if h.damage:
        h.rubble(-W / 2 + 0.3, W / 2 - 0.3, -span / 2, span / 2, 5 if ruin else 2, height=1.6 if ruin else 0.8)
    h.finish()


# -- bridges ----------------------------------------------------------------------


@env_part("bridge_wood")
def bridge_wood(model, name, p, box):
    """Single-lane timber beam bridge, span = box length (parametric through
    the variant size): pile bents at both ends, stringers, plank deck, wheel
    guards and railings. `deck` = deck top above the base (default 2.0)."""
    model.group(name)
    deck_z = p.get("deck", 2.0)
    L = box.length
    W = box.width
    rail_x = W / 2 - 0.1
    deck = model.mb(f"{name}_Deck", parent=name)
    piers = model.mb(f"{name}_Pier", parent=name)
    rails = model.mb(f"{name}_Rail", parent=name)
    piers.style(kind=2)
    for s in (-1, 1):
        y = s * (L / 2 - 0.4)
        for k in range(4):
            x = -W / 2 + 0.6 + (W - 1.2) * k / 3
            piers.cylinder((x, y, 0), (x, y, deck_z - 0.6), 0.15, seg=6, mat="Wood")
        piers.box((0, y, deck_z - 0.5), (W - 0.3, 0.35, 0.25), "Wood")
        piers.box((0, s * (L / 2 - 0.1), deck_z - 0.95), (W - 0.6, 0.2, 1.3), "Wood")
    deck.style(kind=0)
    for k in range(5):
        x = -W / 2 + 0.6 + (W - 1.2) * k / 4
        deck.box((x, 0, deck_z - 0.38 + 0.0), (0.2, L, 0.3), "Wood")
    deck.style(kind=0)
    n = int(L / 0.3)
    for i in range(n):
        y = -L / 2 + L * (i + 0.5) / n
        deck.style(kind=i % 2)
        deck.box((0, y, deck_z - 0.11 + 0.0), (W - 0.4, L / n * 0.94, 0.08), "Wood")
    deck.style(kind=0)
    for s in (-1, 1):
        deck.box((s * (W / 2 - 0.55), 0, deck_z + 0.07), (0.25, L, 0.15), "Wood")
    rails.style(kind=0)
    posts = max(2, int(L / 2.0) + 1)
    for s in (-1, 1):
        for i in range(posts):
            y = -L / 2 + 0.1 + (L - 0.2) * i / (posts - 1)
            rails.box((s * rail_x, y, deck_z + 0.5 - 0.05), (0.15, 0.15, 1.1), "Wood")
            if i < posts - 1:
                y2 = -L / 2 + 0.1 + (L - 0.2) * (i + 1) / (posts - 1)
                rails.oriented_box((s * rail_x, (y + y2) / 2, deck_z + 0.45), (0, y2 - y, 0.8), (1, 0, 0),
                                   (math.hypot(y2 - y, 0.8), 0.06, 0.1), "Wood")
        rails.box((s * rail_x, 0, box.height - 0.06), (0.18, L, 0.12), "Wood")
        rails.box((s * rail_x, 0, deck_z + 0.45), (0.1, L, 0.12), "Wood")
    model.part_props[name] = {"deck_height_m": deck_z, "span_m": round(L, 3), "deck_width_m": round(W - 0.4, 3)}


@env_part("bridge_stone")
def bridge_stone(model, name, p, box):
    """Masonry arch road bridge: `arches` (default 1), `rise` (arch rise as a
    fraction of the half span, default 0.85), `deck` (road surface above the
    base, default height - 1.0). Span per arch follows from the box length."""
    model.group(name)
    L, W, H = box.length, box.width, box.height
    deck_z = p.get("deck", H - 1.0)
    arches = p.get("arches", 1)
    abut = 2.0
    pier = 1.8 if arches > 1 else 0.0
    span = (L - 2 * abut - (arches - 1) * pier) / arches
    spring = 0.8
    rise = min(span / 2 * p.get("rise", 0.85), deck_z - spring - 0.8)
    body = model.mb(f"{name}_Deck", parent=name)
    par = model.mb(f"{name}_Rail", parent=name)
    xw = W / 2 - 0.45

    def bottom(y):
        u = y + L / 2
        if u < abut or u > L - abut:
            return 0.0
        u -= abut
        k = int(u // (span + pier))
        k = min(k, arches - 1)
        local = u - k * (span + pier)
        if local > span:
            return 0.0
        tt = (local - span / 2) / (span / 2)
        return spring + rise * math.sqrt(max(0.0, 1 - tt * tt))

    ys = [-L / 2, -L / 2 + abut]
    for k in range(arches):
        a0 = -L / 2 + abut + k * (span + pier)
        for i in range(1, 12):
            ys.append(a0 + span * i / 12)
        ys.append(a0 + span)
        if k < arches - 1:
            ys.append(a0 + span + pier)
    ys.append(L / 2)
    ys = sorted(set(round(y, 4) for y in ys))
    body.style(kind=1)
    for ya, yb in zip(ys, ys[1:]):
        za, zb = bottom(ya + 1e-4), bottom(yb - 1e-4)
        prof = [(ya, za), (yb, zb), (yb, deck_z), (ya, deck_z)]
        body.extrude_x(prof, -xw, xw, "Stone")
        # voussoir band on both faces
        if za > 0.01 or zb > 0.01:
            for s in (-1, 1):
                band = [(ya, za), (yb, zb), (yb, zb + 0.45), (ya, za + 0.45)]
                body.extrude_x(band, s * xw - 0.06 if s > 0 else -xw - 0.0,
                               s * xw + 0.0 if s > 0 else -xw + 0.06, "Stone")
    body.style(kind=0)
    body.box((0, 0, deck_z + 0.03), (2 * xw - 0.1, L, 0.06), "Ground")
    par.style(kind=1)
    for s in (-1, 1):
        par.box((s * (W / 2 - 0.225), 0, (deck_z + H - 0.15) / 2), (0.45, L, H - 0.15 - deck_z + 0.0), "Stone")
        par.box((s * (W / 2 - 0.225), 0, H - 0.075), (0.5 if False else 0.45, L, 0.15), "Stone")
    model.part_props[name] = {"deck_height_m": round(deck_z, 3), "span_m": round(span, 3), "arches": arches,
                              "deck_width_m": round(2 * xw, 3)}


# -- timber pier (prompt 122) ---------------------------------------------------------


def _rowing_boat(mb, cx, length=4.5, beam=1.4):
    """Small open rowing boat at x = `cx`, along Y, floating at z = 0:
    a lofted hull (pointed bow at +Y, narrower transom at -Y), the dark
    inside as a lower floor, two thwarts."""
    hl, hw = length / 2, beam / 2
    top, bottom = 0.38, -0.22
    rings = []
    for y, k in ((-hl, 0.62), (-hl * 0.6, 0.95), (0.0, 1.0), (hl * 0.55, 0.85), (hl * 0.92, 0.3), (hl, 0.04)):
        w = hw * k
        z0 = bottom + (0.12 if y > hl * 0.5 else 0.0)
        rings.append([(cx - w, y, top), (cx - w * 0.85, y, (top + z0) / 2), (cx - w * 0.35, y, z0),
                      (cx + w * 0.35, y, z0), (cx + w * 0.85, y, (top + z0) / 2), (cx + w, y, top)])
    mb.style(kind=2)
    mb.loft(rings, "Wood", smooth=False)
    mb.style(kind=0)
    mb.box((cx, -0.15, top + 0.005), (beam * 0.78, length * 0.7, 0.02), "Wood")
    for y in (-0.7, 0.6):
        mb.box((cx, y, top + 0.06), (beam * 0.9, 0.22, 0.05), "Wood")


@env_part("pier_wood")
def pier_wood(model, name, p, box):
    """Timber pier segment, length = box length along Y: a plank deck
    (`deck_width`, default 2.4 m) whose top lies at the box height above the
    water level (z = 0), on stringers and pairs of piles every 2.5 m that
    reach `embed` (default 2.5 m) below it. `boats`: two rowing boats moored
    at the sides (the box width then spans them)."""
    model.group(name)
    L = box.length
    W = p.get("deck_width", 2.4)
    deck_z = box.height
    embed = p.get("embed", 2.5)
    mb = model.mb(f"{name}_Deck", parent=name)
    piers = model.mb(f"{name}_Pier", parent=name)
    piers.style(kind=2)
    bents = max(2, int(round(L / 2.5)))
    for i in range(bents):
        y = -L / 2 + L * (i + 0.5) / bents
        for sx in (-1, 1):
            piers.cylinder((sx * (W / 2 - 0.12), y, -embed), (sx * (W / 2 - 0.12), y, deck_z - 0.12), 0.13,
                           seg=6, mat="Wood")
        piers.box((0, y, deck_z - 0.24), (W, 0.2, 0.16), "Wood")
    mb.style(kind=0)
    for x in (-W / 2 + 0.3, 0.0, W / 2 - 0.3):
        mb.box((x, 0, deck_z - 0.11), (0.16, L, 0.1), "Wood")
    n = int(L / 0.4)
    for i in range(n):
        y = -L / 2 + L * (i + 0.5) / n
        mb.style(kind=0 if i % 3 else 2)
        mb.box((0, y, deck_z - 0.03), (W, L / n * 0.92, 0.06), "Wood")
    if p.get("boats", False):
        boats = model.mb(f"{name}_Rail", parent=name)
        for sx in (-1, 1):
            _rowing_boat(boats, sx * (box.hw - 0.7))
    model.part_props[name] = {"env_embed_m": float(embed)}


# -- long steel bridges (prompt 115) --------------------------------------------------


def _beam(mb, a, b, w, h, mat="Metal"):
    """Box beam from point `a` to point `b` (x, y, z), `w` wide across (X)
    and `h` deep (in the plane of Y and Z)."""
    ax = Vector(b) - Vector(a)
    c = (Vector(a) + Vector(b)) / 2
    mb.oriented_box(c, ax, (1, 0, 0), (ax.length, w, h), mat)


def _steel_deck(model, name, L, deck_z, girder, rail_x, collapsed):
    """Roadway slab, footways, railings and girders of a steel bridge from
    y = -L/2 to L/2 (deck top at `deck_z`). Collapsed: the slab broke in
    the middle and both halves hang from their ends down into the river."""
    deck = model.mb(f"{name}_Deck", parent=name)
    rails = model.mb(f"{name}_Rail", parent=name)

    def lay(y0, z0, y1, z1):
        dy, dz = y1 - y0, z1 - z0
        c = ((y0 + y1) / 2, (z0 + z1) / 2)
        n = math.hypot(dy, dz)
        deck.style(kind=2)
        deck.oriented_box((0, c[0], c[1] - girder / 2), (0, dy, dz), (1, 0, 0),
                          (n, 2 * rail_x, girder), "Metal")
        deck.style(kind=0)
        deck.oriented_box((0, c[0], c[1] + 0.03), (0, dy, dz), (1, 0, 0), (n, 7.0, 0.06), "Ground")
        rails.style(kind=0)
        for s in (-1, 1):
            rails.oriented_box((s * rail_x, c[0], c[1] + 0.55), (0, dy, dz), (1, 0, 0),
                               (n, 0.12, 1.1), "Metal")
            rails.oriented_box((s * (rail_x - 1.25), c[0], c[1] + 0.12), (0, dy, dz), (1, 0, 0),
                               (n, 2.4, 0.24), "Stone")

    if not collapsed:
        lay(-L / 2, deck_z, L / 2, deck_z)
        return deck
    # Two halves, each torn off near the middle, hanging into the water.
    for s in (-1, 1):
        y_end = s * L / 2
        y_tip = s * L * 0.06
        z_tip = girder + 0.2
        # Fixed part at the pier, then the hanging part.
        y_hinge = s * L * 0.36
        lay(min(y_end, y_hinge), deck_z, max(y_end, y_hinge), deck_z)
        a, b = (y_hinge, deck_z), (y_tip, z_tip)
        if s < 0:
            lay(a[0], a[1], b[0], b[1])
        else:
            lay(b[0], b[1], a[0], a[1])
    return deck


@env_part("bridge_steel_arch")
def bridge_steel_arch(model, name, p, box):
    """Through steel arch with the roadway hung from it (prompt 115):
    concrete piers at both ends, two arch ribs over the outer edges, tie
    girders, hangers, wind bracing over the roadway, footways and
    railings. `deck` = road surface above the base (water), `collapsed`
    true: the arch and the roadway broke in the middle and lie in the
    river (same frame as the intact variant, build with fit = false)."""
    model.group(name)
    L, W, H = box.length, box.width, box.height
    deck_z = p.get("deck", 18.0)
    collapsed = p.get("collapsed", False)
    girder = 1.6
    rail_x = 6.0
    rib_x = W / 2 - 0.6
    pier_d = 6.0
    piers = model.mb(f"{name}_Pier", parent=name)
    piers.style(kind=2)
    for s in (-1, 1):
        y = s * (L / 2 - pier_d / 2)
        piers.box((0, y, (deck_z - girder) / 2), (W, pier_d, deck_z - girder), "Stone")
    deck = _steel_deck(model, name, L - 2 * pier_d + 0.2, deck_z, girder, rail_x, collapsed)
    # Arch ribs: parabola from the pier tops (y = +-(L/2 - pier_d)) up to H.
    y0 = L / 2 - pier_d
    rise = H - 0.9 - deck_z
    n = 16

    def arch(t):
        y = -y0 + 2 * y0 * t
        u = y / y0
        return y, deck_z + rise * (1 - u * u)

    pts = [arch(k / n) for k in range(n + 1)]
    deck.style(kind=0)
    keep = range(n) if not collapsed else [k for k in range(n) if k < 4 or k >= n - 4]
    for sx in (-1, 1):
        x = sx * rib_x
        for k in keep:
            (ya, za), (yb, zb) = pts[k], pts[k + 1]
            _beam(deck, (x, ya, za), (x, yb, zb), 1.2, 1.8)
        # Tie girder along the roadway edge.
        if not collapsed:
            _beam(deck, (x, -y0, deck_z - girder / 2), (x, y0, deck_z - girder / 2), 1.0, girder + 0.4)
        # Hangers.
        for k in range(1, n):
            if collapsed and 3 < k < n - 3:
                continue
            y, z = pts[k]
            if z - deck_z > 1.0:
                deck.box((x, y, (deck_z + z) / 2), (0.25, 0.25, z - deck_z), "Metal")
    # Wind bracing between the ribs where the arch is high above the road.
    for k in range(2, n - 1, 2):
        if collapsed and 3 < k < n - 3:
            continue
        y, z = pts[k]
        if z - deck_z > 6.0:
            deck.box((0, y, z), (2 * rib_x, 0.6, 0.8), "Metal")
    if collapsed:
        # The broken middle of the arch lies in the river.
        debris = model.mb(f"{name}_Debris", parent=name)
        debris.style(kind=1)
        for sx in (-1, 1):
            x = sx * rib_x
            ya, za = pts[4]
            _beam(debris, (x, ya, za), (x * 0.9, -L * 0.08, 0.9), 1.2, 1.8)
            yb, zb = pts[n - 4]
            _beam(debris, (x, yb, zb), (x * 0.9, L * 0.08, 0.9), 1.2, 1.8)
        for k, (dx, dy, rot) in enumerate([(-3, -6, 0.3), (2, 4, -0.4), (5, -1, 0.8), (-6, 8, 1.2)]):
            debris.oriented_box((dx, dy, 0.45), (math.cos(rot), math.sin(rot), 0), (0, 0, 1),
                                (9.0, 1.4, 0.9), "Metal")
    model.part_props[name] = {"deck_height_m": round(deck_z, 3), "span_m": round(2 * y0, 3),
                              "deck_width_m": round(2 * rail_x, 3), "collapsed": bool(collapsed)}


@env_part("bridge_steel_approach")
def bridge_steel_approach(model, name, p, box):
    """Approach span of a long bridge (prompt 115): plate girders on two
    concrete piers (one at each end, half a pier each, so chained spans
    share them), roadway, footways and railings. `deck` = road surface
    above the pier foot (default height - 1.1), `collapsed`: the girders
    broke in the middle and hang into the river or onto the street below
    (same frame, fit = false)."""
    model.group(name)
    L, W, H = box.length, box.width, box.height
    deck_z = p.get("deck", H - 1.1)
    collapsed = p.get("collapsed", False)
    girder = 1.2 + L / 30.0
    rail_x = 6.0
    piers = model.mb(f"{name}_Pier", parent=name)
    piers.style(kind=2)
    for s in (-1, 1):
        y = s * (L / 2 - 0.9)
        for x in (-4.0, 4.0):
            piers.box((x, y, (deck_z - girder) / 2), (2.4, 1.8, deck_z - girder), "Stone")
        piers.box((0, y, deck_z - girder - 0.6), (W, 1.8, 1.2), "Stone")
    deck = _steel_deck(model, name, L, deck_z, girder, rail_x, collapsed)
    # Edge girders a little wider than the roadway.
    if not collapsed:
        deck.style(kind=0)
        for sx in (-1, 1):
            deck.box((sx * (W / 2 - 0.3), 0, deck_z - girder / 2), (0.6, L, girder), "Metal")
    else:
        debris = model.mb(f"{name}_Debris", parent=name)
        debris.style(kind=1)
        for k, (dx, dy, rot) in enumerate([(-3, -2, 0.2), (3, 3, -0.5), (0, 0, 1.4)]):
            debris.oriented_box((dx, dy, 0.4), (math.cos(rot), math.sin(rot), 0), (0, 0, 1),
                                (min(8.0, L / 4), 1.2, 0.8), "Metal")
    model.part_props[name] = {"deck_height_m": round(deck_z, 3), "span_m": round(L, 3),
                              "deck_width_m": round(2 * rail_x, 3), "collapsed": bool(collapsed)}


@env_part("tunnel_portal")
def tunnel_portal(model, name, p, box):
    """Masonry tunnel portal with flared wing retaining walls (prompt 148).

    Oriented so `+Y` faces OUT of the mountain onto the forecourt and `-Y`
    embeds into the rock headwall. Variants: `Road`, `Rail`, `RoadCollapsed`,
    `RailCollapsed` (sealed with a blasted rock rubble cone).
    """
    model.group(name)
    L, W, H = box.length, box.width, box.height
    bore_w = float(p.get("bore_w", 6.0))
    bore_h = float(p.get("bore_h", 5.2))
    collapsed = bool(p.get("collapsed", False))
    import random
    r = random.Random(int(p.get("seed", 148)))

    wall = model.mb(f"{name}_Walls", parent=name)
    wall.style(kind=int(p.get("stone_kind", 2)))  # dressed grey stone masonry

    # Main headwall flanking the arch (embeds 2.5 m into the mountain behind Y=0).
    half_b = bore_w / 2.0
    pier_w = max(1.4, (W - bore_w) / 2.0)
    head_d = L * 0.55
    head_y = -head_d * 0.35
    spring_z = bore_h * 0.60
    arch_r = half_b
    crown_z = min(H - 0.9, spring_z + arch_r * 0.72)

    # Left & right masonry piers + recessed side wing retaining walls flaring forward (+Y).
    for sx in (-1.0, 1.0):
        px = sx * (half_b + pier_w / 2.0)
        wall.box((px, head_y, H / 2.0), (pier_w, head_d, H), "Stone")
        # Wing retaining wall sloping down from headwall onto the forecourt (+Y).
        wing_len = L * 0.48
        wing_x = sx * (half_b + pier_w * 0.55)
        wing_y = head_y + head_d / 2.0 + wing_len * 0.45
        wing_h = H * 0.62
        wall.box((wing_x, wing_y, wing_h / 2.0), (pier_w * 0.65, wing_len, wing_h), "Stone")
        # Low outer buttress step at the wing tip.
        tip_y = wing_y + wing_len * 0.45
        wall.box((wing_x + sx * 0.25, tip_y, wing_h * 0.32), (pier_w * 0.55, 1.6, wing_h * 0.64), "Stone")

    # Spandrel lintel above the arch crown + heavy projecting coping cornice.
    lintel_h = max(0.8, H - crown_z)
    wall.box((0.0, head_y, crown_z + lintel_h / 2.0), (W, head_d, lintel_h), "Stone")
    wall.box((0.0, head_y + 0.22, H - 0.20), (W + 0.55, head_d + 0.45, 0.40), "Stone")

    # Arch ring (voussoir segments + prominent keystone at the crown).
    K = 8
    ring_t = 0.62
    for k in range(K):
        a0 = math.pi * k / K
        a1 = math.pi * (k + 1) / K
        am = 0.5 * (a0 + a1)
        x0 = -half_b * math.cos(a0)
        x1 = -half_b * math.cos(a1)
        z0 = spring_z + (crown_z - spring_z) * math.sin(a0)
        z1 = spring_z + (crown_z - spring_z) * math.sin(a1)
        seg_l = math.hypot(x1 - x0, z1 - z0) + 0.16
        xm, zm = 0.5 * (x0 + x1), 0.5 * (z0 + z1)
        tx, tz = (x1 - x0) / max(seg_l, 0.01), (z1 - z0) / max(seg_l, 0.01)
        nx, nz = -tz, tx
        cx = xm + nx * (ring_t * 0.45)
        cz = zm + nz * (ring_t * 0.45)
        wall.oriented_box(
            (cx, head_y + 0.12, cz),
            (tx, 0.0, tz),
            (0.0, 1.0, 0.0),
            (seg_l, head_d + 0.28, ring_t),
            "Stone",
        )
    # Keystone protruding slightly at arch apex.
    wall.box((0.0, head_y + 0.26, crown_z + 0.26), (0.72, head_d + 0.42, 0.88), "Stone")

    # Dark subterranean bore backing plate inside the arch so the opening reads
    # as a deep black tunnel throat even against bright terrain behind it.
    if not collapsed:
        dark = model.mb(f"{name}_Floor", parent=name)
        dark.style(kind=3)  # dark soot/charred stone interior
        dark.box((0.0, head_y - head_d * 0.35, crown_z * 0.52), (bore_w + 0.4, 0.5, crown_z * 1.04), "Stone")
    else:
        # Blasted rock and masonry rubble cone completely sealing the arch.
        rubble = model.mb(f"{name}_Debris", parent=name)
        rubble.style(kind=2)
        # Central talus mound spilling out onto the forecourt.
        rubble.box((0.0, head_y + 0.5, crown_z * 0.48), (bore_w + 0.8, head_d * 0.95, crown_z * 0.96), "Stone")
        rubble.box((0.0, head_y + head_d * 0.45, crown_z * 0.30), (bore_w * 0.95, 3.0, crown_z * 0.60), "Stone")
        rubble.box((0.0, head_y + head_d * 0.78, crown_z * 0.14), (bore_w * 0.78, 2.6, crown_z * 0.28), "Stone")
        for _ in range(14):
            bx = r.uniform(-half_b * 0.95, half_b * 0.95)
            by = head_y + r.uniform(0.3, head_d * 0.9 + 1.8)
            bz = r.uniform(0.35, max(0.6, crown_z * (0.78 - (by - head_y) / (head_d + 3.5) * 0.65)))
            sx_b = r.uniform(0.8, 1.9)
            sy_b = r.uniform(0.7, 1.6)
            sz_b = r.uniform(0.55, 1.35)
            yaw = r.uniform(0.0, math.tau)
            rubble.oriented_box(
                (bx, by, bz),
                (math.cos(yaw), math.sin(yaw), 0.0),
                (0.0, 0.0, 1.0),
                (sx_b, sy_b, sz_b),
                "Stone",
            )

