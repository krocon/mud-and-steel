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
## Prompt 073: tall grass on `tall_grass_test` (`SHOT_SET=tall just
## terrain-shots <dir> tall_grass_test`): the stand at 15, 35, 110 and 350 m,
## its edges to meadow, field track, road, water and wood, a vehicle track
## pressed into it, and the view of the picture reference (steeper camera).
## `--biome=` and `--daytime=` pass through for the other biomes and times.
const TALL_FOCUS := Vector3(110, 0, -60)
const TALL_SHOTS: Array[Dictionary] = [
	{"name": "tall_15", "focus": TALL_FOCUS, "distance": 15.0, "yaw": 0.4},
	{"name": "tall_35", "focus": TALL_FOCUS, "distance": 35.0, "yaw": 0.4},
	{"name": "tall_80", "focus": TALL_FOCUS, "distance": 80.0, "yaw": 0.4},
	{"name": "tall_110", "focus": TALL_FOCUS, "distance": 110.0, "yaw": 0.4},
	{"name": "tall_350", "focus": TALL_FOCUS, "distance": 350.0, "yaw": 0.4},
	{"name": "reference", "focus": TALL_FOCUS + Vector3(-10, 0, 8), "distance": 60.0, "yaw": 0.4, "pitch": 50.0},
	{"name": "edge_meadow", "focus": Vector3(-14, 0, -160), "distance": 35.0, "yaw": 0.4},
	{"name": "edge_track", "focus": Vector3(262, 0, -170), "distance": 40.0, "yaw": 0.4},
	{"name": "edge_road", "focus": Vector3(-202, 0, 244), "distance": 40.0, "yaw": 0.4},
	{"name": "edge_water", "focus": Vector3(264, 0, -264), "distance": 40.0, "yaw": 0.4},
	{"name": "edge_wood", "focus": Vector3(-202, 0, 294), "distance": 50.0, "yaw": 0.4},
	{"name": "track", "focus": TALL_FOCUS + Vector3(20, 0, 30), "distance": 35.0, "yaw": 0.4, "track": true},
	{"name": "track_110", "focus": TALL_FOCUS + Vector3(20, 0, 30), "distance": 110.0, "yaw": 0.4, "track": true},
]
## Tall grass on the game maps (`SHOT_SET=tall_land just terrain-shots
## <dir> river_land`): a stand on the river bank, a fallow patch on farms.
const TALL_LAND_SPOTS: Dictionary[String, Vector3] = {
	"river_land": Vector3(-16, 0, -180),
	"farms": Vector3(-42, 0, 250),
}
## Views of the tall grass benchmark (`TALL_AB=1 just terrain-bench
## tall_grass_test`): every view with tall grass and as plain meadow.
const TALL_VIEWS: Array[Dictionary] = [
	{"name": "tall_15", "distance": 15.0, "focus": TALL_FOCUS},
	{"name": "tall_35", "distance": 35.0, "focus": TALL_FOCUS},
	{"name": "tall_80", "distance": 80.0, "focus": TALL_FOCUS},
	{"name": "tall_110", "distance": 110.0, "focus": TALL_FOCUS},
	{"name": "tall_350", "distance": 350.0, "focus": TALL_FOCUS},
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
	var views := VIEWS
	if OS.get_environment("MEADOW_AB") == "1":
		variants = ["all", "no_ground", "no_tufts", "no_dof", "old_sun", "old_blur", "none"]
	# Prompt 073: TALL_AB=1 measures the tall grass views with tall grass
	# ("tall"), with only its look on the ground ("tall_ground", no plants)
	# and the same ground drawn as plain meadow ("meadow").
	if OS.get_environment("TALL_AB") == "1":
		variants = ["tall", "tall_ground", "meadow"]
		views = TALL_VIEWS
	var runs: Array = []
	for v in views.size():
		for k in variants.size():
			# Rotate the order per view against drift.
			runs.append([views[v], variants[(k + v) % variants.size()]])
	for run: Array in runs:
		var view: Dictionary = run[0]
		if run[1] in ["tall", "tall_ground", "meadow"]:
			await _set_tall_variant(run[1] != "meadow", run[1] == "tall")
		else:
			await _set_meadow_variant(run[1])
		var distance: float = view["distance"]
		main.units_root.visible = not view.get("hide_units", false)
		main.water.visible = not view.get("hide_water", false)
		var start := Vector3(-300.0, 0.0, 200.0)
		if view.has("focus"):
			start = view["focus"] - Vector3(0.15, 0.0, -0.12) * distance
		elif view.has("marker") and main.sim_host.map_marker(view["marker"]) != null:
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
		var deco: int = main.map_objects.deco_instances
		print("terrain-bench: %-18s %4.0f m  %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  gpu %s  %4d draw calls  %5.2f M triangles  chunk update %.2f ms  chunks per LOD %s  decoration %d" % [
			view["name"] + ("" if run[1].is_empty() else "/" + run[1]), distance, MEASURE_FRAMES / seconds, seconds * 1000.0 / MEASURE_FRAMES, worst,
			cpu / MEASURE_FRAMES,
			# Metal reports no GPU timestamps (0); show n/a then.
			"%5.2f ms" % (gpu / MEASURE_FRAMES) if gpu > 0.0 else "n/a",
			draw_calls / MEASURE_FRAMES,
			triangles / float(MEASURE_FRAMES) / 1e6, chunk_usec / 1000.0 / MEASURE_FRAMES,
			main.terrain.visible_per_lod, deco,
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


## Tall grass look of the ground on or off (plain meadow), plants on or
## off (prompt 073).
func _set_tall_variant(ground: bool, plants: bool) -> void:
	main.terrain.set_tall_strength(1.0 if ground else 0.0)
	if main.map_objects.tall_grass != plants:
		main.map_objects.tall_grass = plants
		main.map_objects.set_biome(main.map_objects.biome)
	await _frames(30)


## A curved vehicle track through `focus` pressed into the tall grass
## (presentation only): a column of three vehicles' widths.
func _press_track(focus: Vector3) -> void:
	var trails: Node = main.grass_trails
	for k in 120:
		var t := (k - 60) * 0.5
		var at := focus + Vector3(t, 0.0, 6.0 * sin(t / 14.0))
		var heading := Vector2(1.0, 6.0 / 14.0 * cos(t / 14.0))
		trails.press(at, heading, 6.5, 3.1)


func _shots() -> void:
	get_window().size = Vector2i(1920, 1080)
	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	var half: Vector2 = main.sim_host.map_size() * 0.5
	var meadow := OS.get_environment("SHOT_SET") == "meadow"
	var mountains := OS.get_environment("SHOT_SET") == "mountains"
	var tall := OS.get_environment("SHOT_SET") in ["tall", "tall_land"]
	var shots := MEADOW_SHOTS if meadow else (MOUNTAIN_SHOTS if mountains else SHOTS)
	if OS.get_environment("SHOT_SET") == "tall":
		shots = TALL_SHOTS
	elif OS.get_environment("SHOT_SET") == "tall_land":
		var spot: Vector3 = TALL_LAND_SPOTS.get(main.sim_host.map_name(), Vector3.ZERO)
		shots = [
			{"name": "land_45", "focus": spot, "distance": 45.0, "yaw": 0.4},
			{"name": "land_130", "focus": spot, "distance": 130.0, "yaw": 0.4},
		]
	# Prompt 056 (`SHOT_SET=land`): the whole map and every supply point.
	var land := OS.get_environment("SHOT_SET") == "land"
	if land:
		shots = _land_shots()
	if meadow or mountains or land or tall:
		# Clean ground views: no fog of war over the village, no HUD.
		main.fog_of_war.set_fog_enabled(false)
		main.get_node("Hud").visible = false
	var pitch_near: float = rig.pitch_near_deg
	var pitch_far: float = rig.pitch_far_deg
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
		if shot.get("fell", false):
			focus = _fell_trees(focus)
		if shot.get("track", false):
			_press_track(focus)
		main.terrain.set_show_lod(shot.get("lod", false))
		if shot.get("crater", false):
			for k in 3:
				main.terrain.debug_crater(focus + Vector3(-8.0 + 8.0 * k, 0.0, 4.0 * k), 3.0 + k, 1.0)
		# Mountain views (prompt 060): a lower camera and longer distances.
		if not shot.has("pitch") and tall:
			rig.pitch_near_deg = pitch_near
			rig.pitch_far_deg = pitch_far
		if shot.has("pitch"):
			rig.pitch_near_deg = shot["pitch"]
			rig.pitch_far_deg = shot["pitch"]
			rig.max_distance = maxf(rig.max_distance, shot["distance"])
		rig.snap_to(focus, shot["distance"], shot["yaw"])
		print("terrain-shots: view %d '%s' in frames %d..%d" % [i, shot["name"], i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1])
		await _frames(SHOT_FRAMES)
	get_tree().quit()


## Views of a landscape map (prompt 056): the map from above, then every
## supply point from the middle distance and close up.
func _land_shots() -> Array[Dictionary]:
	var shots: Array[Dictionary] = [
		{"name": "overview", "focus": Vector3.ZERO, "distance": 1250.0, "yaw": 0.4, "pitch": 55.0},
		{"name": "overview_low", "focus": Vector3.ZERO, "distance": 700.0, "yaw": 2.2, "pitch": 30.0},
	]
	# Trees felled near the first supply point (drawn only, the
	# simulation is untouched): fallen snowy trees keep their snow.
	shots.append({"name": "fallen_trees", "fell": true, "distance": 45.0, "yaw": 0.9, "pitch": 35.0})
	for p: Dictionary in main.sim_host.supply_points():
		shots.append({"name": p["name"], "focus": p["position"], "distance": 130.0, "yaw": 0.6, "pitch": 38.0})
		shots.append({"name": p["name"] + "_near", "focus": p["position"], "distance": 50.0, "yaw": -0.8, "pitch": 32.0})
	# Prompt 079 (`city_large`): the whole 3 km map, every zone, a start and
	# street views at 15 and 35 m; "city_compare" is the view next to the
	# reference picture (block quarter with the suburb behind it).
	if main.sim_host.map_marker("block_edge_south") != null:
		shots[0]["distance"] = 2600.0
		shots[0]["pitch"] = 60.0
		shots.append_array([
			{"name": "city_compare", "marker": "block_edge_south", "offset": Vector3(0, 0, -40), "distance": 260.0, "yaw": 0.8, "pitch": 38.0},
			{"name": "city_compare_far", "marker": "block_edge_south", "offset": Vector3(0, 0, -60), "distance": 420.0, "yaw": 0.8, "pitch": 40.0},
			{"name": "zone_center", "marker": "town_square", "distance": 280.0, "yaw": 0.6, "pitch": 45.0},
			{"name": "zone_blocks", "marker": "main_street_south", "distance": 420.0, "yaw": 0.6, "pitch": 45.0},
			{"name": "zone_park", "marker": "park_south", "distance": 200.0, "yaw": 0.6, "pitch": 40.0},
			{"name": "zone_warehouses", "marker": "warehouses_south", "distance": 200.0, "yaw": 0.6, "pitch": 40.0},
			{"name": "zone_suburb", "marker": "suburb_south", "distance": 300.0, "yaw": 0.6, "pitch": 40.0},
			{"name": "zone_land", "marker": "strongpoint_south", "distance": 500.0, "yaw": 0.6, "pitch": 35.0},
			{"name": "zone_bypass", "marker": "bypass_west", "distance": 320.0, "yaw": 0.6, "pitch": 38.0},
			{"name": "start", "marker": "start_south", "distance": 220.0, "yaw": 0.6, "pitch": 40.0},
			{"name": "street_15", "marker": "main_street_south", "distance": 15.0, "yaw": 0.0, "pitch": 22.0},
			{"name": "street_35", "marker": "main_street_south", "distance": 35.0, "yaw": 0.0, "pitch": 28.0},
			{"name": "side_street_15", "marker": "side_street_south", "distance": 15.0, "yaw": 1.57, "pitch": 22.0},
			{"name": "side_street_35", "marker": "side_street_south", "distance": 35.0, "yaw": 1.57, "pitch": 28.0},
			{"name": "suburb_street_35", "marker": "crossing_south", "distance": 35.0, "yaw": 0.3, "pitch": 28.0},
		])
	# Prompt 081 (`harbor_city`): the whole map from the sea like the
	# reference pictures, the harbor, the rock, the quarters, houses on the
	# slope and a street at 35 m.
	if main.sim_host.map_marker("rock") != null and main.sim_host.map_marker("square_east") != null:
		shots[0]["distance"] = 2400.0
		shots[0]["pitch"] = 62.0
		shots.append_array([
			{"name": "harbor_compare", "focus": Vector3(-350, 0, 250), "distance": 1700.0, "yaw": 0.0, "pitch": 30.0},
			{"name": "harbor_compare_east", "focus": Vector3(-150, 0, 450), "distance": 900.0, "yaw": 0.35, "pitch": 34.0},
			{"name": "zone_harbor", "marker": "harbor", "distance": 420.0, "yaw": 0.3, "pitch": 40.0},
			{"name": "zone_rock", "marker": "rock", "distance": 320.0, "yaw": -0.4, "pitch": 38.0},
			{"name": "zone_west_flats", "marker": "west_flats", "distance": 420.0, "yaw": 0.0, "pitch": 40.0},
			{"name": "zone_square_east", "marker": "square_east", "distance": 350.0, "yaw": 0.2, "pitch": 40.0},
			{"name": "zone_beach_east", "marker": "beach_east", "distance": 420.0, "yaw": 0.0, "pitch": 38.0},
			{"name": "zone_upper_west", "marker": "upper_west", "distance": 320.0, "yaw": 0.4, "pitch": 38.0},
			{"name": "zone_upper_east", "marker": "upper_east", "distance": 320.0, "yaw": -0.3, "pitch": 38.0},
			{"name": "zone_hinterland", "marker": "hinterland", "distance": 600.0, "yaw": 0.0, "pitch": 40.0},
			{"name": "slope_houses_60", "marker": "upper_east", "distance": 60.0, "yaw": 0.0, "pitch": 30.0},
			{"name": "rock_houses_60", "marker": "rock", "distance": 60.0, "yaw": 0.8, "pitch": 30.0},
			{"name": "harbor_street_35", "marker": "supply_harbor_west", "distance": 35.0, "yaw": 0.4, "pitch": 28.0},
			{"name": "start_west", "marker": "start_west", "distance": 260.0, "yaw": 0.3, "pitch": 40.0},
			{"name": "start_east", "marker": "start_east", "distance": 260.0, "yaw": -0.3, "pitch": 40.0},
		])
	return shots


## Draws the nearest trees to the first supply point as felled (presentation
## only) and returns the point to look at.
func _fell_trees(_focus: Vector3) -> Vector3:
	var points: Array = main.sim_host.supply_points()
	var at: Vector3 = points[0]["position"] if not points.is_empty() else Vector3.ZERO
	var felled := 0
	var center := at
	for id in ["env_tree_conifer", "env_tree_deciduous", "env_tree_birch"]:
		var near: PackedInt64Array = main.sim_host.destructible_objects_near(id, at.x, at.z, 400.0)
		for k in mini(near.size(), 3):
			var state: Dictionary = main.sim_host.object_state(near[k])
			state["stage"] = "destroyed"
			state["remains"] = "fallen"
			state["fall_deg"] = 60 * felled
			main.map_objects.set_object_state(state, false)
			if felled == 0:
				center = main.sim_host.object_position(near[k])
			felled += 1
	return center


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
