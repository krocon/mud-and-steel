extends Node
## Big blasts and the heavy strike in pictures (prompt 054), started by
## `main.gd`:
##
##     godot --path godot --fixed-fps 30 -- --blast-shots=<dir>
##
## On the demo map with every unit holding fire and the fog of war off, it
## triggers the effects directly (the functions the events call) and saves
## PNG screenshots at fixed frames: a big blast in the open by day (shock
## dome, pressure and dust ring, flash), one next to the village, the same
## by night (the flash lights up ground and houses), a tank whose
## ammunition explodes (the turret flies), and a heavy strike: marker smoke
## and area ring, then the salvo, by day and by night. `just blast-shots`.
##
## With `--mode=base --strike-live` it plays a real strike instead: on the
## base skirmish (enemy passive) at high speed it builds depot, works and
## rocket battery position, waits for the unlock, calls the strike on the
## enemy headquarters through `SimHost.call_strike` and records the marked
## target with the countdown and the salvo; it fails (exit code 1) if the
## strike, the siren or the launch do not show, or if fewer than 20 rockets
## got the launch effects of prompt 076. `just strike-shots`.
##
## With `--salvo-series` (prompt 076) it records a rocket salvo after the
## reference pictures and in other lights, biomes and calibers (see
## `_salvo_series`). `just salvo-shots`.
##
## With `--explosion-series` (prompt 072) it triggers every explosion kind
## once on open, flat ground (small, medium and big ground hits, a destroyed
## vehicle, an ammunition explosion), seen from SERIES_DISTANCE like the
## reference video, and saves `<kind>_<ms>.png` at the series times of
## `tools/explosion_series.py` (every frame of the first second, then four
## per second until SERIES_SECONDS; needs `--fixed-fps 30`). `series.txt`
## names the crop (pixels) that shows SERIES_KINDS[kind].crop_m meters
## around the explosion. `just explosion-series`.
##
## With `--throw-series` (prompt 194, scenario `blast_throws.toml`) it fires
## a 38 cm rocket (Sturmtiger) next to one rifle squad and a 15 cm blast
## (an exploding ammunition dump) next to the other, records the soldiers
## flying, and prints the frame times after each blast, the most ragdolls
## at once and the throws (`soldier_motion.gd` counts). `just throw-shots`.

const BaseView := preload("res://scenes/world/base_view.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")

const SHOT_SIZE := Vector2i(1600, 900)
## Salvo series (prompt 076): camera after the reference pictures (oblique
## from above and behind, firing up and to the right), the distance to the
## target, the length of the series and its step (frames at 30 fps).
const SALVO_PITCH_DEG := 50.0
const SALVO_DISTANCE := 95.0
const SALVO_VIEW_TURN := 0.35
const SALVO_RANGE := 520.0
const SALVO_SECONDS := 14.0
const SALVO_STEP_FRAMES := 6
const CALIBER := 150.0
const SALVO := 24
const SALVO_RADIUS := 60.0
const SALVO_GAP_FRAMES := 7
## Explosion series (prompt 072): camera distance, length of a series, and
## per kind the crop in meters (the width the matching video crop shows,
## see tools/explosion-series.sh) and the height of the crop center.
const SERIES_DISTANCE := 60.0
const SERIES_PITCH_DEG := 45.0
const SERIES_SECONDS := 10.0
const SERIES_FPS := 30
const SERIES_KINDS := {
	"small": {"crop_m": 12.0},
	"medium": {"crop_m": 12.6},
	"big": {"crop_m": 16.0},
	"vehicle": {"crop_m": 21.0},
	"ammo": {"crop_m": 28.0},
}

var main: Node3D
var _dir := ""
var _shot_count := 0


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--blast-shots="):
			_dir = arg.trim_prefix("--blast-shots=")
	if "--explosion-series" in OS.get_cmdline_user_args():
		await _series()
		get_tree().quit()
		return
	if "--salvo-series" in OS.get_cmdline_user_args():
		await _salvo_series()
		get_tree().quit()
		return
	if "--throw-series" in OS.get_cmdline_user_args():
		await _throw_series()
		get_tree().quit()
		return
	if "--strike-live" in OS.get_cmdline_user_args():
		var ok: bool = await _live()
		get_tree().quit(0 if ok else 1)
		return
	await _shots()
	get_tree().quit()


