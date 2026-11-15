"""US faction parts (prefix `us.`)."""

from .registry import assembly, build_part, part


@part("us.transmission_cover")
def transmission_cover(model, p, ctx):
    """Rounded cast or three-piece transmission cover at the Sherman's nose.

    Parameters: `y` (axis of the rounding), `z`, `radius`, `width` (default
    lower hull width + 0.1), `segments`.
    """
    y, z = p.req("y"), p.req("z")
    r = p.req("radius")
    w = p.get("width", ctx["track_inner_x"] * 2)
    seg = p.get("segments", 12)
    mb = model.mb("Hull")
    mb.cylinder((-w / 2, y, z), (w / 2, y, z), r, seg=seg, mat="Paint", smooth=True)
    # bolted flange on top
    mb.box((0, y - r * 0.2, z + r * 0.92), (w, 0.12, 0.05), "Paint")


@part("us.hatch_hoods")
def hatch_hoods(model, p, ctx):
    """Driver and assistant driver hatches with periscope hoods on the glacis
    top (M4A3 large-hatch hull). Parameters: `x`, `y`, `size` [x, y]; adds
    two Hatch_NN objects and Socket_Hatch_NN."""
    x, y = p.req("x"), p.req("y")
    sx, sy = p.get("size", [0.55, 0.6])
    z = ctx["hull_top_z"]
    for side in (-1, 1):
        cx = side * x
        name = model.next_index("Hatch_")
        mb = model.mb(name, parent="Hull", pivot=(cx + side * sx / 2, y, z))
        mb.box((cx, y, z + 0.025), (sx, sy, 0.05), "Paint", bevel=0.01)
        mb.box((cx - side * sx * 0.15, y + sy * 0.25, z + 0.09), (0.16, 0.1, 0.08), "Glass")
        model.socket(model.next_index("Socket_Hatch_"), "Hull", (cx, y, z))


# --- M3 half-track (prompt 006) ------------------------------------------------------
# The generic kit has no half-track assembly yet (only the `halftrack_rear` track
# unit and `wheel_axle`). This one lives here so that the parallel faction sessions
# do not touch the shared `assemblies.py`; a generic `halftrack` assembly is
# proposed in the prompt 006 report.


def _arc(cy, cz, r, a0, a1, n):
    import math

    return [(cy + math.cos(math.radians(a0 + (a1 - a0) * i / n)) * r,
             cz + math.sin(math.radians(a0 + (a1 - a0) * i / n)) * r) for i in range(n + 1)]


