"""The base body: one skinned low-poly mesh, 1.80 m from sole to the top of the head.

Proportions follow dressed figures (wool uniform over the body, not the bare body):
short neck inside the collar, broad sloping shoulders, a tunic cinched by the belt,
wide trousers bloused over gaiters or boots, sturdy boots.

Uniform styles change the mesh (tunic length, pockets, breeches, boots, gaiters or
leggings). Stature and face variants are shape keys (`Stocky`, `Slim`, `FaceB`,
`FaceC`) so one exported model gives several looks; Godot sets them per instance.
"""

import math

from mathutils import Matrix, Vector

from .mesh import MeshBuilder, primary_bone
from .skeleton import BONES

BODY_HEIGHT = 1.80
HEAD_RADII = (0.085, 0.103, 0.118)
HEAD_CENTER = Vector((0.0, 0.020, BODY_HEIGHT - HEAD_RADII[2]))

# Torso from the tunic hem to the base of the neck: z, center y, half width, half depth.
# The hem row is replaced per uniform style (tunic_hem, tunic_flare).
_TORSO = [
    (0.80, -0.008, 0.200, 0.150),  # hem
    (0.92, -0.010, 0.198, 0.148),  # hips, tunic skirt
    (0.985, -0.006, 0.186, 0.138),
    (1.02, -0.004, 0.180, 0.134),  # belt line
    (1.09, 0.000, 0.180, 0.132),
    (1.20, 0.008, 0.190, 0.138),
    (1.31, 0.012, 0.204, 0.144),  # chest
    (1.39, 0.006, 0.208, 0.138),  # shoulders
    (1.445, -0.002, 0.180, 0.120),
    (1.485, -0.004, 0.125, 0.100),  # trapezius slope
    (1.515, -0.002, 0.084, 0.084),  # base of the neck
]
_TORSO_WEIGHTS = [
    {"Hips": 1.0}, {"Hips": 1.0}, {"Hips": 0.7, "Spine": 0.3}, {"Hips": 0.5, "Spine": 0.5},
    {"Spine": 1.0}, {"Spine": 0.3, "Chest": 0.7}, {"Chest": 0.4, "UpperChest": 0.6},
    {"UpperChest": 1.0}, {"UpperChest": 1.0}, {"UpperChest": 0.8, "Neck": 0.2},
    {"UpperChest": 0.5, "Neck": 0.5},
]


def _w(*pairs):
    return dict(pairs)


def _bone(name):
    return Vector(BONES[name][1]), Vector(BONES[name][2])


def _limb_axes(name):
    head, tail = _bone(name)
    d = (tail - head).normalized()
    return head, tail, d


def _rot_from_axes(x, y, z):
    return Matrix((x, y, z)).transposed()


def torso_at(z):
    """Center y, half width and half depth of the torso at height z (clamped to the
    torso). Gear is fitted to this profile (gear.py)."""
    rows = _TORSO
    if z <= rows[0][0]:
        return rows[0][1:]
    for (z0, *a), (z1, *b) in zip(rows, rows[1:]):
        if z <= z1:
            t = (z - z0) / (z1 - z0)
            return tuple(p + (q - p) * t for p, q in zip(a, b))
    return rows[-1][1:]


def torso_front(x, z, back=False):
    """y of the tunic surface at side offset x and height z (front, or the back)."""
    cy, rx, ry = torso_at(z)
    k = math.sqrt(max(0.0, 1.0 - (x / rx) ** 2))
    return cy - ry * k if back else cy + ry * k


def torso_top(x):
    """z of the shoulder slope at side offset x (between the shoulder and the neck)."""
    rows = _TORSO[7:]
    for (z0, _, rx0, _), (z1, _, rx1, _) in zip(rows, rows[1:]):
        if rx0 >= x >= rx1:
            return z0 + (z1 - z0) * (rx0 - x) / (rx0 - rx1)
    return rows[-1][0] if x < rows[-1][2] else rows[0][0]


def _torso(b, style):
    hem = style["tunic_hem"]
    flare = style.get("tunic_flare", 0.0)
    rows = list(_TORSO)
    rows[0] = (hem, rows[0][1], rows[1][2] + flare, rows[1][3] + flare)
    rings = [((0, cy, z), rx, ry, w) for (z, cy, rx, ry), w in zip(rows, _TORSO_WEIGHTS)]
    b.tube(rings, "Cloth", sides=12, cap_end=False)
    # Fall collar: closed at the throat, a little higher at the nape.
    neck = _w(("Neck", 0.6), ("UpperChest", 0.4))
    b.tube([((0, -0.002, 1.495), 0.090, 0.090, neck),
            ((0, 0.000, 1.525), 0.082, 0.084, neck),
            ((0, 0.000, 1.550), 0.074, 0.078, _w(("Neck", 1.0)))],
           "Cloth", sides=10, cap_start=False, cap_end=False, color=(0.9, 0.9, 0.9))
    _tunic_details(b, style)


