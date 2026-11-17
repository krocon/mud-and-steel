"""Prompt 063: parts for the vehicles of the reference folder (docs/VEHICLE_REFS.md).

- `small_wheel_bogie`: small road wheels in bogies of 2-4 (T-26, Pz.Kpfw. 35(t),
  Pz.Kpfw. I, T-27, T-37A/T-38, Renault R35, Hotchkiss H39, Lorraine 37L, the
  skirted bogies of the T-35), optional outer girder (Pz.Kpfw. I).
- `hvss_bogie`: horizontal volute spring bogies with twin wide wheels (M4A3E8).
- `sub_turret`: secondary turret on the hull with a fixed gun and/or MGs
  (T-35, the machine-gun cupola of the M3 Lee); `GunTurret_NN`.
- `sponson_gun`: gun in a hull sponson with limited traverse (M3 Lee 75 mm),
  barrel fitted to the catalog length, muzzle socket `Socket_Gun_NN`.
- `amphibious_floats`: buoyancy boxes along the hull sides over the tracks
  (T-37A, T-38, T-40); `screw_propeller`: propeller with rudder at the rear.

Open superstructures of the captured self-propelled guns (Marder I, Lorraine,
H39, R35 conversions) use the existing `casemate` part with `open = true`.
"""

import math

from . import wheels as W
from .registry import part
from .running_gear import SIDES, Gear, _twin
from .turret import offset_polygon
from .weapons import barrel, mantlet, muzzle_brake, muzzle_brake_length


@part("small_wheel_bogie")
def small_wheel_bogie(model, p, ctx):
    """Small road wheels sprung in bogies (Vickers / Skoda / Renault type);
    also large single wheels in pairs (Pz.Kpfw. 38(t), Elefant).

    Uses the common running-gear parameters. Extra parameters:
    - `per_bogie` (wheels per bogie, default 2; consecutive stations front to
      back) or `bogies` [[i, j, ...], ...] (station indices, 0 = front-most
      in `road_wheels` order) for uneven groups,
    - `spring` = "leaf" (quarter-elliptic leaf spring over the bogie, T-26,
      35(t)), "coil" (vertical coil springs, T-37A, R35 'scissors' look),
      "none" (default "leaf"),
    - `girder` (outer beam along the wheel hubs holding the rear wheels,
      Pz.Kpfw. I; default false), `twin` (two discs per station, default
      false), `tire` (default "Rubber").
    """
    per = p.get("per_bogie", 2)
    groups = p.get("bogies")
    spring = p.get("spring", "leaf")
    girder = p.get("girder", False)
    twin = p.get("twin", False)
    tire = p.get("tire", "Rubber")
    if spring not in ("leaf", "coil", "none"):
        raise ValueError(f"small_wheel_bogie: unknown spring {spring!r}")
    g = Gear(p, ctx)
    order = list(g.stations)
    if groups is None:
        groups = [list(range(i, min(i + per, len(order)))) for i in range(0, len(order), per)]
    for grp in groups:
        if any(i < 0 or i >= len(order) for i in grp):
            raise ValueError(f"small_wheel_bogie: bogie {grp} has an index outside road_wheels")
    hull = model.mb("Hull")

    def wheel(mb, c, sd):
        if twin:
            _twin(g, mb, c, sd, tire=tire)
        else:
            W.disc_wheel(mb, c, g.rw_r, g.rw_width, sd, tire=tire, hub=0.3)

    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, wheel)
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.05)
        z = g.wheel_z
        for grp in groups:
            ys = [order[i] for i in grp]
            mid = (max(ys) + min(ys)) / 2
            span = max(ys) - min(ys) + g.rw_r * 2
            # bogie beam inside the wheels, pivot bracket to the hull
            hull.box((xi, mid, z + 0.02), (0.08, span * 0.8, 0.10), "Paint", bevel=0.008)
            hull.box((xi - side * 0.09, mid, z + g.rw_r * 0.9), (0.12, 0.18, g.rw_r * 1.4), "Paint")
            if spring == "leaf":
                hull.box((xi, mid, z + g.rw_r + 0.06), (0.08, span * 0.85, 0.04), "Metal")
            elif spring == "coil":
                for y in ys:
                    hull.cylinder((xi, y, z + 0.05), (xi, y, z + g.rw_r + 0.12), 0.045, seg=8, mat="Metal")
        if girder:
            ys = sorted(order)
            hull.box((x + side * (g.rw_width / 2 + 0.03), (ys[0] + ys[-1]) / 2, z),
                     (0.05, ys[-1] - ys[0] + 0.2, 0.12), "Paint", bevel=0.01)
    return g.result(path)


