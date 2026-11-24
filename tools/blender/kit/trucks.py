"""Soft-skin wheeled vehicles: trucks and cars (prompt 006).

Generic parts for a ladder-frame vehicle with engine hood, cab, cargo body and
fenders, plus the `wheeled` assembly. Model space as everywhere: Z up, front
+Y, meters. All objects are children of `Hull` except the wheels
(`Wheel_<L|R>_NN`, from the `wheel_axle` part in `wheels.py`).
"""

import math

from .catalog import mm
from .registry import assembly, build_part, part


def _req_pair(p, key):
    a, b = p.req(key)
    return min(a, b), max(a, b)


@part("ladder_frame")
def ladder_frame(model, p, ctx):
    """Two frame rails with cross members (trucks).

    Parameters: `y` [rear, front], `z` (top of the rails), `spacing` (center
    distance of the rails), `rail` [width, height], `cross_members` (count,
    evenly spaced, default 5).
    """
    y0, y1 = _req_pair(p, "y")
    z = p.req("z")
    s = p.req("spacing")
    rw, rh = p.get("rail", [0.08, 0.18])
    n = p.get("cross_members", 5)
    mb = model.mb("Hull")
    for side in (-1, 1):
        mb.box((side * s / 2, (y0 + y1) / 2, z - rh / 2), (rw, y1 - y0, rh), "Paint")
    for i in range(n):
        y = y0 + 0.1 + (y1 - y0 - 0.2) * i / max(n - 1, 1)
        mb.box((0, y, z - rh / 2), (s, 0.07, rh * 0.7), "Paint")
    return {"frame_z": z, "frame_rear_y": y0, "frame_front_y": y1}


@part("engine_hood")
def engine_hood(model, p, ctx):
    """Engine hood with radiator shell in front (classic long-nose truck).

    Parameters: `y` [rear (cowl), front (radiator face)], `width`, `bottom_z`,
    `top_z`, `crown` (rounding of the top edge, m), `radiator` [width, height]
    of the grille shell (default hood width and height + 4 cm), `louvers`
    (vent slots per side, default 0), `cap` (radiator filler cap, default true).
    Returns `hood_front_y`, `hood_rear_y`, `hood_top_z`.
    """
    y0, y1 = _req_pair(p, "y")
    w = p.req("width")
    zb, zt = p.req("bottom_z"), p.req("top_z")
    c = p.get("crown", 0.08)
    rw, rh = p.get("radiator", [w, zt - zb + 0.04])
    mb = model.mb("Hull")
    hw = w / 2
    rd = 0.10  # radiator shell depth
    ring = [(-hw, zb), (hw, zb), (hw, zt - c), (hw - c * 0.6, zt - c * 0.15), (0.0, zt),
            (-hw + c * 0.6, zt - c * 0.15), (-hw, zt - c)]
    rings = [[(x, y, z) for x, z in ring] for y in (y0, y1 - rd)]
    mb.loft(rings, "Paint", bevel=0.006)
    # hood hinge along the top and side seam
    mb.box((0, (y0 + y1 - rd) / 2, zt + 0.004), (0.03, y1 - rd - y0, 0.012), "Metal")
    for side in (-1, 1):
        mb.box((side * (hw + 0.004), (y0 + y1 - rd) / 2, zt - c - 0.02), (0.01, y1 - rd - y0, 0.015), "Paint")
        for i in range(p.get("louvers", 0)):
            ly = y0 + 0.12 + i * 0.07
            mb.box((side * (hw + 0.006), ly, (zb + zt) / 2 + 0.05), (0.012, 0.025, (zt - zb) * 0.4), "Metal")
    # radiator shell with grille bars
    rzb = zb - 0.02
    mb.box((0, y1 - rd / 2, rzb + rh / 2), (rw, rd, rh), "Paint", bevel=0.01)
    bars = max(3, int(rw / 0.06))
    for i in range(bars):
        x = -rw / 2 + 0.06 + (rw - 0.12) * i / (bars - 1)
        mb.box((x, y1 + 0.005, rzb + rh / 2), (0.018, 0.02, rh - 0.12), "Metal")
    if p.get("cap", True):
        mb.cylinder((0, y1 - rd / 2, rzb + rh), (0, y1 - rd / 2, rzb + rh + 0.05), 0.035, seg=8, mat="Metal")
    return {"hood_front_y": y1, "hood_rear_y": y0, "hood_top_z": zt}


