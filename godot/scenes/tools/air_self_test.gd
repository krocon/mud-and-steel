extends RefCounted
## Self-test part for air support (prompt 033), run by `self_test.gd` after
## the HUD test with its helpers (`_check`, `_frames`, `_ticks`, `_ids_of`,
## `_positions_of`, `_screen_of_point`, `_left_click`).
##
## The call bar is shown with the faction's aircraft; Shift+4 and a left
## click on the enemy call bombers there (a real command, so the replay test
## that follows re-simulates it). The view draws the sortie's route, the
## aircraft with their models and shadows, and the falling bombs; the
## bombers leave again and the mission cools down.

const TIME_SCALE := 4.0
const SECONDS := 120


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node3D = main.air_view
	var bar: Control = view.air_bar
	test._check(bar != null and bar.visible, "no air support bar")
	if bar == null:
		return
	await test._frames(15)
	var bomb: Button = bar.buttons["bomb"]
	test._check(not bomb.disabled, "bombers cannot be called: %s" % bomb.tooltip_text)
	test._check(bomb.tooltip_text.contains("Heinkel"), "the German bomber is not the He 111: %s" % bomb.tooltip_text)
	var enemies: Array[int] = test._ids_of(false, true)
	enemies.append_array(test._ids_of(false))
	var target: Vector3 = test._positions_of(enemies)[0] if not enemies.is_empty() else Vector3.ZERO
	main.camera_rig.snap_to(target, 300.0, 0.0)
	await test._frames(5)
	for pressed in [true, false]:
		var key := InputEventKey.new()
		key.keycode = KEY_4
		key.physical_keycode = KEY_4
		key.shift_pressed = true
		key.pressed = pressed
		test.get_viewport().push_input(key)
	test._check(bar.pending == "bomb", "Shift+4 did not arm the bombers (%s)" % bar.pending)
	test._left_click(test._screen_of_point(target))
	await test._ticks(2)
	test._check(bar.calls == 1 and bar.pending.is_empty(), "the click did not call the bombers")
	var options: Array = sim_host.air_options()
	var cooling := false
	for o: Dictionary in options:
		if o["mission"] == "bomb":
			cooling = float(o["cooldown"]) > 0.0
	test._check(cooling, "the bomber call is not cooling down")
	test._check(sim_host.air_sorties().size() == 1, "no own sortie shown")

	Engine.time_scale = TIME_SCALE
	var most_aircraft := 0
	var most_bombs := 0
	var deadline: int = sim_host.current_tick() + SECONDS * sim_host.tick_rate()
	var seen := false
	while sim_host.current_tick() < deadline:
		await test.get_tree().process_frame
		most_aircraft = maxi(most_aircraft, view.aircraft_drawn)
		most_bombs = maxi(most_bombs, view.bombs_drawn)
		seen = seen or view.aircraft_drawn > 0
		if seen and view.aircraft_drawn == 0 and sim_host.bomb_positions().is_empty():
			break
	Engine.time_scale = 1.0
	await test._ticks(2)
	test._check(most_aircraft == 3, "drew at most %d of 3 bombers" % most_aircraft)
	test._check(most_bombs > 0, "no falling bombs drawn")
	test._check(sim_host.aircraft_ids().is_empty(), "the bombers did not leave")
	test._check(sim_host.air_sorties().is_empty(), "the sortie is still shown")
	print("self-test: air ok at tick %d: %d bombers, up to %d bombs falling" % [
		sim_host.current_tick(), most_aircraft, most_bombs])
