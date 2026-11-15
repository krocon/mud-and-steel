"""Anti-aircraft guns: cruciform platforms, traversing mounts (prompt 008).

2 cm Flak 38, 8.8 cm Flak 36, 37 mm 61-K, 85 mm 52-K, 40 mm Bofors M1 and
90 mm M2 are modeled in firing position: the `Carriage` is the platform with
its outriggers (and, where the gun fires from its wheels, the axles); the
`Mount` (upper carriage with seats, sights and handwheels) traverses around
Z on the platform; the `Gun` elevates on the mount. The rest pose has the
barrel horizontal, so the catalog length and height apply directly.

`crew_sockets` places one `Socket_Crew_NN` per crew member for any gun.
"""

import math

from .assemblies import _base_ctx, _run_list, _table
from .registry import assembly, build_part, part


@part("cruciform_platform")
def cruciform_platform(model, p, ctx):
    """Firing platform with four outrigger arms and leveling jacks (Carriage).

    Parameters: `center` [diameter, height] of the central platform
    (default [1.2, 0.3]), `z` (top of the platform, default height),
    `arms` [[angle_deg, length], ...] (0 = forward +Y, 90 = right; length
    from the center to the jack; default four arms of `length` / `width`),
    `length` (front-rear arm span, default 4.0), `width` (side arm span,
    "fit" = catalog width, default), `beam` [width, height] of the arm
    beams (default [0.18, 0.16]), `jack` [diameter, height] of the leveling
    jack pads (default [0.32, 0.12]), `y` (platform center, default 0).
    Adds `Carriage`. Returns `platform_top_z`, `platform_y`.
    """
    cd, ch = p.get("center", [1.2, 0.3])
    z = p.get("z", ch + 0.05)
    cy = p.get("y", 0.0)
    L = p.get("length", 4.0)
    Wd = p.get("width", "fit")
    Wd = ctx["width"] if Wd == "fit" else Wd
    bw, bh = p.get("beam", [0.18, 0.16])
    jd, jh = p.get("jack", [0.32, 0.12])
    arms = p.get("arms")
    if arms is None:
        arms = [[0, L / 2], [180, L / 2], [90, Wd / 2], [270, Wd / 2]]
    car = model.mb("Carriage")
    car.cylinder((0, cy, z - ch), (0, cy, z), cd / 2, seg=16, mat="Paint")
    for ang, length in arms:
        a = math.radians(ang)
        d = (math.sin(a), math.cos(a))
        reach = length - jd / 2
        # beam from the platform edge to the jack, sloping down
        p0 = (d[0] * cd * 0.4, cy + d[1] * cd * 0.4, z - bh / 2)
        p1 = (d[0] * reach, cy + d[1] * reach, jh + bh / 2)
        n = (-d[1] * bw / 2, d[0] * bw / 2)
        rings = []
        for (x, y, zz) in (p0, p1):
            rings.append([(x - n[0], y - n[1], zz - bh / 2), (x + n[0], y + n[1], zz - bh / 2),
                          (x + n[0], y + n[1], zz + bh / 2), (x - n[0], y - n[1], zz + bh / 2)])
        car.loft(rings, "Paint", bevel=0.008)
        # jack screw and foot pad at the arm end: the pad reaches the arm length
        jx, jy = d[0] * reach, cy + d[1] * reach
        car.cylinder((jx, jy, 0.0), (jx, jy, jh * 0.4), jd / 2, seg=10, mat="Metal")
        car.cylinder((jx, jy, jh * 0.4), (jx, jy, jh + bh), 0.035, seg=6, mat="Metal")
    return {"platform_top_z": z, "platform_y": cy}


