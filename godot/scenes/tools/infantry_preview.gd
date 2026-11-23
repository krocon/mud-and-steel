extends Node3D
## Infantry preview (prompt 005): every soldier model side by side, one row per
## faction, with selectable biome and animation.
##
## Controls: biome and animation in the panel, left/right arrow keys step through the
## clips, drag with the right mouse button to orbit, mouse wheel to zoom.
##
## Start with `-- --self-test` to check all models headless (clips, skeleton names,
## sockets, biome materials) and quit with exit code 0 (pass) or 1 (fail).
## `-- --shot=<file.png> [--biome=winter] [--clip=walk] [--camo=auto|camo|none]
## [--faction=de] [--roles=rifleman,sniper] [--distance=6]` saves one screenshot and
## quits (needs a window, not --headless). Key C cycles the camouflage mode (prompt 042).

const FACTIONS: Array[String] = ["de", "su", "us", "uk"]
const SPACING := 1.6
const ROW_SPACING := 2.4
## Bones Godot's SkeletonProfileHumanoid requires (see tools/blender/soldier/skeleton.py).
const REQUIRED_BONES: Array[String] = [
	"Hips", "Spine", "Head",
	"LeftShoulder", "LeftUpperArm", "LeftLowerArm", "LeftHand",
	"RightShoulder", "RightUpperArm", "RightLowerArm", "RightHand",
	"LeftUpperLeg", "LeftLowerLeg", "LeftFoot",
	"RightUpperLeg", "RightLowerLeg", "RightFoot",
]
## Roles without a weapon in the hands; "faction/role" for roles armed only in some factions.
const UNARMED_ROLES: Array[String] = ["medic", "mg_assistant", "ammo_carrier"]

@onready var camera: Camera3D = $Camera3D
@onready var models_root: Node3D = $Models
@onready var biome_select: OptionButton = %BiomeSelect
@onready var clip_select: OptionButton = %ClipSelect
@onready var info_label: Label = %Info

var _soldiers: Array[Dictionary] = []  # {node, faction, role, player}
var _orbit_yaw := PI - 0.55  # soldiers face -Z: look at them from the front
var _orbit_pitch := 0.45
var _orbit_distance := 16.0
var _orbit_target := Vector3.ZERO
var _self_test := "--self-test" in OS.get_cmdline_user_args()
var _options := _parse_options()
var _camo_mode: String = _options.get("camo", "auto")


func _ready() -> void:
	_load_models()
	for biome in SoldierLook.BIOMES:
		biome_select.add_item(biome)
	var names: Array = SoldierLook.clips().keys()
	for clip_name: String in names:
		clip_select.add_item(clip_name)
	biome_select.item_selected.connect(func(_i: int) -> void: _apply_biome())
	clip_select.item_selected.connect(func(_i: int) -> void: _play_selected())
	_select_by_text(biome_select, _options.get("biome", "forest"))
	_select_by_text(clip_select, _options.get("clip", "idle"))
	_apply_biome()
	_play_selected()
	_orbit_distance = float(_options.get("distance", _orbit_distance))
	_update_camera()
	if _self_test:
		get_tree().create_timer(120.0).timeout.connect(func() -> void:
			printerr("infantry self-test: timeout")
			get_tree().quit(1))
		_run_self_test.call_deferred()
	elif _options.has("shot"):
		_take_shot.call_deferred(_options["shot"])


func _parse_options() -> Dictionary:
	var out := {}
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--") and "=" in arg:
			out[arg.substr(2).get_slice("=", 0)] = arg.get_slice("=", 1)
	return out


func _select_by_text(button: OptionButton, text: String) -> void:
	for i in button.item_count:
		if button.get_item_text(i) == text:
			button.select(i)


func _take_shot(path: String) -> void:
	for i in 30:
		await get_tree().process_frame
	get_viewport().get_texture().get_image().save_png(path)
	print("infantry preview: saved ", path)
	get_tree().quit(0)