@part("truck_cab")
def truck_cab(model, p, ctx):
    """Closed two-door cab with flat windscreen (trucks).

    Parameters: `y` [rear, front], `width`, `floor_z`, `roof_z` ("fit" = the
    catalog height; default), `belt_z` (window sill), `windscreen_lean` (m the
    roof edge sits behind the cab front, default 0.05), `roof_overhang`
    (default 0.04), `rear_window` (default true), `door` (door length,
    default 0.85; outline and handle on both sides), `seats` (passenger
    sockets in the cab besides the driver, default 0).
    Returns `cab_rear_y`, `cab_front_y`, `roof_z`.
    """
    y0, y1 = _req_pair(p, "y")
    w = p.req("width")
    zf = p.req("floor_z")
    rz = p.get("roof_z", "fit")
    roof_t = 0.04
    zr = (ctx["height"] - roof_t) if rz == "fit" else rz
    zb = p.req("belt_z")
    lean = p.get("windscreen_lean", 0.05)
    ov = p.get("roof_overhang", 0.04)
    hw = w / 2
    mb = model.mb("Hull")
    prof = [(y0, zf), (y1, zf), (y1, zb), (y1 - lean, zr), (y0, zr)]
    mb.extrude_x(prof, -hw, hw, "Paint", bevel=0.01)
    mb.box((0, (y0 + y1 - lean) / 2, zr + roof_t / 2), (w + ov * 2, y1 - lean - y0 + ov * 2, roof_t), "Canvas",
           bevel=0.01)
    # windscreen (two panes) and side windows
    gz0, gz1 = zb + 0.04, zr - 0.06
    for side in (-1, 1):
        cx = side * hw * 0.48
        mb.extrude([(cx - hw * 0.42, y1 - lean * (gz0 - zb) / (zr - zb) + 0.004, gz0),
                    (cx + hw * 0.42, y1 - lean * (gz0 - zb) / (zr - zb) + 0.004, gz0),
                    (cx + hw * 0.42, y1 - lean * (gz1 - zb) / (zr - zb) + 0.004, gz1),
                    (cx - hw * 0.42, y1 - lean * (gz1 - zb) / (zr - zb) + 0.004, gz1)],
                   (0, 0.006, 0), "Glass")
        door = p.get("door", 0.85)
        dy0 = max(y0 + 0.08, y1 - door - 0.06)
        mb.box((side * (hw + 0.004), (dy0 + y1 - 0.12) / 2, (gz0 + gz1) / 2),
               (0.008, y1 - 0.12 - dy0 - 0.06, gz1 - gz0), "Glass")
        # door outline (thin strips) and handle
        for yy in (dy0, y1 - 0.06):
            mb.box((side * (hw + 0.005), yy, (zf + zr) / 2), (0.01, 0.015, zr - zf - 0.08), "Paint")
        mb.box((side * (hw + 0.02), dy0 + 0.08, zb - 0.05), (0.03, 0.12, 0.025), "Metal")
    if p.get("rear_window", True):
        mb.box((0, y0 - 0.004, (gz0 + gz1) / 2 + 0.03), (w * 0.4, 0.008, (gz1 - gz0) * 0.5), "Glass")
    for i in range(p.get("seats", 0)):
        x = hw * (0.1 + 0.6 * (i + 1) / (p.get("seats", 0) + 1)) - hw * 0.2
        model.socket(model.next_index("Socket_Passenger_"), "Hull", (x, (y0 + y1) / 2, zf + 0.45))
    return {"cab_rear_y": y0, "cab_front_y": y1, "roof_z": zr + roof_t}


