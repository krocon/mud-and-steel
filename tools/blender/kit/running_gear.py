"""Tracked running gear: road wheels, suspension, sprocket, idler, rollers, track.

Every running-gear part needs these context values from the assembly:
`track_outer_x` (half width to the outer track edge) and `hull_length`.
Y positions in the blueprint are model-space meters (front +Y, hull center 0).

Common parameters:
- `track_width`, `track_thickness`, `link_pitch` (m, scrolling texture)
- `road_wheel_diameter`, `road_wheels` (list of Y positions of the stations)
- `sprocket = { y, z, diameter, teeth }`, `idler = { y, z, diameter }`
- `return_rollers = { y = [...], z, diameter }` (optional)
- `twin_wheels` (two discs per station, true for leaf-spring and VVSS bogies)

Returns to the context: `track_inner_x`, `track_top_z`, `track_front_y`,
`track_rear_y`, `wheel_center_z`.
"""

import math

from mathutils import Vector

from . import wheels as W
from .registry import part

SIDES = ((-1, "L"), (1, "R"))


def _convex_hull(points):
    pts = sorted(set(points))
    if len(pts) < 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]  # counter-clockwise in (y, z)


def track_path(circles, thickness, max_seg=0.6, min_seg=0.05):
    """Center line of a track around circles [(y, z, r)], counter-clockwise."""
    pts = []
    for y, z, r in circles:
        rr = r + thickness / 2
        for i in range(32):
            a = 2 * math.pi * i / 32
            pts.append((round(y + math.cos(a) * rr, 5), round(z + math.sin(a) * rr, 5)))
    hull = _convex_hull(pts)
    # drop points that are too close, split long straight runs
    out = []
    for i, p in enumerate(hull):
        q = hull[(i + 1) % len(hull)]
        if out and math.dist(out[-1], p) < min_seg:
            continue
        out.append(p)
        d = math.dist(p, q)
        n = int(d // max_seg)
        for k in range(1, n + 1):
            t = k / (n + 1)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def track_belt(mb, path, x_center, width, thickness, pitch):
    """Closed track band along a path, with link UVs (v = links along the band)."""
    n = len(path)
    normals = []
    for i in range(n):
        a = Vector(path[i - 1])
        b = Vector(path[(i + 1) % n])
        t = (b - a).normalized()
        normals.append(Vector((t.y, -t.x)))  # outward for a counter-clockwise loop
    corners = []  # per point: outer-left, outer-right, inner-right, inner-left
    for (y, z), nrm in zip(path, normals):
        o = Vector((y, z)) + nrm * thickness / 2
        i_ = Vector((y, z)) - nrm * thickness / 2
        xl, xr = x_center - width / 2, x_center + width / 2
        corners.append(mb.verts([(xl, o.x, o.y), (xr, o.x, o.y), (xr, i_.x, i_.y), (xl, i_.x, i_.y)]))
    arc = [0.0]
    for i in range(1, n + 1):
        arc.append(arc[-1] + math.dist(path[i - 1], path[i % n]))
    # integer number of links so that the texture closes without a seam
    links = max(1, round(arc[-1] / pitch))
    scale = links / arc[-1]
    for i in range(n):
        j = (i + 1) % n
        a, b = corners[i], corners[j]
        va, vb = arc[i] * scale, arc[i + 1] * scale
        for k, (c0, c1, track_face, u0, u1) in enumerate((
            (0, 1, True, 0.0, 1.0),  # outer (ground) surface
            (1, 2, False, 0.0, 0.0),  # right edge
            (2, 3, True, 1.0, 0.0),  # inner surface (wheel path)
            (3, 0, False, 0.0, 0.0),  # left edge
        )):
            f = mb.face([a[c0], a[c1], b[c1], b[c0]], "Metal")
            if track_face:
                f[mb.track] = 1
                uvs = ((u0, va), (u1, va), (u1, vb), (u0, vb))
                for loop, uv in zip(f.loops, uvs):
                    loop[mb.uv].uv = uv
    # normals of a closed band: recalculation is reliable
    import bmesh

    faces = list({f for c in corners for v in c for f in v.link_faces})
    bmesh.ops.recalc_face_normals(mb.bm, faces=faces)


class Gear:
    """Shared values of one running gear, read from the blueprint."""

    def __init__(self, p, ctx):
        self.tw = p.req("track_width")
        self.tt = p.get("track_thickness", 0.035)
        self.pitch = p.get("link_pitch", 0.15)
        self.rw_r = p.req("road_wheel_diameter") / 2
        self.stations = p.req("road_wheels")
        self.rw_width = p.get("road_wheel_width", self.tw * 0.8)
        sp = p.req("sprocket")
        idl = p.req("idler")
        self.sp = (sp["y"], sp["z"], sp["diameter"] / 2, sp.get("teeth", 13))
        self.idl = (idl["y"], idl["z"], idl["diameter"] / 2, idl.get("spoked", True))
        rr = p.get("return_rollers")
        self.rollers = [] if not rr else [(y, rr["z"], rr["diameter"] / 2) for y in rr["y"]]
        self.outer = ctx["track_outer_x"]
        self.wheel_z = self.rw_r + self.tt
        self.ctx = ctx

    def x(self, side):
        return side * (self.outer - self.tw / 2)

    def build_common(self, model, side, s, wheel_fn, rollers_as_objects=True):
        x = self.x(side)
        for i, y in enumerate(self.stations, start=1):
            c = (x, y, self.wheel_z)
            mb = model.mb(f"Wheel_{s}_{i:02d}", parent="Hull", pivot=c)
            wheel_fn(mb, c, side)
        sy, sz, sr, teeth = self.sp
        mb = model.mb(f"Sprocket_{s}", parent="Hull", pivot=(x, sy, sz))
        W.sprocket(mb, (x, sy, sz), sr, self.tw, side, teeth=teeth)
        iy, iz, ir, spoked = self.idl
        mb = model.mb(f"Idler_{s}", parent="Hull", pivot=(x, iy, iz))
        if spoked:
            W.spoked_idler(mb, (x, iy, iz), ir, self.tw * 0.7, side)
        else:
            W.disc_wheel(mb, (x, iy, iz), ir, self.tw * 0.7, side, tire="Paint")
        for i, (ry, rz, rr) in enumerate(self.rollers, start=1):
            c = (x, ry, rz)
            if rollers_as_objects:
                mb = model.mb(f"Roller_{s}_{i:02d}", parent="Hull", pivot=c)
            else:
                mb = model.mb("Hull")
            W.return_roller(mb, c, rr, self.tw * 0.6, side)
            # roller bracket to the hull side
            model.mb("Hull").box((x - side * (self.tw * 0.3 + 0.06), ry, rz), (0.12, 0.08, 0.08), "Paint")
        circles = [(y, self.wheel_z, self.rw_r) for y in self.stations]
        circles += [(sy, sz, sr - sr * 0.06), (iy, iz, ir)]
        circles += [(ry, rz, rr) for ry, rz, rr in self.rollers]
        path = track_path(circles, self.tt)
        mb = model.mb(f"Track_{s}", parent="Hull", pivot=(x, 0.0, self.wheel_z))
        track_belt(mb, path, x, self.tw, self.tt, self.pitch)
        return path

    def result(self, path):
        ys = [p[0] for p in path]
        zs = [p[1] for p in path]
        return {
            "track_inner_x": self.outer - self.tw,
            "track_top_z": max(zs) + self.tt / 2,
            "track_front_y": max(ys),
            "track_rear_y": min(ys),
            "wheel_center_z": self.wheel_z,
            "track_width": self.tw,
        }


def _twin(gear, mb, c, side, tire="Rubber", dish=0.0):
    """Two road wheel discs per station with a gap for the guide teeth."""
    w = gear.rw_width / 2 - 0.01
    x, y, z = c
    for k in (-1, 1):
        outer = k == side
        W.disc_wheel(mb, (x + k * (w / 2 + 0.01), y, z), gear.rw_r, w, side, tire=tire, dish=dish if outer else 0.0,
                     inner_face=False, hub_cap=outer)


@part("leaf_spring_bogie")
def leaf_spring_bogie(model, p, ctx):
    """Leaf-spring bogies with paired twin road wheels (Panzer IV type).

    Extra parameters: none; road wheel stations are paired front to back into
    bogies (`road_wheels` must have an even count).
    """
    g = Gear(p, ctx)
    if len(g.stations) % 2:
        raise ValueError("leaf_spring_bogie: road_wheels needs an even count")
    hull = model.mb("Hull")
    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, lambda mb, c, sd: _twin(g, mb, c, sd))
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.07)
        ys = sorted(g.stations)
        for a, b in zip(ys[0::2], ys[1::2]):
            mid = (a + b) / 2
            z = g.wheel_z
            # bogie body between the wheel hubs, inside of the wheels
            hull.box((xi, mid, z + 0.05), (0.12, abs(b - a) * 0.55, 0.22), "Paint", bevel=0.01)
            # quarter-elliptic leaf spring on top of the bogie
            hull.box((xi, mid, z + 0.21), (0.10, abs(b - a) * 0.9, 0.05), "Metal")
            # swing arms to the wheel hubs
            for y in (a, b):
                hull.box((xi + side * 0.02, (y + mid) / 2, z), (0.06, abs(y - mid) + 0.06, 0.07), "Paint")
            # mounting bracket to the hull
            hull.box((xi - side * 0.12, mid, z + 0.12), (0.14, 0.3, 0.2), "Paint")
    return g.result(path)


