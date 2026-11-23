extends Node
## Buildings collapsing storey by storey in pictures (prompt 197), started
## by `main.gd` (`just collapse-shots`, needs a window):
##
##     godot --path godot --fixed-fps 40 res://scenes/main.tscn -- --map=river_town --collapse-shots=<dir>
##
## With the fog of war off it brings down the three-storey town house, the
## two-storey stone house and the single-storey farmhouse nearest to the
## middle of the map, for the presentation only (a made-up `ruin` event to
## map_objects.gd and effects.gd; the simulation keeps them standing), each
## close up and from game height, and saves a frame every 0.25 s as
## `<dir>/<house>_<view>/frame_<k>.png` plus one contact sheet per clip
## (`<house>_<view>_sheet.png`). Then it measures the frame time with all
## three falling at once against the same seconds without, per graphics
## quality, and prints one line per level (`collapse-bench:`);
## `--collapse-bench-only` skips the pictures.

const SHOT_SIZE := Vector2i(1280, 720)
const BuildingCollapse := preload("res://scenes/world/building_collapse.gd")
const HOUSES: Array[String] = ["env_house_town", "env_house_stone", "env_house_farm"]
## Camera distance close up and at game height (m).
const VIEWS: Dictionary[String, float] = {"near": 38.0, "game": 110.0}
## Seconds recorded per clip, frames between pictures at --fixed-fps 40.
const CLIP_SECONDS := 7.5
const STEP_FRAMES := 10
const FPS := 40.0
## Thumbnail size and columns of the contact sheet.
const SHEET_CELL := Vector2i(320, 180)
const SHEET_COLUMNS := 6
## Seconds measured per quality level, with and without collapses.
const BENCH_SECONDS := 7.0
## Houses with trees closer than this (m) hide behind them.
const TREE_RADIUS := 14.0
const TREES: Array[String] = ["env_tree_deciduous", "env_tree_conifer", "env_tree_birch", "env_tree_palm"]
## Shot direction of the made-up events (simulation degrees).
const FALL_DEG := 60

var main: Node3D
var _dir := ""
## House -> camera yaw of its clips.
var _yaws: Dictionary[int, float] = {}


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--collapse-shots="):
			_dir = arg.trim_prefix("--collapse-shots=")
	await _shots()
	get_tree().quit()


func _shots() -> void:
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.hud.visible = false
	await _frames(20)
	var sim_host: SimHost = main.sim_host
	var size := sim_host.map_size()
	var middle := Vector3(size.x * 0.5, 0.0, size.y * 0.5)
	var houses: Array[int] = []
	for id in HOUSES:
		var found := sim_host.destructible_objects_near(id, middle.x, middle.z, 5000.0)
		if found.is_empty():
			push_error("collapse-shots: no %s on the map" % id)
			continue
		# Of the nearest few, the one with the fewest trees around it and
		# between it and the near camera.
		var best := found[0]
		var fewest := 1 << 30
		for k in mini(found.size(), 24):
			var view := _clear_view(found[k])
			if view.y < fewest:
				fewest = int(view.y)
				best = found[k]
				_yaws[best] = view.x
		houses.append(best)
	if not "--collapse-bench-only" in OS.get_cmdline_user_args():
		for house in houses:
			for view: String in VIEWS:
				await _clip(house, view)
	await _bench(houses)
	print("collapse-shots: pictures in %s" % ProjectSettings.globalize_path(_dir))


## One clip of `house` from `view`: a frame every 0.25 s and a contact sheet.
func _clip(house: int, view: String) -> void:
	var sim_host: SimHost = main.sim_host
	var objects: Node3D = main.map_objects
	var at: Vector3 = sim_host.object_position(house)
	at.y = sim_host.height_at(at.x, at.z) + 4.0
	var rig: Node3D = main.camera_rig
	_aim(at, VIEWS[view], _yaws.get(house, deg_to_rad(35.0)), view == "near")
	_stand(house)
	await _frames(30)
	var name := "%s_%s" % [str(sim_host.object_state(house).id).trim_prefix("env_"), view]
	var folder := "%s/%s" % [_dir, name]
	DirAccess.make_dir_recursive_absolute(folder)
	_fall(house)
	var cells: Array[Image] = []
	var count := int(CLIP_SECONDS * FPS / STEP_FRAMES)
	for k in count:
		var image := get_viewport().get_texture().get_image()
		image.save_png("%s/frame_%02d.png" % [folder, k])
		var cell := image.duplicate() as Image
		cell.resize(SHEET_CELL.x, SHEET_CELL.y, Image.INTERPOLATE_BILINEAR)
		cells.append(cell)
		await _frames(STEP_FRAMES)
	var log: Array = objects.collapse_log.get(house, [])
	print("collapse-shots: %s bands %s" % [name, log])
	_sheet(cells).save_png("%s/%s_sheet.png" % [_dir, name])
	_stand(house)


