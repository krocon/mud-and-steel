"""Compares a built vehicle model with its references (prompt 063, `just ref-compare <id>`).

    blender -b --factory-startup --python tools/blender/ref_compare.py -- <id> [--out DIR] [--ppm N]

Renders the model of `assets-src/blender/<faction>/<id>.blend` orthographically
from the side (front to the left), the front and the top, all at the same fixed
scale (`--ppm` pixels per meter, default 80), with dimension lines: overall
length, hull length, barrel overhang, height, ground clearance and track ground
contact (side), width, height and track width (front), length and width (top).
Below come a table "model / data.json / catalog" with the deviations in percent
and the drawings of the reference folder (`docs/.vorlagen/fahrzeuge/<folder>`,
mapped by `data/vehicles/index.json`; a folder without a drawing shows the
drawings of its `chassis` relative). Output: `build/renders/compare/<id>.png`
and the same table as `<id>.md` (both outside the repository: the drawings
must never be committed, docs/SOURCES.md).

data.json values are read through `data/vehicles/<faction>.json` (the same
values in one shape). A vehicle without a model yet (catalog `model` empty)
gets the table and the drawings only. Model values:

- length / width / height: like `validate.py` (the blueprint's `[validate]
  exclude_*` objects are left out); its tolerance is 2 % against the catalog.
- hull length: the Hull mesh over the tracks (fenders, superstructure sides;
  exhausts, tow hooks and stowage between the tracks at the hull ends are
  left out).
- barrel overhang: front of the gun beyond the hull length.
- track width / ground contact: `Track_L` (x extent / y extent of the part
  that lies within 5 cm of the ground).
- ground clearance: lowest point of the `Hull` belly on the center line over
  the middle half of the hull (rays from below).
"""

import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog  # noqa: E402

ROOT = catalog.REPO_ROOT
REFS = ROOT / "docs" / ".vorlagen" / "fahrzeuge"
# photos and model pictures outside fahrzeuge/ (prompt 063, section 2)
EXTRA_REFS = {
    "de_jgdpz38t": ROOT / "docs" / ".vorlagen" / "hetzer",
    "de_sdkfz222": ROOT / "docs" / ".vorlagen" / "Leichter-Panzerspähwagen-(2-cm)-Sd.Kfz.-222",
}
IMAGE_EXT = (".jpg", ".jpeg", ".png")
TOLERANCE = 0.02  # validate.py
MAX_EXTRA = 6

# view frames in meters (width, height)
SIDE = (12.5, 4.8)
FRONT = (5.5, 4.8)
TOP = (12.5, 5.5)
GROUND_Z = -0.6  # lower edge of the side and front frames


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if not argv or argv[0].startswith("--"):
        sys.exit("usage: ref_compare.py -- <id> [--out DIR] [--ppm N]")
    out = Path(argv[argv.index("--out") + 1]) if "--out" in argv else ROOT / "build" / "renders" / "compare"
    ppm = int(argv[argv.index("--ppm") + 1]) if "--ppm" in argv else 80
    return argv[0], out, ppm


# ----------------------------------------------------------------------------------
# reference data
# ----------------------------------------------------------------------------------
def reference(uid):
    """(folder, row, database entry) of an id from data/vehicles."""
    index = json.loads((ROOT / "data" / "vehicles" / "index.json").read_text(encoding="utf-8"))
    by_id = {r["id"]: (folder, r) for folder, r in index.items()}
    if uid not in by_id:
        return None, None, None, by_id
    folder, row = by_id[uid]
    entry = None
    db = ROOT / "data" / "vehicles" / f"{uid.split('_')[0]}.json"
    if db.exists():
        entry = next((e for e in json.loads(db.read_text(encoding="utf-8")) if e["id"] == uid), None)
    return folder, row, entry, by_id


def drawings(uid, folder, row, by_id):
    """[(path, label)] of the drawings to show."""
    out = []
    if folder:
        out += [(p, f"{folder}/{p.name}") for p in sorted((REFS / folder).glob("*")) if p.suffix.lower() in IMAGE_EXT]
        if not out and row and row.get("chassis") in by_id:
            rel_folder, _ = by_id[row["chassis"]]
            out += [(p, f"{rel_folder}/{p.name} (relative {row['chassis']}: no drawing of its own)")
                    for p in sorted((REFS / rel_folder).glob("*")) if p.suffix.lower() in IMAGE_EXT]
    extra = EXTRA_REFS.get(uid)
    if extra and extra.is_dir():
        pics = [p for p in sorted(extra.iterdir()) if p.suffix.lower() in IMAGE_EXT][:MAX_EXTRA]
        out += [(p, f"{extra.name}/{p.name}") for p in pics]
    return out


