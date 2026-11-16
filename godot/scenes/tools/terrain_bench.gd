extends Node
## Terrain benchmark, started by `main.gd` with `-- --terrain-bench`:
##
##     godot --path godot --always-on-top -- --terrain-bench --map=test_large
##
## Sets the window to 1920 x 1080, turns V-Sync off and measures frame rate,
## CPU and GPU frame time, draw calls and triangles in three views (near,
## middle, overview), each while panning slowly so chunk selection runs every
## frame. Prints one line per view and quits. `just terrain-bench` runs it.
##
## With `--terrain-shots` (and `--write-movie <dir>/f.png --fixed-fps 30`) it
## instead holds a series of fixed views for SHOT_FRAMES frames each (near,
## middle, overview, map edge, LOD tint, test crater, bridges, ford) for image comparisons;
## `just terrain-shots <dir>` runs it. The movie writer lags about one frame, so
## frame `n * SHOT_FRAMES + SHOT_FRAMES / 2` shows view n.

const VIEWS: Array[Dictionary] = [
	{"name": "near", "distance": 15.0},
	{"name": "middle", "distance": 120.0},
	{"name": "overview", "distance": 600.0},
	# Same view without the unit models: what the terrain alone costs.
	{"name": "terrain", "distance": 600.0, "hide_units": true},
	# Prompt 014: over the stone bridge, with and without the water surface.
	{"name": "river", "distance": 90.0, "marker": "bridge_stone"},
	{"name": "no_water", "distance": 90.0, "marker": "bridge_stone", "hide_water": true},
]
## Views for --terrain-shots: focus, distance, yaw (radians), options.
const SHOTS: Array[Dictionary] = [
	{"name": "near", "focus": Vector3(-6, 0, 44), "distance": 15.0, "yaw": 0.4},
	{"name": "middle", "focus": Vector3(20, 0, 10), "distance": 120.0, "yaw": 0.4},
	{"name": "overview", "focus": Vector3(0, 0, 0), "distance": 600.0, "yaw": 0.4},
	{"name": "hill", "marker": "hill_top", "distance": 200.0, "yaw": 2.4},
	{"name": "river", "find_terrain": 7, "distance": 90.0, "yaw": -0.6},
	{"name": "village", "marker": "village", "distance": 150.0, "yaw": 0.4},
	{"name": "edge", "corner": true, "distance": 420.0, "yaw": 0.785},
	{"name": "lod", "focus": Vector3(0, 0, 0), "distance": 600.0, "yaw": 0.4, "lod": true},
	{"name": "crater", "focus": Vector3(-6, 0, 44), "distance": 40.0, "yaw": 0.4, "crater": true},
	# Prompt 014: bridges and the ford (markers of the generated maps).
	{"name": "bridge_stone", "marker": "bridge_stone", "distance": 70.0, "yaw": 0.5},
	{"name": "bridge_stone_near", "marker": "bridge_stone", "distance": 28.0, "yaw": -0.7},
	{"name": "bridge_wood", "marker": "bridge_wood", "distance": 45.0, "yaw": 0.3},
	{"name": "ford", "marker": "ford", "distance": 60.0, "yaw": -0.4},
]
## Prompt 057: meadow views at four zoom levels, near the village (trees and
## houses cast shadows onto grass) and in open meadow. `SHOT_SET=meadow` picks
## them instead of SHOTS (`SHOT_SET=meadow just terrain-shots <dir>`).
const MEADOW_SHOTS: Array[Dictionary] = [
	{"name": "meadow_15", "marker": "village", "offset": Vector3(-31, 0, -87), "distance": 15.0, "yaw": 0.4},
	{"name": "meadow_35", "marker": "village", "offset": Vector3(-31, 0, -87), "distance": 35.0, "yaw": 0.4},
	{"name": "meadow_110", "marker": "village", "offset": Vector3(-31, 0, -87), "distance": 110.0, "yaw": 0.4},
	{"name": "meadow_350", "marker": "village", "offset": Vector3(-31, 0, -87), "distance": 350.0, "yaw": 0.4},
	{"name": "open_15", "focus": Vector3(-6, 0, 44), "distance": 15.0, "yaw": 0.4},
	{"name": "open_40", "focus": Vector3(-6, 0, 44), "distance": 40.0, "yaw": 0.4},
]
## Prompt 060: the mountain map (`SHOT_SET=mountains just terrain-shots <dir>
## mountain_pass`): the pass, the serpentine and the lookout, a basin, the
## range from above.
const MOUNTAIN_SHOTS: Array[Dictionary] = [
	{"name": "pass", "focus": Vector3(0, 0, 0), "distance": 600.0, "yaw": 0.0, "pitch": 30.0},
	{"name": "serpentine", "marker": "lookout", "offset": Vector3(100, 0, 0), "distance": 260.0, "yaw": 1.3, "pitch": 40.0},
	{"name": "serpentine_near", "marker": "lookout", "offset": Vector3(130, 0, 20), "distance": 90.0, "yaw": 2.2, "pitch": 35.0},
	{"name": "lookout", "marker": "lookout", "distance": 120.0, "yaw": 1.6, "pitch": 25.0},
	{"name": "basin", "focus": Vector3(0, 0, 560), "distance": 700.0, "yaw": 3.14, "pitch": 28.0},
	{"name": "range", "focus": Vector3(-500, 0, -150), "distance": 900.0, "yaw": 0.9, "pitch": 30.0},
	{"name": "cliffs", "focus": Vector3(450, 0, -650), "distance": 400.0, "yaw": -0.5, "pitch": 35.0},
	{"name": "overview", "focus": Vector3(0, 0, 0), "distance": 2200.0, "yaw": 0.4, "pitch": 45.0},
]
const Settings := preload("res://scenes/menu/settings.gd")
const SHOT_FRAMES := 20
const WARMUP_FRAMES := 60
const MEASURE_FRAMES := 240

