"""Boats (prompt 119): landing craft with a bow ramp and small assault boats.

The hull is a `section_hull` (cross-sections along Y with an open well);
landing craft get a `Ramp` object hinged at the bottom of the bow, which the
game lowers and raises (it rotates around X; the model is built with the ramp
up). Generic parts, no faction details:

- `boat_ramp`: the hinged bow ramp (`Ramp`, parent `Hull`).
- `boat_tube`: a round tube along a polyline in the X/Y plane (the side tubes
  of an inflatable boat, a gunwale rail).
- assembly `boat`: `[hull]`, `[ramp]` (optional), `[[part]]`, `[[mg]]`.

The origin lies on the keel (lowest point at z = 0) under the middle of the
keel's flat; the game floats the boat at the water level less its draft.
"""

import math

from .catalog import mm
from .registry import assembly, build_part, part


@part("boat_ramp")
def boat_ramp(model, p, ctx):
    """Bow ramp hinged at its bottom edge, built standing up (closed).

    Parameters: `y` (hinge line, model space; "front" = the bow end of the
    hull from `section_hull`),
    `z` (hinge height), `width` (outer width), `height` (ramp length = how
    high it stands when closed), `lean_deg` (default 8: the closed ramp
    leans forward), `thickness` (default 0.06), `ribs` (default 3: stiffeners
    on the inside). Adds `Ramp` (parent `Hull`, pivot on the hinge).
    """
    y, z = p.req("y"), p.req("z")
    if y == "front":
        y = ctx["hull_front_y"]
    w, h = p.req("width"), p.req("height")
    t = p.get("thickness", 0.06)
    lean = math.tan(math.radians(p.get("lean_deg", 8.0))) * h
    mb = model.mb("Ramp", parent="Hull", pivot=(0.0, y, z))
    hw = w / 2
    # The plate: hinge at the bottom, top leaning forward.
    mb.extrude([(-hw, y, z), (hw, y, z), (hw, y + lean, z + h), (-hw, y + lean, z + h)], (0.0, -t, 0.0), "Paint")
    n = p.get("ribs", 3)
    for k in range(n):
        x = -hw + w * (k + 1) / (n + 1)
        mb.extrude([(x - 0.03, y - t, z + 0.05), (x + 0.03, y - t, z + 0.05),
                    (x + 0.03, y - t + lean * 0.9, z + h * 0.92), (x - 0.03, y - t + lean * 0.9, z + h * 0.92)],
                   (0.0, -0.05, 0.0), "Metal")
    return {"ramp_top_z": z + h}


@part("boat_tube")
def boat_tube(model, p, ctx):
    """Round tube along a closed or open polyline at one height.

    Parameters: `points` list of [x, y] (model space), `z` (center height),
    `radius`, `closed` (default true), `segments` (default 8), `material`
    (default Canvas), `parent` (default Hull).
    """
    pts = [tuple(v) for v in p.req("points")]
    z, r = p.req("z"), p.req("radius")
    seg = p.get("segments", 8)
    mat = p.get("material", "Canvas")
    mb = model.mb(p.get("parent", "Hull"))
    pairs = list(zip(pts, pts[1:]))
    if p.get("closed", True):
        pairs.append((pts[-1], pts[0]))
    for (xa, ya), (xb, yb) in pairs:
        mb.cylinder((xa, ya, z), (xb, yb, z), r, seg=seg, mat=mat)
        mb.revolve((xa, ya, z), (0, 0, 1), [(0.0, -r), (r, 0.0), (0.0, r)], seg=seg, mat=mat)


@assembly("boat")
def boat(model, bp, u, tune):
    """Boat: hull from cross-sections, optional bow ramp, parts and MGs.

    Blueprint sections, in build order: `[hull]` (`section_hull`, usually
    with an `open` well), `[ramp]` (`boat_ramp`, landing craft only),
    `[[part]]`, `[[mg]]` (`pintle_mg`). The hull length is the catalog
    length. Adds `Socket_Fire_01` in the well, one `Socket_Passenger_NN` per
    catalog passenger on the floor of the well and, for boats with an engine
    (catalog `power_hp`), `Socket_Exhaust_01` at the stern. Afterwards the
    origin is moved to the middle of the keel's flat (the lowest vertices).
    """
    ctx = {
        "unit": u,
        "length": mm(u, "length_mm"),
        "width": mm(u, "width_mm"),
        "height": mm(u, "height_mm") + tune.get("height", 0.0),
        "half_width": mm(u, "width_mm") / 2,
        "layout": bp.get("layout", {}),
        # The ramp leans out in front of the hull: `[layout] bow_ramp` m
        # of the catalog length are the ramp's, not the hull's.
        "hull_length": mm(u, "length_mm") - bp.get("layout", {}).get("bow_ramp", 0.0),
    }
    t = dict(bp["hull"])
    name = t.pop("part")
    ctx.update(build_part(name, model, t, ctx, where="[hull]") or {})
    if "ramp" in bp:
        t = dict(bp["ramp"])
        name = t.pop("part", "boat_ramp")
        ctx.update(build_part(name, model, t, ctx, where="[ramp]") or {})
    for key in ("part", "mg"):
        for i, tt in enumerate(bp.get(key, [])):
            tt = dict(tt)
            name = tt.pop("part")
            build_part(name, model, tt, ctx, where=f"[[{key}]] #{i + 1} ({name})")
    floor = ctx.get("hull_floor_z") or 0.3
    model.socket("Socket_Fire_01", "Hull", (0.0, 0.0, floor + 0.2))
    # One seat per passenger of the catalog in the well, rows from the bow
    # (the game seats the men aboard there).
    comp = ctx.get("compartment")
    if comp and u.get("passengers"):
        yf, yr, hw = comp
        cols = max(2, int(2 * hw / 0.6))
        n = int(u["passengers"])
        rows = -(-n // cols)
        pitch = min(0.75, (yf - yr - 0.4) / max(rows, 1))
        for k in range(n):
            r, c = divmod(k, cols)
            x = -hw + 0.3 + (2 * hw - 0.6) * (c / max(cols - 1, 1))
            y = yf - 0.3 - pitch * r
            model.socket(f"Socket_Passenger_{k + 1:02d}", "Hull", (x, y, floor))
    if u.get("power_hp"):
        rear = ctx.get("hull_rear_y", -ctx["length"] / 2)
        model.socket("Socket_Exhaust_01", "Hull", (0.0, rear + 0.3, ctx["height"] * 0.8))
    lo_z = min(v.co.z for q in model.parts.values() for v in q["mb"].bm.verts)
    ys = [v.co.y for q in model.parts.values() for v in q["mb"].bm.verts if v.co.z < lo_z + 0.03]
    model.translate((0.0, -(min(ys) + max(ys)) / 2, -lo_z))
    return ctx