def _patch(b, x, z, size, weights, color=(0.9, 0.9, 0.9), lift=0.004):
    """A flat patch (pocket, flap) lying on the front of the tunic."""
    w, h = size
    y = torso_front(x, z)
    # Tilt with the surface: sideways around Z, and with the slope in Z.
    cy, rx, ry = torso_at(z)
    yaw = math.atan2(ry * x / rx, rx * math.sqrt(max(1e-6, 1.0 - (x / rx) ** 2)))
    slope = math.atan2(torso_front(x, z + 0.04) - torso_front(x, z - 0.04), 0.08)
    rot = Matrix.Rotation(-yaw, 3, "Z") @ Matrix.Rotation(-slope, 3, "X")
    b.box((x, y + lift, z), (w, 0.012, h), "Cloth", weights, rot=rot, color=color)


def _tunic_details(b, style):
    chest = _w(("Chest", 0.5), ("UpperChest", 0.5))
    hips = _w(("Hips", 1.0))
    for sx in (-1, 1):
        if style.get("breast_pockets", True):
            _patch(b, sx * 0.105, 1.285, (0.095, 0.105), chest)
            _patch(b, sx * 0.105, 1.335, (0.105, 0.035), chest, color=(0.82, 0.82, 0.82),
                   lift=0.009)
        if style.get("skirt_pockets", False) and style["tunic_hem"] < 0.84:
            _patch(b, sx * 0.115, 0.895, (0.115, 0.12), hips)
            _patch(b, sx * 0.115, 0.95, (0.125, 0.035), hips, color=(0.82, 0.82, 0.82),
                   lift=0.009)
        if style.get("shoulder_straps", True):
            # Shoulder strap from the collar to the sleeve seam, along the slope.
            a = Vector((sx * 0.085, 0.0, 1.512))
            c = Vector((sx * 0.200, 0.0, 1.452))
            d = (c - a)
            x_axis = d.normalized()
            y_axis = Vector((0, 1, 0))
            rot = _rot_from_axes(x_axis, y_axis, x_axis.cross(y_axis))
            b.box((a + c) / 2 + Vector((0, 0.0, 0.006)), (d.length, 0.052, 0.012), "Cloth",
                  _w(("UpperChest", 1.0)), rot=rot, color=style.get("strap_tint", (0.8, 0.8, 0.8)))


def _neck_head(b):
    b.tube([((0, -0.004, 1.49), 0.062, 0.064, _w(("Neck", 1.0))),
            ((0, 0.010, 1.62), 0.060, 0.066, _w(("Neck", 0.4), ("Head", 0.6)))],
           "Skin", sides=8, cap_start=False, cap_end=False)
    weights = _w(("Head", 1.0))
    rx, ry, rz = HEAD_RADII

    def shape(unit, co):
        # Flatter back of the head, a jaw instead of an egg, the chin forward.
        if unit.y < -0.3:
            co = co + Vector((0, 0.014 * unit.y, 0))
        if unit.z < -0.25:
            k = min(1.0, (-unit.z - 0.25) / 0.5)
            co = co + Vector((unit.x * 0.008 * k, 0, 0))
            if unit.y > 0:
                co = co + Vector((0, 0.022 * k * unit.y, 0.010 * k))
        return co

    b.ellipsoid(HEAD_CENTER, HEAD_RADII, "Skin", weights, rings=8, sides=10, shape=shape)
    c = HEAD_CENTER
    front = c.y + ry
    # Nose, brow, eyes (dark), ears.
    b.box((0, front + 0.001, c.z - 0.020), (0.026, 0.030, 0.044), "Skin", weights, taper=0.55)
    b.box((0, front - 0.014, c.z + 0.022), (0.118, 0.022, 0.02), "Skin", weights)
    for sx in (-1, 1):
        b.box((sx * 0.034, front - 0.011, c.z + 0.004), (0.026, 0.01, 0.012), "Skin", weights,
              color=(0.22, 0.17, 0.14))
        b.box((sx * (rx - 0.001), c.y - 0.010, c.z - 0.016), (0.014, 0.030, 0.050), "Skin", weights,
              color=(0.9, 0.9, 0.9))


