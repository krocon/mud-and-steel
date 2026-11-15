"""Line-up render: rifleman and MG gunner of every faction in all three biomes, next to
the placeholder tank and the 1.80 m reference figure (assets-src/blender/reference).

Run headless (output goes to a scratch directory, never into the repository):
    blender -b --factory-startup --python tools/blender/soldier_lineup.py -- <out_dir>
"""

import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from soldier import materials, preview  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
BLEND_DIR = REPO_ROOT / "assets-src" / "blender"
ROLES = ["rifleman", "mg_gunner"]
FACTIONS = ["de", "su", "us"]


def append_collection(blend, name):
    with bpy.data.libraries.load(str(blend), link=False) as (src, dst):
        dst.collections = [name]
    coll = dst.collections[0]
    bpy.context.scene.collection.children.link(coll)
    return coll


def append_objects(blend):
    with bpy.data.libraries.load(str(blend), link=False) as (src, dst):
        dst.objects = list(src.objects)
    objs = [o for o in dst.objects if o is not None]
    for o in objs:
        bpy.context.scene.collection.objects.link(o)
    return objs


def recolor(coll, faction, biome):
    colors = materials.slot_colors(faction, biome)
    done = set()
    for obj in coll.all_objects:
        if obj.type != "MESH":
            continue
        for slot in obj.material_slots:
            mat = slot.material
            if mat is None or mat.name in done:
                continue
            done.add(mat.name)
            base = mat.name.split(".")[0]
            if base not in colors:
                continue
            rgb = (*materials.srgb_to_linear(colors[base]), 1.0)
            mat.diffuse_color = rgb
            if mat.node_tree and "Tint" in mat.node_tree.nodes:
                mat.node_tree.nodes["Tint"].inputs[6].default_value = rgb


def pose_idle(coll):
    for obj in coll.all_objects:
        if obj.type == "ARMATURE" and obj.animation_data:
            for track in obj.animation_data.nla_tracks:
                if track.name.startswith("idle") and track.strips:
                    action = track.strips[0].action
                    obj.animation_data.action = action
                    if action.slots:
                        obj.animation_data.action_slot = action.slots[0]


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out_dir = Path(argv[0] if argv else "lineup").resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    try:
        cam = preview.setup_scene((1600, 900), engine="BLENDER_EEVEE")
    except TypeError:
        cam = preview.setup_scene((1600, 900))
    preview.ground(60)
    sun = bpy.data.objects.new("Sun", bpy.data.lights.new("Sun", "SUN"))
    sun.data.energy = 4.0
    sun.rotation_euler = (0.75, 0.2, 0.6)
    bpy.context.scene.collection.objects.link(sun)

    # Soldiers: one row per faction, rifleman x3 biomes then MG gunner x3 biomes.
    for row, faction in enumerate(FACTIONS):
        x = 0.0
        for role in ROLES:
            blend = BLEND_DIR / faction / "infantry" / f"{role}.blend"
            for biome in materials.BIOMES:
                coll = append_collection(blend, f"{faction}_inf_{role}")
                root = next(o for o in coll.objects if o.parent is None and o.type == "EMPTY")
                root.location = Vector((x, -row * 1.8, 0))
                recolor(coll, faction, biome)
                pose_idle(coll)
                x += 1.05
            x += 0.4

    # Placeholder tank (7 m class) and the reference figure with scale bars.
    tank = append_objects(BLEND_DIR / "placeholder_tank.blend")
    for o in tank:
        if o.parent is None:
            o.location += Vector((-5.5, -1.8, 0))
    ref = append_collection(BLEND_DIR / "reference" / "scale_reference.blend", "ScaleReference")
    for o in ref.objects:
        if o.parent is None:
            o.location += Vector((1.0, 2.0, 0))

    bpy.context.view_layer.update()
    shots = {
        "lineup_front.png": dict(target=(1.0, -1.6, 0.9), distance=17, yaw=200, pitch=18, lens=50),
        "lineup_game.png": dict(target=(1.0, -1.6, 0.0), distance=40, yaw=160, pitch=55, lens=60),
        "lineup_close.png": dict(target=(2.0, 0.0, 0.9), distance=6, yaw=195, pitch=12, lens=50),
    }
    for name, s in shots.items():
        preview.aim_camera(cam, s["target"], s["distance"], s["yaw"], s["pitch"], lens=s["lens"])
        preview.render(out_dir / name)
        print(f"soldier_lineup: {out_dir / name}")


main()
