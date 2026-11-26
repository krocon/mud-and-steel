extends Node
## Self-test and pictures of the gun crews drawn at the guns (prompt 193),
## started by `main.gd` on `data/scenarios/gun_crews.toml`:
##
##     godot --path godot --headless --fixed-fps 60 res://scenes/main.tscn -- --scenario=gun_crews.toml --gun-crew-test
##
## Part of `just selftest`. Checks that every crewed gun shows one figure
## per man of the simulation's count (`SimHost.unit_crews`): at the start,
## after two losses (two bodies fall, the layer stays), with no man left
## (nobody at the gun), pinned down (layer behind the shield, the others
## flat), while the battery fires (the layer cranks, the loader loads), when
## the tractor hitches a gun (nobody drawn) and after unlimbering (all walk
## back to their places). Exit code 0 = passed.
##
## With `--gun-crew-shots=<dir>` (needs a window, `just gun-crew-shots`) it
## takes the pictures of the prompt instead: the leFH 18 battery loading and
## firing, after two losses, pinned down and abandoned, Pak 40 and Flak 38,
## and the Soviet and American guns, each from game height and close up.

const TIME_SCALE := 4.0
const SCREEN := Vector2i(1280, 720)
## Camera distances of the pictures: close up and game height.
const NEAR_M := 16.0
const GAME_M := 55.0
## The enemy truck's start (out of sight: not in the snapshot).
const ENEMY_START := Vector3(0.0, 0.0, -400.0)

var main: Node3D
var _failures := 0
var _shots := ""
var _shot_count := 0


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--gun-crew-shots="):
			_shots = arg.trim_prefix("--gun-crew-shots=")
	if _shots.is_empty() and get_tree().root.size.x < SCREEN.x:
		get_tree().root.size = SCREEN
	await _run()
	Engine.time_scale = 1.0
	if _shots.is_empty():
		print("self-test: gun crews %s (%d failures)" % ["ok" if _failures == 0 else "FAILED", _failures])
	else:
		print("gun-crew-shots: %d pictures in %s" % [_shot_count, _shots])
	get_tree().quit(1 if _failures > 0 else 0)


