extends Node
## Vehicle tracks and ground effects in pictures (prompt 104), started by
## `main.gd`:
##
##     godot --path godot -- --scenario=tracks.toml [--weather=rain] --track-shots=<dir>
##
## (`tracks.toml`: sand and a ford on `river_fork`, `tracks_mud.toml`: mud
## on `river_land`, `tracks_winter.toml`: snow on `winter_village`.)
##
## With the fog of war off it looks for open patches of every ground kind of
## the map (grass, sand, mud, snow, a ford; on winter maps grass is drawn as
## snow), drives the scenario's tank and truck there and both through one
## figure eight, the truck a few waypoints behind the tank. Per ground it
## saves `<prefix>_<ground>_drive.png` (close, while driving: dust, clods,
## spray) and `<prefix>_<ground>_tracks.png` (from normal playing height
## after the lap: the tracks). `<prefix>` is the map's biome and the weather.
## `just track-shots`.

const SHOT_SIZE := Vector2i(1600, 900)
## Terrain ids per ground of the shots (data/terrain.toml).
const GROUNDS: Dictionary[String, Array] = {
	"grass": [0], "sand": [3], "mud": [4], "snow": [5], "ford": [6],
}
## Winter maps draw grass as snow.
const WINTER_GROUNDS: Dictionary[String, Array] = {"snow": [0, 5], "ford": [6]}
## Half extents of the figure eight (m): along x, along z.
const EIGHT := Vector2(12.0, 6.0)
const WAYPOINTS := 16
## The truck follows this many waypoints behind the tank.
const TRUCK_LAG := 4
## A waypoint counts as reached within this distance (m).
const ARRIVE_M := 4.5
## Share of the patch around the eight that must be the ground kind.
const MIN_SHARE := 0.7
## Search step over the map and sample step inside a patch (m).
const SEARCH_STEP_M := 8.0
const SAMPLE_STEP_M := 3.0
## Game speed on the way to a patch and on the lap.
const TRAVEL_SPEED := 6.0
const LAP_SPEED := 1.5
## Real seconds before a waypoint is given up.
const WAYPOINT_TIMEOUT_S := 25.0
## Camera distances: close while driving, playing height for the tracks.
const DRIVE_DISTANCE := 38.0
const TRACKS_DISTANCE := 75.0

var main: Node3D
var _dir := ""
var _prefix := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--track-shots="):
			_dir = arg.trim_prefix("--track-shots=")
	if _dir.is_empty():
		_dir = "user://track-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	await _run()
	get_tree().quit()


func _run() -> void:
	get_window().size = SHOT_SIZE
	main.camera_rig.edge_scroll_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	# Clean pictures: no HUD, ground tooltip or selection rings.
	main.get_node("Hud").visible = false
	main.selection.set_selection([])
	get_viewport().warp_mouse(Vector2(2.0, 2.0))
	await _frames(10)
	var sim_host: SimHost = main.sim_host
	var biome: String = main.atmosphere.preset.id if main.atmosphere.preset != null else "forest"
	var weather: String = main.weather_look.weather if main.weather_look != null else "clear"
	_prefix = "%s_%s" % [biome, weather]
	var tank := -1
	var truck := -1
	var ids := sim_host.unit_ids()
	var types := sim_host.unit_type_ids()
	var owners := sim_host.unit_owners()
	for i in ids.size():
		if owners[i] != sim_host.local_player():
			continue
		if types[i].contains("opel"):
			truck = ids[i]
		elif not types[i].contains("_inf_"):
			tank = ids[i]
	if tank < 0 or truck < 0:
		push_error("track-shots: the scenario needs an own tank and an own truck")
		return
	var grounds := WINTER_GROUNDS if biome == "winter" else GROUNDS
	for ground: String in grounds:
		var spot := _find(grounds[ground], _position(tank))
		if spot == Vector3.INF:
			print("track-shots: no open patch of %s on this map" % ground)
			continue
		print("track-shots: %s at %s" % [ground, spot])
		await _lap(tank, truck, spot, ground)
	sim_host.speed = 1.0
	print("track-shots: pictures in %s" % ProjectSettings.globalize_path(_dir))


## The patch of `terrains` nearest to `near` that holds the figure eight,
## or Vector3.INF.
func _find(terrains: Array, near: Vector3) -> Vector3:
	var sim_host: SimHost = main.sim_host
	var half: Vector2 = sim_host.map_size() * 0.5 - Vector2(60.0, 60.0)
	var reach := EIGHT + Vector2(6.0, 6.0)
	var best := Vector3.INF
	var best_d := INF
	var x := -half.x
	while x <= half.x:
		var z := -half.y
		while z <= half.y:
			var d := Vector2(x - near.x, z - near.z).length()
			if d < best_d and _share(terrains, Vector2(x, z), reach) >= MIN_SHARE:
				best_d = d
				best = Vector3(x, 0.0, z)
			z += SEARCH_STEP_M
		x += SEARCH_STEP_M
	return best


