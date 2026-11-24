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
    `glass_from_y` (optional): the nose forward of this section position is
    glazed (Glass), e.g. He 111, B-25 bombardier nose.
    Returns `fuselage_front_y`, `fuselage_rear_y`, `fuselage_sections`.
    """
    secs = p.req("sections")
    n = p.get("segments", 12)
    mb = model.mb("Hull")
    rings = [_section_ring(y, max(w, 0.01), max(h, 0.01), zc, e, n) for y, w, h, zc, e in secs]
    gy = p.get("glass_from_y")
    if gy is None:
        mb.loft(rings, "Paint", smooth=True)
    else:
        k = [i for i, sec in enumerate(secs) if abs(sec[0] - gy) < 1e-6]
        if not k or k[0] in (0, len(secs) - 1):
            raise ValueError("fuselage: glass_from_y must be an inner section position")
        mb.loft(rings[:k[0] + 1], "Paint", smooth=True)
        mb.loft(rings[k[0]:], "Glass", smooth=True)
    return {"fuselage_front_y": secs[-1][0], "fuselage_rear_y": secs[0][0], "fuselage_sections": secs}


def _airfoil(chord, thickness):
    """Simple lens airfoil (y, z) from leading edge (0) back to trailing edge."""
    t = chord * thickness
    return [(0.0, 0.0), (-0.1 * chord, t * 0.45), (-0.35 * chord, t * 0.5), (-chord, 0.0),
            (-0.35 * chord, -t * 0.4), (-0.1 * chord, -t * 0.35)]


def _chord_at(f, root_chord, tip_chord, planform):
    """Chord at the fraction `f` of the (exposed) half span."""
    if planform == "elliptical":
        return tip_chord + (root_chord - tip_chord) * math.sqrt(max(0.0, 1.0 - f * f))
    return root_chord + (tip_chord - root_chord) * f


def _wing_half(mb, side, root_y, z, root_chord, tip_chord, half_span, sweep, dihedral, thickness, x0=0.0,
               kink=None, planform="taper", le_share=0.3):
    """One wing half. `kink` = (fraction of the half span, inner dihedral in
    radians) gives a gull or inverted-gull wing (Ju 87, F4U). `planform`
    "elliptical" (prompt 202) rounds the chord off towards the tip; the
    leading edge takes `le_share` of the lost chord, the trailing edge the
    rest."""
    stations = [0.0, 1.0] if kink is None else [0.0, kink[0], 1.0]
    if planform == "elliptical":
        stations = sorted(set(stations) | {0.3, 0.55, 0.75, 0.88, 0.96, 0.99})
    rings = []
    for f in stations:
        x = x0 + f * half_span
        c = _chord_at(f, root_chord, tip_chord, planform)
        ly = root_y - math.tan(sweep) * (x - x0)
        if planform == "elliptical":
            ly -= (root_chord - c) * le_share
        lz = z + _dihedral_z(x - x0, half_span, dihedral, kink)
        rings.append([(side * x, ly + ay, lz + az) for ay, az in _airfoil(c, thickness)])
    mb.loft(rings, "Paint")


def _dihedral_z(dx, half_span, dihedral, kink):
    """Height of the wing chord line above the root at distance dx."""
    if kink is None:
        return math.tan(dihedral) * dx
    xk = kink[0] * half_span
    if dx <= xk:
        return math.tan(kink[1]) * dx
    return math.tan(kink[1]) * xk + math.tan(dihedral) * (dx - xk)


@part("wing")
def wing(model, p, ctx):
    """Tapered, swept wing with dihedral; both halves into Hull, ailerons separate.

    Parameters: `y` (leading edge at the root), `z`, `root_chord`,
    `tip_chord`, `span` (full span, "fit" = catalog wingspan), `sweep_deg`
    (leading edge), `dihedral_deg`, `thickness` (fraction of chord),
    `aileron` [start, end] (fraction of half span), `kink` = { at,
    inner_dihedral_deg } (gull wing: inner panel up to `at` x half span with
    its own dihedral, negative = inverted gull, Ju 87), `x0` (wing root
    distance from the center line, default 0), `planform` ("taper",
    default, or "elliptical": the chord runs out to `tip_chord` along an
    ellipse, Spitfire; prompt 202) with `le_share` (share of the lost chord
    taken from the leading edge, default 0.3; the rest curves the trailing
    edge). Adds `Aileron_L`,
    `Aileron_R`. Returns `wing_y`, `wing_z`, `wing` (planform values for
    `shadow_proxy`).
    """
    span = ctx["wingspan"] if p.get("span", "fit") == "fit" else p.get("span")
    y = p.req("y")
    z = p.req("z")
    rc, tc = p.req("root_chord"), p.req("tip_chord")
    sweep = math.radians(p.get("sweep_deg", 0.0))
    dih = math.radians(p.get("dihedral_deg", 5.0))
    th = p.get("thickness", 0.14)
    a0, a1 = p.get("aileron", [0.55, 0.95])
    kt = p.get("kink")
    kink = None if kt is None else (kt["at"], math.radians(kt["inner_dihedral_deg"]))
    x0 = p.get("x0", 0.0)
    planform = p.get("planform", "taper")
    if planform not in ("taper", "elliptical"):
        raise ValueError(f"wing: unknown planform {planform!r}")
    le = p.get("le_share", 0.3)
    hs = span / 2
    mb = model.mb("Hull")
    for side, s in ((-1, "L"), (1, "R")):
        _wing_half(mb, side, y, z, rc, tc, hs - x0, sweep, dih, th, x0=x0, kink=kink, planform=planform,
                   le_share=le)
        # aileron: thin plate along the trailing edge
        xs = (a0 * hs, a1 * hs)
        chord = [_chord_at((x - x0) / (hs - x0), rc, tc, planform) for x in xs]
        lost = [(rc - c) * le if planform == "elliptical" else 0.0 for c in chord]
        te = [y - math.tan(sweep) * (x - x0) - c - d for x, c, d in zip(xs, chord, lost)]
        zz = [z + _dihedral_z(x - x0, hs - x0, dih, kink) for x in xs]
        hinge = (side * xs[0], te[0] + 0.2 * chord[0], zz[0])
        am = model.mb(f"Aileron_{s}", pivot=hinge)
        pts = [(side * xs[0], te[0] + 0.2 * chord[0], zz[0]), (side * xs[1], te[1] + 0.2 * chord[1], zz[1]),
               (side * xs[1], te[1] - 0.01, zz[1]), (side * xs[0], te[0] - 0.01, zz[0])]
        am.extrude([(a, b, c - 0.012) for a, b, c in pts], (0, 0, 0.024), "Paint")
    wing_info = dict(y=y, z=z, root_chord=rc, tip_chord=tc, half_span=hs, sweep=sweep, dihedral=dih, kink=kink, x0=x0)
    return {"wing_y": y, "wing_z": z, "wing": wing_info}


@part("tail")
def tail(model, p, ctx):
    """Horizontal stabilizer with `Elevator` and fin with `Rudder`.

    Parameters: `y` (leading edge of the stabilizer root), `z`, `span`,
    `root_chord`, `tip_chord`, `fin_height`, `fin_root_chord`,
    `fin_tip_chord`, `fin_sweep_deg`, `twin_fins` (x offset, 0 = single fin).
    Prompt 202: `planform` "elliptical" rounds the tailplane (`root_chord`
    and `tip_chord` are then the whole chord with the elevator, which
    follows the rounded trailing edge); `fin_outline` and `rudder_outline`
    [[y, z], ...] (model space, side view, single fin only) draw fin and
    rudder as plates of that outline instead of the trapezoid
    (`fin_thickness`, default 0.07; `fin_height` and the fin chords are
    then not used).
    """
    y, z = p.req("y"), p.req("z")
    span = p.req("span")
    rc, tc = p.req("root_chord"), p.req("tip_chord")
    planform = p.get("planform", "taper")
    if planform not in ("taper", "elliptical"):
        raise ValueError(f"tail: unknown planform {planform!r}")
    mb = model.mb("Hull")
    if planform == "elliptical":
        hs = span / 2
        fs = (0.0, 0.3, 0.55, 0.75, 0.9, 0.97, 1.0)
        for side in (-1, 1):
            _wing_half(mb, side, y, z, rc * 0.62, tc * 0.62, hs, 0.0, 0.0, 0.1, planform="elliptical", le_share=0.56)
        em = model.mb("Elevator", pivot=(0, y - rc * 0.62, z))
        for side in (-1, 1):
            hinge, te = [], []
            for f in fs:
                c = _chord_at(f, rc, tc, "elliptical")
                le = y - (rc - c) * 0.35
                hinge.append((side * f * hs, le - 0.6 * c, z - 0.015))
                te.append((side * f * hs, le - c, z - 0.015))
            pts = hinge + te[::-1]
            em.extrude(pts if side == 1 else pts[::-1], (0, 0, 0.03), "Paint")
        tail_info = dict(y=y, z=z, span=span, root_chord=rc * 0.8, tip_chord=tc * 0.8)
    else:
        for side in (-1, 1):
            _wing_half(mb, side, y, z, rc * 0.65, tc * 0.65, span / 2, math.radians(10), 0.0, 0.1)
        em = model.mb("Elevator", pivot=(0, y - rc * 0.65, z))
        em.box((0, y - rc * 0.82, z), (span * 0.9, rc * 0.35, 0.03), "Paint")
        tail_info = dict(y=y, z=z, span=span, root_chord=rc * 0.65, tip_chord=tc * 0.65)
    fo, ro = p.get("fin_outline"), p.get("rudder_outline")
    twin = p.get("twin_fins", 0.0)
    if fo or ro:
        if not (fo and ro) or twin:
            raise ValueError("tail: fin_outline and rudder_outline go together and need a single fin")
        ft = p.get("fin_thickness", 0.07)
        mb.extrude([(-ft / 2, a, b) for a, b in fo], (ft, 0, 0), "Paint")
        hy = max(a for a, _ in ro)
        rm = model.mb("Rudder", pivot=(0, hy, min(b for _, b in ro)))
        rm.extrude([(-ft * 0.35, a, b) for a, b in ro], (ft * 0.7, 0, 0), "Paint")
        return {"tail": tail_info}
    fh = p.req("fin_height")
    frc, ftc = p.get("fin_root_chord", rc), p.get("fin_tip_chord", tc)
    fsw = math.radians(p.get("fin_sweep_deg", 20))
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
    return {"tail": tail_info}


@part("propeller")
def propeller(model, p, ctx):
    """Propeller with spinner. Parameters: `position` [x, y, z], `diameter`,
    `blades`, `name` (default Propeller; Propeller_L/_R for twins),
    `phase_deg` (angle of the first blade, default 0 = pointing to +X; 90
    stands a blade of a three-blade propeller upright, prompt 202),
    `spinner` [length, diameter] (prompt 202; the spinner then starts 0.3 x
    length behind the blade plane and ends 0.7 x length ahead of it;
    default: the small 0.5 x 0.34 m spinner), `blade_width` (chord at the
    root, default 0.11)."""
    x, y, z = p.req("position")
    r = p.req("diameter") / 2
    n = p.get("blades", 3)
    name = p.get("name", "Propeller")
    phase = math.radians(p.get("phase_deg", 0.0))
    mb = model.mb(name, pivot=(x, y, z))
    sp = p.get("spinner")
    if sp is None:
        mb.revolve((x, y - 0.25, z), (0, 1, 0), [(0, 0.17), (0.2, 0.16), (0.4, 0.09), (0.5, 0.01)], seg=12,
                   mat="Paint")
    else:
        sl, sr = sp[0], sp[1] / 2
        mb.revolve((x, y - 0.3 * sl, z), (0, 1, 0),
                   [(0, sr), (0.3 * sl, sr * 0.97), (0.55 * sl, sr * 0.78), (0.8 * sl, sr * 0.45),
                    (0.94 * sl, sr * 0.18), (sl, 0.01)], seg=12, mat="Paint")
    w = p.get("blade_width", 0.11)
    for i in range(n):
        a = phase + 2 * math.pi * i / n
        d = (math.cos(a), math.sin(a))
        tip = (x + d[0] * r, y, z + d[1] * r)
        root = (x + d[0] * 0.12, y, z + d[1] * 0.12)
        pts = [(root[0] - d[1] * w, y + 0.02, root[2] + d[0] * w), (tip[0] - d[1] * w * 0.5, y, tip[2] + d[0] * w * 0.5),
               (tip[0] + d[1] * w * 0.5, y - 0.02, tip[2] - d[0] * w * 0.5), (root[0] + d[1] * w, y - 0.02,
                                                                             root[2] - d[0] * w)]
        mb.extrude(pts, (0, -0.03, 0), "Metal")


@part("canopy")
def canopy(model, p, ctx):
    """Cockpit canopy (Glass) with frame. Parameters: `y` [rear, front], `z`
    (sill height), `width`, `height`, `bubble` (true = teardrop),
    `profile` [[f, k], ...] (prompt 202: the scale k of the cross-section at
    the fraction f from rear to front, instead of the built-in shapes; e.g.
    a hood with rear glazing that runs out into a high fuselage spine)."""
    y0, y1 = p.req("y")
    z = p.req("z")
    w = p.req("width")
    h = p.req("height")
    bubble = p.get("bubble", False)
    mb = model.mb("Hull")
    rings = []
    prof = p.get("profile")
    steps = (0.0, 0.25, 0.6, 1.0) if prof is None else [q[0] for q in prof]
    for i, f in enumerate(steps):
        yy = y0 + (y1 - y0) * f
        if prof is not None:
            k = prof[i][1]
        else:
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
    nose wheel instead of tail wheel, uses `tail_*` for it), `fixed` (gear
    that never retracts: Ju 87, Fi 156, Po-2, L-4; default false), `spats`
    (streamlined fairings around strut and wheel, Ju 87; default false),
    `skid` (tail skid instead of a tail wheel, Po-2; default false),
    `ground_z` and `tail_ground_z` (prompt 202: height of the lowest point
    of the main wheels and of the tail wheel in the blueprint's frame,
    default 0; for aircraft drawn level, which `[layout] ground_angle`
    sets down on both).
    Adds `Gear_L`, `Gear_R`, `Gear_Tail` (or `Gear_Nose`) and the root extras
    `gear_retractable` (false for fixed gear: the game hides retractable
    gear in flight).
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
    fixed = p.get("fixed", False)
    spats = p.get("spats", False)
    skid = p.get("skid", False)
    if fixed and state != "extended":
        raise ValueError("landing_gear: fixed gear cannot be retracted")
    model.props["gear_retractable"] = not fixed
    from .wheels import tire_wheel

    gz, tgz = p.get("ground_z", 0.0), p.get("tail_ground_z", 0.0)
    wz = gz + mr  # main wheel center
    for side, s in ((-1, "L"), (1, "R")):
        x = side * tr / 2
        mb = model.mb(f"Gear_{s}", pivot=(x, my, top))
        if state == "extended" and spats:
            if gz:
                raise ValueError("landing_gear: ground_z is not supported with spats")
            tire_wheel(mb, (x, my, mr), mr, mr * 0.45, side)
            # fairing over the upper wheel and a streamlined leg up to the wing
            mb.revolve((x, my - mr * 1.6, mr * 1.05), (0, 1, 0), [(0, 0.02), (mr * 0.6, mr * 0.55), (mr * 1.6, mr * 0.7),
                                                                  (mr * 2.6, mr * 0.45), (mr * 3.2, 0.02)], seg=10,
                       mat="Paint")
            rings = []
            for zz, w in ((mr * 1.4, 0.16), (top, 0.22)):
                rings.append([(x - 0.06, my - w, zz), (x + 0.06, my - w, zz), (x + 0.04, my + w * 0.6, zz),
                              (x - 0.04, my + w * 0.6, zz)])
            mb.loft(rings, "Paint")
        elif state == "extended":
            mb.cylinder((x, my, top), (x, my, wz), 0.05, seg=6, mat="Metal")
            tire_wheel(mb, (x + side * 0.0, my, wz), mr, mr * 0.45, side)
            if not fixed:  # gear door
                mb.box((x + side * 0.08, my, (top + wz) / 2), (0.01, 0.4, top - wz), "Paint")
        else:
            mb.box((x, my, top - 0.01), (0.5, mr * 2.2, 0.01), "Paint")
    name = "Gear_Nose" if nose else "Gear_Tail"
    mb = model.mb(name, pivot=(0, ty, ttop))
    if state == "extended" and skid:
        mb.extrude([(0, ty + 0.25, ttop), (0, ty - 0.15, tgz), (0, ty - 0.2, tgz), (0, ty + 0.15, ttop)], (0.03, 0, 0),
                   "Metal")
    elif state == "extended":
        mb.cylinder((0, ty, ttop), (0, ty, tgz + tr_r), 0.03, seg=6, mat="Metal")
        tire_wheel(mb, (0, ty, tgz + tr_r), tr_r, tr_r * 0.5, 1)
    else:
        mb.box((0, ty, ttop - 0.01), (0.25, tr_r * 2.2, 0.01), "Paint")
    return {"gear_contact": dict(main=(my, gz), tail=(ty, tgz))}


# --- prompt 008: parts for the remaining aircraft ----------------------------------
#
# Names (docs/ASSETS.md): `Bomb_NN` / `Rocket_NN` are external stores that can
# be hidden when released, with `Socket_Bomb_NN` / `Socket_Rocket_NN` at the
# release point; internal bombs only have the sockets in the bay, whose doors
# are `BayDoor_L` / `BayDoor_R`. Fixed forward guns have `Socket_Gun_NN` at
# each muzzle (and `Socket_Muzzle` at their mean position, the tracer origin
# of the gallery and the game); flexible guns are `MG_NN` like on vehicles,
# in a powered turret `GunTurret_NN`. `ShadowProxy` is a flat low-poly
# planform that casts the shadow at high altitude (see `shadow_proxy`).


def _mirror_list(points, mirror):
    out = []
    for p in points:
        out.append(tuple(p))
        if mirror and abs(p[0]) > 1e-6:
            out.append((-p[0], p[1], p[2]))
    return out


@part("nacelle")
def nacelle(model, p, ctx):
    """Engine nacelle on the wing (twin-engine aircraft), into Hull.

    Parameters: `x` (center distance from the center line; mirrored to
    both sides), `y` [rear, front], `z` (axis height), `diameter` (largest
    cross-section), `front_diameter` (cowling face, default 0.85 x),
    `height_scale` (nacelles deeper than wide, default 1.15), `exhaust`
    (adds `Socket_Exhaust_NN` on the outer side, default true). Put the
    propeller in front with `[[propeller]]` (name Propeller_L / _R).
    """
    x = p.req("x")
    y0, y1 = p.req("y")
    z = p.req("z")
    r = p.req("diameter") / 2
    rf = p.get("front_diameter", p.req("diameter") * 0.85) / 2
    hs = p.get("height_scale", 1.15)
    mb = model.mb("Hull")
    L = y1 - y0
    for side in (-1, 1):
        cx = side * x
        rings = []
        for f, rr in ((0.0, 0.05), (0.35, r * 0.85), (0.75, r), (0.97, rf), (1.0, rf * 0.8)):
            yy = y0 + L * f
            rings.append([(cx + math.cos(a) * rr, yy, z + math.sin(a) * rr * hs)
                          for a in (2 * math.pi * i / 10 for i in range(10))])
        mb.loft(rings, "Paint", smooth=True)
        if p.get("exhaust", True):
            model.socket(model.next_index("Socket_Exhaust_"), "Hull", (cx + side * r, y0 + L * 0.7, z))


@part("wing_struts")
def wing_struts(model, p, ctx):
    """Streamlined struts: wing bracing of high-wing aircraft (Fi 156, L-4),
    interplane and cabane struts of biplanes (Po-2), into Hull.

    Parameters: `struts` [[[x0, y0, z0], [x1, y1, z1]], ...] (model space,
    mirrored to -x unless `mirror = false`), `chord` (strut width along Y,
    default 0.06), `thickness` (default 0.025), `material` (default Paint).
    """
    chord = p.get("chord", 0.06)
    t = p.get("thickness", 0.025)
    mat = p.get("material", "Paint")
    mirror = p.get("mirror", True)
    mb = model.mb("Hull")
    for a, b in p.req("struts"):
        pairs = [(a, b)]
        if mirror and (abs(a[0]) > 1e-6 or abs(b[0]) > 1e-6):
            pairs.append(((-a[0], a[1], a[2]), (-b[0], b[1], b[2])))
        for pa, pb in pairs:
            rings = []
            for c in (pa, pb):
                rings.append([(c[0] - t / 2, c[1] + chord / 2, c[2]), (c[0] + t / 2, c[1] + chord / 2, c[2]),
                              (c[0] + t / 2, c[1] - chord / 2, c[2]), (c[0] - t / 2, c[1] - chord / 2, c[2])])
            mb.loft(rings, mat)


@part("upper_wing")
def upper_wing(model, p, ctx):
    """Upper wing of a biplane (Po-2), into Hull, no separate ailerons.

    Parameters like `wing`: `y`, `z`, `root_chord`, `tip_chord`, `span`
    ("fit" = catalog wingspan, default), `sweep_deg`, `dihedral_deg`,
    `thickness`, `x0` (half gap over the fuselage, default 0). Combine
    with `wing_struts` for the interplane and cabane struts.
    """
    span = ctx["wingspan"] if p.get("span", "fit") == "fit" else p.get("span")
    x0 = p.get("x0", 0.0)
    mb = model.mb("Hull")
    for side in (-1, 1):
        _wing_half(mb, side, p.req("y"), p.req("z"), p.req("root_chord"), p.req("tip_chord"), span / 2 - x0,
                   math.radians(p.get("sweep_deg", 0.0)), math.radians(p.get("dihedral_deg", 2.0)),
                   p.get("thickness", 0.10), x0=x0)
    return {"upper_wing": dict(y=p.req("y"), z=p.req("z"), root_chord=p.req("root_chord"),
                               tip_chord=p.req("tip_chord"), half_span=span / 2, x0=x0,
                               sweep=math.radians(p.get("sweep_deg", 0.0)),
                               dihedral=math.radians(p.get("dihedral_deg", 2.0)))}


@part("fixed_guns")
def fixed_guns(model, p, ctx):
    """Forward-firing fixed guns in wings, cowling or nose, into Hull.

    Parameters: `muzzles` [[x, y, z], ...] (model space, mirrored to -x
    unless `mirror = false`), `length` (visible barrel stub behind the
    muzzle, default 0.35), `radius` (default 0.012; 20 mm cannon ~0.02).
    Adds `Socket_Gun_NN` at every muzzle and `Socket_Muzzle` at their mean
    position (unless the model already has one).
    """
    L = p.get("length", 0.35)
    r = p.get("radius", 0.012)
    pts = _mirror_list(p.req("muzzles"), p.get("mirror", True))
    mb = model.mb("Hull")
    for x, y, z in pts:
        mb.cylinder((x, y - L, z), (x, y, z), r, seg=6, mat="Metal")
        model.socket(model.next_index("Socket_Gun_"), "Hull", (x, y, z))
    if not any(s[0] == "Socket_Muzzle" for s in model.sockets):
        n = len(pts)
        model.socket("Socket_Muzzle", "Hull", (sum(q[0] for q in pts) / n, max(q[1] for q in pts),
                                               sum(q[2] for q in pts) / n))


def _gun_along(mb, pos, direction, length, r):
    """Flexible machine gun from `pos` along `direction`; returns the muzzle."""
    from mathutils import Vector

    d = Vector(direction).normalized()
    a = Vector(pos)
    mb.cylinder(tuple(a - d * 0.25), tuple(a + d * 0.05), 0.035, seg=6, mat="Metal")  # receiver
    tip = a + d * length
    mb.cylinder(tuple(a), tuple(tip), r, seg=6, mat="Metal")
    return tuple(tip)


@part("flex_mg")
def flex_mg(model, p, ctx):
    """Flexible machine gun in an open or glazed position (rear gunner,
    beam and ventral guns): own `MG_NN` object with `Socket_Muzzle_NN`.

    Parameters: `position` [x, y, z] (mount pivot), `direction` [x, y, z]
    (default [0, -1, 0.15] = rearward and slightly up), `length` (default
    0.9), `twin` (twin gun MG 81Z, default false), `parent` (default Hull).
    """
    x, y, z = p.req("position")
    d = p.get("direction", [0.0, -1.0, 0.15])
    L = p.get("length", 0.9)
    name = model.next_index("MG_")
    mb = model.mb(name, parent=p.get("parent", "Hull"), pivot=(x, y, z))
    offs = (-0.05, 0.05) if p.get("twin", False) else (0.0,)
    tips = [_gun_along(mb, (x + o, y, z), d, L, 0.011) for o in offs]
    tip = tuple(sum(c) / len(tips) for c in zip(*tips))
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, tip)


@part("gun_turret")
def gun_turret(model, p, ctx):
    """Powered gun turret with glazed dome and machine guns (B-25 dorsal and
    tail positions, Pe-2 / He 111 gun cupolas).

    Parameters: `position` [x, y, z] (turret ring center on the skin),
    `diameter`, `height` (dome height, default 0.5 x diameter), `guns`
    (count, default 2), `direction` [x, y, z] (gun direction in the rest
    pose, default forward [0, 1, 0]), `gun_length` (default 1.1), `parent`
    (default Hull). Adds `GunTurret_NN` (rotates around Z, pivot at the
    ring) with `MG_NN` children and their `Socket_Muzzle_NN`.
    """
    x, y, z = p.req("position")
    dia = p.req("diameter")
    h = p.get("height", dia * 0.5)
    n = p.get("guns", 2)
    d = p.get("direction", [0.0, 1.0, 0.0])
    gl = p.get("gun_length", 1.1)
    tname = model.next_index("GunTurret_")
    tb = model.mb(tname, parent=p.get("parent", "Hull"), pivot=(x, y, z))
    tb.cylinder((x, y, z - 0.05), (x, y, z + 0.02), dia / 2, seg=14, mat="Paint")
    tb.revolve((x, y, z), (0, 0, 1), [(0, dia / 2 * 0.95), (h * 0.5, dia / 2 * 0.85), (h * 0.85, dia / 2 * 0.5),
                                      (h, 0.02)], seg=14, mat="Glass")
    gz = z + h * 0.35
    for i in range(n):
        off = (i - (n - 1) / 2) * 0.16
        name = model.next_index("MG_")
        mb = model.mb(name, parent=tname, pivot=(x + off, y, gz))
        tip = _gun_along(mb, (x + off, y, gz), d, gl, 0.013)
        model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, tip)


def _store(mb, center, length, r, fins=True, nose="round"):
    cx, cy, cz = center
    y0 = cy - length / 2
    prof = [(0.0, r * 0.35), (length * 0.3, r), (length * 0.75, r), (length * 0.92, r * 0.7), (length, 0.01)]
    if nose == "flat":
        prof[-1] = (length, r * 0.5)
    mb.revolve((cx, y0, cz), (0, 1, 0), prof, seg=8, mat="Paint")
    if fins:
        for k in (-1, 1):
            mb.box((cx, y0 + length * 0.08, cz), (r * 2.6 if k > 0 else 0.008, length * 0.16,
                                                  0.008 if k > 0 else r * 2.6), "Paint")


@part("bomb_load")
def bomb_load(model, p, ctx):
    """External bombs on racks (fuselage or wing): each bomb is its own
    `Bomb_NN` object (hidden on release) with `Socket_Bomb_NN` at its center.

    Parameters: `bombs` [[x, y, z], ...] (bomb centers, model space;
    mirrored to -x unless `mirror = false`), `length`, `diameter` (real
    store dimensions, e.g. SC 500: 2.0 x 0.40 m), `rack` (pylon above each
    bomb, default true), `fins` (default true), `swing_arm` (Ju 87
    centerline crutch that swings the bomb clear of the propeller, default
    false).
    """
    L = p.req("length")
    r = p.req("diameter") / 2
    pts = _mirror_list(p.req("bombs"), p.get("mirror", True))
    hull = model.mb("Hull")
    for x, y, z in pts:
        name = model.next_index("Bomb_")
        mb = model.mb(name, parent="Hull", pivot=(x, y, z))
        _store(mb, (x, y, z), L, r, fins=p.get("fins", True))
        if p.get("rack", True):
            hull.box((x, y, z + r + 0.06), (0.06, L * 0.4, 0.12), "Paint")
        if p.get("swing_arm", False):
            for k in (-1, 1):
                hull.cylinder((x + k * r * 0.8, y + L * 0.1, z), (x + k * 0.12, y + L * 0.35, z + r + 0.35), 0.015,
                              seg=4, mat="Metal")
        model.socket(f"Socket_Bomb_{int(name[5:]):02d}", "Hull", (x, y, z))


@part("rocket_rails")
def rocket_rails(model, p, ctx):
    """Unguided rockets under the wings (P-47 HVAR, Il-2 RS-82): rails into
    Hull, each rocket `Rocket_NN` with `Socket_Rocket_NN` at its tail.

    Parameters: `rockets` [[x, y, z], ...] (rocket centers, mirrored to -x
    unless `mirror = false`), `length` (default 1.73 = 5 in HVAR),
    `diameter` (default 0.127), `rail` (default true).
    """
    L = p.get("length", 1.73)
    r = p.get("diameter", 0.127) / 2
    hull = model.mb("Hull")
    for x, y, z in _mirror_list(p.req("rockets"), p.get("mirror", True)):
        name = model.next_index("Rocket_")
        mb = model.mb(name, parent="Hull", pivot=(x, y, z))
        _store(mb, (x, y, z), L, r, fins=True, nose="round")
        if p.get("rail", True):
            hull.box((x, y, z + r + 0.04), (0.03, L * 0.6, 0.08), "Metal")
        model.socket(f"Socket_Rocket_{int(name[7:]):02d}", "Hull", (x, y - L / 2, z))


@part("bomb_bay")
def bomb_bay(model, p, ctx):
    """Internal bomb bay: doors as `BayDoor_L` / `BayDoor_R` (hinged at the
    outer edge, open by rotating around Y) and one `Socket_Bomb_NN` per
    bomb station inside (He 111 vertical cells, B-25 and Pe-2 bays).

    Parameters: `y` [rear, front], `width` (door span), `z` (fuselage
    underside at the bay), `stations` [[x, y, z], ...] (bomb centers) or
    `count` (default: bombs from the catalog weapon list; evenly in two
    rows), `doors` (default true; false = sockets only).
    """
    y0, y1 = p.req("y")
    w = p.req("width")
    z = p.req("z")
    if p.get("doors", True):
        for side, s in ((-1, "L"), (1, "R")):
            hinge = (side * w / 2, (y0 + y1) / 2, z)
            mb = model.mb(f"BayDoor_{s}", parent="Hull", pivot=hinge)
            mb.box((side * w / 4, (y0 + y1) / 2, z - 0.012), (w / 2 - 0.01, y1 - y0, 0.015), "Paint")
    st = p.get("stations")
    if st is None:
        n = p.get("count")
        if n is None:
            n = sum(wp.get("count", 1) for wp in ctx["unit"].get("weapon", []) if "bomb" in wp["name"].lower())
        n = max(n, 1)
        rows = 2 if n > 1 else 1
        per = math.ceil(n / rows)
        st = []
        for i in range(n):
            row, col = i % rows, i // rows
            x = (row - (rows - 1) / 2) * w * 0.4
            yy = y0 + (y1 - y0) * (col + 0.5) / per
            st.append([x, yy, z + 0.25])
    else:
        p.get("count")
    for x, y, zz in st:
        model.socket(model.next_index("Socket_Bomb_"), "Hull", (x, y, zz))


@part("shadow_proxy")
def shadow_proxy(model, p, ctx):
    """Flat low-poly planform that casts the aircraft's shadow at high
    altitude (`ShadowProxy`, child of the root, under 100 triangles).

    Built from the fuselage sections, `wing`, `upper_wing` and `tail` of the
    context, so it must come after them (in `[[part]]`). Every surface lies
    in its own chord plane (with dihedral and gull kink) and the fuselage
    strip on the section centers, so the proxy hides inside the model in
    Blender previews. The Godot import script makes it shadow-only and
    hides it; the game switches it on (and the real meshes' shadows off)
    when the aircraft flies high, where the detailed shadow would be a few
    pixels and cost a full shadow pass.

    Parameters: `thickness` (default 0.02).
    """
    t = p.get("thickness", 0.02)
    wing = ctx.get("wing")
    if wing is None or "fuselage_sections" not in ctx:
        raise KeyError("shadow_proxy: needs fuselage and wing first")
    mb = model.mb("ShadowProxy")

    def quad(pts):
        mb.extrude([(x, y, z - t / 2) for x, y, z in pts], (0, 0, t), "Paint")

    secs = ctx["fuselage_sections"]
    for (y0, w0, _, z0, _), (y1, w1, _, z1, _) in zip(secs, secs[1:]):
        h0, h1 = max(w0, 0.02) / 2, max(w1, 0.02) / 2
        quad([(-h0, y0, z0), (h0, y0, z0), (h1, y1, z1), (-h1, y1, z1)])
    tail = ctx.get("tail")
    for info in (wing, ctx.get("upper_wing"), tail):
        if not info:
            continue
        hs = info.get("half_span", info.get("span", 0) / 2)
        x0 = info.get("x0", 0.0)
        sweep = info.get("sweep", math.radians(10) if info is tail else 0.0)
        dih = info.get("dihedral", 0.0)
        kink = info.get("kink")
        y, z, rc, tc = info["y"], info["z"], info["root_chord"], info["tip_chord"]
        stations = [0.0, 1.0] if kink is None else [0.0, kink[0], 1.0]
        for side in (-1, 1):
            for f0, f1 in zip(stations, stations[1:]):
                pts = []
                for f in (f0, f1):
                    dx = f * (hs - x0)
                    c = rc + (tc - rc) * f
                    le = y - math.tan(sweep) * dx
                    zz = z + _dihedral_z(dx, hs - x0, dih, kink)
                    pts.append(((x0 + dx) * side, le, zz, c))
                (xa, la, za, ca), (xb, lb, zb, cb) = pts
                quad([(xa, la, za), (xb, lb, zb), (xb, lb - cb, zb), (xa, la - ca, za)])