@part("open_tub")
def open_tub(model, p, ctx):
    """Open body of a light car without doors or roof (Willys MB; prompt 202):
    a tub with door cut-outs, the cowl with a folding windscreen frame
    standing up, two front seats, a rear bench between the wheel houses, the
    steering wheel, optionally the bare bows of the canvas top and the spare
    wheel on the rear panel. Takes the place of `[cab]` and `[body]`.

    Parameters: `y` [rear panel, cowl], `width` (tub over the sides),
    `floor_z`, `side_z` (top edge of the sides), `cutout` [y_rear, y_front,
    z] (door openings in both sides), `windscreen_z` (top of the frame),
    `windscreen_lean` (m the top leans back, default 0.10), `wheel_house`
    [y_rear, y_front, inner_x] (boxes over the rear wheels inside the tub),
    `bows` (bare top bows; their top is the catalog height; default true),
    `spare_wheel` [diameter, width] (on the rear panel; its rear face ends
    at the catalog overall length; optional), `drive` ("left" default, or
    "right"), `rear_seats` (Socket_Passenger_NN on the rear bench, default
    2; one more beside the driver).
    Returns `cab_rear_y`, `cab_front_y`, `bed_rear_y`.
    """
    y0, y1 = _req_pair(p, "y")
    w = p.req("width")
    zf, zs = p.req("floor_z"), p.req("side_z")
    cy0, cy1, cz = p.req("cutout")
    wz = p.req("windscreen_z")
    lean = p.get("windscreen_lean", 0.10)
    hw = w / 2
    t = 0.03
    mb = model.mb("Hull")
    L = y1 - y0
    mb.box((0, (y0 + y1) / 2, zf - 0.03), (w, L, 0.06), "Paint")
    for side in (-1, 1):
        x = side * (hw - t / 2)
        mb.box((x, (y0 + y1) / 2, (zf + cz) / 2), (t, L, cz - zf), "Paint", bevel=0.005)
        mb.box((x, (y0 + cy0) / 2, (cz + zs) / 2), (t, cy0 - y0, zs - cz), "Paint", bevel=0.005)
        mb.box((x, (cy1 + y1) / 2, (cz + zs) / 2), (t, y1 - cy1, zs - cz), "Paint", bevel=0.005)
        # grab handles on the rear quarter
        mb.box((side * (hw + 0.012), y0 + 0.35, zs - 0.10), (0.02, 0.16, 0.02), "Metal")
    mb.box((0, y0 + t / 2, (zf + zs) / 2), (w, t, zs - zf), "Paint", bevel=0.005)       # rear panel
    mb.box((0, y1 - 0.06, (zf + zs) / 2), (w, 0.12, zs - zf), "Paint", bevel=0.01)      # cowl with dashboard
    # windscreen: frame of two panes, leaning back, on the cowl
    fw = w - 0.10
    ft = 0.035
    yb, yt = y1 - 0.04, y1 - 0.04 - lean
    for side in (-1, 0, 1):
        x = side * (fw / 2 - ft / 2)
        mb.extrude([(x - ft / 2, yb, zs), (x + ft / 2, yb, zs), (x + ft / 2, yt, wz), (x - ft / 2, yt, wz)],
                   (0, 0.03, 0), "Paint")
    for z, yy in ((zs + ft / 2, yb - lean * ft / 2 / (wz - zs)), (wz - ft / 2, yt + lean * ft / 2 / (wz - zs))):
        mb.box((0, yy + 0.015, z), (fw, 0.03, ft), "Paint")
    for side in (-1, 1):
        xa, xb = side * ft / 2, side * (fw / 2 - ft)
        mb.extrude([(min(xa, xb), yb + 0.012, zs + ft), (max(xa, xb), yb + 0.012, zs + ft),
                    (max(xa, xb), yt + 0.012, wz - ft), (min(xa, xb), yt + 0.012, wz - ft)], (0, 0.006, 0), "Glass")
    # front seats and rear bench (canvas cushions on low frames)
    wh = p.get("wheel_house")
    sx = hw * 0.48
    sy = (cy0 + cy1) / 2 - 0.05
    for side in (-1, 1):
        mb.box((side * sx, sy, zf + 0.14), (0.44, 0.44, 0.28), "Paint")
        mb.box((side * sx, sy, zf + 0.33), (0.46, 0.46, 0.10), "Canvas", bevel=0.02)
        mb.box((side * sx, sy - 0.26, zf + 0.58), (0.46, 0.08, 0.44), "Canvas", bevel=0.02)
    bench_w = w - 0.10
    by = y0 + 0.50
    if wh:
        wy0, wy1, wx = wh
        bench_w = 2 * wx - 0.04
        by = (wy0 + wy1) / 2 - 0.05
        for side in (-1, 1):
            mb.box((side * (wx + (hw - t - wx) / 2), (wy0 + wy1) / 2, (zf + zs - 0.06) / 2),
                   (hw - t - wx, wy1 - wy0, zs - 0.06 - zf), "Paint", bevel=0.01)
    mb.box((0, by, zf + 0.33), (bench_w, 0.42, 0.10), "Canvas", bevel=0.02)
    mb.box((0, by - 0.24, zf + 0.56), (bench_w, 0.08, 0.40), "Canvas", bevel=0.02)
    # steering column and wheel
    drv = -1 if p.get("drive", "left") == "left" else 1
    c0 = (drv * sx, y1 - 0.08, zf + 0.25)
    c1 = (drv * sx, sy + 0.24, zs + 0.08)
    mb.cylinder(c0, c1, 0.02, seg=5, mat="Metal")
    d = (0.0, c1[1] - c0[1], c1[2] - c0[2])
    n = math.hypot(d[1], d[2])
    mb.cylinder(c1, (c1[0], c1[1] + d[1] / n * 0.025, c1[2] + d[2] / n * 0.025), 0.20, seg=10, mat="Rubber")
    # sockets: one beside the driver, the rest on the rear bench
    model.socket(model.next_index("Socket_Passenger_"), "Hull", (-drv * sx, sy, zf + 0.45))
    rs = p.get("rear_seats", 2)
    for i in range(rs):
        x = bench_w * ((i + 0.5) / rs - 0.5) * 0.8
        model.socket(model.next_index("Socket_Passenger_"), "Hull", (x, by, zf + 0.45))
    if p.get("bows", True):
        top = ctx["height"]
        r = 0.016
        py = cy0 - 0.10
        xb_ = hw - 0.02
        for ty, tz in ((py, top - r), (py - 0.55, top - 0.14)):
            for side in (-1, 1):
                mb.cylinder((side * xb_, py, zs - 0.10), (side * xb_, ty, tz), r, seg=5, mat="Metal")
            mb.cylinder((-xb_, ty, tz), (xb_, ty, tz), r, seg=5, mat="Metal")
    sw = p.get("spare_wheel")
    rear = y0
    if sw:
        dia, wid = sw
        rear = ctx["front_y"] - ctx["length"] + 0.004
        yc = rear + wid / 2
        mb.box((hw * 0.35, (y0 + yc) / 2, zs - 0.12), (0.10, max(y0 - yc, 0.02), 0.10), "Metal")
        # the tire stands against the rear panel: short cylinders along Y
        mb.cylinder((hw * 0.35, rear, zs - 0.12), (hw * 0.35, rear + wid, zs - 0.12), dia / 2, seg=12, mat="Rubber")
        mb.cylinder((hw * 0.35, rear - 0.004, zs - 0.12), (hw * 0.35, rear + 0.02, zs - 0.12), dia * 0.28, seg=10,
                    mat="Paint")
    return {"cab_rear_y": y0, "cab_front_y": y1, "bed_rear_y": rear}


