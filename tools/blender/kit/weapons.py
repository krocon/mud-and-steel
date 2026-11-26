"""Guns, mantlets, muzzle brakes, machine guns and gun shields.

The `Gun` object has its pivot at the trunnion and elevates around X. The
barrel points along +Y; `Socket_Muzzle` sits on the muzzle face.
"""

import math

from .registry import part


def muzzle_brake(mb, y, z, r, kind, x=0.0):
    """Adds a muzzle brake starting at y. Returns the new tip y.

    Kinds: none, double_baffle (German ball-shaped, KwK 40 / PaK 40),
    double_baffle_flat, single_baffle, slotted (cylinder with side ports),
    thread_protector (US/Soviet guns without brake: slight muzzle swell).
    """
    if kind == "none":
        return y
    if kind == "thread_protector":
        mb.cylinder((x, y, z), (x, y + 0.08, z), r * 1.15, seg=10, mat="Metal")
        return y + 0.08
    if kind in ("double_baffle", "double_baffle_flat"):
        L = r * 5.5
        R = r * (1.9 if kind == "double_baffle" else 1.6)
        prof = [(0.0, r * 1.05), (0.12 * L, R), (0.38 * L, R), (0.45 * L, r * 1.2), (0.55 * L, r * 1.2),
                (0.62 * L, R * 0.95), (0.92 * L, R * 0.95), (L, r * 1.1)]
        mb.revolve((x, y, z), (0, 1, 0), prof, seg=10, mat="Metal")
        # Dark recessed side baffle ports (prompt 215: readable as muzzle brake from RTS height)
        for fy in (0.25 * L, 0.77 * L):
            mb.box((x, y + fy, z), (R * 1.88, L * 0.16, R * 0.95), "Rubber")
        return y + L
    if kind == "single_baffle":
        L = r * 3.0
        R = r * 1.7
        mb.revolve((x, y, z), (0, 1, 0), [(0, r * 1.1), (0.2 * L, R), (0.8 * L, R), (L, r * 1.2)],
                   seg=10, mat="Metal")
        mb.box((x, y + 0.5 * L, z), (R * 1.88, L * 0.34, R * 0.95), "Rubber")
        return y + L
    if kind == "slotted":
        L = r * 4.0
        mb.cylinder((x, y, z), (x, y + L, z), r * 1.35, seg=10, mat="Metal")
        for k in (-1, 1):
            mb.box((x + k * r * 1.3, y + L * 0.5, z), (0.02, L * 0.6, r * 0.8), "Metal")
        return y + L
    raise ValueError(f"unknown muzzle brake {kind!r}")


def barrel(mb, start_y, z, segments, x=0.0, seg=10):
    """Barrel from segments [[length, r_start, r_end], ...]. Returns tip y."""
    y = start_y
    for length, r0, r1 in segments:
        mb.cylinder((x, y, z), (x, y + length, z), r0, r1=r1, seg=seg, mat="Paint", cap0=True, cap1=True)
        y += length
    return y


def mantlet(mb, kind, y, z, size, x=0.0):
    """Gun mantlet in front of the trunnion. Returns its front face y.

    Kinds: box (Panzer IV: flat plate + recoil sleeve), cast_box (T-34 cast
    mantlet with rounded front), rotor (Sherman M34A1: wide plate with
    cheeks), round (cast dome, small guns), saukopf (cast 'pig's head'
    cone tapering forward: StuG III late, Jagdpanzer 38(t)), ball (armored
    ball mount in a casemate front: SU-85, SU-100; `size` x = diameter).
    """
    sx, sy, sz = size
    if kind == "box":
        mb.box((x, y + sy / 2, z), (sx, sy, sz), "Paint", bevel=0.02)
        return y + sy
    if kind == "cast_box":
        mb.bevel_segments = 3
        mb.box((x, y + sy / 2, z), (sx, sy, sz), "Paint", bevel=min(sy, sz) * 0.3)
        mb.bevel_segments = 1
        return y + sy
    if kind == "rotor":
        mb.box((x, y + sy * 0.5, z), (sx, sy, sz), "Paint", bevel=0.03)
        for k in (-1, 1):
            mb.box((x + k * sx * 0.42, y + sy * 0.55, z), (sx * 0.18, sy * 1.1, sz * 1.05), "Paint", bevel=0.02)
        return y + sy
    if kind == "round":
        mb.revolve((x, y, z), (0, 1, 0), [(0, sx / 2), (sy * 0.6, sx / 2 * 0.9), (sy, sx / 4)], seg=12,
                   mat="Paint")
        return y + sy
    if kind == "saukopf":
        mb.revolve((x, y, z), (0, 1, 0), [(0, sx / 2), (sy * 0.35, sx / 2 * 0.92), (sy * 0.8, sx / 2 * 0.55),
                                          (sy, sx / 2 * 0.3)], seg=12, mat="Paint")
        return y + sy
    if kind == "ball":
        r = sx / 2
        mb.revolve((x, y - r * 0.4, z), (0, 1, 0), [(0, r * 0.5), (r * 0.4, r * 0.95), (r * 0.9, r),
                                                    (r * 1.3, r * 0.75), (r * 1.4, r * 0.35)], seg=14, mat="Paint")
        return y + r
    raise ValueError(f"unknown mantlet {kind!r}")


