extends Node
## Scripted recordings for the trailer (prompt 070), started by `main.gd`
## with `-- --trailer=<shot>`; `just trailer-shots` records every shot.
##
## A shot plays a scene of `data/trailer/<scene>.toml` (a mission script,
## docs/MISSIONS.md, which is no mission of the menu) on its map with a fixed
## seed, runs the simulation ahead to the shot's start without anybody
## watching, then moves the camera along the shot's keys for its length and
## quits. Everything that happens comes from the script's `order` and `fire`
## actions, i.e. normal commands; the simulation is not touched. Several
## shots can show the same scene (same script and seed) from other angles.
##
##     --trailer=<shot>          play the shot (with `--write-movie` and
##                               `--fixed-fps 60`: record it; the line
##                               "trailer: record …" names the first frame)
##     --trailer-stills=<dir>    save the shot's `stills` as PNG files instead
##     --trailer-scout[=<s>]     no camera: run the scene for <s> game seconds
##                               (default: to the shot's end) at high speed and
##                               print who is destroyed when and where
##
## The camera does more than in the game (lower, closer, flat angles, chase
## rides); the camera rig stands still meanwhile and only receives focus and
## distance, which haze and sound follow. Presentation only.

const FPS := 60
const SIZE := Vector2i(1920, 1080)
## Folder of the scene scripts below `data/`.
const SCENE_DIR := "trailer"
## Game speed of the run-up (the bridge runs at most 5 ticks per frame).
const RUN_UP_SPEED := 15.0
## The last seconds before the shot run at normal speed, so smoke, dust and
## sound loops are in place when the first frame is written.
const PRE_ROLL_S := 2.0
## The camera stays this far above the ground below it, in meters.
const CLEARANCE := 0.45
const TICK_HZ := 20
## Bus volumes of a recording in dB (-80: muted).
const BUS_DB: Dictionary[String, float] = {
	"Master": -9.0, "Effects": 0.0, "Ambience": -2.0, "UI": -2.0, "Music": -80.0,
}

