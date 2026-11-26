"""Assembles one infantry soldier and exports it.

Run headless:
    blender -b --factory-startup --python tools/blender/build_soldier.py -- <faction> <role> [biome]
    blender -b --factory-startup --python tools/blender/build_soldier.py -- --all
    blender -b --factory-startup --python tools/blender/build_soldier.py -- --tier2   (prompt 008 roles)

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
    parser.add_argument("--all", action="store_true", help="build every faction/role and shared anim libraries")
    parser.add_argument("--tier2", action="store_true", help="build the roles added in prompt 008")
    parser.add_argument("--anims", action="store_true", help="build only the shared animation libraries")
    parser.add_argument("--skip-anims", action="store_true", help="skip rebuilding shared animation libraries")
    parser.add_argument("--headgear", choices=sorted(gear.HEADGEAR), help="override headgear")
    parser.add_argument("--no-anim", action="store_true", help="legacy alias for --skip-anims")
    parser.add_argument("--no-export", action="store_true", help="only save the .blend")
    args = parser.parse_args(argv)
    if not (args.all or args.tier2 or args.anims) and (args.faction is None or args.role is None):
        parser.error("give <faction> <role>, --all, --tier2 or --anims")
    return args


def blend_path(faction, role):
    return REPO_ROOT / "assets-src" / "blender" / faction / "infantry" / f"{role}.blend"


def glb_path(faction, role):
    return REPO_ROOT / "godot" / "assets" / "models" / faction / "infantry" / f"{role}.glb"


def anim_glb_path(lib_id):
    return REPO_ROOT / "godot" / "assets" / "models" / "infantry" / "anims" / f"{lib_id}.glb"


def build(faction, role, biome="forest", headgear=None, animate=False):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.render.fps = animation.FPS

    kit = loadouts.loadout(faction, role)
    if headgear:
        kit["headgear"] = headgear
    lib_id = loadouts.anim_library_for(kit["weapon"])
    root_name = f"{faction}_inf_{role}"
    collection = bpy.data.collections.new(root_name)
    scene.collection.children.link(collection)

    mats = materials.make_materials(faction, biome)
    root = bpy.data.objects.new(root_name, None)
    collection.objects.link(root)
    root["faction"], root["role"], root["biome"] = faction, role, biome
    root["anim_library"] = lib_id

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
                 weapon_tris=weapon.mesh.triangles() if weapon else 0,
                 anim_library=lib_id)
    return dict(root=root, arm=arm, body=body_obj, gear=gear_obj, clips=clips, stats=stats)


def _prune_constant_anim_channels(path):
    """Strips animation channels whose `(node, path)` never deviates from the
    node's rest TRS across all clips in the library GLB, then repacks the binary
    buffer so unreferenced accessors/bufferViews are dropped (prompt 213).

    Because only globally invariant `(node, path)` pairs are removed (never
    per-clip constant poses of bones that move in other clips), switching
    between any two clips stays bit-identical in Godot and Three.js.
    """
    import struct

    raw = path.read_bytes()
    jlen = struct.unpack_from("<I", raw, 12)[0]
    g = json.loads(raw[20:20 + jlen].decode("utf-8"))
    if not g.get("animations"):
        return
    bin_off = 20 + jlen + 8
    bdata = raw[bin_off:]
    bvs = g.get("bufferViews", [])
    accs = g.get("accessors", [])
    nodes = g.get("nodes", [])

    def read_floats(ai):
        a = accs[ai]
        bv = bvs[a["bufferView"]]
        off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        cnt = a["count"]
        comp = {"SCALAR": 1, "VEC3": 3, "VEC4": 4}[a["type"]]
        vals = struct.unpack_from(f"<{cnt * comp}f", bdata, off)
        return [vals[i * comp:(i + 1) * comp] for i in range(cnt)]

    defaults = {"translation": (0.0, 0.0, 0.0), "rotation": (0.0, 0.0, 0.0, 1.0), "scale": (1.0, 1.0, 1.0)}
    active = set()
    for anim in g["animations"]:
        for ch in anim["channels"]:
            ni = ch["target"]["node"]
            cpath = ch["target"]["path"]
            if (ni, cpath) in active or cpath not in defaults:
                continue
            rest = tuple(nodes[ni].get(cpath, defaults[cpath]))
            for v in read_floats(anim["samplers"][ch["sampler"]]["output"]):
                if cpath == "rotation":
                    if abs(sum(a * b for a, b in zip(v, rest))) < 0.99999:
                        active.add((ni, cpath))
                        break
                elif any(abs(a - b) > 1e-4 for a, b in zip(v, rest)):
                    active.add((ni, cpath))
                    break

    for anim in g["animations"]:
        new_channels = []
        new_samplers = []
        remap = {}
        for ch in anim["channels"]:
            key = (ch["target"]["node"], ch["target"]["path"])
            if key not in active:
                continue
            old_s = ch["sampler"]
            if old_s not in remap:
                remap[old_s] = len(new_samplers)
                new_samplers.append(anim["samplers"][old_s])
            ch = dict(ch, sampler=remap[old_s])
            new_channels.append(ch)
        if not new_channels and anim["channels"]:
            ch0 = dict(anim["channels"][0], sampler=0)
            new_samplers = [anim["samplers"][anim["channels"][0]["sampler"]]]
            new_channels = [ch0]
        anim["channels"] = new_channels
        anim["samplers"] = new_samplers

    # Collect referenced accessors (animations + skins if any) and repack binary buffer.
    used_accs = []
    used_set = set()

    def mark_acc(ai):
        if ai not in used_set:
            used_set.add(ai)
            used_accs.append(ai)

    for s in g.get("skins", []):
        if "inverseBindMatrices" in s:
            mark_acc(s["inverseBindMatrices"])
    for anim in g["animations"]:
        for sm in anim["samplers"]:
            mark_acc(sm["input"])
            mark_acc(sm["output"])

    acc_map = {}
    new_accs = []
    new_bvs = []
    new_bin = bytearray()
    bv_map = {}
    for old_ai in used_accs:
        a = dict(accs[old_ai])
        old_bvi = a["bufferView"]
        if old_bvi not in bv_map:
            bv = dict(bvs[old_bvi])
            off = bv.get("byteOffset", 0)
            blen = bv["byteLength"]
            while len(new_bin) % 4:
                new_bin.append(0)
            bv["byteOffset"] = len(new_bin)
            new_bin.extend(bdata[off:off + blen])
            bv_map[old_bvi] = len(new_bvs)
            new_bvs.append(bv)
        a["bufferView"] = bv_map[old_bvi]
        acc_map[old_ai] = len(new_accs)
        new_accs.append(a)

    for s in g.get("skins", []):
        if "inverseBindMatrices" in s:
            s["inverseBindMatrices"] = acc_map[s["inverseBindMatrices"]]
    for anim in g["animations"]:
        for sm in anim["samplers"]:
            sm["input"] = acc_map[sm["input"]]
            sm["output"] = acc_map[sm["output"]]

    while len(new_bin) % 4:
        new_bin.append(0)
    g["accessors"] = new_accs
    g["bufferViews"] = new_bvs
    if g.get("buffers"):
        g["buffers"][0]["byteLength"] = len(new_bin)

    jbytes = json.dumps(g, separators=(",", ":")).encode("utf-8")
    while len(jbytes) % 4:
        jbytes += b" "
    total_len = 12 + 8 + len(jbytes) + 8 + len(new_bin)
    out_bytes = (
        struct.pack("<4sII", b"glTF", 2, total_len)
        + struct.pack("<I4s", len(jbytes), b"JSON") + jbytes
        + struct.pack("<I4s", len(new_bin), b"BIN\x00") + bytes(new_bin)
    )
    path.write_bytes(out_bytes)


def export_glb(path, animations=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    kwargs = dict(
        filepath=str(path),
        export_format="GLB",
        export_apply=False,
        export_yup=True,
        export_extras=True,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        export_skins=True,
        export_morph=True,
        export_morph_normal=False,
        export_morph_tangent=False,
        export_animations=animations,
        export_animation_mode="NLA_TRACKS",
        export_force_sampling=True,
        export_vertex_color="ACTIVE",
        use_selection=False,
    )
    result = bpy.ops.export_scene.gltf(**kwargs)
    if "FINISHED" not in result:
        sys.exit(f"build_soldier.py: export failed: {result}")


def build_anim_library(lib_id):
    """Exports one shared animation library GLB (`Armature` at scene root, no meshes)
    containing all 61 NLA clips for `lib_id` (prompt 213)."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.render.fps = animation.FPS
    collection = bpy.data.collections.new(f"anim_{lib_id}")
    scene.collection.children.link(collection)
    arm = skeleton.create_armature(collection)
    wid = loadouts.ANIM_LIBRARIES[lib_id]
    weapon = weapons.build_weapon(wid) if wid else None
    kind = weapon.kind if weapon else None
    clips = animation.bake_all(arm, weapon, kind)
    out = anim_glb_path(lib_id)
    export_glb(out, animations=True)
    _prune_constant_anim_channels(out)
    size_kb = out.stat().st_size / 1024.0
    print(f"build_soldier: anim library {lib_id} ({wid or 'unarmed'}) "
          f"{len(clips)} clips, {size_kb:.1f} KB -> {out.relative_to(REPO_ROOT)}")
    return clips


