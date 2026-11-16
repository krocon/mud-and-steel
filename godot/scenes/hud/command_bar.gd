extends VBoxContainer
## Command bar (prompt 031), bottom left.
##
## A grid of order buttons for the selection, each with its key from the
## settings: move, attack, fire at ground, patrol and occupy wait for a
## left click (`selection.command_mode`); stop, fire mode and the four
## stances act at once. Buttons that do not fit the selection are disabled
## (stances and occupy need a squad, orders need a unit that can move,
## barrage and smoke a unit with an indirect weapon, prompt 032; their keys
## fire at the cursor at once, the buttons wait for a click).
##
## Base building (prompt 023, [provisional]): the build menu and the
## production bar of `base_hud.gd` are docked above the grid. "Build" opens
## and closes the build menu; a selected producing building shows its
## production bar as before.

const Settings := preload("res://scenes/menu/settings.gd")
const TransportOrders := preload("res://scenes/selection/transport_orders.gd")
const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const REFRESH := 0.2
## id, label, action (key shown), kind: "mode" (targeted), "now", "stance".
const BUTTONS: Array[Array] = [
	["move", "Move", "order_move", "mode"],
	["attack", "Attack", "order_attack", "mode"],
	["attack_ground", "Ground", "order_attack_ground", "mode"],
	["patrol", "Patrol", "order_patrol", "mode"],
	["stop", "Stop", "order_stop", "now"],
	["fire", "Hold fire", "order_fire_mode", "now"],
	["garrison", "Occupy", "order_garrison", "mode"],
	["barrage", "Barrage", "order_barrage", "mode"],
	["smoke", "Smoke", "order_smoke", "mode"],
	["disembark", "Dismount", "order_disembark", "now"],
	["unlimber", "Unlimber", "order_unlimber", "now"],
	["stance_stand", "Stand", "stance_stand", "stance"],
	["stance_kneel", "Kneel", "stance_kneel", "stance"],
	["stance_prone", "Prone", "stance_prone", "stance"],
	["stance_auto", "Auto", "stance_auto", "stance"],
	["build", "Build", "", "build"],
	# Engineer orders (prompt 035): shown while engineers are selected.
	["work_mines_at", "AT mines", "", "work"],
	["work_mines_ap", "AP mines", "", "work"],
	["work_wire", "Wire", "", "work"],
	["work_hedgehogs", "Obstacles", "", "work"],
	["work_foxhole", "Foxhole", "", "work"],
	["work_trench", "Trench", "", "work"],
	["work_clear", "Clear", "", "work"],
	["work_bridge", "Bridge", "", "work"],
]

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Button per id (self-test).
var buttons: Dictionary[String, Button] = {}
## Whether the build menu is open.
var build_open := false

var _dock: VBoxContainer
var _panel: PanelContainer
var _refresh_left := 0.0
var _keys_signature := ""


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "CommandBar"
	set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	offset_left = 8
	offset_right = 8
	offset_bottom = -8
	offset_top = -8
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	alignment = BoxContainer.ALIGNMENT_END
	_dock = VBoxContainer.new()
	_dock.name = "Dock"
	_dock.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_dock)
	_panel = PanelContainer.new()
	_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(_panel)
	var grid := GridContainer.new()
	grid.columns = 4
	_panel.add_child(grid)
	for spec in BUTTONS:
		var b := Button.new()
		b.name = "Order_" + spec[0]
		b.custom_minimum_size = Vector2(92, 34)
		b.focus_mode = Control.FOCUS_NONE
		b.toggle_mode = spec[3] == "mode" or spec[3] == "build" or spec[3] == "work"
		b.add_theme_font_size_override(&"font_size", 13)
		b.pressed.connect(press.bind(spec[0]))
		grid.add_child(b)
		buttons[spec[0]] = b
	main.selection.command_mode_changed.connect(func(_mode: String) -> void: _update())
	var base_hud: Node = main.get("base_hud")
	if base_hud != null and base_hud.has_method("dock_into"):
		base_hud.dock_into(_dock, self)
	_set_tooltips()
	_update()


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_update()
	usec += Time.get_ticks_usec() - began


## Presses the button `id` (also used by the self-test).
func press(id: String) -> void:
	var selection: Control = main.selection
	for spec in BUTTONS:
		if spec[0] != id:
			continue
		match spec[3]:
			"mode", "work":
				selection.set_command_mode("" if selection.command_mode == id else id)
			"stance":
				selection.set_stance({"stance_stand": 0, "stance_kneel": 1, "stance_prone": 2, "stance_auto": -1}[id])
			"build":
				build_open = not build_open
				var base_hud: Node = main.get("base_hud")
				if base_hud != null and base_hud.has_method("refresh_now"):
					base_hud.refresh_now()
			"now":
				if id == "stop":
					selection.stop_selected()
				elif id == "disembark":
					TransportOrders.disembark(selection)
				elif id == "unlimber":
					TransportOrders.unlimber(selection)
				else:
					selection.toggle_hold_fire()
	_update()


