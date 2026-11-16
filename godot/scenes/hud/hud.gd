extends Control
## Match HUD (prompt 031): minimap, unit panel, command bar, event feed and
## waypoint lines. Created by `main.gd` in the HUD layer; presentation only,
## every order goes through `selection.gd` or `SimHost`.
##
## Layout: command bar bottom left (with the build menu and the production
## bar of prompt 023 docked above it), unit panel bottom center, minimap
## bottom right, events at the left edge.
##
## Prompt 066: the unit card (`unit_card.gd`) opens by a right click on a
## picker tile, by the key `unit_card` (Shift + I) for the selected unit and
## by the button of the unit panel. It sits top right, clear of command bar
## and minimap, takes no keyboard focus (orders keep working, the match
## runs on) and closes with Escape.
##
## `usec` is the time all HUD parts took in the last frame (their
## `_process` and `_draw`), for the self-test and `--perf-bench`.

const Minimap := preload("res://scenes/hud/minimap.gd")
const UnitPanel := preload("res://scenes/hud/unit_panel.gd")
const CommandBar := preload("res://scenes/hud/command_bar.gd")
const EventFeed := preload("res://scenes/hud/event_feed.gd")
const Waypoints := preload("res://scenes/hud/waypoints.gd")
const SupplyRings := preload("res://scenes/hud/supply_rings.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Subtitles := preload("res://scenes/hud/subtitles.gd")
const UnitCard := preload("res://scenes/hud/unit_card.gd")
## Unit card: preferred and smallest size, and the top edge below the
## resource bar.
const CARD_SIZE := Vector2(540, 620)
const CARD_MIN := Vector2(400, 260)
const CARD_TOP := 60.0

var main: Node3D
var minimap: Minimap
var unit_panel: UnitPanel
var command_bar: CommandBar
var event_feed: EventFeed
var waypoints: Waypoints
## Supply and repair radii of selected sources (prompt 036).
var supply_rings: SupplyRings
## Text lines for warnings and callouts (prompt 043).
var subtitles: Subtitles
## Unit card (prompt 066), hidden until asked for.
var unit_card: UnitCard
## HUD time of the last frame in microseconds, and the largest so far.
var usec := 0
var max_usec := 0
var _parts: Array[Control] = []


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "MatchHud"
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	# Behind the HUD panels of other prompts, above the health bars.
	waypoints = Waypoints.new()
	add_child(waypoints)
	waypoints.setup(main)
	supply_rings = SupplyRings.new()
	add_child(supply_rings)
	supply_rings.setup(main)
	minimap = Minimap.new()
	add_child(minimap)
	minimap.setup(main)
	unit_panel = UnitPanel.new()
	add_child(unit_panel)
	unit_panel.setup(main)
	command_bar = CommandBar.new()
	add_child(command_bar)
	command_bar.setup(main)
	event_feed = EventFeed.new()
	add_child(event_feed)
	event_feed.setup(main)
	event_feed.ping.connect(minimap.add_ping)
	subtitles = Subtitles.new()
	add_child(subtitles)
	unit_card = UnitCard.new()
	unit_card.name = "UnitCard"
	unit_card.visible = false
	add_child(unit_card)
	unit_panel.card_requested.connect(show_unit_card)
	var base_hud: Node = main.get("base_hud")
	if base_hud != null:
		for picker: Node in [base_hud.get("build_picker"), base_hud.get("production_picker")]:
			if picker != null:
				picker.connect(&"info_requested", show_unit_card)
	_parts = [waypoints, supply_rings, minimap, unit_panel, command_bar, event_feed]


func _process(_delta: float) -> void:
	# The parts add their time of this frame; read it next frame.
	var total := 0
	for part in _parts:
		total += part.get("usec")
		part.set("usec", 0)
	usec = total
	max_usec = maxi(max_usec, usec)
	_keep_events_above_command_bar()
	_keep_unit_panel_beside_command_bar()
	if unit_card.visible:
		_place_unit_card()


## Shows the unit card of catalog id `type_id`; with `unit` (an own unit)
## also the bonuses of its rank.
func show_unit_card(type_id: String, unit := -1) -> void:
	if type_id.is_empty():
		return
	unit_card.show_type(type_id, unit, main.sim_host)
	_place_unit_card()


## The key `unit_card`: the card of the first selected unit; closes it if
## it shows that type already (or nothing is selected).
func toggle_unit_card() -> void:
	var selected: Array[int] = main.selection.selected
	var type_id := ""
	if not selected.is_empty():
		type_id = str(main.sim_host.unit_info(selected[0]).get("type_id", ""))
	if type_id.is_empty() or (unit_card.visible and unit_card.type_id == type_id):
		unit_card.close()
	else:
		show_unit_card(type_id, selected[0])


## Escape closes the open card before anything else reacts to it.
func _input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if unit_card != null and unit_card.visible and key != null and key.pressed and not key.echo \
			and key.keycode == KEY_ESCAPE:
		unit_card.close()
		get_viewport().set_input_as_handled()


func _unhandled_input(event: InputEvent) -> void:
	if unit_card != null and event.is_action_pressed("unit_card", false, true):
		toggle_unit_card()
		get_viewport().set_input_as_handled()


## The card sits top right: right of the command bar (with the build menu
## docked it reaches far up), above the minimap, and above the unit panel
## when that sits over the minimap and there is room left. On a small
## screen it gets narrower and shorter; its pages scroll.
func _place_unit_card() -> void:
	var screen := get_viewport_rect().size
	var width := minf(CARD_SIZE.x, screen.x - 16.0)
	if command_bar.visible:
		width = clampf(screen.x - 16.0 - command_bar.get_global_rect().end.x, minf(CARD_MIN.x, screen.x - 16.0), width)
	var x := screen.x - 8.0 - width
	var bottom := screen.y - 8.0
	if minimap.visible:
		bottom = minimap.get_global_rect().position.y - 8.0
	var panel := unit_panel.get_global_rect()
	if unit_panel.visible and panel.end.x > x and panel.position.y - 8.0 - CARD_TOP >= CARD_MIN.y:
		bottom = minf(bottom, panel.position.y - 8.0)
	var height := clampf(bottom - CARD_TOP, minf(CARD_MIN.y, screen.y - CARD_TOP - 8.0), CARD_SIZE.y)
	unit_card.custom_minimum_size = Vector2(width, 0.0)
	unit_card.position = Vector2(x, CARD_TOP)
	unit_card.size = Vector2(width, height)


## The event feed sits at the left edge; when the build menu or production
## bar (prompt 058 picture tiles) makes the command bar tall, it moves up
## so the two do not overlap.
func _keep_events_above_command_bar() -> void:
	var top := command_bar.get_global_rect().position.y
	var middle := get_viewport_rect().size.y * 0.5
	var y := minf(middle - 40.0, top - event_feed.size.y - 8.0)
	event_feed.global_position.y = maxf(y, 64.0)
	# No room above it (the picker reaches up to the resource bar): beside
	# it, so the messages do not lie over the tiles (prompt 066).
	event_feed.global_position.x = command_bar.get_global_rect().end.x + 8.0 if y < 64.0 else 8.0


## The unit panel sits bottom center; on a narrow screen (or a large UI
## scale, prompt 043) it moves right so it does not cover the command bar.
## If it does not fit between command bar and minimap either, it goes
## above the minimap.
func _keep_unit_panel_beside_command_bar() -> void:
	var screen := get_viewport_rect().size
	var x := (screen.x - unit_panel.size.x) * 0.5
	if command_bar.visible:
		x = maxf(x, command_bar.get_global_rect().end.x + 8.0)
	var map_rect := minimap.get_global_rect()
	var y := screen.y - 8.0 - unit_panel.size.y
	if minimap.visible and x + unit_panel.size.x > map_rect.position.x - 8.0:
		x = screen.x - 8.0 - unit_panel.size.x
		y = map_rect.position.y - 8.0 - unit_panel.size.y
	unit_panel.global_position = Vector2(maxf(x, 0.0), y)


## Whether the screen point lies on a HUD panel (clicks there are not
## orders).
func covers(screen: Vector2) -> bool:
	for part: Control in [minimap, unit_panel, command_bar, event_feed, unit_card]:
		if part.visible and part.get_global_rect().has_point(screen):
			return true
	return false
