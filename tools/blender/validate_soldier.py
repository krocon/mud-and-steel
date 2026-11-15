"""Validates the infantry soldiers (prompt 005). Exit code 1 if anything is wrong.

Run headless:
    blender -b --factory-startup --python tools/blender/validate_soldier.py [-- <file.blend> ...]

Without files it checks every assets-src/blender/<faction>/infantry/*.blend.

Checks per soldier
- height of the body 1.80 m +- 2 % in the rest pose, feet on Z = 0, origin centered
- root object named <faction>_inf_<role>, identity transform (rotation/scale applied)
- skeleton: all bones required by Godot's SkeletonProfileHumanoid, every deform bone
  name is a profile name, `Weapon` bone present, no IK helper bones left
- weapon parented to bone `Weapon` with a `Socket_Muzzle` empty (except unarmed roles)
- triangle budget (docs/ASSETS.md): body + gear <= 3000, weapon <= 800
- material slots only from the allowed list
- every animation clip exists, and the exported .glb exists and is not older
"""

import sys
from pathlib import Path

import bpy


sys.path.insert(0, str(Path(__file__).resolve().parent))

from soldier import animation, loadouts, skeleton  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
HEIGHT = 1.80
TOLERANCE = 0.02
BUDGET_SOLDIER = 3000
BUDGET_WEAPON = 800
ALLOWED_SLOTS = {"Paint", "Metal", "Rubber", "Wood", "Canvas", "Glass", "Skin", "Cloth",
                 "Leather", "Insignia"}
PROFILE_BONES = {
    "Root", "Hips", "Spine", "Chest", "UpperChest", "Neck", "Head", "LeftEye", "RightEye",
    "Jaw", "LeftShoulder", "LeftUpperArm", "LeftLowerArm", "LeftHand", "RightShoulder",
    "RightUpperArm", "RightLowerArm", "RightHand", "LeftUpperLeg", "LeftLowerLeg",
    "LeftFoot", "LeftToes", "RightUpperLeg", "RightLowerLeg", "RightFoot", "RightToes",
}


def tris(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def check(blend):
    errors = []
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    faction = blend.parent.parent.name
    role = blend.stem
    root_name = f"{faction}_inf_{role}"

    root = bpy.data.objects.get(root_name)
    if root is None:
        return [f"root object {root_name!r} missing"]
    if not _is_identity(root.matrix_world):
        errors.append("root transform is not identity")

    arm = bpy.data.objects.get("Armature")
    if arm is None or arm.type != "ARMATURE":
        return errors + ["armature missing"]
    for obj in bpy.data.objects:
        if obj.type in ("MESH", "ARMATURE") and not _is_identity(obj.matrix_basis) and \
                obj.parent_type != "BONE":
            errors.append(f"{obj.name}: transform not applied")
    bones = {b.name: b for b in arm.data.bones}
    missing = [b for b in skeleton.HUMANOID_REQUIRED if b not in bones]
    if missing:
        errors.append(f"humanoid bones missing: {missing}")
    for b in arm.data.bones:
        if b.use_deform and b.name not in PROFILE_BONES:
            errors.append(f"deform bone {b.name!r} is not a SkeletonProfileHumanoid name")
        if b.name.startswith(("IK_", "Pole_")):
            errors.append(f"helper bone {b.name!r} left in the export")
    if skeleton.WEAPON_BONE not in bones:
        errors.append("bone 'Weapon' missing")

    # Rest pose height of the body.
    arm.data.pose_position = "REST"
    bpy.context.view_layer.update()
    body = bpy.data.objects.get("Body")
    gear = bpy.data.objects.get("Gear")
    if body is None or gear is None:
        return errors + ["Body or Gear mesh missing"]
    deps = bpy.context.evaluated_depsgraph_get()
    zs, xs, ys = [], [], []
    for v in body.evaluated_get(deps).data.vertices:
        co = body.matrix_world @ v.co
        zs.append(co.z)
        xs.append(co.x)
        ys.append(co.y)
    height = max(zs)
    if abs(height - HEIGHT) > HEIGHT * TOLERANCE:
        errors.append(f"body height {height:.3f} m, expected {HEIGHT} m +- {TOLERANCE:.0%}")
    if abs(min(zs)) > 0.01:
        errors.append(f"feet not on the ground (min z {min(zs):.3f})")
    if abs((max(xs) + min(xs)) / 2) > 0.02:
        errors.append("body not centered on the origin (x)")
    arm.data.pose_position = "POSE"

    soldier_tris = tris(body) + tris(gear)
    if soldier_tris > BUDGET_SOLDIER:
        errors.append(f"body + gear {soldier_tris} triangles > {BUDGET_SOLDIER}")

    kit = loadouts.loadout(faction, role)
    weapon_objs = [o for o in bpy.data.objects if o.name.startswith("Weapon_")]
    if kit["weapon"]:
        if len(weapon_objs) != 1:
            errors.append(f"expected one weapon object, found {len(weapon_objs)}")
        else:
            w = weapon_objs[0]
            if w.parent != arm or w.parent_type != "BONE" or w.parent_bone != skeleton.WEAPON_BONE:
                errors.append(f"{w.name} is not parented to bone 'Weapon'")
            if not any(c.name.startswith("Socket_Muzzle") for c in w.children):
                errors.append(f"{w.name} has no Socket_Muzzle")
            if tris(w) > BUDGET_WEAPON:
                errors.append(f"{w.name} {tris(w)} triangles > {BUDGET_WEAPON}")
    elif weapon_objs:
        errors.append("unarmed role has a weapon")

    for obj in (body, gear, *weapon_objs):
        for slot in obj.material_slots:
            name = slot.material.name if slot.material else ""
            if name not in ALLOWED_SLOTS:
                errors.append(f"{obj.name}: material {name!r} is not an allowed slot")

    tracks = {t.name for t in arm.animation_data.nla_tracks} if arm.animation_data else set()
    missing_clips = [c for c in animation.CLIP_NAMES if c not in tracks]
    if missing_clips:
        errors.append(f"animation clips missing: {missing_clips}")

    glb = REPO_ROOT / "godot" / "assets" / "models" / faction / "infantry" / f"{role}.glb"
    if not glb.exists():
        errors.append(f"export missing: {glb.relative_to(REPO_ROOT)}")
    elif glb.stat().st_mtime + 1 < blend.stat().st_mtime:
        errors.append(f"export older than the .blend: {glb.relative_to(REPO_ROOT)}")

    print(f"  {faction}/{role}: height {height:.3f} m, {soldier_tris} tris, "
          f"{len(tracks)} clips")
    return errors


def _rel(path):
    try:
        return path.relative_to(REPO_ROOT)
    except ValueError:
        return path


def _is_identity(m):
    return all(abs(m[i][j] - (1.0 if i == j else 0.0)) < 1e-5 for i in range(4) for j in range(4))


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    files = [Path(a).resolve() for a in argv] or sorted(
        (REPO_ROOT / "assets-src" / "blender").glob("*/infantry/*.blend"))
    if not files:
        sys.exit("validate_soldier: no soldier .blend files found (run `just soldiers`)")
    failed = 0
    print(f"validate_soldier: {len(files)} soldiers")
    for blend in files:
        errors = check(blend)
        for e in errors:
            print(f"  ERROR {_rel(blend)}: {e}")
        failed += bool(errors)
    if failed:
        print(f"validate_soldier: {failed} of {len(files)} soldiers failed")
        sys.exit(1)
    print("validate_soldier: all soldiers ok")


main()
