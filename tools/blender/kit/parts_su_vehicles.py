"""Faction parts of `su` for prompt 008, session `vehicles`: tanks, tank destroyers, assault guns, SPGs, armored cars and trucks.

Owned by that session only (one module per parallel session, so the three
su sessions never edit the same file). Part names carry the faction prefix
like `parts_su.py`: `@part("su.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("su_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401

import math

from mathutils import Matrix

from .turret import offset_polygon


@part("su.offset_turret")
def offset_turret(model, p, ctx):
    """Welded turret whose ring sits off the hull center line (T-70: left).

    Like `plate_turret`, but the turret ring (and the `Turret` pivot) is
    moved sideways by `x`, so the turret traverses around its real ring.

    Parameters: `x` (ring center offset, - = left), `plan` [[x, y], ...]
    relative to the ring center, `height` (m or "fit"), `side_slope_deg`
    (default 10), `front_slope_deg` (extra inward slope of points with
    y > 0, default 0), `rear_slope_deg` (same for y < 0, default 0),
    `bevel`. Returns `turret_top_z`, `turret_plan`, `turret_top_plan`,
    `turret_x` (gun, hatches and MGs take model-space x).
    """
    xo = p.req("x")
    plan = [tuple(v) for v in p.req("plan")]
    h = ctx["turret_height"]
    z0 = ctx["turret_z"]
    ty = ctx["turret_y"]
    p.get("height")
    slope = math.tan(math.radians(p.get("side_slope_deg", 10.0)))
    front = math.tan(math.radians(p.get("front_slope_deg", 0.0)))
    rear = math.tan(math.radians(p.get("rear_slope_deg", 0.0)))
    top = offset_polygon(plan, h * slope)
    top = [(x, y - h * front if y > 0 else y + h * rear) for x, y in top]
    mb = model.mb("Turret", parent="Hull", pivot=(xo, ty, z0))
    rings = [[(xo + x, ty + y, z0) for x, y in plan], [(xo + x, ty + y, z0 + h) for x, y in top]]
    mb.loft(rings, "Paint", bevel=p.get("bevel", 0.015))
    ring_r = ctx.get("turret_ring_r", 0.8)
    mb.cylinder((xo, ty, z0 - 0.06), (xo, ty, z0 + 0.01), ring_r, seg=16, mat="Paint", cap0=True, cap1=False)
    return {"turret_top_z": z0 + h, "turret_plan": plan, "turret_top_plan": top, "turret_x": xo}


@part("su.turret_rear_mg")
def turret_rear_mg(model, p, ctx):
    """DT machine gun in a ball mount in the turret rear, firing backwards (IS-2).

    Parameters: `position` [x, y, z] (ball center, model space), `length`
    (default 1.14 = DT), `ball_diameter` (default 0.22). Adds `MG_NN`
    (child of Turret) and its `Socket_Muzzle_NN`, facing -Y.
    """
    x, y, z = p.req("position")
    length = p.get("length", 1.14)
    r = p.get("ball_diameter", 0.22) / 2
    name = model.next_index("MG_")
    mb = model.mb(name, parent="Turret", pivot=(x, y, z))
    mb.revolve((x, y + r * 0.6, z), (0, -1, 0), [(0, r * 0.4), (r * 0.4, r), (r * 1.2, r * 0.8), (r * 1.4, r * 0.3)],
               seg=10, mat="Paint")
    bl = length * 0.35
    mb.cylinder((x, y - r * 0.6, z), (x, y - r * 0.6 - bl, z), 0.016, seg=6, mat="Metal")
    tip = y - r * 0.6 - bl
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, (x, tip, z), matrix=Matrix.Rotation(math.pi, 4, "Z"))
    return {}


@part("su.bow_mg")
def bow_mg(model, p, ctx):
    """Fixed DT in the hull front, fired by the driver (IS-2): a short
    barrel stub in a small armored boss. Parameters: `position` [x, y, z]
    (boss center), `normal` [x, y, z] (plate normal, default +Y). Adds
    `MG_NN` (child of Hull) and `Socket_Muzzle_NN`.
    """
    x, y, z = p.req("position")
    nx, ny, nz = p.get("normal", [0.0, 1.0, 0.0])
    name = model.next_index("MG_")
    mb = model.mb(name, parent="Hull", pivot=(x, y, z))
    mb.cylinder((x - nx * 0.04, y - ny * 0.04, z - nz * 0.04), (x + nx * 0.05, y + ny * 0.05, z + nz * 0.05), 0.07,
                r1=0.05, seg=8, mat="Paint")
    tip = (x, y + 0.20, z)
    mb.cylinder((x, y, z), tip, 0.014, seg=6, mat="Metal")
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, tip)
    return {}


@part("su.m3a1_body")
def m3a1_body(model, p, ctx):
    """Body of the Lend-Lease M3A1 scout car for the `armored_car` assembly.

    The scout car shares the front end and body style of the US M3 half-track
    family (unditching roller, armored hood with radiator louvres, car-style
    front fenders, windshield armor, open-top body), so this part reuses
    `us.m3_body` unchanged and only supplies the front-axle context that the
    `us.halftrack` assembly would set.

    Parameters: `front_axle_y` (y of the front axle; the tyre radius comes
    from the axles built before, `axle_z`), all other parameters as
    `us.m3_body` (front-axle based coordinates). Returns what `us.m3_body`
    returns.
    """
    from .registry import build_part

    t = {k: p.get(k) for k in list(p.t)}  # passed on; us.m3_body checks them
    ctx["front_axle_y"] = t.pop("front_axle_y", 0.0)
    ctx["front_axle_z"] = ctx["axle_z"]
    return build_part("us.m3_body", model, t, ctx, where="su.m3a1_body")


@part("su.skate_rail")
def skate_rail(model, p, ctx):
    """Skate rail around an open compartment (M3 scout car, M2 halftrack):
    a tube loop on short posts along the top of the compartment walls.

    Parameters: `y` [front, rear], `half_width`, `z` (rail height), `posts`
    (posts per side, default 3), `post_z` (post root height, default z - 0.12),
    `diameter` (default 0.05). Builds into Hull.
    """
    yf, yr = p.req("y")
    hw = p.req("half_width")
    z = p.req("z")
    n = p.get("posts", 3)
    pz = p.get("post_z", z - 0.12)
    r = p.get("diameter", 0.05) / 2
    mb = model.mb("Hull")
    corners = [(-hw, yf), (hw, yf), (hw, yr), (-hw, yr)]
    for (x0, y0), (x1, y1) in zip(corners, corners[1:] + corners[:1]):
        mb.cylinder((x0, y0, z), (x1, y1, z), r, seg=6, mat="Metal")
    for side in (-1, 1):
        for i in range(n):
            yy = yf + (yr - yf) * i / max(1, n - 1)
            mb.cylinder((side * hw, yy, pz), (side * hw, yy, z), 0.02, seg=6, mat="Metal")
    return {}


@part("su.fuel_drums")
def fuel_drums(model, p, ctx):
    """Cylindrical external fuel tanks lying across or along the rear
    fenders (IS-2, ISU-152). Parameters: `positions` [[x, y, z], ...]
    (tank centers), `length` (default 0.7), `diameter` (default 0.36),
    `axis` "y" (along the hull, default) or "x". Builds into Hull.
    """
    L = p.get("length", 0.7)
    r = p.get("diameter", 0.36) / 2
    ax = p.get("axis", "y")
    mb = model.mb("Hull")
    for x, y, z in p.req("positions"):
        if ax == "y":
            a, b = (x, y - L / 2, z), (x, y + L / 2, z)
        else:
            a, b = (x - L / 2, y, z), (x + L / 2, y, z)
        mb.cylinder(a, b, r, seg=10, mat="Paint")
        for k in (-0.3, 0.3):
            if ax == "y":
                c0, c1 = (x, y + k * L - 0.015, z), (x, y + k * L + 0.015, z)
            else:
                c0, c1 = (x + k * L - 0.015, y, z), (x + k * L + 0.015, y, z)
            mb.cylinder(c0, c1, r * 1.04, seg=10, mat="Metal")
    return {}

