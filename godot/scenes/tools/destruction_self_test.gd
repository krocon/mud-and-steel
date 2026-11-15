extends RefCounted
## Self-test part for destruction (prompt 020), run by `self_test.gd` after
## the combat test with its helpers (`_check`, `_frames`, `_ids_of`,
## `_positions_of`, `_indices`).
##
## - The own tanks attack the ground at the nearest building until the
##   simulation says it is a ruin: the house is drawn as its `Ruin` variant
##   (its MultiMesh instance hidden), debris and dust effects were played,
##   and the damaged stage came first.
## - Then they attack the nearest tree until a tree falls: it is drawn as a
##   falling tree with a stump.
## The replay test that follows re-simulates all of it to the same hash.

const TIME_SCALE := 4.0
const BUILDINGS: Array[String] = [
	"env_house_farm", "env_house_farm_brick", "env_house_stone", "env_house_mud", "env_barn", "env_church",
]
const TREES: Array[String] = ["env_tree_deciduous", "env_tree_conifer", "env_tree_birch"]
## Simulated seconds allowed per target.
const SECONDS_PER_TARGET := 120


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node = main.destruction_view
	var objects: Node3D = main.map_objects
	var tanks: Array[int] = []
	var alive: PackedByteArray = sim_host.unit_alive()
	var own: Array[int] = test._ids_of(true)
	var indices: Array[int] = test._indices(own)
	for k in own.size():
		if alive[indices[k]] != 0:
			tanks.append(own[k])
	test._check(not tanks.is_empty(), "no own tank left for the destruction test")
	if tanks.is_empty():
		return
	var center: Vector3 = test._positions_of(tanks)[0]

	var house := _nearest(sim_host, BUILDINGS, center)
	test._check(house >= 0, "no destructible building on '%s'" % sim_host.map_name())
	if house < 0:
		return
	var damaged_before: int = view.damaged
	Engine.time_scale = TIME_SCALE
	await _shell(test, tanks, house)
	Engine.time_scale = 1.0
	var state: Dictionary = sim_host.object_state(house)
	test._check(state.stage == "destroyed" and state.remains == "ruin",
		"building %d (%s) is %s after %d ticks" % [house, state.id, state.stage, sim_host.current_tick()])
	await test._frames(3)
	var events: Array = view.recent.filter(func(e: Dictionary) -> bool: return e.object == house)
	var stages: Array = events.map(func(e: Dictionary) -> String: return e.stage)
	test._check(stages.has("destroyed") and (stages.has("damaged") or view.damaged > damaged_before),
		"building events %s" % [stages])
	var ruin: Node3D = objects.swapped.get(house)
	test._check(ruin != null and ruin.get_child_count() > 0, "building %d is not drawn as a ruin" % house)
	test._check(objects.hidden.has(house), "the intact building %d is still drawn" % house)
	var counts: Dictionary = main.effects.counts
	test._check(counts.get("object_ruin", 0) >= 1 and counts.get("debris", 0) > 0,
		"no collapse effects: %s" % counts)

	var tree := _nearest(sim_host, TREES, center)
	test._check(tree >= 0, "no tree on '%s'" % sim_host.map_name())
	if tree < 0:
		return
	var fallen_before: int = counts.get("object_fallen", 0)
	Engine.time_scale = TIME_SCALE
	await _shell(test, tanks, tree, func() -> bool: return main.effects.counts.get("object_fallen", 0) > fallen_before)
	Engine.time_scale = 1.0
	await test._frames(3)
	var felled: Array = view.recent.filter(func(e: Dictionary) -> bool: return e.remains == "fallen" and e.stage == "destroyed")
	test._check(not felled.is_empty(), "no tree fell")
	if not felled.is_empty():
		var node: Node3D = objects.swapped.get(int(felled[-1].object))
		test._check(node != null and node.get_child_count() == 2, "felled tree %d not drawn with a stump" % felled[-1].object)
	print("self-test: destruction ok at tick %d (building %d a ruin, %d objects destroyed, %d damaged)" % [
		sim_host.current_tick(), house, view.destroyed, view.damaged
	])


## Orders `tanks` to fire at the ground under object `index` until it is
## destroyed (or `done` holds), then stops them.
func _shell(test: Node, tanks: Array[int], index: int, done := Callable()) -> void:
	var sim_host: SimHost = test.main.sim_host
	var at: Vector3 = sim_host.object_position(index)
	var ids := PackedInt64Array(tanks)
	sim_host.attack_ground(ids, at.x, at.z)
	var deadline := sim_host.current_tick() + SECONDS_PER_TARGET * sim_host.tick_rate()
	while sim_host.current_tick() < deadline:
		if sim_host.object_state(index).stage == "destroyed" or (done.is_valid() and done.call()):
			break
		await test.get_tree().process_frame
	sim_host.stop_units(ids)
	await test._ticks(2)


## The nearest object of one of `ids` that can still be destroyed, or -1.
func _nearest(sim_host: SimHost, ids: Array[String], center: Vector3) -> int:
	var best := -1
	var best_distance := INF
	for id in ids:
		var found := sim_host.destructible_objects_near(id, center.x, center.z, 2000.0)
		if found.is_empty():
			continue
		var p: Vector3 = sim_host.object_position(found[0])
		var d := Vector2(p.x - center.x, p.z - center.z).length()
		if d < best_distance:
			best_distance = d
			best = found[0]
	return best
