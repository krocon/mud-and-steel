extends Node
## Pictures of the supply points (prompt 107) as PNG files, started by
## `main.gd` with `-- --mode=base --supply-shots=<dir>` (`just supply-shots
## [dir]`). Needs a window (not headless); quits when done.
##
## 1. `1_neutral.png`: the point nearest the own units as it is at the
##    start, with the ground tooltip; `2_own.png` after the own units took
##    it (real play at eight times the speed).
## 2. `3_ally.png`, `4_enemy.png`, `5_contested.png`: the same point with
##    the state set by hand (`supply_view.frozen`), since the test match
##    has no ally and the enemy does not move; `6_own_subtle.png` with
##    `supply_border` = subtle.
## Beside every picture `minimap_<name>.png`: the minimap enlarged three
## times.

const Settings := preload("res://scenes/menu/settings.gd")

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--supply-shots="):
			dir = arg.trim_prefix("--supply-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var host: SimHost = main.sim_host
	await _frames(20)
	var supply: Node = main.base_view.supply
	var me := host.local_player()
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var positions := host.current_positions()
	var own: Array[int] = []
	var center := Vector3.ZERO
	for i in ids.size():
		if owners[i] == me and not main.base_view.structures.has(ids[i]):
			own.append(ids[i])
			center += positions[i]
	if own.is_empty() or supply.known.is_empty():
		push_error("supply shots: no own units or no supply points")
		get_tree().quit(1)
		return
	center /= own.size()
	var target := 0
	for i in supply.known.size():
		if center.distance_to(supply.known[i].position) < center.distance_to(supply.known[target].position):
			target = i
	var at: Vector3 = supply.known[target].position
	main.camera_rig.snap_to(at, 95.0, deg_to_rad(25.0))
	await _frames(40)
	await _shot("1_neutral", at)
	main.selection._set_selection(own)
	main.selection.order_move_to(at + Vector3(12, 0, 8))
	main.selection._set_selection([])
	host.speed = 8.0
	var frames := 4000
	while int(supply.known[target].owner) != me and frames > 0:
		frames -= 1
		await _frames(1)
	host.speed = 1.0
	await _frames(40)
	await _shot("2_own", at)
	supply.frozen = true
	var k: Dictionary = supply.known[target]
	var enemy := 1 if me != 1 else 0
	for state: Array in [["3_ally", "ally", false], ["4_enemy", "enemy", false], ["5_contested", "own", true]]:
		k.relation = state[1]
		k.owner = me if state[1] == "own" else enemy
		k.contested = state[2]
		k.taker = -1
		await _frames(20 if state[2] else 10)
		if state[2]:
			# Catch the bright phase of the blink.
			while cos(supply._time * TAU * supply.BLINK_HZ) < 0.8:
				await _frames(1)
		await _shot(state[0], at)
	k.relation = "own"
	k.owner = me
	k.contested = false
	Settings.set_value("supply_border", "subtle")
	await _frames(10)
	await _shot("6_own_subtle", at)
	Settings.set_value("supply_border", "strong")
	get_tree().quit()


func _shot(file: String, at: Vector3) -> void:
	var tooltip: Control = main.hud.ground_tooltip
	var screen: Vector2 = (main.camera as Camera3D).unproject_position(at + Vector3(6, 0, 6))
	# The box would hide again without a resting pointer.
	tooltip.set_process(false)
	tooltip.show_at(at + Vector3(6, 0, 6), screen)
	await _frames(4)
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file + ".png"))
	var minimap: Control = main.hud.minimap
	# Canvas to image pixels (the window may scale its content).
	var to_image := Vector2(image.get_size()) / get_viewport().get_visible_rect().size
	var area := minimap.get_global_rect()
	var rect := Rect2i(Rect2(area.position * to_image, area.size * to_image))
	rect = rect.intersection(Rect2i(Vector2i.ZERO, image.get_size()))
	if rect.size.x > 0 and rect.size.y > 0:
		var crop := image.get_region(rect)
		crop.resize(crop.get_width() * 3, crop.get_height() * 3, Image.INTERPOLATE_NEAREST)
		crop.save_png(dir.path_join("minimap_" + file + ".png"))
	tooltip.visible = false
	tooltip.set_process(true)
	print("supply shots: %s" % dir.path_join(file + ".png"))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
