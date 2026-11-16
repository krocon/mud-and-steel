extends Control
## Match HUD (prompt 031): minimap, unit panel, command bar, event feed and
## waypoint lines. Created by `main.gd` in the HUD layer; presentation only,
## every order goes through `selection.gd` or `SimHost`.
##
## Layout: command bar bottom left (with the build menu and the production
## bar of prompt 023 docked above it), unit panel bottom center, minimap
## bottom right, events at the left edge.
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

var main: Node3D
var minimap: Minimap
var unit_panel: UnitPanel
var command_bar: CommandBar
var event_feed: EventFeed
var waypoints: Waypoints
## Supply and repair radii of selected sources (prompt 036).
var supply_rings: SupplyRings
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


## The event feed sits at the left edge; when the build menu or production
## bar (prompt 058 picture tiles) makes the command bar tall, it moves up
## so the two do not overlap.
func _keep_events_above_command_bar() -> void:
	var top := command_bar.get_global_rect().position.y
	var middle := get_viewport_rect().size.y * 0.5
	var y := minf(middle - 40.0, top - event_feed.size.y - 8.0)
	event_feed.global_position.y = maxf(y, 64.0)


## Whether the screen point lies on a HUD panel (clicks there are not
## orders).
func covers(screen: Vector2) -> bool:
	for part: Control in [minimap, unit_panel, command_bar, event_feed]:
		if part.visible and part.get_global_rect().has_point(screen):
			return true
	return false
