extends Node3D
## Wounds and buddy aid of prompt 049, used by `infantry_view.gd`: the clips
## of wounded men and their helpers, the stretcher between two bearers and
## the bandage of treated men. Presentation only: the simulation decides who
## is wounded, who drags, carries or treats whom and where everybody is
## (`SimHost.soldier_wounds`, `soldier_aid_kinds`, `soldier_aid_targets`).
##
## | Simulation | Clip |
## |---|---|
## | lightly wounded, dressing the wound | `bandage` (walking: `limp`) |
## | heavily wounded, down | `wounded`; dragged: `dragged` (head first) |
## | on a stretcher | `wounded`, lifted to the poles |
## | treated, limping | `limp` while walking |
## | dragging a comrade | `drag`, facing him, walking backwards |
## | pulling up a knocked-down comrade | `help_up`, facing him |
## | medic treating | `medic_treat`, facing the casualty |
## | stretcher bearer | `stretcher`; the front one carries the stretcher |
##
## A man who dies lying keeps his pose. Treated men wear a white bandage on
## the left thigh (not on the arm: that is where medics wear their plain
## white armband; no Red Cross, docs/SOURCES.md).

## Codes of `SimHost.soldier_wounds`.
enum Wound { FIT, BANDAGING, DOWN, DOWN_STABLE, LIMPING, BANDAGED, GONE }
## Codes of `SimHost.soldier_aid_kinds`.
enum Aid { NONE, GOING, DRAG, DRAGGED, HELP_UP, TREAT, STRETCHER_FRONT, STRETCHER_BACK, ON_STRETCHER }

## Height of the stretcher poles (m; `STRETCHER_HEIGHT` in animation.py).
const STRETCHER_HEIGHT := 0.72
const STRETCHER_LENGTH := 2.2
const STRETCHER_WIDTH := 0.54
## Helpers farther than this (m) from the comrade they help walk normally.
const NEAR_M := 1.3
const BANDAGE_COLOR := Color(0.92, 0.91, 0.86)

var view: Node3D
## Statistics for the self-test: clips started per kind.
var counts := {"wounded": 0, "bandage": 0, "limp": 0, "drag": 0, "dragged": 0, "help_up": 0,
		"treat": 0, "stretcher": 0, "bandaged": 0}
var _materials: Dictionary[String, StandardMaterial3D] = {}


func setup(infantry_view: Node3D) -> void:
	view = infantry_view


## Picks the clip of a wounded or helping soldier. Returns true when the
## clip is decided here (the view's normal choice is skipped).
func animate(s: Object, hp: int, knocked: int, wound: int, aid: int, target: Vector3,
		position: Vector3, moving: bool, speed: float) -> bool:
	var player: AnimationPlayer = s.player
	var member: Node3D = s.node
	if player == null:
		return false
	if wound >= Wound.DOWN_STABLE and wound <= Wound.BANDAGED or wound == Wound.DOWN and aid == Aid.ON_STRETCHER:
		_bandage(s)
	_stretcher(s, aid == Aid.STRETCHER_FRONT, target)
	if hp <= 0:
		if s.care_lying:
			# Died where he lay: he keeps the pose.
			s.dead = true
			player.pause()
			return true
		_release(s)
		return false
	if knocked > 0:
		_release(s)
		return false
	var clip := ""
	var scale: float = view.TIME_FACTOR
	var face := INF
	var lying := false
	match aid:
		Aid.DRAGGED:
			clip = "dragged"
			lying = true
			if moving:
				# Pulled head first: the head is behind the model's root.
				var step: Vector3 = position - (s.care_last as Vector3)
				face = atan2(step.x, step.z)
		Aid.ON_STRETCHER:
			clip = "wounded"
			lying = true
			member.position.y = position.y + STRETCHER_HEIGHT
			s.last_pos = member.position
		Aid.DRAG:
			clip = "drag"
			face = _toward(position, target)
			scale = speed / maxf(_authored("drag"), 0.01) if moving else 0.0
		Aid.HELP_UP, Aid.TREAT:
			if position.distance_squared_to(target) <= NEAR_M * NEAR_M:
				clip = "help_up" if aid == Aid.HELP_UP else "medic_treat"
				face = _toward(position, target)
		Aid.STRETCHER_FRONT, Aid.STRETCHER_BACK:
			clip = "stretcher"
			scale = speed / maxf(_authored("stretcher"), 0.01) if moving else 0.0
	if clip.is_empty():
		match wound:
			Wound.DOWN, Wound.DOWN_STABLE:
				clip = "wounded"
				lying = true
			Wound.BANDAGING:
				# He fights on (prompt 049): dressing the wound only in a lull.
				var since := Time.get_ticks_msec() - int(s.fired)
				if moving:
					clip = "limp"
				elif since > view.AIM_MS:
					clip = "bandage"
			Wound.LIMPING:
				if moving:
					clip = "limp"
	s.care_last = position
	if clip.is_empty() or (clip != s.clip and not player.has_animation(clip)):
		_release(s)
		return false
	if clip == "limp":
		scale = speed / maxf(_authored("limp"), 0.01)
	s.care = true
	s.care_lying = lying
	if face != INF:
		s.care_yaw = true
		s.yaw = face
		member.rotation.y = face
		s.last_yaw = face
	elif lying:
		# A man on the ground does not turn with the squad.
		s.care_yaw = true
	else:
		s.care_yaw = false
	if absf(scale - s.last_speed_scale) > 1e-4:
		player.speed_scale = scale
		s.last_speed_scale = scale
	if clip != s.clip:
		var key := "treat" if clip == "medic_treat" else clip
		if counts.has(key):
			counts[key] += 1
		s.clip = clip
		player.play(clip, view.BLEND_SECONDS)
	return true