@part("vvss_bogie")
def vvss_bogie(model, p, ctx):
    """Vertical volute spring bogies, two wheels each, roller on top (Sherman).

    Extra parameter: `roller_offset` (Y offset of the roller on each bogie,
    default -0.18 = behind the bogie center). `return_rollers` is ignored, the
    rollers sit on the bogies at `return_roller_diameter`.
    """
    off = p.get("roller_offset", -0.18)
    rr_d = p.get("return_roller_diameter", 0.25)
    p.t.pop("return_rollers", None)
    g = Gear(p, ctx)
    ys = sorted(g.stations)
    if len(ys) % 2:
        raise ValueError("vvss_bogie: road_wheels needs an even count")
    bogies = [((a + b) / 2, abs(b - a)) for a, b in zip(ys[0::2], ys[1::2])]
    roller_z = g.wheel_z + g.rw_r + 0.32
    g.rollers = [(mid + off, roller_z, rr_d / 2) for mid, _ in bogies]
    hull = model.mb("Hull")
    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, lambda mb, c, sd: _twin(g, mb, c, sd), rollers_as_objects=True)
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.06)
        for mid, span in bogies:
            z = g.wheel_z
            # bogie housing bolted to the hull
            hull.box((xi, mid, z + 0.20), (0.18, span * 0.62, 0.36), "Paint", bevel=0.015)
            # two volute springs
            for k in (-1, 1):
                hull.cylinder((x, mid + k * span * 0.14, z + 0.05), (x, mid + k * span * 0.14, z + 0.32),
                              0.07, seg=8, mat="Metal")
            # wheel arms from the housing down to the hubs
            for k in (-1, 1):
                hull.box((x - side * (g.tw * 0.0), mid + k * span * 0.3, z + 0.04), (g.tw * 0.18, span * 0.4, 0.08),
                         "Paint")
            # roller bracket on top of the housing
            hull.box((xi + side * 0.04, mid + off, z + 0.46), (0.1, 0.1, 0.22), "Paint")
            # track skid
            hull.box((xi, mid + off + 0.22, z + 0.42), (0.08, 0.12, 0.12), "Metal")
    return g.result(path)