@part("cargo_bed")
def cargo_bed(model, p, ctx):
    """Open cargo body with drop sides, plank lines and benches (trucks).

    Parameters: `front_y`, `rear_y` ("fit" = catalog overall length, default),
    `width` ("fit" = catalog width, default), `floor_z`, `side_height`,
    `planks` (plank lines per side, default 3), `stakes` (vertical straps per
    side, default 4), `material` (default Paint; Wood for unpainted bodies),
    `seats_per_side` (Socket_Passenger_NN on the side benches, default 0),
    `bench` (draw bench boards, default true when seats_per_side > 0).
    Returns `bed_rear_y`, `bed_top_z`.
    """
    yf = p.req("front_y")
    yr = p.get("rear_y", "fit")
    if yr == "fit":
        yr = ctx["front_y"] - ctx["length"] + 0.008  # plank strips stick out 8 mm
    # "fit": the stakes on the outside (2 cm) end at the catalog width
    w = ctx["width"] - 0.04 if p.get("width", "fit") == "fit" else p.get("width")
    zf = p.req("floor_z")
    h = p.req("side_height")
    mat = p.get("material", "Paint")
    t = 0.04
    hw = w / 2
    L = yf - yr
    cy = (yf + yr) / 2
    mb = model.mb("Hull")
    mb.box((0, cy, zf - 0.04), (w, L, 0.08), mat)
    # bearers under the floor
    for i in range(4):
        y = yr + 0.2 + (L - 0.4) * i / 3
        mb.box((0, y, zf - 0.12), (w - 0.1, 0.08, 0.08), "Paint")
    zt = zf + h
    walls = (((-hw + t / 2, cy), (t, L)), ((hw - t / 2, cy), (t, L)), ((0, yf - t / 2), (w, t)),
             ((0, yr + t / 2), (w, t)))
    for (x, y), (sx, sy) in walls:
        mb.box((x, y, zf + h / 2), (sx, sy, h), mat, bevel=0.005)
    planks = p.get("planks", 3)
    for k in range(1, planks):
        z = zf + h * k / planks
        for side in (-1, 1):
            mb.box((side * (hw + 0.004), cy, z), (0.008, L - 0.04, 0.012), mat)
        mb.box((0, yr - 0.004, z), (w - 0.04, 0.008, 0.012), mat)
    stakes = p.get("stakes", 4)
    for k in range(stakes):
        y = yr + 0.15 + (L - 0.3) * k / max(stakes - 1, 1)
        for side in (-1, 1):
            mb.box((side * (hw + 0.012), y, zf + h / 2), (0.016, 0.05, h + 0.02), "Metal")
    # front board raised (protects the cab rear window)
    mb.box((0, yf - t / 2, zt + 0.12), (w * 0.9, t, 0.24), mat, bevel=0.005)
    n = p.get("seats_per_side", 0)
    if n and p.get("bench", True):
        for side in (-1, 1):
            mb.box((side * (hw - 0.22), cy, zf + 0.42), (0.28, L - 0.2, 0.04), "Wood")
    for side in (-1, 1):
        for i in range(n):
            y = yf - 0.25 - (L - 0.5) * i / max(n - 1, 1)
            model.socket(model.next_index("Socket_Passenger_"), "Hull", (side * (hw - 0.25), y, zf + 0.45))
    return {"bed_rear_y": yr, "bed_front_y": yf, "bed_top_z": zt, "bed_width": w}


