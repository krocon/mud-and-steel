extends Node3D
## Ragdolls for fallen and knocked-down soldiers (prompt 048). Presentation
## only (CLAUDE.md rule 2): the simulation decides who dies or is knocked
## down, where he lies and when he gets up; Godot physics only throws the
## visible body there. Nothing is read back into the game.
##
## A ragdoll is built on demand for one soldier: 11 `PhysicalBone3D`
## capsules (hips, chest, head, arms, legs) with cone joints under a
## `PhysicalBoneSimulator3D` on the soldier's skeleton, and a small floor
## box at his ground height. Ragdolls and floors use their own collision
## layer, so they touch nothing else. At most MAX_ACTIVE simulate at once,
## and only within MAX_DISTANCE_M of the camera; `infantry_view.gd` falls
## back to animation clips otherwise.
##
## The dead are frozen after SETTLE_SECONDS: their pose is written into the
## skeleton and the physics nodes are freed. The knocked down are handed
## back to the animation with `release`, which blends the physics pose out
## while the get-up clip starts.

## Simultaneous ragdolls [provisional].
const MAX_ACTIVE := 8
## Prompt 194: soldiers thrown far by a big blast may use ragdolls beyond
## MAX_ACTIVE, up to this many in all; the rest fly as a clip on the arc of
## the simulation (`infantry_view.gd`). [provisional]
const MAX_BURST := 16
## Camera distance up to which ragdolls are used [provisional].
const MAX_DISTANCE_M := 60.0
const SETTLE_SECONDS := 1.8
## Collision layer 20, used by nothing else.
const LAYER := 1 << 19
const FLOOR_SIZE_M := 8.0
const CONE_SWING_DEG := 50.0
const CONE_TWIST_DEG := 30.0

## Simulated bones: name, child bone that ends the capsule, radius (m),
## mass (kg; a 75 kg soldier with equipment).
const BODIES: Array = [
	["Hips", "Spine", 0.15, 16.0],
	["Chest", "Neck", 0.17, 20.0],
	["Head", "", 0.11, 5.0],
	["LeftUpperArm", "LeftLowerArm", 0.055, 3.0],
	["LeftLowerArm", "LeftHand", 0.045, 2.5],
	["RightUpperArm", "RightLowerArm", 0.055, 3.0],
	["RightLowerArm", "RightHand", 0.045, 2.5],
	["LeftUpperLeg", "LeftLowerLeg", 0.08, 8.0],
	["LeftLowerLeg", "LeftFoot", 0.06, 5.0],
	["RightUpperLeg", "RightLowerLeg", 0.08, 8.0],
	["RightLowerLeg", "RightFoot", 0.06, 5.0],
]

## Ragdolls by member node instance id: {member, skeleton, simulator,
## bodies (bone index -> PhysicalBone3D), floor, age, mode ("dead",
## "down", "release"), blend}.
var _active: Dictionary[int, Dictionary] = {}
## Ragdolls started in total (statistics, self-test).
var started := 0


func active_count() -> int:
	return _active.size()


func is_active(member: Node3D) -> bool:
	return _active.has(member.get_instance_id())


## Whether a new ragdoll may start at `at`; `burst` for a far throw by a
## big blast (own, higher limit).
func can_start(at: Vector3, camera_position: Vector3, burst := false) -> bool:
	var limit := MAX_BURST if burst else MAX_ACTIVE
	return _active.size() < limit and at.distance_to(camera_position) <= MAX_DISTANCE_M


