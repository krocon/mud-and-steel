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
## ticks, AI, snapshots), render CPU time, draw calls and the script time of
## the big presentation systems (the HUD of prompt 031 included; the battle
## views have a dozen units selected). Screenshots go to `user://perf-bench/`. See docs/PERFORMANCE.md.
##
## `--perf-tilts` (prompt 096) measures the standing views three times: with
## the tilt of the zoom, tilted as flat and as steep as the camera goes.
## `--perf-distances=600,1000,1500` (prompt 103) measures other distances,
## beyond the game's zoom limit too. `--perf-overview` also measures the
## overview of prompt 103.
## `--perf-column` (prompt 104) measures only a column of COLUMN_SIZE own
## tanks driving across open ground (dust, exhaust, tracks), followed at
## 60 and 150 m.
## `--perf-kills` (prompt 105) measures only KILLS own vehicles near the
## camera being destroyed within KILL_SPREAD_S (destroyed events for the
## presentation only: the units stay alive in the simulation), at 150 m
## while the kill sequences run and at 60 m while the wrecks burn.
## `--perf-battery` (prompt 193, `just bench-battery` on
## `data/scenarios/gun_crews.toml`) measures only the own crewed guns firing
## a barrage (a battery of six leFH 18 in the middle of the picture) at 40
## and 150 m, first with the gun crew figures, then without them
## (`gun_crew_view.gd` off), A/B in one run.
## `--perf-dof=<mode>` (prompt 183) measures with that depth of field mode
## ("off", "subtle", "miniature") whatever the setting says (not saved).

const Settings := preload("res://scenes/menu/settings.gd")

const BENCH_SIZE := Vector2i(1920, 1080)
const SECONDS := 6.0
const WARMUP_FRAMES := 30
const DISTANCES: Array[float] = [40.0, 150.0, 600.0]
## Real seconds to wait for the first shots after the advance order.
const BATTLE_TIMEOUT := 150.0
## Real seconds of battle before the battle views are measured.
const BATTLE_SETTLE := 8.0
## Vehicles of the column of `--perf-column` (prompt 104), how far they drive
## and the real seconds they drive before the measurement.
const COLUMN_SIZE := 30
const COLUMN_DRIVE_M := 600.0
const COLUMN_SETTLE := 4.0
const COLUMN_DISTANCES: Array[float] = [60.0, 150.0]
## Vehicles of `--perf-kills` (prompt 105) and the real seconds over which
## their destroyed events are spread.
const KILLS := 40
const KILL_SPREAD_S := 2.0

var main: Node3D
var forced_draws := 0
var _shots_seen := 0
## The driving column of `--perf-column` (followed while measuring).
var _column_units := PackedInt64Array()


func _ready() -> void:
	_run()


