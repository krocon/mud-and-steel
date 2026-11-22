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
## - Order grid: "Move" waits for a left click and sends the order,
##   "Stop" stops, "Hold fire" switches the fire mode.
## - Layout (prompt 095): only minimap and top bar (and support) show
##   without a selection; an own unit brings the info field and the order
##   grid with every tile on its fixed place, a tile gives the same order as
##   its key, the pages open and lead back; a unit of the enemy brings the
##   info field only; clicks on HUD parts are no orders; no two parts
##   overlap and all stay inside the window at 1280 x 720 and 3840 x 2160,
##   the smallest and largest interface scale and the largest font.
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
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
const L10nSelfTest := preload("res://scenes/menu/l10n_self_test.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const TIME_SCALE := 4.0
## Fixed places of the first page of the order grid (row by row).
const MAIN_PAGE: Array[String] = [
	"move", "attack_move", "patrol", "secure", "stop",
	"attack", "attack_ground", "attack_object", "barrage", "smoke",
	"fire", "stance", "formation", "garrison",
]
## Window size, interface scale, font size of the overlap checks.
const LAYOUT_CASES: Array[Array] = [
	[Vector2i(1280, 720), 1.0, 1.0],
	[Vector2i(1280, 720), 1.0, 1.3],
	[Vector2i(1280, 720), 0.75, 1.3],
	[Vector2i(3840, 2160), 0.75, 1.0],
	[Vector2i(3840, 2160), 1.5, 1.3],
	[Vector2i(3840, 2160), 3.0, 1.3],
]

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
	await _layout_test(test, main, hud, tanks)
	await _events_test(test, main, hud.event_feed)
	await _alerts_test(test, main, hud.alerts, tanks)
	await _order_feedback_test(test, main, hud, tanks)
	await _selection_keys_test(test, main, tanks)
	await _ground_tooltip_test(test, main, hud.ground_tooltip)
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
	await test._frames(10)
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
	# Prompt 095: hit points as a symbol with numbers, the kind of unit in
	# the header, a picture.
	var shown_info: Dictionary = main.sim_host.unit_info(three[0])
	test._check(panel.health_chip.text == "%d / %d" % [shown_info.hp, shown_info.max_hp] and panel.health_chip.tooltip_text.contains("Hit points"),
		"hit points chip '%s' / '%s'" % [panel.health_chip.text, panel.health_chip.tooltip_text])
	test._check(panel.kind == "vehicles" and panel._kind_icon.text == "Vehicles" and panel._portrait.texture != null,
		"info field: kind '%s' ('%s'), picture %s" % [panel.kind, panel._kind_icon.text, panel._portrait.texture])
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
	await test._frames(12)
	var move: Button = bar.buttons["move"]
	test._check(bar.visible and move.is_visible_in_tree(), "the order grid does not show for own tanks")
	test._check(not move.disabled and move.key_text == "M" and move.tooltip_text.contains("[M]"),
		"move tile: key '%s', tooltip '%s' (disabled %s)" % [move.key_text, move.tooltip_text, move.disabled])
	test._left_click(move.get_global_rect().get_center())
	await test._frames(1)
	test._check(selection.command_mode == "move", "move button left mode '%s'" % selection.command_mode)
	var target: Vector3 = _starts[3] if _starts.size() > 3 else _starts[0]
	test._check(not main.hud.covers(test._screen_of_point(target)), "the move target lies under a HUD panel")
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
	# Prompt 176: the fire tile opens the page of the fire discipline.
	test._left_click((bar.buttons["fire"] as Button).get_global_rect().get_center())
	await test._frames(2)
	test._check(bar.page == "fire", "the fire tile opened page '%s'" % bar.page)
	var other := "fire_at_will" if hold_before != 0 else "hold_fire"
	test._left_click((bar.buttons[other] as Button).get_global_rect().get_center())
	await test._ticks(3)
	var hold_after: int = sim_host.unit_hold_fire()[test._indices(first)[0]]
	test._check(hold_after != hold_before, "fire mode button left hold fire at %d" % hold_after)
	test._left_click((bar.buttons["vehicles_only"] as Button).get_global_rect().get_center())
	await test._ticks(3)
	var limits: int = sim_host.unit_fire_limits()[test._indices(first)[0]]
	test._check(limits & 2 != 0, "the vehicles-only switch left the limits at %d" % limits)
	test._left_click((bar.buttons["vehicles_only"] as Button).get_global_rect().get_center())
	await test._ticks(3)
	bar.open_page("main")
	await test._frames(2)
	# Back to holding fire if the button released it (later parts expect quiet).
	if hold_after == 0:
		sim_host.set_hold_fire(PackedInt64Array(group), true)
	test._check((bar.buttons["stance_stand"] as Button).disabled, "stance buttons enabled for tanks")
	test._check((bar.buttons["stance"] as Button).disabled and bar.buttons["stance"].is_visible_in_tree(),
		"the stance tile is not greyed in its place for tanks")
	await _tile_size_test(test, main, bar)


## Prompts 092 and 095: tiles of 46 px or more at factor 1 that grow with
## the font size setting, the key readable in the corner; every tile lies
## inside the grid; the HUD parts do not overlap at either size.
func _tile_size_test(test: Node, main: Node3D, bar: Control) -> void:
	var hud: Control = main.hud
	var move: Button = bar.buttons["move"]
	var factor: float = HudTheme.scale_for(hud.get_viewport_rect().size)
	test._check(is_equal_approx(HudTheme.scale, factor), "HUD factor %.2f, expected %.2f" % [HudTheme.scale, factor])
	var wanted: float = roundf(HudTheme.TILE_PX * factor)
	test._check(is_equal_approx(move.size.x, wanted) and is_equal_approx(move.size.y, wanted) and wanted >= 27.0,
		"order tile is %s, wanted %.0f px" % [move.size, wanted])
	test._check(HudTheme.font(9) >= 8, "the key in a tile's corner is %d px" % HudTheme.font(9))
	var inside := Rect2(Vector2.ZERO, bar.size)
	for id: String in bar.buttons:
		var b: Button = bar.buttons[id]
		if b.is_visible_in_tree():
			test._check(inside.encloses(b.get_rect()), "order tile '%s' sticks out of the grid" % id)
	# Larger font setting, larger tiles (as far as the window has room).
	var before := move.size
	MenuTheme.set_font_scale(1.3)
	await test._frames(20)
	var larger: float = HudTheme.scale_for(hud.get_viewport_rect().size)
	test._check(larger > factor and move.size.x > before.x and move.size.y > before.y,
		"order tiles do not grow with the font size (%s -> %s)" % [before, move.size])
	_no_overlap(test, hud, "font size 1.3")
	MenuTheme.set_font_scale(Settings.get_value("font_size"))
	await test._frames(20)
	test._check(move.size == before, "order tiles did not return to %s (%s)" % [before, move.size])
	_no_overlap(test, hud, "default size")


## No two HUD parts that take clicks overlap, and all lie inside the window.
func _no_overlap(test: Node, hud: Control, what: String) -> int:
	var parts: Dictionary = hud.panels()
	var screen := Rect2(Vector2.ZERO, hud.get_viewport_rect().size)
	var names := parts.keys()
	for i in names.size():
		var a: Control = parts[names[i]]
		test._check(screen.grow(0.5).encloses(a.get_global_rect()),
			"%s sticks out of the window (%s): %s in %s" % [names[i], what, a.get_global_rect(), screen.size])
		for j in range(i + 1, names.size()):
			var b: Control = parts[names[j]]
			test._check(not a.get_global_rect().intersects(b.get_global_rect()),
				"%s overlaps %s (%s): %s / %s" % [names[i], names[j], what, a.get_global_rect(), b.get_global_rect()])
	return names.size()


## Prompt 095: what shows when, the fixed places of the order grid, tiles
## against keys, the pages, clicks on the HUD, and the overlap checks at
## other window sizes, interface scales and font sizes.
func _layout_test(test: Node, main: Node3D, hud: Control, tanks: Array[int]) -> void:
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var bar: Control = hud.command_bar
	# Nothing selected: minimap and top bar, support if the player has any,
	# group tabs if groups are in use; nothing else.
	selection.set_selection([])
	await test._frames(15)
	var shown: Array = hud.panels().keys()
	for part: String in ["minimap", "minimap buttons", "camera control", "top bar"]:
		test._check(shown.has(part), "%s does not show without a selection (%s)" % [part, shown])
	for part: String in shown:
		test._check(part in ["minimap", "minimap buttons", "camera control", "top bar", "support column", "group tabs"],
			"%s shows without a selection" % part)
	test._check(not hud.unit_panel.visible and not bar.visible, "info field or order grid still visible without a selection")
	test._check(hud.support.visible == (not sim_host.air_options().is_empty() or main.strike_view.bar.visible),
		"support column visible %s, air options %d" % [hud.support.visible, sim_host.air_options().size()])
	# The camera control (prompt 096) stands for the north button; with the
	# control switched off in the settings the button is there.
	test._check(hud.gizmo.is_visible_in_tree() and not hud.minimap.north_button.is_visible_in_tree(),
		"the camera control does not show, or the north button shows beside it")
	Settings.set_value("camera_gizmo", false)
	await test._frames(3)
	test._check(not hud.gizmo.visible and not hud.panels().has("camera control")
		and not hud.covers(hud.gizmo.get_global_rect().get_center()), "the camera control shows although it is switched off")
	test._check(hud.minimap.north_button.is_visible_in_tree() and hud.minimap.zoom_button.is_visible_in_tree()
		and hud.minimap.pause_button.is_visible_in_tree(), "the minimap buttons do not show")
	test._check(_no_overlap(test, hud, "camera control off") > 0, "no HUD part without the camera control")
	Settings.set_value("camera_gizmo", true)
	await test._frames(3)
	test._check(hud.minimap.zoom_button.is_visible_in_tree() and hud.minimap.pause_button.is_visible_in_tree(),
		"zoom or pause button missing beside the camera control")
	# Camera keys: zoom steps (Shift + wheel), the zoom percentage, close
	# and flat view (Shift + F) and the jump to the fighting (Shift + G).
	var cam: Node3D = main.camera_rig
	var cam_view := [cam.focus, cam.target_distance, cam.yaw]
	for tile: Control in [hud.minimap.overview_button, hud.minimap.close_button, hud.minimap.action_button]:
		test._check(tile.is_visible_in_tree() and not tile.tooltip_text.is_empty(), "camera button %s missing" % tile.name)
	cam.target_distance = cam.start_distance
	cam.step_zoom(1, Vector2.ZERO)
	test._check(cam.zoom_percent() == 150, "a zoom step in from 100 %% gives %d %%" % cam.zoom_percent())
	cam.step_zoom(-1, Vector2.ZERO)
	cam.step_zoom(-1, Vector2.ZERO)
	test._check(cam.zoom_percent() == 75, "two zoom steps out from 150 %% give %d %%" % cam.zoom_percent())
	await test._frames(2)
	test._check(hud.zoom_toast.shown_text.contains("75"), "zoom percentage not shown: '%s'" % hud.zoom_toast.shown_text)
	main.close_view.toggle()
	test._check(main.close_view.active and cam.target_distance <= 30.0 and cam.target_tilt < 0.0,
		"close view: distance %.0f, tilt %.2f" % [cam.target_distance, cam.target_tilt])
	main.close_view.toggle()
	test._check(not main.close_view.active and cam.zoom_percent() == 75, "close view did not restore the zoom (%d %%)" % cam.zoom_percent())
	var fight := Vector3(cam.focus.x + 40.0, 0.0, cam.focus.z)
	hud.action_spots.add_event(fight)
	test._check(hud.action_spots.jump() and cam.is_gliding() and hud.zoom_toast.shown_text.contains("1"),
		"no jump to the fighting (toast '%s')" % hud.zoom_toast.shown_text)
	hud.action_spots.spots.clear()
	cam.snap_to(cam_view[0], cam_view[1], cam_view[2])
	await test._frames(2)
	# Own unit: info field and grid; every tile of the first page on its place.
	selection.set_selection([tanks[0]])
	await test._frames(15)
	shown = hud.panels().keys()
	test._check(shown.has("info field") and shown.has("order grid"), "own tank selected, shown: %s" % [shown])
	for slot in MAIN_PAGE.size():
		var id := MAIN_PAGE[slot]
		var tile: Button = bar.buttons[id]
		test._check(bar.slot_of(id) == slot and tile.is_visible_in_tree() and tile.position.is_equal_approx(bar.slot_rect(slot).position),
			"order '%s' is not on place %d (place %d, at %s)" % [id, slot, bar.slot_of(id), tile.position])
		test._check(not str(tile.icon_id).is_empty() and tile.tooltip_text.length() > 2 and tile.text.is_empty(),
			"tile '%s' has no symbol or tooltip" % id)
	# The context column: orders only some units have show there in gold,
	# only while they fit; the camera buttons of the info field are as
	# large as the order tiles.
	test._check(not bar.buttons.has("more"), "the order grid still has a 'more' page")
	var camo: Button = bar.buttons["camouflage"]
	if sim_host.can_camouflage(tanks[0]):
		test._check(camo.is_visible_in_tree() and bar.slot_of("camouflage") == bar.CONTEXT_SLOT
			and camo.position.is_equal_approx(bar.slot_rect(bar.CONTEXT_SLOT).position),
			"camouflage is not the first context tile of a tank (place %d)" % bar.slot_of("camouflage"))
	for hidden: String in ["disembark", "unlimber", "bridge_lay", "works"]:
		test._check(not bar.buttons[hidden].is_visible_in_tree(), "context tile '%s' shows for a tank" % hidden)
	for camera: Button in [hud.unit_panel.center_button, hud.unit_panel.follow_button]:
		test._check(camera.is_visible_in_tree() and camera.size.is_equal_approx(bar.buttons["move"].size),
			"camera button %s is %s, an order tile %s" % [camera.name, camera.size, bar.buttons["move"].size])
	# A tile gives the same order as its key.
	for pair: Array in [["patrol", KEY_P], ["attack", KEY_R], ["attack_ground", KEY_L]]:
		var tile: Button = bar.buttons[pair[0]]
		test._left_click(tile.get_global_rect().get_center())
		await test._frames(1)
		var by_tile: String = selection.command_mode
		selection.set_command_mode("")
		_press(test, pair[1])
		await test._frames(1)
		test._check(by_tile == pair[0] and selection.command_mode == by_tile,
			"tile '%s' gives '%s', its key '%s'" % [pair[0], by_tile, selection.command_mode])
		selection.set_command_mode("")
	# Pages: formation opens its choices and returns after one; the keys
	# work on any page; "back" leads back.
	test._left_click((bar.buttons["formation"] as Button).get_global_rect().get_center())
	await test._frames(2)
	var wedge: Button = bar.buttons["formation_2"]
	test._check(bar.page == "formation" and wedge.is_visible_in_tree() and not bar.buttons["move"].is_visible_in_tree()
		and bar.buttons["back"].is_visible_in_tree(), "the formation page did not open (page '%s')" % bar.page)
	_press(test, KEY_M)
	await test._frames(1)
	test._check(selection.command_mode == "move", "key M gives no move order on the formation page")
	selection.set_command_mode("")
	var formation_before: int = selection.formation
	test._left_click(wedge.get_global_rect().get_center())
	await test._frames(2)
	test._check(selection.formation == 2 and bar.formation_shown == 2 and bar.page == "main",
		"formation tile: formation %d, shown %d, page '%s'" % [selection.formation, bar.formation_shown, bar.page])
	selection.set_formation(formation_before)
	bar.open_page("stance")
	await test._frames(1)
	test._left_click((bar.buttons["back"] as Button).get_global_rect().get_center())
	await test._frames(2)
	test._check(bar.page == "main" and bar.buttons["move"].is_visible_in_tree(), "'back' did not lead to the first page")
	# Clicks on HUD parts are no orders and keep the selection.
	var one: Array[int] = [tanks[0]]
	var parts: Dictionary = hud.panels()
	# (Not on the top bar: it is all buttons, and those open menus.)
	test._check(hud.covers(hud.top_bar.get_global_rect().get_center()), "the HUD does not count the top bar as a panel")
	var spots: Array[Vector2] = [bar.global_position + bar.slot_rect(3).end + Vector2(1, 1),
		hud.unit_panel.global_position + Vector2(2, 2), hud.minimap.column.global_position + Vector2(1, HudTheme.px(HudTheme.SMALL_TILE_PX) + 1),
		hud.gizmo.global_position + hud.gizmo.size - Vector2(2, 2)]
	if parts.has("support column"):
		var disabled: Button = main.air_view.air_bar.buttons["recon"]
		spots.append(disabled.get_global_rect().end + Vector2(-2, 1))
	for spot in spots:
		test._check(hud.covers(spot), "the HUD does not cover %s" % spot)
		test._left_click(spot)
		test._right_click(spot)
	await test._ticks(4)
	test._check(selection.selected == one and not test._moving(one) and sim_host.unit_queue_length(tanks[0]) == 0
		and selection.command_mode.is_empty(), "a click on a HUD part gave an order or changed the selection (%s, mode '%s')" % [
		selection.selected, selection.command_mode])
	# A unit of the enemy (looked at, not commanded): info field, no grid.
	var enemies: Array[int] = _alive(test, test._ids_of(false))
	test._check(not enemies.is_empty(), "no enemy unit left for the HUD test")
	if not enemies.is_empty():
		selection._set_selection([enemies[0]])
		await test._frames(15)
		shown = hud.panels().keys()
		test._check(shown.has("info field") and not shown.has("order grid") and not bar.visible,
			"enemy unit selected, shown: %s" % [shown])
		test._check(not hud.unit_panel._title.text.is_empty(), "the info field names no enemy unit")
	# Groups: a tab per group in use, a click recalls it.
	var groups_before: Dictionary = selection.groups.duplicate(true)
	selection.groups[7] = [tanks[1], tanks[2]]
	selection.set_selection([])
	await test._frames(15)
	var tab: Button = hud.group_tabs.tabs[7]
	test._check(tab.is_visible_in_tree() and not hud.group_tabs.tabs[8].is_visible_in_tree(), "group 7 has no tab, or the empty group 8 has one")
	test._left_click(tab.get_global_rect().get_center())
	await test._frames(15)
	var pair: Array[int] = [tanks[1], tanks[2]]
	pair.sort()
	test._check(selection.selected == pair and tab.button_pressed, "the tab of group 7 selected %s" % [selection.selected])
	# The buttons beside the minimap; the north button shows while the
	# camera control (prompt 096) is switched off, the dial turns with the
	# camera when it is on.
	var rig: Node3D = main.camera_rig
	rig.snap_to(rig.focus, rig.distance, 0.9)
	await test._frames(2)
	test._check(absf(hud.gizmo.turn - 0.9) < 0.01, "the dial shows %.2f, the camera is turned 0.9" % hud.gizmo.turn)
	Settings.set_value("camera_gizmo", false)
	await test._frames(3)
	test._check(absf(hud.minimap.north_button.turn - 0.9) < 0.01, "the north needle shows %.2f, the camera is turned 0.9" % hud.minimap.north_button.turn)
	test._left_click(hud.minimap.north_button.get_global_rect().get_center())
	await test._frames(1)
	test._check(is_zero_approx(rig.target_yaw), "the north button left the camera heading for %.2f" % rig.target_yaw)
	Settings.set_value("camera_gizmo", true)
	rig.snap_to(rig.focus, rig.distance, 0.0)
	await test._frames(3)
	# Other window sizes, interface scales and font sizes: with nothing,
	# several units and one unit with its card selected.
	var root: Window = test.get_tree().root
	var window := root.size
	var interface := root.content_scale_factor
	var checked := 0
	for case: Array in LAYOUT_CASES:
		root.size = case[0]
		root.content_scale_factor = case[1]
		MenuTheme.set_font_scale(case[2])
		var what := "%s, scale %.2f, font %.2f" % [case[0], case[1], case[2]]
		selection.set_selection([])
		await test._frames(15)
		checked += _no_overlap(test, hud, what + ", nothing selected")
		selection.set_selection(tanks.slice(0, 3))
		await test._frames(15)
		test._check(hud.panels().has("order grid") and hud.panels().has("info field"), "grid or info field missing (%s)" % what)
		checked += _no_overlap(test, hud, what + ", three tanks")
		selection.set_selection([tanks[0]])
		hud.show_unit_card(str(sim_host.unit_info(tanks[0]).type_id), tanks[0])
		await test._frames(15)
		test._check(hud.panels().has("unit card"), "the unit card does not show (%s)" % what)
		checked += _no_overlap(test, hud, what + ", one tank and its card")
		hud.unit_card.close()
	root.size = window
	root.content_scale_factor = interface
	MenuTheme.set_font_scale(Settings.get_value("font_size"))
	selection.groups = groups_before
	selection.set_selection([])
	await test._frames(15)
	print("self-test: hud layout ok (%d cases, %d part checks, factor %.2f)" % [LAYOUT_CASES.size(), checked, HudTheme.scale])


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


## Prompt 101: a hit in view stays quiet, one out of view alarms (sound,
## arrow pointing at it, a click on the arrow jumps there); two places
## alarm twice, the same place only once.
func _alerts_test(test: Node, main: Node3D, alerts: Control, tanks: Array[int]) -> void:
	var sound: Node = main.get("battle_sound")
	var tank: int = tanks[0]
	var here: Vector3 = main.sim_host.current_positions()[test._indices([tank] as Array[int])[0]]
	main.camera_rig.snap_to(here, 60.0, 0.0)
	await test._frames(3)
	var raised: int = alerts.raised
	var heard: int = sound.notices.get("ui_under_attack", 0) if sound != null else 0
	alerts.report("fire", tank, here)
	await test._frames(2)
	test._check(alerts.raised == raised and alerts.arrows.is_empty(),
		"a hit in view raised an alarm (%d) or an arrow (%d)" % [alerts.raised - raised, alerts.arrows.size()])
	# Out of view: east of the camera, on the map (test_small: 500 m).
	var far := Vector3(minf(here.x + 180.0, 220.0), 0.0, here.z)
	far.y = main.sim_host.height_at(far.x, far.z)
	alerts.report("fire", tank, far)
	await test._frames(2)
	test._check(alerts.raised == raised + 1, "a hit out of view raised %d alarms" % (alerts.raised - raised))
	if sound != null:
		test._check(sound.notices.get("ui_under_attack", 0) == heard + 1, "no alarm sound for a hit out of view")
	test._check(alerts.arrows.size() == 1, "%d edge arrows for one place" % alerts.arrows.size())
	if alerts.arrows.size() == 1:
		var camera: Camera3D = main.camera
		var center := alerts.get_viewport_rect().get_center()
		var want: Vector2 = (camera.unproject_position(far) - center).normalized()
		var dir: Vector2 = alerts.arrows[0].dir
		test._check(dir.dot(want) > 0.95, "the arrow points %s, the place lies %s" % [dir, want])
	# The same place again: no second alarm; another place: at once.
	alerts.report("fire", tank, far + Vector3(10.0, 0.0, 0.0))
	var other := Vector3(maxf(here.x - 180.0, -220.0), 0.0, here.z)
	other.y = main.sim_host.height_at(other.x, other.z)
	alerts.report("building", tank, other)
	await test._frames(2)
	test._check(alerts.raised == raised + 2, "two places gave %d alarms, expected 2" % (alerts.raised - raised))
	test._check(alerts.arrows.size() == 2, "%d arrows for two places" % alerts.arrows.size())
	var east: Dictionary = {}
	for a: Dictionary in alerts.arrows:
		if (a.at as Vector3).distance_to(far) < 1.0:
			east = a
	test._check(not east.is_empty() and alerts.click_arrow(east.screen), "no arrow to click for the east place")
	await test._frames(90)
	var focus: Vector3 = main.camera_rig.focus
	test._check(Vector2(focus.x, focus.z).distance_to(Vector2(far.x, far.z)) < 2.0,
		"the arrow click moved the camera to %s, expected %s" % [focus, far])
	# Shift + Space: the newest alarm (the building in the west).
	test._check(alerts.jump_to_alarm(), "the alarm jump found no alarm")
	await test._frames(90)
	focus = main.camera_rig.focus
	test._check(Vector2(focus.x, focus.z).distance_to(Vector2(other.x, other.z)) < 2.0,
		"the alarm jump moved the camera to %s, expected %s" % [focus, other])
	# Prompt 173: a unit stuck out of view alarms with its own feed message.
	var feed_before: int = alerts.feed.history.size()
	var south := Vector3(here.x, 0.0, minf(here.z + 180.0, 220.0))
	south.y = main.sim_host.height_at(south.x, south.z)
	alerts.report("stuck", tank, south)
	await test._frames(2)
	test._check(alerts.raised == raised + 3, "a stuck unit out of view raised %d alarms, expected 3 in all" % (alerts.raised - raised))
	var last: Dictionary = alerts.feed.history.back() if alerts.feed.history.size() > feed_before else {}
	test._check(last.get("kind", "") == "stuck", "no feed message for a stuck unit (%s)" % [last])
	main.camera_rig.snap_to(here, 120.0, 0.0)
	await test._frames(2)


## Prompt 175: a dropped order (unlimbering tanks without a gun) shows its
## reason at the pointer and in the feed; the activity line of a tank;
## a target beyond the map edge moves the preview mark to where the tanks
## really stop.
func _order_feedback_test(test: Node, main: Node3D, hud: Control, tanks: Array[int]) -> void:
	var feedback: Control = hud.order_feedback
	var shown: int = feedback.shown
	main.sim_host.unlimber_units(PackedInt64Array(tanks))
	await test._frames(10)
	test._check(feedback.shown == shown + 1, "a dropped order showed %d hints" % (feedback.shown - shown))
	test._check(feedback.hint == tr("ORDER_REJECTED_NOTHING_TO_UNLIMBER"), "hint '%s'" % feedback.hint)
	var last: Dictionary = hud.event_feed.history.back() if not hud.event_feed.history.is_empty() else {}
	test._check(last.get("kind", "") == "rejected", "no feed message for a dropped order (%s)" % [last])
	main.selection.set_selection([tanks[0]])
	await test._frames(20)
	test._check(hud.unit_panel.activity_line.begins_with(tr("HUD_ACTIVITY").get_slice("%", 0)),
		"no activity line: '%s'" % hud.unit_panel.activity_line)
	var preview: Control = hud.order_preview
	var here: Vector3 = main.sim_host.current_positions()[test._indices([tanks[0]] as Array[int])[0]]
	main.camera_rig.snap_to(here, 400.0, 0.0)
	preview._fetch(Vector3(here.x, 0.0, here.z + 2000.0), Vector3.ZERO, tanks)
	await test._frames(3)
	test._check(preview.substitutes > 0, "a target beyond the map edge moved no preview mark")
	preview.slots = PackedFloat32Array()
	await test._frames(2)


## Prompt 101: the whole army holds no structure and no unarmed unit; the
## idle key finds a standing unit and never a driving one.
func _selection_keys_test(test: Node, main: Node3D, tanks: Array[int]) -> void:
	var selection: Control = main.selection
	var sim_host: SimHost = main.sim_host
	selection.select_army()
	var army: Array[int] = selection.selected.duplicate()
	var bad := army.filter(func(id: int) -> bool:
		return selection.is_structure.call(id) or (sim_host.unit_info(id).get("weapons", []) as Array).is_empty())
	test._check(army.size() >= tanks.size() and bad.is_empty(),
		"whole army: %d units, %s structures or unarmed" % [army.size(), bad])
	# One tank drives off; it must not count as idle.
	var mover: int = tanks[0]
	var at: Vector3 = sim_host.current_positions()[test._indices([mover] as Array[int])[0]]
	sim_host.move_units(PackedInt64Array([mover]), at.x + 60.0, at.z)
	await test._ticks(3)
	selection.select_idle(true)
	var idle: Array[int] = selection.selected.duplicate()
	test._check(not idle.has(mover), "the driving tank %d counts as idle" % mover)
	var flags := sim_host.unit_idle()
	var wrong := idle.filter(func(id: int) -> bool: return flags[test._indices([id] as Array[int])[0]] == 0)
	test._check(not idle.is_empty() and wrong.is_empty(), "idle selection %s, not idle among them %s" % [idle, wrong])
	selection.select_idle(false)
	test._check(selection.selected.size() == 1 and idle.has(selection.selected[0]),
		"the idle key selected %s" % [selection.selected])
	sim_host.stop_units(PackedInt64Array([mover]))
	await test._ticks(2)
	selection.set_selection([])


## Prompt 100: the ground tooltip over a road, woods, deep water and a
## house names them and gives the levels of the rules; it waits for the
## pointer to rest and stays away during a box selection.
func _ground_tooltip_test(test: Node, main: Node3D, tip: Control) -> void:
	var sim_host: SimHost = main.sim_host
	var found := {}
	var x := -240.0
	while x <= 240.0 and found.size() < 4:
		var z := -240.0
		while z <= 240.0:
			var info := sim_host.ground_info(x, z)
			var object: String = info.get("object", "")
			if not found.has("road") and info.get("terrain_name", "") == "road" and object.is_empty():
				found["road"] = Vector3(x, 0, z)
			elif not found.has("woods") and info.get("sight", "") == "foliage" and object.begins_with("env_tree"):
				found["woods"] = Vector3(x, 0, z)
			elif not found.has("water") and info.get("terrain_name", "") == "deep_water" and object.is_empty():
				found["water"] = Vector3(x, 0, z)
			elif not found.has("house") and info.get("building", false):
				found["house"] = Vector3(x, 0, z)
			z += 4.0
		x += 4.0
	test._check(found.size() == 4, "test_small lacks a road, woods, deep water or a house: %s" % [found.keys()])
	main.selection.set_selection([])
	var center: Vector2 = tip.get_viewport_rect().get_center()
	var none := tr("GROUND_NONE")
	if found.has("road"):
		tip.show_at(found["road"], center)
		test._check(tip.visible and tip.title == tr("GROUND_T_ROAD") and tip.lines.has(tr("GROUND_MOVE") % [tr("GROUND_VEHICLES"), tr("GROUND_MOVE_ROAD")]),
			"road tooltip: %s / %s" % [tip.title, tip.lines])
		test._check(tip.lines.has(tr("GROUND_SIGHT") % none), "a road gives concealment: %s" % [tip.lines])
	if found.has("woods"):
		tip.show_at(found["woods"], center)
		test._check(tip.title == tr("GROUND_WOODS") and tip.lines.has(tr("GROUND_SIGHT") % tr("GROUND_STRONG"))
			and tip.lines.has(tr("GROUND_COVER") % tr("GROUND_COVER_HARD")), "woods tooltip: %s / %s" % [tip.title, tip.lines])
	if found.has("water"):
		tip.show_at(found["water"], center)
		test._check(tip.title == tr("GROUND_T_DEEP_WATER")
			and tip.lines.has(tr("GROUND_MOVE") % [tr("GROUND_VEHICLES"), tr("GROUND_MOVE_NONE")])
			and tip.lines.has(tr("GROUND_MOVE") % [tr("GROUND_INFANTRY"), tr("GROUND_MOVE_NONE_FOOT")]),
			"water tooltip: %s / %s" % [tip.title, tip.lines])
	if found.has("house"):
		tip.show_at(found["house"], center)
		test._check(tip.lines.has(tr("GROUND_OCCUPY")) and tip.lines.has(tr("GROUND_COVER") % tr("GROUND_COVER_HARD"))
			and tip.lines.has(tr("GROUND_SIGHT") % tr("GROUND_STRONG")), "house tooltip: %s / %s" % [tip.title, tip.lines])
	# Rest and box selection, through the frame update with a fixed point.
	tip._hide()
	tip.forced_point = found.get("road", Vector3.ZERO)
	tip._last_mouse = tip.get_viewport().get_mouse_position()
	tip._rest = 0.0
	tip._update(0.2)
	test._check(not tip.visible, "the ground tooltip showed before the pointer rested")
	tip._update(0.3)
	test._check(tip.visible, "the ground tooltip did not show after resting 0.5 s")
	main.selection._dragging = true
	tip._update(0.1)
	test._check(not tip.visible, "the ground tooltip shows during a box selection")
	main.selection._dragging = false
	tip.forced_point = null
	tip.visible = false


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
	test._check(not rect.intersects(hud.top_bar.get_global_rect()) and not (hud.support.visible and rect.intersects(hud.support.get_global_rect())),
		"unit card covers the top bar or the support column")
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