func _load_models() -> void:
	var row := 0
	var only_roles: PackedStringArray = _options.get("roles", "").split(",", false)
	for faction in FACTIONS:
		if _options.has("faction") and faction != _options["faction"]:
			continue
		var dir_path := "res://assets/models/%s/infantry" % faction
		var files := ResourceLoader.list_directory(dir_path)
		var glbs: Array[String] = []
		for f in files:
			if f.ends_with(".glb") and (only_roles.is_empty() or f.get_basename() in only_roles):
				glbs.append(f)
		glbs.sort()
		for i in glbs.size():
			var scene: PackedScene = load("%s/%s" % [dir_path, glbs[i]])
			if scene == null:
				continue
			var node := scene.instantiate() as Node3D
			models_root.add_child(node)
			var x := (i - (glbs.size() - 1) * 0.5) * SPACING
			node.position = Vector3(x, 0, (row - 1) * ROW_SPACING if not _options.has("faction") else 0.0)
			var player := SoldierLook.find_player(node)
			if player != null:
				SoldierLook.configure_animations(player)
			var label := Label3D.new()
			label.text = "%s %s" % [faction, glbs[i].get_basename()]
			label.pixel_size = 0.004
			label.position = Vector3(0, 2.15, 0)
			label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
			node.add_child(label)
			_soldiers.append({
				"node": node, "faction": faction, "role": glbs[i].get_basename(),
				"player": player,
			})
		row += 1
	info_label.text = "%d soldiers (C: camouflage)" % _soldiers.size()


func _apply_biome() -> void:
	var biome := biome_select.get_item_text(biome_select.selected)
	for i in _soldiers.size():
		var s := _soldiers[i]
		SoldierLook.apply(s.node, s.faction, biome, i, _camo_mode)


func _play_selected() -> void:
	if clip_select.item_count == 0:
		return
	var clip_name := clip_select.get_item_text(clip_select.selected)
	for s in _soldiers:
		var player: AnimationPlayer = s.player
		if player != null and player.has_animation(clip_name):
			player.play(clip_name)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_RIGHT:
		_orbit_yaw -= event.relative.x * 0.01
		_orbit_pitch = clampf(_orbit_pitch + event.relative.y * 0.01, 0.05, 1.5)
		_update_camera()
	elif event is InputEventMouseButton and event.pressed:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			_orbit_distance = maxf(2.0, _orbit_distance * 0.9)
			_update_camera()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_orbit_distance = minf(80.0, _orbit_distance * 1.1)
			_update_camera()
	elif event is InputEventKey and event.pressed and event.keycode == KEY_C:
		var modes := SoldierLook.CAMO_MODES
		_camo_mode = modes[(modes.find(_camo_mode) + 1) % modes.size()]
		info_label.text = "%d soldiers, camouflage: %s" % [_soldiers.size(), _camo_mode]
		_apply_biome()
	elif event is InputEventKey and event.pressed and clip_select.item_count > 0:
		var step := 0
		if event.keycode == KEY_RIGHT:
			step = 1
		elif event.keycode == KEY_LEFT:
			step = -1
		if step != 0:
			clip_select.select(posmod(clip_select.selected + step, clip_select.item_count))
			_play_selected()


func _update_camera() -> void:
	var offset := Vector3(
		sin(_orbit_yaw) * cos(_orbit_pitch), sin(_orbit_pitch), cos(_orbit_yaw) * cos(_orbit_pitch)
	)
	camera.position = _orbit_target + offset * _orbit_distance + Vector3(0, 0.9, 0)
	camera.look_at(_orbit_target + Vector3(0, 0.9, 0), Vector3.UP)


# -- self-test ----------------------------------------------------------------------------