@part("tank_gun")
def tank_gun(model, p, ctx):
    """Main gun in a turret: mantlet, recoil sleeve, barrel, muzzle brake.

    Parameters:
    - `trunnion` [y, z]: relative to the turret ring center and turret base.
    - `mantlet` = { kind, size = [x, y, z] }, `sleeve` = [length, radius]
      (recoil sleeve/cradle in front of the mantlet, optional).
    - `segments` [[length, r0, r1], ...]: barrel tubes from the sleeve forward;
      one length may be "fit" (derived from the catalog overall length).
    - `muzzle_brake`: kind (see `muzzle_brake`).
    - `parent`: Turret (default) or Hull (casemate guns).
    - `x`: sideways offset of the gun (default 0; casemate guns mounted off
      center, e.g. Jagdpanzer 38(t)).
    Adds `Gun` and `Socket_Muzzle`.
    """
    parent = p.get("parent", "Turret")
    ty, tz = p.req("trunnion")
    gx = p.get("x", 0.0)
    y0 = ctx["turret_y"] + ty
    z = ctx["turret_z"] + tz
    mb = model.mb("Gun", parent=parent, pivot=(gx, y0, z))
    m = p.req("mantlet")
    y = mantlet(mb, m["kind"], y0 - m["size"][1] * 0.35, z, m["size"], x=gx)
    sleeve = p.get("sleeve")
    if sleeve:
        mb.cylinder((gx, y - 0.02, z), (gx, y + sleeve[0], z), sleeve[1], r1=sleeve[1] * 0.9, seg=12, mat="Paint")
        # Mantlet collar ring at the cradle-to-barrel transition (prompt 215)
        mb.cylinder((gx, y + sleeve[0] - 0.02, z), (gx, y + sleeve[0] + 0.05, z),
                    sleeve[1] * 0.96, r1=sleeve[1] * 0.72, seg=10, mat="Metal", cap0=False)
        y += sleeve[0]
    segs = p.req("segments")
    brake = p.get("muzzle_brake", "none")
    segs = _fit_segments(segs, y, z, brake, ctx)
    if not sleeve and segs and segs[0][0] >= 0.15:
        r0 = segs[0][1]
        collar_l = min(0.07, segs[0][0] * 0.4)
        mb.cylinder((gx, y - 0.02, z), (gx, y + collar_l, z), r0 * 1.45, r1=r0 * 1.15, seg=10, mat="Metal", cap0=False)
    tip = barrel(mb, y, z, segs, x=gx)
    tip = muzzle_brake(mb, tip, z, segs[-1][2], brake, x=gx)
    model.socket("Socket_Muzzle", "Gun", (gx, tip, z))
    return {"gun_tip_y": tip, "gun_z": z, "gun_x": gx}


def _fit_segments(segs, start_y, z, brake, ctx):
    """Resolves one "fit" length so that the muzzle reaches the catalog length."""
    fits = [i for i, s in enumerate(segs) if s[0] == "fit"]
    if not fits:
        return segs
    if len(fits) > 1:
        raise ValueError("tank_gun: only one barrel segment may be 'fit'")
    target_tip = ctx["overall_front_y"]
    fixed = sum(s[0] for s in segs if s[0] != "fit")
    r_end = segs[-1][2]
    brake_len = muzzle_brake_length(r_end, brake)
    length = target_tip - start_y - fixed - brake_len
    if length <= 0.05:
        raise ValueError(f"tank_gun: no room for the fitted barrel segment ({length:.3f} m)")
    out = [list(s) for s in segs]
    out[fits[0]][0] = length
    return out


def muzzle_brake_length(r, kind):
    return {"none": 0.0, "thread_protector": 0.08, "double_baffle": r * 5.5, "double_baffle_flat": r * 5.5,
            "single_baffle": r * 3.0, "slotted": r * 4.0}[kind]


def machine_gun(mb, x, y, z, length=1.1, r=0.012, jacket=True):
    """Simple MG: receiver, barrel (with perforated jacket look). Returns tip y."""
    mb.box((x, y - 0.25, z), (0.08, 0.5, 0.1), "Metal")
    if jacket:
        mb.cylinder((x, y, z), (x, y + length * 0.55, z), r * 2.4, seg=6, mat="Metal")
    mb.cylinder((x, y, z), (x, y + length * 0.75, z), r, seg=6, mat="Metal")
    return y + length * 0.75


