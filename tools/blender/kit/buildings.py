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
            if o.kind == "window" and self.damage < 2:
                piece(o.lo, o.hi, o.z0, o.z1, "Glass", depth=0.03, inset=t * 0.35)
                fk = 2 if self.frame_mat == "Wood" else 0
                piece(o.u - 0.03, o.u + 0.03, o.z0, o.z1, "Wood", depth=0.06, inset=t * 0.3, kind=fk)
                piece(o.lo, o.hi, (o.z0 + o.z1) / 2 + 0.1, (o.z0 + o.z1) / 2 + 0.16, "Wood", depth=0.06,
                      inset=t * 0.3, kind=fk)
                piece(o.lo - 0.05, o.hi + 0.05, o.z0 - 0.06, o.z0, self.wall_mat, depth=t + 0.06, inset=-0.05)
                if self.shutters:
                    # Folded back against the wall, half the window wide each.
                    out = -(t / 2 + 0.03)
                    piece(o.lo - o.w / 2 - 0.04, o.lo - 0.04, o.z0, o.z1, "Wood", depth=0.04, inset=out, kind=0)
                    piece(o.hi + 0.04, o.hi + o.w / 2 + 0.04, o.z0, o.z1, "Wood", depth=0.04, inset=out, kind=0)
            elif o.kind == "shop" and self.damage < 2:
                # Prompt 120: shop front, glazing over a wooden base and a
                # fascia board above.
                piece(o.lo, o.hi, o.z0 + 0.6, o.z1, "Glass", depth=0.03, inset=t * 0.35)
                piece(o.lo, o.hi, o.z0, o.z0 + 0.6, "Wood", depth=0.06, inset=t * 0.4, kind=2)
                piece(o.lo - 0.1, o.hi + 0.1, o.z1, o.z1 + 0.35, "Wood", depth=0.06, inset=-0.05, kind=2)
            elif o.kind in ("door", "barn_door"):
                piece(o.lo, o.hi, o.z0, o.z1, "Wood", depth=0.06, inset=t * 0.6, kind=2)
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
        """Two roof planes from x0 / x1 (eaves) to the ridge at x = (x0+x1)/2."""
        if self.roof is None:
            return
        mb = self.roof
        xm = (x0 + x1) / 2
        half = (x1 - x0) / 2
        rise = z_ridge - z_eave
        slope = math.hypot(half, rise)
        holes = set()
        if self.damage == 1:
            s = self.r.randrange(0, max(1, strips - 2))
            holes = {(1, s), (1, min(s + 1, strips - 1))}
            if strips > 3:
                holes.add((-1, s + 1))
        elif self.damage >= 2:
            holes = {(sd, i) for sd in (1, -1) for i in range(strips)}
        sl = (y1 - y0) / strips
        for sd in (1, -1):
            d = Vector((-sd * half, 0, rise)).normalized()
            n = Vector((sd * rise, 0, half)).normalized()
            for i in range(strips):
                yc = y0 + sl * (i + 0.5)
                mid = Vector((xm + sd * half / 2, yc, z_eave + rise / 2)) - n * (slab / 2) + n * 0.0
                if (sd, i) in holes:
                    # rafters left in the hole (charred in a ruin)
                    if self.damage >= 2:
                        continue  # a ruin has no roof left; beams lie in the debris
                    mb.style(kind=3 if self.damage >= 2 else 0)
                    for k in (-0.3, 0.3):
                        mb.oriented_box(mid + Vector((0, k * sl, 0)) - n * 0.1, d, Vector((0, 1, 0)),
                                        (slope * 0.98, 0.12, 0.16), "Wood")
                    continue
                mb.style(kind=kind)
                mb.oriented_box(mid + n * (slab / 2), d, Vector((0, 1, 0)), (slope, sl * 1.001, slab), "Roof")
        if self.damage == 0:
            mb.style(kind=kind)
            mb.oriented_box((xm, (y0 + y1) / 2, z_ridge + 0.02), (0, 1, 0), (1, 0, 0), (y1 - y0, 0.3, 0.12), "Roof")
        return slope

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
        piles stay inside the building box."""
        if self.debris is None:
            return
        mb = self.debris
        r = self.r
        hw, hl = self.box.hw, self.box.hl
        for i in range(piles):
            cx, cy = r.uniform(x0, x1), r.uniform(y0, y1)
            rx, ry = r.uniform(0.8, 1.8) * size, r.uniform(0.8, 1.8) * size
            rx = max(0.2, min(rx, (hw - abs(cx)) / 1.35))
            ry = max(0.2, min(ry, (hl - abs(cy)) / 1.35))
            mb.style(kind=self.wall_kind if mats[0] == self.wall_mat else 0)
            mb.blob((cx, cy, 0.0), (rx, ry, height * r.uniform(0.5, 1.0)), mats[0], seg=7, rings=4, rng=r,
                    rough=0.3, flat_bottom=0.0, smooth=False)
        for i in range(piles * 2):
            cx, cy = r.uniform(x0, x1), r.uniform(y0, y1)
            a = r.uniform(0, math.pi)
            ln = r.uniform(1.5, 3.0)
            d = Vector((math.cos(a), math.sin(a), 0.0))
            room = min((hw - abs(cx)) / max(abs(d.x), 1e-3), (hl - abs(cy)) / max(abs(d.y), 1e-3)) * 2
            ln = max(0.4, min(ln, room - 0.2))
            mb.style(kind=3)
            mb.oriented_box((cx, cy, 0.15), (d.x, d.y, r.uniform(-0.1, 0.25)), (0, 0, 1),
                            (ln, 0.15, 0.15), "Wood")

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
    """Gabled (or flat-roofed) house, one variant = one damage level.

    Parameters: `seed`; `damage` (0 intact, 1 damaged, 2 ruin); `plinth`
    (m, default 0.4); `storeys` (full storeys, default 1); `storey_h`
    (default 2.8); `knee` (attic knee wall above the top floor, default 0);
    `wall` ("Plaster", "Stone" or "Wood", default "Plaster"); `wall_kind`;
    `wall_t` (default 0.4); `frame` (half-timbering, default false);
    `roof` ("gable" or "flat"); `roof_kind` (Roof kind, default 0 clay tile);
    `eave` (overhang at the long walls, default 0.45); `verge` (overhang at
    the gables, default 0.3); `windows_long` / `windows_short` per storey
    (default 4 / 2); `window` ([w, h], default [0.9, 1.2]); `door`
    (["+x"|"-x"|"+y"|"-y", width], default ["-x", 1.0]); `barn_doors`
    (barn door [w, h] on both long walls, default none); `parapet` (flat
    roof, default 0.5); `chimney` (default false); `attic_windows`
    (windows per gable in the attic, default 1). Prompt 120: `shutters`
    (open shutters beside the windows, default false); `shop` ([w, h]: a
    shop front instead of the door's neighbours on the ground floor of the
    door side, default none); `outside_stair` (masonry stair along the door
    side up to a door on the first floor, default false); `ridge_along_x`
    is not supported (the ridge runs along Y).
    """
    wall_mat = p.get("wall", "Plaster")
    h = House(model, name, p, box, wall_mat, p.get("wall_kind", 0))
    if wall_mat == "Wood":
        h.frame_mat = "Wood"
    H = box.height
    plinth = p.get("plinth", 0.4)
    storeys = p.get("storeys", 1)
    sh = p.get("storey_h", 2.8)
    knee = p.get("knee", 0.0)
    t = p.get("wall_t", 0.4)
    roof = p.get("roof", "gable")
    eave = p.get("eave", 0.45) if roof == "gable" else 0.0
    verge = p.get("verge", 0.3) if roof == "gable" else 0.0
    x0, x1 = -box.hw + eave, box.hw - eave
    y0, y1 = -box.hl + verge, box.hl - verge
    ww, wh = p.get("window", [0.9, 1.2])
    door_side, door_w = p.get("door", ["-x", 1.0])
    barn = p.get("barn_doors")
    frame = p.get("frame", False)
    shop = p.get("shop")
    ruin = h.damage >= 2
    # Prompt 120: outside stair along the door side, landing near +Y.
    stair_u = None
    if p.get("outside_stair", False) and storeys >= 2:
        stair_u = y1 - t - 0.9
    z_wall_top = plinth + storeys * sh + knee if roof == "gable" else H
    h.plinth(x0, x1, y0, y1, plinth)
    all_ops = []
    for s in range(storeys):
        z0 = plinth + s * sh
        z1 = z0 + sh if s < storeys - 1 or roof == "gable" else H
        if s == storeys - 1 and roof == "gable":
            z1 = z_wall_top
        ops = {}
        sill = z0 + p.get("sill", 0.9)
        for side, (u0, u1), n in (("+x", (y0, y1), p.get("windows_long", 4)),
                                  ("-x", (y0, y1), p.get("windows_long", 4)),
                                  ("+y", (x0, x1), p.get("windows_short", 2)),
                                  ("-y", (x0, x1), p.get("windows_short", 2))):
            us = _spread(u0 + t, u1 - t, n, ww)
            lst = []
            door_u = None
            if s == 0 and side == door_side:
                door_u = (u0 + u1) / 2 if not us else us[len(us) // 2]
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
            for u in us:
                if door_u is not None and abs(u - door_u) < (door_w + ww) / 2 + 0.2:
                    continue
                lst.append(Opening(u, ww, sill, min(sill + wh, z1 - 0.3), "window"))
            ops[side] = lst
            all_ops += lst
        if h.damage == 1:
            _blast(h, [o for v in ops.values() for o in v], 2 if s == storeys - 1 else 1)
        h.rect_walls(x0, x1, y0, y1, z0, z1, ops, t, frame=frame, ruin=(True if ruin else None))
        if s > 0:
            h.slab(x0 + t, x1 - t, y0 + t, y1 - t, z0, partial=(0.0, 0.45) if ruin else None)
    if storeys and roof == "gable" and (knee > 0 or storeys >= 1):
        top_floor = plinth + storeys * sh
        if knee > 0:
            h.slab(x0 + t, x1 - t, y0 + t, y1 - t, top_floor - 0.0, partial=(0.3, 0.7) if ruin else None)
    h.slab(x0 + t, x1 - t, y0 + t, y1 - t, plinth - 0.02, t=0.05)
    if roof == "gable":
        z_ridge = H - 0.14
        rise = z_ridge - z_wall_top
        half = box.hw - 0.0
        # roof planes run out to the eaves: the slope continues past the wall top
        z_eave = z_wall_top - rise * (eave / (box.hw - eave + 1e-6)) * (box.hw - eave) / max(half, 1e-6)
        z_eave = z_wall_top - (rise / (box.hw - eave)) * eave
        for side, y in (("+y", y1), ("-y", y0)):
            h.gable("y", y, 1 if side == "+y" else -1, x0, x1, z_wall_top, z_ridge - 0.1, t, frame=frame)
            # attic windows in the gable
            n = p.get("attic_windows", 1)
            if not ruin and n:
                for u in _spread(x0 + 1.0, x1 - 1.0, n, ww):
                    zz = z_wall_top + 0.4
                    if zz + wh * 0.8 < z_wall_top + rise * 0.6:
                        h.walls.style(kind=0)
                        c = y + (0.03 if side == "+y" else -0.03)
                        h.walls.box((u, c, zz + wh * 0.4), (ww * 0.8, 0.02, wh * 0.8), "Glass")
        h.roof_kind = p.get("roof_kind", 0)
        h.gable_roof(-box.hw, box.hw, -box.hl, box.hl, z_eave, z_ridge, h.roof_kind,
                     strips=max(4, int(box.length / 1.8)))
    else:
        par = p.get("parapet", 0.5)
        h.flat_roof(x0 + t, x1 - t, y0 + t, y1 - t, H - par, kind_mat=wall_mat, kind=p.get("wall_kind", 1))
    if p.get("chimney", False) and not ruin:
        h.roof.style(kind=3 if wall_mat != "Stone" else p.get("wall_kind", 0))
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
    `ridge` (default 13.5) of the nave."""
    h = House(model, name, p, box, p.get("wall", "Stone"), 1 if p.get("wall", "Stone") == "Stone" else 0)
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
    h.plinth(nx0, nx1, yn0, yn1, plinth)
    wins = {side: [Opening(u, 1.4, plinth + 2.2, plinth + 5.6, "arch_window") for u in _spread(yn0 + 1, yn1 - 1, 4, 1.4)]
            for side in ("+x", "-x")}
    wins["+x"] = [Opening((yn0 + yn1) / 2 + 2.5, 1.6, plinth, plinth + 3.0, "door")] + wins["+x"][:1] + wins["+x"][3:]
    if h.damage == 1:
        _blast(h, wins["-x"], 1)
    h.rect_walls(nx0, nx1, yn0, yn1, plinth, nave_top, wins, t, ruin=(True if ruin else None))
    h.gable("y", yn1, 1, nx0, nx1, nave_top, ridge - 0.1, t)
    h.gable_roof(-box.hw, box.hw, yn0, yn1 + 0.2, nave_top - eave * (ridge - nave_top) / (box.hw - eave),
                 ridge, 1, strips=8)
    h.slab(nx0 + t, nx1 - t, yn0 + t, yn1 - t, plinth - 0.02, t=0.05)
    # choir
    cx = (nx1 - nx0) * 0.33
    h.plinth(-cx, cx, yn1 - 0.2, box.hl - 0.2, plinth)
    h.rect_walls(-cx, cx, yn1 - 0.2, box.hl - 0.2, plinth, 6.0,
                 {"+y": [Opening(0, 1.0, plinth + 2.0, plinth + 4.5, "arch_window")]}, 0.6,
                 ruin=(True if ruin else None))
    if not ruin:
        h.gable_roof(-cx - 0.3, cx + 0.3, yn1 - 0.2, box.hl, 6.0 - 0.3 * 3.0 / cx, 9.0, 1, strips=2)
    # tower: walls, belfry openings, spire
    tx0, tx1 = -tw / 2, tw / 2
    tower = model.mb(f"{name}_Tower", parent=name)
    walls = h.walls
    h.walls = tower
    tower.style(kind=1)
    h.plinth(tx0, tx1, yt0, yt1, plinth)
    tops = th if not ruin else th * 0.55
    bell = [Opening(0, 1.0, tops - 3.0, tops - 1.2, "louver")] if not ruin else []
    ops = {s: list(bell) for s in ("+x", "-x", "+y", "-y")}
    if campanile and not ruin:
        # Two open arches per side in the belfry (u: x on the ±y walls, y
        # on the ±x walls).
        yc = (yt0 + yt1) / 2
        for side, uc in (("+x", yc), ("-x", yc), ("+y", 0.0), ("-y", 0.0)):
            ops[side] = [Opening(uc + d, 1.0, tops - 3.2, tops - 0.8, "hole") for d in (-tw / 4, tw / 4)]
    ops["-y"] = ops["-y"] + [Opening(0, 1.4, plinth, plinth + 2.6, "door")]
    # On the ±x walls u is y: the old towers put these openings at y = 0,
    # beyond the tower, which stretched that wall to the nave; the campanile
    # (prompt 120) centers them on the tower. The old churches keep theirs
    # (their models are not rebuilt).
    u_x = (yt0 + yt1) / 2 if campanile else 0.0
    ops["+x"] = ops["+x"] + [Opening(u_x, 0.4, 6, 7.2, "window")]
    if h.damage == 1:
        ops["+x"].append(Opening(u_x - 0.8, 1.6, 9.0, 11.0, "hole"))
    h.rect_walls(tx0, tx1, yt0, yt1, plinth, tops, ops, 0.9, ruin=(True if ruin else None))
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