def db_values(entry):
    if not entry:
        return {}
    g, m = entry["general"], entry["mobility"]
    hull = g.get("length_hull_mm")
    if hull is None and "length_mm" in g and "barrel_overhang_mm" in g:
        hull = g["length_mm"] - g["barrel_overhang_mm"]
    return {
        "length_mm": g.get("length_mm"),
        "length_hull_mm": hull,
        "barrel_overhang_mm": g.get("barrel_overhang_mm"),
        "width_mm": g.get("width_skirts_mm", g.get("width_mm")),
        "height_mm": g.get("height_mm"),
        "track_width_mm": m.get("track_width_mm"),
        "track_ground_contact_mm": m.get("track_ground_contact_mm"),
        "ground_clearance_mm": m.get("ground_clearance_mm"),
    }


def catalog_values(u):
    hull = u.get("length_hull_mm")
    return {
        "length_mm": u.get("length_mm"),
        "length_hull_mm": hull,
        "barrel_overhang_mm": u["length_mm"] - hull if hull and u.get("length_mm") else None,
        "width_mm": u.get("width_mm"),
        "height_mm": u.get("height_mm"),
    }


# ----------------------------------------------------------------------------------
# model measurements
# ----------------------------------------------------------------------------------
def descendants(root):
    out, stack = [], list(root.children)
    while stack:
        o = stack.pop()
        out.append(o)
        stack.extend(o.children)
    return out


def under(obj, names):
    """True if the object or an ancestor has one of the names (fnmatch patterns)."""
    import fnmatch

    while obj is not None:
        if any(fnmatch.fnmatchcase(obj.name, n) for n in names):
            return True
        obj = obj.parent
    return False


def verts(objs):
    for o in objs:
        if o.type != "MESH":
            continue
        mw = o.matrix_world
        for v in o.data.vertices:
            yield mw @ v.co


def bounds(objs):
    pts = list(verts(objs))
    if not pts:
        return None, None
    lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return lo, hi


def measure(root, bp):
    meshes = [o for o in descendants(root) if o.type == "MESH"]
    val = bp.get("validate", {})
    m = {}
    for axis, key, idx in (("length", "length_mm", 1), ("width", "width_mm", 0), ("height", "height_mm", 2)):
        lo, hi = bounds([o for o in meshes if not under(o, val.get(f"exclude_{axis}", []))])
        m[key] = round((hi[idx] - lo[idx]) * 1000)
    # hull length: the Hull mesh over the tracks (fenders, superstructure sides); exhausts, tow
    # hooks and stowage at the hull ends sit between the tracks and are left out
    track = bpy.data.objects.get("Track_L")
    hull = bpy.data.objects.get("Hull")
    inner = min(abs(p.x) for p in verts([track])) if track else 0.0
    side_pts = [p for p in verts([hull]) if abs(p.x) >= inner + 0.02] if hull else []
    if not side_pts:
        side_pts = list(verts([hull] if hull else meshes))
    lo = Vector((0, min(p.y for p in side_pts), 0))
    hi = Vector((0, max(p.y for p in side_pts), 0))
    m["length_hull_mm"] = round((hi.y - lo.y) * 1000)
    geo = {"hull_rear": lo.y, "hull_front": hi.y}
    guns = [o for o in meshes if under(o, ["Gun"])]
    all_lo, all_hi = bounds(meshes)
    geo.update(rear=all_lo.y, front=all_hi.y, top=all_hi.z, half_width=max(-all_lo.x, all_hi.x))
    if guns:
        _, ghi = bounds(guns)
        geo["gun_tip"] = ghi.y
        m["barrel_overhang_mm"] = max(0, round((ghi.y - hi.y) * 1000))
    if track:
        tlo, thi = bounds([track])
        m["track_width_mm"] = round((thi.x - tlo.x) * 1000)
        ground = [p for p in verts([track]) if p.z < tlo.z + 0.05]
        geo["contact"] = (min(p.y for p in ground), max(p.y for p in ground))
        m["track_ground_contact_mm"] = round((geo["contact"][1] - geo["contact"][0]) * 1000)
        geo["track"] = (thi.x, tlo.x) if thi.x < 0 else (tlo.x, thi.x)
        if hull:
            # rays upward on the center line over the middle half of the hull: the first hit is the belly
            from mathutils.bvhtree import BVHTree

            tree = BVHTree.FromObject(hull, bpy.context.evaluated_depsgraph_get())
            inv = hull.matrix_world.inverted()
            hits = []
            for k in range(21):
                y = lo.y + (hi.y - lo.y) * (0.25 + 0.5 * k / 20)
                loc, *_ = tree.ray_cast(inv @ Vector((0.0, y, -1.0)), (inv.to_3x3() @ Vector((0, 0, 1))).normalized())
                if loc is not None:
                    hits.append((hull.matrix_world @ loc).z)
            if hits:
                m["ground_clearance_mm"] = round(min(hits) * 1000)
    wheels = [o for o in meshes if o.name.startswith("Wheel_L_")]
    rollers = [o for o in meshes if o.name.startswith("Roller_L_")]
    sprocket = bpy.data.objects.get("Sprocket_L")
    info = {
        "road wheels per side": len(wheels),
        "return rollers per side": len(rollers),
        "sprocket": ("front" if sprocket.matrix_world.translation.y > 0 else "rear") if sprocket else "-",
    }
    if wheels:
        wlo, whi = bounds([wheels[0]])
        info["road wheel diameter"] = f"{whi.z - wlo.z:.2f} m"
    return m, geo, info


