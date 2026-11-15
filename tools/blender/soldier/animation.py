"""Procedural animation: poses written in code, solved with Blender IK, baked to keys.

How a clip is made
- A clip is a function `pose(t)` (t in seconds) returning a pose dict, either
  parametric (walk cycle) or interpolated between key poses (`keyed`).
- A pose holds forward-kinematic rotations in character axes (X right, Y forward,
  Z up; `rot`), a hip offset (`hips`), where the weapon is (`weapon`, a hold spec)
  and inverse-kinematic targets for hands and feet.
- Each frame: apply FK, place the weapon relative to its anchor bone, move the IK
  helper bones (hands to the weapon grips, feet to their ground positions), let
  Blender solve, read back the local transforms. After all clips the constraints and
  helper bones are removed and the samples become plain keyframes (one action per
  clip), so the export contains only ordinary bone animation.

Rotation convention for `rot` (degrees, applied relative to the parent):
X positive = a downward bone (leg, arm) swings forward, an upward bone (spine, neck,
head) leans back. Z positive = turn left (seen from above). Y positive = the top of
an upward bone tilts to the right.

Clips are authored at real speed. Locomotion speeds (m/s, real) are in CLIPS; the
game scales playback by game speed / authored speed (time factor, docs/SCALE.md).
"""

import math

import bpy
from mathutils import Euler, Matrix, Quaternion, Vector

from . import skeleton
from .weapons import MORTAR_ELEVATION

FPS = 24

# ---------------------------------------------------------------------------------------
# Rig: IK constraints on the helper bones
# ---------------------------------------------------------------------------------------

ARM_CHAINS = {"L": "LeftLowerArm", "R": "RightLowerArm"}
LEG_CHAINS = {"L": "LeftLowerLeg", "R": "RightLowerLeg"}


def setup_ik(arm):
    pose = arm.pose
    for side in ("L", "R"):
        for chain, target, pole in ((ARM_CHAINS[side], f"IK_Hand_{side}", f"Pole_Arm_{side}"),
                                    (LEG_CHAINS[side], f"IK_Foot_{side}", f"Pole_Leg_{side}")):
            c = pose.bones[chain].constraints.new("IK")
            c.name = "IK"
            c.target = arm
            c.subtarget = target
            c.pole_target = arm
            c.pole_subtarget = pole
            c.chain_count = 2
            c.influence = 0.0
        foot = "LeftFoot" if side == "L" else "RightFoot"
        cr = pose.bones[foot].constraints.new("COPY_ROTATION")
        cr.name = "FootRot"
        cr.target = arm
        cr.subtarget = f"IK_Foot_{side}"
        cr.influence = 0.0
    _calibrate_poles(arm)


def _calibrate_poles(arm):
    """Finds the pole angle per chain that puts the joint nearest to the pole target."""
    bpy.context.view_layer.update()
    for side in ("L", "R"):
        for chain, target, pole in ((ARM_CHAINS[side], f"IK_Hand_{side}", f"Pole_Arm_{side}"),
                                    (LEG_CHAINS[side], f"IK_Foot_{side}", f"Pole_Leg_{side}")):
            pb = arm.pose.bones[chain]
            c = pb.constraints["IK"]
            c.influence = 1.0
            # Pull the target towards the root so the chain must bend.
            tgt = arm.pose.bones[target]
            upper = pb.parent
            start = upper.head.copy()
            rest_target = arm.data.bones[target].head_local.copy()
            tgt.matrix = Matrix.Translation(start.lerp(rest_target, 0.7)) @ \
                arm.data.bones[target].matrix_local.to_3x3().to_4x4()
            pole_pos = arm.pose.bones[pole].head.copy()
            best = None
            for deg in range(-180, 180, 15):
                c.pole_angle = math.radians(deg)
                bpy.context.view_layer.update()
                d = (pb.head - pole_pos).length
                if best is None or d < best[0]:
                    best = (d, deg)
            c.pole_angle = math.radians(best[1])
            c.influence = 0.0
            tgt.matrix_basis = Matrix.Identity(4)
    bpy.context.view_layer.update()


# ---------------------------------------------------------------------------------------
# Pose helpers
# ---------------------------------------------------------------------------------------


def mirror(rot):
    """Rotation of the opposite side: X stays, Y and Z flip."""
    x, y, z = rot
    return (x, -y, -z)


def both(name, right_rot):
    """Same pose for both sides: {'RightX': rot, 'LeftX': mirrored}."""
    return {f"Right{name}": right_rot, f"Left{name}": mirror(right_rot)}


ARMS_RELAXED = {
    **both("UpperArm", (4, 38, 0)),
    **both("LowerArm", (18, 0, 0)),
}