def _arm(b, side, style):
    p = "Left" if side < 0 else "Right"
    up_h, up_t, up_d = _limb_axes(f"{p}UpperArm")
    lo_h, lo_t, lo_d = _limb_axes(f"{p}LowerArm")
    sh = f"{p}Shoulder"
    ua, la, hand = f"{p}UpperArm", f"{p}LowerArm", f"{p}Hand"
    # Sleeve: rounded shoulder cap, wide upper arm, open cuff.
    rings = [
        (up_h - up_d * 0.060, 0.040, 0.046, _w((sh, 0.7), (ua, 0.3))),
        (up_h - up_d * 0.015, 0.070, 0.076, _w((sh, 0.4), (ua, 0.6))),
        (up_h + up_d * 0.090, 0.070, 0.074, _w((ua, 1.0))),
        (up_h + up_d * 0.210, 0.062, 0.064, _w((ua, 1.0))),
        (lo_h, 0.058, 0.060, _w((ua, 0.5), (la, 0.5))),
        (lo_h + lo_d * 0.110, 0.058, 0.058, _w((la, 1.0))),
        (lo_t - lo_d * 0.030, 0.051, 0.049, _w((la, 1.0))),
        (lo_t + lo_d * 0.012, 0.051, 0.049, _w((la, 0.7), (hand, 0.3))),
    ]
    b.tube(rings, "Cloth", sides=8)
    # Hand: palm faces the body in the rest pose; wrist, palm with fingers, thumb.
    h_h, h_t, h_d = _limb_axes(hand)
    fwd = Vector((0, 1, 0))
    thick = h_d.cross(fwd).normalized()
    rot = _rot_from_axes(thick, h_d, fwd)
    hw = _w((hand, 1.0))
    b.box(h_h + h_d * 0.015, (0.040, 0.05, 0.062), "Skin", hw, rot=rot)
    b.box(h_h + h_d * 0.090, (0.042, 0.125, 0.092), "Skin", hw, rot=rot)
    b.box(h_h + h_d * 0.075 + fwd * 0.056, (0.034, 0.075, 0.030), "Skin", hw, rot=rot)


# Trouser leg: z, half width, half depth (cloth). Wide, straight-cut wool trousers.
_LEG = [
    (0.97, 0.100, 0.106),
    (0.86, 0.100, 0.108),
    (0.74, 0.093, 0.100),
    (0.62, 0.084, 0.091),
    (0.50, 0.076, 0.082),
    (0.40, 0.073, 0.080),
    (0.30, 0.067, 0.073),
    (0.20, 0.060, 0.065),
    (0.10, 0.056, 0.061),
]


def _leg(b, side, style):
    p = "Left" if side < 0 else "Right"
    ul, ll, foot, toes = f"{p}UpperLeg", f"{p}LowerLeg", f"{p}Foot", f"{p}Toes"
    hip, knee = _bone(ul)
    _, ankle = _bone(ll)
    br = style.get("breeches", 0.0)
    trouser = style.get("trouser_tint", (1, 1, 1))
    boot_top = style["boot_top"]
    gaiter_top = style.get("gaiter_top", 0.0)
    cloth_end = max(boot_top, gaiter_top)
    blouse = style.get("blouse", 0.020)
    sides = 8

    def at(z):
        """Point on the leg axis at height z (hip to ankle)."""
        if z >= knee.z:
            t = (hip.z - z) / (hip.z - knee.z)
            return hip.lerp(knee, t)
        t = (knee.z - z) / (knee.z - ankle.z)
        return knee.lerp(ankle, t)

    def wts(z):
        if z > 0.85:
            return _w(("Hips", 0.35), (ul, 0.65))
        if z > knee.z + 0.04:
            return _w((ul, 1.0))
        if z > knee.z - 0.04:
            return _w((ul, 0.5), (ll, 0.5))
        if z > 0.14:
            return _w((ll, 1.0))
        return _w((ll, 0.7), (foot, 0.3))

    def leg(z):
        return _interp(_LEG, z)

    def cloth(z):
        """Trouser radii: breeches flare at the thigh, cloth bags over the boot top."""
        rx, ry = leg(z)
        if br:
            f = max(0.0, 1.0 - abs(z - 0.72) / 0.20)
            rx, ry = rx + br * f, ry + br * f
        bag = blouse * max(0.0, 1.0 - abs(z - (cloth_end + 0.055)) / 0.10)
        return rx + bag, ry + bag

    zs = [z for z, _, _ in _LEG if z > cloth_end + 0.09]
    zs += [cloth_end + 0.055, cloth_end + 0.012]
    rows = [(at(z), *cloth(z), wts(z), trouser) for z in zs]
    # The trouser end is tucked into the gaiter or boot.
    tuck = leg(cloth_end - 0.01)
    rows.append((at(cloth_end - 0.01), tuck[0], tuck[1], wts(cloth_end - 0.01), trouser))
    b.tube(rows, "Cloth", sides=sides, cap_start=False)

    def shaft(top, bottom, grow, slot):
        zs = [top] + [z for z, _, _ in _LEG if bottom < z < top - 0.03] + [bottom]
        rows = []
        for i, z in enumerate(zs):
            rx, ry = leg(z)
            g = grow * (0.6 if i == 0 else 1.0)  # the top edge hugs the leg
            rows.append((at(z), rx + g, ry + g, wts(z)))
        b.tube(rows, slot, sides=sides, cap_start=False, cap_end=False)

    if gaiter_top > boot_top:
        shaft(gaiter_top, boot_top - 0.02, 0.008, style.get("gaiter_slot", "Canvas"))
    shaft(boot_top, 0.085, 0.011 if gaiter_top > boot_top else 0.009, "Leather")
    _boot(b, hip.x, foot, toes)


