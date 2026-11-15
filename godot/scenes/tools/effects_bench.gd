extends Node
## Effects test scene (prompt 019), started by `main.gd`:
##
##     godot --path godot --fixed-fps 30 -- --effects-shots=<dir>
##     godot --path godot -- --effects-bench
##
## Both run on the demo map with every unit holding fire and trigger the
## effects directly through `effects.gd` (the same functions the combat
## events call), so every effect appears at a known time and place.
##
## `--effects-shots=<dir>` shows every effect in fixed views and saves PNG
## screenshots at fixed frames after the trigger: muzzle flash and powder
## smoke, small arms, shell impacts on the ground and in water, hits on a
## tank (ricochet, penetration), an explosion, a burning wreck, dust trails,
## scorch marks and a barrage of 50 explosions; then dust, impacts and an
## explosion again in desert and winter. Run it with `--fixed-fps 30`, so
## the effects are at the same stage in every run. `just effects-shots <dir>`.
##
## `--effects-bench` sets the window to 1920 x 1080, turns V-Sync off and, in
## every graphics quality, measures the frame rate with 50 explosions at once
## (a new barrage every 2 s) from 150 m and from 60 m, then with 24 burning
## wrecks on top. `just effects-bench`.

const SHOT_SIZE := Vector2i(1600, 900)
const BENCH_SIZE := Vector2i(1920, 1080)
const BARRAGE := 50
const BARRAGE_RADIUS := 70.0
const BARRAGE_SECONDS := 2.0
const BENCH_SECONDS := 6.0
const WARMUP_FRAMES := 60

var main: Node3D
var forced_draws := 0
var _dir := ""
var _shot_count := 0


func _ready() -> void:
	_run()


func _run() -> void:
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--effects-shots"):
			_dir = arg.trim_prefix("--effects-shots").trim_prefix("=")
	if "--effects-bench" in OS.get_cmdline_user_args():
		await _bench()
	else:
		await _shots()
	get_tree().quit()


# --- Shots ----------------------------------------------------------------------


func _shots() -> void:
	if _dir.is_empty():
		_dir = "user://effects-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	await _frames(10)
	var fx: Node3D = main.effects
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var tank := own[0]
	var tank_pos := _position_of(tank)

	# Muzzle flash and powder smoke of the main gun, seen from the side.
	await _view(tank_pos, 24.0, _side_yaw(tank))
	fx.on_combat_event({"type": "shot", "unit": tank, "position": _muzzle(tank), "caliber_mm": 75.0})
	await _capture("muzzle", [1, 3, 12, 40])

	# Machine gun: small flash and dust puffs where the rounds land.
	fx.on_combat_event({"type": "shot", "unit": tank, "position": _muzzle(tank, 1), "caliber_mm": 7.92})
	var ahead := tank_pos + _forward(tank) * 18.0
	for k in 4:
		fx.ground_hit(_ground(ahead + Vector3(k * 1.5 - 2.0, 0, k * 0.7)), 7.92)
	await _capture("small_arms", [1, 6])

	# Shell impacts on the ground: earth fountain, dust, scorch mark.
	var field := tank_pos + _forward(tank) * 40.0
	await _view(field, 38.0, 0.6)
	fx.ground_hit(_ground(field), 75.0)
	await _frames(6)
	fx.ground_hit(_ground(field + Vector3(7, 0, 3)), 75.0)
	await _frames(6)
	fx.ground_hit(_ground(field + Vector3(-6, 0, -4)), 150.0)
	await _capture("ground_hits", [2, 8, 20, 50])

	# Water: spray instead of earth.
	var water := _find_terrain(main.effects.TERRAIN_WATER_SHALLOW, field)
	if water != Vector3.INF:
		await _view(water, 30.0, 0.6)
		fx.ground_hit(water, 75.0)
		await _capture("water", [4, 14])

	# Hits on a tank: a ricochet (sparks), then a penetration.
	var enemy := enemies[0]
	var enemy_pos := _position_of(enemy)
	await _view(enemy_pos, 22.0, _side_yaw(enemy))
	fx.unit_hit(enemy_pos + Vector3.UP * 1.6, 75.0, false)
	await _capture("ricochet", [1, 5])
	fx.unit_hit(enemy_pos + Vector3.UP * 1.2, 88.0, true)
	await _capture("penetration", [2, 8])

	# Explosion and the burning wreck afterwards.
	var victim := enemies[1]
	var victim_pos := _position_of(victim)
	await _view(victim_pos, 34.0, 0.5)
	fx.destroyed(victim, victim_pos)
	await _capture("explosion", [2, 5, 10, 24, 45])
	await _frames(240)
	await _capture("burning_wreck", [0])
	await _view(victim_pos, 90.0, 0.5)
	await _capture("burning_wreck_far", [2])

	# Scorch marks of the shells from above.
	await _view(field, 28.0, 0.6)
	await _capture("scorch_marks", [2])

	# Dust trails and exhaust: the own tanks drive off.
	await _dust_view("forest")

	# A barrage of 50 explosions.
	var center := (tank_pos + enemy_pos) * 0.5
	await _view(center, 170.0, 0.5)
	_barrage(center)
	await _capture("barrage_50", [3, 10, 40])

	# Other biomes: dust, impacts and an explosion.
	for biome in ["desert", "winter"]:
		main.set_biome(biome)
		await _frames(5)
		await _dust_view(biome)
		var spot := _ground(_position_of(own[0]) + _forward(own[0]) * 30.0)
		await _view(spot, 40.0, 0.6)
		fx.ground_hit(spot, 75.0)
		fx.explosion(_ground(spot + Vector3(9, 0, -6)), 1.0)
		await _capture("impacts_%s" % biome, [6, 24])
	print("effects-shots: %d screenshots in %s; forced draws %d; counts %s" % [
		_shot_count, ProjectSettings.globalize_path(_dir), forced_draws, fx.counts])