SHOULDER_POCKET = (0.135, 0.105, 1.425)


def hold(kind, name):
    """Weapon hold spec by weapon kind and hold name."""
    if name == "aim":
        if kind == "flame":
            return dict(anchor="UpperChest", at=(0.17, 0.20, 1.05), point="grip_r",
                        rot=(3, 0, 0), frame="root")
        if kind == "mortar":
            return hold(kind, "ready")
        return dict(anchor="UpperChest", at=SHOULDER_POCKET, point="butt", rot=(0, 0, 0),
                    frame="root")
    if name == "ready":
        if kind in ("launcher", "atr", "mortar"):
            return hold(kind, "shoulder")
        if kind == "mg":
            return dict(anchor="UpperChest", at=(0.19, 0.12, 1.02), point="grip_r",
                        rot=(6, 0, 8), frame="anchor")
        return dict(anchor="UpperChest", at=(0.15, 0.20, 1.08), point="grip_r",
                    rot=(38, -10, 28), frame="anchor")
    if name == "port":  # running: higher and tighter to the body
        if kind in ("launcher", "atr", "mortar", "mg"):
            return hold(kind, "ready")
        return dict(anchor="UpperChest", at=(0.13, 0.20, 1.15), point="grip_r",
                    rot=(50, -12, 35), frame="anchor")
    if name == "shoulder":
        return dict(anchor="UpperChest", at=(0.20, 0.04, 1.36), point="grip_r",
                    rot=(18, 0, 0), frame="anchor")
    if name == "mg_shoulder":
        return dict(anchor="UpperChest", at=(0.17, 0.02, 1.53), point="center",
                    rot=(8, 0, 0), frame="anchor")
    if name == "slung":
        return dict(anchor="UpperChest", at=(0.0, -0.22, 1.22), point="center",
                    rot=(90, 32, 0), frame="anchor")
    if name == "left_low":
        return dict(anchor="UpperChest", at=(-0.23, 0.14, 1.02), point="grip_l",
                    rot=(-45, 0, 8), frame="anchor")
    if name == "reload":
        return dict(anchor="UpperChest", at=(0.14, 0.24, 1.12), point="grip_r",
                    rot=(25, -35, 15), frame="anchor")
    if name == "deployed":  # mortar on its baseplate in front of the gunner
        return dict(anchor="Root", at=(0.05, 0.62, 0.0), point="ground",
                    rot=(MORTAR_ELEVATION, 0, 0), frame="root")
    if name == "dropped":
        return dict(anchor="Root", at=(0.35, 0.55, 0.035), point="center", rot=(0, 90, 30),
                    frame="root")
    raise KeyError(name)


def grips(kind, hold_name):
    """Hand targets that go with a hold: (left, right) weapon points or None (FK)."""
    if hold_name in ("shoulder", "mg_shoulder"):
        return None, ("weapon", "grip_r") if hold_name == "shoulder" else ("weapon", "center_fwd")
    if hold_name in ("slung", "dropped", "deployed"):
        return None, None
    if hold_name == "left_low":
        return ("weapon", "grip_l"), None
    if hold_name in ("ready", "port") and kind == "mg":
        return ("weapon", "fore"), ("weapon", "grip_r")
    if hold_name in ("ready", "port") and kind in ("launcher", "atr", "mortar"):
        return None, ("weapon", "grip_r")
    return ("weapon", "grip_l"), ("weapon", "grip_r")


def foot(side, y=0.0, z=0.0, x=0.0, pitch=0.0):
    """IK foot target: ankle position relative to its rest position (root space)."""
    sx = -1 if side == "L" else 1
    return (Vector((sx * 0.095 + x, -0.01 + y, 0.09 + z)), pitch)


def standing_feet(spread=0.0, l_fwd=0.0, r_fwd=0.0):
    return {"foot_l": (foot("L", y=l_fwd, x=-spread), 1.0),
            "foot_r": (foot("R", y=r_fwd, x=spread), 1.0)}


# ---------------------------------------------------------------------------------------
# Clip context: builds poses for one weapon kind
# ---------------------------------------------------------------------------------------