func _run_self_test() -> void:
	var errors: Array[String] = []
	var clips: Dictionary = SoldierLook.clips()
	if clips.is_empty():
		errors.append("no clip metadata (animations.json)")
	var per_faction := {}
	for s in _soldiers:
		var tag := "%s/%s" % [s.faction, s.role]
		per_faction[s.faction] = per_faction.get(s.faction, []) + [s.role]
		var player: AnimationPlayer = s.player
		if player == null:
			errors.append("%s: no AnimationPlayer" % tag)
			continue
		for clip_name: String in clips:
			if not player.has_animation(clip_name):
				errors.append("%s: clip %s missing" % [tag, clip_name])
		var skeleton := SoldierLook.find_skeleton(s.node)
		if skeleton == null:
			errors.append("%s: no Skeleton3D" % tag)
		else:
			for bone in REQUIRED_BONES:
				if skeleton.find_bone(bone) < 0:
					errors.append("%s: bone %s missing" % [tag, bone])
			if skeleton.find_bone("Weapon") < 0:
				errors.append("%s: bone Weapon missing" % tag)
			errors.append_array(_check_profile(skeleton, tag))
		var has_muzzle: bool = not (s.node as Node).find_children("Socket_Muzzle*", "", true, false).is_empty()
		if s.role in UNARMED_ROLES or ("%s/%s" % [s.faction, s.role]) in UNARMED_ROLES:
			if has_muzzle:
				errors.append("%s: unarmed role has a weapon" % tag)
		elif not has_muzzle:
			errors.append("%s: Socket_Muzzle missing" % tag)
	for faction in FACTIONS:
		for role in ["rifleman", "mg_gunner"]:
			if role not in per_faction.get(faction, []):
				errors.append("%s/%s missing" % [faction, role])
	# Play every clip on every soldier for a few frames, in every biome.
	for b in SoldierLook.BIOMES.size():
		biome_select.select(b)
		_apply_biome()
		for c in clip_select.item_count:
			clip_select.select(c)
			_play_selected()
			await get_tree().process_frame
	errors.append_array(_check_materials())
	errors.append_array(_check_camo())
	errors.append_array(await _check_motion())
	if errors.is_empty():
		print("infantry self-test: ok (%d soldiers, %d clips, %d biomes)" % [
			_soldiers.size(), clips.size(), SoldierLook.BIOMES.size()])
		get_tree().quit(0)
	else:
		for e in errors:
			printerr("infantry self-test: ", e)
		get_tree().quit(1)


## Every skeleton bone except Weapon maps 1:1 onto SkeletonProfileHumanoid.
func _check_profile(skeleton: Skeleton3D, tag: String) -> Array[String]:
	var errors: Array[String] = []
	var profile := SkeletonProfileHumanoid.new()
	for i in skeleton.get_bone_count():
		var bone := skeleton.get_bone_name(i)
		if bone == "Weapon":
			continue
		if profile.find_bone(bone) < 0:
			errors.append("%s: bone %s is not in SkeletonProfileHumanoid" % [tag, bone])
	return errors


## Camouflage (prompt 042): in the forest, "camo" puts every faction's pattern on
## the Cloth slot, "auto" only on the palette's roles, "none" on nobody; other
## biomes never show it.
func _check_camo() -> Array[String]:
	var errors: Array[String] = []
	for biome in ["forest", "winter"]:
		for mode: String in SoldierLook.CAMO_MODES:
			for i in _soldiers.size():
				var s := _soldiers[i]
				SoldierLook.apply(s.node, s.faction, biome, i, mode)
				var want: bool = biome == "forest" and SoldierLook.wears_camo(s.faction, s.role, mode)
				var got := _cloth_camo(s.node)
				if got < 0:
					errors.append("%s/%s: no Cloth material" % [s.faction, s.role])
				elif (got > 0) != want:
					errors.append("%s/%s: camouflage %s in %s/%s" % [
						s.faction, s.role, "shown" if got > 0 else "missing", biome, mode])
	_apply_biome()
	return errors


func _cloth_camo(model: Node) -> int:
	for mi: MeshInstance3D in model.find_children("*", "MeshInstance3D", true, false):
		for i in mi.get_surface_override_material_count():
			var mat := mi.get_surface_override_material(i) as ShaderMaterial
			if mat and mat.resource_name == "Cloth":
				return int(mat.get_shader_parameter(&"camo"))
	return -1


func _check_materials() -> Array[String]:
	var errors: Array[String] = []
	for s in _soldiers:
		for mi: MeshInstance3D in s.node.find_children("*", "MeshInstance3D", true, false):
			for i in mi.mesh.get_surface_count():
				if mi.get_surface_override_material(i) == null:
					errors.append("%s/%s: %s surface %d has no slot material" % [
						s.faction, s.role, mi.name, i])
	return errors


