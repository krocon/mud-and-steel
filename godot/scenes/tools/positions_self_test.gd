extends RefCounted
## Self-test part for positions tier 2 (prompt 052), run by `self_test.gd`
## after the works test (a debug strongpoint is no command either, so it
## comes after the replay test).
##
## A neutral strongpoint (`SimHost.debug_strongpoint`) is built beside an
## own squad: the works view draws its trenches, nests and dugouts as
## procedural meshes, the squad that walks in stands at firing positions,
## the command bar has the new engineer buttons (hidden without engineers)
## and an engineer line shows its length and price while it is drawn.

const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const TIME_SCALE := 4.0
## The strongpoint lies this far (m) from the squad, towards the map center.
const OFFSET_M := 45.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var squads := _own_squads(sim_host)
	test._check(not squads.is_empty(), "positions: no own squad left")
	if squads.is_empty():
		return
	var squad: int = squads[0]
	var at: Vector3 = sim_host.current_positions()[Array(sim_host.unit_ids()).find(squad)]
	var dir := Vector3(-at.x, 0.0, -at.z).normalized()
	if dir.length() < 0.5:
		dir = Vector3(1, 0, 0)
	var center := at + dir * OFFSET_M
	var before := sim_host.field_works().size()
	var placed := sim_host.debug_strongpoint(center.x, center.z)
	test._check(placed > 20, "debug strongpoint placed %d works" % placed)
	await test._ticks(2)
	main.works_view.refresh()
	var view: Node = main.works_view
	test._check(sim_host.field_works().size() >= before + placed, "the strongpoint's works are not visible")
	test._check(view.trench_meshes >= 12, "only %d trench meshes drawn" % view.trench_meshes)
	var bar: Control = main.hud.command_bar if main.get("hud") != null else null
	if bar != null:
		test._check(bar.buttons.has("work_mg_nest") and bar.buttons.has("work_camo_net"),
			"positions buttons missing in the command bar")
	# The squad walks into the ring and takes firing positions.
	sim_host.move_units(PackedInt64Array([squad]), center.x, center.z)
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + 1200
	while sim_host.current_tick() < deadline and _distance(sim_host, squad, center) > 6.0:
		await test._frames(1)
	await test._ticks(200)
	Engine.time_scale = 1.0
	test._check(_distance(sim_host, squad, center) <= 8.0, "squad %d did not reach the strongpoint" % squad)
	# A line being drawn shows its price.
	var selection: Control = main.selection
	selection.set_command_mode("work_teeth")
	selection.set_meta(EngineerOrders.START_META, ["teeth", Vector2(center.x, center.z)])
	var line := EngineerOrders.line_preview(selection, center + Vector3(12, 0, 0))
	test._check(int(line.get("pieces", 0)) == 9, "teeth over 12 m: %s pieces" % line.get("pieces"))
	test._check(str(line.get("text", "")).contains("manpower"), "no price in the line preview")
	EngineerOrders.reset(selection)
	selection.set_command_mode("")
	print("self-test: positions ok (%d works, %d trench meshes, squad %d at the strongpoint)" % [
		placed, view.trench_meshes, squad])


func _own_squads(sim_host: SimHost) -> Array[int]:
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var stances := sim_host.unit_stances()
	var result: Array[int] = []
	for i in ids.size():
		if owners[i] == sim_host.local_player() and alive[i] != 0 and stances[i] >= 0:
			result.append(ids[i])
	return result


func _distance(sim_host: SimHost, id: int, to: Vector3) -> float:
	var i := Array(sim_host.unit_ids()).find(id)
	if i < 0:
		return INF
	var p: Vector3 = sim_host.current_positions()[i]
	return Vector2(p.x - to.x, p.z - to.z).length()
