extends Node
## Records fixed views of base building (prompt 023) as PNG files, started by
## `main.gd` with `-- --mode=base --base-shots=<dir>` (`just base-shots
## <dir>`): the own base with a placement preview, a construction site, the
## completed barracks with its production bar, and the enemy base. The build
## menu is open in the first two, two orders are queued in the fourth (picture
## tiles, prompt 058). Needs a
## window (not headless). Quits when done.

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--base-shots="):
			dir = arg.trim_prefix("--base-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var host: SimHost = main.sim_host
	var hud: Control = main.base_hud
	await _frames(10)
	var hq := _hq(host, 0)
	main.camera_rig.snap_to(hq + Vector3(0, 0, -15), 70.0, deg_to_rad(20.0))
	var barracks := -1
	for o: Dictionary in host.build_options():
		if o["id"] == "base_barracks":
			barracks = o["type"]
	main.hud.command_bar.press("build")
	hud.start_placing(barracks)
	# Move the preview over a valid and an invalid spot.
	var spot := Vector3.ZERO
	for r: float in [20.0, 30.0, 40.0]:
		for k in 16:
			var p := hq + Vector3(cos(TAU * k / 16.0), 0, sin(TAU * k / 16.0)) * r
			if spot == Vector3.ZERO and host.placement_check(barracks, p.x, p.z).is_empty():
				spot = p
	await _shot_at(hud, spot, "1_preview_valid.png")
	await _shot_at(hud, hq, "2_preview_blocked.png")
	hud.place_at(spot)
	main.hud.command_bar.press("build")
	hud.cancel_placing()
	host.speed = 4.0
	await _frames(90)
	await _save("3_construction_site.png")
	while true:
		var done := false
		for s: Dictionary in host.structures():
			if s["owner"] == 0 and s["type_id"] == "base_barracks" and float(s["built"]) >= 1.0:
				done = true
				main.selection._set_selection([s["id"]])
		if done:
			break
		await _frames(10)
	host.speed = 1.0
	await _frames(30)
	for t: int in hud.production_picker.tiles.keys().slice(0, 2):
		hud.production_picker.press_type(t)
	await _frames(40)
	await _save("4_barracks_selected.png")
	main.camera_rig.snap_to(_hq(host, 1) + Vector3(0, 0, 35), 75.0, deg_to_rad(200.0))
	await _frames(60)
	await _save("5_enemy_base.png")
	main.camera_rig.snap_to(Vector3.ZERO, 420.0)
	await _frames(60)
	await _save("6_overview.png")
	get_tree().quit()


func _hq(host: SimHost, player: int) -> Vector3:
	for s: Dictionary in host.structures():
		if s["owner"] == player and s["type_id"] == "base_hq":
			return host.current_positions()[Array(host.unit_ids()).find(s["id"])]
	return Vector3.ZERO


func _shot_at(hud: Control, point: Vector3, file: String) -> void:
	var screen: Vector2 = (main.camera as Camera3D).unproject_position(point)
	get_viewport().warp_mouse(screen)
	await _frames(20)
	await _save(file)


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("base shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