func _run() -> void:
	var sim_host: SimHost = main.sim_host
	var view: Node = main.gun_crew_view
	var own := PackedInt64Array()
	for i in sim_host.unit_ids().size():
		if sim_host.unit_owners()[i] == sim_host.local_player():
			own.append(sim_host.unit_ids()[i])
	sim_host.set_hold_fire(own, true)
	main.camera_rig.edge_scroll_enabled = false
	var battery := _all("de_lefh18")
	var truck := _first("de_opel_blitz")
	var pak := _first("de_pak40")
	var flak := _first("de_flak38")
	_check(battery.size() == 6 and truck >= 0 and pak >= 0 and flak >= 0, "gun_crews.toml units missing")
	if _failures > 0:
		return
	main.camera_rig.snap_to(_position(battery[0]), 60.0, 0.0)
	await _frames(20)
	# Every crewed gun: one figure per man.
	var crewed := 0
	for id in sim_host.unit_ids():
		var crew := _crew(id)
		if crew[0] < 0:
			continue
		crewed += 1
		_check(view.figures_at(id) == crew[0], "gun %d: %d figures, %d men" % [id, view.figures_at(id), crew[0]])
	_check(crewed == 13, "%d crewed guns, expected 13" % crewed)
	var ml20 := _first("su_ml20")
	_check(ml20 >= 0 and view.figures_at(ml20) == 9, "su_ml20 missing or wrong crew count (%d)" % (view.figures_at(ml20) if ml20 >= 0 else -1))

	if not _shots.is_empty():
		await _pictures(battery, pak, flak)
		return

	# Two men lost: two bodies, the layer stays.
	var gun: int = battery[1]
	var layer: Node3D = view.figures_of(gun)[0].node
	var fallen: int = view.fallen_total
	sim_host.debug_gun_crew(gun, 2, false)
	await _frames(3)
	var men: int = _crew(gun)[0]
	_check(men == 4, "gun %d has %d men after two losses, expected 4" % [gun, men])
	_check(view.figures_at(gun) == men, "gun %d: %d figures after two losses, %d men" % [gun, view.figures_at(gun), men])
	_check(view.fallen_total - fallen == 2, "%d figures fell for two losses" % [view.fallen_total - fallen])
	_check(is_instance_valid(layer) and view.figures_of(gun)[0].node == layer, "the layer fell while other men were left")
	var places: Array[int] = []
	for f in view.figures_of(gun):
		places.append(int(f.place))
	_check(places == [0, 1, 2, 3], "places after two losses: %s" % [places])

	# Nobody left: an abandoned gun has nobody at it.
	gun = battery[2]
	sim_host.debug_gun_crew(gun, 99, false)
	await _frames(3)
	_check(_crew(gun)[0] == 0 and view.figures_at(gun) == 0, "abandoned gun %d: %d men, %d figures" % [
		gun, _crew(gun)[0], view.figures_at(gun)])

	# Pinned down: behind the shield and flat on the ground.
	gun = battery[3]
	sim_host.debug_gun_crew(gun, 0, true)
	await _frames(6)
	_check(_crew(gun)[2] & 1 != 0, "gun %d is not pinned down" % gun)
	var clips: Array[String] = []
	for f in view.figures_of(gun):
		clips.append(str(f.clip))
	_check(clips.size() == 6 and clips[0] == "cover_low" and clips[5] == "prone", "pinned crew clips: %s" % [clips])

	# The battery fires: the layer cranks, the loader loads.
	var firing := PackedInt64Array([battery[0], battery[4], battery[5]])
	var target := ENEMY_START
	sim_host.barrage(firing, target.x, target.z, 40, 3, false)
	Engine.time_scale = TIME_SCALE
	# Lambdas capture locals by value: the flags live in a dictionary.
	var seen := {"loading": false, "aiming": false}
	await _until(func() -> bool:
		for f in view.figures_of(battery[0]):
			if f.role == "loader" and f.clip == "gun_load":
				seen["loading"] = true
			if f.role == "layer" and f.clip == "gun_aim":
				seen["aiming"] = true
		return seen["loading"] and seen["aiming"], 3000)
	Engine.time_scale = 1.0
	var aiming: bool = seen["aiming"]
	var loading: bool = seen["loading"]
	_check(aiming, "the layer of the firing gun %d never cranked" % battery[0])
	_check(loading, "the loader of the firing gun %d never loaded" % battery[0])

	# Hitched: the men ride the tractor and split trails fold closed (prompt 227).
	# Unlimbered: the trails spread back to rest and the men walk back.
	var pak_model: Node3D = (main._unit_nodes[pak] as Node3D).get_node("Model")
	var trail_l: Node3D = pak_model.find_child("Trail_L", true, false) as Node3D
	var trail_r: Node3D = pak_model.find_child("Trail_R", true, false) as Node3D
	_check(trail_l != null and trail_r != null, "Pak 40 missing Trail_L / Trail_R")
	if trail_l != null and trail_r != null:
		_check(absf(trail_l.rotation.y) < 0.02 and absf(trail_r.rotation.y) < 0.02,
			"unlimbered Pak 40 trails not in firing rest pose (%.3f, %.3f)" % [trail_l.rotation.y, trail_r.rotation.y])
	Engine.time_scale = TIME_SCALE
	sim_host.tow_gun(PackedInt64Array([truck]), pak)
	await _until(func() -> bool: return _carrier(pak) == truck and not _loading(pak), 3000)
	await _frames(30)
	_check(_carrier(pak) == truck, "the truck did not hitch the Pak 40")
	_check(view.figures_at(pak) == _crew(pak)[0] and view.drawn_at(pak) == 0, "towed Pak 40: %d figures, %d drawn" % [
		view.figures_at(pak), view.drawn_at(pak)])
	if trail_l != null and trail_r != null:
		_check(trail_l.rotation.y > 0.35 and trail_r.rotation.y < -0.35,
			"towed Pak 40 trails did not fold closed (%.3f, %.3f)" % [trail_l.rotation.y, trail_r.rotation.y])
	sim_host.move_units(PackedInt64Array([truck]), _position(truck).x - 40.0, _position(truck).z)
	await _until(func() -> bool: return not _moving(truck), 2400)
	sim_host.unlimber_units(PackedInt64Array([truck]))
	await _until(func() -> bool: return _carrier(pak) < 0 and not _loading(pak), 2400)
	await _frames(2)
	var walking := 0
	for f in view.figures_of(pak):
		if f.walk_to != Vector3.INF:
			walking += 1
	_check(_carrier(pak) < 0, "the Pak 40 was not unlimbered")
	_check(view.drawn_at(pak) == _crew(pak)[0] and walking > 0, "unlimbered Pak 40: %d drawn of %d men, %d walking" % [
		view.drawn_at(pak), _crew(pak)[0], walking])
	await _until(func() -> bool:
		return view.figures_of(pak).all(func(f) -> bool: return f.walk_to == Vector3.INF), 600)
	await _frames(20)
	Engine.time_scale = 1.0
	if trail_l != null and trail_r != null:
		_check(absf(trail_l.rotation.y) < 0.02 and absf(trail_r.rotation.y) < 0.02,
			"unlimbered Pak 40 trails did not spread back (%.3f, %.3f)" % [trail_l.rotation.y, trail_r.rotation.y])
	_check(view.figures_of(pak).all(func(f) -> bool: return f.walk_to == Vector3.INF),
		"the Pak 40 crew did not reach its places")
	print("self-test: gun crews: %d guns, %d bodies, crews follow losses, pins, fire and towing" % [crewed, view.fallen_total])


