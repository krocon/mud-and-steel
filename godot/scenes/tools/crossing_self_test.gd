extends RefCounted
## Self-test part for prompt 055 (bridges, bridge-laying tanks, camouflage,
## paratroopers), run by `self_test.gd` before the replay test with its
## helpers (`_check`, `_frames`, `_ticks`, `_ids_of`, `_positions_of`).
##
## The command bar's camouflage button puts branches on the own vehicles
## (real commands: the replay test re-simulates them); the air bar calls
## paratroopers by a drag over the own half of the map, the view draws their
## canopies (landing is left to the simulation tests). A collapsing deck and a
## laid bridge are drawn from made-up bridge data (the demo has no
## engineers or bridge-laying tanks; the simulation tests cover the rules).

const TIME_SCALE := 4.0
const SECONDS := 120


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node3D = main.get("crossing_view")
	test._check(view != null, "no crossing view")
	if view == null:
		return
	# Camouflage through the command bar.
	var alive := sim_host.unit_alive()
	var ids := sim_host.unit_ids()
	var vehicles: Array[int] = []
	for id in test._ids_of(true):
		var i := ids.find(id)
		if i >= 0 and alive[i] != 0 and sim_host.can_camouflage(id):
			vehicles.append(id)
	test._check(not vehicles.is_empty(), "no own vehicle can put up camouflage")
	var bar: Control = main.hud.command_bar
	main.selection.set_selection(vehicles)
	await test._frames(12)
	# The camouflage tile shows in the context column of the order grid.
	var button: Button = bar.buttons.get("camouflage")
	test._check(button != null and button.is_visible_in_tree() and not button.disabled, "no camouflage button for the selected vehicles")
	if button != null:
		bar.press("camouflage")
	await test._ticks(3)
	view.refresh()
	var states := 0
	for id in vehicles:
		if sim_host.camouflage_state(id) > 0:
			states += 1
	test._check(states == vehicles.size(), "%d of %d vehicles put up camouflage" % [states, vehicles.size()])
	test._check(view.camo_drawn > 0, "no branches or nets drawn")
	bar._update()
	test._check(button == null or str(button.get_meta(&"label")) == "Uncover", "the button does not offer to uncover (%s)" % (str(button.get_meta(&"label")) if button != null else ""))
	main.selection.set_selection([])

	# Paratroopers by a drag in the air bar.
	var air: Control = main.air_view.air_bar
	var options: Dictionary = {}
	for o: Dictionary in sim_host.air_options():
		options[o["mission"]] = o
	var airborne: Dictionary = options.get("airborne", {})
	test._check(not airborne.is_empty() and str(airborne["aircraft"]) != "", "no transport aircraft for the paratroopers")
	test._check(air.buttons.has("airborne"), "no paratrooper button in the air bar")
	var before := sim_host.unit_ids().size()
	var landed := 0
	var most_chutes := 0
	if air.buttons.has("airborne") and not air.buttons["airborne"].disabled:
		# Into the map corner farthest from the enemy: the sticks must not
		# decide the battle the menu flow test continues with.
		var enemy_ids: Array[int] = test._ids_of(false)
		enemy_ids.append_array(test._ids_of(false, true))
		var enemies: Array[Vector3] = test._positions_of(enemy_ids)
		var drop := Vector3(-200.0, 0.0, -200.0)
		var best := -1.0
		for corner: Vector3 in [Vector3(-200, 0, -200), Vector3(200, 0, -200), Vector3(-200, 0, 200), Vector3(200, 0, 200)]:
			var nearest := INF
			for e: Vector3 in enemies:
				nearest = minf(nearest, Vector2(e.x - corner.x, e.z - corner.z).length())
			if nearest > best:
				best = nearest
				drop = corner
		air.set_pending("airborne")
		var from := drop + Vector3(-40.0, 0.0, 0.0)
		var to := drop + Vector3(40.0, 0.0, 0.0)
		test._check(air.call_along(from, to), "the drag did not call the paratroopers")
		# Only until the first canopies are drawn: the battle goes on
		# meanwhile, and the menu flow test after this one needs the enemy
		# alive. Landing is covered by the simulation tests.
		Engine.time_scale = TIME_SCALE
		var deadline: int = sim_host.current_tick() + SECONDS * sim_host.tick_rate()
		while sim_host.current_tick() < deadline and most_chutes == 0:
			await test.get_tree().process_frame
			most_chutes = maxi(most_chutes, view.chutes_drawn)
		Engine.time_scale = 1.0
		await test._ticks(2)
		landed = sim_host.unit_ids().size() - before
		test._check(most_chutes > 0, "no canopies drawn")
	else:
		var why := "no button"
		if air.buttons.has("airborne"):
			why = (air.buttons["airborne"] as Button).tooltip_text
		test._check(false, "the paratroopers cannot be called: %s" % why)

	# A collapsing deck and a laid bridge, drawn from made-up data.
	var center: Vector3 = test._positions_of(vehicles)[0] if not vehicles.is_empty() else Vector3.ZERO
	view._decks = [{"bridge": 0, "unit": -1, "intact": false,
		"start": center + Vector3(-6.0, 3.0, 30.0), "end": center + Vector3(6.0, 3.0, 30.0), "width_m": 4.0}]
	view._deck_count = sim_host.bridges_intact().size()
	view._on_event({"type": "collapsed", "bridge": 0, "at": center + Vector3(0.0, 3.0, 30.0)})
	test._check(view.pieces_spawned >= 4, "the deck did not break into pieces")
	await test._frames(30)
	var laid: Node3D = view._make_laid({"bridge": 99, "unit": 0, "intact": true,
		"start": center + Vector3(0.0, 0.5, 40.0), "end": center + Vector3(0.0, 0.5, 52.0), "width_m": 3.0})
	test._check(laid != null and laid.get_child_count() >= 4, "no laid bridge deck")
	if laid != null:
		laid.free()
	view._deck_count = -1
	print("self-test: crossings ok at tick %d: %d camouflaged, %d sticks drawn, %d squads landed, %d deck pieces" % [
		sim_host.current_tick(), states, most_chutes, landed, view.pieces_spawned])