## Back to the view's own clip choice.
func _release(s: Object) -> void:
	if not s.care:
		return
	s.care = false
	s.care_yaw = false
	s.care_lying = false
	s.clip = ""


func _authored(clip: String) -> float:
	return view._authored_speed(clip)


## Yaw that turns a model (facing -Z) from `from` towards `to`.
func _toward(from: Vector3, to: Vector3) -> float:
	var d := to - from
	if Vector2(d.x, d.z).length_squared() < 0.0025:
		return INF
	return atan2(-d.x, -d.z)


## The bandage on the left thigh, once per soldier (hidden with the gear far
## from the camera).
func _bandage(s: Object) -> void:
	if s.bandage_done:
		return
	s.bandage_done = true
	var skeleton: Skeleton3D = s.skeleton
	if skeleton == null:
		skeleton = SoldierLook.find_skeleton(s.node)
		s.skeleton = skeleton
	if skeleton == null or skeleton.find_bone("LeftUpperLeg") < 0:
		return
	var attach := BoneAttachment3D.new()
	attach.name = "Bandage"
	attach.bone_name = "LeftUpperLeg"
	skeleton.add_child(attach)
	var band := MeshInstance3D.new()
	var mesh := CylinderMesh.new()
	mesh.top_radius = 0.088
	mesh.bottom_radius = 0.085
	mesh.height = 0.09
	mesh.radial_segments = 10
	mesh.rings = 1
	band.mesh = mesh
	band.material_override = _material("bandage", BANDAGE_COLOR)
	band.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Along the thigh bone (local +Y), about mid-thigh.
	band.position = Vector3(0.0, 0.2, 0.0)
	attach.add_child(band)
	s.bandage = band
	s.details.append(band)
	counts["bandaged"] += 1


## The stretcher of a front bearer: two poles and the canvas between him and
## the casualty's position, at pole height.
func _stretcher(s: Object, show: bool, patient: Vector3) -> void:
	var node: Node3D = s.stretcher
	if not show:
		if node != null and node.visible:
			node.visible = false
		return
	if node == null:
		node = _make_stretcher()
		add_child(node)
		s.stretcher = node
		counts["stretcher"] += 1
	var front: Vector3 = s.position
	var along := patient - front
	along.y = 0.0
	if along.length_squared() < 0.01:
		node.visible = false
		return
	node.visible = true
	# The stretcher's middle lies under the casualty.
	node.position = Vector3(patient.x, (front.y + patient.y) * 0.5 + STRETCHER_HEIGHT - 0.05, patient.z)
	node.rotation = Vector3(0.0, atan2(along.x, along.z), 0.0)


func _make_stretcher() -> Node3D:
	var root := Node3D.new()
	root.name = "Stretcher"
	var wood := _material("wood", Color(0.42, 0.31, 0.2))
	var canvas := _material("canvas", Color(0.45, 0.43, 0.33))
	for side in [-1.0, 1.0]:
		var pole := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(0.04, 0.04, STRETCHER_LENGTH)
		pole.mesh = box
		pole.material_override = wood
		pole.position = Vector3(side * STRETCHER_WIDTH * 0.5, 0.0, 0.0)
		root.add_child(pole)
	var cloth := MeshInstance3D.new()
	var sheet := BoxMesh.new()
	sheet.size = Vector3(STRETCHER_WIDTH - 0.04, 0.01, STRETCHER_LENGTH - 0.5)
	cloth.mesh = sheet
	cloth.material_override = canvas
	cloth.position = Vector3(0.0, -0.01, 0.0)
	root.add_child(cloth)
	return root


func _material(name: String, color: Color) -> StandardMaterial3D:
	if not _materials.has(name):
		var m := StandardMaterial3D.new()
		m.albedo_color = color
		m.roughness = 0.9
		_materials[name] = m
	return _materials[name]
