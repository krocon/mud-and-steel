extends RefCounted
## Self-test part for infantry (prompt 022), run by `self_test.gd` with its
## helpers (`_check`, `_frames`, `_ticks`, `_key`, `_right_click`,
## `_screen_of_point`, `_indices`, `_moving`).
##
## - Every squad has its soldiers in the snapshot, and the squad scene's
##   members stand where the simulation puts them.
## - V makes the selected squads go prone (members play `prone`), B returns
##   to the automatic stance.
## - A right click on the church sends both own squads there: the church
##   holds 16 soldiers, so the first squad (9) gets in and is hidden, the
##   second stays outside; a ruin reports its condition.
## - A move order takes them out again.

const TIME_SCALE := 4.0
## The church of `test_small` (map object, meters) and a farmhouse ruin.
const CHURCH := Vector3(-105.578, 0.0, 139.732)
const RUIN := Vector3(-117.676, 0.0, 161.98)


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var own := _squads(sim_host, true)
	var enemy := _squads(sim_host, false)
	test._check(own.size() == 2 and enemy.size() == 2, "expected 2 squads per side, got %d/%d" % [own.size(), enemy.size()])
	if own.size() != 2:
		return
	main.camera_rig.snap_to(_position(sim_host, own[0]), 60.0, 0.0)
	await test._frames(3)
	_check_members(test, main, own)

	# Stance keys.
	main.selection._set_selection(own)
	test._key(KEY_V)
	await test._ticks(20)
	await test._frames(2)
	var stances := sim_host.unit_stances()
	for i in test._indices(own):
		test._check(stances[i] == 2, "V left squad %d in stance %d" % [sim_host.unit_ids()[i], stances[i]])
	var prone := 0
	for s: Dictionary in main.infantry_view._soldiers.values():
		if s["clip"] == "prone":
			prone += 1
	test._check(prone >= 9, "only %d members play 'prone'" % prone)
	# Prompt 048: they dived down on the way.
	var motion_counts: Dictionary = main.infantry_view.motion.counts
	test._check(int(motion_counts["dive"]) >= 9, "only %d members dived: %s" % [motion_counts["dive"], motion_counts])
	test._key(KEY_B)
	await test._ticks(20)
	stances = sim_host.unit_stances()
	for i in test._indices(own):
		test._check(stances[i] == 1, "B left squad %d in stance %d (expected kneeling)" % [sim_host.unit_ids()[i], stances[i]])

	# Buildings.
	var church := sim_host.building_at(CHURCH.x, CHURCH.z)
	test._check(church >= 0, "no building at the church")
	var ruin := sim_host.building_at(RUIN.x, RUIN.z)
	test._check(ruin >= 0 and sim_host.building_info(ruin)["condition"] == "ruin",
		"farmhouse ruin: %s" % (sim_host.building_info(ruin) if ruin >= 0 else {}))
	if church < 0:
		return
	var point := CHURCH
	point.y = sim_host.height_at(point.x, point.z)
	main.camera_rig.snap_to(point, 160.0, 0.0)
	await test._frames(2)
	test._right_click(test._screen_of_point(point))
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + 2400
	await test._ticks(5)
	while sim_host.current_tick() < deadline and (test._moving(own) or _inside(test, sim_host, own, church).is_empty()):
		await test._frames(1)
	Engine.time_scale = 1.0
	var inside := _inside(test, sim_host, own, church)
	test._check(inside.size() == 1, "%d squads in the church (capacity 16) by tick %d: %s" % [
		inside.size(), sim_host.current_tick(), sim_host.unit_garrisons()])
	var info := sim_host.building_info(church)
	test._check(info["occupants"] == 9 and info["owner"] == sim_host.local_player(), "church: %s" % info)
	await test._frames(2)
	var shown := 0
	for s: Dictionary in main.infantry_view._soldiers.values():
		var unit := int(s["node"].get_parent().get_parent().name.trim_prefix("Unit"))
		if unit in inside and (s["node"] as Node3D).visible:
			shown += 1
	test._check(shown == 0, "%d soldiers in the church are drawn" % shown)

	# Out again.
	# Away from the church and out of the line of fire of the destruction
	# test that follows (it shells the nearest building).
	var outside := point + Vector3(60.0, 0.0, 30.0)
	sim_host.move_units(PackedInt64Array(own), outside.x, outside.z)
	await test._ticks(5)
	var garrisons := sim_host.unit_garrisons()
	for i in test._indices(own):
		test._check(garrisons[i] == -1, "squad %d still in building %d after a move order" % [sim_host.unit_ids()[i], garrisons[i]])
	print("self-test: infantry ok (stances, %d members prone, church %d with %d soldiers, left at tick %d)" % [
		prone, church, info["occupants"], sim_host.current_tick()])


func _squads(sim_host: SimHost, own: bool) -> Array[int]:
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var stances := sim_host.unit_stances()
	var result: Array[int] = []
	for i in ids.size():
		if stances[i] >= 0 and (owners[i] == sim_host.local_player()) == own:
			result.append(ids[i])
	result.sort()
	return result


func _position(sim_host: SimHost, id: int) -> Vector3:
	var ids := sim_host.unit_ids()
	for i in ids.size():
		if ids[i] == id:
			return sim_host.current_positions()[i]
	return Vector3.ZERO


## The squads of `ids` that are in `building`.
func _inside(test: Node, sim_host: SimHost, ids: Array[int], building: int) -> Array[int]:
	var garrisons := sim_host.unit_garrisons()
	var all := sim_host.unit_ids()
	var result: Array[int] = []
	for i in test._indices(ids):
		if garrisons[i] == building:
			result.append(all[i])
	return result


## Members of the own squads exist, use the squad scene and stand within
## a few centimeters (plus interpolation) of their soldiers.
func _check_members(test: Node, main: Node3D, own: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	var units := sim_host.soldier_units()
	var indices := sim_host.soldier_indices()
	var positions := sim_host.soldier_positions()
	var checked := 0
	for k in units.size():
		if not (units[k] in own):
			continue
		var node: Node3D = main._unit_nodes.get(units[k])
		var model := node.get_node_or_null("Model") if node != null else null
		test._check(model != null and model.has_meta(&"squad"), "squad %d has no squad scene" % units[k])
		if model == null:
			return
		var member := model.get_node_or_null("Member_%02d" % (indices[k] + 1)) as Node3D
		test._check(member != null, "squad %d has no member %d" % [units[k], indices[k] + 1])
		if member == null:
			continue
		var off := member.global_position.distance_to(positions[k])
		test._check(off < 0.5, "member %d of squad %d stands %.2f m from its soldier" % [indices[k] + 1, units[k], off])
		checked += 1
	test._check(checked == 18, "checked %d members of the own squads, expected 18" % checked)