func _shots() -> void:
	if _dir.is_empty():
		_dir = "user://blast-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	await _frames(10)
	var fx: Node3D = main.effects
	var blast: Node3D = main.blast_effects
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var tank_pos := _position_of(own[0])

	# A big blast in the open by day.
	var field := _ground(tank_pos + _forward(own[0]) * 45.0)
	await _view(field + Vector3(0, 0, 0), 75.0, 0.6)
	_big(field)
	await _capture("blast_day", [1, 2, 4, 7, 12, 24, 45])

	# Next to the village.
	var house := _house()
	var near := _ground(house + Vector3(14, 0, 10))
	await _view((house + near) * 0.5, 60.0, 0.9)
	await _frames(60)
	_big(near)
	await _capture("blast_village_day", [1, 3, 8])

	# A tank's ammunition explodes: the turret flies off.
	var victim := enemies[1]
	var victim_pos := _position_of(victim)
	await _view(victim_pos, 32.0, (main._unit_nodes[victim] as Node3D).rotation.y + PI * 0.5)
	fx.destroyed(victim, victim_pos)
	blast.ammo_explosion(victim, victim_pos)
	await _capture("ammo_explosion", [2, 4, 8, 14, 22, 32, 60, 90])

	# A heavy strike by day: marked target, then the salvo.
	var target := _ground((tank_pos + _position_of(enemies[0])) * 0.5 + Vector3(-40, 0, 0))
	await _strike(target, "strike_day")

	# Night (prompt 040): the flash lights up ground and houses.
	main.atmosphere.set_weather("clear", "night")
	await _frames(30)
	await _view((house + near) * 0.5, 60.0, 0.9)
	await _frames(240)
	await _capture("blast_village_night", [0])
	_big(near)
	await _capture("blast_village_night", [1, 2, 4, 8])
	await _view(field, 75.0, 0.6)
	_big(field)
	await _capture("blast_night", [1, 3])
	await _strike(target + Vector3(30, 0, 50), "strike_night")
	print("blast-shots: %d screenshots in %s; counts %s, strike %s" % [
		_shot_count, ProjectSettings.globalize_path(_dir), blast.counts, main.strike_view.counts])


# --- Explosion series (prompt 072) ------------------------------------------------


func _series() -> void:
	if _dir.is_empty():
		_dir = "user://explosion-series"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	(main.get_node("Hud") as CanvasLayer).visible = false
	# The reference video looks down at about 45 degrees.
	main.camera_rig.pitch_near_deg = SERIES_PITCH_DEG
	main.camera_rig.pitch_far_deg = SERIES_PITCH_DEG
	await _frames(10)
	var fx: Node3D = main.effects
	var blast: Node3D = main.blast_effects
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var tank_pos := _position_of(own[0])
	var forward := _forward(own[0])
	var side := forward.cross(Vector3.UP).normalized()
	var crops := PackedStringArray()
	var k := 0
	for kind: String in SERIES_KINDS:
		# A fresh spot per kind, away from the smoke of the previous one.
		var at := _ground(tank_pos + forward * 45.0 + side * (k * 70.0 - 140.0))
		var unit := -1
		if kind == "vehicle" or kind == "ammo":
			unit = enemies[1] if kind == "vehicle" else enemies[mini(2, enemies.size() - 1)]
			at = _position_of(unit)
		k += 1
		await _view(at, SERIES_DISTANCE, 0.6)
		await _frames(30)
		crops.append("%s %s" % [kind, _series_crop(at, float(SERIES_KINDS[kind].crop_m))])
		match kind:
			"small":
				fx.ground_hit(at, 50.0)
			"medium":
				fx.ground_hit(at, 90.0)
			"big":
				_big(at)
			"vehicle":
				fx.destroyed(unit, at)
			"ammo":
				fx.destroyed(unit, at)
				blast.ammo_explosion(unit, at)
		var offsets: Array = []
		for ms in _series_ms():
			offsets.append(int(round(ms * SERIES_FPS / 1000.0)))
		await _capture_series(kind, offsets, _series_ms())
	var file := FileAccess.open(_dir + "/series.txt", FileAccess.WRITE)
	file.store_string("\n".join(crops) + "\n")
	print("explosion-series: %d pictures in %s" % [_shot_count, ProjectSettings.globalize_path(_dir)])


