extends RefCounted
## Self-test part for vehicle tracks and ground effects (prompt 104), run by
## `self_test.gd` after the combat test with its helpers (`_check`,
## `_frames`, `_ids_of`, `_indices`).
##
## - The own tanks drove in the earlier parts: the raster holds stamps.
## - A drive over sand stamps along the way, the same drive on a road does
##   not (`ground_tracks.step` with a made-up vehicle on the map's sand and
##   road).
## - An enemy vehicle that is not visible stamps nothing, a visible one does
##   (`ground_tracks._collect` with the fog's `visible_at` replaced).
## - In the rain the vehicles throw mud on bare earth and meadow instead of
##   dust, and tracks in sand fade sooner.

## Made-up unit ids of the drives over sand and road.
const SAND_ID := -104001
const ROAD_ID := -104002
## Length of a test drive (m).
const DRIVE_M := 4.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var tracks: Node = main.get("ground_tracks")
	test._check(tracks != null, "no ground tracks")
	if tracks == null:
		return
	test._check(tracks.window_m > 0.0, "ground tracks are off at quality '%s'" % main.atmosphere.quality)
	test._check(tracks.stamps > 0, "the tanks drove, but no track was stamped")
	# Fields (prompt 088) take tracks like meadow, ploughed soil like mud.
	for field: int in [9, 10, 11]:
		test._check(not tracks.track_ground(field).is_empty(), "field terrain %d takes no tracks" % field)

	# Over sand, then on a road.
	var info: Dictionary = tracks._info("de_pz4_h")
	test._check(not info.is_empty() and info.width > 2.0, "no track layout of the Panzer IV: %s" % info)
	test._check(tracks._info("de_inf_rifle_squad").is_empty(), "infantry would leave vehicle tracks")
	var sand := _find(sim_host, 3)
	var road := _find(sim_host, 2)
	test._check(not sand.is_empty() and not road.is_empty(), "no sand or road on the test map")
	if not sand.is_empty() and not road.is_empty():
		var before: int = tracks.stamps
		_drive(tracks, SAND_ID, sand, info)
		test._check(tracks.stamps > before, "a drive over sand stamped nothing")
		test._check(tracks.amount_at(sand[0]) > 0.0, "the raster is empty on the drive over sand")
		before = tracks.stamps
		_drive(tracks, ROAD_ID, road, info)
		test._check(tracks.stamps == before, "a drive on a road stamped %d tracks" % (tracks.stamps - before))
		test._check(is_zero_approx(tracks.amount_at(road[0])), "the raster is set on the road")

	# An unseen enemy vehicle stamps nothing; seen, it does.
	var enemy := _enemy_vehicle(test, sim_host, tracks)
	test._check(enemy >= 0, "no enemy vehicle left for the track test")
	if enemy >= 0:
		var node: Node3D = main._unit_nodes[enemy]
		var at := node.global_position
		var visible_at: Callable = main.effects.visible_at
		main.effects.visible_at = func(_point: Vector3) -> bool: return false
		tracks._last[enemy] = [at + Vector3(DRIVE_M * 0.5, 0.0, 0.0), 0.0]
		var before: int = tracks.stamps
		tracks._collect()
		test._check(tracks.stamps == before, "an enemy vehicle out of sight left a track")
		main.effects.visible_at = func(_point: Vector3) -> bool: return true
		tracks._last[enemy] = [at + Vector3(DRIVE_M * 0.5, 0.0, 0.0), 0.0]
		tracks._collect()
		var soft: bool = tracks.ground_params(at)[0] > 0.0
		test._check(not soft or tracks.stamps > before, "a visible enemy vehicle left no track")
		main.effects.visible_at = visible_at

	# Ground effects by weather.
	var effects: Node3D = main.effects
	test._check(effects.drive_effect(0, false) == "dust", "dry meadow does not raise dust: %s" % effects.drive_effect(0, false))
	test._check(effects.drive_effect(4, false) == "mud", "mud does not throw clods: %s" % effects.drive_effect(4, false))
	test._check(effects.drive_effect(6, false) == "wake", "a ford does not splash: %s" % effects.drive_effect(6, false))
	test._check(effects.drive_effect(5, false) == "snow", "snow does not raise snow dust: %s" % effects.drive_effect(5, false))
	var look: Node = main.weather_look
	var wetness: float = look.wetness
	look.wetness = 0.6
	test._check(effects.drive_effect(0, effects.wet()) == "mud", "in the rain bare earth still raises dust")
	if not sand.is_empty():
		var rain_life: float = tracks.ground_params(sand[0])[1]
		look.wetness = 0.0
		var dry_life: float = tracks.ground_params(sand[0])[1]
		test._check(rain_life < dry_life, "tracks in sand last as long in the rain (%.0f s, dry %.0f s)" % [rain_life, dry_life])
	look.wetness = wetness
	print("self-test: tracks ok: %d stamps, window %.0f m at %.2f m per texel" % [tracks.stamps, tracks.window_m, tracks.texel_m])


## A point of `terrain` with the same terrain ± DRIVE_M / 2 along x or
## along z: [point, direction], or [] if there is none.
func _find(sim_host: SimHost, terrain: int) -> Array:
	var half: Vector2 = sim_host.map_size() * 0.5 - Vector2(20.0, 20.0)
	var x := -half.x
	while x <= half.x:
		var z := -half.y
		while z <= half.y:
			for direction: Vector3 in [Vector3.RIGHT, Vector3.BACK]:
				if _all(sim_host, terrain, Vector3(x, 0.0, z), direction):
					return [Vector3(x, 0.0, z), direction]
			z += 1.0
		x += 1.0
	return []


func _all(sim_host: SimHost, terrain: int, at: Vector3, direction: Vector3) -> bool:
	for k in 5:
		var p := at + direction * DRIVE_M * (k / 4.0 - 0.5)
		if int(sim_host.terrain_at(p.x, p.z)) != terrain:
			return false
	return true


## A made-up vehicle drives DRIVE_M along `found[1]` through `found[0]`, seen.
func _drive(tracks: Node, id: int, found: Array, info: Dictionary) -> void:
	var center: Vector3 = found[0]
	var direction: Vector3 = found[1]
	var steps := int(DRIVE_M / tracks.STEP_M)
	for k in steps + 1:
		var at: Vector3 = center + direction * (-DRIVE_M * 0.5 + k * tracks.STEP_M)
		tracks.step(id, at, atan2(direction.x, direction.z), info, true)


func _enemy_vehicle(test: Node, sim_host: SimHost, tracks: Node) -> int:
	var enemies: Array[int] = test._ids_of(false)
	var alive := sim_host.unit_alive()
	var indices: Array[int] = test._indices(enemies)
	var types := sim_host.unit_type_ids()
	for k in enemies.size():
		var i := indices[k]
		if alive[i] != 0 and not tracks._info(types[i]).is_empty() and test.main._unit_nodes.has(enemies[k]):
			return enemies[k]
	return -1
