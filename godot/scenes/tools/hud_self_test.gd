extends RefCounted
## Self-test part for the HUD (prompt 031), run by `self_test.gd` at its end
## with its helpers (`_check`, `_frames`, `_ticks`, `_mouse`,
## `_left_click`, `_right_click`, `_screen_of_point`, `_indices`,
## `_moving`). `remember` must be called at the start of the self-test.
##
## - Minimap: ground image and unit dots; click and drag move the camera,
##   right click moves the selection there.
## - Unit panel: tiles for a multiple selection, Shift + click removes a
##   unit, click selects one; the single view shows name and weapons.
## - Waypoint chain: right click, then Shift + right click twice: the tank
##   drives through all three points in order (the start positions of
##   three own tanks, known to be reachable); markers are drawn.
## - Command bar: "Move" waits for a left click and sends the order,
##   "Stop" stops, "Hold fire" switches the fire mode.
## - Events: a posted message and Space move the camera there; health bar
##   setting "never" hides the bars.

const Settings := preload("res://scenes/menu/settings.gd")
const TIME_SCALE := 4.0

## Start positions of the own tanks.
var _starts: Array[Vector3] = []


func remember(test: Node) -> void:
	_starts = test._positions_of(test._ids_of(true))


func run(test: Node) -> void:
	var main: Node3D = test.main
	var hud: Control = main.hud
	test._check(hud != null, "no match HUD")
	if hud == null:
		return
	var tanks := _alive(test, test._ids_of(true))
	test._check(tanks.size() >= 3, "need 3 own tanks for the HUD test, %d alive" % tanks.size())
	if tanks.size() < 3:
		return
	await _minimap_test(test, main, hud.minimap, tanks)
	await _panel_test(test, main, hud.unit_panel, tanks)
	Engine.time_scale = TIME_SCALE
	await _chain_test(test, main, tanks[0])
	await _command_bar_test(test, main, hud.command_bar, tanks)
	Engine.time_scale = 1.0
	await _events_test(test, main, hud.event_feed)
	await _health_bar_test(test, main)
	main.selection.set_selection([])
	await test._frames(30)
	print("self-test: hud ok (%d dots, %.2f ms HUD per frame, max %.2f ms)" % [
		hud.minimap.dot_count, hud.usec / 1000.0, hud.max_usec / 1000.0
	])


func _alive(test: Node, ids: Array[int]) -> Array[int]:
	var alive: PackedByteArray = test.main.sim_host.unit_alive()
	var indices: Array[int] = test._indices(ids)
	var out: Array[int] = []
	for k in ids.size():
		if indices[k] >= 0 and alive[indices[k]] != 0:
			out.append(ids[k])
	return out