class Poses:
    def __init__(self, kind):
        self.kind = kind or "none"

    def armed(self, hold_name, infl=1.0):
        """weapon + hand targets for a hold."""
        if self.kind == "none":
            return {}
        l, r = grips(self.kind, hold_name)
        out = {"weapon": hold(self.kind, hold_name)}
        out["hand_l"] = (l, infl if l else 0.0)
        out["hand_r"] = (r, infl if r else 0.0)
        return out

    def stand(self, hold_name="ready", **extra):
        p = {"rot": {"Spine": (-2, 0, 0), "Neck": (-3, 0, 0)}, **standing_feet(0.02)}
        p.update(self.armed(hold_name))
        p.update(extra)
        return p

    def aim_stand(self):
        rot = {"Hips": (0, 0, -28), "Spine": (-4, 0, 12), "Chest": (-3, 0, 10),
               "UpperChest": (0, 0, 6), "Neck": (-6, 0, 0), "Head": (-14, -8, 0)}
        p = {"rot": rot, "foot_l": (foot("L", y=0.22, x=-0.06), 1.0),
             "foot_r": (foot("R", y=-0.12, x=0.04, pitch=0), 1.0),
             "hips": Vector((0, 0.03, -0.03))}
        p.update(self.armed("aim"))
        return p

    def kneel(self, hold_name="ready"):
        rot = {"Spine": (-6, 0, 0), "Chest": (-4, 0, 0), "Neck": (-4, 0, 0)}
        p = {"rot": rot, "hips": Vector((0, -0.08, -0.44)),
             "foot_l": (foot("L", y=0.36, x=-0.03), 1.0),
             "foot_r": (foot("R", y=-0.40, z=0.07, pitch=-62), 1.0)}
        p.update(self.armed(hold_name))
        return p

    def aim_kneel(self):
        p = self.kneel("aim")
        p["rot"] = {"Hips": (0, 0, -25), "Spine": (-8, 0, 12), "Chest": (-4, 0, 10),
                    "UpperChest": (0, 0, 5), "Neck": (-4, 0, 0), "Head": (-14, -8, 0)}
        return p

    def prone(self, hold_name="aim"):
        rot = {"Hips": (-88, 0, 0), "Spine": (12, 0, 0), "Chest": (8, 0, 0),
               "UpperChest": (6, 0, 0), "Neck": (28, 0, 0), "Head": (22, 0, 0),
               **both("UpperLeg", (0, -8, 0)), **both("Foot", (55, 0, 0)),
               **both("UpperArm", (60, 30, 0)), **both("LowerArm", (60, 0, 0))}
        p = {"rot": rot, "hips": Vector((0, -0.62, -0.80))}
        p.update(self.armed(hold_name))
        if hold_name == "aim":
            p["rot"]["Head"] = (16, -6, 0)
        return p

    def lying_back(self):
        rot = {"Hips": (86, 0, 0), "Spine": (-4, 0, 0), "Neck": (-10, 0, 8), "Head": (0, 0, 25),
               **both("UpperLeg", (8, -10, 0)), **both("LowerLeg", (-12, 0, 0)),
               **both("Foot", (-30, 0, 0)),
               "RightUpperArm": (-10, -40, 0), "LeftUpperArm": (20, 70, 0),
               **both("LowerArm", (30, 0, 0))}
        return {"rot": rot, "hips": Vector((0, 0.25, -0.83))}


# ---------------------------------------------------------------------------------------
# Interpolation
# ---------------------------------------------------------------------------------------


def smooth(s):
    return s * s * (3 - 2 * s)


def keyed(keys, loop=False):
    """keys: list of (time, pose). Returns pose(t) with smoothstep easing."""
    def pose(t):
        if t <= keys[0][0]:
            return blend(keys[0][1], keys[0][1], 0.0)
        for (t0, p0), (t1, p1) in zip(keys, keys[1:]):
            if t <= t1:
                s = (t - t0) / (t1 - t0) if t1 > t0 else 1.0
                return blend(p0, p1, smooth(s))
        return blend(keys[-1][1], keys[-1][1], 0.0)
    return pose


def blend(p0, p1, s):
    out = {"rot": {}, "hips": Vector((0, 0, 0)), "lists": {}}
    bones = set(p0.get("rot", {})) | set(p1.get("rot", {}))
    for b in bones:
        r0 = Vector(p0.get("rot", {}).get(b, (0, 0, 0)))
        r1 = Vector(p1.get("rot", {}).get(b, (0, 0, 0)))
        out["rot"][b] = tuple(r0.lerp(r1, s))
    out["hips"] = Vector(p0.get("hips", (0, 0, 0))).lerp(Vector(p1.get("hips", (0, 0, 0))), s)
    for key in ("weapon", "hand_l", "hand_r", "foot_l", "foot_r"):
        out["lists"][key] = [(p0.get(key), 1.0 - s), (p1.get(key), s)]
    out["kick"] = (1 - s) * p0.get("kick", 0.0) + s * p1.get("kick", 0.0)
    out["pole_l"] = p0.get("pole_l") if s < 0.5 else p1.get("pole_l")
    out["pole_r"] = p0.get("pole_r") if s < 0.5 else p1.get("pole_r")
    return out


def single(p):
    return blend(p, p, 0.0)


# ---------------------------------------------------------------------------------------
# Clip library
# ---------------------------------------------------------------------------------------

