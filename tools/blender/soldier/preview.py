"""Preview renders for soldiers: animation contact sheets and line-ups (headless).

Renders go to a scratch directory, never into the repository.
"""

import math
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector


def setup_scene(resolution=(480, 480), engine="BLENDER_WORKBENCH"):
    scene = bpy.context.scene
    scene.render.engine = engine
    scene.render.resolution_x, scene.render.resolution_y = resolution
    scene.render.film_transparent = False
    if engine == "BLENDER_WORKBENCH":
        shading = scene.display.shading
        shading.light = "STUDIO"
        shading.color_type = "MATERIAL"
        shading.show_shadows = True
        shading.show_cavity = True
    world = scene.world or bpy.data.worlds.new("World")
    scene.world = world
    world.color = (0.55, 0.58, 0.6)
    cam_data = bpy.data.cameras.get("PreviewCam") or bpy.data.cameras.new("PreviewCam")
    cam = bpy.data.objects.get("PreviewCam") or bpy.data.objects.new("PreviewCam", cam_data)
    if cam.name not in scene.collection.objects:
        scene.collection.objects.link(cam)
    scene.camera = cam
    return cam


def ground(size=40.0, color=(0.32, 0.30, 0.25)):
    mesh = bpy.data.meshes.new("PreviewGround")
    h = size / 2
    mesh.from_pydata([(-h, -h, 0), (h, -h, 0), (h, h, 0), (-h, h, 0)], [], [(0, 1, 2, 3)])
    mat = bpy.data.materials.new("PreviewGround")
    mat.diffuse_color = (*color, 1)
    mat.use_nodes = True
    mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (*color, 1)
    mat.node_tree.nodes["Principled BSDF"].inputs["Roughness"].default_value = 1.0
    mesh.materials.append(mat)
    obj = bpy.data.objects.new("PreviewGround", mesh)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def aim_camera(cam, target, distance, yaw_deg, pitch_deg, ortho=None, lens=50):
    yaw, pitch = math.radians(yaw_deg), math.radians(pitch_deg)
    offset = Vector((math.sin(yaw) * math.cos(pitch), -math.cos(yaw) * math.cos(pitch),
                     math.sin(pitch))) * distance
    cam.location = Vector(target) + offset
    direction = Vector(target) - cam.location
    cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    if ortho:
        cam.data.type = "ORTHO"
        cam.data.ortho_scale = ortho
    else:
        cam.data.type = "PERSP"
        cam.data.lens = lens


def render(path):
    scene = bpy.context.scene
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    return Path(path)


def play(arm, clip, t, fps):
    action = bpy.data.actions[clip]
    arm.animation_data.action = action
    if hasattr(arm.animation_data, "action_slot") and action.slots:
        arm.animation_data.action_slot = action.slots[0]
    bpy.context.scene.frame_set(int(round(t * fps)))


def contact_sheet(paths, cols, out):
    """Tiles equally sized PNGs into one image (pure Blender + numpy)."""
    imgs = [bpy.data.images.load(str(p)) for p in paths]
    w, h = imgs[0].size
    rows = (len(imgs) + cols - 1) // cols
    sheet = np.ones((rows * h, cols * w, 4), dtype=np.float32)
    for i, img in enumerate(imgs):
        px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)
        r, c = divmod(i, cols)
        # Image rows start at the bottom.
        y0 = (rows - 1 - r) * h
        sheet[y0:y0 + h, c * w:(c + 1) * w] = px
    out_img = bpy.data.images.new("sheet", cols * w, rows * h, alpha=True)
    out_img.pixels = sheet.ravel()
    out_img.filepath_raw = str(out)
    out_img.file_format = "PNG"
    out_img.save()
    for img in imgs:
        bpy.data.images.remove(img)
    bpy.data.images.remove(out_img)
    return Path(out)
