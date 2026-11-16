extends RefCounted
## Self-test part for field works (prompt 035), run by `self_test.gd` with
## its helpers (`_check`, `_frames`, `_ticks`, `_ids_of`, `_tanks`,
## `_indices`, `_moving`). It runs after the replay test: the debug
## minefield is no command and would make the replay differ.
##
## The test match has no engineers, so the engineer buttons of the command
## bar stay hidden for a selected tank. A hidden anti-tank section
## (`SimHost.debug_minefield`) in front of an own tank is not drawn; the
## tank drives into it, a mine goes off, the tank throws its track (drawn
## beside it) and the player now sees the field's markers.

const TIME_SCALE := 4.0
## The field lies this far (m) in front of the tank.
const FIELD_M := 40.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var tanks := _alive(sim_host, test._tanks(test._ids_of(true)))
	test._check(not tanks.is_empty(), "works: no own tank left")
	if tanks.is_empty():
		return
	var tank: int = tanks[0]
	var tank_only: Array[int] = [tank]
	main.selection._set_selection([tank])
	var bar: Control = main.hud.command_bar if main.get("hud") != null else null
	if bar != null:
		bar._update()
		test._check(not bar.buttons["work_wire"].visible, "engineer buttons show for a tank")
	var engineers := EngineerOrders.engineers(main.selection)
	test._check(engineers.is_empty(), "a tank counts as an engineer")

	var at: Vector3 = sim_host.current_positions()[test._indices(tank_only)[0]]
	var dir := Vector3(at.x, 0.0, at.z).normalized() * -1.0
	if dir.length() < 0.5:
		dir = Vector3(1, 0, 0)
	# The tank must drive straight through the field: a unit or wreck on its
	# way makes it drive round, past the mines (where the battle leaves its
	# wrecks depends on the weapon ranges, prompt 067).
	dir = _clear_direction(sim_host, at, test._indices(tank_only)[0], dir)
	var field := at + dir * FIELD_M
	var placed := sim_host.debug_minefield(field.x, field.z, true)
	test._check(placed > 10, "debug minefield placed %d mines" % placed)
	main.works_view.refresh()
	test._check(main.works_view.mine_markers == 0, "hidden mines are drawn")
	test._check(sim_host.known_mines().is_empty(), "the player knows hidden mines")

	var beyond := at + dir * (FIELD_M * 2.0)
	sim_host.move_units(PackedInt64Array([tank]), beyond.x, beyond.z)
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + 1600
	while sim_host.current_tick() < deadline and not sim_host.tracks_off().has(tank):
		await test._frames(1)
	Engine.time_scale = 1.0
	test._check(sim_host.tracks_off().has(tank), "tank %d hit no mine by tick %d" % [tank, sim_host.current_tick()])
	await test._ticks(2)
	main.works_view.refresh()
	test._check(main.works_view.tracks_drawn >= 1, "no thrown track drawn")
	test._check(main.works_view.mine_markers > 0, "the exploded field shows no markers")
	test._check(sim_host.known_mines().all(func(m: Dictionary) -> bool: return not m.friendly),
		"neutral mines count as friendly")
	print("self-test: works ok (%d mines placed, tank %d lost its track at tick %d, %d markers)" % [
		placed, tank, sim_host.current_tick(), main.works_view.mine_markers])


const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")


## `preferred` or the nearest direction to it (in steps of 45°) in which no
## other unit (wrecks are units) stands within 9 m of the way through the
## field; `preferred` if every direction is blocked.
func _clear_direction(sim_host: SimHost, at: Vector3, own: int, preferred: Vector3) -> Vector3:
	var positions := sim_host.current_positions()
	var start := Vector3(at.x, 0.0, at.z)
	for degrees: float in [0.0, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0, 180.0]:
		var dir := preferred.rotated(Vector3.UP, deg_to_rad(degrees))
		var end := start + dir * (FIELD_M * 2.0)
		var free := true
		for i in positions.size():
			if i == own:
				continue
			var p := Vector3(positions[i].x, 0.0, positions[i].z)
			if Geometry3D.get_closest_point_to_segment(p, start, end).distance_to(p) < 9.0:
				free = false
				break
		if free:
			return dir
	return preferred


func _alive(sim_host: SimHost, ids: Array[int]) -> Array[int]:
	var all := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var result: Array[int] = []
	for id in ids:
		var i := all.find(id)
		if i >= 0 and alive[i] != 0:
			result.append(id)
	return result
