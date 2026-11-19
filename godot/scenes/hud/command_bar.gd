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
##
## Formations (prompt 046): the first cells of the grid set the formation
## of the next moves (`selection.formation`, key `,` cycles); attack-move
## ("Advance") and secure are targeted orders like move.
##
## Size and folding (prompt 092): the buttons carry larger text (`MenuTheme.HUD_FONT`) and
## shrink on a narrow window (`MenuTheme.hud_fit`); the header line folds
## the grid away (setting `hud_commands_open`, kept over matches). Folded,
## every key still gives its order: the keys are read by the selection, not
## by the buttons.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const TransportOrders := preload("res://scenes/selection/transport_orders.gd")
const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const FormationOrders := preload("res://scenes/selection/formation_orders.gd")
const CrossingOrders := preload("res://scenes/selection/crossing_orders.gd")
const REFRESH := 0.2
## id, label (translation key), action (key shown), kind: "mode" (targeted), "now", "stance".
const BUTTONS: Array[Array] = [
	["move", "HUD_CMD_MOVE", "order_move", "mode"],
	["attack", "HUD_CMD_ATTACK", "order_attack", "mode"],
	["attack_ground", "HUD_CMD_GROUND", "order_attack_ground", "mode"],
	# Shelling a building, wall or tree until it is destroyed (prompt 093).
	["attack_object", "HUD_CMD_SHELL", "order_attack_object", "mode"],
	["patrol", "HUD_CMD_PATROL", "order_patrol", "mode"],
	["attack_move", "HUD_CMD_ADVANCE", "order_attack_move", "mode"],
	["secure", "HUD_CMD_SECURE", "order_secure", "mode"],
	["stop", "HUD_CMD_STOP", "order_stop", "now"],
	["fire", "HUD_CMD_HOLD_FIRE", "order_fire_mode", "now"],
	["garrison", "HUD_CMD_OCCUPY", "order_garrison", "mode"],
	["barrage", "HUD_CMD_BARRAGE", "order_barrage", "mode"],
	["smoke", "HUD_CMD_SMOKE", "order_smoke", "mode"],
	["disembark", "HUD_CMD_DISMOUNT", "order_disembark", "now"],
	["unlimber", "HUD_CMD_UNLIMBER", "order_unlimber", "now"],
	["stance_stand", "HUD_CMD_STAND", "stance_stand", "stance"],
	["stance_kneel", "HUD_CMD_KNEEL", "stance_kneel", "stance"],
	["stance_prone", "HUD_CMD_PRONE", "stance_prone", "stance"],
	["stance_auto", "HUD_CMD_AUTO", "stance_auto", "stance"],
	["build", "HUD_CMD_BUILD", "", "build"],
	# Engineer orders (prompt 035): shown while engineers are selected.
	["work_mines_at", "HUD_WORK_MINES_AT", "", "work"],
	["work_mines_ap", "HUD_WORK_MINES_AP", "", "work"],
	["work_wire", "HUD_WORK_WIRE", "", "work"],
	["work_hedgehogs", "HUD_WORK_HEDGEHOGS", "", "work"],
	["work_foxhole", "HUD_WORK_FOXHOLE", "", "work"],
	["work_trench", "HUD_WORK_TRENCH", "", "work"],
	["work_clear", "HUD_WORK_CLEAR", "", "work"],
	["work_bridge", "HUD_WORK_BRIDGE", "", "work"],
	# Positions (prompt 052).
	["work_comm_trench", "HUD_WORK_COMM_TRENCH", "", "work"],
	["work_mg_nest", "HUD_WORK_MG_NEST", "", "work"],
	["work_dugout", "HUD_WORK_DUGOUT", "", "work"],
	["work_vehicle_pit", "HUD_WORK_VEHICLE_PIT", "", "work"],
	["work_camo_net", "HUD_WORK_CAMO_NET", "", "work"],
	["work_bunker", "HUD_WORK_BUNKER", "", "work"],
	["work_teeth", "HUD_WORK_TEETH", "", "work"],
	# Bridge-laying tanks and camouflage (prompt 055): shown while such
	# units are selected.
	["bridge_lay", "HUD_CMD_LAY_BRIDGE", "", "layer"],
	["bridge_pickup", "HUD_CMD_TAKE_UP", "", "layer_now"],
	["camouflage", "HUD_CMD_CAMOUFLAGE", "", "camo"],
]

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Button per id (self-test).
var buttons: Dictionary[String, Button] = {}
## Whether the build menu is open.
var build_open := false
## Formation choice (prompt 046; self-test).
var formation_choice: OptionButton

