"""Heavy mortars in firing position (prompt 008): 12 cm GrW 42, 120-PM-38,
4.2 inch M2.

`Carriage` is the baseplate with the bipod (and optional transport wheels),
`Gun` the tube with its pivot on the baseplate socket. The tube elevation is
baked into the rest pose; `elevation_deg = "fit"` derives it from the
catalog height, the bipod foot position from the catalog length.
"""

import math

from mathutils import Vector

from .antiair import _center_footprint
from .assemblies import _base_ctx, _run_list, _table
from .registry import assembly, build_part, part


@part("baseplate")
def baseplate(model, p, ctx):
    """Mortar baseplate (Carriage).

    Parameters: `shape` "rect" | "round" (default rect), `size` [x, y]
    (rect) or `diameter` (round), `thickness` (default 0.05), `y` (center,
    default 0), `ribs` (radial or cross ribs on top, default true),
    `socket_z` (height of the ball socket for the tube, default
    thickness + 0.06). Returns `base_y`, `base_socket_z`, `base_rear_y`.
    """
    shape = p.get("shape", "rect")
    t = p.get("thickness", 0.05)
    y = p.get("y", 0.0)
    car = model.mb("Carriage")
    if shape == "round":
        r = p.req("diameter") / 2
        car.cylinder((0, y, 0), (0, y, t), r, seg=16, mat="Paint", smooth=False)
        rear = y - r
        if p.get("ribs", True):
            for i in range(6):
                a = math.pi * i / 3
                car.box((math.sin(a) * r * 0.45, y + math.cos(a) * r * 0.45, t + 0.02),
                        (0.03 + abs(math.cos(a)) * r * 0.75, 0.03 + abs(math.sin(a)) * r * 0.75, 0.04), "Paint")
    elif shape == "rect":
        sx, sy = p.req("size")
        car.box((0, y, t / 2), (sx, sy, t), "Paint", bevel=0.01)
        rear = y - sy / 2
        if p.get("ribs", True):
            car.box((0, y, t + 0.02), (0.04, sy * 0.9, 0.04), "Paint")
            car.box((0, y, t + 0.02), (sx * 0.9, 0.04, 0.04), "Paint")
    else:
        raise ValueError(f"baseplate: unknown shape {shape!r}")
    sz = p.get("socket_z", t + 0.06)
    car.cylinder((0, y, t), (0, y, sz), 0.07, seg=10, mat="Metal")
    return {"base_y": y, "base_socket_z": sz, "base_rear_y": rear}


@part("mortar_tube")
def mortar_tube(model, p, ctx):
    """Mortar tube (`Gun`) standing on the baseplate socket.

    Parameters: `length` (tube length, real value), `caliber` (bore, m),
    `wall` (default caliber * 0.12), `elevation_deg` (degrees or "fit": so
    that the muzzle reaches the catalog height), `collar` (fraction of the
    length where the bipod clamps, default 0.65), `sight` (sight box at the
    collar, default true). Adds `Gun` (pivot at the socket) and
    `Socket_Muzzle`. Returns `tube_collar` (point), `tube_elevation_deg`.
    """
    L = p.req("length")
    r = p.req("caliber") / 2 + p.get("wall", p.req("caliber") * 0.12)
    y0 = ctx["base_y"]
    z0 = ctx["base_socket_z"]
    e = p.get("elevation_deg", "fit")
    if e == "fit":
        # muzzle rim top: z0 + L sin(e) + r cos(e) = height
        h = ctx["height"] - z0
        best = None
        for i in range(300, 890):
            a = math.radians(i / 10)
            v = L * math.sin(a) + r * math.cos(a)
            if best is None or abs(v - h) < best[0]:
                best = (abs(v - h), i / 10)
        if best[0] > 0.01:
            raise ValueError(f"mortar_tube: catalog height {ctx['height']:.2f} m not reachable with a "
                             f"{L:.2f} m tube between 30 and 89 degrees; give elevation_deg")
        e = best[1]
    a = math.radians(e)
    d = Vector((0, math.cos(a), math.sin(a)))
    base = Vector((0, y0, z0))
    mb = model.mb("Gun", parent="Carriage", pivot=tuple(base))
    mb.revolve(tuple(base), tuple(d), [(0.0, r * 1.25), (0.12, r * 1.15), (0.14, r), (L - 0.06, r), (L - 0.05, r * 1.12),
                                       (L, r * 1.12)], seg=12, mat="Paint")
    collar_f = p.get("collar", 0.65)
    collar = base + d * (L * collar_f)
    mb.cylinder(tuple(collar - d * 0.04), tuple(collar + d * 0.04), r * 1.5, seg=10, mat="Metal")
    if p.get("sight", True):
        mb.box(tuple(collar + Vector((-r * 2.2, 0, 0.05))), (0.06, 0.12, 0.10), "Metal")
    tip = base + d * L
    model.socket("Socket_Muzzle", "Gun", tuple(tip))
    return {"tube_collar": tuple(collar), "tube_elevation_deg": e, "gun_tip_y": tip.y, "gun_z": tip.z}