@part("christie")
def christie(model, p, ctx):
    """Large Christie road wheels on swing arms, no return rollers (T-34).

    Extra parameter: `dish` (outward bulge of the wheel disc, m).
    """
    dish = p.get("dish", 0.05)
    g = Gear(p, ctx)
    hull = model.mb("Hull")
    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, lambda mb, c, sd: _twin(g, mb, c, sd, dish=dish))
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.05)
        for y in g.stations:
            # swing arm from the hull to the hub, sloping backwards
            hull.box((xi, y - 0.12, g.wheel_z + 0.06), (0.1, 0.32, 0.1), "Paint")
    return g.result(path)


@part("halftrack_rear")
def halftrack_rear(model, p, ctx):
    """Track unit of a halftrack. Combine with a `wheel_axle` for the front.

    Uses the common parameters; `interleaved = true` staggers the road wheels
    inward and outward (German Schachtellaufwerk), otherwise a VVSS-like bogie
    layout with single wheels (US halftracks).
    """
    interleaved = p.get("interleaved", False)
    g = Gear(p, ctx)
    path = None
    for side, s in SIDES:
        def wheel(mb, c, sd, _n=[0]):
            _n[0] += 1
            x, y, z = c
            shift = (g.tw * 0.22) * (1 if _n[0] % 2 else -1) if interleaved else 0.0
            W.disc_wheel(mb, (x + shift, y, z), g.rw_r, g.tw * 0.45, sd, tire="Paint" if interleaved else "Rubber")

        path = g.build_common(model, side, s, wheel)
    return g.result(path)