## Starts a ragdoll on `member` (a soldier with a Skeleton3D): every body
## gets `velocity` (m/s) and `spin` (rad/s about the hips); the floor lies at
## `ground_y`. `mode` "dead" freezes by itself, "down" waits for `release`.
## Prompt 194, a throw by a blast: `landing` is where the simulation puts
## him (the floor reaches from here to there; `Vector3.INF`: no throw) and
## `gravity_scale` makes the bodies fall in game time (time factor 2: 4).
func start(member: Node3D, velocity: Vector3, spin: Vector3, ground_y: float, mode: String,
		landing := Vector3.INF, gravity_scale := 1.0) -> bool:
	var key := member.get_instance_id()
	var limit := MAX_ACTIVE if landing == Vector3.INF else MAX_BURST
	if _active.has(key) or _active.size() >= limit:
		return false
	var skeleton := SoldierLook.find_skeleton(member)
	if skeleton == null:
		return false
	var simulator := PhysicalBoneSimulator3D.new()
	simulator.name = "Ragdoll"
	skeleton.add_child(simulator)
	var bodies: Dictionary[int, PhysicalBone3D] = {}
	for spec: Array in BODIES:
		var bone := skeleton.find_bone(spec[0])
		if bone < 0:
			continue
		var body := _make_body(skeleton, bone, spec)
		body.gravity_scale = gravity_scale
		simulator.add_child(body)
		body.global_transform = skeleton.global_transform * skeleton.get_bone_global_pose(bone)
		bodies[bone] = body
	var floor_at := member.global_position
	var floor_size := FLOOR_SIZE_M
	if landing != Vector3.INF:
		floor_at = (floor_at + landing) * 0.5
		floor_size = maxf(FLOOR_SIZE_M, Vector2(landing.x - member.global_position.x,
				landing.z - member.global_position.z).length() + FLOOR_SIZE_M)
	var floor := _make_floor(floor_at, ground_y, floor_size)
	add_child(floor)
	simulator.physical_bones_start_simulation()
	var center := skeleton.global_transform * skeleton.get_bone_global_pose(skeleton.find_bone("Hips")).origin
	for body: PhysicalBone3D in bodies.values():
		body.linear_velocity = velocity + spin.cross(body.global_position - center)
		body.angular_velocity = spin
	_active[key] = {
		"member": member, "skeleton": skeleton, "simulator": simulator, "bodies": bodies,
		"floor": floor, "age": 0.0, "mode": mode, "blend": 0.0, "blend_time": 0.0,
	}
	started += 1
	return true


## Blends a "down" ragdoll back into the animation over `seconds` (the
## get-up clip plays meanwhile), then frees it.
func release(member: Node3D, seconds: float) -> void:
	var e: Dictionary = _active.get(member.get_instance_id(), {})
	if e.is_empty():
		return
	e["mode"] = "release"
	e["blend_time"] = maxf(seconds, 0.01)
	e["blend"] = 0.0


## How the body lies: {"back": bool, "head": Vector3 (horizontal, from the
## hips towards the head), "hips": Vector3 (world)}. Empty if no ragdoll.
func lying(member: Node3D) -> Dictionary:
	var e: Dictionary = _active.get(member.get_instance_id(), {})
	if e.is_empty():
		return {}
	var skeleton: Skeleton3D = e["skeleton"]
	var hips := skeleton.find_bone("Hips")
	var body: PhysicalBone3D = (e["bodies"] as Dictionary).get(hips)
	if body == null:
		return {}
	# Hips axes at rest in skeleton space: the soldier faces -Z, up is +Y.
	var rest := skeleton.get_bone_global_rest(hips).basis.orthonormalized()
	var front_local := rest.inverse() * Vector3(0, 0, -1)
	var up_local := rest.inverse() * Vector3.UP
	var basis := body.global_basis.orthonormalized()
	var front := basis * front_local
	var head := basis * up_local
	head.y = 0.0
	return {"back": front.y > 0.0, "head": head.normalized() if head.length() > 0.01 else Vector3.FORWARD,
		"hips": body.global_position}


## Frees every ragdoll at once (scene change).
func clear() -> void:
	for e: Dictionary in _active.values():
		_free(e)
	_active.clear()


func _physics_process(delta: float) -> void:
	for key: int in _active.keys():
		var e: Dictionary = _active[key]
		var member: Node3D = e["member"]
		if not is_instance_valid(member) or not member.is_inside_tree():
			_free(e)
			_active.erase(key)
			continue
		e["age"] = float(e["age"]) + delta
		match e["mode"]:
			"dead":
				if float(e["age"]) >= SETTLE_SECONDS:
					freeze(member)
			"release":
				e["blend"] = float(e["blend"]) + delta
				var t := clampf(float(e["blend"]) / float(e["blend_time"]), 0.0, 1.0)
				(e["simulator"] as PhysicalBoneSimulator3D).influence = 1.0 - t
				if t >= 1.0:
					_free(e)
					_active.erase(key)


