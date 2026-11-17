extends Node
## Map object benchmark and shots (prompt 013), started by `main.gd`:
##
##     godot --path godot --always-on-top -- --objects-bench --map=test_large
##
## Finds the 128 m chunk with the most trees and measures frame rate, draw
## calls, triangles and instance counts there while panning slowly: near
## (15 m), middle (120 m) and overview (600 m), the overview once more
## without objects and decoration, and the first village from 150 m.
## `just objects-bench` runs it.
##
## With `--objects-shots` (and `--write-movie <dir>/f.png --fixed-fps 30`) it
## holds fixed views for SHOT_FRAMES frames each instead; frame
## `n * SHOT_FRAMES + SHOT_FRAMES / 2` shows view n. `just objects-shots <dir>`.
##
## `--rocks` (prompt 074, on a map with grown rock such as `rock_hills`)
## shoots the rock selection instead: slab, outcrop and ridge piece nearest
## to the map center at 15, 35, 110 and 350 m, the ridge piece on the
## steepest slope, and three views like the reference pictures, in all
## three biomes (`--daytime=evening` for low sun). `--rocks-bench` measures
## frame rate over the densest ridge field instead of the wood views.

const WARMUP_FRAMES := 90
const MEASURE_FRAMES := 240
const SHOT_FRAMES := 30
## Views relative to the densest wood chunk ("wood") or the village marker.
const VIEWS: Array[Dictionary] = [
	{"name": "near", "at": "edge", "distance": 15.0},
	{"name": "near-nodeco", "at": "edge", "distance": 15.0, "deco": false},
	{"name": "near-noshadow", "at": "edge", "distance": 15.0, "shadows": false},
	{"name": "near-noobj", "at": "edge", "distance": 15.0, "objects": false},
	{"name": "middle", "at": "edge", "distance": 120.0},
	{"name": "middle-noshadow", "at": "edge", "distance": 120.0, "shadows": false},
	{"name": "overview", "at": "wood", "distance": 600.0},
	{"name": "bare", "at": "wood", "distance": 600.0, "hide": true},
	{"name": "village", "at": "village", "distance": 150.0},
]
const SHOTS: Array[Dictionary] = [
	{"name": "wood-near", "at": "edge", "distance": 25.0, "yaw": 0.5},
	{"name": "wood-middle", "at": "edge", "distance": 120.0, "yaw": 0.5},
	{"name": "wood-overview", "at": "wood", "distance": 600.0, "yaw": 0.5},
	{"name": "village", "at": "village", "distance": 110.0, "yaw": 0.9},
	{"name": "village-near", "at": "village-edge", "distance": 45.0, "yaw": 2.2},
	{"name": "meadow-near", "at": "meadow", "distance": 15.0, "yaw": 0.4},
	{"name": "hedgerow", "at": "hedge", "distance": 60.0, "yaw": 1.2},
]

var main: Node3D
var forced_draws := 0
var _points: Dictionary[String, Vector3] = {}


func _ready() -> void:
	_run()


const ROCK_DISTANCES: Array[float] = [15.0, 35.0, 110.0, 350.0]
const ROCK_KINDS: Array[String] = ["env_rock_slab", "env_rock_outcrop", "env_rock_massif"]


func _run() -> void:
	get_window().size = Vector2i(1920, 1080)
	main.camera_rig.edge_scroll_enabled = false
	var args := OS.get_cmdline_user_args()
	if "--rocks" in args or "--rocks-bench" in args:
		_find_rock_points()
		if "--rocks-bench" in args:
			await _rocks_bench()
		else:
			await _rock_shots()
		get_tree().quit()
		return
	_find_points()
	if "--objects-shots" in args:
		await _shots()
	else:
		await _bench()
	get_tree().quit()


