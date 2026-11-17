"""Preview renders of environment objects with the 1.80 m figure and a tank.

    blender -b --factory-startup --python-exit-code 1 --python tools/blender/env_lineup.py -- <out_dir> [<id> ...]

Writes into <out_dir> (a scratch folder, never the repository):
- `<id>.png`: all variants of one object side by side, 3/4 view, with the
  reference figure
- `scale_lineup_front.png` / `scale_lineup_persp.png`: the first variant of
  the main objects in a row with the reference figure and the Panzer IV,
  orthographic front view (true heights) and a perspective view
and prints the measured sizes (door heights, fence heights, tree heights).
Blender preview only: foliage cards get a rough noise alpha; the real look
is the Godot shader (`just gallery`).
"""

import math
import sys
import tempfile
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog, env, render  # noqa: E402
from kit.core import SLOTS, descendants, world_bbox  # noqa: E402

REFERENCE = catalog.REPO_ROOT / "assets-src" / "blender" / "reference" / "scale_reference.blend"
TANK = ("de", "de_pz4_h")
KIND_TINT = {
    ("Bark", 1): (0.78, 0.76, 0.70), ("Bark", 2): (0.42, 0.34, 0.24), ("Bark", 3): (0.45, 0.28, 0.18),
    ("Stone", 1): (0.62, 0.58, 0.50), ("Stone", 2): (0.55, 0.55, 0.52), ("Stone", 3): (0.50, 0.24, 0.17),
    ("Plaster", 1): (0.66, 0.52, 0.36), ("Roof", 1): (0.25, 0.26, 0.29), ("Roof", 2): (0.66, 0.58, 0.34),
    ("Paint", 1): (0.6, 0.08, 0.06), ("Paint", 2): (0.85, 0.85, 0.82), ("Paint", 3): (0.35, 0.2, 0.12),
    ("Wood", 3): (0.07, 0.06, 0.05), ("Wood", 2): (0.18, 0.13, 0.09),
}


def append(path):
    with bpy.data.libraries.load(str(path), link=False) as (src, dst):
        dst.objects = list(src.objects)
    objs = [o for o in dst.objects if o is not None]
    for o in objs:
        bpy.context.scene.collection.objects.link(o)
    for mat in list(bpy.data.materials):
        base, _, suffix = mat.name.partition(".")
        if suffix.isdigit() and base in bpy.data.materials:
            mat.user_remap(bpy.data.materials[base])
            bpy.data.materials.remove(mat)
    return objs


