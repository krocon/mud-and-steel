"""Builds the placeholder tank from primitives and saves it as a .blend file.

Run headless:
    blender -b --factory-startup --python tools/create_placeholder_tank.py

Conventions (see docs/ARCHITECTURE.md, "Asset pipeline"):
- 1 Blender unit = 1 meter, Z up, the tank faces +Y (becomes -Z in Godot).
- The origin is on the ground, in the center of the hull.
- Geometry is built in place, so all objects have identity rotation and scale
  (transformations are applied by construction).
- Hierarchy: Hull > Turret > Barrel, so turret and barrel can rotate later.
"""

import math
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix

REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT = REPO_ROOT / "assets-src" / "blender" / "placeholder_tank.blend"


def material(name, rgb):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*rgb, 1.0)
    bsdf = mat.node_tree.nodes.get("Principled BSDF") if mat.node_tree else None
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        bsdf.inputs["Roughness"].default_value = 0.8
    return mat


def mesh_object(name, build, mat, parent=None, location=(0.0, 0.0, 0.0)):
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    build(bm)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    obj.parent = parent
    obj.location = location
    return obj


def box(size, center):
    def build(bm):
        matrix = Matrix.Translation(center) @ Matrix.Diagonal((*size, 1.0))
        bmesh.ops.create_cube(bm, size=1.0, matrix=matrix)

    return build


def barrel_along_y(radius, length):
    def build(bm):
        # The cone primitive points along Z; rotate it onto Y and shift it so
        # it starts at the local origin (the mantlet) and extends forward.
        matrix = Matrix.Translation((0.0, length / 2.0, 0.0)) @ Matrix.Rotation(
            math.radians(90.0), 4, "X"
        )
        bmesh.ops.create_cone(
            bm,
            cap_ends=True,
            segments=16,
            radius1=radius,
            radius2=radius,
            depth=length,
            matrix=matrix,
        )

    return build


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.unit_settings.scale_length = 1.0

    olive = material("Olive", (0.30, 0.32, 0.17))
    dark = material("DarkOlive", (0.20, 0.21, 0.12))
    steel = material("Steel", (0.12, 0.12, 0.12))

    hull = mesh_object("Hull", box((3.0, 6.0, 1.2), (0.0, 0.0, 0.6)), olive)
    turret = mesh_object(
        "Turret",
        box((2.2, 2.6, 0.9), (0.0, 0.0, 0.45)),
        dark,
        parent=hull,
        location=(0.0, -0.3, 1.2),
    )
    mesh_object(
        "Barrel",
        barrel_along_y(0.1, 3.6),
        steel,
        parent=turret,
        location=(0.0, 1.2, 0.45),
    )

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT))
    print(f"Saved {OUTPUT}")


main()
