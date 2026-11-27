"""Environment objects: variants, foliage cards, env bake (prompt 007).

Environment objects (vegetation, props, buildings, bridges, fences and
fortifications, catalog faction `neutral`) are built like units from a
blueprint, but with their own structure (docs/ASSETS.md, "Environment
objects"):

    <catalog id>                 root empty, glTF extras with catalog values
    ├── <Variant>                mesh: one instanceable mesh per variant
    ├── <Variant>_Far            optional distant version of a tree variant
    └── <Variant>                empty for modular variants (buildings) ...
        ├── <Variant>_Walls      ... with one mesh per part: Walls, Roof,
        ├── <Variant>_Roof           Floor, Debris, Tower, Deck, ...
        └── <Variant>_Floor

All variants stand at the origin; the gallery and the map spread them out.
Generators live in `vegetation`, `parts_neutral` and `buildings` and are
registered with `@env_part`. They receive the target box (length along Y,
width along X, height along Z) and build roughly into it; `fit` scales the
finished variant exactly into the box unless the variant sets `fit = false`.

Vertex colors (attribute `Color`, glTF COLOR_0) for the Godot env shaders:
- R: ambient occlusion (ray traced; foliage: position inside the crown)
- G: Bark/Foliage = wind weight (0 = rigid, 1 = moves most); others = edge wear
- B: Bark/Foliage = wind phase (per card or branch); others = height / 4 m
- A: material kind of the slot, A = 1 - kind / 4 (kind 0..3, see SLOT_KINDS)

Foliage faces use explicit UVs in a *virtual atlas*: the integer part of U
selects the procedural pattern in `env_foliage.gdshader` (CARD_* below), the
fraction is the position on the card. There are no texture files.
"""

import math
import random

import bmesh
import bpy
from mathutils import Vector

from . import core
from .core import MeshBuilder, v3
from .registry import Params

ENV_KIT_VERSION = 1

# Environment material slots (docs/ASSETS.md, "Material slots"). Registered in
# the shared slot table at import so that MeshBuilder accepts them; the colors
# are Blender preview colors only.
ENV_SLOTS = {
    "Bark": (0.25, 0.19, 0.13),
    "Foliage": (0.20, 0.30, 0.12),
    "Stone": (0.45, 0.43, 0.40),
    "Plaster": (0.72, 0.68, 0.60),
    "Ground": (0.30, 0.24, 0.17),
    "Roof": (0.45, 0.20, 0.14),
    "Rock": (0.50, 0.48, 0.44),
}
for _name, _rgb in ENV_SLOTS.items():
    core.SLOTS.setdefault(_name, _rgb)

# Slots an environment model may use (validate_env.py).
ALLOWED_SLOTS = ("Paint", "Metal", "Rubber", "Wood", "Canvas", "Glass") + tuple(ENV_SLOTS)

# Material kinds per slot (vertex color A). Kind 0 is the default.
SLOT_KINDS = {
    "Bark": ("oak", "birch", "palm", "pine"),
    "Stone": ("fieldstone", "ashlar", "concrete", "brick"),
    "Plaster": ("lime", "adobe", "whitewash", "ochre"),
    "Roof": ("clay_tile", "slate", "thatch", "planks"),
    "Wood": ("weathered", "fresh", "dark", "charred"),
    "Paint": ("field_grey", "warning_red", "white", "rust"),
    "Metal": ("steel", "rust", "galvanized", "dark"),
    "Ground": ("earth", "spoil", "mud", "sand"),
    "Canvas": ("hessian", "tent"),
    "Foliage": ("leaf", "needle", "dry", "dark"),
    "Rock": ("bedrock", "scree"),
}

# Foliage card patterns (integer part of U, env_foliage.gdshader).
CARD_BROADLEAF = 0
CARD_NEEDLE = 1
CARD_PALM = 2
CARD_GRASS = 3
CARD_FLOWERS = 4
CARD_SHRUB = 5
CARD_TWIGS = 6
CARD_SOLID = 7
# 8 far crown, 9 far conifer: vegetation.py. Prompt 120: small silver-grey
# olive leaves (also the far crown of olives).
CARD_OLIVE = 10
CARD_SUCCULENT = 11  # prompt 202: solid, cactus green (kind 0) or agave grey-green (kind 1)
CARD_PINE_TUFT = 12  # prompt 202: round needle tufts (stone pine)
CARD_VINE = 13  # prompt 202: large fresh green leaves (vineyard)
CARD_BLOSSOM = 14  # prompt 241: blossoming magenta/rose ornamental tree cluster
CARD_CROWN_BLOSSOM = 15  # prompt 241: far blossoming tree crown
CARD_SUNFLOWER = 16  # prompt 241: yellow sunflower stand with dark centers and green leaves

