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
## - Unit values (prompt 066): for every buildable id the tile's tooltip and
##   the unit card open and show values without a missing translation, in
##   English and German; the numbers shown are those of `SimHost.unit_stats`
##   (what `cli unit-stats` prints), fetched once per type; a picker with
##   every buildable id shows the ratings and a right click on a tile opens the card; the
##   unit panel shows the ratings, its button and the key (Shift + I) open
##   the card, Escape closes it; while it is open, orders still reach the
##   units and a click on the card is no order; it stays inside the window
##   and off the minimap and the command bar.

const Settings := preload("res://scenes/menu/settings.gd")
const Language := preload("res://scenes/menu/language.gd")
const L10nSelfTest := preload("res://scenes/menu/l10n_self_test.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
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
	# Quiet again, as the parts below expect: the combat part leaves the
	# selected tanks firing at will at enemies that hold fire, and whether
	# they finish them off (deciding the match, the result screen then
	# covers the HUD) before this part ends depends on where the formation
	# of the order part put them and on how long the parts in between run.
	print("self-test: hud starts at tick %d, %d enemy units alive" % [
		main.sim_host.current_tick(), main.sim_host.alive_unit_count(1)])
	main.sim_host.set_hold_fire(PackedInt64Array(test._ids_of(true)), true)
	await _minimap_test(test, main, hud.minimap, tanks)
	await _panel_test(test, main, hud.unit_panel, tanks)
	Engine.time_scale = TIME_SCALE
	await _chain_test(test, main, tanks[0])
	await _command_bar_test(test, main, hud.command_bar, tanks)
	Engine.time_scale = 1.0
	await _events_test(test, main, hud.event_feed)
	await _health_bar_test(test, main)
	await _unit_values_test(test, main, hud, tanks)
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


# --- Unit values (prompt 066) ---------------------------------------------

## A buy option as `SimHost.build_options` returns it, for `id`.
func _option(id: String, type: int) -> Dictionary:
	return {"type": type, "id": id, "name": UnitStats.name_of(id), "fuel": 11, "ammo": 22, "manpower": 33,
		"time": 44.0, "blocked": "", "structure": false}


func _unit_values_test(test: Node, main: Node3D, hud: Control, tanks: Array[int]) -> void:
	var card: Control = hud.unit_card
	test._check(card != null and not card.visible, "unit card missing or open at the start")
	if card == null:
		return
	var ids := SimHost.buildable_ids()
	test._check(ids.size() >= 30, "only %d buildable ids" % ids.size())
	var with_ratings := 0
	var with_record := 0
	for language: String in ["en", "de"]:
		Language.apply(language)
		for i in ids.size():
			var id := ids[i]
			# The card straight from the bridge: the function `cli unit-stats` prints.
			var stats := SimHost.unit_stats(id)
			var lines := Picker.tooltip_lines(_option(id, i), i)
			test._check(lines.size() >= 3 and UnitStats.summary_lines(id).size() <= UnitStats.MAX_TIP_LINES,
				"tooltip of %s has %d lines" % [id, lines.size()])
			_no_keys(test, lines, "tooltip of %s (%s)" % [id, language])
			var ratings: Dictionary = stats.get("ratings", {})
			for rating: String in ratings:
				var shown := "%s %d" % [Language.t(UnitStats.RATING_NAMES[rating]), int(ratings[rating])]
				test._check("\n".join(lines).contains(shown), "tooltip of %s lacks '%s'" % [id, shown])
			hud.show_unit_card(id)
			test._check(card.visible and card.type_id == id, "unit card did not open for %s" % id)
			var texts: PackedStringArray = card.texts()
			_no_keys(test, texts, "unit card of %s (%s)" % [id, language])
			test._check(card.tab_texts("game").size() >= 3, "unit card of %s shows no values: %s" % [id, card.tab_texts("game")])
			test._check(card.rating_bars.ratings == ratings, "unit card of %s shows ratings %s, the bridge has %s" % [
				id, card.rating_bars.ratings, ratings])
			_card_numbers(test, id, stats, card.tab_texts("game"), texts)
			if language == "en":
				with_ratings += 1 if not ratings.is_empty() else 0
				if not SimHost.vehicle_record(id).is_empty():
					with_record += 1
					var record := SimHost.vehicle_record(id)
					var tech := "\n".join(card.tab_texts("tech"))
					test._check(tech.contains(str(record.source.name)) and card.tab_texts("tech").size() > 20,
						"technical data of %s: %d texts" % [id, card.tab_texts("tech").size()])
	# German numbers: a decimal comma and a point for thousands.
	test._check(UnitStats.num(1234.5, 1) == "1.234,5", "German number %s" % UnitStats.num(1234.5, 1))
	Language.apply("en")
	test._check(UnitStats.num(1234.5, 1) == "1,234.5", "English number %s" % UnitStats.num(1234.5, 1))
	test._check(with_ratings >= 30 and with_record >= 5, "%d buildable ids with ratings, %d with technical data" % [with_ratings, with_record])
	# Once per type: a second pass over everything asks the bridge nothing.
	var fetches: int = UnitStats.fetches
	for i in ids.size():
		Picker.tooltip_lines(_option(ids[i], i), i)
	test._check(UnitStats.fetches == fetches, "cards fetched again: %d -> %d" % [fetches, UnitStats.fetches])
	card.close()
	await _picker_tiles_test(test, hud, ids)
	await _card_controls_test(test, main, hud, tanks)
	print("self-test: unit values ok (%d buildable ids, %d with ratings, %d with technical data, %d cards fetched)" % [
		ids.size(), with_ratings, with_record, UnitStats.fetches])


func _no_keys(test: Node, texts: PackedStringArray, where: String) -> void:
	var key_word := RegEx.create_from_string("[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+")
	for text in texts:
		for m in key_word.search_all(text):
			if not Language.has(m.get_string()):
				test._check(false, "%s: text without translation '%s'" % [where, text])
				return


## Sample numbers of the card against the bridge's values.
func _card_numbers(test: Node, id: String, stats: Dictionary, game: PackedStringArray, all: PackedStringArray) -> void:
	var shown := "\n".join(game)
	var expected: Array[String] = []
	if stats.has("protection"):
		expected.append(UnitStats.num(int(stats.protection.hp)))
	if stats.has("against_vehicles"):
		expected.append("%d mm" % int(stats.against_vehicles.penetration_mm))
		expected.append(str(stats.against_vehicles.weapon))
	for weapon: Dictionary in stats.get("weapons", []):
		expected.append(UnitStats.num(int(weapon.range_game_m)) + " m")
	if stats.has("movement") and stats.movement.has("road_game_mps_x10"):
		expected.append(UnitStats.tenths(int(stats.movement.road_game_mps_x10), 1) + " m/s")
	if stats.has("other") and stats.other.has("sight_game_m"):
		expected.append(UnitStats.num(int(stats.other.sight_game_m)) + " m")
	for text in expected:
		test._check(shown.contains(text), "unit card of %s lacks '%s'" % [id, text])
	if stats.has("other") and stats.other.has("cost"):
		var cost: Array = stats.other.cost
		var line := Language.t("HUD_COST_LINE") % [cost[0], cost[1], cost[2], roundi(int(stats.other.build_game_s_x10) / 10.0)]
		test._check(all.has(line), "unit card of %s lacks the price '%s'" % [id, line])


## A picker with every buildable id (more than 30 tiles over its tabs):
## ratings on the tiles as the bridge has them, a right click opens the
## card, refreshing it costs little and no bridge call.
func _picker_tiles_test(test: Node, hud: Control, ids: PackedStringArray) -> void:
	var card: Control = hud.unit_card
	var picker := Picker.new()
	picker.name = "TestPicker"
	picker.position = Vector2(8, 70)
	hud.add_child(picker)
	picker.info_requested.connect(hud.show_unit_card)
	var options: Array = []
	for i in ids.size():
		options.append(_option(ids[i], i))
	picker.set_options("Test", options, {}, {})
	var checked := 0
	var largest := 0
	for tab: String in picker.tab_ids():
		picker.open_tab(tab)
		await test._frames(2)
		largest = maxi(largest, picker.tiles.size())
		for type: int in picker.tiles:
			var tile: Button = picker.tiles[type]
			var id := str(options[type].id)
			var strip: Control = tile.get_node_or_null("Ratings")
			var ratings: Dictionary = SimHost.unit_stats(id).get("ratings", {})
			test._check(strip != null and strip.ratings == ratings and strip.shown().size() == ratings.size(),
				"tile of %s shows ratings %s, the bridge has %s" % [id, strip.ratings if strip != null else {}, ratings])
			test._check(strip != null and Rect2(Vector2.ZERO, tile.size).encloses(strip.get_rect()),
				"ratings of %s stick out of the tile" % id)
			test._check(tile.tooltip_text == "\n".join(Picker.tooltip_lines(options[type], int(tile.get_meta(&"index")))),
				"tile of %s has another tooltip than its lines" % id)
			checked += 1
	test._check(checked == ids.size() and checked >= 30, "%d of %d tiles checked" % [checked, ids.size()])
	picker.open_tab("vehicles")
	await test._frames(2)
	var fetches: int = UnitStats.fetches
	var began := Time.get_ticks_usec()
	for i in 100:
		picker.set_options("Test", options, {}, {})
	var usec := (Time.get_ticks_usec() - began) / 100.0
	test._check(UnitStats.fetches == fetches, "refreshing the picker fetched %d cards" % (UnitStats.fetches - fetches))
	test._check(usec < 2000.0, "refreshing the picker takes %.0f us" % usec)
	# Right click on the first tile: its card; the tile is not picked.
	var picked: Array = []
	picker.picked.connect(func(o: Dictionary) -> void: picked.append(o))
	var first_type: int = picker.tiles.keys()[0]
	var first: Button = picker.tiles[first_type]
	test._right_click(first.get_global_rect().get_center())
	await test._frames(2)
	test._check(card.visible and card.type_id == str(options[first_type].id), "right click on a tile: card of '%s'" % card.type_id)
	test._check(picked.is_empty(), "right click picked %s" % [picked])
	var panel: Control = first.call("_make_custom_tooltip", "")
	test._check(panel != null and panel.find_children("", "Label", true, false).size() >= 3, "tooltip panel of a tile is empty")
	if panel != null:
		panel.free()
	print("self-test: picker with %d tiles (%d in the largest tab), %.0f us per refresh" % [checked, largest, usec])
	card.close()
	hud.remove_child(picker)
	picker.free()
	await test._frames(1)


func _press(test: Node, keycode: Key, shift := false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		event.physical_keycode = keycode
		event.shift_pressed = shift
		event.pressed = pressed
		test.get_viewport().push_input(event)


func _card_controls_test(test: Node, main: Node3D, hud: Control, tanks: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var card: Control = hud.unit_card
	var panel: Control = hud.unit_panel
	var tank := tanks[0]
	var one: Array[int] = [tank]
	var type_id := str(sim_host.unit_info(tank).type_id)
	selection.set_selection(one)
	await test._frames(3)
	# Unit panel: the ratings of the type and the button to the card.
	var ratings: Dictionary = SimHost.unit_stats(type_id).get("ratings", {})
	test._check(panel.ratings.is_visible_in_tree() and panel.ratings.ratings == ratings and not ratings.is_empty(),
		"unit panel shows ratings %s, the bridge has %s" % [panel.ratings.ratings, ratings])
	# The bonuses of a rank are their own line (a recruit has none); the
	# full check is in `experience_self_test.gd`.
	var rank := int(sim_host.unit_experience(tank).get("rank", 0))
	test._check(panel.bonus_line.is_empty() == (rank == 0), "rank %d shows the bonus line '%s'" % [rank, panel.bonus_line])
	test._left_click(panel.card_button.get_global_rect().get_center())
	await test._frames(2)
	test._check(card.visible and card.type_id == type_id, "unit panel button: card of '%s'" % card.type_id)
	test._check(selection.selected == one, "the button changed the selection to %s" % [selection.selected])
	# Escape closes the card and does not open the pause menu.
	_press(test, KEY_ESCAPE)
	await test._frames(2)
	test._check(not card.visible, "Escape left the unit card open")
	test._check(not main.game_menu.pause_panel.visible and not test.get_tree().paused, "Escape with the card open paused the game")
	# The key: Shift + I opens, again closes; plain I is still "Secure".
	test._check(Settings.action_key_name("unit_card") != "", "no key for the unit card")
	_press(test, KEY_I, true)
	await test._frames(2)
	test._check(card.visible and card.type_id == type_id, "Shift + I: card of '%s' (visible %s)" % [card.type_id, card.visible])
	test._check(selection.command_mode.is_empty(), "Shift + I started the order '%s'" % selection.command_mode)
	_press(test, KEY_I, true)
	await test._frames(2)
	test._check(not card.visible, "Shift + I again left the card open")
	_press(test, KEY_I)
	await test._frames(1)
	test._check(selection.command_mode == "secure" and not card.visible, "I: mode '%s', card %s" % [selection.command_mode, card.visible])
	selection.set_command_mode("")
	# (The rank's bonus line is checked by `experience_self_test.gd`, which
	# runs after the replay test: giving experience is no command.)
	hud.show_unit_card(type_id, tank)
	await test._frames(2)
	# Layout: inside the window, off the minimap and the command bar.
	var screen := Rect2(Vector2.ZERO, hud.get_viewport_rect().size)
	var rect := card.get_global_rect()
	test._check(screen.encloses(rect), "unit card %s sticks out of the window %s" % [rect, screen.size])
	test._check(not rect.intersects(hud.minimap.get_global_rect()), "unit card covers the minimap")
	test._check(not rect.intersects(hud.command_bar.get_global_rect()), "unit card covers the command bar")
	test._check(hud.covers(rect.get_center()), "the HUD does not count the card as a panel")
	# The match runs on and orders still work while the card is open.
	var tick := sim_host.current_tick()
	await test._frames(30)
	test._check(sim_host.current_tick() > tick, "the match stands still while the card is open")
	main.camera_rig.snap_to(_starts[0], 160.0, 0.0)
	await test._frames(2)
	# A click on the card is no order and keeps the selection.
	test._left_click(rect.get_center())
	test._right_click(rect.get_center())
	await test._ticks(3)
	test._check(selection.selected == one and not test._moving(one) and sim_host.unit_queue_length(tank) == 0,
		"a click on the unit card gave an order or changed the selection (%s)" % [selection.selected])
	# A right click on the ground next to it moves the tank; H stops it.
	var target: Vector3 = _starts[1] if test._positions_of(one)[0].distance_to(_starts[1]) > 10.0 else _starts[2]
	var at: Vector2 = test._screen_of_point(target)
	test._check(not hud.covers(at), "the move target %s lies under a HUD panel" % at)
	test._right_click(at)
	await test._ticks(8)
	test._check(test._moving(one) and card.visible, "right click with the card open did not move the tank")
	_press(test, KEY_H)
	await test._ticks(5)
	test._check(not test._moving(one) and card.visible, "H with the card open did not stop the tank")
	# The close button.
	var close := card.find_child("Close", true, false) as Button
	test._left_click(close.get_global_rect().get_center())
	await test._frames(2)
	test._check(not card.visible, "the close button left the card open")