@part("hvss_bogie")
def hvss_bogie(model, p, ctx):
    """Horizontal volute spring suspension: bogies of two twin wheel stations
    with the springs lying between the wheel arms and a return roller on each
    bogie (M4A3E8 'Easy Eight', M4A3(76)W HVSS).

    Uses the common parameters (`road_wheels` with an even count; the
    stations pair up front to back). Extra parameters: `roller_offset` (Y
    offset of the roller on its bogie, default 0.0), `return_roller_diameter`
    (default 0.22). `return_rollers` is ignored (rollers sit on the bogies).
    """
    off = p.get("roller_offset", 0.0)
    rr_d = p.get("return_roller_diameter", 0.22)
    p.t.pop("return_rollers", None)
    g = Gear(p, ctx)
    ys = sorted(g.stations)
    if len(ys) % 2:
        raise ValueError("hvss_bogie: road_wheels needs an even count")
    bogies = [((a + b) / 2, abs(b - a)) for a, b in zip(ys[0::2], ys[1::2])]
    g.rollers = [(mid + off, g.wheel_z + g.rw_r + 0.26, rr_d / 2) for mid, _ in bogies]
    hull = model.mb("Hull")
    path = None
    for side, s in SIDES:
        path = g.build_common(model, side, s, lambda mb, c, sd: _twin(g, mb, c, sd), rollers_as_objects=True)
        x = g.x(side)
        xi = x - side * (g.tw / 2 + 0.05)
        z = g.wheel_z
        for mid, span in bogies:
            # bogie frame bolted to the hull, horizontal spring between the arms
            hull.box((xi, mid, z + 0.16), (0.16, span * 0.5, 0.26), "Paint", bevel=0.012)
            hull.cylinder((x, mid - span * 0.32, z + 0.08), (x, mid + span * 0.32, z + 0.08), 0.06, seg=8,
                          mat="Metal")
            for k in (-1, 1):
                hull.box((x, mid + k * span * 0.28, z + 0.02), (g.tw * 0.16, span * 0.36, 0.08), "Paint")
            # shock absorber, roller bracket
            hull.cylinder((xi, mid - span * 0.2, z + 0.32), (xi, mid + span * 0.15, z - 0.02), 0.035, seg=6,
                          mat="Metal")
            hull.box((xi + side * 0.04, mid + off, z + 0.36), (0.09, 0.10, 0.20), "Paint")
    return g.result(path)


