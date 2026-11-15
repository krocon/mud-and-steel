"""Preview renders with Eevee: turntable sheet per model and the line-up.

Only for checking models; output goes to a scratch or ignored folder, never
into the repository. Materials are switched to a preview look in the running
session only (slot color times baked AO), the saved .blend is not touched.
"""

import math
import tempfile
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector

from .core import SLOTS, descendants, world_bbox

PREVIEW_COLORS = dict(SLOTS, Paint=(0.42, 0.40, 0.26))


def preview_materials():
    """Base color = slot color x AO (vertex color R); track faces darker."""
    for name, rgb in PREVIEW_COLORS.items():
        mat = bpy.data.materials.get(name)
        if mat is None or mat.get("mas_preview"):
            continue
        mat["mas_preview"] = True
        nt = mat.node_tree
        bsdf = nt.nodes.get("Principled BSDF")
        attr = nt.nodes.new("ShaderNodeAttribute")
        attr.attribute_name = "Color"
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        mul = nt.nodes.new("ShaderNodeMix")
        mul.data_type = "RGBA"
        mul.blend_type = "MULTIPLY"
        mul.inputs["Factor"].default_value = 1.0
        mul.inputs["A"].default_value = (*rgb, 1.0)
        nt.links.new(attr.outputs["Color"], sep.inputs["Color"])
        # AO as gray: build an RGB from the red channel
        comb = nt.nodes.new("ShaderNodeCombineColor")
        for k in ("Red", "Green", "Blue"):
            nt.links.new(sep.outputs["Red"], comb.inputs[k])
        nt.links.new(comb.outputs["Color"], mul.inputs["B"])
        nt.links.new(mul.outputs["Result"], bsdf.inputs["Base Color"])


def setup_scene(resolution=(900, 700)):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = resolution
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    scene.view_settings.view_transform = "Standard"
    world = scene.world or bpy.data.worlds.new("World")
    scene.world = world
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    bg.inputs["Color"].default_value = (0.55, 0.6, 0.65, 1.0)
    bg.inputs["Strength"].default_value = 0.6
    if "PreviewSun" not in bpy.data.objects:
        sun = bpy.data.lights.new("PreviewSun", "SUN")
        sun.energy = 3.5
        sun.angle = math.radians(3)
        so = bpy.data.objects.new("PreviewSun", sun)
        scene.collection.objects.link(so)
        so.rotation_euler = (math.radians(50), 0, math.radians(-35))
    if "PreviewGround" not in bpy.data.objects:
        me = bpy.data.meshes.new("PreviewGround")
        g = 200
        me.from_pydata([(-g, -g, 0), (g, -g, 0), (g, g, 0), (-g, g, 0)], [], [(0, 1, 2, 3)])
        mat = bpy.data.materials.new("PreviewGroundMat")
        mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.36, 0.34, 0.3, 1)
        me.materials.append(mat)
        go = bpy.data.objects.new("PreviewGround", me)
        scene.collection.objects.link(go)
    preview_materials()
    return scene


def _camera(scene, name="PreviewCam"):
    cam = bpy.data.objects.get(name)
    if cam is None:
        cam = bpy.data.objects.new(name, bpy.data.cameras.new(name))
        scene.collection.objects.link(cam)
    scene.camera = cam
    return cam


def _aim(cam, target, azimuth_deg, elevation_deg, distance):
    az, el = math.radians(azimuth_deg), math.radians(elevation_deg)
    d = Vector((math.sin(az) * math.cos(el), math.cos(az) * math.cos(el), math.sin(el)))
    cam.location = target + d * distance
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()


def _render(scene, path):
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def _combine(paths, cols, out):
    imgs = [bpy.data.images.load(str(p)) for p in paths]
    w, h = imgs[0].size
    rows = math.ceil(len(imgs) / cols)
    sheet = np.ones((rows * h, cols * w, 4), dtype=np.float32)
    for i, img in enumerate(imgs):
        px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)
        r, c = divmod(i, cols)
        sheet[(rows - 1 - r) * h:(rows - r) * h, c * w:(c + 1) * w] = px
    res = bpy.data.images.new("sheet", cols * w, rows * h, alpha=True)
    res.pixels = sheet.ravel()
    res.filepath_raw = str(out)
    res.file_format = "PNG"
    res.save()
    for img in imgs:
        bpy.data.images.remove(img)


def turntable(root, out):
    """Four views (front 3/4, side, rear 3/4, top) of one model in one PNG."""
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    scene = setup_scene()
    objs = [root] + descendants(root)
    lo, hi = world_bbox(objs)
    center = (lo + hi) / 2
    size = (hi - lo).length
    cam = _camera(scene)
    cam.data.lens = 50
    views = [(-40, 25), (-90, 4), (140, 28), (-20, 70)]
    tmp = Path(tempfile.mkdtemp(prefix="mas_turntable_"))
    paths = []
    for i, (az, el) in enumerate(views):
        _aim(cam, center, az, el, size * 1.55)
        p = tmp / f"view_{i}.png"
        _render(scene, p)
        paths.append(p)
    _combine(paths, 2, out)
    return out


def lineup(roots, out, reference=None, gap=1.5):
    """All models in a row with the 1.80 m reference figure.

    The models stand one behind the other along Y (gap in meters) and are
    seen from the left side: an orthographic view (lengths and heights are
    true to scale) and a perspective 3/4 view, stacked into one PNG.
    """
    out = Path(out)
    res = (2400, 640)
    scene = setup_scene(resolution=res)
    y = 0.0
    items = list(roots) + ([reference] if reference is not None else [])
    for root in items:
        bpy.context.view_layer.update()
        lo, hi = world_bbox([root] + descendants(root))
        root.location.y += y - hi.y
        y -= (hi.y - lo.y) + gap
    bpy.context.view_layer.update()
    allobjs = []
    for r in items:
        allobjs += [r] + descendants(r)
    lo, hi = world_bbox(allobjs)
    center = (lo + hi) / 2
    cam = _camera(scene)
    tmp = Path(tempfile.mkdtemp(prefix="mas_lineup_"))
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = (hi.y - lo.y) * 1.04
    visible_h = cam.data.ortho_scale * res[1] / res[0]
    # ground line close to the bottom edge of the image
    _aim(cam, Vector((0.0, center.y, visible_h / 2 - 0.25)), -90.0, 0.0, 60.0)
    p1 = tmp / "side.png"
    _render(scene, p1)
    cam.data.type = "PERSP"
    cam.data.lens = 35
    _aim(cam, Vector((0.0, center.y, 1.0)), -62, 16, (hi.y - lo.y) * 0.95)
    p2 = tmp / "persp.png"
    _render(scene, p2)
    _combine([p1, p2], 1, out)
    return out
