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


func _run() -> void:
	get_window().size = Vector2i(1920, 1080)
	main.camera_rig.edge_scroll_enabled = false
	_find_points()
	if "--objects-shots" in OS.get_cmdline_user_args():
		await _shots()
	else:
		await _bench()
	get_tree().quit()


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
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	var rig: Node3D = main.camera_rig
	var objects: Node3D = main.map_objects
	print("objects-bench: map '%s', %d objects in %d nodes (built in %.0f ms), render size %s, %s" % [
		main.sim_host.map_name(), objects.object_instances, objects.object_nodes, objects.build_msec,
		get_viewport().get_visible_rect().size, RenderingServer.get_video_adapter_name(),
	])
	for view in VIEWS:
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
