extends Node
## Crater benchmark (prompt 021), started by `main.gd`:
##
##     godot --path godot -- --effects-craters [--map=test_large]
##
## Sets the window to 1920 x 1080 and turns V-Sync off. Every unit holds
## fire. From 150 m above the middle between both sides it measures the
## frame rate for a few seconds without craters, then while 100 craters of
## 7.5 cm rounds fall within 2 s on an area 120 m across (simulation craters
## through `SimHost.debug_crater`, each with the ground-hit effect of prompt
## 019), and in the seconds afterwards. Reports frames per second, the
## worst frame, terrain uploads and their cost, and rebuilt decoration
## squares. `just crater-view-bench [map]`.

const BENCH_SIZE := Vector2i(1920, 1080)
const CRATERS := 100
const CRATER_SECONDS := 2.0
const RADIUS := 60.0
const CALIBER := 75.0
const IDLE_SECONDS := 4.0
const AFTER_SECONDS := 4.0
const WARMUP_FRAMES := 60

var main: Node3D
var forced_draws := 0


func _ready() -> void:
	_run()


func _run() -> void:
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	get_window().size = BENCH_SIZE
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	await _frames(10)
	var center := (_centroid(true) + _centroid(false)) * 0.5
	main.camera_rig.snap_to(center, 150.0, 0.5)
	# A first impact compiles the effect pipelines.
	main.effects.ground_hit(_ground(center + Vector3(200, 0, 200)), CALIBER)
	await _frames(WARMUP_FRAMES)
	print("crater-bench: map '%s', render size %s, %s, quality '%s', biome '%s'" % [
		main.sim_host.map_name(), get_viewport().get_visible_rect().size,
		RenderingServer.get_video_adapter_name(), main.atmosphere.quality, main.atmosphere.preset.id])

	var idle := await _measure(IDLE_SECONDS, center, false)
	_report("idle", idle)
	var view: Node = main.crater_view
	var uploads_before: int = view.uploads
	var rebuilds_before: int = main.map_objects.crater_rebuilds
	var count_before: int = main.sim_host.crater_count()
	var during := await _measure(CRATER_SECONDS, center, true)
	_report("100 craters in 2 s", during)
	var after := await _measure(AFTER_SECONDS, center, false)
	_report("after", after)
	print("crater-bench: %d craters made, %d terrain uploads (last %.1f ms), %d decoration squares rebuilt, %d forced draws" % [
		main.sim_host.crater_count() - count_before, view.uploads - uploads_before, view.last_upload_ms,
		main.map_objects.crater_rebuilds - rebuilds_before, forced_draws])
	get_tree().quit()


## Frames for `seconds`; with `craters`, 100 craters spread evenly over
## the time. Returns [frames, seconds, worst frame ms].
func _measure(seconds: float, center: Vector3, craters: bool) -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = 21
	var began := Time.get_ticks_usec()
	var last := began
	var frames := 0
	var worst := 0.0
	var made := 0
	while (Time.get_ticks_usec() - began) / 1e6 < seconds:
		var t := (Time.get_ticks_usec() - began) / 1e6
		while craters and made < CRATERS and made < int(ceil(t / CRATER_SECONDS * CRATERS)):
			var p := center + Vector3(rng.randf_range(-RADIUS, RADIUS), 0, rng.randf_range(-RADIUS, RADIUS))
			main.sim_host.debug_crater(p.x, p.z, CALIBER)
			main.effects.ground_hit(_ground(p), CALIBER)
			made += 1
		await _frame()
		var now := Time.get_ticks_usec()
		worst = maxf(worst, (now - last) / 1000.0)
		last = now
		frames += 1
	return [frames, (Time.get_ticks_usec() - began) / 1e6, worst]


func _report(name: String, m: Array) -> void:
	print("crater-bench: %-20s %6.1f FPS  frame %5.2f ms  worst %6.2f ms" % [
		name, m[0] / m[1], m[1] * 1000.0 / m[0], m[2]])


func _centroid(own: bool) -> Vector3:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var positions: PackedVector3Array = sim_host.current_positions()
	var sum := Vector3.ZERO
	var n := 0
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own:
			sum += positions[i]
			n += 1
	return sum / maxi(n, 1)


func _ground(point: Vector3) -> Vector3:
	return Vector3(point.x, main.sim_host.height_at(point.x, point.z), point.z)


func _frames(count: int) -> void:
	for i in count:
		await _frame()


func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