# ----------------------------------------------------------------------------------
# scene: annotations and renders
# ----------------------------------------------------------------------------------
def basis(right, up):
    right, up = Vector(right), Vector(up)
    back = right.cross(up)
    return Matrix((right, up, back)).transposed().to_4x4()


VIEWS = {
    # name: (right, up, frame)
    "side": ((0, -1, 0), (0, 0, 1), SIDE),
    "front": ((-1, 0, 0), (0, 0, 1), FRONT),
    "top": ((0, -1, 0), (1, 0, 0), TOP),
    "table": ((1, 0, 0), (0, 1, 0), None),
}

RED = (0.8, 0.1, 0.1, 1.0)
BLACK = (0.0, 0.0, 0.0, 1.0)
BLUE = (0.1, 0.25, 0.8, 1.0)


class Sheet:
    """Annotation objects per view (hidden in the other views' renders)."""

    def __init__(self):
        self.objects = {name: [] for name in VIEWS}
        self.coll = bpy.data.collections.new("RefCompare")
        bpy.context.scene.collection.children.link(self.coll)

    def _link(self, view, ob, color):
        ob.color = color
        self.coll.objects.link(ob)
        self.objects[view].append(ob)
        return ob

    def text(self, view, pos, body, size=0.2, align="CENTER", color=BLACK):
        cu = bpy.data.curves.new("txt", "FONT")
        cu.body = body
        cu.size = size
        cu.align_x = align
        cu.align_y = "CENTER"
        ob = bpy.data.objects.new("txt", cu)
        right, up, _ = VIEWS[view]
        ob.matrix_world = Matrix.Translation(Vector(pos)) @ basis(right, up)
        return self._link(view, ob, color)

    def bar(self, view, p0, p1, color=RED, t=0.025, ticks=True):
        p0, p1 = Vector(p0), Vector(p1)
        d = p1 - p0
        if d.length < 1e-4:
            return
        right, up, _ = VIEWS[view]
        n = Vector(right).cross(Vector(up))
        u = d.normalized().cross(n).normalized()
        pts, faces = [], []
        for a, b in ((p0, p1),) + (((p0 - u * 0.1, p0 + u * 0.1), (p1 - u * 0.1, p1 + u * 0.1)) if ticks else ()):
            dd = (b - a).normalized()
            w = dd.cross(n).normalized() * t / 2
            k = len(pts)
            pts += [a - w - n * t / 2, a + w - n * t / 2, b + w - n * t / 2, b - w - n * t / 2,
                    a - w + n * t / 2, a + w + n * t / 2, b + w + n * t / 2, b - w + n * t / 2]
            faces += [tuple(k + i for i in f) for f in ((0, 1, 2, 3), (4, 7, 6, 5), (0, 4, 5, 1), (1, 5, 6, 2),
                                                   (2, 6, 7, 3), (3, 7, 4, 0))]
        me = bpy.data.meshes.new("dim")
        me.from_pydata([tuple(p) for p in pts], [], faces)
        self._link(view, bpy.data.objects.new("dim", me), color)

    def dim(self, view, p0, p1, label, offset, color=RED):
        """Dimension line p0-p1 with its label `offset` (vector) from the middle."""
        self.bar(view, p0, p1, color)
        mid = (Vector(p0) + Vector(p1)) / 2 + Vector(offset)
        self.text(view, mid, label, color=color)