## The pictures of the prompt (needs a window).
func _pictures(battery: Array[int], pak: int, flak: int) -> void:
	var sim_host: SimHost = main.sim_host
	var view: Node = main.gun_crew_view
	DirAccess.make_dir_recursive_absolute(_shots)
	var target := ENEMY_START
	sim_host.barrage(PackedInt64Array(battery), target.x, target.z, 60, 40, false)
	# Loading: wait until the loader of gun 0 carries a shell.
	await _until(func() -> bool:
		for f in view.figures_of(battery[0]):
			if f.role == "loader" and f.clip == "gun_load":
				return true
		return false, 4000)
	await _seconds(0.6)
	await _pair("lefh18_loading", battery[0])
	# Firing: right after a shot of gun 0.
	var shots_before: int = main.combat_view.shots
	await _until(func() -> bool: return main.combat_view.shots > shots_before, 4000)
	await _pair("lefh18_firing", battery[0], 0.05)
	sim_host.debug_gun_crew(battery[1], 2, false)
	await _seconds(2.5)
	await _pair("lefh18_two_lost", battery[1])
	sim_host.debug_gun_crew(battery[2], 0, true)
	await _seconds(1.0)
	await _pair("lefh18_pinned", battery[2])
	sim_host.debug_gun_crew(battery[3], 99, false)
	await _seconds(3.0)
	await _pair("lefh18_abandoned", battery[3])
	await _pair("pak40", pak)
	await _pair("flak38", flak)
	for id in ["su_zis3", "su_m30", "us_57mm_m1", "us_bofors_m1"]:
		await _pair(id.trim_prefix("su_").trim_prefix("us_"), _first(id))
	# The whole battery from game height.
	main.camera_rig.snap_to(_position(battery[2]).lerp(_position(battery[3]), 0.5), 90.0, 0.4)
	await _seconds(1.0)
	_save("battery_game")


## Close and game-height picture of gun `id`.
func _pair(name: String, id: int, wait := 0.4) -> void:
	var at := _position(id)
	main.camera_rig.snap_to(at, NEAR_M, 0.6)
	await _seconds(wait)
	_save(name + "_near")
	main.camera_rig.snap_to(at, GAME_M, 0.6)
	await _seconds(0.4)
	_save(name + "_game")


func _save(name: String) -> void:
	var path := _shots.path_join(name + ".png")
	if get_viewport().get_texture().get_image().save_png(path) == OK:
		_shot_count += 1
		print("gun-crew-shots: %s" % path)


func _seconds(seconds: float) -> void:
	var until := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < until:
		await _frames(1)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("self-test: gun crews: " + message)


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame


func _until(done: Callable, ticks: int) -> void:
	var sim_host: SimHost = main.sim_host
	var deadline := sim_host.current_tick() + ticks
	var frames := ticks * 10
	while sim_host.current_tick() < deadline and frames > 0 and not done.call():
		frames -= 1
		await _frames(1)


## Men, full, flags, reload ticks of unit `id` (men -1: no crew).
func _crew(id: int) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var i := sim_host.unit_ids().find(id)
	var crews := sim_host.unit_crews()
	if i < 0 or crews.size() < (i + 1) * 4:
		return [-1, 0, 0, 0]
	return [crews[i * 4], crews[i * 4 + 1], crews[i * 4 + 2], crews[i * 4 + 3]]


func _first(type_id: String) -> int:
	var all := _all(type_id)
	return all[0] if not all.is_empty() else -1


func _all(type_id: String) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	var result: Array[int] = []
	for i in types.size():
		if types[i] == type_id:
			result.append(ids[i])
	return result


func _index(id: int) -> int:
	return main.sim_host.unit_ids().find(id)


func _position(id: int) -> Vector3:
	var i := _index(id)
	return main.sim_host.current_positions()[i] if i >= 0 else Vector3.ZERO


func _carrier(id: int) -> int:
	var i := _index(id)
	return main.sim_host.unit_carriers()[i] if i >= 0 else -1


func _loading(id: int) -> bool:
	var i := _index(id)
	return i >= 0 and main.sim_host.unit_loading()[i] != 0


func _moving(id: int) -> bool:
	var i := _index(id)
	return i >= 0 and main.sim_host.unit_moving()[i] != 0
