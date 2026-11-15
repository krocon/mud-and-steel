"""Vertex-color bake shared by all models.

Every mesh gets a corner color attribute `Color` (exported as glTF COLOR_0)
that the Godot shader `unit_camo.gdshader` reads:

- R: ambient occlusion (1 = open, 0 = fully occluded), ray traced against the
  whole model plus the ground plane, so wheels darken the hull and the
  undersides are dark. Back faces do not occlude.
- G: edge wear mask (1 on bevel strips of plate edges).
- B: height above ground, model-space Z / 4 m (mud and dust from below).
- A: 1 - track flag (0 on track link surfaces: scrolling link pattern).

Deterministic: fixed sample directions, no randomness.
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

from .core import FLAG_TRACK, FLAG_WEAR

SAMPLES = 32
MAX_DIST = 1.5
HEIGHT_RANGE = 4.0


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


def bake(objects):
    meshes = [o for o in objects if o.type == "MESH"]
    verts, polys = [], []
    for o in meshes:
        mw = o.matrix_world
        base = len(verts)
        verts += [mw @ v.co for v in o.data.vertices]
        polys += [[base + i for i in p.vertices] for p in o.data.polygons]
    # ground plane
    g = 50.0
    base = len(verts)
    verts += [Vector((-g, -g, 0)), Vector((g, -g, 0)), Vector((g, g, 0)), Vector((-g, g, 0))]
    polys.append([base, base + 1, base + 2, base + 3])
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
        cache = {}
        for poly in me.polygons:
            n = (nm @ poly.normal).normalized()
            center = mw @ poly.center
            t, b = _frame(n)
            w = wear.data[poly.index].value if wear else 0
            tr = track.data[poly.index].value if track else 0
            for li in poly.loop_indices:
                vi = me.loops[li].vertex_index
                key = (vi, round(n.x, 2), round(n.y, 2), round(n.z, 2))
                p = mw @ me.vertices[vi].co
                if key not in cache:
                    origin = p + n * 0.004 + (center - p) * 0.03
                    occ = 0.0
                    total = 0.0
                    for d in dirs:
                        wd = (t * d.x + b * d.y + n * d.z)
                        weight = d.z
                        total += weight
                        dist = _first_front_hit(tree, origin, wd)
                        if dist is not None:
                            occ += weight * (1.0 - dist / MAX_DIST) ** 0.5
                    cache[key] = 1.0 - occ / total
                ao = cache[key]
                h = max(0.0, min(1.0, p.z / HEIGHT_RANGE))
                col.data[li].color = (ao, float(w), h, 0.0 if tr else 1.0)