## Header line that folds the grid (self-test).
var fold_button: Button

var _dock: VBoxContainer
var _panel: PanelContainer
var _grid: GridContainer
var _formation_label: Label
var _fit := 0.0
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
	var box := VBoxContainer.new()
	_panel.add_child(box)
	fold_button = Button.new()
	fold_button.name = "Fold"
	fold_button.flat = true
	fold_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	fold_button.focus_mode = Control.FOCUS_NONE
	fold_button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	fold_button.tooltip_text = "HUD_TIP_FOLD"
	fold_button.pressed.connect(func() -> void: set_open(not is_open()))
	box.add_child(fold_button)
	var grid := GridContainer.new()
	grid.columns = 5
	box.add_child(grid)
	_grid = grid
	_add_formation_choice(grid)
	for spec in BUTTONS:
		var b := Button.new()
		b.name = "Order_" + spec[0]
		b.focus_mode = Control.FOCUS_NONE
		b.toggle_mode = spec[3] == "mode" or spec[3] == "build" or spec[3] == "work" \
				or spec[3] == "layer"
		b.pressed.connect(press.bind(spec[0]))
		grid.add_child(b)
		buttons[spec[0]] = b
	main.selection.command_mode_changed.connect(func(_mode: String) -> void: _update())
	var base_hud: Node = main.get("base_hud")
	if base_hud != null and base_hud.has_method("dock_into"):
		base_hud.dock_into(_dock, self)
	_set_tooltips()
	_fit_size()
	set_open(Settings.get_value("hud_commands_open"), false)
	_update()


## Whether the order grid is folded open.
func is_open() -> bool:
	return _grid.visible


## Folds the order grid open or away; `remember` keeps it in the settings.
func set_open(open: bool, remember := true) -> void:
	_grid.visible = open
	fold_button.text = ("▾ " if open else "▸ ") + tr("HUD_ORDERS")
	if remember and Settings.get_value("hud_commands_open") != open:
		Settings.set_value("hud_commands_open", open)
		Settings.save_file()


## Button sizes and fonts for the window width and the font size setting.
func _fit_size() -> void:
	var fit := MenuTheme.hud_fit(get_viewport_rect().size.x) if is_inside_tree() else 1.0
	var signature := fit * 1000.0 + MenuTheme.font_scale
	if is_equal_approx(signature, _fit):
		return
	_fit = signature
	for id: String in buttons:
		MenuTheme.style_hud_button(buttons[id], fit)
	MenuTheme.style_hud_button(formation_choice, fit)
	MenuTheme.style_hud_button(_formation_label, fit)
	fold_button.add_theme_font_size_override(&"font_size", MenuTheme.font(roundi(MenuTheme.HUD_FONT * fit)))


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_fit_size()
		_update()
	usec += Time.get_ticks_usec() - began


## The formation choice of prompt 046 (label and list), the first two
## cells of the grid.
func _add_formation_choice(grid: GridContainer) -> void:
	var label := Label.new()
	label.text = "HUD_FORMATION"
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	grid.add_child(label)
	_formation_label = label
	formation_choice = OptionButton.new()
	formation_choice.name = "Formation"
	formation_choice.focus_mode = Control.FOCUS_NONE
	formation_choice.fit_to_longest_item = false
	formation_choice.clip_text = true
	for code in FormationOrders.LABELS.size():
		formation_choice.add_item(FormationOrders.label(code), code)
	formation_choice.tooltip_text = tr("HUD_TIP_FORMATION") % Settings.action_key_name("order_formation")
	formation_choice.item_selected.connect(func(index: int) -> void: main.selection.set_formation(index))
	main.selection.formation_changed.connect(func(code: int) -> void: formation_choice.select(code))
	formation_choice.select(main.selection.formation)
	grid.add_child(formation_choice)