@part("truck_fenders")
def truck_fenders(model, p, ctx):
    """Front wings (simple bent sheet, ZiS-5V style), running boards and rear
    wings over the rear wheels.

    Parameters: `front_wheel` [y, radius], `x` [inner, outer] of the front
    wings, `clearance` (gap above the tire, default 0.06), `running_board`
    [y_rear, z] (board from the wing back to y_rear, optional), `rear_wheel`
    [y, radius] and `rear_x` [inner, outer] (rear wings, optional),
    `rear_z` (top of the rear wings, default above the tire).
    """
    mb = model.mb("Hull")
    fy, fr = p.req("front_wheel")
    xi, xo = sorted(p.req("x"))
    cl = p.get("clearance", 0.06)
    top = 2 * fr + cl
    t = 0.012
    # profile of the bent wing: low front lip, angled up, flat over the wheel, down at the back
    prof = [(fy + fr * 1.15, fr * 1.15), (fy + fr * 0.75, top), (fy - fr * 0.80, top), (fy - fr * 1.05, fr * 1.25)]
    run = p.get("running_board")
    if run:
        prof.append((fy - fr * 1.10, run[1]))
    for side in (-1, 1):
        x0, x1 = side * xi, side * xo
        for (ya, za), (yb, zb) in zip(prof, prof[1:]):
            dy, dz = yb - ya, zb - za
            ln = math.hypot(dy, dz)
            ny, nz = -dz / ln * t, dy / ln * t
            pts = [(ya, za), (yb, zb), (yb + ny, zb + nz), (ya + ny, za + nz)]
            mb.extrude_x(pts, min(x0, x1), max(x0, x1), "Paint")
        if run:
            y_rear, z = run
            y_front = fy - fr * 1.10
            mb.box((side * (xi + xo) / 2, (y_front + y_rear) / 2, z), (xo - xi, y_front - y_rear, 0.02), "Paint")
            mb.box((side * (xi + 0.02), (y_front + y_rear) / 2, z - 0.06), (0.04, y_front - y_rear, 0.1), "Paint")
    rw = p.get("rear_wheel")
    if rw:
        ry, rr = rw
        rxi, rxo = sorted(p.req("rear_x"))
        rz = p.get("rear_z", 2 * rr + cl)
        for side in (-1, 1):
            mb.box((side * (rxi + rxo) / 2, ry, rz), (rxo - rxi, rr * 2.3, 0.012), "Paint")
            mb.box((side * (rxi + rxo) / 2, ry - rr * 1.15, rz - 0.15), (rxo - rxi, 0.012, 0.3), "Paint")  # mud flap
    return {}