@part("us.m3_body")
def m3_body(model, p, ctx):
    """Body of the US M3 half-track family: unditching roller, bumper, chassis
    rails, armored hood with radiator louvres, car-style front fenders with
    running boards, open-top armored body with cab doors, windshield armor
    plate with vision slits, rear door, floor and bench seats.

    Coordinates: y forward from the front axle (the assembly moves the origin
    to the footprint center afterwards), z up from the ground.

    Parameters:
    - `roller` = { y, z, diameter, width }: unditching roller at the front.
    - `bumper` = { y, z }: front bumper (full width of the fenders).
    - `rails` = { half_x, z, height }: chassis frame rails.
    - `hood` = { front_y, rear_y, half_width, bottom_z, front_top_z, top_z,
      louvre_deg }: engine hood; the radiator louvre armor leans back by
      `louvre_deg` from vertical.
    - `fender` = { outer_x, radius, thickness, board_z, rear_y }: arched
      fender over the front wheel (`radius` around the wheel center),
      running board at `board_z` back to `rear_y`.
    - `body` = { front_y, rear_y ("fit" = catalog length behind the roller),
      bottom_z, top_z, wall, door = [rear_y, top_z], floor_z }: open-top body,
      side walls end at the catalog half width.
    - `windshield` = { top_z, deg, slits }: armor plate over the windshield,
      leaning back `deg` from vertical, `slits` vision slits.
    - `seats`: true adds bench seats along the body sides and front seats.
    Returns `body_rear_y`, `body_front_y`, `body_top_z`, `floor_z`, `hull_top_z`.
    """
    import math

    hw = ctx["half_width"]
    hull = model.mb("Hull")
    ro = p.req("roller")
    rr = ro["diameter"] / 2
    front_tip = ro["y"] + rr
    # roller drum and side brackets
    hull.cylinder((-ro["width"] / 2, ro["y"], ro["z"]), (ro["width"] / 2, ro["y"], ro["z"]), rr, seg=12,
                  mat="Paint")
    for k in (-1, 1):
        hull.box((k * (ro["width"] / 2 + 0.04), ro["y"] - 0.2, ro["z"]), (0.06, 0.5, 0.12), "Paint", bevel=0.01)
    bu = p.req("bumper")
    fe = p.req("fender")
    hull.box((0, bu["y"], bu["z"]), (fe["outer_x"] * 2, 0.10, 0.14), "Paint", bevel=0.01)

    bo = p.req("body")
    rear = bo["rear_y"]
    if rear == "fit":
        rear = front_tip - ctx["length"]
    front = bo["front_y"]
    bz, tz = bo["bottom_z"], bo["top_z"]
    wall = bo.get("wall", 0.02)
    door_rear, door_top = bo["door"]
    floor_z = bo["floor_z"]

    rails = p.req("rails")
    for k in (-1, 1):
        hull.box((k * rails["half_x"], (bu["y"] + rear) / 2, rails["z"]), (0.10, bu["y"] - rear, rails["height"]),
                 "Paint")

    # hood with sloped radiator louvre armor
    ho = p.req("hood")
    hy0, hy1 = ho["front_y"], ho["rear_y"]
    hb, hft, ht = ho["bottom_z"], ho["front_top_z"], ho["top_z"]
    dy = (hft - hb) * math.tan(math.radians(ho["louvre_deg"]))
    hhw = ho["half_width"]
    hull.extrude_x([(hy1, hb), (hy0, hb), (hy0 - dy, hft), (hy1, ht)], -hhw, hhw, "Paint", bevel=0.015)
    # louvre slats on the radiator armor
    n = 6
    for i in range(n):
        t = (i + 0.5) / n
        hull.box((0, hy0 - dy * t + 0.012, hb + (hft - hb) * t), (hhw * 1.7, 0.02, 0.025), "Metal")
    # hood side hinges / top seam
    hull.box((0, (hy0 + hy1) / 2 - dy / 2, ht + 0.005), (0.04, hy0 - hy1 - dy, 0.012), "Metal")

    # front fenders over the wheels and running boards
    wy, wz = ctx["front_axle_y"], ctx["front_axle_z"]
    rf, th = fe["radius"], fe.get("thickness", 0.03)
    outer = _arc(wy, wz, rf, 10, 170, 10)
    inner = list(reversed(_arc(wy, wz, rf - th, 10, 170, 10)))
    for k in (-1, 1):
        xa, xb = sorted((k * hhw, k * fe["outer_x"]))
        hull.extrude_x(outer + inner, xa, xb, "Paint")
        # running board from the fender to the body
        y0 = wy - rf * math.cos(math.radians(10))
        hull.box((k * (hhw + fe["outer_x"]) / 2, (y0 + fe["rear_y"]) / 2, fe["board_z"]),
                 (fe["outer_x"] - hhw, y0 - fe["rear_y"], 0.03), "Paint")
        # valance between hood and fender
        hull.box((k * (hhw + 0.01), (hy0 + hy1) / 2, (hb + wz) / 2), (0.02, hy0 - hy1, hb - wz + 0.3), "Paint")

    # open-top body: side walls with the cab door cut-out, front dash, rear wall, floor
    prof = [(rear, bz), (front, bz), (front, door_top), (door_rear, door_top), (door_rear, tz), (rear, tz)]
    for k in (-1, 1):
        xa, xb = sorted((k * (hw - wall), k * hw))
        hull.extrude_x(prof, xa, xb, "Paint", bevel=0.004)
    hull.box((0, front - wall / 2, (bz + ht) / 2), (hw * 2, wall, ht - bz), "Paint")
    hull.box((0, rear + wall / 2, (bz + tz) / 2), (hw * 2, wall, tz - bz), "Paint", bevel=0.004)
    hull.box((0, rear + wall + 0.01, (bz + tz) / 2), (0.62, 0.02, tz - bz - 0.12), "Paint")  # rear door
    for k in (-1, 1):  # door hinges
        hull.box((k * 0.33, rear - 0.01, tz - 0.25), (0.06, 0.03, 0.08), "Metal")
        hull.box((k * 0.33, rear - 0.01, bz + 0.2), (0.06, 0.03, 0.08), "Metal")
    hull.box((0, (front + rear) / 2, floor_z - 0.02), (hw * 2 - 2 * wall, front - rear, 0.04), "Paint")

    # windshield armor plate (lowered, with vision slits) on the cowl
    ws = p.req("windshield")
    wtop = ws["top_z"]
    wdy = (wtop - ht) * math.tan(math.radians(ws["deg"]))
    pts = [(-hw, front, ht), (hw, front, ht), (hw, front - wdy, wtop), (-hw, front - wdy, wtop)]
    hull.extrude(pts, (0, -wall, 0), "Paint", bevel=0.004)
    slits = ws.get("slits", 2)
    for i in range(slits):
        x = -hw * 0.5 + (hw * (i / max(slits - 1, 1))) if slits > 1 else 0.0
        zc = ht + (wtop - ht) * 0.55
        hull.box((x, front - wdy * 0.55 + 0.012, zc), (0.36, 0.02, 0.035), "Glass")
    # windshield frame posts
    for k in (-1, 1):
        hull.box((k * (hw - 0.03), front - wdy / 2 - 0.03, (ht + wtop) / 2), (0.05, 0.05, wtop - ht), "Paint")

    if p.get("seats", True):
        seat_z = floor_z + 0.42
        for k in (-1, 1):
            hull.box((k * (hw - wall - 0.22), (door_rear - 0.2 + rear + 0.15) / 2, seat_z - 0.05),
                     (0.40, door_rear - 0.35 - rear, 0.10), "Canvas")
            hull.box((k * (hw - wall - 0.04), (door_rear - 0.2 + rear + 0.15) / 2, seat_z + 0.25),
                     (0.06, door_rear - 0.35 - rear, 0.40), "Canvas")
        for x in (-0.55, 0.0, 0.55):
            hull.box((x, front - 0.65, seat_z - 0.05), (0.42, 0.42, 0.10), "Canvas")
            hull.box((x, front - 0.88, seat_z + 0.22), (0.42, 0.06, 0.45), "Canvas")
    return {"body_rear_y": rear, "body_front_y": front, "body_top_z": tz, "floor_z": floor_z,
            "hull_top_z": tz, "hull_front_y": front_tip, "hull_rear_y": rear}