## Share of the samples in the rectangle `center` ± `reach` that are one of
## `terrains`; 0 at once if the center or any sample is deep water.
func _share(terrains: Array, center: Vector2, reach: Vector2) -> float:
	var sim_host: SimHost = main.sim_host
	if not int(sim_host.terrain_at(center.x, center.y)) in terrains:
		return 0.0
	var hits := 0
	var count := 0
	var total := (int(2.0 * reach.x / SAMPLE_STEP_M) + 1) * (int(2.0 * reach.y / SAMPLE_STEP_M) + 1)
	var misses_allowed := total * (1.0 - MIN_SHARE)
	var dx := -reach.x
	while dx <= reach.x:
		var dz := -reach.y
		while dz <= reach.y:
			var terrain := int(sim_host.terrain_at(center.x + dx, center.y + dz))
			if terrain == 7:
				return 0.0
			count += 1
			if terrain in terrains:
				hits += 1
			elif count - hits > misses_allowed:
				return 0.0
			dz += SAMPLE_STEP_M
		dx += SAMPLE_STEP_M
	return float(hits) / maxi(count, 1)


## Waypoint `k` of the figure eight around `spot`.
func _waypoint(spot: Vector3, k: int) -> Vector3:
	var t := TAU * float(posmod(k, WAYPOINTS)) / WAYPOINTS
	return spot + Vector3(EIGHT.x * sin(t), 0.0, EIGHT.y * sin(2.0 * t))


func _lap(tank: int, truck: int, spot: Vector3, ground: String) -> void:
	var sim_host: SimHost = main.sim_host
	# To the patch: the tank to the first waypoint, the truck behind it.
	sim_host.speed = TRAVEL_SPEED
	var tank_k := 0
	var truck_k := -TRUCK_LAG
	await _drive([tank, truck], [_waypoint(spot, tank_k), _waypoint(spot, truck_k)], true)
	sim_host.speed = LAP_SPEED
	var shot := false
	while tank_k < WAYPOINTS:
		tank_k += 1
		truck_k += 1
		await _drive([tank, truck], [_waypoint(spot, tank_k), _waypoint(spot, truck_k)], false)
		if not shot and tank_k >= WAYPOINTS / 2 - 2:
			shot = true
			main.camera_rig.snap_to(_position(tank), DRIVE_DISTANCE, 0.7)
			await _frames(2)
			_save("%s_drive" % ground)
	# The truck finishes its lap too.
	while truck_k < WAYPOINTS:
		truck_k += 1
		await _drive([truck], [_waypoint(spot, truck_k)], false)
	sim_host.move_units(PackedInt64Array([tank, truck]), _position(tank).x, _position(tank).z)
	await _frames(20)
	main.camera_rig.snap_to(spot, TRACKS_DISTANCE, 0.7)
	await _frames(30)
	_save("%s_tracks" % ground)


## Orders every unit to its target and waits until all arrived (or the
## timeout); the camera follows the first. `far`: a long way, no timeout
## per waypoint but per 100 m.
func _drive(units: Array, targets: Array, far: bool) -> void:
	var sim_host: SimHost = main.sim_host
	for n in units.size():
		var target: Vector3 = targets[n]
		sim_host.move_units(PackedInt64Array([units[n]]), target.x, target.z)
	var began := Time.get_ticks_msec()
	var timeout := WAYPOINT_TIMEOUT_S
	if far:
		timeout *= 1.0 + _position(units[0]).distance_to(targets[0]) / 100.0
	while (Time.get_ticks_msec() - began) / 1000.0 < timeout:
		var arrived := true
		for n in units.size():
			var at := _position(units[n])
			if Vector2(at.x - targets[n].x, at.z - targets[n].z).length() > ARRIVE_M:
				arrived = false
		if arrived:
			return
		main.camera_rig.snap_to(_position(units[0]), DRIVE_DISTANCE * 1.6, 0.7)
		await get_tree().process_frame


func _position(id: int) -> Vector3:
	var node: Node3D = main._unit_nodes.get(id)
	return node.global_position if node != null else Vector3.ZERO


func _save(name: String) -> void:
	var path := _dir.path_join("%s_%s.png" % [_prefix, name])
	get_viewport().get_texture().get_image().save_png(path)
	print("track-shots: %s" % path)


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
