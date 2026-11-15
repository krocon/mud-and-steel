"""Aircraft parts: fuselage loft, wings, tail, propeller, canopy, landing gear.

The fuselage is the `Hull` object at the root. Positions are model space
(nose +Y). The model stands on its landing gear; the assembly places the
origin under the center of gravity (docs/ASSETS.md).
"""

import math

from .registry import part
from .turret import superellipse


def _section_ring(y, w, h, zc, e, n):
    return [(x, y, zc + z) for x, z in superellipse(h, w, e, n)]


@part("fuselage")
def fuselage(model, p, ctx):
    """Fuselage as a loft through cross-sections.

    Parameters: `sections` [[y, width, height, z_center, exponent], ...] from
    tail to nose (exponent 2 = ellipse, 3+ = boxy), `segments` (ring
    resolution, default 12). Width/height 0 gives a point (tail cone).
    Returns `fuselage_front_y`, `fuselage_rear_y`.
    """
    secs = p.req("sections")
    n = p.get("segments", 12)
    mb = model.mb("Hull")
    rings = [_section_ring(y, max(w, 0.01), max(h, 0.01), zc, e, n) for y, w, h, zc, e in secs]
    mb.loft(rings, "Paint", smooth=True)
    return {"fuselage_front_y": secs[-1][0], "fuselage_rear_y": secs[0][0]}


def _airfoil(chord, thickness):
    """Simple lens airfoil (y, z) from leading edge (0) back to trailing edge."""
    t = chord * thickness
    return [(0.0, 0.0), (-0.1 * chord, t * 0.45), (-0.35 * chord, t * 0.5), (-chord, 0.0),
            (-0.35 * chord, -t * 0.4), (-0.1 * chord, -t * 0.35)]


def _wing_half(mb, side, root_y, z, root_chord, tip_chord, half_span, sweep, dihedral, thickness, x0=0.0):
    rings = []
    for f in (0.0, 1.0):
        x = x0 + f * half_span
        c = root_chord + (tip_chord - root_chord) * f
        ly = root_y - math.tan(sweep) * (x - x0)
        lz = z + math.tan(dihedral) * (x - x0)
        rings.append([(side * x, ly + ay, lz + az) for ay, az in _airfoil(c, thickness)])
    mb.loft(rings, "Paint")


@part("wing")
def wing(model, p, ctx):
    """Tapered, swept wing with dihedral; both halves into Hull, ailerons separate.

    Parameters: `y` (leading edge at the root), `z`, `root_chord`,
    `tip_chord`, `span` (full span, "fit" = catalog wingspan), `sweep_deg`
    (leading edge), `dihedral_deg`, `thickness` (fraction of chord),
    `aileron` [start, end] (fraction of half span).
    Adds `Aileron_L`, `Aileron_R`.
    """
    span = ctx["wingspan"] if p.get("span", "fit") == "fit" else p.get("span")
    y = p.req("y")
    z = p.req("z")
    rc, tc = p.req("root_chord"), p.req("tip_chord")
    sweep = math.radians(p.get("sweep_deg", 0.0))
    dih = math.radians(p.get("dihedral_deg", 5.0))
    th = p.get("thickness", 0.14)
    a0, a1 = p.get("aileron", [0.55, 0.95])
    hs = span / 2
    mb = model.mb("Hull")
    for side, s in ((-1, "L"), (1, "R")):
        _wing_half(mb, side, y, z, rc, tc, hs, sweep, dih, th)
        # aileron: thin plate along the trailing edge
        xs = (a0 * hs, a1 * hs)
        chord = [rc + (tc - rc) * (x / hs) for x in xs]
        te = [y - math.tan(sweep) * x - c for x, c in zip(xs, chord)]
        zz = [z + math.tan(dih) * x for x in xs]
        hinge = (side * xs[0], te[0] + 0.2 * chord[0], zz[0])
        am = model.mb(f"Aileron_{s}", pivot=hinge)
        pts = [(side * xs[0], te[0] + 0.2 * chord[0], zz[0]), (side * xs[1], te[1] + 0.2 * chord[1], zz[1]),
               (side * xs[1], te[1] - 0.01, zz[1]), (side * xs[0], te[0] - 0.01, zz[0])]
        am.extrude([(a, b, c - 0.012) for a, b, c in pts], (0, 0, 0.024), "Paint")
    return {"wing_y": y, "wing_z": z}


@part("tail")
def tail(model, p, ctx):
    """Horizontal stabilizer with `Elevator` and fin with `Rudder`.

    Parameters: `y` (leading edge of the stabilizer root), `z`, `span`,
    `root_chord`, `tip_chord`, `fin_height`, `fin_root_chord`,
    `fin_tip_chord`, `fin_sweep_deg`, `twin_fins` (x offset, 0 = single fin).
    """
    y, z = p.req("y"), p.req("z")
    span = p.req("span")
    rc, tc = p.req("root_chord"), p.req("tip_chord")
    mb = model.mb("Hull")
    for side in (-1, 1):
        _wing_half(mb, side, y, z, rc * 0.65, tc * 0.65, span / 2, math.radians(10), 0.0, 0.1)
    em = model.mb("Elevator", pivot=(0, y - rc * 0.65, z))
    em.box((0, y - rc * 0.82, z), (span * 0.9, rc * 0.35, 0.03), "Paint")
    fh = p.req("fin_height")
    frc, ftc = p.get("fin_root_chord", rc), p.get("fin_tip_chord", tc)
    fsw = math.radians(p.get("fin_sweep_deg", 20))
    twin = p.get("twin_fins", 0.0)
    xs = (-twin, twin) if twin else (0.0,)
    for i, x in enumerate(xs):
        rings = []
        for f in (0.0, 1.0):
            c = frc + (ftc - frc) * f
            ly = y - math.tan(fsw) * fh * f
            rings.append([(x + az, ly + ay, z + fh * f) for ay, az in _airfoil(c * 0.7, 0.1)])
        mb.loft(rings, "Paint")
        name = "Rudder" if len(xs) == 1 else f"Rudder_{'LR'[i]}"
        rm = model.mb(name, pivot=(x, y - frc * 0.7, z))
        rm.extrude([(x, y - frc * 0.7, z), (x, y - frc * 0.7 - math.tan(fsw) * fh * 0.0, z + fh * 0.9),
                    (x, y - frc - math.tan(fsw) * fh * 0.6, z + fh * 0.9), (x, y - frc, z)],
                   (0.03, 0, 0), "Paint")