## Rock points (prompt 074): per kind the instance nearest to the map
## center; "massif-slope": the ridge piece with the largest height
## difference over 20 m; "ridge-field": the 128 m chunk with the most ridge
## pieces; "slab-field": the slab with the most slabs within 40 m.
func _find_rock_points() -> void:
	var host: SimHost = main.sim_host
	var kinds := host.object_kinds()
	var best_d: Dictionary[String, float] = {}
	var steepest := -1.0
	var per_chunk: Dictionary[Vector2i, int] = {}
	var slabs: Array[Vector3] = []
	for group: Dictionary in host.object_groups(128.0):
		var id: String = kinds[group.kind].get_slice(":", 0)
		if not id in ROCK_KINDS:
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
			var d := Vector2(p.x, p.z).length()
			if d < best_d.get(id, INF):
				best_d[id] = d
				_points[id] = p
			if id == "env_rock_massif":
				var c: Vector2i = group.chunk
				per_chunk[c] = per_chunk.get(c, 0) + 1
				var drop := absf(host.height_at(p.x + 10.0, p.z) - host.height_at(p.x - 10.0, p.z)) \
					+ absf(host.height_at(p.x, p.z + 10.0) - host.height_at(p.x, p.z - 10.0))
				if drop > steepest:
					steepest = drop
					_points["massif-slope"] = p
			elif id == "env_rock_slab":
				slabs.append(p)
	var best := Vector2i.ZERO
	for c: Vector2i in per_chunk:
		if per_chunk[c] > per_chunk.get(best, 0):
			best = c
	var half := host.map_size() * 0.5
	var field := Vector3(-half.x + (best.x + 0.5) * 128.0, 0.0, -half.y + (best.y + 0.5) * 128.0)
	_points["ridge-field"] = Vector3(field.x, host.height_at(field.x, field.z), field.z)
	var most := -1
	for a in slabs:
		var n := 0
		for b in slabs:
			if a.distance_to(b) < 40.0:
				n += 1
		if n > most:
			most = n
			_points["slab-field"] = a
	print("objects-shots: rocks %s, steepest ridge piece drop %.1f m, ridge field chunk %s (%d pieces), slab field %d slabs" % [
		_points, steepest, best, per_chunk.get(best, 0), most
	])


func _rock_shot_list() -> Array[Dictionary]:
	var shots: Array[Dictionary] = []
	for id in ROCK_KINDS:
		if not _points.has(id):
			continue
		for d in ROCK_DISTANCES:
			shots.append({"name": "%s-%d" % [id.trim_prefix("env_rock_"), int(d)], "at": id, "distance": d, "yaw": 0.6})
	if _points.has("massif-slope"):
		shots.append({"name": "massif-slope-110", "at": "massif-slope", "distance": 110.0, "yaw": 0.6})
		shots.append({"name": "massif-slope-35", "at": "massif-slope", "distance": 35.0, "yaw": 2.2})
	# Like the reference pictures: slabs with an outcrop from about 120 m,
	# an outcrop from about 60 m, a ridge field from high above.
	if _points.has("slab-field"):
		shots.append({"name": "compare-slabs", "at": "slab-field", "distance": 120.0, "yaw": 0.3})
	if _points.has("env_rock_outcrop"):
		shots.append({"name": "compare-outcrop", "at": "env_rock_outcrop", "distance": 60.0, "yaw": 1.0})
	shots.append({"name": "compare-ridge", "at": "ridge-field", "distance": 260.0, "yaw": 0.0})
	return shots


## With `--out=<dir>` every view is saved as `<dir>/<biome>-<view>.png`
## (`just rock-shots`); otherwise `--write-movie` records the frames.
func _rock_shots() -> void:
	var rig: Node3D = main.camera_rig
	var shots := _rock_shot_list()
	var out := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	if not out.is_empty():
		DirAccess.make_dir_recursive_absolute(out)
	main.fog_of_war.set_fog_enabled(false)
	main.hud.visible = false
	var biomes: Array[String] = ["forest", "desert", "winter"]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--biomes="):
			biomes.assign(arg.trim_prefix("--biomes=").split(","))
	var i := 0
	for biome in biomes:
		main.set_biome(biome)
		for shot in shots:
			rig.snap_to(_points[shot.at], shot.distance, shot.yaw)
			var name := "%s-%s" % [biome, shot.name]
			print("objects-shots: view %d '%s' in frames %d..%d" % [
				i, name, i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1
			])
			# Real time without a movie: give the decoration time to build.
			await _frames(SHOT_FRAMES if out.is_empty() else SHOT_FRAMES * 2)
			if not out.is_empty():
				var image := get_viewport().get_texture().get_image()
				image.save_png(out.path_join(name + ".png"))
			i += 1


## Frame rate over the densest ridge field (prompt 074) at 35, 110 and
## 350 m, like `_bench`.
func _rocks_bench() -> void:
	var views: Array[Dictionary] = []
	for d in [35.0, 110.0, 350.0]:
		views.append({"name": "ridges-%d" % int(d), "at": "ridge-field", "distance": d})
	await _measure(views)


