extends Node
## Picture of the alarms out of view (prompt 101) as a PNG file, started by
## `main.gd` with `-- --alert-shots=<dir>` (`just alert-shots`): two own
## units reported under fire and lost at places off the screen, so the edge
## arrows and the feed lines show. Needs a window. Quits when done.

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--alert-shots="):
			dir = arg.trim_prefix("--alert-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var own := -1
	for i in ids.size():
		if owners[i] == host.local_player():
			own = ids[i]
			break
	var center := Vector3(0.0, 0.0, 0.0)
	main.camera_rig.snap_to(center, 70.0, deg_to_rad(20.0))
	await _frames(20)
	var alerts: Node = main.hud.alerts
	alerts.report("fire", own, Vector3(180.0, host.height_at(180.0, -60.0), -60.0))
	alerts.report("lost", own, Vector3(-150.0, host.height_at(-150.0, 120.0), 120.0))
	await _frames(10)
	await _save("alerts.png")
	get_tree().quit()


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("alert shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