@part("sub_turret")
def sub_turret(model, p, ctx):
    """Secondary turret on the hull roof or on the main turret (multi-turret
    tanks T-35, T-28, Neubaufahrzeug; the MG cupola of the M3 Lee).

    Parameters: `position` [x, y] (ring center, model space), `z` (ring
    height, default hull top), `diameter` (round turret) or `plan` [[x, y],
    ...] (relative to the ring center), `height`, `side_slope_deg` (default
    8), `parent` (default Hull; "Turret" for a cupola turret on the main
    turret), `gun` = { length, radius, trunnion = [y, z] (relative to the
    ring center and base), `direction_deg` (yaw, + = to the left, default 0),
    `mantlet` = [w, l, h] (optional box) } for a fixed small gun (part of
    the turret mesh), `mgs` (number of machine guns, default 0; each one an
    `MG_NN` child with `Socket_Muzzle_NN`), `mg_direction_deg` (default =
    gun direction), `hatch` (small roof hatch, default true).
    Adds `GunTurret_NN` (rotates around Z, pivot at the ring center).
    Returns `sub_turret_top_z`.
    """
    px, py = p.req("position")
    z0 = p.get("z", ctx.get("hull_top_z", 1.0))
    h = p.req("height")
    slope = math.tan(math.radians(p.get("side_slope_deg", 8.0)))
    parent = p.get("parent", "Hull")
    name = model.next_index("GunTurret_")
    mb = model.mb(name, parent=parent, pivot=(px, py, z0))
    if "plan" in p:
        plan = [tuple(v) for v in p.req("plan")]
        p.get("diameter")
        top = offset_polygon(plan, h * slope)
        rings = [[(px + x, py + y, z0) for x, y in plan], [(px + x, py + y, z0 + h) for x, y in top]]
        mb.loft(rings, "Paint", bevel=0.01)
        r = max(math.hypot(x, y) for x, y in plan)
    else:
        r = p.req("diameter") / 2
        mb.cylinder((px, py, z0), (px, py, z0 + h), r, r1=max(0.05, r - h * slope), seg=14, mat="Paint")
    mb.cylinder((px, py, z0 - 0.04), (px, py, z0 + 0.01), r * 0.92, seg=14, mat="Paint", cap0=True, cap1=False)
    if p.get("hatch", True):
        mb.cylinder((px, py - r * 0.25, z0 + h), (px, py - r * 0.25, z0 + h + 0.03), r * 0.4, seg=10, mat="Paint")
    yaw = math.radians(p.get("gun", {}).get("direction_deg", 0.0) if "gun" in p else 0.0)
    gun = p.get("gun")
    if gun:
        gy, gz = gun.get("trunnion", [r * 0.6, h * 0.5])
        length = gun["length"]
        rad = gun["radius"]
        d = (-math.sin(yaw), math.cos(yaw))
        sx, sy = px + d[0] * gy, py + d[1] * gy
        zz = z0 + gz
        if gun.get("mantlet"):
            mw, ml, mh = gun["mantlet"]
            mb.box((sx + d[0] * ml / 2, sy + d[1] * ml / 2, zz), (mw if abs(d[1]) > 0.7 else ml,
                                                                  ml if abs(d[1]) > 0.7 else mw, mh), "Paint")
        mb.cylinder((sx, sy, zz), (sx + d[0] * length, sy + d[1] * length, zz), rad, r1=rad * 0.9, seg=8,
                    mat="Paint")
    mg_yaw = math.radians(p.get("mg_direction_deg", math.degrees(yaw)))
    n = p.get("mgs", 0)
    for i in range(n):
        off = (i - (n - 1) / 2) * 0.14
        d = (-math.sin(mg_yaw), math.cos(mg_yaw))
        bx = px + d[0] * r * 0.85 + d[1] * off
        by = py + d[1] * r * 0.85 - d[0] * off
        bz = z0 + h * 0.45
        mg_name = model.next_index("MG_")
        mmb = model.mb(mg_name, parent=name, pivot=(bx, by, bz))
        mmb.revolve((bx, by, bz), (d[0], d[1], 0.0), [(0, 0.05), (0.04, 0.07), (0.1, 0.05)], seg=8, mat="Paint")
        tip = (bx + d[0] * 0.55, by + d[1] * 0.55, bz)
        mmb.cylinder((bx, by, bz), tip, 0.014, seg=6, mat="Metal")
        model.socket(f"Socket_Muzzle_{int(mg_name[3:]) + 1:02d}", mg_name, tip)
    return {"sub_turret_top_z": z0 + h}