var main: Node3D
## Frames rendered with RenderingServer.force_draw because the window could
## not draw (hidden behind other windows or on another Space). Godot skips
## rendering then, which would make frame rates meaningless and movie frames
## stale.
var forced_draws := 0
## Sun elevation of the preset before MEADOW_AB changed it.
var _sun_elevation := -1.0


func _ready() -> void:
	_run()


func _run() -> void:
	if "--terrain-shots" in OS.get_cmdline_user_args():
		await _shots()
		return
	var window := get_window()
	window.size = Vector2i(1920, 1080)
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	await _frames(10)
	print("terrain-bench: map '%s', render size %s, %s" % [
		main.sim_host.map_name(), get_viewport().get_visible_rect().size,
		RenderingServer.get_video_adapter_name()
	])
	# Prompt 057: MEADOW_AB=1 measures every view once per variant, with
	# one part of the meadow look turned off at a time (in one run, so a
	# busy machine affects all variants alike).
	var variants: Array[String] = [""]
	if OS.get_environment("MEADOW_AB") == "1":
		variants = ["all", "no_ground", "no_tufts", "no_dof", "old_sun", "old_blur", "none"]
	var runs: Array = []
	for v in VIEWS.size():
		for k in variants.size():
			# Rotate the order per view against drift.
			runs.append([VIEWS[v], variants[(k + v) % variants.size()]])
	for run: Array in runs:
		var view: Dictionary = run[0]
		await _set_meadow_variant(run[1])
		var distance: float = view["distance"]
		main.units_root.visible = not view.get("hide_units", false)
		main.water.visible = not view.get("hide_water", false)
		var start := Vector3(-300.0, 0.0, 200.0)
		if view.has("marker") and main.sim_host.map_marker(view["marker"]) != null:
			# Pan across the marker (the pan below covers 0.4 distances).
			start = main.sim_host.map_marker(view["marker"]) - Vector3(0.15, 0.0, -0.12) * distance
		rig.snap_to(start, distance, 0.0)
		await _frames(WARMUP_FRAMES)
		var began := Time.get_ticks_usec()
		var cpu := 0.0
		var gpu := 0.0
		var draw_calls := 0
		var triangles := 0
		var chunk_usec := 0
		var worst := 0.0
		var last := began
		for i in MEASURE_FRAMES:
			# Pan 0.4 camera distances over the run, diagonal to the grid.
			var t := float(i) / MEASURE_FRAMES
			rig.snap_to(start + Vector3(0.3, 0.0, -0.25) * distance * t * 1.3, distance, 0.3)
			await _frame()
			var now := Time.get_ticks_usec()
			worst = maxf(worst, (now - last) / 1000.0)
			last = now
			cpu += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
			gpu += RenderingServer.viewport_get_measured_render_time_gpu(viewport_rid)
			chunk_usec += main.terrain.last_update_usec
			draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
			triangles += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
		var seconds := (Time.get_ticks_usec() - began) / 1e6
		print("terrain-bench: %-18s %4.0f m  %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  gpu %s  %4d draw calls  %5.2f M triangles  chunk update %.2f ms  chunks per LOD %s" % [
			view["name"] + ("" if run[1].is_empty() else "/" + run[1]), distance, MEASURE_FRAMES / seconds, seconds * 1000.0 / MEASURE_FRAMES, worst,
			cpu / MEASURE_FRAMES,
			# Metal reports no GPU timestamps (0); show n/a then.
			"%5.2f ms" % (gpu / MEASURE_FRAMES) if gpu > 0.0 else "n/a",
			draw_calls / MEASURE_FRAMES,
			triangles / float(MEASURE_FRAMES) / 1e6, chunk_usec / 1000.0 / MEASURE_FRAMES,
			main.terrain.visible_per_lod,
		])
	print("terrain-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)
	get_tree().quit()


## Turns parts of the meadow look (prompt 057) off for MEADOW_AB:
## "no_ground" terrain shader meadow, "no_tufts" blade tufts (013 clumps
## instead), "no_dof" depth of field, "old_sun" sun at 42 degrees, "old_blur"
## shadow blur 1, "none" all of them (the look before); "all" or "" everything on.
func _set_meadow_variant(variant: String) -> void:
	if variant.is_empty():
		return
	var off_all := variant == "none"
	var preset: Resource = main.atmosphere.preset
	if _sun_elevation < 0.0:
		_sun_elevation = preset.sun_elevation_deg
	Settings.set_value("depth_of_field", "off" if off_all or variant == "no_dof" else "subtle")
	preset.sun_elevation_deg = 42.0 if off_all or variant == "old_sun" else _sun_elevation
	main.atmosphere.shadow_blur = 1.0 if off_all or variant == "old_blur" else 2.0
	main.atmosphere.set_quality(main.atmosphere.quality)
	main.terrain.apply_biome(preset)
	if off_all or variant == "no_ground":
		main.terrain._material.set_shader_parameter("meadow_strength", 0.0)
		main.terrain._material.set_shader_parameter("flower_density", 0.0)
	var tufts := not (off_all or variant == "no_tufts")
	if main.map_objects.meadow_tufts != tufts:
		main.map_objects.meadow_tufts = tufts
		main.map_objects.set_biome(main.map_objects.biome)
	await _frames(2)


func _shots() -> void:
	get_window().size = Vector2i(1920, 1080)
	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	var half: Vector2 = main.sim_host.map_size() * 0.5
	var meadow := OS.get_environment("SHOT_SET") == "meadow"
	var mountains := OS.get_environment("SHOT_SET") == "mountains"
	var shots := MEADOW_SHOTS if meadow else (MOUNTAIN_SHOTS if mountains else SHOTS)
	if meadow or mountains:
		# Clean ground views: no fog of war over the village, no HUD.
		main.fog_of_war.set_fog_enabled(false)
		main.get_node("Hud").visible = false
	for i in shots.size():
		var shot := shots[i]
		if OS.has_environment("SHOT_ONLY") and shot.name != OS.get_environment("SHOT_ONLY"):
			continue
		var focus: Vector3 = shot.get("focus", Vector3.ZERO)
		if shot.has("marker"):
			var marker: Variant = main.sim_host.map_marker(shot["marker"])
			if marker != null:
				focus = marker + shot.get("offset", Vector3.ZERO)
		if shot.has("find_terrain"):
			# First sample of that terrain east of the origin (the river).
			for x in range(0, int(half.x), 2):
				if main.sim_host.terrain_at(x, 0.0) == shot["find_terrain"]:
					focus = Vector3(x, 0.0, 0.0)
					break
		if shot.get("corner", false):
			# Just inside the south-east corner; the camera hangs outside the
			# map and looks north-west at the edge cut.
			focus = Vector3(half.x - 50.0, 0.0, half.y - 50.0)
		main.terrain.set_show_lod(shot.get("lod", false))
		if shot.get("crater", false):
			for k in 3:
				main.terrain.debug_crater(focus + Vector3(-8.0 + 8.0 * k, 0.0, 4.0 * k), 3.0 + k, 1.0)
		# Mountain views (prompt 060): a lower camera and longer distances.
		if shot.has("pitch"):
			rig.pitch_near_deg = shot["pitch"]
			rig.pitch_far_deg = shot["pitch"]
			rig.max_distance = maxf(rig.max_distance, shot["distance"])
		rig.snap_to(focus, shot["distance"], shot["yaw"])
		print("terrain-shots: view %d '%s' in frames %d..%d" % [i, shot["name"], i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1])
		await _frames(SHOT_FRAMES)
	get_tree().quit()


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
