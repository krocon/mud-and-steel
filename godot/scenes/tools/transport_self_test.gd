extends RefCounted
## Self-test part for transport (prompt 034), run by `self_test.gd` with its
## helpers (`_check`, `_frames`, `_ticks`, `_key`, `_right_click`,
## `_screen_of_unit`, `_ids_of`, `_tanks`, `_indices`).
##
## The test match has no trucks, so an own squad rides on an own tank
## (tank desant): a right click on the tank with the squad selected mounts
## it, the squad's node hides and its soldiers sit on the tank's deck, the
## passenger cannot be selected; J with the tank selected lets it off again.

const TIME_SCALE := 4.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var squads := _alive(sim_host, test._ids_of(true, true))
	var tanks := _alive(sim_host, test._tanks(test._ids_of(true)))
	test._check(not squads.is_empty() and not tanks.is_empty(), "transport: no own squad or tank left")
	if squads.is_empty() or tanks.is_empty():
		return
	var squad: int = squads[0]
	var tank: int = tanks[0]
	var squad_only: Array[int] = [squad]
	var tank_only: Array[int] = [tank]
	test._check(sim_host.unit_seats(tank) == 10, "a medium tank seats %d riders, expected 10" % sim_host.unit_seats(tank))
	# Bring the tank next to the squad.
	var at: Vector3 = sim_host.current_positions()[test._indices(squad_only)[0]]
	sim_host.move_units(PackedInt64Array([tank]), at.x + 15.0, at.z)
	Engine.time_scale = TIME_SCALE
	await test._ticks(5)
	var deadline := sim_host.current_tick() + 1200
	while sim_host.current_tick() < deadline and test._moving(tank_only):
		await test._frames(1)
	main.camera_rig.snap_to(sim_host.current_positions()[test._indices(tank_only)[0]], 60.0, 0.0)
	await test._frames(3)

	main.selection._set_selection([squad])
	test._right_click(test._screen_of_unit(tank))
	await test._ticks(2)
	deadline = sim_host.current_tick() + 2400
	while sim_host.current_tick() < deadline and _carrier(test, sim_host, squad) != tank:
		await test._frames(1)
	Engine.time_scale = 1.0
	test._check(_carrier(test, sim_host, squad) == tank, "squad %d did not mount tank %d by tick %d" % [
		squad, tank, sim_host.current_tick()])
	var riders: int = sim_host.unit_passengers()[test._indices(tank_only)[0]]
	test._check(riders > 0, "tank %d reports no passengers" % tank)
	main.transport_view.refresh()
	await test._frames(2)
	var node: Node3D = main._unit_nodes.get(squad)
	test._check(node != null and not node.visible, "the riding squad is still drawn")
	test._check(main.transport_view.seated(tank) == riders, "%d figures sit on the tank, %d riders" % [
		main.transport_view.seated(tank), riders])
	test._check(not main.selection.selected.has(squad), "a passenger stays selected")
	test._check(not main.selection._own_indices().has(test._indices(squad_only)[0]), "a passenger can be selected")

	# Prompt 099: the riders' range circle shows around the tank (pale,
	# dashed: they cannot fire like this); it is the same code path a
	# towed gun takes.
	main.selection._set_selection([tank])
	await test._frames(2)
	var squad_fire := 0.0
	for w: Dictionary in sim_host.unit_info(squad)["weapons"]:
		squad_fire = maxf(squad_fire, float(w["range_m"]))
	var radii: Array[float] = main.hud.range_rings.radii
	test._check(squad_fire > 0.0 and radii.has(squad_fire),
		"riders' range circle %.1f m missing around the tank: %s" % [squad_fire, radii])

	# Off again.
	test._key(KEY_J)
	Engine.time_scale = TIME_SCALE
	await test._ticks(2)
	deadline = sim_host.current_tick() + 600
	while sim_host.current_tick() < deadline and _carrier(test, sim_host, squad) >= 0:
		await test._frames(1)
	Engine.time_scale = 1.0
	test._check(_carrier(test, sim_host, squad) < 0, "squad %d still rides after J" % squad)
	main.transport_view.refresh()
	await test._frames(2)
	test._check(node != null and node.visible, "the dismounted squad is not drawn")
	test._check(main.transport_view.seated(tank) == 0, "figures still sit on the tank")
	print("self-test: transport ok (squad %d rode tank %d with %d riders, off at tick %d)" % [
		squad, tank, riders, sim_host.current_tick()])


func _carrier(test: Node, sim_host: SimHost, id: int) -> int:
	var only: Array[int] = [id]
	var i: int = test._indices(only)[0]
	return sim_host.unit_carriers()[i] if i >= 0 else -1


func _alive(sim_host: SimHost, ids: Array[int]) -> Array[int]:
	var all := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var result: Array[int] = []
	for id in ids:
		var i := all.find(id)
		if i >= 0 and alive[i] != 0:
			result.append(id)
	return result