## Sends the own tanks off and follows them from behind.
func _dust_view(biome: String) -> void:
	var own := _ids_of(true)
	var from := _centroid(own)
	var goal := from + Vector3(-60, 0, -120)
	main.sim_host.move_units(PackedInt64Array(own), goal.x, goal.z)
	await _frames(60)
	await _view(_centroid(own), 45.0, 2.6)
	await _frames(40)
	main.camera_rig.snap_to(_centroid(own), 45.0, 2.6)
	await _capture("dust_%s" % biome, [1])
	main.sim_host.stop_units(PackedInt64Array(own))


func _barrage(center: Vector3) -> void:
	for k in BARRAGE:
		var angle := k * 2.399963
		var r := BARRAGE_RADIUS * sqrt((k + 0.5) / BARRAGE)
		main.effects.explosion(_ground(center + Vector3(cos(angle) * r, 0, sin(angle) * r)), 1.0)


func _view(focus: Vector3, distance: float, yaw: float) -> void:
	main.camera_rig.snap_to(focus, distance, yaw)
	await _frames(3)


## Saves screenshots `offsets` frames after now (frame 0 = the next one).
func _capture(name: String, offsets: Array) -> void:
	var frame := 0
	for offset: int in offsets:
		while frame <= offset:
			await _frame()
			frame += 1
		var image := get_viewport().get_texture().get_image()
		var path := "%s/%s_%02d.png" % [_dir, name, offset]
		image.save_png(path)
		_shot_count += 1
		print("effects-shots: %s" % path.get_file())


# --- Bench ----------------------------------------------------------------------


