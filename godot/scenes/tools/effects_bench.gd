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
## wrecks on top, then two rocket salvos at once (prompt 076: two own units
## get the battery position's model and fire 24 rockets each through
## `SimHost.debug_rocket`) from 150 m and 60 m, with the rocket effects and,
## for comparison, without them (balls, as before). `just effects-bench`;
## `-- --salvos` measures only the idle view and the salvos. A view with
## frames drawn by force (window not drawable, e.g. a locked screen) is
## marked invalid. `-- --muzzles` (prompt 077) measures only the idle view
## and 40 cannon shots a second (fired round robin by every vehicle of the
## map, calibers 50 to 122 mm, plus two 20 mm autocannons firing bursts) from
## 150 m and from 60 m.
##
## `--muzzle-series=<dir>` (prompt 077, `just muzzle-shots`, needs
## `--fixed-fps 30`) gives one own unit the model of each gun class in turn
## and fires it: every frame of the first half second and then four
## pictures a second until the powder smoke is gone, `<run>_<ms>.png`; per
## run a crop around the muzzle in `series.txt` for the tile sheets of
## `tools/muzzle-series.sh`. Runs: the reference view (88 mm, oblique from
## above, as the reference picture), 20 / 37 / 75 / 88 / 122 mm from the
## side, a 76 mm gun without muzzle brake, a howitzer firing indirectly with
## a raised barrel, 75 mm at 15 / 110 / 350 m, 88 mm at evening and night,
## 75 mm in desert and winter, and last a real fight of both sides
## (`battle_<n>.png`, every third frame). `--only=<run>,<run>` records some.

const SHOT_SIZE := Vector2i(1600, 900)
const BENCH_SIZE := Vector2i(1920, 1080)
const BARRAGE := 50
const BARRAGE_RADIUS := 70.0
const BARRAGE_SECONDS := 2.0
const BENCH_SECONDS := 6.0
const WARMUP_FRAMES := 60
## Two salvos (prompt 076): rockets each, ms apart, caliber, range.
const SALVO := 24
const SALVO_RIPPLE_S := 0.25
const SALVO_CALIBER := 132.0
const BaseView := preload("res://scenes/world/base_view.gd")
## Muzzle views (prompt 077): cannon shots a second, their calibers in turn,
## and 20 mm rounds a second (two autocannons in bursts).
const MUZZLE_RATE := 40.0
const MUZZLE_CALIBERS: Array[float] = [75.0, 88.0, 50.0, 122.0, 76.2, 37.0, 85.0, 75.0]
const AUTOCANNON_RATE := 10.0
## Muzzle series (prompt 077): seconds after the shot, and the runs: model,
## caliber, camera distance (m), crop (m across), view, daytime, biome.
const MUZZLE_SERIES_SECONDS := 4.0
const MUZZLE_RUNS: Array[Dictionary] = [
	{"name": "reference", "model": "de_tiger1_e", "caliber": 88.0, "distance": 40.0, "crop_m": 30.0, "view": "reference"},
	{"name": "c20", "model": "de_pz2_f", "caliber": 20.0, "crop_m": 8.0, "burst": 5},
	{"name": "c37", "model": "de_pz38t_e", "caliber": 37.0, "crop_m": 10.0},
	{"name": "c75", "model": "de_pz4_h", "caliber": 75.0, "crop_m": 16.0},
	{"name": "c76_no_brake", "model": "su_t34_76_1943", "caliber": 76.2, "crop_m": 16.0},
	{"name": "c88", "model": "de_tiger1_e", "caliber": 88.0, "crop_m": 20.0},
	{"name": "c122", "model": "su_is2", "caliber": 122.0, "crop_m": 24.0},
	{"name": "howitzer", "model": "su_m30", "caliber": 122.0, "crop_m": 24.0, "indirect": true},
	{"name": "c75_15m", "model": "de_pz4_h", "caliber": 75.0, "distance": 15.0, "crop_m": 12.0},
	{"name": "c75_110m", "model": "de_pz4_h", "caliber": 75.0, "distance": 110.0, "crop_m": 20.0},
	{"name": "c75_350m", "model": "de_pz4_h", "caliber": 75.0, "distance": 350.0, "crop_m": 40.0},
	{"name": "c88_evening", "model": "de_tiger1_e", "caliber": 88.0, "crop_m": 20.0, "daytime": "evening"},
	{"name": "c88_night", "model": "de_tiger1_e", "caliber": 88.0, "crop_m": 20.0, "daytime": "night"},
	{"name": "c75_desert", "model": "de_pz4_h", "caliber": 75.0, "crop_m": 16.0, "biome": "desert"},
	{"name": "c75_winter", "model": "de_pz4_h", "caliber": 75.0, "crop_m": 16.0, "biome": "winter"},
	{"name": "battle"},
]
## The fight of the run "battle": seconds recorded.
const BATTLE_SECONDS := 10.0

