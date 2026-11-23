extends RefCounted
## Self-test part for destruction (prompt 020), run by `self_test.gd` after
## the combat test with its helpers (`_check`, `_frames`, `_ids_of`,
## `_positions_of`, `_indices`).
##
## - The own tanks get the order "shell building" (prompt 093) on the
##   nearest building, given like the player does: the order waits for its
##   click, the building under the cursor is the outlined target, the click
##   sends the order. They shell it until the simulation says it is a ruin
##   and then stop firing by themselves: the house is drawn as its `Ruin`
##   variant (its MultiMesh instance hidden), debris and dust effects were
##   played, and the damaged stage came first. It falls storey by storey
##   (prompt 197): roof first, ground storey last, then only the ruin.
## - Then they attack the nearest tree until a tree falls: it is drawn as a
##   falling tree with a stump.
## The replay test that follows re-simulates all of it to the same hash.

const ShellOrders := preload("res://scenes/selection/shell_orders.gd")
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
	await _order_shelling(test, tanks, house)
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + SECONDS_PER_TARGET * sim_host.tick_rate()
	while sim_host.current_tick() < deadline and sim_host.object_state(house).stage != "destroyed":
		await test.get_tree().process_frame
	# The order ends with the ruin: the tanks stand, and the ruin is no
	# target for another order.
	await test._ticks(40)
	Engine.time_scale = 1.0
	test._check(_tanks_stand(test, tanks), "tanks still drive after building %d became a ruin" % house)
	var house_at: Vector3 = sim_host.object_position(house)
	test._check(sim_host.shell_target_at(house_at.x, house_at.z, 0.0) != house,
		"the ruin %d still counts as a target to shell" % house)
	sim_host.stop_units(PackedInt64Array(tanks))
	await test._ticks(2)
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
	await _check_collapse(test, house)

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


## The house fell storey by storey (prompt 197): the bands from the top
## down (roof first, ground storey last), each with its dust, and at the end
## only the ruin is left.
func _check_collapse(test: Node, house: int) -> void:
	var objects: Node3D = test.main.map_objects
	var bands: Array = objects._collapse_bands(house)
	test._check(bands.size() >= 2, "building %d has collapse bands %s" % [house, bands])
	var deadline := Time.get_ticks_msec() + 20000
	while objects.collapsing.has(house) and not objects.collapsing[house].finished \
			and Time.get_ticks_msec() < deadline:
		await test.get_tree().process_frame
	await test._frames(2)
	var fell: Array = objects.collapse_log.get(house, [])
	var names: Array = bands.map(func(b: Dictionary) -> String: return b.name)
	test._check(fell == names, "building %d fell in bands %s, expected %s" % [house, fell, names])
	for k in range(1, bands.size()):
		test._check(float(bands[k].bottom) < float(bands[k - 1].bottom),
			"band %s is not below %s" % [bands[k].name, bands[k - 1].name])
	test._check(not names.is_empty() and names[-1] == "storey_1", "the ground storey does not fall last: %s" % [names])
	test._check(test.main.effects.counts.get("collapse_stage", 0) >= names.size(),
		"dust per storey: %s" % test.main.effects.counts)
	var ruin: Node3D = objects.swapped.get(house)
	var standing := 0
	if ruin != null:
		for child in ruin.get_children():
			if child.name == "Collapse":
				test._check(child.finished and child.get_node_or_null("Model") == null,
					"the collapsing model of %d is still drawn" % house)
			else:
				standing += 1
	test._check(standing == 1, "building %d ends with %d nodes besides the collapse, not the ruin" % [house, standing])
	print("self-test: building %d collapsed in %s" % [house, fell])


## Gives `tanks` the order to shell building `house` the way the player
## does: select them, start the order (button of the command bar), point at
## the building, click.
func _order_shelling(test: Node, tanks: Array[int], house: int) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var at: Vector3 = sim_host.object_position(house)
	at.y = sim_host.height_at(at.x, at.z)
	main.camera_rig.snap_to(at, 120.0, 0.0)
	await test._frames(3)
	selection._set_selection(tanks)
	test._check(sim_host.can_shell_objects(PackedInt64Array(tanks)), "the tanks cannot shell objects")
	var button: Button = main.hud.command_bar.buttons.get("attack_object")
	test._check(button != null, "no 'shell building' button in the command bar")
	main.hud.command_bar.press("attack_object")
	test._check(selection.command_mode == ShellOrders.MODE, "the shelling order does not wait for a click")
	# The cursor on the wall of the house, a little above the ground.
	var screen: Vector2 = main.camera.unproject_position(at + Vector3.UP * 1.5)
	var picked: int = ShellOrders.target_at(selection, screen)
	test._check(picked == house, "the cursor on building %d picks object %d" % [house, picked])
	var ordered: Array[int] = []
	var note := func(object: int) -> void: ordered.append(object)
	selection.object_attack_ordered.connect(note)
	if picked == house:
		selection.click_command(screen)
	else:
		sim_host.attack_object(PackedInt64Array(tanks), house)
		selection.set_command_mode("")
	selection.object_attack_ordered.disconnect(note)
	test._check(picked != house or ordered == [house], "the click ordered %s instead of building %d" % [ordered, house])
	test._check(selection.command_mode.is_empty(), "the shelling order stays armed after its click")
	await test._ticks(2)


## Whether none of `tanks` drives.
func _tanks_stand(test: Node, tanks: Array[int]) -> bool:
	var moving: PackedByteArray = test.main.sim_host.unit_moving()
	for index: int in test._indices(tanks):
		if index >= 0 and index < moving.size() and moving[index] != 0:
			return false
	return true


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