## Frame times with the three houses falling at once, against standing
## ones, per graphics quality.
func _bench(houses: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	var center := Vector3.ZERO
	for house in houses:
		center += sim_host.object_position(house)
	center /= maxf(houses.size(), 1)
	center.y = sim_host.height_at(center.x, center.z)
	var spread := 0.0
	for house in houses:
		spread = maxf(spread, center.distance_to(sim_host.object_position(house)))
	main.camera_rig.snap_to(center, clampf(spread * 1.6 + 60.0, 110.0, 400.0), deg_to_rad(35.0))
	var before: String = main.atmosphere.quality
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	for level in ["low", "medium", "high"]:
		main.atmosphere.set_quality(level)
		for house in houses:
			_stand(house)
		await _frames(240)
		var still := await _frame_ms()
		for house in houses:
			_fall(house)
		var falling := await _frame_ms()
		print("collapse-bench: %s standing frame %.2f ms (max %.1f), collapse scripts %.3f, render CPU %.2f | three falling frame %.2f ms (max %.1f), collapse scripts %.3f, render CPU %.2f" % [
			level, still.frame, still.worst, still.script, still.render,
			falling.frame, falling.worst, falling.script, falling.render])
	for house in houses:
		_stand(house)
	main.atmosphere.set_quality(before)


## Over the frames of BENCH_SECONDS (as many frames as the collapse takes
## at --fixed-fps): mean frame time, mean script time of the collapses
## (BuildingCollapse.usec_total) and CPU time of rendering (viewport and
## frame setup), worst frame, all in ms. With a locked screen or vsync the frame time stays at
## the display's rate; the CPU times are what the collapse costs.
func _frame_ms() -> Dictionary:
	var frames := int(BENCH_SECONDS * FPS)
	var total := 0.0
	var worst := 0.0
	var script := 0.0
	var render := 0.0
	var rid := get_viewport().get_viewport_rid()
	var collapse_usec := BuildingCollapse.usec_total
	var last := Time.get_ticks_usec()
	for k in frames:
		await get_tree().process_frame
		render += RenderingServer.viewport_get_measured_render_time_cpu(rid) + RenderingServer.get_frame_setup_time_cpu()
		var now := Time.get_ticks_usec()
		var ms := (now - last) / 1000.0
		last = now
		total += ms
		worst = maxf(worst, ms)
	script = (BuildingCollapse.usec_total - collapse_usec) / 1000.0
	return {"frame": total / frames, "worst": worst, "script": script / frames, "render": render / frames}


## Places the camera on `at`; close up it looks flatter, at game height
## as the zoom gives.
func _aim(at: Vector3, distance: float, yaw: float, flat: bool) -> void:
	var rig: Node3D = main.camera_rig
	rig.snap_to(at, distance, yaw)
	var limits: Vector2 = rig.tilt_range()
	rig.target_tilt = clampf(deg_to_rad(30.0) - rig.base_pitch(), limits.x, limits.y) if flat else 0.0
	rig.snap_to(at, distance, yaw)


## The camera yaw (of eight) with the fewest trees around `house` and on
## the line to the near camera, and that count.
func _clear_view(house: int) -> Vector2:
	var at: Vector3 = main.sim_host.object_position(house)
	at.y = main.sim_host.height_at(at.x, at.z) + 4.0
	var best := Vector2(0.0, INF)
	for k in 8:
		var yaw := k * TAU / 8.0
		_aim(at, VIEWS["near"], yaw, true)
		var eye: Vector3 = main.camera.global_position
		var trees := _trees_near(at)
		for f in [0.25, 0.5, 0.75]:
			trees += _trees_near(at.lerp(eye, f), 9.0)
		if trees < best.y:
			best = Vector2(yaw, trees)
	return best


## Trees within `radius` of `at`.
func _trees_near(at: Vector3, radius := TREE_RADIUS) -> int:
	var count := 0
	for id in TREES:
		count += main.sim_host.destructible_objects_near(id, at.x, at.z, radius).size()
	return count


## Draws `house` standing again (its MultiMesh instance).
func _stand(house: int) -> void:
	var state: Dictionary = main.sim_host.object_state(house)
	state["object"] = house
	state["stage"] = "intact"
	main.map_objects.set_object_state(state, false)


## A made-up ruin event for `house` (presentation only).
func _fall(house: int) -> void:
	var state: Dictionary = main.sim_host.object_state(house)
	state["object"] = house
	state["stage"] = "destroyed"
	state["remains"] = "ruin"
	state["variant"] = "Ruin"
	state["fall_deg"] = FALL_DEG
	state["cause"] = "hit"
	main.map_objects.set_object_state(state, true)
	main.effects.object_event(state)


func _sheet(cells: Array[Image]) -> Image:
	var rows := ceili(cells.size() / float(SHEET_COLUMNS))
	var sheet := Image.create(SHEET_CELL.x * SHEET_COLUMNS, SHEET_CELL.y * rows, false, Image.FORMAT_RGBA8)
	for k in cells.size():
		var cell := cells[k]
		cell.convert(Image.FORMAT_RGBA8)
		sheet.blit_rect(cell, Rect2i(Vector2i.ZERO, SHEET_CELL),
			Vector2i((k % SHEET_COLUMNS) * SHEET_CELL.x, (k / SHEET_COLUMNS) * SHEET_CELL.y))
	return sheet


func _frames(n: int) -> void:
	for k in n:
		await get_tree().process_frame