## Prompt 048: a ragdoll falls, settles and freezes its pose; a knocked-down
## ragdoll is released into the animation; the head leads a turn; a helmet
## flies off and comes to rest. Presentation only.
func _check_motion() -> Array[String]:
	var errors: Array[String] = []
	var armed: Array[Dictionary] = []
	for s in _soldiers:
		if s.role == "rifleman":
			armed.append(s)
	if armed.size() < 3:
		return ["motion: fewer than three riflemen"]
	var ragdolls := preload("res://scenes/world/ragdolls.gd").new()
	add_child(ragdolls)
	# A dead soldier thrown backwards: falls, settles, keeps the pose.
	var dead: Node3D = armed[0].node
	(armed[0].player as AnimationPlayer).play("death_a")
	if not ragdolls.start(dead, Vector3(0, 3.0, 2.4), Vector3(4.0, 0, 0), 0.0, "dead"):
		errors.append("motion: ragdoll did not start")
	var skeleton := SoldierLook.find_skeleton(dead)
	var hips := skeleton.find_bone("Hips")
	var head := skeleton.find_bone("Head")
	for i in int((ragdolls.SETTLE_SECONDS + 0.4) * Engine.physics_ticks_per_second):
		await get_tree().physics_frame
	(armed[0].player as AnimationPlayer).active = false
	await get_tree().process_frame
	if not dead.has_meta(&"frozen_pose"):
		errors.append("motion: dead ragdoll not frozen")
	if ragdolls.active_count() != 0:
		errors.append("motion: %d ragdolls still active" % ragdolls.active_count())
	var head_y := (skeleton.global_transform * skeleton.get_bone_global_pose(head).origin).y
	if head_y > 0.8 or head_y < -0.2:
		errors.append("motion: frozen body's head at %.2f m (not lying on the ground)" % head_y)
	# A knocked-down soldier: lies, is released into the get-up clip.
	var down: Node3D = armed[1].node
	var down_skeleton := SoldierLook.find_skeleton(down)
	ragdolls.start(down, Vector3(-2.0, 3.0, 0), Vector3(0, 0, 4.0), 0.0, "down")
	for i in 70:
		await get_tree().physics_frame
	var lying: Dictionary = ragdolls.lying(down)
	if lying.is_empty():
		errors.append("motion: no lying state of a knocked-down ragdoll")
	elif (lying["hips"] as Vector3).y > 0.6:
		errors.append("motion: knocked-down hips still %.2f m up" % (lying["hips"] as Vector3).y)
	(armed[1].player as AnimationPlayer).play("get_up_back" if lying.get("back", true) else "get_up_front")
	ragdolls.release(down, 0.2)
	for i in 30:
		await get_tree().physics_frame
	if ragdolls.is_active(down) or down_skeleton.find_child("Ragdoll", false, false) != null and \
			not (down_skeleton.get_node("Ragdoll") as Node).is_queued_for_deletion():
		errors.append("motion: released ragdoll still there")
	# Head first: the head turns further than the hips.
	var turner: Node3D = armed[2].node
	var t_skeleton := SoldierLook.find_skeleton(turner)
	var t_player := armed[2].player as AnimationPlayer
	t_player.play("idle")
	t_player.active = false
	await get_tree().process_frame
	var t_head := t_skeleton.find_bone("Head")
	var before := t_skeleton.get_bone_global_pose(t_head).basis * Vector3(0, 0, -1)
	var lead := preload("res://scenes/world/turn_lead.gd").new()
	t_skeleton.add_child(lead)
	lead.target = 1.0
	# Modifier results are read while they apply (Godot restores the poses).
	var seen: Array[Vector3] = [before]
	lead.modification_processed.connect(func() -> void:
		seen[0] = t_skeleton.get_bone_global_pose(t_head).basis * Vector3(0, 0, -1))
	for i in 40:
		await get_tree().process_frame
	var after := seen[0]
	var turned := absf(Vector2(before.x, before.z).angle_to(Vector2(after.x, after.z)))
	if turned < deg_to_rad(30.0):
		errors.append("motion: head leads the turn by only %.0f deg" % rad_to_deg(turned))
	lead.queue_free()
	# A helmet flies off and lands.
	var helmets := preload("res://scenes/world/helmets.gd").new()
	add_child(helmets)
	var hatless: Node3D = armed[3 % armed.size()].node if armed.size() > 3 else armed[0].node
	if not helmets.knock_off(hatless, "de", "forest", Vector3(1.5, 3.0, 0), Vector3(0, 8, 3), 0.0):
		errors.append("motion: helmet not knocked off")
	var gear := hatless.find_child("Gear", true, false) as GeometryInstance3D
	if gear == null or gear.get_instance_shader_parameter(&"helmet_off") != 1.0:
		errors.append("motion: helmet still drawn on the head")
	for i in 120:
		await get_tree().process_frame
		if helmets.flying() == 0:
			break
	if helmets.flying() != 0:
		errors.append("motion: helmet still flying")
	ragdolls.queue_free()
	helmets.queue_free()
	return errors