def annotate(sheet, m, geo, u):
    hw = geo["half_width"]
    H = m["height_mm"] / 1000  # without the excluded parts (antennas)
    yr, yf = geo["rear"], geo["front"]
    hr, hf = geo["hull_rear"], geo["hull_front"]
    xs = -hw - 0.4  # side annotations between camera (-X) and model
    sheet.dim("side", (xs, yr, H + 0.6), (xs, yf, H + 0.6), f"length {m['length_mm'] / 1000:.2f} m", (0, 0, 0.18))
    sheet.dim("side", (xs, hr, H + 0.3), (xs, hf, H + 0.3), f"hull {m['length_hull_mm'] / 1000:.2f} m", (0, 0, -0.18))
    if "gun_tip" in geo and geo["gun_tip"] > hf + 0.02:
        sheet.dim("side", (xs, hf, H + 0.3), (xs, geo["gun_tip"], H + 0.3),
                  f"gun +{m['barrel_overhang_mm'] / 1000:.2f}", (0, 0, 0.18), color=BLUE)
    sheet.dim("side", (xs, yr - 0.35, 0), (xs, yr - 0.35, H), "", (0, 0, 0))
    sheet.text("side", (xs, yr - 0.45, H / 2), f"{m['height_mm'] / 1000:.2f} m", align="LEFT", color=RED)
    if "contact" in geo:
        c0, c1 = geo["contact"]
        sheet.dim("side", (xs, c0, -0.15), (xs, c1, -0.15), f"track contact {m['track_ground_contact_mm'] / 1000:.2f} m",
                  (0, 0, -0.22))
    if "ground_clearance_mm" in m:
        gc = m["ground_clearance_mm"] / 1000
        sheet.bar("side", (xs, 0.0, 0.0), (xs, 0.0, gc), color=BLUE)
        sheet.text("side", (xs, -0.1, gc + 0.15), f"clearance {gc:.2f}", align="LEFT", color=BLUE)
    yfront = yf + 0.4
    sheet.dim("front", (-hw, yfront, H + 0.3), (hw, yfront, H + 0.3), f"width {m['width_mm'] / 1000:.2f} m",
              (0, 0, 0.18))
    sheet.dim("front", (-hw - 0.3, yfront, 0), (-hw - 0.3, yfront, H), "", (0, 0, 0))
    sheet.text("front", (-hw - 0.4, yfront, H / 2), f"{m['height_mm'] / 1000:.2f}", align="LEFT", color=RED)
    if "track" in geo:
        t0, t1 = geo["track"]
        sheet.dim("front", (t0, yfront, -0.15), (t1, yfront, -0.15), f"track {m['track_width_mm'] / 1000:.2f}",
                  (0, 0, -0.22))
    zt = H + 0.4
    sheet.dim("top", (hw + 0.35, yr, zt), (hw + 0.35, yf, zt), f"length {m['length_mm'] / 1000:.2f} m", (0.2, 0, 0))
    sheet.dim("top", (-hw, yr - 0.35, zt), (hw, yr - 0.35, zt), "", (0, 0, 0))
    sheet.text("top", (0, yr - 0.45, zt), f"{m['width_mm'] / 1000:.2f} m", align="LEFT", color=RED)


def camera(view, center, frame, ppm):
    right, up, _ = VIEWS[view]
    cam = bpy.data.cameras.get("RefCam") or bpy.data.cameras.new("RefCam")
    cam.type = "ORTHO"
    cam.clip_start, cam.clip_end = 0.1, 500
    ob = bpy.data.objects.get("RefCam") or bpy.data.objects.new("RefCam", cam)
    if ob.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(ob)
    back = Vector(right).cross(Vector(up))
    ob.matrix_world = Matrix.Translation(Vector(center) + back * 100) @ basis(right, up)
    w, h = frame
    cam.ortho_scale = max(w, h)
    cam.sensor_fit = "AUTO"
    scene = bpy.context.scene
    scene.camera = ob
    scene.render.resolution_x = round(w * ppm)
    scene.render.resolution_y = round(h * ppm)


def setup_render(model_objs):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    scene.render.image_settings.file_format = "PNG"
    scene.view_settings.view_transform = "Standard"
    world = scene.world or bpy.data.worlds.new("World")
    scene.world = world
    world.color = (1.0, 1.0, 1.0)
    sh = scene.display.shading
    sh.light = "FLAT"
    sh.color_type = "OBJECT"
    sh.show_object_outline = True
    sh.object_outline_color = (0.0, 0.0, 0.0)
    sh.show_cavity = True
    sh.cavity_type = "WORLD"
    for o in model_objs:
        o.color = (0.82, 0.82, 0.78, 1.0)


def render(view, sheet, path):
    for name, objs in sheet.objects.items():
        for o in objs:
            o.hide_render = name != view
    bpy.context.scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


# ----------------------------------------------------------------------------------
# table
# ----------------------------------------------------------------------------------
ROWS = [
    ("length_mm", "length (overall)"),
    ("length_hull_mm", "hull length"),
    ("barrel_overhang_mm", "barrel overhang"),
    ("width_mm", "width"),
    ("height_mm", "height"),
    ("track_width_mm", "track width"),
    ("track_ground_contact_mm", "track ground contact"),
    ("ground_clearance_mm", "ground clearance"),
]


def pct(a, b):
    if a is None or b in (None, 0):
        return ""
    return f"{(a - b) * 100 / b:+.1f} %"


def table_rows(model, db, cat):
    rows = [["value (mm)", "model", "data.json", "catalog", "model vs catalog", "model vs data.json"]]
    for key, label in ROWS:
        mv, dv, cv = model.get(key), db.get(key), cat.get(key)
        flag = ""
        if mv is not None and cv and key in ("length_mm", "width_mm", "height_mm") and abs(mv - cv) > TOLERANCE * cv:
            flag = "  (beyond 2 %)"
        rows.append([label, "" if mv is None else str(mv), "" if dv is None else str(dv),
                     "" if cv is None else str(cv), pct(mv, cv) + flag, pct(mv, dv)])
    return rows


def markdown(uid, u, rows, info, draws, has_model):
    lines = [f"# Reference comparison {uid}", "", f"{u.get('name_en', uid)} ({u.get('designation', '')})", ""]
    if not has_model:
        lines += ["No model yet: only data.json against the catalog.", ""]
    lines.append("| " + " | ".join(rows[0]) + " |")
    lines.append("|" + "---|" * len(rows[0]))
    for r in rows[1:]:
        lines.append("| " + " | ".join(r) + " |")
    if info:
        lines += [""] + [f"- {k}: {v}" for k, v in info.items()]
    lines += ["", "Drawings:"] + ([f"{i}. {label}" for i, (_, label) in enumerate(draws, 1)] or ["none"])
    return "\n".join(lines) + "\n"


def table_scene(sheet, uid, u, rows, info, draws, width_m):
    """Text objects of the table in their own plane; returns the frame (w, h)."""
    origin = Vector((1000.0, 1000.0, -50.0))
    cols = [0.0, 4.2, 6.2, 8.2, 10.2, 13.6]
    lines = [f"{uid}: {u.get('name_en', '')}  -  model / data.json (onwar.com) / catalog  -  "
             f"views at the same scale, front to the left"]
    y = 0.0
    sheet.text("table", origin + Vector((0.2, y, 0)), lines[0], size=0.24, align="LEFT")
    y -= 0.45
    for i, r in enumerate(rows):
        for c, cell in zip(cols, r):
            if cell:
                bad = "beyond" in cell
                sheet.text("table", origin + Vector((0.2 + c, y, 0)), cell, size=0.2, align="LEFT",
                           color=RED if bad else BLACK)
        y -= 0.32 if i else 0.4
    if info:
        y -= 0.1
        text = ";  ".join(f"{k}: {v}" for k, v in info.items())
        sheet.text("table", origin + Vector((0.2, y, 0)), text, size=0.2, align="LEFT")
        y -= 0.32
    y -= 0.1
    sheet.text("table", origin + Vector((0.2, y, 0)), "Drawings below (top to bottom):", size=0.2, align="LEFT")
    y -= 0.3
    for i, (_, label) in enumerate(draws or [(None, "none")], 1):
        sheet.text("table", origin + Vector((0.4, y, 0)), f"{i}. {label}" if draws else label, size=0.18, align="LEFT")
        y -= 0.27
    h = -y + 0.3
    center = origin + Vector((width_m / 2, -h / 2 + 0.25, 0))
    return center, (width_m, h)


# ----------------------------------------------------------------------------------
# compose
# ----------------------------------------------------------------------------------
def compose(parts, draws, width, out):
    """parts: [(png path, x, y)] at their pixel size; draws below, scaled to width."""
    import OpenImageIO as oiio
    from OpenImageIO import ROI, ImageBuf, ImageBufAlgo, ImageSpec

    bufs = []
    height = 0
    for path, x, y in parts:
        b = ImageBufAlgo.channels(ImageBuf(str(path)), (0, 1, 2))
        bufs.append((b, x, y))
        height = max(height, y + b.spec().height)
    pics = []
    for path, _ in draws:
        src = ImageBuf(str(path))
        if src.has_error or src.spec().width == 0:
            print(f"ref-compare: cannot read {path.name}: {src.geterror()}")
            continue
        src = ImageBufAlgo.channels(src, (0, 1, 2)) if src.spec().nchannels >= 3 else \
            ImageBufAlgo.channels(src, (0, 0, 0))
        w, h = src.spec().width, src.spec().height
        if w > width:
            nh = round(h * width / w)
            src = ImageBufAlgo.resize(src, roi=ROI(0, width, 0, nh, 0, 1, 0, 3))
            h = nh
        pics.append((src, height + 10))
        height += h + 10
    canvas = ImageBuf(ImageSpec(width, height + 10, 3, oiio.UINT8))
    ImageBufAlgo.fill(canvas, (1.0, 1.0, 1.0))
    for b, x, y in bufs:
        ImageBufAlgo.paste(canvas, x, y, 0, 0, b)
    for src, y in pics:
        ImageBufAlgo.paste(canvas, 0, y, 0, 0, src)
        ImageBufAlgo.render_line(canvas, 0, y - 5, width - 1, y - 5, (0.6, 0.6, 0.6))
    if not canvas.write(str(out)):
        sys.exit(f"ref-compare: cannot write {out}: {canvas.geterror()}")


def main():
    uid, out_dir, ppm = args()
    faction = uid.split("_")[0]
    u = catalog.unit(faction, uid)
    folder, row, entry, by_id = reference(uid)
    if folder is None:
        print(f"ref-compare: {uid} is not in data/vehicles/index.json (no reference folder); comparing with the "
              "catalog only")
    draws = drawings(uid, folder, row, by_id)
    blend = catalog.blend_path(faction, uid)
    has_model = blend.exists()
    out_dir.mkdir(parents=True, exist_ok=True)
    tmp = out_dir / f".{uid}"
    tmp.mkdir(exist_ok=True)
    model, info = {}, {}
    if has_model:
        bpy.ops.wm.open_mainfile(filepath=str(blend))
        root = bpy.data.objects.get(uid)
        if root is None:
            sys.exit(f"ref-compare: {blend} has no root object {uid}")
        bp = catalog.blueprint(faction, uid)
        model, geo, info = measure(root, bp)
    else:
        print(f"ref-compare: no model yet ({blend.relative_to(ROOT)} missing): table and drawings only")
        bpy.ops.wm.read_homefile(use_empty=True)
    rows = table_rows(model, db_values(entry), catalog_values(u))
    md = markdown(uid, u, rows, info, draws, has_model)
    (out_dir / f"{uid}.md").write_text(md, encoding="utf-8")
    print(md)

    sheet = Sheet()
    model_objs = descendants(bpy.data.objects[uid]) if has_model else []
    setup_render(model_objs)
    width_px = round((SIDE[0] + FRONT[0]) * ppm)
    parts = []
    y = 0
    if has_model:
        annotate(sheet, model, geo, u)
        mid_y = (geo["rear"] + geo["front"]) / 2
        views = [
            ("side", (0.0, mid_y, GROUND_Z + SIDE[1] / 2), SIDE, (0, 0)),
            ("front", (0.0, 0.0, GROUND_Z + FRONT[1] / 2), FRONT, (round(SIDE[0] * ppm), 0)),
            ("top", (0.0, mid_y, 0.0), TOP, (0, round(SIDE[1] * ppm))),
        ]
        for name, center, frame, (px, py) in views:
            camera(name, center, frame, ppm)
            path = tmp / f"{name}.png"
            render(name, sheet, path)
            parts.append((path, px, py))
        y = round((SIDE[1] + TOP[1]) * ppm)
    center, frame = table_scene(sheet, uid, u, rows, info, draws, width_px / ppm)
    camera("table", center, frame, ppm)
    path = tmp / "table.png"
    render("table", sheet, path)
    parts.append((path, 0, y))
    out = out_dir / f"{uid}.png"
    compose(parts, draws, width_px, out)
    print(f"ref-compare: wrote {out.relative_to(ROOT) if out.is_relative_to(ROOT) else out} and {uid}.md")


main()