## The times of a series in ms (as `series_times` in tools/explosion_series.py).
func _series_ms() -> Array[int]:
	var result: Array[int] = []
	for i in SERIES_FPS:
		result.append(int(round(i * 1000.0 / SERIES_FPS)))
	var t := 1000
	while t <= int(SERIES_SECONDS * 1000.0):
		result.append(t)
		t += 250
	return result


## "x,y,w,h" of a square crop that shows `meters` across, centered a third
## of its size above the ground point `at`.
func _series_crop(at: Vector3, meters: float) -> String:
	var camera: Camera3D = main.camera
	var center := camera.unproject_position(at + Vector3.UP * meters * 0.3)
	var right := camera.global_basis.x
	var a := camera.unproject_position(at - right * meters * 0.5)
	var b := camera.unproject_position(at + right * meters * 0.5)
	var size := mini(int(a.distance_to(b)), SHOT_SIZE.y)
	var x := clampi(int(center.x) - size / 2, 0, SHOT_SIZE.x - size)
	var y := clampi(int(center.y) - size / 2, 0, SHOT_SIZE.y - size)
	return "%d,%d,%d,%d" % [x, y, size, size]


## Saves `<name>_<ms>.png` at the frames `offsets` after now.
func _capture_series(name: String, offsets: Array, ms: Array[int]) -> void:
	var frame := 0
	for n in offsets.size():
		while frame <= int(offsets[n]):
			await _frame()
			frame += 1
		var image := get_viewport().get_texture().get_image()
		image.save_png("%s/%s_%05d.png" % [_dir, name, ms[n]])
		_shot_count += 1


# --- Rocket salvo (prompt 076) -----------------------------------------------------


## The salvo after the reference pictures: an own unit gets the battery
## position's model (ammunition dump and three launch frames) and fires 24
## rockets 250 ms apart through `SimHost.debug_rocket` (the rules of a strike
## rocket), seen obliquely from above and behind like the reference, in
## winter at dusk, every SALVO_STEP_FRAMES until the last smoke is gone;
## then the side view over the whole flight, the approach at the target,
## day and night, the other biomes and the three factions' calibers.
func _salvo_series() -> void:
	if _dir.is_empty():
		_dir = "user://salvo-series"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	(main.get_node("Hud") as CanvasLayer).visible = false
	main.camera_rig.pitch_near_deg = SALVO_PITCH_DEG
	main.camera_rig.pitch_far_deg = SALVO_PITCH_DEG
	await _frames(10)
	var own := _ids_of(true)
	var battery := own[0]
	var at := _position_of(battery)
	var enemies := _ids_of(false)
	var toward := _position_of(enemies[0]) - at
	toward.y = 0.0
	var target := _ground(at + toward.normalized() * minf(SALVO_RANGE, toward.length()))
	_make_battery(battery, "winter")
	var only := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--only="):
			only = arg.trim_prefix("--only=")
	var runs: Array[Dictionary] = [
		{"name": "winter_evening", "biome": "winter", "daytime": "evening", "series": true},
		{"name": "winter_side", "biome": "winter", "daytime": "evening", "view": "side", "at": [20, 40, 70, 110]},
		{"name": "winter_target", "biome": "winter", "daytime": "evening", "view": "target", "at": [80, 86, 100]},
		{"name": "winter_day", "biome": "winter", "daytime": "day"},
		{"name": "winter_night", "biome": "winter", "daytime": "night"},
		{"name": "forest_day", "biome": "forest", "daytime": "day"},
		{"name": "forest_night", "biome": "forest", "daytime": "night"},
		{"name": "desert_day", "biome": "desert", "daytime": "day"},
		{"name": "desert_evening", "biome": "desert", "daytime": "evening"},
		{"name": "faction_de", "biome": "forest", "daytime": "evening", "caliber": 150.0},
		{"name": "faction_us", "biome": "forest", "daytime": "evening", "caliber": 114.0},
	]
	for run in runs:
		if not only.is_empty() and not (str(run.name) in only.split(",")):
			continue
		if main.atmosphere.preset.id != run.biome:
			main.set_biome(run.biome)
			_make_battery(battery, run.biome)
		main.atmosphere.set_weather("clear", run.daytime)
		var view := str(run.get("view", "reference"))
		var direction := (target - at).normalized()
		match view:
			"side":
				main.camera_rig.pitch_near_deg = 18.0
				main.camera_rig.pitch_far_deg = 18.0
				await _view(at.lerp(target, 0.4), 260.0, _yaw_looking(direction.cross(Vector3.UP)))
			"target":
				main.camera_rig.pitch_near_deg = 35.0
				main.camera_rig.pitch_far_deg = 35.0
				await _view(target, 120.0, _yaw_looking(-direction.rotated(Vector3.UP, 0.5)))
			_:
				main.camera_rig.pitch_near_deg = SALVO_PITCH_DEG
				main.camera_rig.pitch_far_deg = SALVO_PITCH_DEG
				# Firing up and to the right as in the reference.
				await _view(at + direction * 12.0, SALVO_DISTANCE, _yaw_looking(direction.rotated(Vector3.UP, SALVO_VIEW_TURN)))
		main.strike_view.aim_launchers(battery, target)
		await _frames(90)
		var caliber := float(run.get("caliber", 132.0))
		var frames: Array = []
		if run.get("series", false):
			for k in int(SALVO_SECONDS * 30.0 / SALVO_STEP_FRAMES) + 1:
				frames.append(k * SALVO_STEP_FRAMES)
		else:
			frames = run.get("at", [6, 30, 60, 90, 200])
		await _fire_salvo(battery, target, caliber, run.name, frames)
		# Let the smoke drift off before the next run.
		await _frames(int(main.effects._settings.fx_trail_life * 30.0))
	print("salvo-series: %d pictures in %s; rockets %d, counts %s" % [
		_shot_count, ProjectSettings.globalize_path(_dir), main.effects.rockets.launched, main.effects.counts])


