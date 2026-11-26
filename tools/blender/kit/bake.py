"""Vertex-color bake shared by all models.

Every mesh gets a corner color attribute `Color` (exported as glTF COLOR_0)
that the Godot shader `unit_camo.gdshader` reads:

- R: ambient occlusion (1 = open, 0 = fully occluded), ray traced against the
  assembly plus the ground plane (or a rotationally symmetric turret-ring disc
  for rotating turrets/mounts, prompt 214), so wheels darken the hull and the
  undersides are dark without freezing moving shadows onto the hull. Back faces
  do not occlude.
- G: edge wear mask (1 on bevel strips of plate edges).
- B: height above ground, model-space Z / 4 m (mud and dust from below).
- A: 1 - track flag (0 on track link surfaces: scrolling link pattern).

Smooth-shaded polygons (`poly.use_smooth == True`) sample AO along their smooth
corner normal from a symmetric vertex offset so adjacent smooth faces share the
same `Color` value and glTF export does not split smooth vertices (prompt 214).
Flat-shaded polygons keep per-face normals and slight face-center offsets for
crisp AO contrast along hard armor edges.

Deterministic: fixed sample directions, no randomness.
"""

import math
import re

from mathutils import Vector
from mathutils.bvhtree import BVHTree

from .core import FLAG_TRACK, FLAG_WEAR

SAMPLES = 32
MAX_DIST = 1.5
HEIGHT_RANGE = 4.0

# Moving vehicle assemblies that must not bake frozen shadows into the static
# hull (or receive frozen shadows from it), prompt 214.
_ROTATING_RE = re.compile(r"^(Turret|Mount|GunTurret_\d\d)$")
_DETACHED_RE = re.compile(r"^(Gun|Ramp|Bridge|Propeller(_[LR])?|Bomb_\d\d|Rocket_\d\d)$")
_GEAR_RE = re.compile(r"^Gear_(L|R|Tail|Nose)$")
_SHADOW_PROXY = "ShadowProxy"


def _directions(n):
    """Fibonacci points on the upper hemisphere (z >= 0)."""
    golden = math.pi * (3 - math.sqrt(5))
    out = []
    for i in range(n):
        z = 1 - (i + 0.5) / n
        r = math.sqrt(max(0.0, 1 - z * z))
        a = golden * i
        out.append(Vector((math.cos(a) * r, math.sin(a) * r, z)))
    return out


def _frame(n):
    ref = Vector((0, 0, 1)) if abs(n.z) < 0.9 else Vector((1, 0, 0))
    t = n.cross(ref).normalized()
    b = n.cross(t)
    return t, b


def _first_front_hit(tree, origin, d):
    """Distance to the first surface facing the ray, or None.

    Back faces are skipped: corners buried inside another part (overlapping
    primitives are common in the kit) would otherwise come out black and
    darken the visible part of their faces.
    """
    travelled = 0.0
    o = origin
    for _ in range(6):
        loc, nrm, _idx, dist = tree.ray_cast(o, d, MAX_DIST - travelled)
        if loc is None:
            return None
        travelled += dist
        if nrm.dot(d) < 0.0:
            return travelled
        o = loc + d * 1e-4
        travelled += 1e-4
    return None


def _disc_occluder(cx, cy, z, radius, segments=24):
    """Rotationally symmetric horizontal disc facing +Z (turret-ring deck proxy)."""
    verts = [Vector((cx, cy, z))]
    for i in range(segments):
        a = 2.0 * math.pi * i / segments
        verts.append(Vector((cx + math.cos(a) * radius, cy + math.sin(a) * radius, z)))
    polys = [[0, 1 + i, 1 + (i + 1) % segments] for i in range(segments)]
    return verts, polys