@part("aa_mount")
def aa_mount(model, p, ctx):
    """Traversing upper carriage of an anti-aircraft gun (`Mount`).

    Parameters: `z` (bottom, default platform top), `y` (traverse axis,
    default platform center), `pedestal` [diameter, height] (default
    [0.7, 0.35]), `body` [x, y, z] size of the upper carriage box above the
    pedestal (default [0.9, 1.0, 0.35]), `cheeks` [height, length,
    spacing] of the two side frames that carry the trunnions (default
    [0.5, 0.9, 0.5]), `seats` [[x, y, z], ...] (gun layer seats, model
    space), `shield` = { width, height, bottom_z, y, slope_deg } (optional
    shield that traverses with the mount), `handwheels` [[x, y, z], ...].
    Adds `Mount` (child of Carriage, pivot on the traverse axis) and an
    optional `Shield` (child of Mount). Returns `mount_top_z`,
    `trunnion_z` (top of the cheeks), `mount_y`.
    """
    z0 = p.get("z", ctx.get("platform_top_z", 0.4))
    y = p.get("y", ctx.get("platform_y", 0.0))
    pd, ph = p.get("pedestal", [0.7, 0.35])
    bx, by, bz = p.get("body", [0.9, 1.0, 0.35])
    chh, chl, chs = p.get("cheeks", [0.5, 0.9, 0.5])
    mb = model.mb("Mount", parent="Carriage", pivot=(0.0, y, z0))
    mb.cylinder((0, y, z0), (0, y, z0 + ph), pd / 2, seg=14, mat="Paint")
    top = z0 + ph
    mb.box((0, y, top + bz / 2), (bx, by, bz), "Paint", bevel=0.015)
    top += bz
    for k in (-1, 1):
        mb.extrude([(k * chs / 2, y - chl / 2, top), (k * chs / 2, y + chl / 2, top),
                    (k * chs / 2, y + chl * 0.15, top + chh), (k * chs / 2, y - chl * 0.25, top + chh)],
                   (k * 0.03, 0, 0), "Paint")
    for sx, sy, sz in p.get("seats", []):
        mb.box((sx, sy, sz), (0.32, 0.30, 0.05), "Paint")
        mb.cylinder((sx, sy, top - 0.05), (sx, sy, sz - 0.03), 0.025, seg=6, mat="Metal")
    for hx, hy, hz in p.get("handwheels", []):
        mb.cylinder((hx, hy, hz), (hx + math.copysign(0.03, hx or 1), hy, hz), 0.10, seg=10, mat="Metal")
    sh = p.get("shield")
    if sh:
        sb = model.mb("Shield", parent="Mount", pivot=(0.0, sh["y"], sh["bottom_z"]))
        w, h = sh["width"], sh["height"]
        slope = math.tan(math.radians(sh.get("slope_deg", 10.0)))
        yb, zb = sh["y"], sh["bottom_z"]
        for k in (-1, 1):
            pts = [(k * 0.08, yb, zb), (k * w / 2, yb, zb), (k * w / 2, yb - h * slope, zb + h),
                   (k * 0.08, yb - h * slope, zb + h)]
            sb.extrude(pts, (0, -0.01, 0), "Paint")
        sb.extrude([(-0.08, yb, zb), (0.08, yb, zb), (0.08, yb - 0.1 * h * slope, zb + 0.1 * h),
                    (-0.08, yb - 0.1 * h * slope, zb + 0.1 * h)], (0, -0.01, 0), "Paint")
    return {"mount_top_z": top, "trunnion_z": top + chh, "mount_y": y}


@part("crew_sockets")
def crew_sockets(model, p, ctx):
    """One `Socket_Crew_NN` per crew member, on an arc around a center or at
    given positions.

    Parameters: `positions` [[x, y], ...] (model space, on the ground), or
    `count` (default catalog crew), `center` [x, y] (default origin),
    `radius` (default 2.0), `start_deg` / `end_deg` (arc, 0 = forward,
    clockwise seen from above; default 100 to 260 = behind the gun),
    `parent` (default Carriage), `z` (default 0).
    """
    parent = p.get("parent", "Carriage")
    z = p.get("z", 0.0)
    pos = p.get("positions")
    if pos is None:
        n = p.get("count", ctx["unit"].get("crew", 0))
        cx, cy = p.get("center", [0.0, 0.0])
        r = p.get("radius", 2.0)
        a0, a1 = p.get("start_deg", 100.0), p.get("end_deg", 260.0)
        pos = []
        for i in range(n):
            a = math.radians(a0 + (a1 - a0) * (i + 0.5) / max(n, 1))
            pos.append([cx + math.sin(a) * r, cy + math.cos(a) * r])
    else:
        for k in ("count", "center", "radius", "start_deg", "end_deg"):
            p.get(k)
    for x, y in pos:
        model.socket(model.next_index("Socket_Crew_"), parent, (x, y, z))


@assembly("aa_gun")
def aa_gun(model, bp, u, tune):
    """Anti-aircraft gun in firing position.

    Blueprint sections, in build order: `[carriage]` (cruciform_platform
    or another part that builds `Carriage`), `[[axle]]` (optional
    wheel_axle tables, parent Carriage: guns that fire from their wheels or
    keep the bogies attached), `[mount]` (aa_mount), `[gun]` (field_gun,
    parent Mount; one barrel segment "fit" = catalog overall length
    measured from the rear end of the model), `[[part]]`.

    The model is moved so that the origin lies at the center of the
    footprint (ground contact of jacks and wheels).
    """
    ctx, _ = _base_ctx(bp, u, tune)
    model.mb("Carriage")
    name, t = _table(bp["carriage"], "[carriage]")
    build_part(name, model, t, ctx, where="[carriage]")
    for i, t in enumerate(bp.get("axle", [])):
        t = dict(t)
        name = t.pop("part", "wheel_axle")
        t.setdefault("index", i + 1)
        t.setdefault("parent", "Carriage")
        build_part(name, model, t, ctx, where=f"[[axle]] #{i + 1}")
    name, t = _table(bp["mount"], "[mount]")
    build_part(name, model, t, ctx, where="[mount]")
    lo, _ = model.extent()
    ctx["overall_front_y"] = lo.y + ctx["length"]
    name, t = _table(bp["gun"], "[gun]")
    t.setdefault("parent", "Mount")
    build_part(name, model, t, ctx, where="[gun]")
    _run_list(model, bp, "part", ctx)
    _center_footprint(model)
    return ctx


def _center_footprint(model, tol=0.03):
    """Moves the model so that the ground contact is centered on the origin
    (same rule as tools/blender/validate.py)."""
    verts = [v.co for p in model.parts.values() for v in p["mb"].bm.verts]
    lo_z = min(v.z for v in verts)
    g = [v for v in verts if v.z < lo_z + tol]
    cx = (min(v.x for v in g) + max(v.x for v in g)) / 2
    cy = (min(v.y for v in g) + max(v.y for v in g)) / 2
    model.translate((-cx, -cy, -lo_z))