def _boot(b, x, foot, toes):
    """Boot from the heel to the toe cap: a tube along +Y with a flat sole."""
    flat = math.cos(math.pi / 8)  # an 8-gon turned half a step has flat sides
    rows = [  # y, half width, half height, weights
        (-0.088, 0.036, 0.034, _w((foot, 1.0))),
        (-0.070, 0.054, 0.060, _w((foot, 1.0))),
        (-0.010, 0.060, 0.074, _w((foot, 1.0))),
        (0.060, 0.062, 0.062, _w((foot, 1.0))),
        (0.125, 0.064, 0.048, _w((foot, 0.5), (toes, 0.5))),
        (0.180, 0.060, 0.044, _w((toes, 1.0))),
        (0.214, 0.042, 0.034, _w((toes, 1.0))),
    ]
    b.tube([((x, y, ry * flat), rx / flat, ry, w) for y, rx, ry, w in rows], "Leather", sides=8,
           phase=0.5)
    # Heel and sole edge.
    b.box((x, -0.045, 0.014), (0.108, 0.085, 0.028), "Leather", _w((foot, 1.0)),
          color=(0.8, 0.8, 0.8))


def _interp(profile, z):
    if z >= profile[0][0]:
        return profile[0][1:]
    for (z0, a0, b0), (z1, a1, b1) in zip(profile, profile[1:]):
        if z0 >= z >= z1:
            t = (z0 - z) / (z0 - z1)
            return (a0 + (a1 - a0) * t, b0 + (b1 - b0) * t)
    return profile[-1][1:]


def build_body(style):
    b = MeshBuilder(["Cloth", "Skin", "Leather", "Canvas"])
    _torso(b, style)
    _neck_head(b)
    for side in (-1, 1):
        _arm(b, side, style)
        _leg(b, side, style)
    return b


# -- shape keys ----------------------------------------------------------------------

_STATURE_FACTORS = {
    # bone -> (stocky, slim) scale of the distance to the bone axis
    "Hips": (1.10, 0.93), "Spine": (1.14, 0.91), "Chest": (1.10, 0.93),
    "UpperChest": (1.08, 0.94), "Neck": (1.08, 0.93),
}


def _stature_factor(bone, which):
    i = 0 if which == "Stocky" else 1
    if bone in _STATURE_FACTORS:
        return _STATURE_FACTORS[bone][i]
    if "Arm" in bone or "Leg" in bone or "Shoulder" in bone:
        return (1.08, 0.92)[i]
    return 1.0


def stature_offset(co, weights, which):
    """Displaced coordinate of a vertex for the stature shape key `Stocky` / `Slim`."""
    co = Vector(co)
    bone = primary_bone(weights)
    f = _stature_factor(bone, which)
    if f == 1.0:
        return co
    head, tail = _bone(bone)
    if bone in _STATURE_FACTORS:
        # Torso: scale around the vertical body axis, belly a bit more for Stocky.
        axis_pt = Vector((0.0, 0.0, co.z))
        off = co - axis_pt
        if which == "Stocky" and bone == "Spine" and off.y > 0:
            off.y *= 1.08
        return axis_pt + off * f
    d = tail - head
    t = max(0.0, min(1.0, (co - head).dot(d) / d.length_squared))
    axis_pt = head + d * t
    return axis_pt + (co - axis_pt) * f


def face_offset(co, weights, which):
    co = Vector(co)
    if primary_bone(weights) != "Head":
        return co
    rel = co - HEAD_CENTER
    if which == "FaceB":
        # Broad face, heavy jaw and nose.
        if rel.z < 0.0:
            rel.x *= 1.12
        if rel.y > 0.08:
            rel.y *= 1.08
    else:
        # Long narrow face, chin forward.
        rel.x *= 0.93
        if rel.z < -0.03:
            rel.z *= 1.08
            rel.y += 0.008
    return HEAD_CENTER + rel


def shape_keys_for(builder, include_face=True):
    keys = {}
    for which in ("Stocky", "Slim"):
        keys[which] = [stature_offset(co, w, which) for co, w in zip(builder.verts, builder.weights)]
    if include_face:
        for which in ("FaceB", "FaceC"):
            keys[which] = [face_offset(co, w, which) for co, w in zip(builder.verts, builder.weights)]
    return keys