func _bench() -> void:
	var window := get_window()
	window.size = BENCH_SIZE
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	await _frames(10)
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var center := (_centroid(own) + _centroid(enemies)) * 0.5
	print("effects-bench: render size %s, %s, biome '%s'" % [
		get_viewport().get_visible_rect().size, RenderingServer.get_video_adapter_name(), main.atmosphere.preset.id])
	for level: String in main.GraphicsQuality.LEVELS:
		main.atmosphere.set_quality(level)
		await _frames(5)
		var fx: Node3D = main.effects
		var emitters := 0
		var particles := 0
		for pool: Array in fx._pools.values():
			for p: GPUParticles3D in pool:
				emitters += 1
				particles += p.amount
		print("effects-bench: %s: %d pooled emitters, %d particles at most, %d lights, %d decals" % [
			level, emitters, particles, fx._settings.fx_lights, fx._settings.fx_decals])
		for view in [{"name": "idle 150 m", "distance": 150.0, "barrage": false},
				{"name": "50 explosions 150 m", "distance": 150.0, "barrage": true},
				{"name": "50 explosions 60 m", "distance": 60.0, "barrage": true},
				{"name": "+24 burning wrecks", "distance": 150.0, "barrage": true, "wrecks": true}]:
			main.camera_rig.snap_to(center, view["distance"], 0.5)
			if view.get("wrecks", false):
				for k in 24:
					var angle := k * TAU / 24.0
					fx._start_wreck(100000 + k, _ground(center + Vector3(cos(angle), 0, sin(angle)) * 40.0))
			# A first barrage in the warm-up compiles the pipelines.
			if view["barrage"]:
				_barrage(center)
			await _frames(WARMUP_FRAMES)
			var began := Time.get_ticks_usec()
			var last := began
			var next_barrage := 0.0
			var frames := 0
			var worst := 0.0
			var cpu := 0.0
			var draw_calls := 0
			while (Time.get_ticks_usec() - began) / 1e6 < BENCH_SECONDS:
				var t := (Time.get_ticks_usec() - began) / 1e6
				if view["barrage"] and t >= next_barrage:
					_barrage(center)
					next_barrage += BARRAGE_SECONDS
				await _frame()
				var now := Time.get_ticks_usec()
				worst = maxf(worst, (now - last) / 1000.0)
				last = now
				frames += 1
				cpu += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
				draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
			var seconds := (Time.get_ticks_usec() - began) / 1e6
			print("effects-bench: %-6s %-20s %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  %4d draw calls" % [
				level, view["name"], frames / seconds, seconds * 1000.0 / frames, worst, cpu / frames, draw_calls / frames])
		# Let the wrecks of this level burn out of the next one's budget.
		for wreck in fx._wrecks:
			(wreck.root as Node).queue_free()
		fx._wrecks.clear()
	print("effects-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)


# --- Helpers --------------------------------------------------------------------


func _ids_of(own: bool) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var result: Array[int] = []
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own:
			result.append(ids[i])
	return result


func _position_of(id: int) -> Vector3:
	return (main._unit_nodes[id] as Node3D).global_position


func _centroid(ids: Array[int]) -> Vector3:
	var sum := Vector3.ZERO
	for id in ids:
		sum += _position_of(id)
	return sum / ids.size()


func _forward(id: int) -> Vector3:
	return -(main._unit_nodes[id] as Node3D).global_basis.z


## Camera yaw that looks at the unit's left side.
func _side_yaw(id: int) -> float:
	return (main._unit_nodes[id] as Node3D).rotation.y + PI * 0.5


func _muzzle(id: int, which := 0) -> Vector3:
	var model := (main._unit_nodes[id] as Node3D).get_node("Model")
	var name := "Socket_Muzzle" if which == 0 else "Socket_Muzzle_%02d" % (which + 1)
	var socket := model.find_child(name, true, false) as Node3D
	return socket.global_position if socket != null else _position_of(id) + Vector3.UP * 2.0


func _ground(point: Vector3) -> Vector3:
	return Vector3(point.x, main.sim_host.height_at(point.x, point.z), point.z)


## Nearest sample of a terrain id within 300 m (spiral search), or INF.
func _find_terrain(terrain: int, near: Vector3) -> Vector3:
	for r in range(0, 300, 4):
		for k in 16:
			var angle := k * TAU / 16.0
			var p := near + Vector3(cos(angle), 0, sin(angle)) * r
			if main.sim_host.terrain_at(p.x, p.z) == terrain:
				return _ground(p)
	return Vector3.INF


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame
