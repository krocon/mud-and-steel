"""Halftracks and other open vehicles: hull from cross-sections, fenders, MG posts
and the `halftrack` assembly (prompt 006).

Added for the Sd.Kfz. 251 (de) and the M3 halftrack (us); generic, no faction
details. The hull is lofted from cross-sections along Y, which covers the
faceted 251 body (lower plates sloping out, upper plates sloping in) as well
as the box-shaped M3 body (all half-widths equal). An open troop compartment
is cut into the hull between two sections.
"""

from .catalog import mm
from .registry import assembly, build_part, part
from .weapons import machine_gun

SECTION_KEYS = ("y", "bottom_z", "bottom_hw", "chine_z", "chine_hw", "top_z", "top_hw")


def _ring(s, inset=0.0):
    """Outer ring of a section, counter-clockwise seen from the front:
    bottom L, bottom R, chine R, top R, top L, chine L."""
    y = s["y"]
    return [
        (-(s["bottom_hw"] - inset), y, s["bottom_z"]),
        (s["bottom_hw"] - inset, y, s["bottom_z"]),
        (s["chine_hw"] - inset, y, s["chine_z"]),
        (s["top_hw"] - inset, y, s["top_z"]),
        (-(s["top_hw"] - inset), y, s["top_z"]),
        (-(s["chine_hw"] - inset), y, s["chine_z"]),
    ]


@part("section_hull")
def section_hull(model, p, ctx):
    """Hull lofted from cross-sections along Y (halftracks, trucks, open vehicles).

    Parameters:
    - `sections`: list of tables, front to rear, each with `y`, `bottom_z`,
      `bottom_hw`, `chine_z`, `chine_hw`, `top_z`, `top_hw` (half-widths in m).
      The chine is the knuckle between lower and upper side plates; give it
      the same half-width as the top for vertical sides.
    - `open`: [y_front, y_rear] of an open-topped compartment (both must be
      section positions). The compartment gets walls of `wall` thickness
      (default 0.02 m) down to `floor_z` (default bottom_z + 0.15).
    - `height`: "fit" (default) scales all `top_z` and `chine_z` so the
      highest top equals the catalog height, or "keep".
    - `bevel`: plate edge bevel (default 0.01).
    The Y positions are scaled to the catalog hull length (error if more than
    8 % off, like `plate_hull`). Returns `hull_top_z`, `hull_front_y`,
    `hull_rear_y`, `hull_floor_z`, `compartment` = (y_front, y_rear, inner
    half-width at the top).
    """
    raw = [dict(s) for s in p.req("sections")]
    for i, s in enumerate(raw):
        missing = [k for k in SECTION_KEYS if k not in s]
        extra = sorted(set(s) - set(SECTION_KEYS))
        if missing or extra:
            raise KeyError(f"section_hull: section #{i + 1}: missing {missing}, unknown {extra}")
        if not s["bottom_z"] < s["chine_z"] <= s["top_z"]:
            raise ValueError(f"section_hull: section #{i + 1}: needs bottom_z < chine_z <= top_z")
    ys = [s["y"] for s in raw]
    if ys != sorted(ys, reverse=True):
        raise ValueError("section_hull: sections must run from front (+Y) to rear")
    raw_len = ys[0] - ys[-1]
    sy = ctx["hull_length"] / raw_len
    if abs(sy - 1.0) > 0.08:
        raise ValueError(f"section_hull: sections are {raw_len:.2f} m long but the catalog hull is "
                         f"{ctx['hull_length']:.2f} m; fix the sections (scale {sy:.3f})")
    cy = (ys[0] + ys[-1]) / 2
    top = max(s["top_z"] for s in raw)
    sz = ctx["height"] / top if p.get("height", "fit") == "fit" else 1.0
    if abs(sz - 1.0) > 0.08:
        raise ValueError(f"section_hull: highest top_z {top:.2f} m but the catalog height is "
                         f"{ctx['height']:.2f} m; fix the sections (scale {sz:.3f})")
    secs = []
    for s in raw:
        s = dict(s)
        s["y"] = (s["y"] - cy) * sy
        s["top_z"] *= sz
        s["chine_z"] = min(s["chine_z"] * sz, s["top_z"])
        secs.append(s)

    bevel = p.get("bevel", 0.01)
    hull = model.mb("Hull")
    opening = p.get("open")
    if not opening:
        hull.loft([_ring(s) for s in secs], "Paint", bevel=bevel)
        return _result(secs, None, None)

    yf, yr = ((v - cy) * sy for v in opening)
    idx_f = _index(secs, yf, "open[0]")
    idx_r = _index(secs, yr, "open[1]")
    if not idx_f < idx_r:
        raise ValueError("section_hull: open = [y_front, y_rear] with y_front > y_rear")
    t = p.get("wall", 0.02)
    floor = p.get("floor_z", secs[idx_f]["bottom_z"] + 0.15)
    if idx_f > 0:
        hull.loft([_ring(s) for s in secs[:idx_f + 1]], "Paint", bevel=bevel)
    if idx_r < len(secs) - 1:
        hull.loft([_ring(s) for s in secs[idx_r:]], "Paint", bevel=bevel)
    _open_compartment(hull, secs[idx_f:idx_r + 1], t, floor)
    inner_hw = min(min(s["top_hw"], s["chine_hw"]) for s in secs[idx_f:idx_r + 1]) - t
    return _result(secs, floor, (yf, yr, inner_hw))


