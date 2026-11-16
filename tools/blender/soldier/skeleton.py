"""The soldier skeleton.

Bone names are exactly those of Godot's `SkeletonProfileHumanoid` (without fingers,
eyes and jaw), so Godot's retargeting maps every bone automatically. One extra bone,
`Weapon`, carries the weapon (child of `Root`); helper bones for inverse kinematics
(`IK_*`, `Pole_*`) exist only while animations are baked and are removed before export.

Rest pose: A-pose (arms 45 degrees down), standing, facing +Y, feet on Z = 0.
"""

import bpy
from mathutils import Vector

# Arms hang 45 degrees from the vertical in the rest pose.
_S = 0.7071

# name: (parent, head, tail, roll axis: the bone's local Z points along this vector)
BONES = {
    "Root": (None, (0, 0, 0), (0, 0.25, 0), (0, 0, 1)),
    "Hips": ("Root", (0, 0, 0.95), (0, 0, 1.05), (0, 1, 0)),
    "Spine": ("Hips", (0, 0, 1.05), (0, 0, 1.17), (0, 1, 0)),
    "Chest": ("Spine", (0, 0, 1.17), (0, 0, 1.30), (0, 1, 0)),
    "UpperChest": ("Chest", (0, 0, 1.30), (0, 0, 1.46), (0, 1, 0)),
    "Neck": ("UpperChest", (0, 0, 1.47), (0, 0.01, 1.58), (0, 1, 0)),
    "Head": ("Neck", (0, 0.01, 1.58), (0, 0.01, 1.80), (0, 1, 0)),
}

UPPER_ARM = 0.30
LOWER_ARM = 0.26
HAND = 0.17


def _side_bones(side, sx):
    p = "Left" if side == "L" else "Right"
    shoulder = Vector((sx * 0.195, 0.0, 1.43))
    direction = Vector((sx * _S, 0.0, -_S))
    elbow = shoulder + direction * UPPER_ARM + Vector((0, -0.02, 0))
    wrist = elbow + direction * LOWER_ARM + Vector((0, 0.02, 0))
    hand_tip = wrist + direction * HAND
    hip = Vector((sx * 0.100, 0.0, 0.93))
    knee = Vector((sx * 0.100, 0.015, 0.50))
    ankle = Vector((sx * 0.100, -0.01, 0.09))
    ball = Vector((sx * 0.100, 0.12, 0.03))
    toe = Vector((sx * 0.100, 0.21, 0.03))
    return {
        f"{p}Shoulder": ("UpperChest", (sx * 0.03, 0, 1.43), tuple(shoulder), (0, 1, 0)),
        f"{p}UpperArm": (f"{p}Shoulder", tuple(shoulder), tuple(elbow), (0, 1, 0)),
        f"{p}LowerArm": (f"{p}UpperArm", tuple(elbow), tuple(wrist), (0, 1, 0)),
        f"{p}Hand": (f"{p}LowerArm", tuple(wrist), tuple(hand_tip), (0, 1, 0)),
        f"{p}UpperLeg": ("Hips", tuple(hip), tuple(knee), (0, 1, 0)),
        f"{p}LowerLeg": (f"{p}UpperLeg", tuple(knee), tuple(ankle), (0, 1, 0)),
        f"{p}Foot": (f"{p}LowerLeg", tuple(ankle), tuple(ball), (0, 0, 1)),
        f"{p}Toes": (f"{p}Foot", tuple(ball), tuple(toe), (0, 0, 1)),
    }


BONES.update(_side_bones("L", -1.0))
BONES.update(_side_bones("R", 1.0))

# Deform bones in hierarchy order (parents first).
DEFORM_BONES = list(BONES.keys())

# Exported, non-deforming: the weapon carrier. Local axes: X right, Y forward, Z up.
WEAPON_BONE = "Weapon"
EXTRA_BONES = {
    WEAPON_BONE: ("Root", (0.3, 0.0, 1.0), (0.3, 0.2, 1.0), (0, 0, 1)),
}


def _helper_bones():
    helpers = {}
    for p in ("Left", "Right"):
        side = p[0]
        foot = BONES[f"{p}Foot"]
        helpers[f"IK_Foot_{side}"] = ("Root", foot[1], foot[2], foot[3])
        hand = BONES[f"{p}Hand"]
        helpers[f"IK_Hand_{side}"] = ("Root", hand[1], hand[2], hand[3])
        sx = -1 if side == "L" else 1
        helpers[f"Pole_Arm_{side}"] = ("Root", (sx * 0.5, -0.4, 1.1), (sx * 0.5, -0.3, 1.1), (0, 0, 1))
        helpers[f"Pole_Leg_{side}"] = ("Root", (sx * 0.1, 0.8, 0.5), (sx * 0.1, 0.9, 0.5), (0, 0, 1))
    return helpers


HELPER_BONES = _helper_bones()

# Names Godot's SkeletonProfileHumanoid requires; the validator checks them.
HUMANOID_REQUIRED = [
    "Hips", "Spine", "Head",
    "LeftShoulder", "LeftUpperArm", "LeftLowerArm", "LeftHand",
    "RightShoulder", "RightUpperArm", "RightLowerArm", "RightHand",
    "LeftUpperLeg", "LeftLowerLeg", "LeftFoot",
    "RightUpperLeg", "RightLowerLeg", "RightFoot",
]


def create_armature(collection, name="Armature"):
    data = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, data)
    collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    all_bones = {**BONES, **EXTRA_BONES, **HELPER_BONES}
    for bname, (parent, head, tail, roll_axis) in all_bones.items():
        eb = data.edit_bones.new(bname)
        eb.head = head
        eb.tail = tail
        eb.align_roll(Vector(roll_axis))
    for bname, (parent, *_rest) in all_bones.items():
        eb = data.edit_bones[bname]
        if parent:
            eb.parent = data.edit_bones[parent]
        eb.use_deform = bname in BONES
    bpy.ops.object.mode_set(mode="OBJECT")
    for pb in obj.pose.bones:
        pb.rotation_mode = "QUATERNION"
    return obj


def remove_helper_bones(armature):
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    for bname in HELPER_BONES:
        eb = armature.data.edit_bones.get(bname)
        if eb is not None:
            armature.data.edit_bones.remove(eb)
    bpy.ops.object.mode_set(mode="OBJECT")
