extends SkeletonModifier3D
## Turning "head first" (prompt 048): the head, the neck and the chest look
## towards where the soldier is turning before his body (the member node,
## turned by `infantry_view.gd`) follows. Applied on top of the animation;
## the weapon bone turns with the chest so the hands stay on the weapon.
## Presentation only.

## Largest lead of the head (radians) and the share of it each bone takes:
## the chest turns first a little, the neck more, the head the rest.
const MAX_LEAD := deg_to_rad(75.0)
const CHEST_SHARE := 0.3
const NECK_SHARE := 0.3
## How fast the lead follows its target (1/s).
const RATE := 10.0

## Wanted lead in radians (target yaw minus body yaw, positive = left).
var target := 0.0
## Current lead.
var lead := 0.0

var _chest := -1
var _neck := -1
var _head := -1
var _weapon := -1


func _ready() -> void:
	_find_bones()


func _skeleton_changed(_old_skeleton: Skeleton3D, _new_skeleton: Skeleton3D) -> void:
	_find_bones()


func _find_bones() -> void:
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	_chest = skeleton.find_bone("Chest")
	_neck = skeleton.find_bone("Neck")
	_head = skeleton.find_bone("Head")
	_weapon = skeleton.find_bone("Weapon")


func _process_modification_with_delta(delta: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	if _chest < 0 or _neck < 0 or _head < 0:
		_find_bones()
		if _chest < 0 or _neck < 0 or _head < 0:
			return
	var wanted := clampf(target, -MAX_LEAD, MAX_LEAD)
	lead = lerpf(lead, wanted, clampf(delta * RATE, 0.0, 1.0))
	if absf(lead) < 0.002:
		return
	var chest_angle := lead * CHEST_SHARE
	# The weapon hangs on the root: turn it about the chest with the chest.
	if _weapon >= 0:
		var pivot := skeleton.get_bone_global_pose(_chest).origin
		var turn := Transform3D(Basis(Vector3.UP, chest_angle), Vector3.ZERO)
		var shift := Transform3D(Basis.IDENTITY, pivot) * turn * Transform3D(Basis.IDENTITY, -pivot)
		var weapon_global := shift * skeleton.get_bone_global_pose(_weapon)
		var parent := skeleton.get_bone_parent(_weapon)
		var local := weapon_global if parent < 0 else skeleton.get_bone_global_pose(parent).affine_inverse() * weapon_global
		skeleton.set_bone_pose_position(_weapon, local.origin)
		skeleton.set_bone_pose_rotation(_weapon, local.basis.orthonormalized().get_rotation_quaternion())
	_turn(skeleton, _chest, chest_angle)
	_turn(skeleton, _neck, lead * NECK_SHARE)
	_turn(skeleton, _head, lead * (1.0 - CHEST_SHARE - NECK_SHARE))


## Turns bone `b` by `angle` about the skeleton's up axis.
func _turn(skeleton: Skeleton3D, b: int, angle: float) -> void:
	var parent := skeleton.get_bone_parent(b)
	var up := Vector3.UP
	if parent >= 0:
		up = (skeleton.get_bone_global_pose(parent).basis.inverse() * Vector3.UP).normalized()
	var rotation := Quaternion(up, angle) * skeleton.get_bone_pose_rotation(b)
	skeleton.set_bone_pose_rotation(b, rotation)