def _index(secs, y, what):
    for i, s in enumerate(secs):
        if abs(s["y"] - y) < 1e-6:
            return i
    raise ValueError(f"section_hull: {what} = {y:.3f} is not a section position")


def _open_compartment(mb, secs, t, floor):
    """Closed U-shaped shell: outer hull without roof, inner walls, floor."""
    rings = []
    for s in secs:
        if not s["bottom_z"] < floor < s["top_z"] - 0.1:
            raise ValueError(f"section_hull: floor_z {floor:.2f} must lie between bottom_z and top_z - 0.1")
        outer = _ring(s)  # bL, bR, cR, tR, tL, cL
        y = s["y"]
        cz = max(s["chine_z"], floor + 0.01)
        it = s["top_hw"] - t
        ic = _hw_at(s, cz) - t
        fl = _hw_at(s, floor) - t
        inner = [(it, y, s["top_z"]), (ic, y, cz), (fl, y, floor), (-fl, y, floor), (-ic, y, cz), (-it, y, s["top_z"])]
        # U polygon: tL, cL, bL, bR, cR, tR, then the inner wall from top R over the floor to top L
        rings.append([outer[4], outer[5], outer[0], outer[1], outer[2], outer[3]] + inner)
    vs = [mb.verts(r) for r in rings]
    n = len(rings[0])
    faces = []
    for a, b in zip(vs, vs[1:]):
        for i in range(n):
            j = (i + 1) % n
            faces.append(mb.face([a[i], a[j], b[j], b[i]], "Paint"))
    faces.append(mb.face(list(reversed(vs[0])), "Paint"))
    faces.append(mb.face(vs[-1], "Paint"))
    mb._shell(faces, bevel=0.0)


def _hw_at(s, z):
    """Outer half-width of a section at height z."""
    if z <= s["chine_z"]:
        a, b = ("bottom_z", "bottom_hw"), ("chine_z", "chine_hw")
    else:
        a, b = ("chine_z", "chine_hw"), ("top_z", "top_hw")
    z0, z1 = s[a[0]], s[b[0]]
    k = 0.0 if abs(z1 - z0) < 1e-9 else (z - z0) / (z1 - z0)
    return s[a[1]] + (s[b[1]] - s[a[1]]) * k


def _result(secs, floor, compartment):
    return {
        "hull_top_z": max(s["top_z"] for s in secs),
        "hull_front_y": secs[0]["y"],
        "hull_rear_y": secs[-1]["y"],
        "hull_floor_z": floor,
        "compartment": compartment,
        "hull_sections": secs,
    }


@part("fender")
def fender(model, p, ctx):
    """Mudguard plate over a wheel or track, bent along a side profile.

    Parameters: `profile` list of [y, z] points (open polyline, front to
    rear), `x` [inner, outer] (outer may be "full" = catalog half width),
    `thickness` (default 0.01), `both_sides` (default true, mirrors to -x),
    `parent` (default Hull).
    """
    pts = [tuple(v) for v in p.req("profile")]
    x0, x1 = p.req("x")
    if x1 == "full":
        x1 = ctx["half_width"]
    t = p.get("thickness", 0.01)
    mb = model.mb(p.get("parent", "Hull"))
    sides = (1, -1) if p.get("both_sides", True) else (1,)
    for side in sides:
        xa, xb = sorted((side * x0, side * x1))
        for (ya, za), (yb, zb) in zip(pts, pts[1:]):
            mb.extrude_x([(ya, za), (yb, zb), (yb, zb - t), (ya, za - t)], xa, xb, "Paint")


@part("plate")
def plate(model, p, ctx):
    """Thin plate on a sloped surface: doors, lids, hatches, appliqué armor.

    Parameters: `corners` four [x, y, z] points (counter-clockwise seen from
    outside), `thickness` (default 0.012, built outward along the face
    normal), `material` (default Paint), `mirror` (default false: also
    builds the copy mirrored to -x), `parent` (default Hull).
    """
    from mathutils import Vector

    pts = [Vector(c) for c in p.req("corners")]
    if len(pts) != 4:
        raise ValueError("plate: corners needs four points")
    t = p.get("thickness", 0.012)
    mat = p.get("material", "Paint")
    mb = model.mb(p.get("parent", "Hull"))
    sets = [pts]
    if p.get("mirror", False):
        sets.append([Vector((-v.x, v.y, v.z)) for v in reversed(pts)])
    for q in sets:
        n = (q[1] - q[0]).cross(q[2] - q[0]).normalized()
        mb.extrude([tuple(v) for v in q], tuple(n * t), mat)


