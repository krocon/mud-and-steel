extends Node
## Pictures of the order feedback (prompt 175) as PNG files, started by
## `main.gd` with `-- --feedback-shots=<dir>` (`just feedback-shots`):
## the target preview of an order into deep water (the marks move to where
## the tanks stop), the info field of a tank in combat (activity and fire
## line), and the penetration light with the pointer over an enemy. Needs
## a window. Quits when done.

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--feedback-shots="):
			dir = arg.trim_prefix("--feedback-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(30)
	var host: SimHost = main.sim_host
	var me := host.local_player()
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var tanks: Array[int] = []
	var enemies: Array[int] = []
	for i in ids.size():
		if host.unit_stances()[i] >= 0:
			continue
		if owners[i] == me:
			tanks.append(ids[i])
		else:
			enemies.append(ids[i])
	# 1. An order into deep water (the widest water near the tanks), or
	# else beyond the map edge: the marks stand where the tanks stop.
	var start: Vector3 = host.current_positions()[ids.find(tanks[0])]
	var water := Vector3.INF
	var best := -1
	for gx in range(-60, 61):
		for gz in range(-60, 61):
			var x := gx * 4.0
			var z := gz * 4.0
			var wet := 0
			for r in range(0, 40, 4):
				var all := true
				for o: Vector2 in [Vector2(r, 0), Vector2(-r, 0), Vector2(0, r), Vector2(0, -r)]:
					if host.ground_info(x + o.x, z + o.y).get("terrain_name", "") != "deep_water":
						all = false
				if not all:
					break
				wet = r
			if wet > best and host.ground_info(x, z).get("terrain_name", "") == "deep_water":
				best = wet
				water = Vector3(x, 0.0, z)
	if best < 8:
		water = Vector3(start.x, 0.0, start.z + 600.0)
	main.selection.set_selection(tanks.slice(0, 4))
	if water != Vector3.INF:
		main.camera_rig.snap_to(water.lerp(start, 0.35), 160.0, deg_to_rad(35.0))
		await _frames(20)
		main.hud.order_preview._fetch(water, Vector3.ZERO, tanks.slice(0, 4))
		await _frames(5)
		await _save("unreachable.png")
		main.hud.order_preview.slots = PackedFloat32Array()
	# 2. A tank in combat: wait until one fires or aims, then its info field.
	var fighter := -1
	for k in 600:
		await _frames(2)
		for id in tanks:
			var status := host.unit_fire_status(id)
			if not status.is_empty() and str(status.status) in ["firing", "reloading", "aiming"]:
				fighter = id
				break
		if fighter >= 0:
			break
	if fighter < 0:
		fighter = tanks[0]
	main.selection.set_selection([fighter])
	var at: Vector3 = host.current_positions()[host.unit_ids().find(fighter)]
	main.camera_rig.snap_to(at, 90.0, deg_to_rad(20.0))
	await _frames(30)
	await _save("info_in_combat.png")
	# 3. The penetration light: the pointer over the nearest visible enemy.
	var camera: Camera3D = main.camera
	for enemy in enemies:
		var i := host.unit_ids().find(enemy)
		if i < 0 or host.unit_alive()[i] == 0:
			continue
		var node: Node3D = main._unit_nodes.get(enemy)
		if node == null or not node.is_visible_in_tree():
			continue
		var p: Vector3 = host.current_positions()[i]
		main.camera_rig.snap_to(p.lerp(at, 0.5), 110.0, deg_to_rad(20.0))
		await _frames(20)
		p = host.current_positions()[host.unit_ids().find(enemy)]
		get_viewport().warp_mouse(camera.unproject_position(p + Vector3.UP))
		await _frames(20)
		if main.hud.order_feedback.hover_target >= 0:
			break
	await _save("penetration.png")
	get_tree().quit()


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("feedback shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