@assembly("wheeled")
def wheeled(model, bp, u, tune):
    """Wheeled soft-skin vehicle (trucks, cars) without turret.

    Blueprint sections, in build order: `[layout]`, `[[axle]]` (wheel_axle
    tables, numbered front to rear), `[frame]`, `[hood]`, `[cab]`, `[body]`,
    `[fenders]` (each optional except `[[axle]]`; each names its part), then
    `[[part]]`.

    `[layout]`: `front_y` (foremost point: radiator, bumper), written so that
    the axles straddle y = 0. Fitted to the catalog: the body's rear end (catalog
    overall length behind `front_y`), its width (catalog width) and the cab roof
    (catalog height). Afterwards the model is moved so that the origin lies
    midway between the first and the last axle (center of the ground contact).
    """
    layout = bp.get("layout", {})
    ctx = {
        "unit": u,
        "length": mm(u, "length_mm"),
        "width": mm(u, "width_mm"),
        "height": mm(u, "height_mm") + tune.get("height", 0.0),
        "half_width": mm(u, "width_mm") / 2,
        "layout": layout,
        "front_y": layout["front_y"],
    }
    model.mb("Hull")
    axles = bp.get("axle", [])
    if not axles:
        raise KeyError("[[axle]]: a wheeled vehicle needs at least one axle")
    for i, t in enumerate(axles):
        t = dict(t)
        name = t.pop("part", "wheel_axle")
        t.setdefault("index", i + 1)
        build_part(name, model, t, ctx, where=f"[[axle]] #{i + 1}")
    for key in ("frame", "hood", "cab", "body", "fenders"):
        if key in bp:
            t = dict(bp[key])
            name = t.pop("part")
            build_part(name, model, t, ctx, where=f"[{key}]")
    for i, t in enumerate(bp.get("part", [])):
        t = dict(t)
        name = t.pop("part")
        build_part(name, model, t, ctx, where=f"[[part]] #{i + 1} ({name})")
    ys = [a["y"] for a in axles]
    lo, _ = model.extent()  # tires are slightly rounded: put their lowest point on z = 0
    model.translate((0.0, -(max(ys) + min(ys)) / 2, -lo.z))
    return ctx
