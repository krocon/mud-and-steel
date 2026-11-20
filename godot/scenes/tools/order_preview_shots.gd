extends Node
## Pictures of the target preview and the range circles (prompt 099) as PNG
## files, started by `main.gd` with `-- --order-preview-shots=<dir>`
## (`just order-preview-shots`): the own units with the right button held
## and dragged for a line, a wedge and a column, then the range and sight
## circles of the selection. Extra arguments (`--map=`, `--biome=`,
## `--daytime=`, `--weather=`) pick the scene; `--prefix=` names the files.
## Needs a window. Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")
## Formation codes (`SimHost.formation_names`): line, wedge, column.
const SHOTS := {1: "line", 2: "wedge", 3: "column"}
const UNITS := 8

var main: Node3D
var dir := ""
var prefix := "meadow"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--order-preview-shots="):
			dir = arg.trim_prefix("--order-preview-shots=")
		if arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var selection: Control = main.selection
	var own: Array[int] = []
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var positions := host.current_positions()
	var center := Vector3.ZERO
	for i in ids.size():
		if owners[i] == host.local_player() and own.size() < UNITS:
			own.append(ids[i])
			center += positions[i]
	if own.is_empty():
		push_error("order preview shots: no own units")
		get_tree().quit(1)
		return
	center /= own.size()
	selection._set_selection(own)
	var target := center + Vector3(0.0, 0.0, -45.0)
	target.y = host.height_at(target.x, target.z)
	main.camera_rig.snap_to(center.lerp(target, 0.5), 120.0, deg_to_rad(20.0))
	await _frames(30)
	for code: int in SHOTS:
		selection.set_formation(code)
		var from: Vector2 = main.camera.unproject_position(target)
		var to: Vector2 = main.camera.unproject_position(target + Vector3(-12.0, 0.0, -18.0))
		_press(from, to)
		await _frames(4)
		await _save("%s_preview_%s.png" % [prefix, SHOTS[code]])
		selection._right_pressing = false
		selection.queue_redraw()
		await _frames(2)
	Settings.set_value("range_circles", "always")
	Input.action_press("show_ranges")
	main.camera_rig.snap_to(center, 330.0, deg_to_rad(20.0))
	await _frames(20)
	await _save("%s_circles.png" % prefix)
	Input.action_release("show_ranges")
	get_tree().quit()


## Holds the right button at `from` and drags to `to` (no release: no order).
func _press(from: Vector2, to: Vector2) -> void:
	var press := InputEventMouseButton.new()
	press.button_index = MOUSE_BUTTON_RIGHT
	press.pressed = true
	press.position = from
	press.global_position = from
	get_viewport().push_input(press)
	var motion := InputEventMouseMotion.new()
	motion.position = to
	motion.global_position = to
	motion.button_mask = MOUSE_BUTTON_MASK_RIGHT
	get_viewport().push_input(motion)


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("order preview shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