@part("propeller")
def propeller(model, p, ctx):
    """Propeller with spinner. Parameters: `position` [x, y, z], `diameter`,
    `blades`, `name` (default Propeller; Propeller_L/_R for twins)."""
    x, y, z = p.req("position")
    r = p.req("diameter") / 2
    n = p.get("blades", 3)
    name = p.get("name", "Propeller")
    mb = model.mb(name, pivot=(x, y, z))
    mb.revolve((x, y - 0.25, z), (0, 1, 0), [(0, 0.17), (0.2, 0.16), (0.4, 0.09), (0.5, 0.01)], seg=12,
               mat="Paint")
    for i in range(n):
        a = 2 * math.pi * i / n
        d = (math.cos(a), math.sin(a))
        tip = (x + d[0] * r, y, z + d[1] * r)
        root = (x + d[0] * 0.12, y, z + d[1] * 0.12)
        w = 0.11
        pts = [(root[0] - d[1] * w, y + 0.02, root[2] + d[0] * w), (tip[0] - d[1] * w * 0.5, y, tip[2] + d[0] * w * 0.5),
               (tip[0] + d[1] * w * 0.5, y - 0.02, tip[2] - d[0] * w * 0.5), (root[0] + d[1] * w, y - 0.02,
                                                                             root[2] - d[0] * w)]
        mb.extrude(pts, (0, -0.03, 0), "Metal")


@part("canopy")
def canopy(model, p, ctx):
    """Cockpit canopy (Glass) with frame. Parameters: `y` [rear, front], `z`
    (sill height), `width`, `height`, `bubble` (true = teardrop)."""
    y0, y1 = p.req("y")
    z = p.req("z")
    w = p.req("width")
    h = p.req("height")
    bubble = p.get("bubble", False)
    mb = model.mb("Hull")
    rings = []
    steps = (0.0, 0.25, 0.6, 1.0)
    for f in steps:
        yy = y0 + (y1 - y0) * f
        k = math.sin(math.pi * min(0.999, max(0.05, f if bubble else 0.3 + 0.7 * f)))
        # upper half of a rounded section: an arch closed by the flat sill
        rings.append([(xx * k, yy, z + zz * k) for xx, zz in superellipse(2 * h, w, 2.2, 10) if zz >= -1e-6])
    mb.loft(rings, "Glass", smooth=True)
    mb.box(((0), y1, z + h * 0.5), (0.03, 0.03, h), "Paint")


@part("landing_gear")
def landing_gear(model, p, ctx):
    """Main gear and tail wheel, extended or retracted.

    Parameters: `state` "extended" | "retracted", `main_y`, `main_track`
    (wheel center distance), `main_wheel_diameter`, `strut_top_z`,
    `tail_y`, `tail_wheel_diameter`, `tail_strut_top_z`, `nose` (true =
    nose wheel instead of tail wheel, uses `tail_*` for it).
    Adds `Gear_L`, `Gear_R`, `Gear_Tail` (or `Gear_Nose`).
    """
    state = p.get("state", "extended")
    my = p.req("main_y")
    tr = p.req("main_track")
    mr = p.req("main_wheel_diameter") / 2
    top = p.req("strut_top_z")
    ty = p.req("tail_y")
    tr_r = p.get("tail_wheel_diameter", 0.3) / 2
    ttop = p.get("tail_strut_top_z", tr_r * 3)
    nose = p.get("nose", False)
    from .wheels import tire_wheel

    for side, s in ((-1, "L"), (1, "R")):
        x = side * tr / 2
        mb = model.mb(f"Gear_{s}", pivot=(x, my, top))
        if state == "extended":
            mb.cylinder((x, my, top), (x, my, mr), 0.05, seg=6, mat="Metal")
            tire_wheel(mb, (x + side * 0.0, my, mr), mr, mr * 0.45, side)
            mb.box((x + side * 0.08, my, (top + mr) / 2), (0.01, 0.4, top - mr), "Paint")
        else:
            mb.box((x, my, top - 0.01), (0.5, mr * 2.2, 0.01), "Paint")
    name = "Gear_Nose" if nose else "Gear_Tail"
    mb = model.mb(name, pivot=(0, ty, ttop))
    if state == "extended":
        mb.cylinder((0, ty, ttop), (0, ty, tr_r), 0.03, seg=6, mat="Metal")
        tire_wheel(mb, (0, ty, tr_r), tr_r, tr_r * 0.5, 1)
    else:
        mb.box((0, ty, ttop - 0.01), (0.25, tr_r * 2.2, 0.01), "Paint")