## The shots. Per shot:
##   map, scene (file in data/trailer, default: the shot's name), seed,
##   start_s (game seconds before the first frame), seconds (length),
##   hud (default false: no HUD, no fog of war, no relation tint),
##   weather, daytime (default: the map's), stills (seconds into the shot),
##   select ([owner, catalog id prefix]: units the player has selected),
##   camera: keys in time order, interpolated (Catmull-Rom):
##     t      seconds into the shot
##     at     Vector2 map position (x east, y south) the camera looks at, or
##     unit   [owner, catalog id prefix, index]: that unit (index -1: the
##            centre of all living ones), sorted by unit id
##     lift   meters above the ground the camera looks at (default 1)
##     dist   meters between camera and that point
##     yaw    degrees, 0 looks north, 90 looks west (as the game's camera)
##     heading  true: `yaw` is relative to the unit's driving direction
##     pitch  degrees above the horizon
##     fov    field of view in degrees (default: the game's 30)
##     haze   distance the haze and the sound are set for (default: `dist`
##            for steep views, farther for flat ones)
const SHOTS: Dictionary[String, Dictionary] = {
	# --- Scene "column" on farms: march and first contact ---------------------
	"column_pass": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 6.0, "seconds": 8.0,
		"stills": [1.0, 4.0, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(5, 262), "dist": 24.0, "yaw": 196.0, "pitch": 3.0, "lift": 1.2},
			{"t": 8.0, "at": Vector2(3, 258), "dist": 22.0, "yaw": 204.0, "pitch": 3.5, "lift": 1.2},
		],
	},
	"column_high": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 18.0, "seconds": 6.0,
		"stills": [1.0, 4.0],
		"camera": [
			{"t": 0.0, "unit": [0, "de_sdkfz251_1", -1], "dist": 120.0, "yaw": 150.0, "pitch": 48.0},
			{"t": 6.0, "unit": [0, "de_sdkfz251_1", -1], "dist": 95.0, "yaw": 172.0, "pitch": 42.0},
		],
	},
	"column_chase": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 7.0, "seconds": 7.0,
		"stills": [1.0, 3.5, 6.0],
		"camera": [
			{"t": 0.0, "unit": [0, "de_pz4_h", 3], "heading": true, "dist": 13.0, "yaw": 150.0, "pitch": 6.0, "lift": 1.5},
			{"t": 7.0, "unit": [0, "de_pz4_h", 3], "heading": true, "dist": 10.0, "yaw": 112.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"contact_line": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 36.5, "seconds": 8.0,
		"stills": [1.0, 1.9, 3.4, 5.9, 7.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-2, 100), "dist": 30.0, "yaw": -18.0, "pitch": 6.0, "lift": 1.5},
			{"t": 8.0, "at": Vector2(-2, 98), "dist": 24.0, "yaw": -30.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"contact_t34": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 36.2, "seconds": 6.0,
		"stills": [1.0, 2.2, 3.7, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-22, -9), "dist": 34.0, "yaw": 196.0, "pitch": 9.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(-22, -9), "dist": 24.0, "yaw": 226.0, "pitch": 7.0, "lift": 1.5},
		],
	},
	# --- Scene "bridge" on river_land: the main battle ------------------------
	"bridge_player": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 15.0, "seconds": 8.0, "hud": true, "select": [0, "de_pz4_h"], "stills": [1.0, 4.3, 6.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-12, 2), "dist": 205.0, "yaw": 0.0, "pitch": 56.0, "lift": 0.0},
			{"t": 8.0, "at": Vector2(-4, 2), "dist": 190.0, "yaw": 0.0, "pitch": 55.0, "lift": 0.0},
		],
	},
	"bridge_pak": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 9.5, "seconds": 5.0, "stills": [1.0, 3.0, 3.3, 4.5],
		"camera": [
			{"t": 0.0, "unit": [0, "de_pak40", 1], "dist": 12.0, "yaw": 42.0, "pitch": 9.0, "lift": 1.0},
			{"t": 5.0, "unit": [0, "de_pak40", 1], "dist": 9.5, "yaw": 62.0, "pitch": 7.0, "lift": 1.0},
		],
	},
	"bridge_low": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 15.5, "seconds": 7.0, "stills": [1.0, 3.6, 4.2, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-62, 2), "dist": 38.0, "yaw": -98.0, "pitch": 6.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(-58, 2), "dist": 30.0, "yaw": -82.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"bridge_east": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 27.0, "seconds": 7.0, "stills": [1.0, 3.5, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(80, 4), "dist": 34.0, "yaw": -98.0, "pitch": 8.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(80, 4), "dist": 28.0, "yaw": -114.0, "pitch": 6.0, "lift": 1.5},
		],
	},
	"bridge_top": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 14.0, "seconds": 7.0, "stills": [1.0, 5.2, 6.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-5, 2), "dist": 215.0, "yaw": 0.0, "pitch": 74.0, "lift": 0.0},
			{"t": 7.0, "at": Vector2(-5, 2), "dist": 195.0, "yaw": 12.0, "pitch": 70.0, "lift": 0.0},
		],
	},
	# --- Variety: desert, winter, town ----------------------------------------
	"desert_road": {
		"map": "desert_village", "scene": "desert", "seed": 7, "start_s": 6.0, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "unit": [0, "us_m4a3_75", 0], "dist": 30.0, "yaw": 205.0, "pitch": 5.0, "lift": 1.5},
			{"t": 6.0, "unit": [0, "us_m4a3_75", 0], "dist": 20.0, "yaw": 230.0, "pitch": 6.0, "lift": 1.5},
		],
	},
	"desert_square": {
		"map": "desert_village", "scene": "desert", "seed": 7, "start_s": 47.0, "seconds": 7.0,
		"stills": [1.2, 4.6, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(8, 26), "dist": 38.0, "yaw": 140.0, "pitch": 13.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(8, 26), "dist": 30.0, "yaw": 185.0, "pitch": 10.0, "lift": 1.5},
		],
	},
	"winter_is2": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 5.0, "seconds": 7.0,
		"stills": [1.0, 3.4, 6.0],
		"camera": [
			{"t": 0.0, "unit": [0, "su_is2", 0], "heading": true, "dist": 17.0, "yaw": 65.0, "pitch": 6.0, "lift": 1.6},
			{"t": 7.0, "unit": [0, "su_is2", 0], "heading": true, "dist": 13.0, "yaw": 110.0, "pitch": 5.0, "lift": 1.6},
		],
	},
	"winter_village": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 6.5, "seconds": 6.0,
		"stills": [1.0, 1.9, 3.4, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(40, 72), "dist": 36.0, "yaw": 150.0, "pitch": 10.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(40, 72), "dist": 28.0, "yaw": 175.0, "pitch": 8.0, "lift": 1.5},
		],
	},
	"winter_infantry": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 1.0, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "unit": [0, "su_inf_rifle_squad", -1], "dist": 20.0, "yaw": 205.0, "pitch": 8.0, "lift": 1.0},
			{"t": 6.0, "unit": [0, "su_inf_rifle_squad", -1], "dist": 15.0, "yaw": 225.0, "pitch": 7.0, "lift": 1.0},
		],
	},
	"town_street": {
		"map": "town_small", "scene": "town", "seed": 7, "start_s": 1.5, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 128), "dist": 30.0, "yaw": 180.0, "pitch": 8.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(0, 122), "dist": 24.0, "yaw": 180.0, "pitch": 10.0, "lift": 1.5},
		],
	},
	# --- Maps from above, for planning shots ----------------------------------
	"survey_farms": {
		"map": "farms", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_desert_village": {
		"map": "desert_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1522.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_winter_village": {
		"map": "winter_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_river_land": {
		"map": "river_land", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_town_small": {
		"map": "town_small", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1141.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_twin_meadow": {
		"map": "twin_meadow", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
}

var main: Node3D
var _name := ""
var _shot: Dictionary = {}
var _stills_dir := ""
var _scout := false
var _scout_seconds := -1.0
## -1: run-up, else the frame of the shot.
var _frame := -1
var _stills_left: Array = []
## Smoothed heading per camera key (chase rides must not jitter).
var _headings: Dictionary[int, float] = {}
## Combat events by type since the last scout report.
var _scout_counts: Dictionary[String, int] = {}


## The shot named on the command line, or "".
static func requested() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--trailer="):
			return arg.trim_prefix("--trailer=")
	return ""


## Starts the scene of the requested shot on `sim_host` (called by `main.gd`
## in place of the normal start).
static func start(sim_host: SimHost) -> bool:
	var shot_name := requested()
	if not SHOTS.has(shot_name):
		push_error("unknown trailer shot '%s'; known: %s" % [shot_name, ", ".join(SHOTS.keys())])
		return false
	var shot: Dictionary = SHOTS[shot_name]
	var scene: String = shot.get("scene", shot_name)
	var file := GamePaths.data_dir().path_join(SCENE_DIR).path_join(scene + ".toml")
	sim_host.hold_fire_at_start = false
	sim_host.weather = shot.get("weather", "")
	sim_host.daytime = shot.get("daytime", "")
	return sim_host.start_mission_on(int(shot.get("seed", 1)), file.simplify_path(),
		GamePaths.map_dir(shot["map"]).simplify_path())


func _ready() -> void:
	_name = requested()
	_shot = SHOTS[_name]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--trailer-stills="):
			_stills_dir = arg.trim_prefix("--trailer-stills=")
		elif arg == "--trailer-scout":
			_scout = true
		elif arg.begins_with("--trailer-scout="):
			_scout = true
			_scout_seconds = float(arg.trim_prefix("--trailer-scout="))
	_stills_left = (_shot.get("stills", [float(_shot["seconds"]) * 0.5]) as Array).duplicate()
	if not _stills_dir.is_empty():
		DirAccess.make_dir_recursive_absolute(_stills_dir)

	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	rig.set_process(false)
	rig.set_process_unhandled_input(false)
	# Nobody plays: no pause menu, no result screen, no clicks.
	main.game_menu.visible = false
	main.game_menu.process_mode = Node.PROCESS_MODE_DISABLED
	get_viewport().gui_disable_input = true
	main.selection.set_process_input(false)
	main.selection.set_process_unhandled_input(false)
	main.atmosphere.set_quality("high")
	if not _shot.get("hud", false):
		(main.get_node("Hud") as CanvasLayer).visible = false
		main.fog_of_war.set_fog_enabled(false)
		main.effects.visible_at = func(_point: Vector3) -> bool: return true
		if main.battle_sound != null:
			main.battle_sound.visible_at = main.effects.visible_at
		# The relation tint is part of the interface, not of the world.
		main._enemy_material.albedo_color.a = 0.0
		main._ally_material.albedo_color.a = 0.0
		# Flags and capture rings of the supply points.
		for marker: Node3D in main.base_view._supply_nodes:
			marker.visible = false
	else:
		# The player's view with a large interface (a game setting), so it
		# can be read in a video.
		get_window().content_scale_factor = 1.25
	get_window().scaling_3d_scale = 1.0
	if _scout:
		main.combat_view.combat_event.connect(_on_scout_event)
	elif DisplayServer.get_name() != "headless":
		get_window().size = SIZE
	_set_volumes()
	AudioServer.set_bus_mute(0, true)
	main.sim_host.speed = RUN_UP_SPEED
	_place_camera(0.0)


func _process(_delta: float) -> void:
	var host: SimHost = main.sim_host
	var tick := host.current_tick()
	if _scout:
		_scout_step(tick)
		return
	var start_tick := roundi(float(_shot.get("start_s", 0.0)) * TICK_HZ)
	if _frame < 0:
		# Run-up: fast and silent, then the pre-roll at normal speed.
		if tick >= start_tick - roundi(PRE_ROLL_S * TICK_HZ) and host.speed != 1.0:
			host.speed = 1.0
			AudioServer.set_bus_mute(0, false)
		_place_camera(0.0)
		if tick < start_tick:
			return
		_frame = 0
		if _shot.has("select"):
			var ids: Array = []
			for node in _unit_nodes(_shot["select"]):
				ids.append(int(str(node.name).trim_prefix("Unit")))
			main.selection.set_selection(ids)
		print("trailer: record %s start_frame=%d frames=%d fps=%d tick=%d" % [
			_name, Engine.get_frames_drawn(), _frame_count(), FPS, tick])
	var t := float(_frame) / FPS
	_place_camera(t)
	if not _stills_dir.is_empty():
		_save_stills(t)
	_frame += 1
	if _frame >= _frame_count() or (not _stills_dir.is_empty() and _stills_left.is_empty()):
		print("trailer: %s done at tick %d" % [_name, host.current_tick()])
		set_process(false)
		# The frame in work is still drawn (and written) before the quit.
		get_tree().quit.call_deferred()


func _frame_count() -> int:
	return roundi(float(_shot["seconds"]) * FPS)


## The same volumes whatever the player's settings say: headroom on the
## master bus (explosions clip a 16-bit recording otherwise; the edit sets
## the loudness), and no music, which a cut would chop.
func _set_volumes() -> void:
	for bus: String in BUS_DB:
		var index := AudioServer.get_bus_index(bus)
		if index >= 0:
			AudioServer.set_bus_volume_db(index, BUS_DB[bus])
			AudioServer.set_bus_mute(index, BUS_DB[bus] <= -80.0)


# --- Camera ---------------------------------------------------------------------


## Places the camera for `t` seconds into the shot.
func _place_camera(t: float) -> void:
	var keys: Array = _shot.get("camera", [])
	if keys.is_empty():
		return
	# The segment of `t` and the position in it.
	var i := 0
	while i < keys.size() - 2 and t >= float(keys[i + 1]["t"]):
		i += 1
	var a: Dictionary = keys[i]
	var b: Dictionary = keys[mini(i + 1, keys.size() - 1)]
	var span := float(b["t"]) - float(a["t"])
	var u := clampf((t - float(a["t"])) / span, 0.0, 1.0) if span > 0.0 else 0.0
	var before: Dictionary = keys[maxi(i - 1, 0)]
	var after: Dictionary = keys[mini(i + 2, keys.size() - 1)]
	var indices := [maxi(i - 1, 0), i, mini(i + 1, keys.size() - 1), mini(i + 2, keys.size() - 1)]
	var four := [before, a, b, after]

	var value := func(key: String, fallback: float, logarithmic := false) -> float:
		var v: Array[float] = []
		for k: Dictionary in four:
			var x := float(k.get(key, fallback))
			v.append(log(x) if logarithmic else x)
		var r := _catmull(v[0], v[1], v[2], v[3], u)
		return exp(r) if logarithmic else r

	var points: Array[Vector3] = []
	var yaws: Array[float] = []
	for n in 4:
		var k: Dictionary = four[n]
		var point := _look_point(k)
		points.append(point)
		var yaw := deg_to_rad(float(k.get("yaw", 0.0)))
		if k.get("heading", false):
			yaw += _heading(indices[n], k)
		yaws.append(yaw)
	var target := points[1].cubic_interpolate(points[2], points[0], points[3], u)
	# Angles turn the short way between neighbouring keys.
	for n in range(1, 4):
		yaws[n] = yaws[n - 1] + angle_difference(yaws[n - 1], yaws[n]) if four[n].get("heading", false) \
			or four[n - 1].get("heading", false) else yaws[n]
	var yaw := _catmull(yaws[0], yaws[1], yaws[2], yaws[3], u)
	var distance: float = value.call("dist", 80.0, true)
	var pitch := deg_to_rad(value.call("pitch", 45.0))
	var fov: float = value.call("fov", main.camera_rig.fov_deg)
	# Flat views look far: the haze of a steep view would hide the distance.
	var haze: float = value.call("haze", -1.0)
	if haze <= 0.0:
		haze = maxf(distance / maxf(sin(pitch), 0.2), 100.0)

	var rig: Node3D = main.camera_rig
	var camera: Camera3D = main.camera
	var ground := Vector3(target.x, main.sim_host.height_at(target.x, target.z), target.z)
	rig.focus = ground
	rig.distance = haze
	rig.target_distance = haze
	rig.yaw = yaw
	rig.target_yaw = yaw
	rig.global_position = ground
	var back := Vector3(0.0, sin(pitch), cos(pitch)).rotated(Vector3.UP, yaw)
	var eye := target + back * distance
	eye.y = maxf(eye.y, main.sim_host.height_at(eye.x, eye.z) + CLEARANCE)
	camera.global_position = eye
	camera.look_at(target, Vector3.UP)
	camera.fov = fov
	camera.near = clampf(distance * 0.02, 0.05, 2.0)
	camera.far = maxf(distance * 4.0, 2500.0)
	main.sun.directional_shadow_max_distance = clampf(haze * 2.5, 60.0, 500.0)


## The point a camera key looks at, right now.
func _look_point(key: Dictionary) -> Vector3:
	var lift := float(key.get("lift", 1.0))
	if key.has("unit"):
		var nodes := _unit_nodes(key["unit"])
		if not nodes.is_empty():
			var sum := Vector3.ZERO
			for node in nodes:
				sum += node.global_position
			return sum / nodes.size() + Vector3.UP * lift
	var at: Vector2 = key.get("at", Vector2.ZERO)
	return Vector3(at.x, main.sim_host.height_at(at.x, at.y) + lift, at.y)


## Driving direction of the key's unit as a camera yaw, smoothed.
func _heading(index: int, key: Dictionary) -> float:
	var nodes := _unit_nodes(key.get("unit", []))
	if nodes.is_empty():
		return _headings.get(index, 0.0)
	var heading: float = nodes[0].rotation.y
	if _headings.has(index):
		heading = lerp_angle(_headings[index], heading, 0.04)
	_headings[index] = heading
	return heading


## The drawn nodes of `[owner, catalog id prefix, index]`: the living units
## of that kind sorted by id, or only the one at `index` (also when it is
## a wreck by now).
func _unit_nodes(spec: Array) -> Array[Node3D]:
	var result: Array[Node3D] = []
	if spec.size() < 2:
		return result
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var alive := host.unit_alive()
	var matching: Array[int] = []
	var living: Dictionary[int, bool] = {}
	for i in ids.size():
		if owners[i] == int(spec[0]) and types[i].begins_with(str(spec[1])):
			matching.append(ids[i])
			living[ids[i]] = alive[i] != 0
	matching.sort()
	var index := int(spec[2]) if spec.size() > 2 else -1
	for n in matching.size():
		var id := matching[n]
		if (index < 0 and living[id]) or n == index:
			var node: Node3D = main._unit_nodes.get(id)
			if node != null:
				result.append(node)
	return result


static func _catmull(p0: float, p1: float, p2: float, p3: float, u: float) -> float:
	return 0.5 * ((2.0 * p1) + (-p0 + p2) * u + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
		+ (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u)


# --- Stills ---------------------------------------------------------------------


func _save_stills(t: float) -> void:
	if _stills_left.is_empty() or t < float(_stills_left[0]):
		return
	var at := float(_stills_left.pop_front())
	# The picture of this frame exists after it was drawn.
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var path := "%s/%s_%04.1f.png" % [_stills_dir, _name, at]
	image.save_png(path)
	print("trailer: still %s" % path)


# --- Scouting -------------------------------------------------------------------


func _scout_step(tick: int) -> void:
	var seconds := _scout_seconds
	if seconds < 0.0:
		seconds = float(_shot.get("start_s", 0.0)) + float(_shot["seconds"])
	if tick % (5 * TICK_HZ) < 5 and tick > 0:
		_print_groups(tick)
	if tick >= roundi(seconds * TICK_HZ):
		_print_groups(tick)
		set_process(false)
		get_tree().quit()


func _on_scout_event(event: Dictionary) -> void:
	_scout_counts[event["type"]] = int(_scout_counts.get(event["type"], 0)) + 1
	if event["type"] == "destroyed":
		var at: Vector3 = event["position"]
		print("trailer-scout: %6.2f s destroyed %s at (%d, %d) by %s" % [
			float(main.sim_host.current_tick()) / TICK_HZ, _unit_text(event["unit"]),
			roundi(at.x), roundi(at.z), _unit_text(event["by"])])


## "owner type #index" of a unit id, the index as camera keys count it.
func _unit_text(id: int) -> String:
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var at := ids.find(id)
	if at < 0:
		return "unit %d" % id
	var index := 0
	for i in ids.size():
		if owners[i] == owners[at] and types[i] == types[at] and ids[i] < id:
			index += 1
	return "%d %s #%d" % [owners[at], types[at], index]


## Where the living units of every owner and type stand.
func _print_groups(tick: int) -> void:
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var alive := host.unit_alive()
	var positions := host.current_positions()
	var groups: Dictionary[String, Array] = {}
	for i in ids.size():
		if alive[i] != 0:
			var key := "%d %s" % [owners[i], types[i]]
			if not groups.has(key):
				groups[key] = []
			groups[key].append(Vector2i(roundi(positions[i].x), roundi(positions[i].z)))
	var keys := groups.keys()
	keys.sort()
	var text := ""
	for key: String in keys:
		text += "\n    %s: %s" % [key, groups[key]]
	print("trailer-scout: %6.2f s  %s%s" % [float(tick) / TICK_HZ, _scout_counts, text])
	_scout_counts.clear()
