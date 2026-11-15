extends Node3D
## Infantry preview (prompt 005): every soldier model side by side, one row per
## faction, with selectable biome and animation.
##
## Controls: biome and animation in the panel, left/right arrow keys step through the
## clips, drag with the right mouse button to orbit, mouse wheel to zoom.
##
## Start with `-- --self-test` to check all models headless (clips, skeleton names,
## sockets, biome materials) and quit with exit code 0 (pass) or 1 (fail).
## `-- --shot=<file.png> [--biome=winter] [--clip=walk]` saves one screenshot and quits
## (needs a window, not --headless).

const FACTIONS: Array[String] = ["de", "su", "us"]
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
const UNARMED_ROLES: Array[String] = ["medic", "mg_assistant", "ammo_carrier", "su/officer"]

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
	for faction in FACTIONS:
		var dir_path := "res://assets/models/%s/infantry" % faction
		var files := ResourceLoader.list_directory(dir_path)
		var glbs: Array[String] = []
		for f in files:
			if f.ends_with(".glb"):
				glbs.append(f)
		glbs.sort()
		for i in glbs.size():
			var scene: PackedScene = load("%s/%s" % [dir_path, glbs[i]])
			if scene == null:
				continue
			var node := scene.instantiate() as Node3D
			models_root.add_child(node)
			var x := (i - (glbs.size() - 1) * 0.5) * SPACING
			node.position = Vector3(x, 0, (row - 1) * ROW_SPACING)
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
	info_label.text = "%d soldiers" % _soldiers.size()


func _apply_biome() -> void:
	var biome := biome_select.get_item_text(biome_select.selected)
	for i in _soldiers.size():
		var s := _soldiers[i]
		SoldierLook.apply(s.node, s.faction, biome, i)


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


func _check_materials() -> Array[String]:
	var errors: Array[String] = []
	for s in _soldiers:
		for mi: MeshInstance3D in s.node.find_children("*", "MeshInstance3D", true, false):
			for i in mi.mesh.get_surface_count():
				if mi.get_surface_override_material(i) == null:
					errors.append("%s/%s: %s surface %d has no slot material" % [
						s.faction, s.role, mi.name, i])
	return errors