WALK_SPEED = 1.4   # m/s real
RUN_SPEED = 3.6
CRAWL_SPEED = 0.35
CARRY_SPEED = 1.2


def gait(t, period, speed, stance, lift, bounce, lean, hold_name, poses, arm_swing,
         carry=None):
    """Walk/run cycle with planted feet: each foot moves backwards at `speed` while on
    the ground, so the feet do not slide when the unit moves at that speed."""
    phase = (t / period) % 1.0
    stride = speed * period * stance  # distance the planted foot travels
    p = {"rot": {"Spine": (-lean, 0, 0), "Neck": (lean * 0.5, 0, 0)}}
    for side, offset in (("L", 0.0), ("R", 0.5)):
        ph = (phase + offset) % 1.0
        if ph < stance:
            u = ph / stance
            y = stride * (0.5 - u)
            z = 0.0
            pitch = 0.0
        else:
            u = (ph - stance) / (1.0 - stance)
            y = stride * (-0.5 + smooth(u))
            z = lift * math.sin(math.pi * u)
            pitch = -18.0 * math.sin(math.pi * min(1.0, u * 1.6))
        if ph < stance and u > 0.75:  # heel off before swing
            k = (u - 0.75) / 0.25
            pitch = -25.0 * k
            z = 0.04 * k
        p[f"foot_{side.lower()}"] = (foot(side, y=y + 0.03, z=z, pitch=pitch), 1.0)
    # Hips: two bobs per cycle, sway and turn with the stride.
    bob = math.cos(4.0 * math.pi * phase)
    p["hips"] = Vector((0.012 * math.sin(2 * math.pi * phase), 0.0, -0.035 - bounce * 0.5 + bounce * 0.5 * bob))
    p["rot"]["Hips"] = (0, 2.5 * math.sin(2 * math.pi * phase), 6 * math.cos(2 * math.pi * phase))
    p["rot"]["Chest"] = (0, 0, -5 * math.cos(2 * math.pi * phase))
    swing = arm_swing * math.cos(2 * math.pi * phase)
    p["rot"]["RightUpperArm"] = (swing + 4, 38, 0)
    p["rot"]["LeftUpperArm"] = (-swing + 4, -38, 0)
    p["rot"]["RightLowerArm"] = (22 + 0.3 * max(0, swing), 0, 0)
    p["rot"]["LeftLowerArm"] = (22 + 0.3 * max(0, -swing), 0, 0)
    p.update(poses.armed(hold_name))
    if carry:
        p.update(carry)
    return p


def crawl(t, period, poses):
    phase = (t / period) % 1.0
    base = poses.prone("slung")
    base.pop("hand_l", None)
    base.pop("hand_r", None)
    rot = dict(base["rot"])
    a = math.sin(2 * math.pi * phase)
    rot["RightUpperArm"] = (75 + 25 * a, 30, 0)
    rot["LeftUpperArm"] = (75 - 25 * a, -30, 0)
    rot["RightLowerArm"] = (70 - 20 * a, 0, 0)
    rot["LeftLowerArm"] = (70 + 20 * a, 0, 0)
    # Knee drawn up sideways on the side whose arm is back.
    rot["RightUpperLeg"] = (30 * max(0, -a), 0, -35 * max(0, -a))
    rot["LeftUpperLeg"] = (30 * max(0, a), 0, 35 * max(0, a))
    rot["RightLowerLeg"] = (-60 * max(0, -a), 0, 0)
    rot["LeftLowerLeg"] = (-60 * max(0, a), 0, 0)
    rot["Spine"] = (12, 0, 6 * a)
    rot["Hips"] = (-88, 0, -4 * a)
    base["rot"] = rot
    base["hips"] = Vector((0, -0.62, -0.83))
    base["weapon"] = hold(poses.kind, "slung") if poses.kind != "none" else None
    return base