func _run() -> void:
	var window := get_window()
	window.size = BENCH_SIZE
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	main.camera_rig.edge_scroll_enabled = false
	main.combat_view.combat_event.connect(_on_combat_event)
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--perf-dof="):
			Settings.set_value("depth_of_field", arg.trim_prefix("--perf-dof="))
	await _frames(10)
	var sim_host: SimHost = main.sim_host
	print("perf-bench: map '%s', render size %s, %s, %d units, %d soldiers, quality '%s'" % [
		sim_host.map_name(), get_viewport().get_visible_rect().size,
		RenderingServer.get_video_adapter_name(), sim_host.unit_ids().size(),
		sim_host.soldier_units().size(), main.atmosphere.quality])
	print("perf-bench: depth of field '%s'" % Settings.get_value("depth_of_field"))
	if "--perf-column" in OS.get_cmdline_user_args():
		await _column()
		get_tree().quit()
		return
	if "--perf-battery" in OS.get_cmdline_user_args():
		await _battery()
		get_tree().quit()
		return
	if "--perf-kills" in OS.get_cmdline_user_args():
		await _kills()
		get_tree().quit()
		return
	var battle_only := "--perf-battle" in OS.get_cmdline_user_args()
	var tilts: Array[String] = [""]
	if "--perf-tilts" in OS.get_cmdline_user_args():
		tilts.append_array(["flat", "steep"])
	var distances: Array[float] = DISTANCES.duplicate()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--perf-distances="):
			distances.clear()
			for part in arg.trim_prefix("--perf-distances=").split(","):
				distances.append(float(part))
			main.camera_rig.max_distance = maxf(main.camera_rig.max_distance, distances.max())
	for tilt in tilts:
		for distance in distances:
			if not battle_only:
				await _measure("standing", distance, tilt)
	if "--perf-overview" in OS.get_cmdline_user_args():
		# Prompt 103: the overview (no shadows, no decoration, steepest tilt).
		main.overview.enter()
		await _frames(60)
		await _measure("overview", main.overview.distance(), "steep")
		main.overview.leave()
		await _frames(60)
	if "--perf-quick" in OS.get_cmdline_user_args():
		get_tree().quit()
		return
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
	# A typical selection of a dozen units for the HUD (prompt 031): unit
	# panel tiles, waypoint lines, minimap rings.
	main.selection.set_selection(Array(_own_units()).slice(0, 12))
	for distance in distances:
		await _measure("battle", distance)
	print("perf-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)
	get_tree().quit()


## A column of COLUMN_SIZE own tanks drives north; the camera follows it.
func _column() -> void:
	var sim_host: SimHost = main.sim_host
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	var column := PackedInt64Array()
	for id in _own_units():
		var type := types[ids.find(id)]
		if column.size() < COLUMN_SIZE and not type.contains("_inf_") and not type.contains("opel"):
			column.append(id)
	var center := _center(column)
	sim_host.move_units(column, center.x, center.z - COLUMN_DRIVE_M)
	var began := Time.get_ticks_msec()
	while (Time.get_ticks_msec() - began) / 1000.0 < COLUMN_SETTLE:
		main.camera_rig.snap_to(_center(column), 150.0, 0.7)
		await _frame()
	_column_units = column
	for distance in COLUMN_DISTANCES:
		await _measure("column", distance)


## KILLS own vehicles nearest the army's center are destroyed (events for
## the presentation only) within KILL_SPREAD_S; measured at 150 m right
## away (the kill sequences), then at 60 m (burning wrecks).
func _kills() -> void:
	var sim_host: SimHost = main.sim_host
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var focus := _nearest_own(_center(_own_units()))
	var vehicles: Array = []
	for id in _own_units():
		var i := ids.find(id)
		if not types[i].contains("_inf_") and sim_host.unit_footprints()[i] > 2.0:
			vehicles.append([positions[i].distance_squared_to(focus), id, positions[i]])
	vehicles.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	vehicles = vehicles.slice(0, KILLS)
	_column_units = PackedInt64Array(vehicles.map(func(v: Array) -> int: return v[1]))
	main.camera_rig.snap_to(_center(_column_units), 150.0, 0.7)
	await _frames(WARMUP_FRAMES)
	var began := Time.get_ticks_msec()
	var next := 0
	while next < vehicles.size():
		if (Time.get_ticks_msec() - began) / 1000.0 >= KILL_SPREAD_S * next / vehicles.size():
			var v: Array = vehicles[next]
			main.combat_view.combat_event.emit({"type": "destroyed", "unit": v[1], "by": -1,
				"position": v[2], "tick": sim_host.current_tick()})
			next += 1
		await _frame()
	await _measure("kills", 150.0)
	await _measure("kills", 60.0)


## The own crewed guns fire at the enemy start; measured with and without
## the crew figures (prompt 193).
func _battery() -> void:
	var sim_host: SimHost = main.sim_host
	var guns := PackedInt64Array()
	var battery := PackedInt64Array()
	var crews := sim_host.unit_crews()
	var ids := sim_host.unit_ids()
	var types := sim_host.unit_type_ids()
	var enemy := Vector3.ZERO
	for i in ids.size():
		if sim_host.unit_owners()[i] != sim_host.local_player():
			enemy = sim_host.current_positions()[i]
		elif crews.size() > i * 4 and crews[i * 4] > 0:
			guns.append(ids[i])
			if types[i] == "de_lefh18":
				battery.append(ids[i])
	_column_units = battery if not battery.is_empty() else guns
	sim_host.barrage(guns, enemy.x, enemy.z, 60, 200, false)
	var began := Time.get_ticks_msec()
	while _shots_seen < 6 and (Time.get_ticks_msec() - began) / 1000.0 < BATTLE_TIMEOUT:
		await _frame()
	print("perf-bench: battery of %d guns (%d crewed in all) fired after %.0f s" % [
		battery.size(), guns.size(), (Time.get_ticks_msec() - began) / 1000.0])
	for crews_on in [true, false]:
		main.gun_crew_view.enabled = crews_on
		print("perf-bench: gun crews %s" % ("on" if crews_on else "off"))
		for distance in [40.0, 150.0]:
			await _measure("battery_" + ("on" if crews_on else "off"), distance)
			print("perf-bench: %-14s gun crews %.2f ms, %s figures (near, mid, still, hidden)" % [
				"", main.gun_crew_view.process_usec / 1000.0, main.gun_crew_view.counts])


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


## Position of the living own unit nearest to `point` (the army's center
## may lie in the gap between tanks and infantry).
func _nearest_own(point: Vector3) -> Vector3:
	var sim_host: SimHost = main.sim_host
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var positions := sim_host.current_positions()
	var best := point
	var best_d := INF
	for i in positions.size():
		if owners[i] == sim_host.local_player() and alive[i] != 0:
			var d := positions[i].distance_squared_to(point)
			if d < best_d:
				best_d = d
				best = positions[i]
	return best


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


## `tilt`: "" for the tilt of the zoom, "flat" or "steep" for the limits.
func _measure(phase: String, distance: float, tilt := "") -> void:
	var label := "%s, %d m" % [phase, int(distance)]
	var focus := _front() if phase == "battle" else _nearest_own(_center(_own_units()))
	if phase == "column" or phase == "kills" or phase.begins_with("battery"):
		focus = _center(_column_units)
	main.camera_rig.snap_to(focus, distance, 0.7)
	main.camera_rig.snap_tilt({"": 0.0, "flat": -PI, "steep": PI}[tilt])
	if not tilt.is_empty():
		label = "%s, %d m %s" % [phase, int(distance), tilt]
		phase += "_" + tilt
	await _frames(WARMUP_FRAMES)
	var viewport_rid := get_viewport().get_viewport_rid()
	var times: Array[float] = []
	var began := Time.get_ticks_usec()
	var last := began
	var sim_usec := 0
	var ticks := 0
	var render := 0.0
	var draw_calls := 0
	var primitives := 0
	# GDScript per system (microseconds): units, infantry, combat view and
	# its event effects, effects, decoration, terrain chunks.
	var parts := [0, 0, 0, 0, 0, 0, 0, 0, 0]
	while (Time.get_ticks_usec() - began) / 1e6 < SECONDS:
		if phase == "battle":
			# Follow the front slowly, like a player would.
			main.camera_rig.snap_to(main.camera_rig.focus.lerp(_front(), 0.02), distance, 0.7)
		elif phase == "column":
			main.camera_rig.snap_to(_center(_column_units), distance, 0.7)
		await _frame()
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
		sim_usec += main.sim_host.process_usec()
		ticks += main.sim_host.process_ticks()
		render += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
		draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		primitives += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
		parts[0] += main.draw_usec
		parts[1] += main.infantry_view.process_usec
		parts[2] += main.combat_view.process_usec
		parts[3] += main.effects.process_usec + (main.ground_tracks.process_usec if main.get("ground_tracks") != null else 0)
		parts[4] += main.map_objects.process_usec
		parts[5] += main.terrain.last_update_usec
		# Sound (prompt 029; its event handling counts under the combat view).
		parts[6] += main.battle_sound.process_usec if main.battle_sound != null else 0
		parts[7] += main.hud.usec
		parts[8] += main.unit_highlight.process_usec
	var seconds := (Time.get_ticks_usec() - began) / 1e6
	var frames := times.size()
	var sorted := times.duplicate()
	sorted.sort()
	DirAccess.make_dir_recursive_absolute("user://perf-bench")
	get_viewport().get_texture().get_image().save_png("user://perf-bench/%s_%d.png" % [phase, int(distance)])
	print("perf-bench: %-14s %6.1f FPS  frame %5.2f ms (p99 %6.2f, worst %6.2f)  sim %5.2f ms (%.2f ticks)  render cpu %5.2f ms  %5d draw calls  %5.2f M primitives" % [
		label, frames / seconds, seconds * 1000.0 / frames, sorted[(frames - 1) * 99 / 100],
		sorted[frames - 1], sim_usec / 1000.0 / frames, float(ticks) / frames,
		render / frames, draw_calls / frames, primitives / 1e6 / frames])
	print("perf-bench: %-14s scripts ms: units %.2f  infantry %.2f  combat view %.2f  effects %.2f  decoration %.2f  terrain %.2f  sound %.2f  hud %.2f  unit markers %.2f" % [
		"", parts[0] / 1000.0 / frames, parts[1] / 1000.0 / frames, parts[2] / 1000.0 / frames,
		parts[3] / 1000.0 / frames, parts[4] / 1000.0 / frames, parts[5] / 1000.0 / frames,
		parts[6] / 1000.0 / frames, parts[7] / 1000.0 / frames, parts[8] / 1000.0 / frames])


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