@part("pintle_mg")
def pintle_mg(model, p, ctx):
    """Machine gun on a post or rail mount of an open vehicle, optional shield.

    Parameters: `position` [x, y, z] (mount pivot, model space), `length`
    (gun length, default 1.22 = MG 42), `post` (height of the post below the
    pivot, default 0.25; 0 = none), `shield` = { width, height, y, slope_deg }
    (optional gun shield in front of the pivot, moves with the gun), `jacket`
    (perforated barrel jacket, default true), `parent` (default Hull),
    `primary` (default false: also adds `Socket_Muzzle` on this MG, for
    vehicles whose main weapon is a machine gun).
    Adds `MG_NN` and `Socket_Muzzle_NN` (NN = MG index + 1).
    """
    import math

    x, y, z = p.req("position")
    length = p.get("length", 1.22)
    post = p.get("post", 0.25)
    parent = p.get("parent", "Hull")
    name = model.next_index("MG_")
    mb = model.mb(name, parent=parent, pivot=(x, y, z))
    if post > 0:  # the pintle swivels with the gun, so it belongs to the MG object
        mb.cylinder((x, y, z - post), (x, y, z - 0.04), 0.025, seg=6, mat="Metal")
    mb.box((x, y, z - 0.03), (0.06, 0.12, 0.06), "Metal")
    tip = machine_gun(mb, x, y + 0.25, z + 0.04, length * 0.8, r=0.012, jacket=p.get("jacket", True))
    sh = p.get("shield")
    if sh:
        w, h = sh["width"], sh["height"]
        sy = y + sh.get("y", 0.25)
        lean = math.tan(math.radians(sh.get("slope_deg", 10.0))) * h
        zb = z - h * 0.45
        for side in (-1, 1):  # two halves beside the barrel slot
            xo, xi = side * w / 2, side * 0.03
            mb.extrude([(x + xi, sy, zb), (x + xo, sy, zb), (x + xo, sy - lean, zb + h), (x + xi, sy - lean, zb + h)],
                       (0, -0.008, 0), "Paint")
        mb.extrude([(x - 0.03, sy, zb), (x + 0.03, sy, zb), (x + 0.03, sy - lean * 0.3, zb + h * 0.3),
                    (x - 0.03, sy - lean * 0.3, zb + h * 0.3)], (0, -0.008, 0), "Paint")
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, (x, tip, z + 0.04))
    if p.get("primary", False):
        model.socket("Socket_Muzzle", name, (x, tip, z + 0.04))


@assembly("halftrack")
def halftrack(model, bp, u, tune):
    """Halftrack: steered front axle, rear track unit, lofted hull.

    Blueprint sections, in build order: `[layout]`, `[running_gear]`
    (`halftrack_rear`), `[front_axle]` (`wheel_axle`; its wheels get the
    next free index after the road wheels), `[hull]` (`section_hull` or
    `plate_hull`), `[[part]]`, `[[mg]]` (e.g. `pintle_mg`).

    `[layout]`: `track_outer_x` (half distance of the outer track edges;
    halftrack tracks usually sit inside the hull width). The hull length is
    the catalog `length_hull_mm` if present, else the catalog length.

    After building, the model is moved along Y so that the origin lies at
    the center of the ground contact (front tires to rear track end).
    """
    layout = bp.get("layout", {})
    ctx = {
        "unit": u,
        "length": mm(u, "length_mm"),
        "width": mm(u, "width_mm"),
        "height": mm(u, "height_mm") + tune.get("height", 0.0),
        "half_width": mm(u, "width_mm") / 2,
        "layout": layout,
        "hull_length": mm(u, "length_hull_mm") if "length_hull_mm" in u else mm(u, "length_mm"),
        "track_outer_x": layout["track_outer_x"],
    }
    model.mb("Hull")
    name, t = _table(bp["running_gear"], "[running_gear]")
    build_part(name, model, t, ctx, where="[running_gear]")
    n_wheels = len(bp["running_gear"]["road_wheels"])
    name, t = _table(bp["front_axle"], "[front_axle]")
    t.setdefault("index", n_wheels + 1)
    build_part(name, model, t, ctx, where="[front_axle]")
    name, t = _table(bp["hull"], "[hull]")
    build_part(name, model, t, ctx, where="[hull]")
    for key in ("part", "mg"):
        for i, tt in enumerate(bp.get(key, [])):
            name, params = _table(tt, f"[[{key}]] #{i + 1}")
            build_part(name, model, params, ctx, where=f"[[{key}]] #{i + 1} ({name})")
    lo_y, hi_y = _ground_contact_y(model)
    model.translate((0.0, -(lo_y + hi_y) / 2, 0.0))
    return ctx


def _table(t, where):
    t = dict(t)
    name = t.pop("part", None)
    if not name:
        raise KeyError(f"{where}: missing 'part'")
    return name, t


def _ground_contact_y(model, tol=0.03):
    """Y extent of all vertices within `tol` of the lowest point (same rule as
    tools/blender/validate.py uses for the footprint center)."""
    lo_z = min(v.co.z for p in model.parts.values() for v in p["mb"].bm.verts)
    ys = [v.co.y for p in model.parts.values() for v in p["mb"].bm.verts if v.co.z < lo_z + tol]
    return min(ys), max(ys)