def clip_library(kind):
    kind = kind or "none"
    P = Poses(kind)
    lib = {}

    def add(name, length, fn, loop=False, speed=0.0):
        lib[name] = dict(length=length, pose=fn, loop=loop, speed=speed)

    # Idle: breathing.
    def idle(t):
        p = P.stand("ready")
        b = math.sin(2 * math.pi * t / 2.4)
        p["rot"] = {**p["rot"], "Chest": (-1.5 * b, 0, 0), "Head": (2 * b, 0, 3 * math.sin(2 * math.pi * t / 4.8))}
        return single(p)
    add("idle", 2.4, idle, loop=True)

    walk_hold = "ready"
    add("walk", 1.10, lambda t: single(gait(t, 1.10, WALK_SPEED, 0.62, 0.09, 0.03, 4, walk_hold, P, 14)),
        loop=True, speed=WALK_SPEED)
    add("run", 0.72, lambda t: single(gait(t, 0.72, RUN_SPEED, 0.38, 0.20, 0.06, 12, "port", P, 30)),
        loop=True, speed=RUN_SPEED)

    def kneel_idle(t):
        p = P.kneel("ready")
        b = math.sin(2 * math.pi * t / 2.4)
        p["rot"] = {**p["rot"], "Chest": (-4 - 1.5 * b, 0, 0)}
        return single(p)
    add("kneel", 2.4, kneel_idle, loop=True)

    def prone_idle(t):
        p = P.prone("aim")
        b = math.sin(2 * math.pi * t / 3.0)
        p["rot"] = {**p["rot"], "Head": (10 + 3 * b, 0, 6 * math.sin(2 * math.pi * t / 6.0))}
        return single(p)
    add("prone", 3.0, prone_idle, loop=True)

    add("crawl", 1.6, lambda t: single(crawl(t, 1.6, P)), loop=True, speed=CRAWL_SPEED)

    # Aim and fire in three stances. Fire = one shot with recoil; loop it for bursts.
    for stance, fn in (("stand", P.aim_stand), ("kneel", P.aim_kneel),
                       (("prone"), lambda: P.prone("aim"))):
        aim_pose = fn()
        add(f"aim_{stance}", 1.0, keyed([(0.0, aim_pose), (1.0, aim_pose)]), loop=True)
        kicked = dict(aim_pose)
        kicked["kick"] = 1.0 if stance != "prone" else 0.6
        add(f"fire_{stance}", 0.30, keyed([(0.0, aim_pose), (0.04, kicked), (0.30, aim_pose)]),
            loop=True)

    # Reload: weapon tilted, left hand to the pouch and back.
    s = P.stand("ready")
    r = P.stand("reload")
    pouch = ("anchor", "Hips", (-0.09, 0.16, 0.99))
    r_pouch = dict(r, hand_l=(pouch, 1.0))
    r_well = dict(r, hand_l=(("weapon", "grip_l"), 1.0))
    add("reload", 2.2, keyed([(0.0, s), (0.35, r), (0.75, r_pouch), (1.05, r_pouch),
                              (1.5, r_well), (1.85, r), (2.2, s)]))

    # Grenade throw (right arm), weapon in the left hand.
    t0 = P.stand("left_low")
    wind = dict(t0)
    wind["rot"] = {"Spine": (4, 0, -20), "Chest": (6, 0, -15), "Head": (0, 0, 20),
                   "RightUpperArm": (-40, -40, 0), "RightLowerArm": (100, 0, 0)}
    wind["foot_l"] = (foot("L", y=0.28, x=-0.05), 1.0)
    wind["foot_r"] = (foot("R", y=-0.15), 1.0)
    release = dict(wind)
    release["rot"] = {"Spine": (-12, 0, 18), "Chest": (-8, 0, 12),
                      "RightUpperArm": (150, -10, 0), "RightLowerArm": (15, 0, 0)}
    follow = dict(release)
    follow["rot"] = {"Spine": (-18, 0, 22), "Chest": (-10, 0, 14),
                     "RightUpperArm": (60, 30, 0), "RightLowerArm": (30, 0, 0)}
    add("throw_grenade", 1.6, keyed([(0.0, s), (0.3, t0), (0.75, wind), (0.95, release),
                                     (1.15, follow), (1.6, s)]))

    # Hit: flinch back and recover.
    hit = P.stand("ready")
    hit["rot"] = {"Spine": (12, 0, -6), "Chest": (8, 0, 0), "Neck": (12, 0, 0), "Head": (10, 0, 8)}
    hit["hips"] = Vector((0, -0.05, -0.04))
    add("hit", 0.6, keyed([(0.0, s), (0.1, hit), (0.6, s)]))

    # Death A: falls backwards.
    buckle = P.stand("ready")
    buckle["rot"] = {"Spine": (14, 0, 0), "Neck": (20, 0, 0), "Head": (15, 0, 0),
                     **both("UpperLeg", (30, 0, 0)), **both("LowerLeg", (-50, 0, 0))}
    buckle["hips"] = Vector((0, -0.10, -0.25))
    for k in ("foot_l", "foot_r"):
        buckle.pop(k)
    buckle["hand_l"] = (("weapon", "grip_l"), 0.4)
    buckle["hand_r"] = (("weapon", "grip_r"), 0.4)
    down = P.lying_back()
    if kind != "none":
        down["weapon"] = hold(kind, "dropped")
    add("death_a", 1.5, keyed([(0.0, s), (0.35, buckle), (1.0, down), (1.5, down)]))

    # Death B: knees give way, falls forward on the face.
    knees = P.kneel("ready")
    knees["rot"] = {"Spine": (-25, 0, 0), "Neck": (-20, 0, 0), "Head": (-15, 0, 0),
                    **both("UpperArm", (10, 30, 0))}
    knees["hips"] = Vector((0, -0.02, -0.50))
    knees["foot_l"] = (foot("L", y=-0.42, z=0.07, pitch=-62), 1.0)
    knees["hand_l"] = (("weapon", "grip_l"), 0.0)
    knees["hand_r"] = (("weapon", "grip_r"), 0.0)
    face = P.prone("slung")
    face["rot"] = {**face["rot"], "Neck": (0, 0, 0), "Head": (0, 0, -40),
                   "RightUpperArm": (20, 60, 0), "LeftUpperArm": (120, -40, 0),
                   **both("LowerArm", (20, 0, 0))}
    face["hips"] = Vector((0, -0.20, -0.82))
    for k in ("hand_l", "hand_r"):
        face[k] = (None, 0.0)
    if kind != "none":
        face["weapon"] = hold(kind, "dropped")
    add("death_b", 1.7, keyed([(0.0, s), (0.45, knees), (1.2, face), (1.7, face)]))

    # Transitions.
    st, kn = P.stand("ready"), P.kneel("ready")
    pr = P.prone("aim")
    mid = P.kneel("ready")
    mid["rot"] = {"Spine": (-45, 0, 0), "Chest": (-10, 0, 0), "Neck": (20, 0, 0)}
    mid["hips"] = Vector((0, -0.25, -0.62))
    add("stand_to_kneel", 0.6, keyed([(0.0, st), (0.6, kn)]))
    add("kneel_to_stand", 0.6, keyed([(0.0, kn), (0.6, st)]))
    add("stand_to_prone", 1.2, keyed([(0.0, st), (0.45, kn), (0.8, mid), (1.2, pr)]))
    add("prone_to_stand", 1.2, keyed([(0.0, pr), (0.4, mid), (0.75, kn), (1.2, st)]))

    # MG: carry on the shoulder (walking) and set up (to prone, ready to fire).
    if kind == "mg":
        carry = {"weapon": hold(kind, "mg_shoulder"), "hand_r": (("weapon", "center_fwd"), 1.0),
                 "hand_l": (None, 0.0)}
        add("mg_carry", 1.2, lambda t: single(gait(t, 1.2, CARRY_SPEED, 0.62, 0.08, 0.03, 6,
                                                   "ready", P, 16, carry=carry)),
            loop=True, speed=CARRY_SPEED)
    else:
        add("mg_carry", 1.2, lambda t: single(gait(t, 1.2, CARRY_SPEED, 0.62, 0.08, 0.03, 6,
                                                   "ready", P, 16)),
            loop=True, speed=CARRY_SPEED)
    lowered = P.kneel("ready")
    lowered["rot"] = {"Spine": (-35, 0, 0), "Chest": (-10, 0, 0), "Neck": (15, 0, 0)}
    lowered["hips"] = Vector((0, -0.15, -0.55))
    add("mg_setup", 2.2, keyed([(0.0, st), (0.5, kn), (1.1, lowered), (1.7, pr), (2.2, pr)]))

    # Mortar: kneeling behind the deployed mortar, load and duck.
    base = P.kneel("deployed") if kind == "mortar" else P.kneel("slung")
    if kind != "mortar" and kind != "none":
        base["weapon"] = hold(kind, "slung")
    tube_hand = ("weapon", "fore") if kind == "mortar" else ("anchor", "Root", (-0.05, 0.55, 0.45))
    base["hand_l"] = (tube_hand, 1.0)
    base["hand_r"] = (("anchor", "Root", (0.30, 0.25, 0.12)), 1.0)
    pick = dict(base)
    pick["rot"] = {"Spine": (-30, 0, -15), "Chest": (-10, 0, -5), "Neck": (-10, 0, 0)}
    lift = dict(base)
    lift["rot"] = {"Spine": (-15, 0, 0), "Neck": (-5, 0, 0)}
    muzzle = ("weapon", "muzzle_up") if kind == "mortar" else ("anchor", "Root", (0.05, 0.6, 0.75))
    lift["hand_r"] = (muzzle, 1.0)
    duck = dict(base)
    duck["rot"] = {"Spine": (-28, 0, 10), "Chest": (-8, 0, 0), "Neck": (-25, 0, 0), "Head": (-10, 0, -30)}
    duck["hand_r"] = (("anchor", "Root", (0.22, 0.10, 0.75)), 1.0)
    add("mortar_operate", 2.4, keyed([(0.0, pick), (0.5, pick), (1.1, lift), (1.3, lift),
                                      (1.6, duck), (2.1, base), (2.4, pick)]), loop=True)

    # Medic: kneeling, both hands working at a casualty in front.
    med = P.kneel("slung")
    if kind != "none":
        med["weapon"] = hold(kind, "slung")
    med["rot"] = {"Spine": (-28, 0, 0), "Chest": (-8, 0, 0), "Neck": (-15, 0, 0), "Head": (-15, 0, 0)}
    a = dict(med, hand_l=(("anchor", "Root", (-0.10, 0.55, 0.22)), 1.0),
             hand_r=(("anchor", "Root", (0.14, 0.50, 0.25)), 1.0))
    b = dict(med, hand_l=(("anchor", "Root", (-0.04, 0.50, 0.28)), 1.0),
             hand_r=(("anchor", "Root", (0.06, 0.58, 0.20)), 1.0))
    add("medic_treat", 2.0, keyed([(0.0, a), (0.5, b), (1.0, a), (1.5, b), (2.0, a)]), loop=True)
    return lib


