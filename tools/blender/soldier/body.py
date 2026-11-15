"""The base body: one skinned low-poly mesh, 1.80 m from sole to the top of the head.

Uniform styles change the mesh (tunic length, breeches, boots, gaiters or leggings).
Stature and face variants are shape keys (`Stocky`, `Slim`, `FaceB`, `FaceC`) so one
exported model gives several looks; Godot sets them per instance.
"""

from mathutils import Matrix, Vector

from .mesh import MeshBuilder, primary_bone
from .skeleton import BONES

BODY_HEIGHT = 1.80
HEAD_CENTER = Vector((0.0, 0.015, 1.695))
HEAD_RADII = (0.082, 0.098, 0.105)


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


def _torso(b, style):
    hem = style["tunic_hem"]
    flare = style.get("tunic_flare", 0.0)
    rings = [
        ((0, -0.005, hem), 0.170 + flare, 0.122 + flare, _w(("Hips", 1.0))),
        ((0, -0.010, 0.92), 0.176, 0.124, _w(("Hips", 1.0))),
        ((0, -0.005, 1.00), 0.170, 0.117, _w(("Hips", 0.6), ("Spine", 0.4))),
        ((0, 0.000, 1.08), 0.160, 0.110, _w(("Spine", 1.0))),
        ((0, 0.008, 1.18), 0.170, 0.116, _w(("Spine", 0.3), ("Chest", 0.7))),
        ((0, 0.012, 1.30), 0.186, 0.124, _w(("Chest", 0.4), ("UpperChest", 0.6))),
        ((0, 0.005, 1.40), 0.196, 0.122, _w(("UpperChest", 1.0))),
        ((0, -0.005, 1.465), 0.165, 0.105, _w(("UpperChest", 1.0))),
        ((0, 0.000, 1.505), 0.075, 0.068, _w(("UpperChest", 0.5), ("Neck", 0.5))),
    ]
    b.tube(rings, "Cloth", sides=10, cap_end=False)
    # Collar.
    b.tube([((0, 0.002, 1.49), 0.078, 0.072, _w(("Neck", 0.5), ("UpperChest", 0.5))),
            ((0, 0.004, 1.535), 0.068, 0.064, _w(("Neck", 1.0)))],
           "Cloth", sides=8, cap_start=False, cap_end=False)


def _neck_head(b, face_tint):
    b.tube([((0, 0.0, 1.48), 0.054, 0.052, _w(("Neck", 1.0))),
            ((0, 0.008, 1.62), 0.052, 0.054, _w(("Neck", 0.4), ("Head", 0.6)))],
           "Skin", sides=8, cap_start=False, cap_end=False)
    weights = _w(("Head", 1.0))

    def shape(unit, co):
        # Flatter back of the head, a little jaw.
        if unit.y < -0.3:
            co = co + Vector((0, 0.012 * unit.y, 0))
        if unit.z < -0.4 and unit.y > 0:
            co = co + Vector((0, 0.01, 0))
        return co

    b.ellipsoid(HEAD_CENTER, HEAD_RADII, "Skin", weights, rings=7, sides=10, shape=shape)
    # Nose, brow and ears.
    b.box((0, 0.112, 1.685), (0.022, 0.03, 0.04), "Skin", weights, taper=0.6)
    b.box((0, 0.098, 1.72), (0.11, 0.02, 0.02), "Skin", weights)
    for sx in (-1, 1):
        b.box((sx * 0.082, 0.005, 1.69), (0.016, 0.035, 0.05), "Skin", weights)


def _arm(b, side, style):
    p = "Left" if side < 0 else "Right"
    up_h, up_t, up_d = _limb_axes(f"{p}UpperArm")
    lo_h, lo_t, lo_d = _limb_axes(f"{p}LowerArm")
    sh = f"{p}Shoulder"
    ua, la, hand = f"{p}UpperArm", f"{p}LowerArm", f"{p}Hand"
    rings = [
        (up_h - up_d * 0.02, 0.062, 0.064, _w((sh, 0.4), (ua, 0.6))),
        (up_h + up_d * 0.10, 0.058, 0.060, _w((ua, 1.0))),
        (up_h + up_d * 0.22, 0.050, 0.052, _w((ua, 1.0))),
        (lo_h, 0.045, 0.047, _w((ua, 0.5), (la, 0.5))),
        (lo_h + lo_d * 0.12, 0.046, 0.046, _w((la, 1.0))),
        (lo_t + lo_d * 0.01, 0.039, 0.037, _w((la, 0.7), (hand, 0.3))),
    ]
    b.tube(rings, "Cloth", sides=7)
    # Hand: palm faces the body in the rest pose.
    h_h, h_t, h_d = _limb_axes(hand)
    fwd = Vector((0, 1, 0))
    thick = h_d.cross(fwd).normalized()
    rot = _rot_from_axes(thick, h_d, fwd)
    b.box(h_h + h_d * 0.07, (0.035, 0.13, 0.08), "Skin", _w((hand, 1.0)), rot=rot)
    b.box(h_h + h_d * 0.04 + fwd * 0.045, (0.03, 0.06, 0.025), "Skin", _w((hand, 1.0)), rot=rot)