## Fires SALVO rockets 250 ms apart (7 or 8 frames at 30 fps) and saves
## `<name>_<ms>.png` at `frames` after the first launch.
func _fire_salvo(battery: int, target: Vector3, caliber: float, name: String, frames: Array) -> void:
	var host: SimHost = main.sim_host
	var rng := RandomNumberGenerator.new()
	rng.seed = 76
	var frame := 0
	var fired := 0
	var shot := 0
	var last: int = frames.back()
	while frame <= last:
		if fired < SALVO and frame >= int(fired * 7.5):
			var a := rng.randf() * TAU
			var r := SALVO_RADIUS * sqrt(rng.randf())
			host.debug_rocket(battery, target.x + cos(a) * r, target.z + sin(a) * r, caliber)
			fired += 1
		await _frame()
		while shot < frames.size() and int(frames[shot]) == frame:
			var image := get_viewport().get_texture().get_image()
			image.save_png("%s/salvo_%s_%05d.png" % [_dir, name, int(round(frame * 1000.0 / 30.0))])
			_shot_count += 1
			shot += 1
		frame += 1
	print("salvo-series: %s done, %d pictures" % [name, shot])


## Unit `id` looks like the battery position (the borrowed model with
## launch frames) and its rails are found anew.
func _make_battery(id: int, biome: String) -> void:
	var node: Node3D = main._unit_nodes[id]
	BaseView.attach_model(node, "base_rocket_battery", biome)
	main.effects._sockets.erase(id)


## Camera yaw (rts_camera.gd) that looks along the horizontal `direction`.
func _yaw_looking(direction: Vector3) -> float:
	return atan2(-direction.x, -direction.z)


# --- A real strike -------------------------------------------------------------


