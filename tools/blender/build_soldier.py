"""Assembles one infantry soldier and exports it.

Run headless:
    blender -b --factory-startup --python tools/blender/build_soldier.py -- <faction> <role> [biome]
    blender -b --factory-startup --python tools/blender/build_soldier.py -- --all

Writes
    assets-src/blender/<faction>/infantry/<role>.blend
    godot/assets/models/<faction>/infantry/<role>.glb
    godot/assets/models/infantry/animations.json   (clip names, loop flags, speeds)

The model is the same for every biome: camouflage and uniform colors are switched by
the material system in Godot (soldier_look.gd). `biome` only sets the colors stored in
the .blend/.glb (default forest), e.g. for renders.

Model layout (docs/ASSETS.md, section "Infantry"):
    <faction>_inf_<role>   empty, root, origin between the feet
      Armature             humanoid skeleton (SkeletonProfileHumanoid names) + `Weapon`
        Body               skinned body, shape keys Stocky/Slim/FaceB/FaceC
        Gear               skinned headgear and equipment, shape keys Stocky/Slim
        Weapon_<id>        child of bone `Weapon`
          Socket_Muzzle    empty at the muzzle, +Y forward
"""

import argparse
import json
import sys
from pathlib import Path

import bpy
from mathutils import Matrix

sys.path.insert(0, str(Path(__file__).resolve().parent))

from soldier import animation, body, gear, loadouts, materials, skeleton, weapons  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description="Build an infantry soldier.")
    parser.add_argument("faction", nargs="?", choices=loadouts.FACTIONS)
    parser.add_argument("role", nargs="?")
    parser.add_argument("biome", nargs="?", default="forest", choices=materials.BIOMES)
    parser.add_argument("--all", action="store_true", help="build every faction/role")
    parser.add_argument("--headgear", choices=sorted(gear.HEADGEAR), help="override headgear")
    parser.add_argument("--no-anim", action="store_true", help="skip animations (fast preview)")
    parser.add_argument("--no-export", action="store_true", help="only save the .blend")
    args = parser.parse_args(argv)
    if not args.all and (args.faction is None or args.role is None):
        parser.error("give <faction> <role> or --all")
    return args


def blend_path(faction, role):
    return REPO_ROOT / "assets-src" / "blender" / faction / "infantry" / f"{role}.blend"


def glb_path(faction, role):
    return REPO_ROOT / "godot" / "assets" / "models" / faction / "infantry" / f"{role}.glb"


def build(faction, role, biome="forest", headgear=None, animate=True):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.render.fps = animation.FPS

    kit = loadouts.loadout(faction, role)
    if headgear:
        kit["headgear"] = headgear
    root_name = f"{faction}_inf_{role}"
    collection = bpy.data.collections.new(root_name)
    scene.collection.children.link(collection)

    mats = materials.make_materials(faction, biome)
    root = bpy.data.objects.new(root_name, None)
    collection.objects.link(root)
    root["faction"], root["role"], root["biome"] = faction, role, biome

    arm = skeleton.create_armature(collection)
    arm.parent = root

    body_mesh = body.build_body(kit["uniform"])
    body_obj = body_mesh.to_object("Body", mats, collection, arm, body.shape_keys_for(body_mesh))

    gear_mesh = gear.build_gear(faction, kit["headgear"], kit["items"])
    gear_obj = gear_mesh.to_object("Gear", mats, collection, arm,
                                   body.shape_keys_for(gear_mesh, include_face=False))

    weapon = None
    kind = None
    if kit["weapon"]:
        weapon = weapons.build_weapon(kit["weapon"])
        kind = weapon.kind
        wobj = weapon.mesh.to_object(f"Weapon_{weapon.id}", mats, collection)
        rest = arm.data.bones[skeleton.WEAPON_BONE].matrix_local.copy()
        wobj.parent = arm
        wobj.parent_type = "BONE"
        wobj.parent_bone = skeleton.WEAPON_BONE
        bpy.context.view_layer.update()
        wobj.matrix_world = rest
        muzzle = bpy.data.objects.new("Socket_Muzzle", None)
        muzzle.empty_display_type = "SINGLE_ARROW"
        muzzle.empty_display_size = 0.1
        collection.objects.link(muzzle)
        muzzle.parent = wobj
        # Single arrow points along local Z; rotate so +Z of the empty is the weapon's +Y.
        muzzle.matrix_parent_inverse = Matrix.Identity(4)
        muzzle.location = weapon.points["muzzle"]
        muzzle.rotation_euler = (-1.5707963, 0, 0)
        wobj["weapon"] = weapon.id

    clips = None
    if animate:
        clips = animation.bake_all(arm, weapon, kind)
    else:
        skeleton.remove_helper_bones(arm)

    stats = dict(body_tris=body_mesh.triangles(), gear_tris=gear_mesh.triangles(),
                 weapon_tris=weapon.mesh.triangles() if weapon else 0)
    return dict(root=root, arm=arm, body=body_obj, gear=gear_obj, clips=clips, stats=stats)


def export_glb(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    kwargs = dict(
        filepath=str(path),
        export_format="GLB",
        export_apply=False,
        export_yup=True,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        export_skins=True,
        export_morph=True,
        export_animations=True,
        export_animation_mode="NLA_TRACKS",
        export_force_sampling=True,
        export_vertex_color="ACTIVE",
        use_selection=False,
    )
    result = bpy.ops.export_scene.gltf(**kwargs)
    if "FINISHED" not in result:
        sys.exit(f"build_soldier.py: export failed: {result}")


def write_clip_metadata(clips):
    path = REPO_ROOT / "godot" / "assets" / "models" / "infantry" / "animations.json"
    data = {
        "_comment": "Generated by tools/blender/build_soldier.py. speed = authored "
                    "locomotion speed in m/s (real time); scale playback by game speed.",
        "fps": animation.FPS,
        "clips": {name: {"loop": c["loop"], "speed": c["speed"], "length": round(c["length"], 3)}
                  for name, c in clips.items()},
    }
    path.write_text(json.dumps(data, indent=2) + "\n")


def build_and_save(faction, role, biome, headgear=None, animate=True, export=True):
    result = build(faction, role, biome, headgear, animate)
    out = blend_path(faction, role)
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
    if export:
        export_glb(glb_path(faction, role))
        if result["clips"]:
            write_clip_metadata(result["clips"])
    s = result["stats"]
    print(f"build_soldier: {faction} {role} ({biome}) body {s['body_tris']} + gear "
          f"{s['gear_tris']} tris, weapon {s['weapon_tris']} tris -> {out.relative_to(REPO_ROOT)}")
    return result


def main():
    args = parse_args()
    if args.all:
        for faction, role in loadouts.all_combinations():
            build_and_save(faction, role, args.biome, animate=not args.no_anim,
                           export=not args.no_export)
    else:
        build_and_save(args.faction, args.role, args.biome, args.headgear,
                       animate=not args.no_anim, export=not args.no_export)


if __name__ == "__main__":
    main()