var main: Node3D
var forced_draws := 0
var _salvo_target := Vector3.ZERO
var _dir := ""
var _shot_count := 0
## Muzzle views (prompt 077): shots fired so far in this view.
var _muzzle_shots := 0


func _ready() -> void:
	_run()


func _run() -> void:
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--effects-shots"):
			_dir = arg.trim_prefix("--effects-shots").trim_prefix("=")
		if arg.begins_with("--muzzle-series="):
			_dir = arg.trim_prefix("--muzzle-series=")
	if Array(OS.get_cmdline_user_args()).any(func(a: String) -> bool: return a.begins_with("--muzzle-series")):
		await _muzzle_series()
	elif "--effects-bench" in OS.get_cmdline_user_args():
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


## One rocket from each of the two batteries at the enemy.
func _salvo_round(own: Array[int], enemies: Array[int]) -> void:
	var target := _salvo_target
	for id in own.slice(0, 2):
		var a := randf() * TAU
		var r := 60.0 * sqrt(randf())
		main.sim_host.debug_rocket(id, target.x + cos(a) * r, target.z + sin(a) * r, SALVO_CALIBER)


## Between the batteries and a third of the way to the enemy.
func _salvo_focus(own: Array[int]) -> Vector3:
	var at := (_position_of(own[0]) + _position_of(own[1])) * 0.5
	var to := _salvo_target - at
	return at + to * 0.2


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
	# Two own units become rocket batteries (prompt 076).
	# Open ground between the sides (where the barrages land), so the
	# impacts destroy nothing and leave no burning wrecks.
	_salvo_target = center
	for id in own.slice(0, 2):
		BaseView.attach_model(main._unit_nodes[id], "base_rocket_battery", main.atmosphere.preset.id)
		main.effects._sockets.erase(id)
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
		var views: Array[Dictionary] = [{"name": "idle 150 m", "distance": 150.0, "barrage": false},
				{"name": "50 explosions 150 m", "distance": 150.0, "barrage": true},
				{"name": "50 explosions 60 m", "distance": 60.0, "barrage": true},
				{"name": "+24 burning wrecks", "distance": 150.0, "barrage": true, "wrecks": true},
				{"name": "2 salvos, balls 150 m", "distance": 150.0, "barrage": false, "salvo": true, "plain": true},
				{"name": "2 salvos 150 m", "distance": 150.0, "barrage": false, "salvo": true},
				{"name": "2 salvos, balls 60 m", "distance": 60.0, "barrage": false, "salvo": true, "plain": true},
				{"name": "2 salvos 60 m", "distance": 60.0, "barrage": false, "salvo": true},
				{"name": "40 shots/s 150 m", "distance": 150.0, "barrage": false, "muzzles": true},
				{"name": "40 shots/s 60 m", "distance": 60.0, "barrage": false, "muzzles": true}]
		# `--salvos`: only the idle view and the salvos (a shorter run).
		if "--salvos" in OS.get_cmdline_user_args():
			views = views.filter(func(v: Dictionary) -> bool: return v.get("salvo", false) or not v["barrage"] and not v.get("muzzles", false))
		# `--muzzles`: only the idle view and the cannon shots (prompt 077).
		if "--muzzles" in OS.get_cmdline_user_args():
			views = views.filter(func(v: Dictionary) -> bool: return v.get("muzzles", false) or not v["barrage"] and not v.get("salvo", false))
		for view in views:
			var salvo: bool = view.get("salvo", false)
			var muzzles: bool = view.get("muzzles", false)
			_muzzle_shots = 0
			if salvo:
				# Without the wrecks of the view before.
				for wreck in fx._wrecks:
					(wreck.root as Node).queue_free()
				fx._wrecks.clear()
			fx.rockets.enabled = not view.get("plain", false)
			main.camera_rig.snap_to(_salvo_focus(own) if salvo else center, view["distance"], 0.5)
			if view.get("wrecks", false):
				for k in 24:
					var angle := k * TAU / 24.0
					fx._start_wreck(100000 + k, _ground(center + Vector3(cos(angle), 0, sin(angle)) * 40.0))
			# A first barrage in the warm-up compiles the pipelines.
			if view["barrage"]:
				_barrage(center)
			await _frames(WARMUP_FRAMES)
			var forced_before := forced_draws
			var began := Time.get_ticks_usec()
			var last := began
			var next_barrage := 0.0
			var frames := 0
			var worst := 0.0
			var cpu := 0.0
			var draw_calls := 0
			var fired := 0
			var max_trails := 0
			while (Time.get_ticks_usec() - began) / 1e6 < BENCH_SECONDS:
				var t := (Time.get_ticks_usec() - began) / 1e6
				if view["barrage"] and t >= next_barrage:
					_barrage(center)
					next_barrage += BARRAGE_SECONDS
				if salvo and fired < SALVO and t >= fired * SALVO_RIPPLE_S:
					_salvo_round(own, enemies)
					fired += 1
				if muzzles:
					_muzzle_round(t, own + enemies)
				max_trails = maxi(max_trails, fx.rockets.trails_live())
				await _frame()
				var now := Time.get_ticks_usec()
				worst = maxf(worst, (now - last) / 1000.0)
				last = now
				frames += 1
				cpu += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
				draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
			var seconds := (Time.get_ticks_usec() - began) / 1e6
			print("effects-bench: %-6s %-22s %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  %4d draw calls%s" % [
				level, view["name"], frames / seconds, seconds * 1000.0 / frames, worst, cpu / frames, draw_calls / frames,
				("  %d trails" % max_trails if salvo else "")
				+ ("  [%d frames forced: window not drawable, invalid]" % (forced_draws - forced_before)
					if forced_draws > forced_before else "")])
			if salvo:
				# Let the smoke of this salvo go before the next one.
				var waited := Time.get_ticks_usec()
				while (Time.get_ticks_usec() - waited) / 1e6 < float(fx._settings.fx_trail_life) + 1.0:
					await _frame()
		# Let the wrecks of this level burn out of the next one's budget.
		for wreck in fx._wrecks:
			(wreck.root as Node).queue_free()
		fx._wrecks.clear()
	print("effects-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)


## Fires the cannon shots due at `t` s of a muzzle view: MUZZLE_RATE a
## second round robin over `shooters` (the two rocket batteries excluded),
## and AUTOCANNON_RATE 20 mm rounds a second from two of them.
func _muzzle_round(t: float, shooters: Array[int]) -> void:
	var guns := shooters.slice(2).filter(func(id: int) -> bool: return not main.effects._is_infantry(id))
	var due := int(t * MUZZLE_RATE)
	while _muzzle_shots < due:
		var id: int = guns[_muzzle_shots % guns.size()]
		var caliber: float = MUZZLE_CALIBERS[_muzzle_shots % MUZZLE_CALIBERS.size()]
		main.effects.on_combat_event({"type": "shot", "unit": id, "position": _muzzle(id), "caliber_mm": caliber,
				"weapon": 0, "projectile": -1})
		if _muzzle_shots % int(MUZZLE_RATE / AUTOCANNON_RATE) == 0:
			for k in 2:
				var gun: int = guns[(k * 7 + 3) % guns.size()]
				main.effects.on_combat_event({"type": "shot", "unit": gun, "position": _muzzle(gun), "caliber_mm": 20.0,
						"weapon": 0, "projectile": -1})
		_muzzle_shots += 1


# --- Muzzle series (prompt 077) ---------------------------------------------------


func _muzzle_series() -> void:
	if _dir.is_empty():
		_dir = "user://muzzle-series"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	(main.get_node("Hud") as CanvasLayer).visible = false
	await _frames(10)
	var only := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--only="):
			only = arg.trim_prefix("--only=")
	var shooter := _ids_of(true)[0]
	var crops := PackedStringArray()
	for run: Dictionary in MUZZLE_RUNS:
		if not only.is_empty() and not (str(run.name) in only.split(",")):
			continue
		if run.name == "battle":
			await _muzzle_battle()
			continue
		var biome := str(run.get("biome", "forest"))
		if main.atmosphere.preset.id != biome:
			main.set_biome(biome)
		main.atmosphere.set_weather("clear", str(run.get("daytime", "day")))
		_give_model(shooter, str(run.model))
		await _frames(5)
		var at := _position_of(shooter)
		var forward := _forward(shooter)
		var distance := float(run.get("distance", 35.0))
		var caliber := float(run.caliber)
		var indirect: bool = run.get("indirect", false)
		var target := _ground(at + forward * 150.0)
		if indirect:
			# A first round lays the gun (its barrel rises with the shell's
			# elevation); the series shows the second one.
			main.sim_host.debug_rocket(shooter, target.x, target.z, caliber)
			await _frames(120)
		var muzzle := _muzzle(shooter)
		if str(run.get("view", "side")) == "reference":
			# As the reference picture: oblique from above and behind the
			# left, the gun firing up and to the right.
			main.camera_rig.pitch_near_deg = 32.0
			main.camera_rig.pitch_far_deg = 32.0
			await _view(at + forward * 10.0, distance, _yaw_of(forward.rotated(Vector3.UP, 0.9)))
		else:
			main.camera_rig.pitch_near_deg = 14.0
			main.camera_rig.pitch_far_deg = 14.0
			# From the side away from the other own units, so none stands
			# between the camera and the gun.
			var side := forward.cross(Vector3.UP).normalized()
			var away := at - _centroid(_ids_of(true))
			var look := -side if side.dot(away) > 0.0 else side
			await _view(at + forward * 4.0 + Vector3.UP * 1.5, distance, _yaw_of(look))
		await _frames(30)
		crops.append("%s %s" % [run.name, _muzzle_crop(muzzle + forward * float(run.crop_m) * 0.2, float(run.crop_m))])
		# Series times as `series_times(4.0, 0.5)` in tools/explosion_series.py.
		var frames: Array[int] = []
		var times_ms: Array[int] = []
		for f in 15:
			frames.append(f)
			times_ms.append(int(round(f * 1000.0 / 30.0)))
		var t := 0.5
		while t <= MUZZLE_SERIES_SECONDS + 0.001:
			frames.append(int(round(t * 30.0)))
			times_ms.append(int(round(t * 1000.0)))
			t += 0.25
		var frame := 0
		var burst := int(run.get("burst", 1))
		var fired := 0
		for n in frames.size():
			var wanted := frames[n]
			while frame <= wanted:
				if fired < burst and frame == fired * 3:
					if indirect:
						main.sim_host.debug_rocket(shooter, target.x, target.z, caliber)
					else:
						main.effects.on_combat_event({"type": "shot", "unit": shooter, "position": _muzzle(shooter),
								"caliber_mm": caliber, "weapon": 0, "projectile": -1})
					fired += 1
				await _frame()
				frame += 1
			var image := get_viewport().get_texture().get_image()
			image.save_png("%s/%s_%05d.png" % [_dir, run.name, times_ms[n]])
			_shot_count += 1
		# Let the smoke go before the next run.
		await _frames(60)
	var file := FileAccess.open(_dir + "/series.txt", FileAccess.WRITE)
	file.store_string("\n".join(crops) + "\n")
	print("muzzle-series: %d pictures in %s; counts %s" % [_shot_count, ProjectSettings.globalize_path(_dir), main.effects.counts])


## A real fight (last run): both sides fire at will and close in; once the
## first shot fell, every third frame for BATTLE_SECONDS is saved as
## `battle_<n>.png` (tools/muzzle-series.sh makes a video of them). Real
## shells: tracers from the muzzle, recoil, raised barrels.
func _muzzle_battle() -> void:
	main.set_biome("forest")
	main.atmosphere.set_weather("clear", "day")
	var host: SimHost = main.sim_host
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	host.set_hold_fire(PackedInt64Array(own + enemies), false)
	host.attack_unit(PackedInt64Array(own), enemies[0])
	host.attack_unit(PackedInt64Array(enemies), own[0])
	var view: Node3D = main.combat_view
	var shots_before: int = view.shots
	var waited := 0
	main.camera_rig.pitch_near_deg = 35.0
	main.camera_rig.pitch_far_deg = 35.0
	while view.shots == shots_before and waited < 30 * 60:
		await _frame()
		waited += 1
	var a := _centroid(own)
	var b := _centroid(enemies)
	await _view(a.lerp(b, 0.12), 60.0, _yaw_of((b - a).normalized().rotated(Vector3.UP, 1.2)))
	var n := 0
	for f in int(BATTLE_SECONDS * 30.0):
		await _frame()
		if f % 3 == 0:
			get_viewport().get_texture().get_image().save_png("%s/battle_%04d.png" % [_dir, n])
			n += 1
	print("muzzle-series: battle %d pictures, %d shots, %d recoils, %d tracers from the muzzle" % [
		n, view.shots - shots_before, main.effects.recoils, view.tracers_from_muzzle])


## Unit `id` gets the catalog model `type_id`; caches of the views that hold
## parts of the old model are dropped.
func _give_model(id: int, type_id: String) -> void:
	var node: Node3D = main._unit_nodes[id]
	main._use_catalog_model(node, type_id, id)
	main.effects._sockets.erase(id)
	main.combat_view._turrets.erase(id)
	if main.effects.has_method("forget_unit"):
		main.effects.forget_unit(id)


## "x,y,w,h" of a square crop that shows `meters` across around `at`.
func _muzzle_crop(at: Vector3, meters: float) -> String:
	var camera: Camera3D = main.camera
	var center := camera.unproject_position(at)
	var right := camera.global_basis.x
	var a := camera.unproject_position(at - right * meters * 0.5)
	var b := camera.unproject_position(at + right * meters * 0.5)
	var size := clampi(int(a.distance_to(b)), 32, SHOT_SIZE.y)
	var x := clampi(int(center.x) - size / 2, 0, SHOT_SIZE.x - size)
	var y := clampi(int(center.y) - size / 2, 0, SHOT_SIZE.y - size)
	return "%d,%d,%d,%d" % [x, y, size, size]


## Camera yaw (rts_camera.gd) that looks along the horizontal `direction`.
func _yaw_of(direction: Vector3) -> float:
	return atan2(-direction.x, -direction.z)


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