def preview_env_materials():
    """Slot color x kind tint x AO; foliage cards with a noise alpha."""
    for name, rgb in SLOTS.items():
        mat = bpy.data.materials.get(name)
        if mat is None or mat.get("env_preview"):
            continue
        mat["env_preview"] = True
        mat["mas_preview"] = True  # keep kit.render.preview_materials from relinking
        mat.use_backface_culling = False
        nt = mat.node_tree
        nodes, links = nt.nodes, nt.links
        bsdf = nodes.get("Principled BSDF")
        col = nodes.new("ShaderNodeAttribute")
        col.attribute_name = "Color"
        sep = nodes.new("ShaderNodeSeparateColor")
        links.new(col.outputs["Color"], sep.inputs["Color"])
        # kind from alpha: A = 1 - kind / 4
        kind = nodes.new("ShaderNodeMath")
        kind.operation = "MULTIPLY_ADD"
        links.new(col.outputs["Alpha"], kind.inputs[0])
        kind.inputs[1].default_value = -4.0
        kind.inputs[2].default_value = 4.0
        ramp = nodes.new("ShaderNodeValToRGB")
        ramp.color_ramp.interpolation = "CONSTANT"
        el = ramp.color_ramp.elements
        el[0].color = (*rgb, 1)
        el[1].position = 0.999
        el[1].color = (*rgb, 1)
        for k in (1, 2, 3):
            e = el.new((k - 0.5) / 4)
            e.color = (*KIND_TINT.get((name, k), rgb), 1)
        div = nodes.new("ShaderNodeMath")
        div.operation = "DIVIDE"
        div.inputs[1].default_value = 4.0
        links.new(kind.outputs[0], div.inputs[0])
        links.new(div.outputs[0], ramp.inputs["Fac"])
        ao = nodes.new("ShaderNodeMix")
        ao.data_type = "RGBA"
        ao.blend_type = "MULTIPLY"
        ao.inputs["Factor"].default_value = 1.0
        links.new(ramp.outputs["Color"], ao.inputs["A"])
        comb = nodes.new("ShaderNodeCombineColor")
        for c in ("Red", "Green", "Blue"):
            links.new(sep.outputs["Red"], comb.inputs[c])
        links.new(comb.outputs["Color"], ao.inputs["B"])
        links.new(ao.outputs["Result"], bsdf.inputs["Base Color"])
        if name == "Foliage":
            uv = nodes.new("ShaderNodeUVMap")
            sxyz = nodes.new("ShaderNodeSeparateXYZ")
            links.new(uv.outputs["UV"], sxyz.inputs["Vector"])
            fx = nodes.new("ShaderNodeMath")
            fx.operation = "FRACT"
            links.new(sxyz.outputs["X"], fx.inputs[0])
            cxyz = nodes.new("ShaderNodeCombineXYZ")
            links.new(fx.outputs[0], cxyz.inputs["X"])
            links.new(sxyz.outputs["Y"], cxyz.inputs["Y"])
            noise = nodes.new("ShaderNodeTexNoise")
            noise.inputs["Scale"].default_value = 9.0
            links.new(cxyz.outputs["Vector"], noise.inputs["Vector"])
            dist = nodes.new("ShaderNodeVectorMath")
            dist.operation = "DISTANCE"
            dist.inputs[1].default_value = (0.5, 0.5, 0.0)
            links.new(cxyz.outputs["Vector"], dist.inputs[0])
            sub = nodes.new("ShaderNodeMath")
            sub.operation = "SUBTRACT"
            links.new(noise.outputs["Fac"], sub.inputs[0])
            links.new(dist.outputs["Value"], sub.inputs[1])
            gt = nodes.new("ShaderNodeMath")
            gt.operation = "GREATER_THAN"
            gt.inputs[1].default_value = 0.12
            links.new(sub.outputs[0], gt.inputs[0])
            links.new(gt.outputs[0], bsdf.inputs["Alpha"])


def spread(root, x0, gap=1.5):
    """Places the variants of one object side by side along X; returns width."""
    x = x0
    for child in sorted(root.children, key=lambda c: c.name):
        lo, hi = world_bbox([child] + descendants(child))
        if lo.x == math.inf:
            continue
        child.location.x += x - lo.x
        x += (hi.x - lo.x) + gap
        bpy.context.view_layer.update()
    return x - x0


def reference(x, y=0.0):
    ref = bpy.data.objects.new("Reference_180", None)
    bpy.context.scene.collection.objects.link(ref)
    for o in append(REFERENCE):
        if o.parent is None:
            o.parent = ref
    ref.location = (x, y, 0)
    bpy.context.view_layer.update()
    return ref


def object_sheet(uid, out):
    bpy.ops.wm.read_homefile(use_empty=True)
    append(catalog.blend_path("neutral", uid))
    root = bpy.data.objects[uid]
    bpy.context.view_layer.update()
    w = spread(root, 0.0, gap=max(0.5, 0.15 * catalog.mm(catalog.unit("neutral", uid), "length_mm")))
    reference(-1.5)
    preview_env_materials()
    scene = render.setup_scene(resolution=(1600, 800))
    objs = [o for o in bpy.data.objects if o.type == "MESH" and o.name != "PreviewGround"]
    lo, hi = world_bbox(objs)
    center = (lo + hi) / 2
    cam = render._camera(scene)
    cam.data.type = "PERSP"
    cam.data.lens = 40
    size = max(hi.x - lo.x, (hi.z - lo.z) * 2.0, 2.0)
    render._aim(cam, center, -30, 22, size * 1.05)
    render._render(scene, out)
    return w


