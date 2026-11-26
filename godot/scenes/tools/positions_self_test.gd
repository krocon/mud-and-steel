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
	# Towards the map center, else the first of eight directions whose
	# point the squad can reach (where it stands after the earlier parts
	# depends on them; a point on a steep slope is no strongpoint).
	var center := at + dir * OFFSET_M
	for k in 8:
		var try_dir := dir.rotated(Vector3.UP, k * PI / 4.0)
		var p := at + try_dir * OFFSET_M
		if _open_ground(sim_host, squad, p):
			dir = try_dir
			center = p
			break
	var before := sim_host.field_works().size()
	var placed := sim_host.debug_strongpoint(center.x, center.z)
	test._check(placed > 20, "debug strongpoint placed %d works" % placed)
	# Prompt 230: also exercise a U-shaped vehicle pit mesh (intact and damaged).
	var pit_at := center - dir * 18.0
	sim_host.debug_work("vehicle_pit", Vector2(pit_at.x, pit_at.z), Vector2(pit_at.x, pit_at.z))
	await test._ticks(2)
	main.works_view.refresh()
	var view: Node = main.works_view
	test._check(sim_host.field_works().size() >= before + placed, "the strongpoint's works are not visible")
	test._check(view.trench_meshes >= 13, "only %d trench meshes drawn" % view.trench_meshes)
	var bar: Control = main.hud.command_bar if main.get("hud") != null else null
	if bar != null:
		test._check(bar.buttons.has("work_mg_nest") and bar.buttons.has("work_camo_net"),
			"positions buttons missing in the command bar")
	# The squad walks into the ring and takes firing positions: to the
	# center, or where it can reach next to it (a bunker or nest may stand
	# on the center, prompt 174).
	var pv := sim_host.formation_preview(PackedInt64Array([squad]), center.x, center.z, 1, 0.0, 0.0)
	var goal := center
	if pv.size() >= 6:
		goal = Vector3(pv[4], center.y, pv[5])
	test._check(Vector2(goal.x - center.x, goal.z - center.z).length() <= 20.0,
		"the strongpoint's center is %.1f m from where the squad can get" % Vector2(goal.x - center.x, goal.z - center.z).length())
	sim_host.move_units(PackedInt64Array([squad]), center.x, center.z)
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + 1200
	while sim_host.current_tick() < deadline and _distance(sim_host, squad, goal) > 6.0:
		await test._frames(1)
	await test._ticks(200)
	Engine.time_scale = 1.0
	test._check(_distance(sim_host, squad, goal) <= 8.0, "squad %d did not reach the strongpoint" % squad)
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


## Whether the squad can reach `p` and the ground within 16 m of it, on
## nearly level ground (the strongpoint needs room).
static func _open_ground(sim_host: SimHost, squad: int, p: Vector3) -> bool:
	var h := sim_host.height_at(p.x, p.z)
	for o: Vector2 in [Vector2.ZERO, Vector2(16, 0), Vector2(-16, 0), Vector2(0, 16), Vector2(0, -16)]:
		var q := Vector2(p.x + o.x, p.z + o.y)
		if absf(sim_host.height_at(q.x, q.y) - h) > 0.4:
			return false
		var preview := sim_host.formation_preview(PackedInt64Array([squad]), q.x, q.y, 1, 0.0, 0.0)
		if preview.size() < 6 or Vector2(preview[4] - q.x, preview[5] - q.y).length() > 3.0:
			return false
	return true


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