@part("sponson_gun")
def sponson_gun(model, p, ctx):
    """Gun in a hull sponson with limited traverse (M3 Lee / Grant 75 mm).

    Parameters: `x` (gun axis offset, + = right), `y` (front face of the
    sponson), `z` (gun axis height), `sponson` = [w, l, h] (armored box
    around the mount, ending at `y`; optional), `cast` (rounded sponson,
    default false), `mantlet` = { kind, size } (see weapons.mantlet,
    optional), `segments` [[length, r0, r1], ...] with one length "fit"
    (the muzzle then reaches the catalog overall length, measured from the
    rear-most point built so far: list the sponson gun before parts that
    stick out at the rear, such as the exhaust), `muzzle_brake` (default "none"),
    `parent` (default Hull). Adds the geometry to the parent mesh (the
    sponson gun does not move) and `Socket_Gun_NN` on the muzzle. Returns
    `sponson_tip_y`.
    """
    x = p.req("x")
    y = p.req("y")
    z = p.req("z")
    parent = p.get("parent", "Hull")
    mb = model.mb(parent)
    size = p.get("sponson")
    if size:
        w, ln, hh = size
        if p.get("cast", False):
            mb.bevel_segments = 3
            mb.box((x, y - ln / 2, z), (w, ln, hh), "Paint", bevel=min(w, hh) * 0.25)
            mb.bevel_segments = 1
        else:
            mb.box((x, y - ln / 2, z), (w, ln, hh), "Paint", bevel=0.02)
    yy = y
    m = p.get("mantlet")
    if m:
        yy = mantlet(mb, m["kind"], y - m["size"][1] * 0.3, z, m["size"], x=x)
    segs = [list(s) for s in p.req("segments")]
    brake = p.get("muzzle_brake", "none")
    fits = [i for i, s in enumerate(segs) if s[0] == "fit"]
    if len(fits) > 1:
        raise ValueError("sponson_gun: only one barrel segment may be 'fit'")
    if fits:
        lo, _ = model.extent()
        target = lo.y + ctx["length"]
        fixed = sum(s[0] for s in segs if s[0] != "fit")
        length = target - yy - fixed - muzzle_brake_length(segs[-1][2], brake)
        if length <= 0.05:
            raise ValueError(f"sponson_gun: no room for the fitted barrel segment ({length:.3f} m)")
        segs[fits[0]][0] = length
    tip = barrel(mb, yy, z, segs, x=x)
    tip = muzzle_brake(mb, tip, z, segs[-1][2], brake, x=x)
    model.socket(model.next_index("Socket_Gun_"), parent, (x, tip, z))
    return {"sponson_tip_y": tip}


@part("amphibious_floats")
def amphibious_floats(model, p, ctx):
    """Buoyancy boxes along both hull sides above the tracks (T-37A, T-38:
    float-filled fenders; T-40: hull sponsons).

    Parameters: `y` [front, rear], `z` [bottom, top], `width` (outward
    from `inner_x`), `inner_x` (default: inner track edge from the running
    gear), `taper` (front and rear ends drop by this many meters at the
    bottom, default 0.1). Adds to Hull.
    """
    yf, yr = p.req("y")
    zb, zt = p.req("z")
    w = p.req("width")
    xi = p.get("inner_x", ctx.get("track_inner_x", 0.8))
    taper = p.get("taper", 0.1)
    mb = model.mb("Hull")
    prof = [(yr, zb + taper), (yr + taper * 2, zb), (yf - taper * 2, zb), (yf, zb + taper), (yf, zt), (yr, zt)]
    for side in (-1, 1):
        x0, x1 = sorted((side * xi, side * (xi + w)))
        mb.extrude_x(prof, x0, x1, "Paint", bevel=0.01)


@part("screw_propeller")
def screw_propeller(model, p, ctx):
    """Water propeller at the rear of an amphibious tank, with an optional
    rudder behind it (T-37A, T-38, T-40).

    Parameters: `position` [x, y, z] (hub), `diameter`, `blades` (default
    3), `rudder` (default true), `tunnel` (half-pipe guard above the
    propeller, default false). Adds to Hull (the propeller is not animated).
    """
    x, y, z = p.req("position")
    d = p.req("diameter")
    n = p.get("blades", 3)
    mb = model.mb("Hull")
    mb.cylinder((x, y + 0.25, z), (x, y - 0.08, z), 0.04, seg=8, mat="Metal")
    mb.cylinder((x, y, z), (x, y - 0.1, z), 0.06, seg=8, mat="Metal")
    for i in range(n):
        a = 2 * math.pi * i / n
        cx, cz = x + math.cos(a) * d / 4, z + math.sin(a) * d / 4
        mb.box((cx, y - 0.05, cz), (abs(math.cos(a)) * d / 2 + 0.02, 0.02, abs(math.sin(a)) * d / 2 + 0.02), "Metal")
    if p.get("rudder", True):
        mb.box((x, y - 0.18 - d * 0.15, z), (0.02, d * 0.3, d * 0.9), "Metal")
    if p.get("tunnel", False):
        mb.box((x, y - 0.05, z + d / 2 + 0.03), (d * 1.1, 0.2, 0.02), "Paint")