def scale_lineup(ids, out):
    bpy.ops.wm.read_homefile(use_empty=True)
    picks = []
    for uid in ids:
        append(catalog.blend_path("neutral", uid))
        root = bpy.data.objects[uid]
        first = sorted(root.children, key=lambda c: c.name)
        bp = catalog.blueprint("neutral", uid)
        keep = bp["variant"][0]["name"]
        for c in first:
            if c.name.split(".")[0] != keep:  # appended names may get a .001 suffix
                for o in [c] + descendants(c):
                    bpy.data.objects.remove(o, do_unlink=True)
        picks.append(root)
    tank_blend = catalog.blend_path(*TANK)
    if tank_blend.exists():
        append(tank_blend)
        picks.append(bpy.data.objects[TANK[1]])
    ref = reference(0.0)
    picks.append(ref)
    # row along X, seen from the front (-Y): orthographic side view + perspective
    bpy.context.view_layer.update()
    x = 0.0
    for r in picks:
        lo, hi = world_bbox([r] + descendants(r))
        r.location.x += x - lo.x
        x += (hi.x - lo.x) + 2.0
        bpy.context.view_layer.update()
    preview_env_materials()
    lo, hi = world_bbox([o for r in picks for o in [r] + descendants(r)])
    res = (3000, max(400, int(3000 * (hi.z + 2.0) / ((hi.x - lo.x) * 1.03))))
    scene = render.setup_scene(resolution=res)
    cam = render._camera(scene)
    cam.data.clip_end = 5000.0
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = (hi.x - lo.x) * 1.03
    vis_h = cam.data.ortho_scale * res[1] / res[0]
    out = Path(out)
    render._aim(cam, Vector(((lo.x + hi.x) / 2, 0.0, vis_h / 2 - 0.5)), 180.0, 0.0, 200.0)
    render._render(scene, out.with_name(out.stem + "_front.png"))
    cam.data.type = "PERSP"
    cam.data.lens = 35
    render._aim(cam, Vector(((lo.x + hi.x) / 2, 0.0, 3.0)), 200.0, 14.0, (hi.x - lo.x) * 0.75)
    render._render(scene, out.with_name(out.stem + "_persp.png"))
    return out.with_name(out.stem + "_front.png")


def report(ids):
    print("env_lineup: measured sizes (L x W x H, m) of the first variant; door heights from extras")
    for uid in ids:
        bpy.ops.wm.read_homefile(use_empty=True)
        append(catalog.blend_path("neutral", uid))
        root = bpy.data.objects[uid]
        for c in sorted(root.children, key=lambda c: c.name):
            lo, hi = world_bbox([c] + descendants(c))
            d = hi - lo
            extra = ""
            if "door_height_m" in c:
                extra = f"  door {c['door_height_m']:.2f} m"
            if "deck_height_m" in c:
                extra = f"  deck {c['deck_height_m']:.2f} m, span {c['span_m']:.1f} m"
            print(f"  {uid:24} {c.name:14} {d.y:6.2f} x {d.x:6.2f} x {d.z:6.2f}{extra}")


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = Path(argv[0] if argv else tempfile.mkdtemp(prefix="env_lineup_")).resolve()
    out.mkdir(parents=True, exist_ok=True)
    ids = argv[1:] or [uid for _, uid in catalog.all_blueprints(env=True)]
    for uid in ids:
        object_sheet(uid, out / f"{uid}.png")
        print(f"env_lineup: {out / (uid + '.png')}")
    lineup_ids = [i for i in ("env_house_farm", "env_barn", "env_church", "env_tree_deciduous", "env_tree_conifer",
                              "env_tree_birch", "env_bush", "env_fence_wood", "env_wall_stone", "env_hedge",
                              "fort_sandbag_position", "fort_hedgehog", "fort_barbed_wire", "env_pole_telegraph",
                              "env_hay_bale", "env_house_town", "env_block_desert", "env_block_desert_long",
                              "env_block_town") if i in ids]
    if lineup_ids:
        print(f"env_lineup: {scale_lineup(lineup_ids, out / 'scale_lineup.png')}")
    report(ids)


try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