func _live() -> bool:
	if _dir.is_empty():
		_dir = "user://blast-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	var host: SimHost = main.sim_host
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	await _frames(10)
	var me := host.local_player()
	var hq := _structure("base_hq", true)
	if hq.is_empty():
		printerr("strike-live: no headquarters (start with --mode=base)")
		return false
	host.speed = 30.0
	for id in ["base_depot", "base_factory", "base_rocket_battery"]:
		if not await _build(id, hq["position"]):
			printerr("strike-live: could not build %s" % id)
			host.speed = 1.0
			return false
	while not str(host.strike_option().get("blocked", "?")).is_empty():
		await _frames(10)
	host.speed = 1.0
	var target: Vector3 = _structure("base_hq", false).get("position", Vector3.ZERO)
	var view: Node3D = main.strike_view
	host.call_strike(target.x, target.z)
	var waited := 0
	while int(view.counts.get("called", 0)) == 0 and waited < 300:
		await _frames(1)
		waited += 1
	await _view(target, 170.0, 0.5)
	await _frames(240)
	var ui: Node = UiSound.instance()
	var siren: bool = ui != null and str(ui.signal_playing()).begins_with(view.SIREN_KEY)
	var counting: bool = view._countdown.visible
	await _capture("strike_live_marked", [0])
	host.speed = 4.0
	waited = 0
	while int(view.counts.get("launched", 0)) == 0 and waited < 600:
		await _frames(1)
		waited += 1
	host.speed = 1.0
	# Prompt 076: the launch at the position (flare, flame, trail).
	var battery_at: Vector3 = _structure("base_rocket_battery", true).get("position", target)
	await _view(battery_at + (target - battery_at).normalized() * 12.0, 70.0,
		_yaw_looking((target - battery_at).normalized().rotated(Vector3.UP, SALVO_VIEW_TURN)))
	await _capture("strike_live_launch", [6, 20, 45])
	await _view(target, 170.0, 0.5)
	# The first rockets land after their flight (at least 3 s).
	waited = 0
	while main.blast_effects.counts.get("big_blast", 0) < 3 and waited < 900:
		await _frames(1)
		waited += 1
	await _capture("strike_live_salvo", [10, 60, 140])
	await _view(_structure("base_rocket_battery", true).get("position", target), 45.0, 0.8)
	await _capture("strike_live_battery", [2])
	var rockets: int = main.effects.rockets.launched
	var ok: bool = int(view.counts.get("called", 0)) == 1 and int(view.counts.get("launched", 0)) == 1 \
		and siren and counting and main.blast_effects.counts.get("big_blast", 0) >= 10 and rockets >= 20
	print("strike-live: %s; strike %s, blasts %s, siren %s, countdown %s, rockets launched %d, player %d" % [
		"PASSED" if ok else "FAILED", view.counts, main.blast_effects.counts, siren, counting, rockets, me])
	return ok


## The first structure of `type_id` (own or enemy): its dictionary with
## `position`.
func _structure(type_id: String, own: bool) -> Dictionary:
	var host: SimHost = main.sim_host
	for s: Dictionary in host.structures():
		if s["type_id"] == type_id and (int(s["owner"]) == host.local_player()) == own:
			var node: Node3D = main._unit_nodes.get(int(s["id"]))
			if node != null:
				s["position"] = node.global_position
			return s
	return {}


## Places `type_id` at the first free spot around `near` and waits until
## it is completed.
func _build(type_id: String, near: Vector3) -> bool:
	var host: SimHost = main.sim_host
	var unit_type := -1
	for o: Dictionary in host.build_options():
		if o["id"] == type_id:
			unit_type = int(o["type"])
	if unit_type < 0:
		return false
	var placed := false
	for r in range(20, 80, 6):
		for k in 16:
			var a := k * TAU / 16.0
			var p := near + Vector3(cos(a), 0.0, sin(a)) * r
			if placed or not host.placement_check(unit_type, p.x, p.z).is_empty():
				continue
			host.place_building(unit_type, p.x, p.z)
			placed = true
	if not placed:
		return false
	for i in 3000:
		await _frames(1)
		var s := _structure(type_id, true)
		if not s.is_empty() and float(s["built"]) >= 1.0:
			return true
	return false


# --- Throw series (prompt 194) ---------------------------------------------------


