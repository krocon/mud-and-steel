"""Mesh builder, model assembly, sockets and material slots.

All geometry is written in *model space*: 1 unit = 1 m, Z up, front +Y, origin
on the ground at the footprint center (docs/ASSETS.md). Parts never create
Blender objects themselves; they add faces to the `MeshBuilder` of a named
object (`model.mb("Hull")`) and register sockets. `Model.finish()` then creates
the objects with the pivot as object origin, the parent hierarchy, identity
rotation and scale 1, so no transform ever has to be applied afterwards.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Vector

# Material slots of docs/ASSETS.md. The Godot import script replaces them with
# the camouflage shader; the colors here are only for Blender previews.
SLOTS = {
    "Paint": (0.32, 0.34, 0.24),
    "Metal": (0.22, 0.21, 0.20),
    "Rubber": (0.05, 0.05, 0.05),
    "Wood": (0.40, 0.28, 0.16),
    "Canvas": (0.42, 0.40, 0.30),
    "Glass": (0.10, 0.14, 0.16),
    "Skin": (0.70, 0.52, 0.42),
    "Cloth": (0.30, 0.30, 0.22),
}

# Face flags stored as integer face attributes and read by the bake.
FLAG_WEAR = "mas_wear"  # 1 = edge strip that shows worn paint
FLAG_TRACK = "mas_track"  # 1 = track link surface (scrolling link pattern)


def slot_material(name):
    """Returns the shared material for a slot, creating it on first use."""
    if name not in SLOTS:
        raise ValueError(f"unknown material slot {name!r}; allowed: {', '.join(SLOTS)}")
    mat = bpy.data.materials.get(name)
    if mat is None:
        mat = bpy.data.materials.new(name)
        rgb = SLOTS[name]
        mat.diffuse_color = (*rgb, 1.0)
        bsdf = mat.node_tree.nodes.get("Principled BSDF") if mat.node_tree else None
        if bsdf is not None:
            bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
            bsdf.inputs["Roughness"].default_value = 0.35 if name in ("Metal", "Glass") else 0.8
            bsdf.inputs["Metallic"].default_value = 0.6 if name == "Metal" else 0.0
    return mat


def v3(p):
    return Vector((float(p[0]), float(p[1]), float(p[2])))


class MeshBuilder:
    """Collects faces of one object in model space."""

    def __init__(self):
        self.bm = bmesh.new()
        self.wear = self.bm.faces.layers.int.new(FLAG_WEAR)
        self.track = self.bm.faces.layers.int.new(FLAG_TRACK)
        self.uv = self.bm.loops.layers.uv.new("UVMap")
        self.slots = []
        # bevel segments for the next shells: 1 = plate edges, 3+ = cast look
        self.bevel_segments = 1

    # -- low level -----------------------------------------------------------

    def _mat(self, name):
        slot_material(name)  # validates the name
        if name not in self.slots:
            self.slots.append(name)
        return self.slots.index(name)

    def verts(self, points):
        return [self.bm.verts.new(v3(p)) for p in points]

    def face(self, verts, mat, smooth=False, wear=False):
        f = self.bm.faces.new(verts)
        f.material_index = self._mat(mat)
        f.smooth = smooth
        f[self.wear] = 1 if wear else 0
        return f

    def _shell(self, faces, bevel=0.0, outward=True):
        """Fixes normals of a closed shell and optionally bevels its edges."""
        if outward:
            bmesh.ops.recalc_face_normals(self.bm, faces=faces)
        if bevel > 0.0:
            edges = list({e for f in faces for e in f.edges})
            verts = list({v for f in faces for v in f.verts})
            res = bmesh.ops.bevel(
                self.bm,
                geom=verts + edges,
                offset=bevel,
                offset_type="OFFSET",
                segments=self.bevel_segments,
                profile=0.5,
                affect="EDGES",
                clamp_overlap=True,
            )
            for f in res["faces"]:
                f[self.wear] = 1
                f.smooth = self.bevel_segments > 1
            faces = faces + list(res["faces"])
        return faces

    # -- primitives ----------------------------------------------------------

    def box(self, center, size, mat, bevel=0.0, smooth=False):
        """Axis-aligned box given by center and size (x, y, z)."""
        cx, cy, cz = center
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
        corners = [
            (cx + sx * hx, cy + sy * hy, cz + sz * hz)
            for sz in (-1, 1)
            for sy in (-1, 1)
            for sx in (-1, 1)
        ]
        v = self.verts(corners)
        idx = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
        faces = [self.face([v[i] for i in q], mat, smooth) for q in idx]
        return self._shell(faces, bevel)

    def extrude_x(self, profile, x_left, x_right, mat, bevel=0.0, smooth=False):
        """Extrudes a side profile (list of (y, z)) across X.

        `x_left` / `x_right` are numbers or functions `(y, z) -> x`, which gives
        sloped side plates (e.g. a T-34 upper hull) from a single profile.
        """
        fl = x_left if callable(x_left) else (lambda y, z, c=x_left: c)
        fr = x_right if callable(x_right) else (lambda y, z, c=x_right: c)
        left = self.verts([(fl(y, z), y, z) for y, z in profile])
        right = self.verts([(fr(y, z), y, z) for y, z in profile])
        n = len(profile)
        faces = [self.face(left, mat, False), self.face(list(reversed(right)), mat, False)]
        for i in range(n):
            j = (i + 1) % n
            faces.append(self.face([left[i], left[j], right[j], right[i]], mat, smooth))
        return self._shell(faces, bevel)

    def extrude(self, points, direction, mat, bevel=0.0):
        """Extrudes a planar polygon (3D points) along a vector: plates, skirts."""
        d = v3(direction)
        a = self.verts(points)
        b = self.verts([v3(p) + d for p in points])
        n = len(points)
        faces = [self.face(a, mat), self.face(list(reversed(b)), mat)]
        for i in range(n):
            j = (i + 1) % n
            faces.append(self.face([a[i], a[j], b[j], b[i]], mat))
        return self._shell(faces, bevel)

    def loft(self, rings, mat, cap_start=True, cap_end=True, smooth=False, cap_mat=None, bevel=0.0):
        """Connects rings of equal vertex count (lists of 3D points) with quads."""
        vs = [self.verts(r) for r in rings]
        n = len(rings[0])
        faces = []
        for a, b in zip(vs, vs[1:]):
            for i in range(n):
                j = (i + 1) % n
                faces.append(self.face([a[i], a[j], b[j], b[i]], mat, smooth))
        cm = cap_mat or mat
        if cap_start:
            faces.append(self.face(list(reversed(vs[0])), cm))
        if cap_end:
            faces.append(self.face(vs[-1], cm))
        return self._shell(faces, bevel, outward=cap_start and cap_end)

    def cylinder(self, p0, p1, r0, r1=None, seg=12, mat="Paint", cap0=True, cap1=True,
                 smooth=True, cap_mat=None, phase=0.0):
        """Cylinder or frustum from p0 to p1 with radius r0 at p0 and r1 at p1."""
        r1 = r0 if r1 is None else r1
        a, b = v3(p0), v3(p1)
        u, w = _basis(b - a)
        rings = []
        for c, r in ((a, r0), (b, r1)):
            rings.append([
                c + (u * math.cos(phase + 2 * math.pi * i / seg) + w * math.sin(phase + 2 * math.pi * i / seg)) * r
                for i in range(seg)
            ])
        return self.loft(rings, mat, cap0, cap1, smooth, cap_mat)

    def revolve(self, axis_p0, axis_dir, profile, seg=12, mat="Paint", smooth=True, cap_mat=None):
        """Lathe: profile is a list of (t, r) along the axis (t in meters)."""
        a = v3(axis_p0)
        d = v3(axis_dir).normalized()
        u, w = _basis(d)
        rings = []
        for t, r in profile:
            c = a + d * t
            rings.append([c + (u * math.cos(2 * math.pi * i / seg) + w * math.sin(2 * math.pi * i / seg)) * max(r, 1e-4)
                          for i in range(seg)])
        return self.loft(rings, mat, True, True, smooth, cap_mat)

    def gear(self, center, axis_x_width, radius, teeth, tooth_depth, mat="Metal"):
        """Toothed disc around the X axis (drive sprockets)."""
        cx, cy, cz = center
        pts = []
        for i in range(teeth * 2):
            ang = math.pi * i / teeth
            r = radius if i % 2 == 0 else radius - tooth_depth
            pts.append((cy + math.cos(ang) * r, cz + math.sin(ang) * r))
        return self.extrude_x(pts, cx - axis_x_width / 2, cx + axis_x_width / 2, mat)

    # -- output --------------------------------------------------------------

    def project_uvs(self):
        """Box-projects UVs (1 UV unit = 1 m) on faces without explicit UVs."""
        for f in self.bm.faces:
            if f[self.track]:
                continue
            n = f.normal
            ax = max(range(3), key=lambda i: abs(n[i]))
            for loop in f.loops:
                co = loop.vert.co
                uv = (co.y, co.z) if ax == 0 else (co.x, co.z) if ax == 1 else (co.x, co.y)
                loop[self.uv].uv = uv

    def to_mesh(self, name, offset):
        self.bm.normal_update()
        self.project_uvs()
        bmesh.ops.translate(self.bm, verts=list(self.bm.verts), vec=-v3(offset))
        mesh = bpy.data.meshes.new(name)
        self.bm.to_mesh(mesh)
        self.bm.free()
        for s in self.slots:
            mesh.materials.append(slot_material(s))
        return mesh


def _basis(axis):
    d = axis.normalized()
    ref = Vector((0, 0, 1)) if abs(d.z) < 0.9 else Vector((1, 0, 0))
    u = d.cross(ref).normalized()
    w = d.cross(u).normalized()
    return u, w


def surface_matrix(pos, normal, up=(0, 0, 1)):
    """Socket orientation for markings: local +Z = surface normal, +Y = up.

    On faces that point (nearly) up, e.g. wing tops and baseplates, "up" is
    the model front (+Y) instead (prompt 008)."""
    z = v3(normal).normalized()
    y = v3(up)
    if abs(y.normalized().dot(z)) > 0.99:
        y = Vector((0.0, 1.0, 0.0))
    y = (y - z * y.dot(z)).normalized()
    x = y.cross(z)
    m = Matrix((x, y, z)).transposed().to_4x4()
    m.translation = v3(pos)
    return m


class Model:
    """A unit model under construction."""

    def __init__(self, unit_id):
        self.unit_id = unit_id
        self.parts = {}  # name -> dict(parent, pivot, mb)
        self.order = []
        self.sockets = []  # (name, parent, matrix, props)
        self.props = {}
        self.notes = {}  # free values for later parts (e.g. "hull_top")

    def mb(self, name, parent=None, pivot=(0.0, 0.0, 0.0)):
        """Returns the mesh builder of an object, creating it on first use."""
        if name not in self.parts:
            if parent is not None and parent not in self.parts:
                raise KeyError(f"{name}: parent {parent} does not exist yet")
            self.parts[name] = {"parent": parent, "pivot": v3(pivot), "mb": MeshBuilder()}
            self.order.append(name)
        return self.parts[name]["mb"]

    def has(self, name):
        return name in self.parts

    def extent(self, exclude=()):
        """Model-space bounds (lo, hi) of all geometry built so far.

        `exclude` holds fnmatch patterns of object names (e.g. "Skirt_*");
        children of excluded objects are excluded too.
        """
        lo = Vector((math.inf,) * 3)
        hi = Vector((-math.inf,) * 3)
        for name, p in self.parts.items():
            if self.excluded(name, exclude):
                continue
            for v in p["mb"].bm.verts:
                lo = Vector(map(min, lo, v.co))
                hi = Vector(map(max, hi, v.co))
        return lo, hi

    def excluded(self, name, patterns):
        """True if the object or one of its ancestors matches a pattern."""
        import fnmatch

        while name is not None:
            if any(fnmatch.fnmatchcase(name, pat) for pat in patterns):
                return True
            name = self.parts[name]["parent"] if name in self.parts else None
        return False

    def translate(self, d):
        """Moves all geometry, pivots and sockets (e.g. to center the footprint)."""
        d = v3(d)
        for p in self.parts.values():
            bmesh.ops.translate(p["mb"].bm, verts=list(p["mb"].bm.verts), vec=d)
            p["pivot"] += d
        for i, (name, parent, m, props) in enumerate(self.sockets):
            m = m.copy()
            m.translation = m.translation + d
            self.sockets[i] = (name, parent, m, props)

    def pitch(self, angle, level=("Propeller",)):
        """Tilts the whole model nose up by `angle` (radians, around X through
        the origin): geometry, pivots and sockets (prompt 202: aircraft drawn
        level and set down on their tail wheel). Parts whose name starts
        with one of `level` only move with their pivot and keep their axes
        (a propeller must go on spinning around its own Y)."""
        r4 = Matrix.Rotation(angle, 4, "X")
        r3 = r4.to_3x3()
        for name, p in self.parts.items():
            verts = list(p["mb"].bm.verts)
            if name.startswith(tuple(level)):
                d = r3 @ p["pivot"] - p["pivot"]
                bmesh.ops.translate(p["mb"].bm, verts=verts, vec=d)
                p["pivot"] += d
            else:
                bmesh.ops.rotate(p["mb"].bm, verts=verts, cent=(0.0, 0.0, 0.0), matrix=r3)
                p["pivot"] = r3 @ p["pivot"]
        for i, (name, parent, m, props) in enumerate(self.sockets):
            self.sockets[i] = (name, parent, r4 @ m, props)

    def pivot(self, name):
        return self.parts[name]["pivot"].copy()

    def next_index(self, prefix):
        """Next free two-digit index, e.g. MG_01, MG_02."""
        i = 1
        names = set(self.parts) | {s[0] for s in self.sockets}
        while f"{prefix}{i:02d}" in names:
            i += 1
        return f"{prefix}{i:02d}"

    def socket(self, name, parent, pos, matrix=None, **props):
        """Registers an attachment empty. `pos` is in model space."""
        m = matrix.copy() if matrix is not None else Matrix.Translation(v3(pos))
        m.translation = v3(pos)
        self.sockets.append((name, parent, m, props))

    def finish(self, collection=None):
        """Creates the Blender objects. Returns the root object."""
        coll = collection or bpy.context.scene.collection
        root = bpy.data.objects.new(self.unit_id, None)
        root.empty_display_type = "ARROWS"
        coll.objects.link(root)
        for k, v in self.props.items():
            root[k] = v
        objs = {None: root}
        for name in self.order:
            p = self.parts[name]
            mesh = p["mb"].to_mesh(name, p["pivot"])
            obj = bpy.data.objects.new(name, mesh)
            coll.objects.link(obj)
            parent = p["parent"]
            obj.parent = objs[parent]
            base = self.parts[parent]["pivot"] if parent else Vector()
            obj.location = p["pivot"] - base
            objs[name] = obj
        for name, parent, m, props in self.sockets:
            emp = bpy.data.objects.new(name, None)
            emp.empty_display_type = "SINGLE_ARROW" if name.startswith("Socket_Marking") else "PLAIN_AXES"
            emp.empty_display_size = 0.15
            coll.objects.link(emp)
            emp.parent = objs[parent]
            base = self.parts[parent]["pivot"] if parent else Vector()
            local = m.copy()
            local.translation = m.translation - base
            emp.matrix_basis = local
            for k, v in props.items():
                emp[k] = v
            objs[name] = emp
        return root


def tri_count(obj):
    if obj.type != "MESH":
        return 0
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def descendants(root):
    out = []
    stack = list(root.children)
    while stack:
        o = stack.pop()
        out.append(o)
        stack.extend(o.children)
    return out


def world_bbox(objects, depsgraph=None):
    """Axis-aligned bounds (min, max) of the given mesh objects in world space."""
    lo = Vector((math.inf,) * 3)
    hi = Vector((-math.inf,) * 3)
    for o in objects:
        if o.type != "MESH":
            continue
        mw = o.matrix_world
        for v in o.data.vertices:
            p = mw @ v.co
            lo = Vector(map(min, lo, p))
            hi = Vector(map(max, hi, p))
    return lo, hi
