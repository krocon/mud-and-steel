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
    behind the muzzle in closed travel position), `trail_spread_deg` (0 =
    closed travel position on `Carriage`; > 0 = firing position with separate
    movable `Trail_L` / `Trail_R` child objects pivoted at the trail hinges),
    `trail_width`, `trail_height`, `spade` [width, height], `mount_z` (top of
    the upper carriage), `spokes` (wheel spokes, 0 = disc wheel).
    Adds `Carriage`, `Wheel_L_01`, `Wheel_R_01`, optional `Trail_L`/`Trail_R`,
    `Socket_Tow` at the closed trail end and returns `axle_z`,
    `carriage_rear_y`, `mount_z`.
    """
    from .wheels import tire_wheel

    ay = p.req("axle_y")
    r = p.req("wheel_diameter") / 2
    ww = p.req("wheel_width")
    track = p.req("wheel_track")
    spread_deg = float(p.get("trail_spread_deg", 0.0))
    spread = math.radians(spread_deg)
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
    hinge_y = ay - 0.1
    z0, z1 = r - 0.02, 0.18
    leg_len = max(tl - 0.1, 0.4)
    if spread_deg > 0.0:
        model.props["trail_spread_deg"] = round(spread_deg, 3)
    for side, s in ((-1, "L"), (1, "R")):
        ang = side * spread / 2
        x0 = side * hinge_x
        dx, dy = math.sin(ang), -math.cos(ang)
        px, py = math.cos(ang), math.sin(ang)
        if spread_deg > 0.0:
            tmb = model.mb(f"Trail_{s}", parent="Carriage", pivot=(x0, hinge_y, z0))
            end_x = x0 + dx * leg_len
            end_y = hinge_y + dy * leg_len
        else:
            tmb = car
            end_x = side * 0.12
            end_y = ay - tl
        # tapered box beam as a loft between two cross-sections perpendicular to the leg
        rings = []
        for (cx, cy, cz, w, h) in (
            (x0, hinge_y, z0, tw, th),
            (end_x - dx * 0.03, end_y - dy * 0.03, z1, tw * 0.75, th * 0.6),
        ):
            hw, hh = w / 2, h / 2
            rings.append([
                (cx - px * hw, cy - py * hw, cz - hh),
                (cx + px * hw, cy + py * hw, cz - hh),
                (cx + px * hw, cy + py * hw, cz + hh),
                (cx - px * hw, cy - py * hw, cz + hh),
            ])
        tmb.loft(rings, "Paint", bevel=0.008)
        # spade at the trail end, digging into the ground
        sp_c = (end_x - dx * 0.015, end_y - dy * 0.015, spade_h / 2)
        sp_rings = []
        for off in (-0.015, 0.015):
            cx, cy, cz = sp_c[0] + dx * off, sp_c[1] + dy * off, sp_c[2]
            hw, hh = spade_w / 2, spade_h / 2
            sp_rings.append([
                (cx - px * hw, cy - py * hw, cz - hh),
                (cx + px * hw, cy + py * hw, cz - hh),
                (cx + px * hw, cy + py * hw, cz + hh),
                (cx - px * hw, cy - py * hw, cz + hh),
            ])
        tmb.loft(sp_rings, "Metal")
        # lifting handle along the trail
        h_c = (end_x - dx * 0.30, end_y - dy * 0.30, z1 + th * 0.4)
        h_rings = []
        for off in (-0.125, 0.125):
            cx, cy, cz = h_c[0] + dx * off, h_c[1] + dy * off, h_c[2]
            h_rings.append([
                (cx - px * 0.02, cy - py * 0.02, cz - 0.02),
                (cx + px * 0.02, cy + py * 0.02, cz - 0.02),
                (cx + px * 0.02, cy + py * 0.02, cz + 0.02),
                (cx - px * 0.02, cy - py * 0.02, cz + 0.02),
            ])
        tmb.loft(h_rings, "Metal")
        if spread_deg > 0.0:
            # towing lunette lug at the inner rear tip; meets at x = 0 when folded
            lug_in = -side * (hinge_x * 0.5)
            l_c = (end_x - dx * 0.20 + px * lug_in, end_y - dy * 0.20 + py * lug_in, 0.22)
            l_rings = []
            for off in (-0.12, 0.12):
                cx, cy, cz = l_c[0] + dx * off, l_c[1] + dy * off, l_c[2]
                l_rings.append([
                    (cx - px * 0.04, cy - py * 0.04, cz - 0.03),
                    (cx + px * 0.04, cy + py * 0.04, cz - 0.03),
                    (cx + px * 0.04, cy + py * 0.04, cz + 0.03),
                    (cx - px * 0.04, cy - py * 0.04, cz + 0.03),
                ])
            tmb.loft(l_rings, "Metal")
    rear_y = (hinge_y - math.cos(spread / 2) * leg_len) if spread_deg > 0.0 else (ay - tl)
    if spread_deg == 0.0:
        car.box((0, ay - tl + 0.35, 0.22), (0.12, 0.3, 0.06), "Metal")  # towing eye bracket
    model.socket("Socket_Tow", "Carriage", (0.0, ay - tl + 0.2, 0.22))
    return {"axle_z": r, "carriage_rear_y": rear_y, "mount_z": mount_z}


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