FLAG_UV = "env_uv"  # 1 = face has explicit UVs (cards), keep them
FLAG_KIND = "env_kind"  # material kind 0..3
FLAG_PHASE = "env_phase"  # wind phase id (cards, branches)
FLAG_WIND = "env_wind"  # 0 = rigid, 1 = plant part that sways

# Triangle budgets (docs/ASSETS.md) per catalog category, per variant.
BUDGETS = {
    "env_tree": 2000, "env_bush": 500, "env_grass": 50, "env_rock": 300,
    "env_building": 3000, "env_bridge": 3000, "env_fence": 2000, "env_pole": 800,
    "env_prop": 800,
    "fort_bunker": 2000, "fort_trench": 2000, "fort_sandbag": 2000, "fort_obstacle": 2000,
    "fort_wire": 2000, "fort_minefield_at": 800, "fort_minefield_ap": 800,
}
BUDGET_OVERRIDES = {"env_church": 5000, "env_church_large": 5000, "env_church_brick": 5000, "env_dump_ammo": 1500, "env_dump_fuel": 1500,
                    "env_mosque": 5000, "env_police_post": 5000,
                    "env_rock_outcrop": 1500, "env_rock_massif": 3000, "env_rock_cliff": 4000,
                    # Prompt 078: apartment blocks (100-170 windows, interiors when damaged).
                    "env_block_desert": 6000, "env_block_desert_long": 6000, "env_block_town": 6000,
                    # Prompt 202: American city blocks (fire escapes, large windows).
                    "env_block_tenement": 6000, "env_loft_iron": 6000, "env_warehouse_quay": 6000,
                    # Prompt 120: four-storey corner house and the public building
                    # (shutters at every window, shop fronts).
                    "env_house_med_row_corner": 4500, "env_town_hall_med": 6000,
                    # Prompt 127: the 36 m castle range (like the church).
                    "env_castle_hall": 5000,
                    # Prompt 202, group E: stone blocks, palace, cathedral, station hall, headframe.
                    "env_block_tenement_stone": 6000, "env_palace": 6000, "env_cathedral": 6000,
                    "env_station_hall": 5000, "env_mine_headframe": 4000, "env_station": 4500,
                    "env_spoil_heap": 600}
FAR_BUDGET = 200

# Surfaces per building (prompt 186): the optional `[surface]` table of a
# blueprint, stored as the glTF extra `env_surface` (JSON). The names must
# match EnvLook.PLASTERS / BRICKS / BONDS / ROOFS in godot/assets/shaders/
# env_look.gd (their index is what the shader gets).
SURFACE_NAMES = {
    "plaster": ("lime", "cream", "ochre", "ochre_red", "pink", "grey_green", "white"),
    "brick": ("red", "yellow", "dark"),
    "bond": ("stretcher", "english"),
    "roof": ("clay", "mission", "beaver", "spanish", "slate"),
}
SURFACE_RANGES = {"spall": (0.0, 1.0), "roof_row": (0.0, 0.6), "roof_missing": (0.0, 0.4), "moss": (0.0, 3.0)}


def surface_props(table, where):
    """Checks a blueprint's [surface] table; returns it as a plain dict."""
    out = {}
    for key, value in table.items():
        if key in SURFACE_NAMES:
            if value not in SURFACE_NAMES[key]:
                raise ValueError(f"{where}: surface.{key} = {value!r}; known: {', '.join(SURFACE_NAMES[key])}")
            out[key] = value
        elif key in SURFACE_RANGES:
            low, high = SURFACE_RANGES[key]
            if not isinstance(value, (int, float)) or not low <= value <= high:
                raise ValueError(f"{where}: surface.{key} = {value!r}; a number from {low} to {high}")
            out[key] = float(value)
        else:
            known = sorted(list(SURFACE_NAMES) + list(SURFACE_RANGES))
            raise KeyError(f"{where}: unknown surface key {key!r}; known: {', '.join(known)}")
    return out
