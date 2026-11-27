extends Control
## Match HUD (prompts 031 and 095). Created by `main.gd` in the HUD layer;
## presentation only, every order goes through `selection.gd` or `SimHost`.
##
## Every part has a fixed corner, so nothing pushes anything else around:
##
## | Where | Part | Shows |
## |-------|------|-------|
## | bottom left | `minimap` with its north mark, `minimap.column` right of it | always |
## | above the minimap | `gizmo`: camera control (turn, tilt, home; prompt 096) | unless switched off in the settings |
## | bottom center | `unit_panel`, the info field of the selection | with a selection |
## | bottom right | `command_bar`, the 5 x 3 order grid and its context column | with an own selection that takes orders |
## | above the grid | `dock`: build menu and production bar (`base_hud.gd`) | while open |
## | top right | `top_bar`: resources, build, help, menu | always |
## | right, under it | `support`: air support and rocket strike tiles | when the player has support |
## | top center | `group_tabs`, under them `hints` (what a pending order waits for) | groups in use |
## | left | `event_feed`, above the minimap | messages |
##
## The grid's room is kept free while it is hidden, so showing it moves
## nothing but the dock, which stands in the grid's corner meanwhile. The
## camera control stands in a strip of `HudTheme.GIZMO_ROOM` above the
## minimap; while it shows, the north button of the minimap's column is
## hidden (the dial shows north, a click on its "N" turns there).
## `HudTheme.scale` sizes all parts; it is set here every frame and the
## parts are told when it changed (`apply_scale`).
##
## Prompt 066: the unit card (`unit_card.gd`) opens by a right click on a
## picker tile, by the key `unit_card` (Shift + I) for the selected unit and
## by the button of the info field. It sits left of the support column,
## under the top bar and above the bottom row, takes no keyboard focus
## (orders keep working, the match runs on) and closes with Escape.
##
## `usec` is the time all HUD parts took in the last frame (their
## `_process` and `_draw`), for the self-test and `--perf-bench`.

