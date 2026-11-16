extends RefCounted
## Self-test part for fire (prompt 041), run by `self_test.gd` after the
## destruction test with its helpers (`_check`, `_ticks`, `_ids_of`,
## `_positions_of`, `_indices`).
##
## The own tanks fire high explosive at the ground next to the nearest hay,
## firewood, barn or farmhouse until something burns (shell bursts and hits
## roll for fire in the simulation), trying the next target when one is
## destroyed without catching fire. Then `fire_view.gd` must draw flames
## for it; the replay test that follows re-simulates the fire.

const TIME_SCALE := 4.0
const TARGETS: Array[String] = ["env_hay_bale", "env_woodpile", "env_barn", "env_house_farm"]
## Simulated seconds allowed per target.
const SECONDS_PER_TARGET := 90
const MAX_TARGETS := 5


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node = main.fire_view
	test._check(view != null, "no fire view")
	if view == null:
		return
	var tanks: Array[int] = []
	var alive: PackedByteArray = sim_host.unit_alive()
	var own: Array[int] = test._ids_of(true)
	var indices: Array[int] = test._indices(own)
	for k in own.size():
		if alive[indices[k]] != 0:
			tanks.append(own[k])
	test._check(not tanks.is_empty(), "no own tank left for the fire test")
	if tanks.is_empty():
		return
	var center: Vector3 = test._positions_of(tanks)[0]
	var tried: Dictionary[int, bool] = {}
	var burning := -1
	Engine.time_scale = TIME_SCALE
	for attempt in MAX_TARGETS:
		burning = _object_fire(sim_host)
		if burning >= 0:
			break
		var target := _nearest(sim_host, center, tried)
		if target < 0:
			break
		tried[target] = true
		burning = await _shell(test, tanks, target)
		if burning >= 0:
			break
	Engine.time_scale = 1.0
	test._check(burning >= 0, "nothing caught fire after %d targets" % tried.size())
	if burning < 0:
		return
	await test._ticks(10)
	test._check(view.started > 0 and view.flames() > 0,
		"burning object %d is not drawn (started %d, flames %d)" % [burning, view.started, view.flames()])
	print("self-test: fire ok at tick %d (object %d burns, %d fires drawn)" % [
		sim_host.current_tick(), burning, view.flames()])


## Index of a burning map object, or -1.
func _object_fire(sim_host: SimHost) -> int:
	for key in sim_host.fire_keys():
		if key >= 0:
			return key
	return -1


## Fires at the ground 2 m beside `index` until any object burns (its
## index) or the target is destroyed or the time is up (-1).
func _shell(test: Node, tanks: Array[int], index: int) -> int:
	var sim_host: SimHost = test.main.sim_host
	var at: Vector3 = sim_host.object_position(index)
	var ids := PackedInt64Array(tanks)
	sim_host.attack_ground(ids, at.x + 2.0, at.z)
	var deadline := sim_host.current_tick() + SECONDS_PER_TARGET * sim_host.tick_rate()
	var found := -1
	while sim_host.current_tick() < deadline:
		found = _object_fire(sim_host)
		if found >= 0 or sim_host.object_state(index).stage == "destroyed":
			break
		await test.get_tree().process_frame
	sim_host.stop_units(ids)
	await test._ticks(2)
	return found


## The nearest object of `TARGETS` that can still burn, or -1.
func _nearest(sim_host: SimHost, center: Vector3, tried: Dictionary[int, bool]) -> int:
	var best := -1
	var best_distance := INF
	for id in TARGETS:
		for index in sim_host.destructible_objects_near(id, center.x, center.z, 2000.0):
			if tried.has(index):
				continue
			var p: Vector3 = sim_host.object_position(index)
			var d := Vector2(p.x - center.x, p.z - center.z).length()
			if d < best_distance:
				best_distance = d
				best = index
			break
	return best