func _minimap_test(test: Node, main: Node3D, minimap: Control, tanks: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	await test._frames(2)
	test._check(minimap.visible and minimap.dot_count > 0, "minimap shows %d dots" % minimap.dot_count)
	var alive := 0
	for a in sim_host.unit_alive():
		alive += a
	test._check(minimap.dot_count == alive, "minimap has %d dots for %d living units in sight" % [minimap.dot_count, alive])
	var area: Control = minimap._area
	var rig: Node3D = main.camera_rig
	# Click at a quarter of the map, drag to three quarters.
	var from: Vector2 = area.size * 0.25
	var to: Vector2 = area.size * 0.7
	test._mouse(MOUSE_BUTTON_LEFT, area.global_position + from, true)
	await test._frames(1)
	var expected: Vector3 = minimap.to_world(from)
	test._check(Vector2(rig.focus.x, rig.focus.z).distance_to(Vector2(expected.x, expected.z)) < 1.0,
		"minimap click put the camera at %s, expected %s" % [rig.focus, expected])
	for step in range(1, 5):
		var motion := InputEventMouseMotion.new()
		motion.position = area.global_position + from.lerp(to, step / 4.0)
		motion.global_position = motion.position
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		test.get_viewport().push_input(motion)
	test._mouse(MOUSE_BUTTON_LEFT, area.global_position + to, false)
	await test._frames(1)
	expected = minimap.to_world(to)
	test._check(Vector2(rig.focus.x, rig.focus.z).distance_to(Vector2(expected.x, expected.z)) < 1.0,
		"minimap drag put the camera at %s, expected %s" % [rig.focus, expected])
	# Right click: the selected tank drives to the start of tank 0.
	main.selection.set_selection([tanks[1]])
	var target: Vector2 = minimap.to_minimap(_starts[1])
	test._mouse(MOUSE_BUTTON_RIGHT, area.global_position + target, true)
	test._mouse(MOUSE_BUTTON_RIGHT, area.global_position + target, false)
	await test._ticks(3)
	var points := sim_host.unit_waypoints(tanks[1])
	test._check(not points.is_empty() and Vector2(points[-1].x, points[-1].z).distance_to(Vector2(_starts[1].x, _starts[1].z)) < 3.0,
		"minimap right click: tank %d heads to %s, expected %s" % [tanks[1], points, _starts[1]])
	sim_host.stop_units(PackedInt64Array([tanks[1]]))
	await test._ticks(2)


func _panel_test(test: Node, main: Node3D, panel: Control, tanks: Array[int]) -> void:
	var selection: Control = main.selection
	var three: Array[int] = tanks.slice(0, 3)
	selection.set_selection(three)
	await test._frames(2)
	test._check(panel.visible and panel.tile_ids == three, "unit panel tiles %s for %s" % [panel.tile_ids, three])
	var tiles: Array = panel._grid.get_children()
	if tiles.size() == 3:
		var middle := (tiles[1] as Control).get_global_rect().get_center()
		test._left_click(middle, true)
		await test._frames(2)
		test._check(selection.selected == [three[0], three[2]], "shift click on a tile left %s" % [selection.selected])
		var first := (panel._grid.get_child(0) as Control).get_global_rect().get_center()
		test._left_click(first)
		await test._frames(2)
		test._check(selection.selected == [three[0]], "click on a tile selected %s" % [selection.selected])
	await test._frames(1)
	test._check(not panel._title.text.is_empty() and panel._details.text.contains("Weapons"),
		"single unit view: '%s' / '%s'" % [panel._title.text, panel._details.text])
	var info: Dictionary = main.sim_host.unit_info(three[0])
	test._check(info.get("max_hp", 0) > 0 and not (info.get("weapons", []) as Array).is_empty(), "unit info %s" % info)


func _chain_test(test: Node, main: Node3D, tank: int) -> void:
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var points: Array[Vector3] = [_starts[0], _starts[1], _starts[2]]
	var center := (points[0] + points[1] + points[2]) / 3.0
	main.camera_rig.snap_to(center, 160.0, 0.0)
	await test._frames(2)
	selection.set_selection([tank])
	test._right_click(test._screen_of_point(points[0]))
	for k in [1, 2]:
		test._mouse(MOUSE_BUTTON_RIGHT, test._screen_of_point(points[k]), true, true)
		test._mouse(MOUSE_BUTTON_RIGHT, test._screen_of_point(points[k]), false, true)
	await test._ticks(3)
	test._check(sim_host.unit_queue_length(tank) == 2, "Shift + right click queued %d waypoints" % sim_host.unit_queue_length(tank))
	await test._frames(1)
	test._check(main.hud.waypoints.markers >= 2, "%d waypoint markers drawn" % main.hud.waypoints.markers)
	var one: Array[int] = [tank]
	var closest: Array[float] = [INF, INF, INF]
	var reached: Array[int] = []
	var deadline := sim_host.current_tick() + 20 * 120
	while sim_host.current_tick() < deadline:
		await test.get_tree().process_frame
		var p: Vector3 = test._positions_of(one)[0]
		for k in 3:
			var d := Vector2(p.x, p.z).distance_to(Vector2(points[k].x, points[k].z))
			closest[k] = minf(closest[k], d)
			if d < 3.0 and (reached.is_empty() or reached[-1] != k):
				reached.append(k)
		if sim_host.unit_queue_length(tank) == 0 and not test._moving(one) and reached.has(2):
			break
	test._check(closest.all(func(d: float) -> bool: return d < 3.0), "chain passed the waypoints at %s m" % [closest])
	test._check(reached.slice(-3) == [0, 1, 2] or reached == [0, 1, 2], "waypoints reached in order %s" % [reached])
	print("self-test: waypoint chain ok (%s, closest %s)" % [reached, closest])


func _command_bar_test(test: Node, main: Node3D, bar: Control, tanks: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var group: Array[int] = tanks.slice(0, 3)
	selection.set_selection(group)
	await test._frames(1)
	var move: Button = bar.buttons["move"]
	test._check(not move.disabled and move.text.contains("[M]"), "move button '%s' (disabled %s)" % [move.text, move.disabled])
	test._left_click(move.get_global_rect().get_center())
	await test._frames(1)
	test._check(selection.command_mode == "move", "move button left mode '%s'" % selection.command_mode)
	var target: Vector3 = _starts[3] if _starts.size() > 3 else _starts[0]
	test._left_click(test._screen_of_point(target))
	await test._ticks(5)
	test._check(selection.command_mode.is_empty(), "mode '%s' after the click" % selection.command_mode)
	test._check(test._moving(group), "command bar move did not start the group")
	var stop: Button = bar.buttons["stop"]
	test._left_click(stop.get_global_rect().get_center())
	await test._ticks(5)
	test._check(not test._moving(group), "stop button did not stop the group")
	var first: Array[int] = [group[0]]
	var hold_before: int = sim_host.unit_hold_fire()[test._indices(first)[0]]
	test._left_click((bar.buttons["fire"] as Button).get_global_rect().get_center())
	await test._ticks(3)
	var hold_after: int = sim_host.unit_hold_fire()[test._indices(first)[0]]
	test._check(hold_after != hold_before, "fire mode button left hold fire at %d" % hold_after)
	# Back to holding fire if the button released it (later parts expect quiet).
	if hold_after == 0:
		sim_host.set_hold_fire(PackedInt64Array(group), true)
	test._check((bar.buttons["stance_stand"] as Button).disabled, "stance buttons enabled for tanks")


func _events_test(test: Node, main: Node3D, feed: Control) -> void:
	var point := _starts[2]
	main.camera_rig.snap_to(Vector3.ZERO, 200.0, 0.0)
	feed.post("sighted", "Self-test event", point)
	await test._frames(1)
	var event := InputEventKey.new()
	event.keycode = KEY_SPACE
	event.physical_keycode = KEY_SPACE
	event.pressed = true
	test.get_viewport().push_input(event)
	await test._frames(90)
	var focus: Vector3 = main.camera_rig.focus
	test._check(Vector2(focus.x, focus.z).distance_to(Vector2(point.x, point.z)) < 2.0,
		"Space moved the camera to %s, expected %s" % [focus, point])
	test._check(feed.get_child_count() >= 1, "no event message shown")


func _health_bar_test(test: Node, main: Node3D) -> void:
	var bars: Control = main.combat_view.health_bars
	main.selection.set_selection(_alive(test, test._ids_of(true)))
	await test._frames(2)
	var before: int = bars.drawn
	Settings.set_value("health_bars", "never")
	await test._frames(2)
	test._check(before > 0 and bars.drawn == 0, "health bars with 'never': %d (before %d)" % [bars.drawn, before])
	Settings.set_value("health_bars", "damaged")