## Presses the button `id` (also used by the self-test).
func press(id: String) -> void:
	var selection: Control = main.selection
	for spec in BUTTONS:
		if spec[0] != id:
			continue
		match spec[3]:
			"mode", "work", "layer":
				selection.set_command_mode("" if selection.command_mode == id else id)
			"layer_now":
				CrossingOrders.pick_up(selection)
			"camo":
				CrossingOrders.toggle_camouflage(selection)
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
	var layers := CrossingOrders.layers(selection).size()
	var camo := CrossingOrders.camouflageable(selection)
	# Units with a gun that damages objects (prompt 093).
	var shells := mobile > 0 and sim_host.can_shell_objects(PackedInt64Array(selected))
	for spec in BUTTONS:
		var b: Button = buttons[spec[0]]
		var key := Settings.action_key_name(spec[2]) if not (spec[2] as String).is_empty() else ""
		var label := tr(spec[1])
		if spec[0] == "fire":
			label = tr("HUD_CMD_FIRE_AT_WILL") if holding > 0 and holding == selected.size() else tr("HUD_CMD_HOLD_FIRE")
		b.text = label if key.is_empty() else "%s [%s]" % [label, key]
		match spec[3]:
			"mode":
				b.disabled = mobile == 0 or (spec[0] == "garrison" and squads == 0) \
						or (spec[0] == "barrage" and guns == 0) or (spec[0] == "smoke" and smoke == 0) \
						or (spec[0] == "attack_object" and not shells)
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
			"layer":
				b.visible = layers > 0
				b.set_pressed_no_signal(selection.command_mode == spec[0])
			"layer_now":
				b.visible = layers > 0
			"camo":
				b.visible = not camo.is_empty()
				var on := 0
				for id: int in camo:
					if sim_host.camouflage_state(id) > 0:
						on += 1
				b.text = tr("HUD_CMD_UNCOVER") if on > 0 and on == camo.size() else tr("HUD_CMD_CAMOUFLAGE")


func _notification(what: int) -> void:
	# Tooltips with numbers and keys are built text: rebuild them in the
	# new language.
	if what == NOTIFICATION_TRANSLATION_CHANGED and sim_host != null and not buttons.is_empty():
		_set_tooltips()
		set_open(is_open(), false)
		formation_choice.tooltip_text = tr("HUD_TIP_FORMATION") % Settings.action_key_name("order_formation")


func _set_tooltips() -> void:
	buttons["move"].tooltip_text = "HUD_TIP_MOVE"
	buttons["attack"].tooltip_text = "HUD_TIP_ATTACK"
	buttons["attack_ground"].tooltip_text = "HUD_TIP_GROUND"
	buttons["attack_object"].tooltip_text = "HUD_TIP_SHELL"
	buttons["attack_move"].tooltip_text = tr("HUD_TIP_ADVANCE") % Settings.action_key_name("order_move")
	buttons["secure"].tooltip_text = tr("HUD_TIP_SECURE") % FormationOrders.SECURE_RADIUS
	buttons["patrol"].tooltip_text = "HUD_TIP_PATROL"
	buttons["garrison"].tooltip_text = "HUD_TIP_OCCUPY"
	buttons["barrage"].tooltip_text = "HUD_TIP_BARRAGE"
	buttons["smoke"].tooltip_text = "HUD_TIP_SMOKE"
	buttons["disembark"].tooltip_text = "HUD_TIP_DISMOUNT"
	buttons["unlimber"].tooltip_text = "HUD_TIP_UNLIMBER"
	buttons["build"].tooltip_text = "HUD_TIP_BUILD"
	buttons["bridge_lay"].tooltip_text = "HUD_TIP_LAY_BRIDGE"
	buttons["bridge_pickup"].tooltip_text = "HUD_TIP_TAKE_UP"
	buttons["camouflage"].tooltip_text = "HUD_TIP_CAMOUFLAGE"
	for spec in BUTTONS:
		if spec[3] == "work":
			var kind := EngineerOrders.kind_of(spec[0])
			buttons[spec[0]].tooltip_text = _work_tooltip(kind)


## Tooltip of an engineer button: what it does and what one piece costs.
func _work_tooltip(kind: String) -> String:
	match kind:
		"clear":
			return tr("HUD_TIP_WORK_CLEAR")
		"bridge":
			return tr("HUD_TIP_WORK_BRIDGE")
	var cost := sim_host.work_cost(kind)
	var price := ""
	if cost.size() >= 4:
		price = " " + tr("HUD_TIP_WORK_PRICE") % [cost[0], cost[1], cost[2], cost[3]]
	var how := tr("HUD_TIP_WORK_LINE") if EngineerOrders.KINDS[kind][1] else tr("HUD_TIP_WORK_POINT")
	return how + price
