"""A small mesh builder: primitives with material slot, skin weights and vertex color.

Every vertex carries a weight dict {bone: weight}, so the result can be skinned to the
soldier armature directly. Coordinates are meters, Z up, front +Y, right +X.
"""

import math

import bpy
from mathutils import Matrix, Vector


def _perp_axes(direction):
    """Two unit axes perpendicular to direction: u (sideways) and v (forward-ish)."""
    d = Vector(direction).normalized()
    ref = Vector((0.0, 1.0, 0.0))
    if abs(d.dot(ref)) > 0.95:
        ref = Vector((0.0, 0.0, 1.0)) if d.y < 0 else Vector((0.0, 0.0, -1.0))
    v = (ref - d * ref.dot(d)).normalized()
    u = v.cross(d).normalized()
    return u, v


class MeshBuilder:
    def __init__(self, slots):
        self.slots = list(slots)
        self.verts = []  # Vector
        self.weights = []  # dict bone -> weight
        self.colors = []  # (r, g, b)
        self.faces = []  # (indices, slot index)

    # -- low level ---------------------------------------------------------------

    def slot(self, name):
        if name not in self.slots:
            self.slots.append(name)
        return self.slots.index(name)

    def vert(self, co, weights, color=(1.0, 1.0, 1.0)):
        self.verts.append(Vector(co))
        self.weights.append(dict(weights))
        self.colors.append(tuple(color))
        return len(self.verts) - 1

    def face(self, indices, slot):
        self.faces.append((tuple(indices), self.slot(slot)))

    def triangles(self):
        return sum(len(f) - 2 for f, _ in self.faces)

    # -- primitives ----------------------------------------------------------------

    def tube(self, rings, slot, sides=8, cap_start=True, cap_end=True, color=(1, 1, 1),
             phase=0.0):
        """Rings: list of (center, rx, ry, weights[, color]). rx is along the side axis
        u, ry along the forward axis v of the ring plane (perpendicular to the tube)."""
        ring_idx = []
        n = len(rings)
        for i, ring in enumerate(rings):
            center, rx, ry, weights = ring[:4]
            col = ring[4] if len(ring) > 4 else color
            center = Vector(center)
            prev_c = Vector(rings[max(i - 1, 0)][0])
            next_c = Vector(rings[min(i + 1, n - 1)][0])
            direction = next_c - prev_c
            if direction.length < 1e-9:
                direction = Vector((0, 0, 1))
            u, v = _perp_axes(direction)
            idx = []
            for s in range(sides):
                a = 2.0 * math.pi * (s + phase) / sides
                co = center + u * (math.cos(a) * rx) + v * (math.sin(a) * ry)
                idx.append(self.vert(co, weights, col))
            ring_idx.append(idx)
        for i in range(n - 1):
            a, b = ring_idx[i], ring_idx[i + 1]
            for s in range(sides):
                t = (s + 1) % sides
                self.face((a[s], a[t], b[t], b[s]), slot)
        if cap_start:
            self._fix_cap(ring_idx[0], slot, rings[0][0], rings[1][0], start=True)
        if cap_end:
            self._fix_cap(ring_idx[-1], slot, rings[-1][0], rings[-2][0], start=False)
        return ring_idx

    def _fix_cap(self, ring, slot, center, neighbour, start):
        # Winding: the cap normal must point away from the neighbouring ring.
        idx = list(ring)
        normal = self._poly_normal(idx)
        outward = Vector(center) - Vector(neighbour)
        if normal.dot(outward) < 0:
            idx.reverse()
        self.face(idx, slot)

    def _poly_normal(self, idx):
        n = Vector((0, 0, 0))
        pts = [self.verts[i] for i in idx]
        for i in range(len(pts)):
            a, b = pts[i], pts[(i + 1) % len(pts)]
            n.x += (a.y - b.y) * (a.z + b.z)
            n.y += (a.z - b.z) * (a.x + b.x)
            n.z += (a.x - b.x) * (a.y + b.y)
        return n

    def box(self, center, size, slot, weights, rot=None, color=(1, 1, 1), taper=1.0):
        """Box with size (x, y, z) around center; rot is a 3x3 Matrix (local axes).
        taper scales the +Z face in x and y."""
        m = rot if rot is not None else Matrix.Identity(3)
        sx, sy, sz = (s / 2.0 for s in size)
        idx = []
        for z in (-1, 1):
            k = taper if z > 0 else 1.0
            for y in (-1, 1):
                for x in (-1, 1):
                    local = Vector((x * sx * k, y * sy * k, z * sz))
                    idx.append(self.vert(Vector(center) + m @ local, weights, color))
        # indices: (x,y,z) -> i = x + 2y + 4z with 0/1
        quads = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
        for q in quads:
            self.face([idx[i] for i in q], slot)
        return idx

    def cylinder(self, p0, p1, radius, slot, weights, sides=8, color=(1, 1, 1), r1=None):
        r1 = radius if r1 is None else r1
        return self.tube([(p0, radius, radius, weights), (p1, r1, r1, weights)], slot,
                         sides=sides, color=color)

    def revolve(self, profile, center, slot, weights, sides=10, scale=(1.0, 1.0),
                color=(1, 1, 1), close_top=True, offset_y=0.0):
        """Surface of revolution around a vertical axis through center.
        profile: list of (radius, z) from bottom to top; scale stretches x/y."""
        center = Vector(center)
        rings = []
        for r, z in profile:
            ring = []
            for s in range(sides):
                a = 2.0 * math.pi * s / sides
                co = center + Vector((math.cos(a) * r * scale[0],
                                      math.sin(a) * r * scale[1] + offset_y * r, z))
                ring.append(self.vert(co, weights, color))
            rings.append(ring)
        for i in range(len(rings) - 1):
            a, b = rings[i], rings[i + 1]
            for s in range(sides):
                t = (s + 1) % sides
                self.face((a[s], a[t], b[t], b[s]), slot)
        if close_top:
            top = self.vert(center + Vector((0, 0, profile[-1][1] + 0.0)), weights, color)
            last = rings[-1]
            for s in range(sides):
                t = (s + 1) % sides
                self.face((last[s], last[t], top), slot)
        return rings

    def ellipsoid(self, center, radii, slot, weights, rings=6, sides=8, color=(1, 1, 1),
                  shape=None):
        """UV ellipsoid. shape(co_unit, co) may return a displaced coordinate."""
        center = Vector(center)
        bottom = self.vert(center - Vector((0, 0, radii[2])), weights, color)
        grid = []
        for i in range(1, rings):
            phi = math.pi * i / rings - math.pi / 2
            ring = []
            for s in range(sides):
                a = 2.0 * math.pi * s / sides + math.pi / 2
                unit = Vector((math.cos(phi) * math.cos(a), math.cos(phi) * math.sin(a), math.sin(phi)))
                co = center + Vector((unit.x * radii[0], unit.y * radii[1], unit.z * radii[2]))
                if shape is not None:
                    co = shape(unit, co)
                ring.append(self.vert(co, weights, color))
            grid.append(ring)
        top = self.vert(center + Vector((0, 0, radii[2])), weights, color)
        for s in range(sides):
            t = (s + 1) % sides
            self.face((grid[0][t], grid[0][s], bottom), slot)
            self.face((grid[-1][s], grid[-1][t], top), slot)
        for i in range(len(grid) - 1):
            a, b = grid[i], grid[i + 1]
            for s in range(sides):
                t = (s + 1) % sides
                self.face((a[s], a[t], b[t], b[s]), slot)
        return grid

    def merge(self, other, transform=None, weights=None):
        """Appends another builder, optionally transformed and re-weighted."""
        base = len(self.verts)
        for co, w, c in zip(other.verts, other.weights, other.colors):
            co = transform @ co if transform is not None else co
            self.vert(co, weights if weights is not None else w, c)
        for f, s in other.faces:
            self.face([i + base for i in f], other.slots[s])

    # -- output --------------------------------------------------------------------

    def to_object(self, name, materials, collection, armature=None, shape_keys=None):
        """Creates the mesh object. materials: dict slot name -> bpy material.
        shape_keys: dict key name -> list of vertex coordinates (same order)."""
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata([tuple(v) for v in self.verts], [], [f for f, _ in self.faces])
        for slot in self.slots:
            mesh.materials.append(materials[slot])
        for poly, (_, s) in zip(mesh.polygons, self.faces):
            poly.material_index = s
            poly.use_smooth = False
        # Simple projected UVs (front view, 1 m = 1 UV unit) so the export has texture
        # coordinates; real atlas UVs come with textures later.
        uv = mesh.uv_layers.new(name="UVMap")
        for loop in mesh.loops:
            co = self.verts[loop.vertex_index]
            uv.data[loop.index].uv = (co.x + co.y * 0.5 + 0.5, co.z * 0.5)
        attr = mesh.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
        for loop in mesh.loops:
            r, g, b = self.colors[loop.vertex_index]
            attr.data[loop.index].color = (r, g, b, 1.0)
        mesh.validate()
        mesh.update()
        obj = bpy.data.objects.new(name, mesh)
        collection.objects.link(obj)
        if armature is not None:
            groups = {}
            for i, w in enumerate(self.weights):
                for bone, value in w.items():
                    if value <= 0:
                        continue
                    if bone not in groups:
                        groups[bone] = obj.vertex_groups.new(name=bone)
                    groups[bone].add([i], value, "REPLACE")
            obj.parent = armature
            mod = obj.modifiers.new("Armature", "ARMATURE")
            mod.object = armature
        if shape_keys:
            obj.shape_key_add(name="Basis", from_mix=False)
            for key_name, coords in shape_keys.items():
                key = obj.shape_key_add(name=key_name, from_mix=False)
                for i, co in enumerate(coords):
                    key.data[i].co = co
        return obj


def primary_bone(weights):
    return max(weights.items(), key=lambda kv: kv[1])[0]
