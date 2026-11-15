extends Node
## Infantry drawing benchmark (prompt 022): `just infantry-view-bench`.
##
## Runs on `data/scenarios/infantry_bench.toml` (56 rifle squads, 504
## soldiers; `main.gd` starts it with every unit holding fire). Measures the
## frame time with the soldiers standing from three camera distances, then
## marching towards each other, then fighting, and prints frames per second,
## frame time (mean and worst), process time (GDScript and the simulation)
## and draw calls, plus how many soldiers each animation LOD band held.
## A screenshot of every phase goes to `user://infantry-bench/`.

const BENCH_SIZE := Vector2i(1920, 1080)
const SECONDS := 5.0
const WARMUP_FRAMES := 30

var main: Node3D
var forced_draws := 0


func _ready() -> void:
	_run()


func _run() -> void:
	var window := get_window()
	window.size = BENCH_SIZE
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	await _frames(10)
	var sim_host: SimHost = main.sim_host
	print("infantry-bench: render size %s, %s, %d soldiers in %d squads, quality '%s'" % [
		get_viewport().get_visible_rect().size, RenderingServer.get_video_adapter_name(),
		sim_host.soldier_units().size(), sim_host.unit_ids().size(), main.atmosphere.quality])
	for distance: float in [60.0, 150.0, 400.0]:
		await _measure("standing, camera %d m" % int(distance), distance)
	# Only the local player gives orders: its squads march and open fire,
	# the enemy stands and holds fire.
	var own: Array[int] = []
	var owners := sim_host.unit_owners()
	var ids := sim_host.unit_ids()
	for i in ids.size():
		if owners[i] == sim_host.local_player():
			own.append(ids[i])
	sim_host.move_units(PackedInt64Array(own), 0.0, 0.0)
	await _measure("marching, camera 150 m", 150.0)
	sim_host.set_hold_fire(PackedInt64Array(own), false)
	await _measure("fighting, camera 150 m", 150.0)
	await _measure("fighting, camera 60 m", 60.0)
	print("infantry-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)
	get_tree().quit()


func _measure(label: String, distance: float) -> void:
	main.camera_rig.snap_to(Vector3(0, 0, 0), distance, 0.7)
	await _frames(WARMUP_FRAMES)
	var began := Time.get_ticks_usec()
	var last := began
	var frames := 0
	var worst := 0.0
	var process := 0.0
	var view_usec := 0
	var draw_calls := 0
	var bands := [0, 0, 0, 0]
	while (Time.get_ticks_usec() - began) / 1e6 < SECONDS:
		await _frame()
		var now := Time.get_ticks_usec()
		worst = maxf(worst, (now - last) / 1000.0)
		last = now
		frames += 1
		process += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		view_usec += main.infantry_view.process_usec
		draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		for b in 4:
			bands[b] += main.infantry_view.counts[b]
	var seconds := (Time.get_ticks_usec() - began) / 1e6
	DirAccess.make_dir_recursive_absolute("user://infantry-bench")
	var shot := "user://infantry-bench/%s.png" % label.replace(", ", "_").replace(" ", "_")
	get_viewport().get_texture().get_image().save_png(shot)
	print("infantry-bench: %-24s %6.1f FPS  frame %5.2f ms (worst %6.2f)  process %5.2f ms (infantry view %4.2f)  %5d draw calls  LOD near/mid/far/hidden %d/%d/%d/%d" % [
		label, frames / seconds, seconds * 1000.0 / frames, worst, process / frames,
		view_usec / 1000.0 / frames, draw_calls / frames, bands[0] / frames, bands[1] / frames, bands[2] / frames, bands[3] / frames])


func _frames(count: int) -> void:
	for i in count:
		await _frame()


func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