## Whether the build menu should show (read by `base_hud.gd`).
func build_menu_open() -> bool:
	return build_open


func _update() -> void:
	var selection: Control = main.selection
	var selected: Array[int] = selection.selected
	var ids := sim_host.unit_ids()
	var stances := sim_host.unit_stances()
	var hold := sim_host.unit_hold_fire()
	var base_view: Node = main.get("base_view")
	var mobile := 0
	var squads := 0
	var holding := 0
	var guns := 0
	var smoke := 0
	for id: int in selected:
		var i := ids.find(id)
		if i < 0:
			continue
		if base_view == null or not base_view.structures.has(id):
			mobile += 1
		if stances[i] >= 0:
			squads += 1
		holding += hold[i]
		var weapon := sim_host.unit_indirect(id)
		if weapon.y > 0.0:
			guns += 1
			if weapon.z > 0.0:
				smoke += 1
	var economy: bool = sim_host.economy_state().get("enabled", false)
	# Vehicles with passengers, towing links (prompt 034).
	var transport := TransportOrders.counts(selection)
	var engineers := EngineerOrders.engineers(selection).size()
	for spec in BUTTONS:
		var b: Button = buttons[spec[0]]
		var key := Settings.action_key_name(spec[2]) if not (spec[2] as String).is_empty() else ""
		var label: String = spec[1]
		if spec[0] == "fire":
			label = "Fire at will" if holding > 0 and holding == selected.size() else "Hold fire"
		b.text = label if key.is_empty() else "%s [%s]" % [label, key]
		match spec[3]:
			"mode":
				b.disabled = mobile == 0 or (spec[0] == "garrison" and squads == 0) \
						or (spec[0] == "barrage" and guns == 0) or (spec[0] == "smoke" and smoke == 0)
				b.set_pressed_no_signal(selection.command_mode == spec[0])
			"stance":
				b.disabled = squads == 0
			"now":
				b.disabled = selected.is_empty() or (spec[0] == "disembark" and transport.x == 0) \
						or (spec[0] == "unlimber" and transport.y == 0)
			"build":
				b.visible = economy
				b.set_pressed_no_signal(build_open)
			"work":
				b.visible = engineers > 0
				b.set_pressed_no_signal(selection.command_mode == spec[0])


func _set_tooltips() -> void:
	buttons["move"].tooltip_text = "Then left click the target. Right click moves at once; Shift + right click adds a waypoint."
	buttons["attack"].tooltip_text = "Then left click an enemy. Right click on an enemy attacks at once."
	buttons["attack_ground"].tooltip_text = "Then left click a ground point. Ctrl + right click fires there at once."
	buttons["patrol"].tooltip_text = "Then left click a point: the units drive back and forth."
	buttons["garrison"].tooltip_text = "Then left click a building: squads occupy it."
	buttons["barrage"].tooltip_text = "Mortars and guns: then left click a point. The key fires at the cursor at once."
	buttons["smoke"].tooltip_text = "Mortars and guns with smoke rounds: then left click a point. The key fires at the cursor at once."
	buttons["disembark"].tooltip_text = "Passengers of the selected vehicles get off once they stand. Right click a vehicle with squads selected to mount."
	buttons["unlimber"].tooltip_text = "Selected vehicles drop their towed guns. Right click an own gun with vehicles selected to hitch it."
	buttons["build"].tooltip_text = "Opens the build menu."
	for spec in BUTTONS:
		if spec[3] == "work":
			var kind := EngineerOrders.kind_of(spec[0])
			buttons[spec[0]].tooltip_text = _work_tooltip(kind)


## Tooltip of an engineer button: what it does and what one piece costs.
func _work_tooltip(kind: String) -> String:
	match kind:
		"clear":
			return "Engineers: then left click an area. They clear known mines, wire and obstacles there."
		"bridge":
			return "Engineers: then left click a bridge. An intact one is blown up, a destroyed one repaired."
	var cost := sim_host.work_cost(kind)
	var price := ""
	if cost.size() >= 4:
		price = " Each piece: fuel %d, ammo %d, manpower %d, %d s." % [cost[0], cost[1], cost[2], cost[3]]
	var how := "two left clicks (start and end of the line)" if EngineerOrders.KINDS[kind][1] else "a left click"
	return "Engineers: then %s.%s" % [how, price]