const Minimap := preload("res://scenes/hud/minimap.gd")
const UnitPanel := preload("res://scenes/hud/unit_panel.gd")
const CommandBar := preload("res://scenes/hud/command_bar.gd")
const EventFeed := preload("res://scenes/hud/event_feed.gd")
const Waypoints := preload("res://scenes/hud/waypoints.gd")
const SupplyRings := preload("res://scenes/hud/supply_rings.gd")
const RangeRings := preload("res://scenes/hud/range_rings.gd")
const OrderPreview := preload("res://scenes/hud/order_preview.gd")
const Alerts := preload("res://scenes/hud/alerts.gd")
const ZoomToast := preload("res://scenes/hud/zoom_toast.gd")
const ActionSpots := preload("res://scenes/hud/action_spots.gd")
const OrderFeedback := preload("res://scenes/hud/order_feedback.gd")
const WorldFeedback := preload("res://scenes/hud/world_feedback.gd")
const GroundTooltip := preload("res://scenes/hud/ground_tooltip.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const Subtitles := preload("res://scenes/hud/subtitles.gd")
const UnitCard := preload("res://scenes/hud/unit_card.gd")
const TopBar := preload("res://scenes/hud/top_bar.gd")
const GroupTabs := preload("res://scenes/hud/group_tabs.gd")
const CameraGizmo := preload("res://scenes/hud/camera_gizmo.gd")
const Settings := preload("res://scenes/menu/settings.gd")
## Unit card: preferred and smallest size.
const CARD_SIZE := Vector2(540, 620)
const CARD_MIN := Vector2(400, 260)
## Height of the replay bar (`replay/replay_bar.gd`), which lies over the
## bottom edge while a replay runs.
const REPLAY_BAR := 64.0

var main: Node3D
var minimap: Minimap
## Camera control above the minimap (prompt 096).
var gizmo: CameraGizmo
var unit_panel: UnitPanel
var command_bar: CommandBar
var event_feed: EventFeed
var waypoints: Waypoints
## Supply and repair radii of selected sources (prompt 036).
var supply_rings: SupplyRings
## Range and sight circles of the selection (prompt 099).
var range_rings: RangeRings
## Slots of a formation move before and just after the order (prompt 099).
var order_preview: OrderPreview
## Alarms out of view and their edge arrows (prompt 101).
var alerts: Alerts
## "Zoom 150 %" and the messages of the camera keys; places of combat for
## the jump key (Shift + G).
var zoom_toast: ZoomToast
var action_spots: ActionSpots
## Dropped orders and the penetration light at the pointer (prompt 175).
var order_feedback: OrderFeedback
## World-space pop-ups, building garrison plaques and supply hover labels (prompt 236).
var world_feedback: WorldFeedback
## What the ground under the resting mouse does (prompt 100).
var ground_tooltip: GroundTooltip
## Text lines for warnings and callouts (prompt 043).
var subtitles: Subtitles
## Unit card (prompt 066), hidden until asked for.
var unit_card: UnitCard
## Resources, build, help and menu (prompt 095).
var top_bar: TopBar
## Tabs of the control groups in use.
var group_tabs: GroupTabs
## Column of support tiles under the top bar (`air_bar.gd`, the strike
## tile of `strike_view.gd` put theirs in).
var support: GridContainer
## Build menu and production bar, above the order grid.
var dock: VBoxContainer
## Hint lines of pending orders (air support, rocket strike, placing a
## building), top center under the group tabs.
var hints: VBoxContainer
## HUD time of the last frame in microseconds, and the largest so far.
var usec := 0
var max_usec := 0
var _parts: Array[Control] = []
var _scale := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "MatchHud"
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	HudTheme.scale = HudTheme.scale_for(get_viewport_rect().size)
	# Behind the HUD panels of other prompts, above the health bars.
	waypoints = Waypoints.new()
	add_child(waypoints)
	waypoints.setup(main)
	supply_rings = SupplyRings.new()
	add_child(supply_rings)
	supply_rings.setup(main)
	range_rings = RangeRings.new()
	add_child(range_rings)
	range_rings.setup(main)
	order_preview = OrderPreview.new()
	add_child(order_preview)
	order_preview.setup(main)
	world_feedback = WorldFeedback.new()
	add_child(world_feedback)
	world_feedback.setup(main)
	minimap = Minimap.new()
	add_child(minimap)
	minimap.setup(main)
	add_child(minimap.column)
	gizmo = CameraGizmo.new()
	add_child(gizmo)
	gizmo.setup(main.camera_rig)
	unit_panel = UnitPanel.new()
	add_child(unit_panel)
	unit_panel.setup(main)
	unit_panel.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	unit_panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	hints = VBoxContainer.new()
	hints.name = "Hints"
	hints.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hints.add_theme_constant_override(&"separation", 2)
	add_child(hints)
	dock = VBoxContainer.new()
	dock.name = "Dock"
	dock.mouse_filter = Control.MOUSE_FILTER_IGNORE
	dock.alignment = BoxContainer.ALIGNMENT_END
	dock.add_theme_constant_override(&"separation", 6)
	dock.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	dock.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	dock.grow_vertical = Control.GROW_DIRECTION_BEGIN
	add_child(dock)
	command_bar = CommandBar.new()
	add_child(command_bar)
	command_bar.setup(main)
	top_bar = TopBar.new()
	add_child(top_bar)
	top_bar.setup(main)
	support = GridContainer.new()
	support.name = "Support"
	# The gaps between its tiles take clicks too: no order slips through.
	support.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(support)
	group_tabs = GroupTabs.new()
	add_child(group_tabs)
	group_tabs.setup(main)
	event_feed = EventFeed.new()
	add_child(event_feed)
	event_feed.setup(main)
	event_feed.ping.connect(minimap.add_ping)
	alerts = Alerts.new()
	add_child(alerts)
	alerts.setup(main, event_feed)
	zoom_toast = ZoomToast.new()
	add_child(zoom_toast)
	zoom_toast.setup(main)
	action_spots = ActionSpots.new()
	add_child(action_spots)
	action_spots.setup(main, event_feed, zoom_toast)
	order_feedback = OrderFeedback.new()
	add_child(order_feedback)
	order_feedback.setup(main, event_feed)
	subtitles = Subtitles.new()
	add_child(subtitles)
	unit_card = UnitCard.new()
	unit_card.name = "UnitCard"
	unit_card.visible = false
	add_child(unit_card)
	ground_tooltip = GroundTooltip.new()
	add_child(ground_tooltip)
	ground_tooltip.setup(main)
	unit_panel.card_requested.connect(show_unit_card)
	var base_hud: Node = main.base_hud
	if base_hud != null:
		base_hud.dock_into(dock, command_bar, hints)
		for picker: Node in [base_hud.get("build_picker"), base_hud.get("production_picker")]:
			if picker != null:
				picker.connect(&"info_requested", show_unit_card)
	_parts = [waypoints, supply_rings, range_rings, order_preview, world_feedback, minimap, gizmo, unit_panel, command_bar, event_feed, alerts, ground_tooltip, top_bar, group_tabs, zoom_toast]
	_layout()


var _layout_key := []


func _process(_delta: float) -> void:
	# The parts add their time of this frame; read it next frame.
	var total := 0
	for part in _parts:
		total += part.get("usec")
		part.set("usec", 0)
	usec = total
	max_usec = maxi(max_usec, usec)
	_layout()
	if unit_card.visible:
		_place_unit_card()


## Shows the unit card of catalog id `type_id`; with `unit` (an own unit)
## also the bonuses of its rank.
func show_unit_card(type_id: String, unit := -1) -> void:
	if type_id.is_empty():
		return
	unit_card.show_type(type_id, unit, main.sim_host)
	_place_unit_card()


## The key `unit_card`: the card of the active unit (prompt 094; the one
## last clicked); closes it if it shows that type already (or nothing is
## selected).
func toggle_unit_card() -> void:
	var selected: Array[int] = main.selection.selected
	var unit: int = main.selection.active_unit
	if unit < 0 and not selected.is_empty():
		unit = selected[0]
	var type_id := ""
	if unit >= 0:
		type_id = str(main.sim_host.unit_info(unit).get("type_id", ""))
	if type_id.is_empty() or (unit_card.visible and unit_card.type_id == type_id):
		unit_card.close()
	else:
		show_unit_card(type_id, unit)


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


## Puts every part into its corner. Runs every frame: the window, the
## interface scale and the font size may change at any time, and the
## panels that grow with their content (info field, dock) settle here.
func _layout() -> void:
	var screen := get_viewport_rect().size
	var factor := HudTheme.scale_for(screen)
	var scale_changed := not is_equal_approx(factor, _scale)
	var gizmo_on := bool(Settings.get_value("camera_gizmo"))
	var support_mask := 0
	var bit := 1
	for tile: Control in support.get_children():
		if tile.visible:
			support_mask |= bit
		bit <<= 1
	var mission: Control = main.mission_hud
	var mission_bottom := (mission.position.y + mission.size.y) if (mission != null and mission.visible) else 0.0
	var key := [
		screen, factor, gizmo_on, support_mask,
		minimap.size, minimap.column.size, gizmo.size,
		command_bar.size, HudTheme.shown(command_bar),
		unit_panel.size, HudTheme.shown(unit_panel),
		top_bar.size, dock.size, group_tabs.size,
		hints.get_combined_minimum_size(), event_feed.size, mission_bottom,
	]
	# Prompt 208: skip layout pass when screen, scale and panel sizes are unchanged.
	if not scale_changed and key == _layout_key:
		return
	_layout_key = key
	if scale_changed:
		_scale = factor
		HudTheme.scale = factor
		for child in get_children():
			if child.has_method(&"apply_scale"):
				child.call(&"apply_scale")
		var base_hud: Node = main.base_hud
		if base_hud != null:
			base_hud.apply_scale()
		support.add_theme_constant_override(&"h_separation", int(HudTheme.px(HudTheme.GAP)))
		support.add_theme_constant_override(&"v_separation", int(HudTheme.px(HudTheme.GAP)))
		hints.add_theme_font_size_override(&"font_size", HudTheme.font(14))
	var margin := HudTheme.MARGIN
	var bottom := screen.y - margin - (REPLAY_BAR if main.sim_host.is_replay() else 0.0)
	# Bottom left: the minimap and its buttons.
	minimap.position = Vector2(margin, bottom - minimap.size.y)
	# The camera control above it; the north button only without it.
	gizmo.visible = gizmo_on
	minimap.show_north_button(not gizmo.visible)
	gizmo.position = Vector2(margin, minimap.position.y - HudTheme.px(HudTheme.GIZMO_GAP) - gizmo.size.y)
	var column: Control = minimap.column
	column.position = Vector2(minimap.position.x + minimap.size.x + HudTheme.px(HudTheme.GAP), bottom - column.size.y)
	# Bottom right: the order grid; its room stays free while it is hidden.
	command_bar.position = Vector2(screen.x - margin - command_bar.size.x, bottom - command_bar.size.y)
	# Top right: the top bar, the support column under it.
	top_bar.position = Vector2(screen.x - margin - top_bar.size.x, margin)
	var support_top := top_bar.position.y + top_bar.size.y + margin
	var tiles := 0
	var tile_height := 0.0
	for tile: Control in support.get_children():
		if tile.visible:
			tiles += 1
			tile_height = tile.custom_minimum_size.y + HudTheme.px(HudTheme.GAP)
	support.visible = tiles > 0
	# A second column when one does not fit above the order grid.
	var room := command_bar.position.y - margin - support_top
	support.columns = clampi(ceili(tiles * tile_height / maxf(room, 1.0)), 1, 3)
	support.size = Vector2.ZERO
	support.position = Vector2(screen.x - margin - support.size.x, support_top)
	# The dock stands on the order grid (in its corner while the grid is
	# hidden), left of the support column where it reaches up to it.
	var dock_stands := command_bar.position.y if HudTheme.shown(command_bar) else bottom + margin
	if HudTheme.shown(unit_panel):
		dock_stands = minf(dock_stands, unit_panel.position.y)
	var dock_bottom := screen.y - dock_stands + margin
	var beside := support.visible and screen.y - dock_bottom - dock.size.y < support.position.y + support.size.y + margin
	var dock_right := margin + (support.size.x + margin if beside else 0.0)
	dock.offset_left = -dock_right
	dock.offset_right = -dock_right
	dock.offset_top = -dock_bottom
	dock.offset_bottom = -dock_bottom
	# Bottom center: the info field, between minimap and order grid.
	var left := column.position.x + column.size.x + margin
	var right := command_bar.position.x - margin
	var width := minf(HudTheme.px(UnitPanel.WIDTH), right - left)
	unit_panel.custom_minimum_size.x = width
	var x := clampf((screen.x - width) * 0.5, left, maxf(right - width, left))
	unit_panel.offset_left = x
	unit_panel.offset_right = x + width
	unit_panel.offset_top = bottom - screen.y
	unit_panel.offset_bottom = bottom - screen.y
	# Top center: the group tabs, the hint lines under them.
	group_tabs.position = Vector2((screen.x - group_tabs.size.x) * 0.5, margin)
	hints.size = Vector2.ZERO
	hints.position = Vector2((screen.x - hints.size.x) * 0.5, margin + HudTheme.px(34))
	# Left: the messages, clear of the minimap and the room kept above it,
	# and under the mission panel.
	var feed_top := 64.0
	if mission != null and mission.visible:
		feed_top = maxf(feed_top, mission_bottom + margin)
	var feed_y := minf(screen.y * 0.5 - 40.0, minimap.position.y - HudTheme.px(HudTheme.GIZMO_ROOM) - event_feed.size.y - margin)
	event_feed.global_position = Vector2(margin, maxf(feed_y, feed_top))
	# Subtitles above the info field.
	subtitles.offset_bottom = minf(-200.0, unit_panel.position.y - margin - screen.y) if HudTheme.shown(unit_panel) else -200.0


## The card sits under the top bar, left of the support column (and of the
## dock while that is open), above the bottom row. On a small screen it
## gets narrower and shorter; its pages scroll.
func _place_unit_card() -> void:
	var screen := get_viewport_rect().size
	var margin := HudTheme.MARGIN
	var right := screen.x - margin
	if support.visible:
		right = support.position.x - margin
	if _dock_open():
		right = minf(right, dock.position.x - margin)
	var top := top_bar.position.y + top_bar.size.y + margin
	var bottom := command_bar.position.y - margin
	if HudTheme.shown(unit_panel):
		bottom = minf(bottom, unit_panel.position.y - margin)
	var width := clampf(right - margin, minf(CARD_MIN.x, screen.x - 2.0 * margin), CARD_SIZE.x)
	var height := clampf(bottom - top, minf(CARD_MIN.y, screen.y - top - margin), CARD_SIZE.y)
	unit_card.custom_minimum_size = Vector2(width, 0.0)
	unit_card.position = Vector2(maxf(right - width, margin), top)
	unit_card.size = Vector2(width, height)


## Whether the dock shows a panel.
func _dock_open() -> bool:
	for panel: Control in dock.get_children():
		if panel.visible:
			return true
	return false


## The HUD parts that take clicks, by name (self-test: none may overlap
## another). A part that is hidden or fading out is left out.
func panels() -> Dictionary[String, Control]:
	var out: Dictionary[String, Control] = {}
	var all: Dictionary[String, Control] = {"minimap": minimap, "minimap buttons": minimap.column, "camera control": gizmo, "info field": unit_panel,
		"order grid": command_bar, "top bar": top_bar, "support column": support, "group tabs": group_tabs,
		"unit card": unit_card}
	for key: String in all:
		if HudTheme.shown(all[key]) and all[key].size.x > 0.0:
			out[key] = all[key]
	for panel: Control in dock.get_children():
		if panel.visible:
			out["dock " + panel.name] = panel
	return out


## Whether the screen point lies on a HUD panel (clicks there are not
## orders).
func covers(screen: Vector2) -> bool:
	var parts := panels()
	for key: String in parts:
		if parts[key].get_global_rect().has_point(screen):
			return true
	return event_feed.visible and event_feed.get_global_rect().has_point(screen)
