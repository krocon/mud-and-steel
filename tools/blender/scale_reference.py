"""Builds the 1.80 m reference figure and scale bars and saves them as a .blend.

Run headless:
    blender -b --factory-startup --python tools/blender/scale_reference.py

Line-up renders of new models append or link the `ScaleReference` collection from
assets-src/blender/reference/scale_reference.blend to compare sizes.

Conventions (docs/ASSETS.md): 1 Blender unit = 1 m, Z up, front faces +Y, origin on
the ground, geometry built in place (identity rotation and scale).

Contents:
- `Figure_180`: a simple standing soldier, exactly 1.80 m tall, origin between the
  feet, facing +Y.
- `ScaleBar_X`: 10 m along +X in alternating 1 m segments, starting 1 m right of the
  figure, with 0.1 m ticks on the first meter.
- `ScaleBar_Z`: 3 m upright pole in 0.5 m segments, 1 m left of the figure.
"""

from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
OUTPUT = REPO_ROOT / "assets-src" / "blender" / "reference" / "scale_reference.blend"

FIGURE_HEIGHT = 1.80


def material(name, rgb):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*rgb, 1.0)
    bsdf = mat.node_tree.nodes.get("Principled BSDF") if mat.node_tree else None
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        bsdf.inputs["Roughness"].default_value = 0.9
    return mat


def add_box(bm, size, center):
    """Adds an axis-aligned box given by size (x, y, z) and center."""
    matrix = Matrix.Translation(center) @ Matrix.Diagonal((*size, 1.0))
    bmesh.ops.create_cube(bm, size=1.0, matrix=matrix)


def mesh_object(name, build, materials, collection, parent=None):
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    build(bm)
    bm.to_mesh(mesh)
    bm.free()
    for mat in materials:
        mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.parent = parent
    return obj


def build_figure(bm):
    """A blocky soldier: 1.80 m from sole to top of the helmet."""
    h = FIGURE_HEIGHT
    head = 0.24  # head incl. helmet
    leg = 0.47 * h  # sole to hip
    torso = h - leg - head
    # Legs and boots.
    for x in (-0.1, 0.1):
        add_box(bm, (0.14, 0.18, leg), (x, 0.0, leg / 2))
        add_box(bm, (0.12, 0.28, 0.08), (x, 0.04, 0.04))
    # Torso and arms.
    add_box(bm, (0.42, 0.24, torso), (0.0, 0.0, leg + torso / 2))
    for x in (-0.27, 0.27):
        add_box(bm, (0.1, 0.12, torso * 0.95), (x, 0.0, leg + torso * 0.5))
    # Neck, head and helmet brim; the head top is exactly at FIGURE_HEIGHT.
    add_box(bm, (0.1, 0.1, 0.04), (0.0, 0.0, leg + torso + 0.02))
    add_box(bm, (0.2, 0.22, head - 0.04), (0.0, 0.0, h - (head - 0.04) / 2))
    add_box(bm, (0.3, 0.32, 0.03), (0.0, 0.0, h - 0.09))
    # Nose marks the front (+Y).
    add_box(bm, (0.04, 0.04, 0.04), (0.0, 0.12, h - 0.14))


def build_bar_x(bm_dark, bm_light):
    for i in range(10):
        target = bm_dark if i % 2 == 0 else bm_light
        add_box(target, (1.0, 0.1, 0.02), (1.5 + i, 0.0, 0.01))
    for i in range(10):
        target = bm_light if i % 2 == 0 else bm_dark
        add_box(target, (0.01, 0.1, 0.03), (1.0 + i * 0.1 + 0.005, 0.0, 0.015))


def build_bar_z(bm_dark, bm_light):
    for i in range(6):
        target = bm_dark if i % 2 == 0 else bm_light
        add_box(target, (0.06, 0.06, 0.5), (-1.0, 0.0, 0.25 + i * 0.5))


def two_tone(name, builder, dark, light, collection):
    mesh = bpy.data.meshes.new(name)
    bm_dark, bm_light = bmesh.new(), bmesh.new()
    builder(bm_dark, bm_light)
    # Merge both into one mesh with two material slots.
    bm = bmesh.new()
    for index, part in enumerate((bm_dark, bm_light)):
        temp = bpy.data.meshes.new("temp")
        part.to_mesh(temp)
        part.free()
        offset = len(bm.faces)
        bm.from_mesh(temp)
        bm.faces.ensure_lookup_table()
        for face in bm.faces[offset:]:
            face.material_index = index
        bpy.data.meshes.remove(temp)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(dark)
    mesh.materials.append(light)
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    return obj


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.unit_settings.scale_length = 1.0

    collection = bpy.data.collections.new("ScaleReference")
    scene.collection.children.link(collection)

    cloth = material("Cloth", (0.35, 0.33, 0.25))
    dark = material("ScaleDark", (0.02, 0.02, 0.02))
    light = material("ScaleLight", (0.9, 0.9, 0.9))

    figure = mesh_object("Figure_180", build_figure, [cloth], collection)
    two_tone("ScaleBar_X", build_bar_x, dark, light, collection)
    two_tone("ScaleBar_Z", build_bar_z, dark, light, collection)

    bpy.context.view_layer.update()
    top = max((figure.matrix_world @ v.co).z for v in figure.data.vertices)
    assert abs(top - FIGURE_HEIGHT) < 1e-5, f"figure is {top} m tall"

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT))
    print(f"Saved {OUTPUT} (figure {top:.3f} m)")


main()