def write_clip_metadata(clips):
    path = REPO_ROOT / "godot" / "assets" / "models" / "infantry" / "animations.json"
    data = {
        "_comment": "Generated by tools/blender/build_soldier.py. speed = authored "
                    "locomotion speed in m/s (real time); scale playback by game speed. "
                    "rise/advance (mount, dismount) = hip travel up and forward in m.",
        "fps": animation.FPS,
        "libraries": sorted(loadouts.ANIM_LIBRARIES),
        "roles": loadouts.role_anim_libraries(),
        "clips": {name: {"loop": c["loop"], "speed": c["speed"], "length": round(c["length"], 3),
                         **{k: v for k, v in c.items() if k not in ("loop", "speed", "length")}}
                  for name, c in clips.items()},
    }
    path.write_text(json.dumps(data, indent=2) + "\n")


def build_and_save(faction, role, biome, headgear=None, export=True):
    # Soldiers store mesh/skeleton/shape keys/extras only; animations live in
    # shared libraries under godot/assets/models/infantry/anims/ (prompt 213).
    result = build(faction, role, biome, headgear, animate=False)
    out = blend_path(faction, role)
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
    if export:
        export_glb(glb_path(faction, role), animations=False)
    s = result["stats"]
    print(f"build_soldier: {faction} {role} ({biome}, anim={s['anim_library']}) body "
          f"{s['body_tris']} + gear {s['gear_tris']} tris, weapon {s['weapon_tris']} tris "
          f"-> {out.relative_to(REPO_ROOT)}")
    return result


def main():
    args = parse_args()
    skip_anims = args.skip_anims or args.no_anim
    if args.anims:
        clips = None
        for lib_id in sorted(loadouts.ANIM_LIBRARIES):
            clips = build_anim_library(lib_id)
        if clips:
            write_clip_metadata(clips)
        return
    if args.all or args.tier2:
        if not skip_anims and not args.no_export:
            clips = None
            for lib_id in sorted(loadouts.ANIM_LIBRARIES):
                clips = build_anim_library(lib_id)
            if clips:
                write_clip_metadata(clips)
        for faction, role in loadouts.all_combinations():
            if args.tier2 and role not in loadouts.TIER2_ROLES:
                continue
            build_and_save(faction, role, args.biome, export=not args.no_export)
    else:
        res = build_and_save(args.faction, args.role, args.biome, args.headgear,
                             export=not args.no_export)
        if not skip_anims and not args.no_export:
            lib_id = res["stats"]["anim_library"]
            if not anim_glb_path(lib_id).exists():
                clips = build_anim_library(lib_id)
                write_clip_metadata(clips)


if __name__ == "__main__":
    main()