@part("mortar_bipod")
def mortar_bipod(model, p, ctx):
    """Bipod from the tube collar to two feet (into Carriage).

    Parameters: `foot_y` (y of the feet, or "fit": front end of the model
    at the catalog length from the baseplate rear), `spread` (distance of
    the feet, or "fit" = catalog width minus the foot pads), `foot` (pad
    size, default 0.12), `leg_radius` (default 0.022), `cross_bar` (height
    fraction of the spreader bar, default 0.4).
    """
    cx, cy, cz = ctx["tube_collar"]
    fy = p.get("foot_y", "fit")
    foot = p.get("foot", 0.12)
    if fy == "fit":
        fy = ctx["base_rear_y"] + ctx["length"] - foot / 2
    sp = p.get("spread", "fit")
    sp = ctx["width"] - foot if sp == "fit" else sp
    lr = p.get("leg_radius", 0.022)
    cb = p.get("cross_bar", 0.4)
    car = model.mb("Carriage")
    feet = []
    for k in (-1, 1):
        f = (k * sp / 2, fy, 0.03)
        car.cylinder((cx, cy, cz), f, lr, seg=6, mat="Paint")
        car.box((f[0], f[1], 0.015), (foot, foot, 0.03), "Metal")
        feet.append(Vector(f))
    top = Vector((cx, cy, cz))
    a, b = (top + (feet[0] - top) * (1 - cb)), (top + (feet[1] - top) * (1 - cb))
    car.cylinder(tuple(a), tuple(b), lr * 0.8, seg=6, mat="Metal")
    # elevating screw from the spreader to the collar
    car.cylinder(tuple((a + b) / 2), (cx, cy, cz), lr * 0.9, seg=6, mat="Metal")


@assembly("mortar")
def mortar(model, bp, u, tune):
    """Heavy mortar in firing position.

    Blueprint sections, in build order: `[baseplate]`, `[tube]`
    (mortar_tube), `[bipod]` (mortar_bipod), `[[axle]]` (optional
    transport wheels, wheel_axle with parent Carriage), `[[part]]`. The
    model is moved so that the origin lies at the center of the footprint.
    """
    ctx, _ = _base_ctx(bp, u, tune)
    model.mb("Carriage")
    for key in ("baseplate", "tube", "bipod"):
        name, t = _table(bp[key], f"[{key}]")
        build_part(name, model, t, ctx, where=f"[{key}]")
    for i, t in enumerate(bp.get("axle", [])):
        t = dict(t)
        name = t.pop("part", "wheel_axle")
        t.setdefault("index", i + 1)
        t.setdefault("parent", "Carriage")
        build_part(name, model, t, ctx, where=f"[[axle]] #{i + 1}")
    _run_list(model, bp, "part", ctx)
    _center_footprint(model)
    return ctx