CLIP_NAMES = list(clip_library("rifle").keys())


# ---------------------------------------------------------------------------------------
# Baking
# ---------------------------------------------------------------------------------------


class Baker:
    def __init__(self, arm, weapon):
        self.arm = arm
        self.weapon = weapon  # weapons.Weapon or None
        self.pb = arm.pose.bones
        self.rest = {b.name: b.matrix_local.copy() for b in arm.data.bones}
        self.rest_rot = {n: m.to_3x3().normalized() for n, m in self.rest.items()}
        self.export_bones = skeleton.DEFORM_BONES + [skeleton.WEAPON_BONE]

    # -- spec evaluation --

    def delta(self, bone):
        """Transform that maps rest-pose points of `bone` to its current pose."""
        return self.pb[bone].matrix @ self.rest[bone].inverted()

    def weapon_point(self, name):
        p = self.weapon.points
        if name == "center_fwd":
            return p["center"] + Vector((0, 0.18, -0.04))
        if name == "muzzle_up":
            return p["muzzle"] + Vector((0, 0.06, 0.0))
        return p[name]

    def weapon_matrix(self, spec, kick=0.0):
        d = self.delta(spec["anchor"])
        at = d @ Vector(spec["at"])
        r = Euler([math.radians(a) for a in spec["rot"]], "XYZ").to_matrix()
        if spec["frame"] == "anchor":
            r = d.to_3x3().normalized() @ r
        point = self.weapon_point(spec["point"])
        if kick:
            r = r @ Euler((math.radians(5 * kick), 0, 0)).to_matrix()
            at = at + r @ Vector((0, -0.035 * kick, 0))
        m = r.to_4x4()
        m.translation = at - r @ point
        return m

    def target_pos(self, spec, weapon_m):
        if spec is None:
            return None
        if spec[0] == "weapon":
            if self.weapon is None or weapon_m is None:
                return None
            return weapon_m @ self.weapon_point(spec[1])
        _, anchor, at = spec
        return self.delta(anchor) @ Vector(at)

    @staticmethod
    def blend_vectors(items):
        total = sum(w for v, w in items if v is not None and w > 0)
        if total <= 1e-6:
            return None
        acc = Vector((0, 0, 0))
        for v, w in items:
            if v is not None and w > 0:
                acc += v * (w / total)
        return acc

    # -- one frame --

    def reset(self):
        for pb in self.pb:
            pb.matrix_basis = Matrix.Identity(4)

    def apply_fk(self, pose):
        rot = dict(ARMS_RELAXED)
        rot.update(pose["rot"])
        for bone, angles in rot.items():
            rr = self.rest_rot[bone]
            r = Euler([math.radians(a) for a in angles], "XYZ").to_matrix()
            basis = rr.inverted() @ r @ rr
            self.pb[bone].rotation_quaternion = basis.to_quaternion()
        hips = self.rest_rot["Hips"].inverted() @ pose["hips"]
        self.pb["Hips"].location = hips

    def place_helper(self, name, pos, pitch=0.0):
        rr = self.rest_rot[name]
        rot = Euler((math.radians(pitch), 0, 0)).to_matrix() @ rr
        m = rot.to_4x4()
        m.translation = pos
        self.pb[name].matrix = m

    def solve(self, pose):
        lists = pose["lists"]
        # Weapon.
        weapon_m = None
        if self.weapon is not None:
            mats = []
            for spec, w in lists["weapon"]:
                if spec is not None and w > 0:
                    mats.append((self.weapon_matrix(spec, pose["kick"]), w))
            if mats:
                weapon_m = self._blend_matrices(mats)
                self.pb[skeleton.WEAPON_BONE].matrix = weapon_m
        # Hands.
        for side in ("L", "R"):
            items, infl = [], 0.0
            for entry, w in lists[f"hand_{side.lower()}"]:
                if entry is None:
                    continue
                spec, i = entry
                pos = self.target_pos(spec, weapon_m)
                if pos is None:
                    continue
                items.append((pos, w))
                infl += i * w
            pos = self.blend_vectors(items)
            c = self.pb[ARM_CHAINS[side]].constraints["IK"]
            c.influence = min(1.0, infl) if pos is not None else 0.0
            if pos is not None:
                self.place_helper(f"IK_Hand_{side}", pos)
            sx = -1 if side == "L" else 1
            pole = pose.get(f"pole_{side.lower()}") or (sx * 0.55, -0.45, 1.0)
            self.place_helper(f"Pole_Arm_{side}", self.delta("UpperChest") @ Vector(pole))
        # Feet.
        for side in ("L", "R"):
            items, pitches, infl = [], [], 0.0
            for entry, w in lists[f"foot_{side.lower()}"]:
                if entry is None:
                    continue
                (pos, pitch), i = entry
                items.append((pos, w))
                pitches.append((pitch, w))
                infl += i * w
            pos = self.blend_vectors(items)
            c = self.pb[LEG_CHAINS[side]].constraints["IK"]
            fr = self.pb["LeftFoot" if side == "L" else "RightFoot"].constraints["FootRot"]
            if pos is not None:
                total = sum(w for _, w in pitches)
                pitch = sum(p * w for p, w in pitches) / total
                self.place_helper(f"IK_Foot_{side}", pos, pitch)
                c.influence = fr.influence = min(1.0, infl)
            else:
                c.influence = fr.influence = 0.0
            sx = -1 if side == "L" else 1
            self.place_helper(f"Pole_Leg_{side}", self.delta("Hips") @ Vector((sx * 0.12, 0.9, 0.55)))

    @staticmethod
    def _blend_matrices(mats):
        total = sum(w for _, w in mats)
        loc = Vector((0, 0, 0))
        quat = None
        acc_w = 0.0
        scale = Vector((0, 0, 0))
        for m, w in mats:
            l, q, s = m.decompose()
            loc += l * (w / total)
            scale += s * (w / total)
            if quat is None:
                quat, acc_w = q, w
            else:
                acc_w += w
                quat = quat.slerp(q, w / acc_w)
        return Matrix.LocRotScale(loc, quat, scale)

    def sample(self, pose):
        self.reset()
        self.apply_fk(pose)
        bpy.context.view_layer.update()
        self.solve(pose)
        bpy.context.view_layer.update()
        out = {}
        for name in self.export_bones:
            pb = self.pb[name]
            m = self.arm.convert_space(pose_bone=pb, matrix=pb.matrix, from_space="POSE",
                                       to_space="LOCAL")
            out[name] = m.decompose()
        return out

    # -- clips --

    def bake_clip(self, clip):
        frames = int(round(clip["length"] * FPS))
        samples = []
        for f in range(frames + 1):
            t = min(f / FPS, clip["length"])
            if clip["loop"] and f == frames:
                t = 0.0  # identical first and last key: seamless loop
            samples.append(self.sample(clip["pose"](t)))
        return samples


