extends Node
## Pictures of the overview, "HUD off" and the photo mode (prompt 103) as
## PNG files, started by `main.gd` with `-- --view-shots=<dir>` (`just
## view-shots`): the normal view, the overview, the same view without HUD,
## and a photo of the photo mode with the miniature depth of field (saved by
## the photo mode itself). Needs a window. Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--view-shots="):
			dir = arg.trim_prefix("--view-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(30)
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var own: Array[int] = []
	for i in ids.size():
		if owners[i] == host.local_player() and own.size() < 6:
			own.append(ids[i])
	main.selection.set_selection(own)
	var at: Vector3 = host.current_positions()[Array(ids).find(own[0])]
	main.camera_rig.snap_to(at, 140.0, deg_to_rad(20.0))
	await _frames(40)
	await _save("view_1_normal.png")
	main.overview.enter()
	await _frames(150)
	await _save("view_2_overview.png")
	main.overview.leave()
	await _frames(150)
	main.view_toggles.set_hidden(true, true)
	await _frames(10)
	await _save("view_3_hud_off.png")
	main.view_toggles.set_hidden(false)
	main.photo_mode.enter()
	Settings.set_value("depth_of_field", "miniature")
	main.camera_rig.target_distance = 35.0
	main.camera_rig.target_tilt = -0.25
	await _frames(90)
	var path: String = await main.photo_mode.save_photo()
	if not path.is_empty():
		DirAccess.copy_absolute(path, dir.path_join("view_4_photo.png"))
		print("view shots: %s" % dir.path_join("view_4_photo.png"))
	await _save("view_5_photo_mode_panel.png")
	main.photo_mode.leave()
	get_tree().quit()


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("view shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