@part("us.pedestal_mg")
def pedestal_mg(model, p, ctx):
    """Machine gun on a pedestal mount in an open body (M25 pedestal of the M3).

    Parameters: `position` [x, y] (front-axle coordinates like the body),
    `length` (gun length), `jacket`, `top_z` (top of the gun receiver; "fit"
    = catalog height, i.e. the catalog height is measured over the gun).
    Adds `MG_NN` (child of Hull) with `Socket_Muzzle_NN`, and `Socket_Muzzle`
    at the same muzzle: the machine gun is the vehicle's main weapon.
    """
    from .weapons import machine_gun

    x, y = p.req("position")
    top = p.get("top_z", "fit")
    top = ctx["height"] if top == "fit" else top
    z = top - 0.05
    floor = ctx["floor_z"]
    hull = model.mb("Hull")
    hull.cylinder((x, y, floor), (x, y, z - 0.10), 0.05, r1=0.035, seg=8, mat="Paint")
    hull.cylinder((x, y, floor), (x, y, floor + 0.03), 0.18, seg=8, mat="Paint")
    name = model.next_index("MG_")
    mb = model.mb(name, parent="Hull", pivot=(x, y, z - 0.08))
    mb.cylinder((x, y, z - 0.10), (x, y, z - 0.04), 0.035, seg=6, mat="Metal")  # cradle
    tip = machine_gun(mb, x, y + 0.15, z, p.get("length", 1.04), jacket=p.get("jacket", True))
    mb.box((x, y + 0.05, z - 0.12), (0.18, 0.18, 0.02), "Metal")  # ammunition tray
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, (x, tip, z))
    model.socket("Socket_Muzzle", name, (x, tip, z))


@assembly("us.halftrack")
def us_halftrack(model, bp, u, tune):
    """US half-track (M3 family): `[layout]`, `[running_gear]` (halftrack_rear),
    `[front_axle]` (wheel_axle), `[body]` (us.m3_body), `[[part]]`, `[[mg]]`.

    `[layout]`: `fender_overhang` (catalog half width minus the outer track
    edge). Coordinates are front-axle based (front axle at y = 0); after the
    build the model is moved so that the origin lies at the center of the
    ground contact (front tires to rear of the track). The body rear is fitted
    to the catalog length (`us.m3_body`), the pedestal MG to the catalog height.
    """
    from .assemblies import _base_ctx, _run_list, _table

    ctx, layout = _base_ctx(bp, u, tune)
    ctx["track_outer_x"] = ctx["half_width"] - layout.get("fender_overhang", 0.0)
    model.mb("Hull")
    name, t = _table(bp["running_gear"], "[running_gear]")
    build_part(name, model, t, ctx, where="[running_gear]")
    name, t = _table(bp["front_axle"], "[front_axle]")
    build_part(name, model, t, ctx, where="[front_axle]")
    ctx["front_axle_y"] = t["y"]
    ctx["front_axle_z"] = t["diameter"] / 2
    name, t = _table(bp["body"], "[body]")
    build_part(name, model, t, ctx, where="[body]")
    _run_list(model, bp, "part", ctx)
    _run_list(model, bp, "mg", ctx)
    # origin at the center of the ground contact
    ys, xs = [], []
    for part_ in model.parts.values():
        for v in part_["mb"].bm.verts:
            if v.co.z < 0.03:
                ys.append(v.co.y)
                xs.append(v.co.x)
    cy = (min(ys) + max(ys)) / 2
    cx = (min(xs) + max(xs)) / 2
    model.translate((-cx, -cy, 0.0))
    return ctx