# Far meshes of rock ridge and cliff pieces (prompts 074, 243).
FAR_BUDGET_OVERRIDES = {"env_rock_massif": 300, "env_rock_cliff": 450}


def far_budget(u):
    return FAR_BUDGET_OVERRIDES.get(u["id"], FAR_BUDGET)

ENV_PARTS = {}


def env_part(name):
    """Registers an environment generator `fn(model, group, params, box)`."""

    def deco(fn):
        if name in ENV_PARTS:
            raise ValueError(f"env part {name} registered twice")
        ENV_PARTS[name] = fn
        return fn

    return deco


def budget(u):
    return BUDGET_OVERRIDES.get(u["id"], BUDGETS.get(u["category"], 2000))


class Box:
    """Target box of a variant: length (Y), width (X), height (Z) in meters."""

    def __init__(self, length, width, height):
        self.length, self.width, self.height = length, width, height

    @property
    def hl(self):
        return self.length / 2

    @property
    def hw(self):
        return self.width / 2

    def __repr__(self):
        return f"Box(L {self.length:.3f}, W {self.width:.3f}, H {self.height:.3f})"


class EnvMeshBuilder(MeshBuilder):
    """MeshBuilder with explicit-UV cards and per-face kind / wind flags."""

    def __init__(self):
        super().__init__()
        self.uvflag = self.bm.faces.layers.int.new(FLAG_UV)
        self.kind = self.bm.faces.layers.int.new(FLAG_KIND)
        self.phase = self.bm.faces.layers.int.new(FLAG_PHASE)
        self.windf = self.bm.faces.layers.int.new(FLAG_WIND)
        self.cur_kind = 0
        self.cur_phase = 0
        self.cur_wind = 0

    def face(self, verts, mat, smooth=False, wear=False):
        f = super().face(verts, mat, smooth, wear)
        f[self.kind] = self.cur_kind
        f[self.phase] = self.cur_phase
        f[self.windf] = self.cur_wind
        return f

    def style(self, kind=None, phase=None, wind=None):
        """Sets kind / wind phase / wind flag for the faces that follow."""
        if kind is not None:
            self.cur_kind = kind
        if phase is not None:
            self.cur_phase = phase
        if wind is not None:
            self.cur_wind = 1 if wind else 0
        return self

    def quad_uv(self, corners, uvs, mat, smooth=True):
        """One quad with explicit UVs (corners counter-clockwise)."""
        v = self.verts(corners)
        f = self.face(v, mat, smooth)
        f[self.uvflag] = 1
        for loop, uv in zip(f.loops, uvs):
            loop[self.uv].uv = uv
        return f

    def card(self, center, right, up, w, h, pattern, mat="Foliage", anchor="center"):
        """Alpha card: w x h quad spanned by unit vectors `right` and `up`.

        `pattern` is the CARD_* atlas cell. `anchor` = "center" or "bottom".
        """
        c, r, u = v3(center), v3(right).normalized() * (w / 2), v3(up).normalized()
        if anchor == "bottom":
            b, t = c, c + u * h
        else:
            b, t = c - u * (h / 2), c + u * (h / 2)
        p = float(pattern)
        return self.quad_uv([b - r, b + r, t + r, t - r],
                            [(p + 0.001, 0.001), (p + 0.999, 0.001), (p + 0.999, 0.999), (p + 0.001, 0.999)],
                            mat)

    def project_uvs(self):
        for f in self.bm.faces:
            if f[self.uvflag]:
                continue
            n = f.normal
            ax = max(range(3), key=lambda i: abs(n[i]))
            for loop in f.loops:
                co = loop.vert.co
                uv = (co.y, co.z) if ax == 0 else (co.x, co.z) if ax == 1 else (co.x, co.y)
                loop[self.uv].uv = uv

    # -- shapes used by several generators -----------------------------------

    def blob(self, center, radii, mat, seg=10, rings=6, rng=None, rough=0.15, flat_bottom=None, smooth=True):
        """Lat-long ellipsoid with noise; vertices below `flat_bottom` are clamped."""
        rng = rng or random.Random(0)
        c = v3(center)
        noise = {}
        rows = []
        for i in range(rings + 1):
            th = math.pi * i / rings
            row = []
            for j in range(seg if 0 < i < rings else 1):
                ph = 2 * math.pi * j / seg
                key = (i, j)
                noise[key] = 1.0 + (rng.random() - 0.5) * 2 * rough
                d = Vector((math.sin(th) * math.cos(ph), math.sin(th) * math.sin(ph), math.cos(th)))
                p = c + Vector((d.x * radii[0], d.y * radii[1], d.z * radii[2])) * noise[key]
                if flat_bottom is not None and p.z < flat_bottom:
                    p.z = flat_bottom
                row.append(p)
            rows.append(self.verts(row))
        faces = []
        top, bottom = rows[0][0], rows[-1][0]
        for j in range(seg):
            k = (j + 1) % seg
            faces.append(self.face([top, rows[1][k], rows[1][j]][::-1], mat, smooth))
            faces.append(self.face([bottom, rows[-2][j], rows[-2][k]][::-1], mat, smooth))
        for i in range(1, rings - 1):
            a, b = rows[i], rows[i + 1]
            for j in range(seg):
                k = (j + 1) % seg
                faces.append(self.face([a[j], a[k], b[k], b[j]][::-1], mat, smooth))
        bmesh.ops.recalc_face_normals(self.bm, faces=faces)
        return faces

    def tube(self, points, radii, seg, mat, cap_start=False, cap_end=True, smooth=True):
        """Tapered tube along a polyline (branches, trunks, logs)."""
        pts = [v3(p) for p in points]
        rings = []
        for i, p in enumerate(pts):
            if i == 0:
                d = pts[1] - pts[0]
            elif i == len(pts) - 1:
                d = pts[-1] - pts[-2]
            else:
                d = (pts[i + 1] - pts[i - 1])
            u, w = core._basis(d)
            r = radii[i]
            rings.append([p + (u * math.cos(2 * math.pi * k / seg) + w * math.sin(2 * math.pi * k / seg)) * r
                          for k in range(seg)])
        return self.loft(rings, mat, cap_start, cap_end, smooth)

    def oriented_box(self, center, x_axis, y_axis, size, mat, bevel=0.0):
        """Box with arbitrary orientation (x/y axes in model space)."""
        c = v3(center)
        ax = v3(x_axis).normalized()
        ay = v3(y_axis)
        ay = (ay - ax * ay.dot(ax)).normalized()
        az = ax.cross(ay)
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
        corners = [c + ax * sx * hx + ay * sy * hy + az * sz * hz
                   for sz in (-1, 1) for sy in (-1, 1) for sx in (-1, 1)]
        v = self.verts(corners)
        idx = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
        faces = [self.face([v[i] for i in q], mat) for q in idx]
        return self._shell(faces, bevel)

    def prism(self, polygon, z0, z1, mat, bevel=0.0, cap_bottom=True):
        """Vertical prism from a plan polygon [(x, y)] between z0 and z1."""
        a = self.verts([(x, y, z0) for x, y in polygon])
        b = self.verts([(x, y, z1) for x, y in polygon])
        n = len(polygon)
        faces = [self.face(list(reversed(b)), mat)]
        if cap_bottom:
            faces.append(self.face(a, mat))
        for i in range(n):
            j = (i + 1) % n
            faces.append(self.face([a[i], a[j], b[j], b[i]], mat))
        return self._shell(faces, bevel, outward=cap_bottom)

    def transform_faces(self, faces, matrix):
        verts = list({v for f in faces for v in f.verts})
        bmesh.ops.transform(self.bm, matrix=matrix, verts=verts)