## Writes the ragdoll's pose into the skeleton (it stays there; the caller
## stops the animation) and frees the physics nodes.
func freeze(member: Node3D) -> void:
	var key := member.get_instance_id()
	var e: Dictionary = _active.get(key, {})
	if e.is_empty():
		return
	var skeleton: Skeleton3D = e["skeleton"]
	var bodies: Dictionary = e["bodies"]
	var inv := skeleton.global_transform.affine_inverse()
	var n := skeleton.get_bone_count()
	var globals: Array[Transform3D] = []
	globals.resize(n)
	var done: Array[bool] = []
	done.resize(n)
	for b in n:
		_global_of(skeleton, b, bodies, inv, globals, done)
	_free(e)
	_active.erase(key)
	member.set_meta(&"frozen_pose", true)
	for b in n:
		var parent := skeleton.get_bone_parent(b)
		var local := globals[b] if parent < 0 else globals[parent].affine_inverse() * globals[b]
		skeleton.set_bone_pose_position(b, local.origin)
		skeleton.set_bone_pose_rotation(b, local.basis.orthonormalized().get_rotation_quaternion())


func _global_of(skeleton: Skeleton3D, b: int, bodies: Dictionary, inv: Transform3D,
		globals: Array[Transform3D], done: Array[bool]) -> Transform3D:
	if done[b]:
		return globals[b]
	var body: PhysicalBone3D = bodies.get(b)
	var parent := skeleton.get_bone_parent(b)
	var g: Transform3D
	if body != null:
		g = inv * body.global_transform
		g.basis = g.basis.orthonormalized()
	elif parent >= 0:
		g = _global_of(skeleton, parent, bodies, inv, globals, done) * skeleton.get_bone_pose(b)
	else:
		g = skeleton.get_bone_pose(b)
	globals[b] = g
	done[b] = true
	return g


func _free(e: Dictionary) -> void:
	var simulator: PhysicalBoneSimulator3D = e["simulator"]
	if is_instance_valid(simulator):
		simulator.physical_bones_stop_simulation()
		simulator.queue_free()
	var floor: Node = e["floor"]
	if is_instance_valid(floor):
		floor.queue_free()


func _make_body(skeleton: Skeleton3D, bone: int, spec: Array) -> PhysicalBone3D:
	var body := PhysicalBone3D.new()
	body.name = "Ragdoll_" + str(spec[0])
	body.set("bone_name", spec[0])
	body.collision_layer = LAYER
	body.collision_mask = LAYER
	body.mass = float(spec[3])
	body.friction = 0.9
	body.bounce = 0.05
	body.linear_damp = 0.2
	body.angular_damp = 2.0
	body.joint_type = PhysicalBone3D.JOINT_TYPE_CONE
	body.set("joint_constraints/swing_span", CONE_SWING_DEG)
	body.set("joint_constraints/twist_span", CONE_TWIST_DEG)
	# Capsule from the bone's head to its child (along the bone's +Y).
	var length := 0.22
	var child := skeleton.find_bone(spec[1]) if str(spec[1]) != "" else -1
	if child >= 0:
		var a := skeleton.get_bone_global_rest(bone).origin
		var c := skeleton.get_bone_global_rest(child).origin
		length = maxf(a.distance_to(c), 0.1)
	var radius: float = spec[2]
	var shape := CapsuleShape3D.new()
	shape.radius = radius
	shape.height = maxf(length, radius * 2.0 + 0.01)
	var collision := CollisionShape3D.new()
	collision.shape = shape
	collision.position = Vector3(0.0, length * 0.5, 0.0)
	body.add_child(collision)
	return body


func _make_floor(at: Vector3, ground_y: float, size: float) -> StaticBody3D:
	var floor := StaticBody3D.new()
	floor.name = "RagdollFloor"
	floor.collision_layer = LAYER
	floor.collision_mask = LAYER
	var shape := BoxShape3D.new()
	shape.size = Vector3(size, 1.0, size)
	var collision := CollisionShape3D.new()
	collision.shape = shape
	floor.add_child(collision)
	floor.position = Vector3(at.x, ground_y - 0.5, at.z)
	return floor
