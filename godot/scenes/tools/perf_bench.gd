extends Node
## Overall frame-rate benchmark (prompt 016): `just bench-view`.
##
## `main.gd` starts `data/scenarios/bench_large.toml` (504 units, 2250
## soldiers on the 4 km map) with a Hard computer opponent. The bench
## measures three camera distances over the own army while it stands, then
## sends the army towards the enemy, waits for the battle and measures the
## same distances over the fighting (effects, tracers, wrecks, crushed
## objects). Per view it prints frames per second, frame time (mean, p99,
## worst), the simulation's share of the frame (`SimHost.process_usec`:
## ticks, AI, snapshots), GDScript process time, render CPU time and draw
## calls. Screenshots go to `user://perf-bench/`. See docs/PERFORMANCE.md.

const BENCH_SIZE := Vector2i(1920, 1080)
const SECONDS := 6.0
const WARMUP_FRAMES := 30
const DISTANCES: Array[float] = [40.0, 150.0, 600.0]
## Real seconds to wait for the first shots after the advance order.
const BATTLE_TIMEOUT := 150.0
## Real seconds of battle before the battle views are measured.
const BATTLE_SETTLE := 8.0

var main: Node3D
var forced_draws := 0
var _shots_seen := 0


func _ready() -> void:
	_run()


func _run() -> void:
	var window := get_window()
	window.size = BENCH_SIZE
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	main.camera_rig.edge_scroll_enabled = false
	main.combat_view.combat_event.connect(_on_combat_event)
	await _frames(10)
	var sim_host: SimHost = main.sim_host
	print("perf-bench: map '%s', render size %s, %s, %d units, %d soldiers, quality '%s'" % [
		sim_host.map_name(), get_viewport().get_visible_rect().size,
		RenderingServer.get_video_adapter_name(), sim_host.unit_ids().size(),
		sim_host.soldier_units().size(), main.atmosphere.quality])
	for distance in DISTANCES:
		await _measure("standing", distance)
	# Only the local player is ordered: its army drives towards the enemy's
	# side; the Hard AI meets it.
	var own := _own_units()
	var center := _center(own)
	sim_host.move_units(own, center.x * 0.5, -150.0)
	var began := Time.get_ticks_msec()
	while _shots_seen < 20 and (Time.get_ticks_msec() - began) / 1000.0 < BATTLE_TIMEOUT:
		main.camera_rig.snap_to(_center(_own_units()), 300.0, 0.7)
		await _frame()
	print("perf-bench: battle after %.0f s (%d shots)" % [(Time.get_ticks_msec() - began) / 1000.0, _shots_seen])
	began = Time.get_ticks_msec()
	while (Time.get_ticks_msec() - began) / 1000.0 < BATTLE_SETTLE:
		main.camera_rig.snap_to(_front(), 150.0, 0.7)
		await _frame()
	for distance in DISTANCES:
		await _measure("battle", distance)
	print("perf-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)
	get_tree().quit()


func _on_combat_event(event: Dictionary) -> void:
	if event.get("type", "") == "shot":
		_shots_seen += 1


func _own_units() -> PackedInt64Array:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var own := PackedInt64Array()
	for i in ids.size():
		if owners[i] == sim_host.local_player() and alive[i] != 0:
			own.append(ids[i])
	return own


## Mean position of `units` (of the latest snapshot).
func _center(units: PackedInt64Array) -> Vector3:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var sum := Vector3.ZERO
	var count := 0
	for i in ids.size():
		if units.has(ids[i]):
			sum += positions[i]
			count += 1
	return sum / maxi(count, 1)


## The own unit nearest to the enemy side (smallest z): where the fighting is.
func _front() -> Vector3:
	var sim_host: SimHost = main.sim_host
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var positions := sim_host.current_positions()
	var best := Vector3(0, 0, INF)
	for i in positions.size():
		if owners[i] == sim_host.local_player() and alive[i] != 0 and positions[i].z < best.z:
			best = positions[i]
	return best if best.z < INF else Vector3.ZERO


func _measure(phase: String, distance: float) -> void:
	var label := "%s, %d m" % [phase, int(distance)]
	var focus := _front() if phase == "battle" else _center(_own_units())
	main.camera_rig.snap_to(focus, distance, 0.7)
	await _frames(WARMUP_FRAMES)
	var viewport_rid := get_viewport().get_viewport_rid()
	var times: Array[float] = []
	var began := Time.get_ticks_usec()
	var last := began
	var sim_usec := 0
	var ticks := 0
	var process := 0.0
	var render := 0.0
	var draw_calls := 0
	var primitives := 0
	while (Time.get_ticks_usec() - began) / 1e6 < SECONDS:
		if phase == "battle":
			# Follow the front slowly, like a player would.
			main.camera_rig.snap_to(main.camera_rig.focus.lerp(_front(), 0.02), distance, 0.7)
		await _frame()
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
		sim_usec += main.sim_host.process_usec()
		ticks += main.sim_host.process_ticks()
		process += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		render += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
		draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		primitives += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
	var seconds := (Time.get_ticks_usec() - began) / 1e6
	var frames := times.size()
	var sorted := times.duplicate()
	sorted.sort()
	DirAccess.make_dir_recursive_absolute("user://perf-bench")
	get_viewport().get_texture().get_image().save_png("user://perf-bench/%s_%d.png" % [phase, int(distance)])
	print("perf-bench: %-14s %6.1f FPS  frame %5.2f ms (p99 %6.2f, worst %6.2f)  sim %5.2f ms (%.2f ticks)  process %5.2f ms  render cpu %5.2f ms  %5d draw calls  %5.2f M primitives" % [
		label, frames / seconds, seconds * 1000.0 / frames, sorted[(frames - 1) * 99 / 100],
		sorted[frames - 1], sim_usec / 1000.0 / frames, float(ticks) / frames,
		process / frames, render / frames, draw_calls / frames, primitives / 1e6 / frames])


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