class EnvModel(core.Model):
    """A Model with variant groups (empties) between root and meshes."""

    def __init__(self, unit_id):
        super().__init__(unit_id)
        self.groups = []  # names of variant empties, in order
        self.group_of = {}  # mesh part -> group name (or None)
        self.part_props = {}  # object name -> extras

    def mb(self, name, parent=None, pivot=(0.0, 0.0, 0.0)):
        if name not in self.parts:
            self.parts[name] = {"parent": None, "pivot": v3(pivot), "mb": EnvMeshBuilder()}
            self.order.append(name)
            self.group_of[name] = parent
        return self.parts[name]["mb"]

    def group(self, name):
        if name not in self.groups:
            self.groups.append(name)
        return name

    def drop(self, name):
        """Removes a mesh part again (e.g. empty debris)."""
        p = self.parts.pop(name)
        p["mb"].bm.free()
        self.order.remove(name)
        self.group_of.pop(name, None)

    def variant_parts(self, variant):
        if variant in self.groups:
            return [n for n in self.order if self.group_of.get(n) == variant]
        return [variant] if variant in self.parts else []

    def finish(self, collection=None):
        coll = collection or bpy.context.scene.collection
        root = bpy.data.objects.new(self.unit_id, None)
        root.empty_display_type = "ARROWS"
        coll.objects.link(root)
        for k, v in self.props.items():
            root[k] = v
        objs = {}
        for g in self.groups:
            e = bpy.data.objects.new(g, None)
            e.empty_display_type = "CUBE"
            e.empty_display_size = 0.5
            coll.objects.link(e)
            e.parent = root
            objs[g] = e
            for k, v in self.part_props.get(g, {}).items():
                e[k] = v
        for name in self.order:
            p = self.parts[name]
            mesh = p["mb"].to_mesh(name, p["pivot"])
            obj = bpy.data.objects.new(name, mesh)
            coll.objects.link(obj)
            obj.parent = objs.get(self.group_of.get(name), root)
            obj.location = p["pivot"]
            for k, v in self.part_props.get(name, {}).items():
                obj[k] = v
            objs[name] = obj
        for name, parent, m, props in self.sockets:
            emp = bpy.data.objects.new(name, None)
            emp.empty_display_type = "PLAIN_AXES"
            emp.empty_display_size = 0.15
            coll.objects.link(emp)
            emp.parent = objs.get(parent, root)
            emp.matrix_basis = m
            for k, v in props.items():
                emp[k] = v
        return root