func _throw_series() -> void:
	if _dir.is_empty():
		_dir = "user://throw-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	await _frames(30)
	var shooter: int = _ids_of(true)[0]
	var squads := _ids_of(false)
	var runs := [
		{"name": "throw_sturmtiger", "squad": squads[0], "caliber": 380.0, "offset": Vector3(-5, 0, 2)},
		{"name": "throw_dump", "squad": squads[1], "caliber": 150.0, "offset": Vector3(-3, 0, 1)},
	]
	var motion: Node = main.infantry_view.motion
	for run: Dictionary in runs:
		var at := _ground(_position_of(int(run["squad"])) + (run["offset"] as Vector3))
		await _view(at, 34.0, 0.7)
		await _frames(30)
		var thrown_before := int(motion.counts.get("thrown", 0))
		var blasts := int(main.blast_effects.counts.get("big_blast", 0))
		main.sim_host.debug_rocket(shooter, at.x, at.z, float(run["caliber"]))
		# Waits for the impact (the blast effects or the first soldier flying).
		var waited := 0
		while waited < 900 and int(main.blast_effects.counts.get("big_blast", 0)) == blasts \
				and int(motion.counts.get("thrown", 0)) == thrown_before:
			await _frame()
			waited += 1
		var frame_ms: Array[float] = []
		var most := 0
		var last := Time.get_ticks_usec()
		var frame := 0
		for offset: int in [0, 3, 6, 9, 12, 16, 20, 26, 34, 50, 80]:
			while frame <= offset:
				await _frame()
				var now := Time.get_ticks_usec()
				frame_ms.append((now - last) / 1000.0)
				last = now
				most = maxi(most, motion.ragdolls.active_count())
				frame += 1
			var image := get_viewport().get_texture().get_image()
			var path := "%s/%s_%02d.png" % [_dir, run["name"], offset]
			image.save_png(path)
			_shot_count += 1
			# Saving is not part of the frame time.
			last = Time.get_ticks_usec()
		frame_ms.sort()
		var mean := 0.0
		for ms in frame_ms:
			mean += ms
		mean /= maxf(frame_ms.size(), 1.0)
		print("throw-shots: %s: impact after %d frames, %d thrown (3 m and more), ragdolls at most %d, frame ms mean %.1f, median %.1f, max %.1f" % [
			run["name"], waited, int(motion.counts.get("thrown", 0)) - thrown_before, most, mean,
			frame_ms[frame_ms.size() / 2], frame_ms[-1]])
		await _frames(90)
	print("throw-shots: %d screenshots in %s; motion %s, weapons %d, helmets %d" % [
		_shot_count, ProjectSettings.globalize_path(_dir), motion.counts, motion.helmets.weapons_thrown,
		motion.helmets.knocked_off])


func _big(at: Vector3) -> void:
	main.effects.ground_hit(at, CALIBER)
	main.blast_effects.big_blast(at, CALIBER)


## Marker smoke and ring for a few seconds, then the rockets.
func _strike(target: Vector3, name: String) -> void:
	var view: Node3D = main.strike_view
	var marker: Dictionary = view._make_marker({"id": 9000 + _shot_count, "owner": 1, "target": target,
		"radius": SALVO_RADIUS, "from": target + Vector3(0, 0, -300)}, false)
	await _view(target, 190.0, 0.5)
	await _frames(150)
	await _capture(name + "_marked", [0])
	for k in SALVO:
		var angle := k * 2.399963
		var r := SALVO_RADIUS * sqrt((k + 0.5) / SALVO)
		_big_rocket(_ground(target + Vector3(cos(angle) * r, 0, sin(angle) * r)))
		await _frames(SALVO_GAP_FRAMES)
		if k == 6 or k == 15:
			await _capture(name + "_salvo_%02d" % k, [0])
	await _capture(name + "_after", [20])
	view._retire(marker)


func _big_rocket(at: Vector3) -> void:
	main.effects.ground_hit(at, 132.0)
	main.blast_effects.big_blast(at, 132.0)


## A farm house of the village (the map's "village" marker).
func _house() -> Vector3:
	var found: PackedInt64Array = main.sim_host.destructible_objects_near("env_house_farm", -138.0, 138.0, 120.0)
	if found.is_empty():
		return _ground(Vector3(-138, 0, 138))
	return _ground(main.sim_host.object_position(found[0]))


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
		print("blast-shots: %s" % path.get_file())


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


func _forward(id: int) -> Vector3:
	return -(main._unit_nodes[id] as Node3D).global_basis.z


func _ground(point: Vector3) -> Vector3:
	return Vector3(point.x, main.sim_host.height_at(point.x, point.z), point.z)


func _frames(count: int) -> void:
	for i in count:
		await _frame()


func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
	await get_tree().process_frame
