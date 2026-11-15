"""Towed gun carriages: split trails, axle, cradle, recoil system.

The `Carriage` is the root part of a towed gun; `Gun` (barrel, cradle, recoil
cylinders) elevates around X at the trunnion, `Shield` is a child of Carriage.
Model space: the barrel points +Y, trails run to -Y.
"""

import math

from .registry import part
from .weapons import barrel, muzzle_brake


@part("split_trail_carriage")
def split_trail_carriage(model, p, ctx):
    """Split-trail carriage with axle beam and trail legs.

    Parameters: `axle_y`, `wheel_diameter`, `wheel_width`, `wheel_track`
    (center distance), `trail_length` ("fit" = to the catalog overall length
    behind the muzzle), `trail_spread_deg` (0 = closed travel position),
    `trail_width`, `trail_height`, `spade` [width, height], `mount_z` (top of
    the upper carriage), `spokes` (wheel spokes, 0 = disc wheel).
    Adds `Carriage`, `Wheel_L_01`, `Wheel_R_01`, `Socket_Tow` at the trail end
    and returns `axle_z`, `carriage_rear_y`.
    """
    from .wheels import tire_wheel

    ay = p.req("axle_y")
    r = p.req("wheel_diameter") / 2
    ww = p.req("wheel_width")
    track = p.req("wheel_track")
    spread = math.radians(p.get("trail_spread_deg", 0.0))
    tw = p.get("trail_width", 0.14)
    th = p.get("trail_height", 0.2)
    spade_w, spade_h = p.get("spade", [0.35, 0.3])
    mount_z = p.get("mount_z", r + 0.25)
    tl = p.get("trail_length", "fit")
    if tl == "fit":
        tl = ctx["trail_length_fit"](ay)
    car = model.mb("Carriage")
    for side, s in ((-1, "L"), (1, "R")):
        x = side * track / 2
        mb = model.mb(f"Wheel_{s}_01", parent="Carriage", pivot=(x, ay, r))
        tire_wheel(mb, (x, ay, r), r, ww, side, spokes=p.get("spokes", 0))
    # axle beam and upper carriage
    car.box((0, ay, r), (track - ww, 0.16, 0.14), "Paint", bevel=0.01)
    car.box((0, ay + 0.05, (r + mount_z) / 2), (0.5, 0.45, mount_z - r + 0.1), "Paint", bevel=0.015)
    # trail legs: from the axle hinge to the spades
    hinge_x = 0.22
    for side in (-1, 1):
        ang = side * spread / 2
        end_x = side * (0.12 if spread == 0 else hinge_x) + math.sin(ang) * tl
        end_y = ay - math.cos(ang) * tl
        x0 = side * hinge_x
        z0, z1 = r - 0.02, 0.18
        # tapered box beam as a loft between two rectangles
        rings = []
        for (cx, cy, cz, w, h) in ((x0, ay - 0.1, z0, tw, th), (end_x, end_y + 0.03, z1, tw * 0.75, th * 0.6)):
            rings.append([(cx - w / 2, cy, cz - h / 2), (cx + w / 2, cy, cz - h / 2),
                          (cx + w / 2, cy, cz + h / 2), (cx - w / 2, cy, cz + h / 2)])
        car.loft(rings, "Paint", bevel=0.008)
        # spade at the trail end, digging into the ground
        car.box((end_x, end_y + 0.015, spade_h / 2), (spade_w, 0.03, spade_h), "Metal")
        # handle
        car.box((end_x, end_y + 0.3, z1 + th * 0.4), (0.04, 0.25, 0.04), "Metal")
    end_y = ay - tl
    car.box((0, end_y + 0.35, 0.22), (0.12, 0.3, 0.06), "Metal")  # towing eye bracket
    model.socket("Socket_Tow", "Carriage", (0.0, end_y + 0.2, 0.22))
    return {"axle_z": r, "carriage_rear_y": end_y, "mount_z": mount_z}


@part("field_gun")
def field_gun(model, p, ctx):
    """Barrel group of a towed gun: cradle, recoil cylinders, barrel, brake.

    Parameters: `trunnion` [y, z] (model space), `cradle` [length, width,
    height] (part behind and under the barrel), `recoil` [length, radius]
    (cylinders above/below the barrel, optional), `breech` [length, size],
    `segments` (barrel; one length may be "fit" when the assembly sets the
    catalog front `overall_front_y`, e.g. `aa_gun`), `muzzle_brake`,
    `parent` (default Carriage; Mount for anti-aircraft guns).
    Adds `Gun` (child of the parent) and `Socket_Muzzle`.
    """
    ty, tz = p.req("trunnion")
    mb = model.mb("Gun", parent=p.get("parent", "Carriage"), pivot=(0.0, ty, tz))
    cl, cw, ch = p.req("cradle")
    mb.box((0, ty + cl * 0.15, tz - ch * 0.35), (cw, cl, ch), "Paint", bevel=0.01)
    bl, bs = p.get("breech", [0.45, 0.22])
    y0 = ty - cl * 0.35
    mb.box((0, y0 - bl / 2 + 0.1, tz), (bs, bl, bs), "Metal", bevel=0.01)
    rec = p.get("recoil")
    if rec:
        mb.cylinder((0, y0, tz - 0.12), (0, y0 + rec[0], tz - 0.12), rec[1], seg=8, mat="Paint")
    segs = p.req("segments")
    brake = p.get("muzzle_brake", "none")
    if any(sg[0] == "fit" for sg in segs):
        from .weapons import _fit_segments

        if "overall_front_y" not in ctx:
            raise ValueError("field_gun: a 'fit' barrel segment needs an assembly that fits the barrel (aa_gun)")
        segs = _fit_segments(segs, y0 + 0.1, tz, brake, ctx)
    tip = barrel(mb, y0 + 0.1, tz, segs)
    tip = muzzle_brake(mb, tip, tz, segs[-1][2], brake)
    model.socket("Socket_Muzzle", "Gun", (0.0, tip, tz))
    return {"gun_tip_y": tip, "gun_z": tz}