# -- fitting -------------------------------------------------------------------

def _verts_of(model, names):
    out = []
    for n in names:
        out += list(model.parts[n]["mb"].bm.verts)
    return out


def extent(model, names, sunken=False):
    vs = _verts_of(model, names)
    if sunken:
        vs = [v for v in vs if v.co.z <= 1e-4] or vs
    lo = Vector((min(v.co.x for v in vs), min(v.co.y for v in vs), min(v.co.z for v in vs)))
    hi = Vector((max(v.co.x for v in vs), max(v.co.y for v in vs), max(v.co.z for v in vs)))
    return lo, hi


def fit(model, names, box, sunken=False, center="bounds", embed=0.0):
    """Scales the parts exactly into `box` (scale 1 objects, vertex transform).

    Non-sunken: geometry below the ground is clamped to z = 0, then z is
    scaled so that the top reaches the box height. Sunken objects keep
    z = 0 as the terrain surface and are scaled to reach down to -height.
    Horizontally the parts are scaled to the box size and moved so that the
    bounds center (`center = "bounds"`) or the center of the ground contact
    (`"ground"`, trees: the trunk base) sits on the origin.

    Embedded objects (`embed` > 0, rocks of prompt 074): z = 0 is the
    ground, the foot reaches down to -embed and keeps its depth; only the
    part above the ground is scaled to the box height.
    """
    vs = _verts_of(model, names)
    if embed > 0.0:
        for v in vs:
            v.co.z = max(v.co.z, -embed)
    elif not sunken:
        for v in vs:
            if v.co.z < 0.0:
                v.co.z = 0.0
        zmin = min(v.co.z for v in vs)
        for v in vs:
            v.co.z -= zmin
    lo, hi = extent(model, names, sunken)
    size = hi - lo
    sx = box.width / size.x if size.x > 1e-6 else 1.0
    sy = box.length / size.y if size.y > 1e-6 else 1.0
    if sunken:
        sz = box.height / -lo.z if lo.z < -1e-6 else 1.0
    else:
        sz = box.height / hi.z if hi.z > 1e-6 else 1.0
    cx, cy = (lo.x + hi.x) / 2, (lo.y + hi.y) / 2
    if center == "ground" and not sunken:
        g = [v for v in vs if v.co.z < 0.03]
        if g:
            cx = (min(v.co.x for v in g) + max(v.co.x for v in g)) / 2
            cy = (min(v.co.y for v in g) + max(v.co.y for v in g)) / 2
    for v in vs:
        v.co.x = (v.co.x - cx) * sx
        v.co.y = (v.co.y - cy) * sy
        if embed > 0.0:
            if v.co.z > 0:
                v.co.z = v.co.z * sz
        elif not sunken or v.co.z < 0:
            v.co.z = v.co.z * sz
    return Vector((sx, sy, sz))