def _bake_group(meshes, ground=True, extra_occluders=None):
    """Bakes vertex AO/wear/height/track for one mutually occluding mesh group."""
    if not meshes:
        return
    verts, polys = [], []
    for o in meshes:
        mw = o.matrix_world
        base = len(verts)
        verts += [mw @ v.co for v in o.data.vertices]
        polys += [[base + i for i in p.vertices] for p in o.data.polygons]
    if ground:
        g = 50.0
        base = len(verts)
        verts += [Vector((-g, -g, 0)), Vector((g, -g, 0)), Vector((g, g, 0)), Vector((-g, g, 0))]
        polys.append([base, base + 1, base + 2, base + 3])
    if extra_occluders:
        for ev, ep in extra_occluders:
            base = len(verts)
            verts += ev
            polys += [[base + i for i in p] for p in ep]
    tree = BVHTree.FromPolygons(verts, polys, epsilon=0.0)
    dirs = _directions(SAMPLES)

    for o in meshes:
        me = o.data
        mw = o.matrix_world
        nm = mw.to_3x3().inverted().transposed()
        wear = me.attributes.get(FLAG_WEAR)
        track = me.attributes.get(FLAG_TRACK)
        if "Color" in me.color_attributes:
            me.color_attributes.remove(me.color_attributes["Color"])
        col = me.color_attributes.new("Color", "FLOAT_COLOR", "CORNER")
        me.color_attributes.active_color = col
        # Corner normals exist in Blender 4.1+/5.x and preserve smooth/split edges.
        cnorms = me.corner_normals
        cache = {}
        for poly in me.polygons:
            smooth = poly.use_smooth
            w = wear.data[poly.index].value if wear else 0
            tr = track.data[poly.index].value if track else 0
            if not smooth:
                fn = (nm @ poly.normal).normalized()
                center = mw @ poly.center
                ft, fb = _frame(fn)
            for li in poly.loop_indices:
                vi = me.loops[li].vertex_index
                p = mw @ me.vertices[vi].co
                if smooth:
                    raw_n = cnorms[li].vector
                    if raw_n.length_squared < 1e-12:
                        raw_n = me.vertices[vi].normal
                    n = (nm @ raw_n).normalized()
                    key = (vi, 1, round(n.x, 2), round(n.y, 2), round(n.z, 2))
                    if key not in cache:
                        t, b = _frame(n)
                        # Symmetric offset along the smooth normal: no poly.center
                        # bias, so all smooth corners sharing vertex `vi` get the
                        # exact same AO value and stay merged in glTF.
                        origin = p + n * 0.004
                        occ = 0.0
                        total = 0.0
                        for d in dirs:
                            wd = t * d.x + b * d.y + n * d.z
                            weight = d.z
                            total += weight
                            dist = _first_front_hit(tree, origin, wd)
                            if dist is not None:
                                occ += weight * (1.0 - dist / MAX_DIST) ** 0.5
                        cache[key] = 1.0 - occ / total
                else:
                    n = fn
                    key = (vi, 0, round(n.x, 2), round(n.y, 2), round(n.z, 2))
                    if key not in cache:
                        origin = p + n * 0.004 + (center - p) * 0.03
                        occ = 0.0
                        total = 0.0
                        for d in dirs:
                            wd = ft * d.x + fb * d.y + n * d.z
                            weight = d.z
                            total += weight
                            dist = _first_front_hit(tree, origin, wd)
                            if dist is not None:
                                occ += weight * (1.0 - dist / MAX_DIST) ** 0.5
                        cache[key] = 1.0 - occ / total
                ao = cache[key]
                h = max(0.0, min(1.0, p.z / HEIGHT_RANGE))
                col.data[li].color = (ao, float(w), h, 0.0 if tr else 1.0)


def bake(objects):
    """Single-BVH AO bake (used by env.py per variant)."""
    meshes = [o for o in objects if o.type == "MESH"]
    _bake_group(meshes, ground=True)


def _assembly_key(obj):
    """Classifies a vehicle mesh into static, rotating, detached, gear, or proxy."""
    if obj.name == _SHADOW_PROXY:
        return ("proxy", obj.name)
    cur = obj
    while cur is not None:
        if _DETACHED_RE.match(cur.name):
            return ("detached", cur.name)
        if _GEAR_RE.match(cur.name):
            return ("gear", cur.name)
        if _ROTATING_RE.match(cur.name):
            return ("rotating", cur.name)
        cur = cur.parent
    return ("static", "static")


def bake_vehicle(objects):
    """Decoupled vehicle AO bake (prompt 214):

    - Static hull group (`Hull`, `Carriage`, tracks, wheels, skirts, fixed props)
      bakes against itself + ground plane (no moving parts in occluders).
    - Rotating groups (`Turret`, `Mount`, `GunTurret_NN` with their fixed
      children such as `Cupola`, `TurretSkirt`, turret MGs/hatches) each bake
      against themselves + a rotationally symmetric deck disc at turret-ring
      height, giving uniform contact AO at every yaw angle.
    - Independently moving/detachable parts (`Gun`, `Ramp`, `Bridge`,
      `Propeller*`, `Bomb_*`, `Rocket_*`) bake self-AO only (no ground plane).
    - Landing gear (`Gear_*`) bakes per leg with ground plane (never darkens
      the fuselage/wing in flight).
    - `ShadowProxy` bakes alone without occluding the aircraft.
    """
    meshes = [o for o in objects if o.type == "MESH"]
    by_name = {o.name: o for o in objects}
    groups = {}
    for o in meshes:
        groups.setdefault(_assembly_key(o), []).append(o)

    if ("static", "static") in groups:
        _bake_group(groups[("static", "static")], ground=True)

    for (kind, name), group_meshes in sorted(groups.items()):
        if kind == "static":
            continue
        if kind == "rotating":
            # Place a rotationally symmetric deck disc right under the rotating
            # group centered on its pivot XY so contact AO rotates seamlessly.
            root_obj = by_name.get(name)
            piv = root_obj.matrix_world.translation if root_obj else Vector()
            pts = [o.matrix_world @ v.co for o in group_meshes for v in o.data.vertices]
            if pts:
                z_min = min(p.z for p in pts) - 0.002
                max_r = max(math.hypot(p.x - piv.x, p.y - piv.y) for p in pts)
                disc = _disc_occluder(piv.x, piv.y, z_min, max_r + 1.5)
                _bake_group(group_meshes, ground=False, extra_occluders=[disc])
            else:
                _bake_group(group_meshes, ground=False)
        elif kind == "gear":
            _bake_group(group_meshes, ground=True)
        else:
            # detached (Gun, Ramp, Bridge, Propeller*, Bomb_*, Rocket_*) or proxy
            _bake_group(group_meshes, ground=False)