# --- prompt 008: running gear for the remaining tanks and SPGs ----------------------


@part("interleaved")
def interleaved(model, p, ctx):
    """Interleaved road wheels on torsion bars (German 'Schachtellaufwerk':
    Tiger I, Panther, Pz.Kpfw. II Ausf. L 'Luchs').

    Extra parameters: `rows` (2 or 3 wheel layers across the track, default 3:
    Tiger I; 2: Luchs), `tire` ("Rubber" for rubber-tired wheels, default;
    "Paint" for steel-tired ones), `arm` (length of the trailing torsion arm,
    default 0.32). Stations alternate between the outer layers and the inner
    layer, so neighboring wheels overlap like the real ones; every station is
    one `Wheel_*_NN` object. The large wheels need no return rollers.
    """
    rows = p.get("rows", 3)
    tire = p.get("tire", "Rubber")
    arm = p.get("arm", 0.32)
    if rows not in (2, 3):
        raise ValueError("interleaved: rows must be 2 or 3")
    g = Gear(p, ctx)
    hull = model.mb("Hull")
    layer_w = g.tw / rows * 0.92
    path = None
    for side, s in SIDES:
        idx = [0]

        def wheel(mb, c, sd, _idx=idx):
            i = _idx[0]
            _idx[0] += 1
            x, y, z = c
            step = g.tw / rows
            if rows == 3:
                layers = (1, -1) if i % 2 == 0 else (0,)
            else:
                layers = (0.5,) if i % 2 == 0 else (-0.5,)
            for k in layers:
                outer = k == max(layers) and i % 2 == 0
                W.disc_wheel(mb, (x + sd * k * step, y, z), g.rw_r, layer_w, sd, tire=tire, dish=0.03 if outer else 0.0,
                             inner_face=False, hub_cap=k >= 0)

        path = g.build_common(model, side, s, wheel)
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.06)
        for y in g.stations:
            # trailing torsion arm from the hull side to the hub
            hull.box((xi, y - arm / 2, g.wheel_z + 0.04), (0.10, arm, 0.10), "Paint")
    return g.result(path)


@part("torsion_bar")
def torsion_bar(model, p, ctx):
    """Road wheels on torsion-bar swing arms with optional return rollers
    (StuG III, T-70, IS-2, ISU-152, SU-76, M26, Wespe).

    Uses the common parameters (`return_rollers` for vehicles that have them).
    Extra parameters: `twin` (two discs per station, default true; false =
    single wide wheel), `tire` (default "Rubber"), `dish`, `arm` (swing arm
    length, default 0.30), `arm_leading` (arms point forward from the pivot,
    default false = trailing arms), `bump_stops` (small stops above the
    arms, default true), `spokes` (number of raised spokes on the outer
    face of single wheels, default 0 = plain disc: spoked wheels of the
    early T-60).
    """
    twin = p.get("twin", True)
    tire = p.get("tire", "Rubber")
    dish = p.get("dish", 0.0)
    spokes = p.get("spokes", 0)
    arm = p.get("arm", 0.30)
    lead = -1.0 if p.get("arm_leading", False) else 1.0
    stops = p.get("bump_stops", True)
    g = Gear(p, ctx)
    hull = model.mb("Hull")

    def wheel(mb, c, sd):
        if twin:
            _twin(g, mb, c, sd, tire=tire, dish=dish)
        else:
            W.disc_wheel(mb, c, g.rw_r, g.rw_width, sd, tire=tire, dish=dish)
            if spokes:
                W.spoke_ribs(mb, c, g.rw_r * 0.86, c[0] + sd * g.rw_width / 2, sd, spokes)

    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, wheel)
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.05)
        for y in g.stations:
            hull.box((xi, y + lead * arm / 2, g.wheel_z + 0.05), (0.09, arm, 0.09), "Paint")
            hull.cylinder((xi - side * 0.06, y + lead * arm, g.wheel_z + 0.08),
                          (xi + side * 0.04, y + lead * arm, g.wheel_z + 0.08), 0.07, seg=8, mat="Paint")
            if stops:
                hull.box((xi, y + lead * arm * 0.3, g.wheel_z + g.rw_r * 0.75), (0.08, 0.08, 0.06), "Metal")
    return g.result(path)