def ground(model, names):
    """Moves the parts so that the lowest point is at z = 0."""
    lo, _ = extent(model, names)
    for v in _verts_of(model, names):
        v.co.z -= lo.z


# -- bake ------------------------------------------------------------------------

def _hash01(i):
    return ((i * 0.6180339887) + 0.13) % 1.0


def bake_env(objects):
    """Kit AO bake, then env channels (wind, phase, kind)."""
    from .bake import bake

    meshes = [o for o in objects if o.type == "MESH"]
    # All variants stand at the origin: bake each variant (a direct child of
    # the root with its parts) on its own, so they do not shade each other
    # (prompt 059; models built before keep their bake until rebuilt).
    groups = {}
    for o in meshes:
        top = o
        while top.parent is not None and top.parent.parent is not None:
            top = top.parent
        groups.setdefault(top.name, []).append(o)
    for name in sorted(groups):
        bake(groups[name])
    for o in meshes:
        me = o.data
        col = me.color_attributes["Color"]
        kind = me.attributes.get(FLAG_KIND)
        phase = me.attributes.get(FLAG_PHASE)
        windf = me.attributes.get(FLAG_WIND)
        mats = [s.material.name if s.material else "" for s in o.material_slots]
        zs = [v.co.z for v in me.vertices] or [0.0]
        zmin, zmax = min(zs), max(zs)
        span = max(zmax - zmin, 1e-3)
        fol = [v.co for p in me.polygons if mats[p.material_index] == "Foliage" for v in
               (me.vertices[i] for i in p.vertices)]
        if fol:
            fc = sum(fol, Vector()) / len(fol)
            fr = max(max((p - fc).length for p in fol), 1e-3)
        for poly in me.polygons:
            slot = mats[poly.material_index] if poly.material_index < len(mats) else ""
            k = kind.data[poly.index].value if kind else 0
            ph = phase.data[poly.index].value if phase else 0
            plant = bool(windf.data[poly.index].value) if windf else False
            for li in poly.loop_indices:
                r, g, b, _a = col.data[li].color
                p = me.vertices[me.loops[li].vertex_index].co
                if plant:
                    h = (p.z - zmin) / span
                    if slot == "Foliage":
                        g = min(1.0, 0.35 + 0.65 * h)
                        r = 0.45 + 0.55 * min(1.0, (p - fc).length / fr) if fol else r
                        r = max(r, 0.3)
                    else:
                        g = min(1.0, h * h)
                    b = _hash01(ph)
                col.data[li].color = (r, g, b, 1.0 - k / 4.0)


def foliage_normals(obj):
    """Custom normals of foliage cards point away from the crown center.

    Gives round, soft crown lighting instead of flat-shaded cards.
    """
    me = obj.data
    mats = [s.material.name if s.material else "" for s in obj.material_slots]
    if "Foliage" not in mats:
        return
    fi = mats.index("Foliage")
    pts = [me.vertices[i].co for p in me.polygons if p.material_index == fi for i in p.vertices]
    if not pts:
        return
    center = sum(pts, Vector()) / len(pts)
    corner = [c.vector.copy() for c in me.corner_normals]
    loop_normals = []
    for poly in me.polygons:
        for li in poly.loop_indices:
            if poly.material_index == fi:
                p = me.vertices[me.loops[li].vertex_index].co
                d = p - center
                d.z *= 0.6
                d = d.normalized() if d.length > 1e-6 else Vector((0, 0, 1))
                n = (d * 0.75 + poly.normal * 0.25 + Vector((0, 0, 0.35))).normalized()
            else:
                n = corner[li]
            loop_normals.append(n)
    me.normals_split_custom_set(loop_normals)


def params(table, where):
    return Params(table, where)


def rng(seed):
    return random.Random(int(seed) * 7919 + 17)