def bake_all(arm, weapon, kind):
    setup_ik(arm)
    lib = clip_library(kind)
    baker = Baker(arm, weapon)
    baked = {name: baker.bake_clip(clip) for name, clip in lib.items()}
    # Remove constraints, then write plain keyframes.
    for pb in arm.pose.bones:
        for c in list(pb.constraints):
            pb.constraints.remove(c)
    baker.reset()
    skeleton.remove_helper_bones(arm)
    actions = []
    arm.animation_data_create()
    for name, samples in baked.items():
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        arm.animation_data.action = action
        prev = {}
        for f, frame in enumerate(samples):
            for bone, (loc, quat, scale) in frame.items():
                pb = arm.pose.bones[bone]
                if bone in prev and prev[bone].dot(quat) < 0:
                    quat = -quat
                prev[bone] = quat
                pb.rotation_quaternion = quat
                pb.keyframe_insert("rotation_quaternion", frame=f, group=bone)
                if bone in ("Hips", skeleton.WEAPON_BONE):
                    pb.location = loc
                    pb.keyframe_insert("location", frame=f, group=bone)
                if bone == skeleton.WEAPON_BONE:
                    pb.scale = scale
                    pb.keyframe_insert("scale", frame=f, group=bone)
        actions.append(action)
        track = arm.animation_data.nla_tracks.new()
        track.name = name
        strip = track.strips.new(name, 0, action)
        strip.name = name
        track.mute = True
        arm.animation_data.action = None
    baker.reset()
    return {name: dict(loop=c["loop"], speed=c["speed"], length=c["length"])
            for name, c in lib.items()}