## Wood: center of the chunk with the most trees; edge: 50 m south of it
## (so near views look into the wood); village: the map marker; hedge: a
## hedge segment near the meadow.
func _find_points() -> void:
	var host: SimHost = main.sim_host
	var kinds := host.object_kinds()
	var trees: Dictionary[Vector2i, int] = {}
	var best := Vector2i.ZERO
	for group: Dictionary in host.object_groups(128.0):
		if not kinds[group.kind].begins_with("env_tree_"):
			continue
		var chunk: Vector2i = group.chunk
		trees[chunk] = trees.get(chunk, 0) + (group.buffer as PackedFloat32Array).size() / 12
		if trees[chunk] > trees.get(best, 0):
			best = chunk
	var half := host.map_size() * 0.5
	var wood := Vector3(-half.x + (best.x + 0.5) * 128.0, 0.0, -half.y + (best.y + 0.5) * 128.0)
	_points["wood"] = wood
	_points["edge"] = wood + Vector3(0.0, 0.0, 50.0)
	var village: Variant = host.map_marker("village")
	_points["village"] = village if village != null else Vector3.ZERO
	_points["village-edge"] = _points["village"] + Vector3(0.0, 0.0, 48.0)
	_points["meadow"] = Vector3(-6.0, 0.0, 44.0)
	_points["hedge"] = Vector3.ZERO
	var nearest := INF
	for group: Dictionary in host.object_groups(128.0):
		if kinds[group.kind] != "env_hedge:Segment":
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], 0.0, buffer[k * 12 + 11])
			if p.length() < nearest:
				nearest = p.length()
				_points["hedge"] = p
	print("objects-bench: wood chunk %s with %d trees, village %s, hedge %s" % [
		best, trees.get(best, 0), _points["village"], _points["hedge"]
	])


func _bench() -> void:
	await _measure(VIEWS)


func _measure(views: Array) -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	var rig: Node3D = main.camera_rig
	var objects: Node3D = main.map_objects
	print("objects-bench: map '%s', %d objects in %d nodes (built in %.0f ms), render size %s, %s" % [
		main.sim_host.map_name(), objects.object_instances, objects.object_nodes, objects.build_msec,
		get_viewport().get_visible_rect().size, RenderingServer.get_video_adapter_name(),
	])
	for view: Dictionary in views:
		var hide: bool = view.get("hide", false)
		objects.set_objects_visible(not hide and view.get("objects", true))
		objects.set_decoration_visible(not hide and view.get("deco", true))
		objects.set_shadows(view.get("shadows", true))
		var distance: float = view.distance
		var start: Vector3 = _points[view.at]
		rig.snap_to(start, distance, 0.0)
		await _frames(WARMUP_FRAMES)
		var began := Time.get_ticks_usec()
		var cpu := 0.0
		var draw_calls := 0
		var triangles := 0
		var worst := 0.0
		var last := began
		var deco := 0
		for i in MEASURE_FRAMES:
			var t := float(i) / MEASURE_FRAMES
			rig.snap_to(start + Vector3(0.3, 0.0, -0.25) * distance * t, distance, 0.3)
			await _frame()
			var now := Time.get_ticks_usec()
			worst = maxf(worst, (now - last) / 1000.0)
			last = now
			cpu += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
			draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
			triangles += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
			deco += objects.deco_instances
		var seconds := (Time.get_ticks_usec() - began) / 1e6
		print("objects-bench: %-8s %4.0f m  %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  %5d draw calls  %5.2f M triangles  decoration %d" % [
			view.name, distance, MEASURE_FRAMES / seconds, seconds * 1000.0 / MEASURE_FRAMES, worst,
			cpu / MEASURE_FRAMES, draw_calls / MEASURE_FRAMES, triangles / float(MEASURE_FRAMES) / 1e6,
			deco / MEASURE_FRAMES,
		])
	objects.set_objects_visible(true)
	objects.set_decoration_visible(true)
	print("objects-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)


func _shots() -> void:
	var rig: Node3D = main.camera_rig
	for i in SHOTS.size():
		var shot := SHOTS[i]
		rig.snap_to(_points[shot.at], shot.distance, shot.yaw)
		print("objects-shots: view %d '%s' in frames %d..%d" % [i, shot.name, i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1])
		await _frames(SHOT_FRAMES)


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