def _leg(b, side, style):
    p = "Left" if side < 0 else "Right"
    ul, ll, foot, toes = f"{p}UpperLeg", f"{p}LowerLeg", f"{p}Foot", f"{p}Toes"
    hip, knee = _bone(ul)
    _, ankle = _bone(ll)
    br = style.get("breeches", 0.0)
    trouser = style.get("trouser_tint", (1, 1, 1))

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

    profile = [  # z, rx, ry (cloth)
        (0.97, 0.090, 0.090),
        (0.84, 0.084 + br * 0.4, 0.086 + br * 0.5),
        (0.72, 0.074 + br, 0.078 + br),
        (0.60, 0.062 + br * 0.5, 0.066 + br * 0.5),
        (knee.z, 0.056, 0.058),
        (0.40, 0.054, 0.058),
        (0.28, 0.046, 0.050),
        (0.16, 0.044, 0.046),
        (0.10, 0.044, 0.046),
    ]
    boot_top = style["boot_top"]
    gaiter_top = style.get("gaiter_top", 0.0)
    cloth_end = max(boot_top, gaiter_top)
    cloth = [r for r in profile if r[0] > cloth_end + 0.02]
    cloth.append((cloth_end - 0.01, *[v for v in _interp(profile, cloth_end - 0.01)]))
    b.tube([(at(z), rx, ry, wts(z), trouser) for z, rx, ry in cloth], "Cloth", sides=8,
           cap_start=False)
    if gaiter_top > boot_top:
        g = [(z, rx, ry) for z, rx, ry in profile if boot_top < z < gaiter_top]
        top = _interp(profile, gaiter_top)
        rows = [(gaiter_top, top[0] + 0.006, top[1] + 0.006)]
        rows += [(z, rx + 0.006, ry + 0.006) for z, rx, ry in g]
        bot = _interp(profile, boot_top + 0.02)
        rows.append((boot_top + 0.02, bot[0] + 0.006, bot[1] + 0.006))
        b.tube([(at(z), rx, ry, wts(z)) for z, rx, ry in rows], style.get("gaiter_slot", "Canvas"),
               sides=8)
    # Boot shaft.
    rows = [(boot_top, *[v + 0.008 for v in _interp(profile, boot_top)])]
    rows += [(z, rx + 0.008, ry + 0.008) for z, rx, ry in profile if 0.1 < z < boot_top]
    rows.append((0.10, 0.052, 0.056))
    b.tube([(at(z), rx, ry, wts(z)) for z, rx, ry in rows], "Leather", sides=8)
    # Shoe: heel to ball (Foot), ball to toe (Toes).
    x = hip.x
    b.box((x, 0.03, 0.055), (0.10, 0.20, 0.11), "Leather", _w((foot, 1.0)), taper=0.85)
    b.box((x, 0.165, 0.035), (0.095, 0.10, 0.07), "Leather", _w((toes, 1.0)), taper=0.8)


def _interp(profile, z):
    for (z0, a0, b0), (z1, a1, b1) in zip(profile, profile[1:]):
        if z0 >= z >= z1:
            t = (z0 - z) / (z0 - z1)
            return (a0 + (a1 - a0) * t, b0 + (b1 - b0) * t)
    return profile[-1][1:]


def build_body(style):
    b = MeshBuilder(["Cloth", "Skin", "Leather", "Canvas"])
    _torso(b, style)
    _neck_head(b, None)
    for side in (-1, 1):
        _arm(b, side, style)
        _leg(b, side, style)
    return b


# -- shape keys ----------------------------------------------------------------------

_STATURE_FACTORS = {
    # bone -> (stocky, slim) scale of the distance to the bone axis
    "Hips": (1.12, 0.92), "Spine": (1.16, 0.90), "Chest": (1.12, 0.92),
    "UpperChest": (1.10, 0.93), "Neck": (1.10, 0.92),
}


def _stature_factor(bone, which):
    i = 0 if which == "Stocky" else 1
    if bone in _STATURE_FACTORS:
        return _STATURE_FACTORS[bone][i]
    if "Arm" in bone or "Leg" in bone or "Shoulder" in bone:
        return (1.10, 0.92)[i]
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