@part("mg")
def mg(model, p, ctx):
    """Machine gun on a mount: coaxial, hull ball mount or roof pintle.

    Parameters: `mount` = "coax" | "ball" | "pintle", `position` [x, y, z]
    (model space; coax: relative to the gun trunnion), `length` (gun length),
    `ball_diameter` (ball mount), `jacket` (cooling jacket visible).
    Adds `MG_NN` (child of Turret for coax/pintle, Hull for ball) and
    `Socket_Muzzle_NN`.
    """
    mount = p.req("mount")
    px, py, pz = p.req("position")
    length = p.get("length", 1.2)
    jacket = p.get("jacket", True)
    name = model.next_index("MG_")
    if mount == "coax":
        base = model.pivot("Gun")
        x, y, z = base.x + px, base.y + py, base.z + pz
        parent = "Turret"
    elif mount == "ball":
        x, y, z = px, py, pz
        parent = "Hull"
    elif mount == "pintle":
        x, y, z = px, ctx["turret_y"] + py, ctx["turret_top_z"] + pz
        parent = "Turret"
    else:
        raise ValueError(f"mg: unknown mount {mount!r}")
    mb = model.mb(name, parent=parent, pivot=(x, y, z))
    if mount == "ball":
        r = p.get("ball_diameter", 0.22) / 2
        mb.revolve((x, y - r * 0.6, z), (0, 1, 0), [(0, r * 0.4), (r * 0.4, r), (r * 1.2, r * 0.8), (r * 1.4, r * 0.3)],
                   seg=10, mat="Paint")
        tip = machine_gun(mb, x, y + r * 0.6, z, length * 0.6, jacket=jacket)
    elif mount == "pintle":
        model.mb("Turret").cylinder((x, y, z - pz), (x, y, z - 0.05), 0.03, seg=6, mat="Metal")
        tip = machine_gun(mb, x, y, z, length, r=0.016, jacket=jacket)
    else:
        tip = y + 0.05
        mb.cylinder((x, y - 0.1, z), (x, tip, z), 0.012, seg=6, mat="Metal")
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, (x, tip, z))


@part("gun_shield")
def gun_shield(model, p, ctx):
    """Gun shield of a towed gun (spaced double plate on the PaK 40).

    Parameters: `y` (front plate position), `width`, `bottom_z`, `top_z`
    ("fit" = catalog height), `slope_deg` (top leans back), `cutout`
    [width, height] around the barrel, `wing_deg` (side wings angled back),
    `wing_width`, `spaced` (second plate behind), `thickness`.
    Adds `Shield` (child of Carriage).
    """
    y = p.req("y")
    w = p.req("width")
    zb = p.req("bottom_z")
    zt = ctx["height"] if p.get("top_z", "fit") == "fit" else p.get("top_z")
    slope = math.tan(math.radians(p.get("slope_deg", 15.0)))
    t = p.get("thickness", 0.012)
    cw, ch = p.get("cutout", [0.3, 0.25])
    cz = ctx["gun_z"]
    wing = p.get("wing_width", 0.25)
    wing_a = math.radians(p.get("wing_deg", 25.0))
    mb = model.mb("Shield", parent="Carriage", pivot=(0.0, y, zb))

    def plate(yf, inset):
        hw = w / 2 - math.cos(wing_a) * wing - inset  # wings end at the full width
        h = zt - zb
        dy = h * slope
        # main plate as two halves left and right of the barrel cutout, plus top part
        for side in (-1, 1):
            xo, xi = side * hw, side * cw / 2
            pts = [(xi, yf, zb), (xo, yf, zb), (xo, yf - dy, zt), (xi, yf - dy, zt)]
            mb.extrude(pts, (0, -t, 0), "Paint")
        pts = [(-cw / 2, yf - (cz + ch / 2 - zb) * slope, cz + ch / 2), (cw / 2, yf - (cz + ch / 2 - zb) * slope,
               cz + ch / 2), (cw / 2, yf - dy, zt), (-cw / 2, yf - dy, zt)]
        mb.extrude(pts, (0, -t, 0), "Paint")
        pts = [(-cw / 2, yf, zb), (cw / 2, yf, zb), (cw / 2, yf - (cz - ch / 2 - zb) * slope, cz - ch / 2),
               (-cw / 2, yf - (cz - ch / 2 - zb) * slope, cz - ch / 2)]
        mb.extrude(pts, (0, -t, 0), "Paint")
        # side wings angled back
        for side in (-1, 1):
            x0 = side * hw
            x1 = x0 + side * math.cos(wing_a) * wing
            y1 = yf - math.sin(wing_a) * wing
            pts = [(x0, yf, zb), (x1, y1, zb), (x1, y1 - dy, zt), (x0, yf - dy, zt)]
            mb.extrude(pts, (0, -t, 0), "Paint")

    plate(y, 0.0)
    if p.get("spaced", True):
        plate(y - 0.06, 0.03)
    return {"shield_y": y}
