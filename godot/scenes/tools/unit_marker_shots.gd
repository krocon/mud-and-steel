extends Node
## Pictures of the own-unit markers (prompt 164) as PNG files, started by
## `main.gd` with `-- --unit-marker-shots=<dir>` (`just unit-marker-shots`):
## the same view of an own squad with the setting `off`, the faint markers
## (`always`, cursor elsewhere), the cursor over the squad, and the show-all
## key held. Extra arguments (`--map=`, `--daytime=`, `--weather=`) pick the
## scene; `--prefix=` names the files. Needs a window. Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")

var main: Node3D
var dir := ""
var prefix := "meadow"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--unit-marker-shots="):
			dir = arg.trim_prefix("--unit-marker-shots=")
		if arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var squad := -1
	var at := Vector3.ZERO
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var stances := host.unit_stances()
	var positions := host.current_positions()
	for i in ids.size():
		if owners[i] == host.local_player() and stances[i] >= 0 and not host.unit_is_crew(ids[i]):
			squad = ids[i]
			at = positions[i]
			break
	if squad < 0:
		push_error("unit marker shots: no own squad")
		get_tree().quit(1)
		return
	var highlight: Node = main.unit_highlight
	main.camera_rig.snap_to(at, 110.0, deg_to_rad(20.0))
	highlight.forced_point = Vector3(-100000, 0, -100000)
	Settings.set_value("unit_markers", "off")
	await _frames(30)
	await _save("%s_1_off.png" % prefix)
	Settings.set_value("unit_markers", "always")
	await _frames(5)
	await _save("%s_2_markers.png" % prefix)
	highlight.forced_point = _squad_position(squad)
	await _frames(5)
	await _save("%s_3_hover.png" % prefix)
	highlight.forced_point = Vector3(-100000, 0, -100000)
	Input.action_press("show_units")
	await _frames(5)
	await _save("%s_4_show_all.png" % prefix)
	Input.action_release("show_units")
	highlight.forced_point = null
	get_tree().quit()


func _squad_position(id: int) -> Vector3:
	var host: SimHost = main.sim_host
	var k := Array(host.unit_ids()).find(id)
	return host.current_positions()[k] if k >= 0 else Vector3.ZERO


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("unit marker shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
