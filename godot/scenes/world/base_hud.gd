extends Control
## Base building HUD (prompt 023): resources, build menu, placement preview,
## production of the selected building and its rally point. Presentation
## only: every order goes to `SimHost`, which checks it again.
##
## - The resource bar shows fuel, ammunition and manpower with the income
##   per minute.
## - The build menu (bottom left) lists what the player can build with its
##   price; a greyed-out button says why not. Clicking one starts the
##   placement: a box of the building's size follows the mouse, green where
##   the simulation would accept it, red otherwise (the reason is shown).
##   Left click places it (Shift keeps placing), right click or Esc cancels.
## - With one own producing building selected, the production bar shows
##   what it makes, the queue with the progress of the first order, and a
##   button that cancels the last order (full refund). Right click on the
##   ground sets the building's rally point.
## - Prompt 036: with one own building or construction site selected, the
##   bar has "Repair" (start / stop, paid while it runs) and "Sell" (a
##   site: "Cancel site"); see docs/GAME_DESIGN.md "Supply and repair".
##
## Hidden in matches without base building. Created by `main.gd` in the HUD.
## Prompt 031: the command bar docks the build menu and the production bar
## (`dock_into`); its "Build" button opens the build menu.
## Prompt 058: both show picture tiles in tabs (`hud/picker.gd`); Alt + 1…0
## picks a tile of the production bar if it shows, else of the build menu.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const RESOURCE_NAMES := ["Fuel", "Ammunition", "Manpower"]
const OK_COLOR := Color(0.3, 0.9, 0.35, 0.45)
const BAD_COLOR := Color(0.95, 0.25, 0.2, 0.45)
const GHOST_HEIGHT := 4.0
## The panels are rebuilt this often (s).
const REFRESH := 0.25

var main: Node3D
var sim_host: SimHost
var base_view: Node
## Unit type being placed, -1 when not placing.
var placing := -1
## Reason of the last placement check at the mouse ("" = valid).
var placement_reason := ""
## Unit ids of the buildings the production bar shows.
var production_building := -1
## The own building the Repair and Sell buttons act on (prompt 036), or -1.
var service_building := -1

var _resources: Label
var _build_panel: PanelContainer
var _production_panel: PanelContainer
var _production_box: VBoxContainer
## Picture tiles of the build menu and the production bar (prompt 058).
var build_picker: Picker
var production_picker: Picker
var _queue_label: Label
var _cancel: Button
var _repair: Button
var _sell: Button
var _hint: Label
var _ghost: MeshInstance3D
var _ghost_material: StandardMaterial3D
var _refresh_left := 0.0
var _enabled := false
## The command bar holding the panels (prompt 031), or null.
var _command_bar: Node


func setup(main_node: Node3D, view: Node) -> void:
	main = main_node
	sim_host = main.sim_host
	base_view = view
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	var top := PanelContainer.new()
	top.set_anchors_preset(Control.PRESET_CENTER_TOP)
	top.offset_left = -330
	top.offset_right = 330
	top.offset_top = 6
	_resources = Label.new()
	_resources.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	top.add_child(_resources)
	add_child(top)

	_build_panel = PanelContainer.new()
	_build_panel.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	_build_panel.offset_left = 8
	_build_panel.offset_right = 8
	_build_panel.offset_top = -8
	_build_panel.offset_bottom = -8
	_build_panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	build_picker = Picker.new()
	build_picker.name = "BuildPicker"
	build_picker.picked.connect(func(o: Dictionary) -> void: start_placing(o.type))
	_build_panel.add_child(build_picker)
	add_child(_build_panel)

	_production_panel = PanelContainer.new()
	_production_panel.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	_production_panel.offset_left = -230
	_production_panel.offset_right = 230
	_production_panel.offset_top = -8
	_production_panel.offset_bottom = -8
	_production_panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	_production_box = VBoxContainer.new()
	_production_panel.add_child(_production_box)
	production_picker = Picker.new()
	production_picker.name = "ProductionPicker"
	production_picker.picked.connect(func(o: Dictionary) -> void:
		if production_building >= 0:
			sim_host.produce(production_building, o.type))
	_production_box.add_child(production_picker)
	_queue_label = Label.new()
	_queue_label.add_theme_font_size_override(&"font_size", 13)
	_queue_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_production_box.add_child(_queue_label)
	_cancel = Button.new()
	_cancel.text = "Cancel last"
	_cancel.add_theme_font_size_override(&"font_size", 13)
	_cancel.pressed.connect(func() -> void:
		if production_building >= 0:
			sim_host.cancel_production(production_building))
	_production_box.add_child(_cancel)
	var service := HBoxContainer.new()
	_production_box.add_child(service)
	_repair = Button.new()
	_repair.text = "Repair"
	_repair.tooltip_text = "Repair the building over time (half its price for a full repair); again to stop"
	_repair.pressed.connect(func() -> void:
		if service_building >= 0:
			sim_host.repair_building(service_building))
	service.add_child(_repair)
	_sell = Button.new()
	_sell.text = "Sell"
	_sell.pressed.connect(func() -> void:
		if service_building >= 0:
			sim_host.sell_building(service_building)
			main.selection.set_selection([]))
	service.add_child(_sell)
	_production_panel.visible = false
	add_child(_production_panel)

	_hint = Label.new()
	_hint.set_anchors_preset(Control.PRESET_CENTER_TOP)
	_hint.offset_left = -300
	_hint.offset_right = 300
	_hint.offset_top = 46
	_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	add_child(_hint)

	_ghost = MeshInstance3D.new()
	_ghost.mesh = BoxMesh.new()
	_ghost_material = StandardMaterial3D.new()
	_ghost_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_ghost_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_ghost.material_override = _ghost_material
	_ghost.visible = false
	main.add_child(_ghost)
	_refresh()


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh()
	if placing >= 0:
		_update_ghost(get_viewport().get_mouse_position())


## Moves the build menu and the production bar into `dock` (the command
## bar, prompt 031); `bar.build_menu_open()` decides whether the build menu
## shows.
func dock_into(dock: Control, bar: Node) -> void:
	_command_bar = bar
	for panel: Control in [_production_panel, _build_panel]:
		remove_child(panel)
		panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
		panel.grow_vertical = Control.GROW_DIRECTION_END
		dock.add_child(panel)
	_refresh()


## Rebuilds the panels now (the command bar opened the build menu).
func refresh_now() -> void:
	_refresh()


## Starts placing `unit_type` (also used by the self-test).
func start_placing(unit_type: int) -> void:
	placing = unit_type
	var size := sim_host.structure_size(unit_type)
	(_ghost.mesh as BoxMesh).size = Vector3(size.x, GHOST_HEIGHT, size.y)
	_ghost.visible = true


func cancel_placing() -> void:
	placing = -1
	_ghost.visible = false
	_hint.text = ""


## Places the building at the ground point; returns whether the simulation
## would accept it (the order is sent either way and checked there).
func place_at(point: Vector3) -> bool:
	if placing < 0:
		return false
	var reason := sim_host.placement_check(placing, point.x, point.z)
	if not reason.is_empty():
		_hint.text = "Cannot build here: %s" % reason
		return false
	sim_host.place_building(placing, point.x, point.z)
	return true


func _update_ghost(screen: Vector2) -> void:
	var hit: Variant = GroundPick.from_screen(main.camera, screen)
	if hit == null:
		_ghost.visible = false
		return
	var point := hit as Vector3
	_ghost.visible = true
	_ghost.position = point + Vector3.UP * GHOST_HEIGHT * 0.5
	placement_reason = sim_host.placement_check(placing, point.x, point.z)
	_ghost_material.albedo_color = OK_COLOR if placement_reason.is_empty() else BAD_COLOR
	_hint.text = "Place: left click · cancel: right click" if placement_reason.is_empty() else placement_reason


func _input(event: InputEvent) -> void:
	if not _enabled:
		return
	var button := event as InputEventMouseButton
	var key := event as InputEventKey
	var hotkey := Picker.hotkey_index(key)
	if hotkey >= 0:
		var picker: Picker = production_picker if _production_panel.visible and production_building >= 0 else (build_picker if _build_panel.visible else null)
		if picker != null:
			picker.press_index(hotkey)
			get_viewport().set_input_as_handled()
			return
	if placing >= 0:
		if key != null and key.pressed and key.keycode == KEY_ESCAPE:
			cancel_placing()
			get_viewport().set_input_as_handled()
		elif button != null and button.pressed:
			if button.button_index == MOUSE_BUTTON_RIGHT:
				cancel_placing()
				get_viewport().set_input_as_handled()
			elif button.button_index == MOUSE_BUTTON_LEFT and not _over_panel(button.position):
				var hit: Variant = GroundPick.from_screen(main.camera, button.position)
				if hit != null and place_at(hit as Vector3) and not button.shift_pressed:
					cancel_placing()
				get_viewport().set_input_as_handled()
		return
	# Right click with only own buildings selected: rally point.
	if button != null and button.pressed and button.button_index == MOUSE_BUTTON_RIGHT \
			and not button.ctrl_pressed and not button.meta_pressed:
		var selected: Array = main.selection.selected
		var own: Array[int] = base_view.own_structures(selected)
		if not own.is_empty() and own.size() == selected.size():
			var hit: Variant = GroundPick.from_screen(main.camera, button.position)
			if hit != null:
				var point := hit as Vector3
				for id in own:
					sim_host.set_rally_point(id, point.x, point.z)
				main.selection.move_ordered.emit(point)
			get_viewport().set_input_as_handled()


func _over_panel(screen: Vector2) -> bool:
	for panel: Control in [_build_panel, _production_panel]:
		if panel.visible and panel.get_global_rect().has_point(screen):
			return true
	return false


func _refresh() -> void:
	var state := sim_host.economy_state()
	_enabled = state.get("enabled", false)
	visible = _enabled
	if _command_bar != null:
		_build_panel.visible = _enabled and _command_bar.build_menu_open()
		_production_panel.visible = _production_panel.visible and _enabled
	if not _enabled:
		return
	var income: PackedInt64Array = state.income
	var parts: Array[String] = []
	for i in 3:
		var key: String = ["fuel", "ammo", "manpower"][i]
		parts.append("%s %d (+%d/min)" % [RESOURCE_NAMES[i], state[key], income[i]])
	_resources.text = "    ".join(parts)
	# Own construction sites per type, and the progress of the furthest.
	var sites := {}
	var site_progress := {}
	var me := sim_host.local_player()
	for s: Dictionary in base_view.structures.values():
		if int(s.owner) == me and float(s.built) < 1.0:
			var t := int(s.type)
			sites[t] = int(sites.get(t, 0)) + 1
			site_progress[t] = maxf(float(site_progress.get(t, 0.0)), float(s.built))
	build_picker.set_options("Build", sim_host.build_options(), sites, site_progress)
	_refresh_production()


func _refresh_production() -> void:
	var own: Array[int] = base_view.own_structures(main.selection.selected)
	production_building = -1
	var options: Array = []
	for id in own:
		options = sim_host.production_options(id)
		if not options.is_empty():
			production_building = id
			break
	service_building = own[0] if own.size() == 1 and main.selection.selected.size() == 1 else -1
	_production_panel.visible = production_building >= 0 or service_building >= 0
	_refresh_service()
	for part: Control in [production_picker, _queue_label, _cancel]:
		part.visible = production_building >= 0
	if production_building < 0:
		return
	var s: Dictionary = base_view.structures[production_building]
	var queue: PackedInt32Array = s.queue
	var queued := {}
	for t in queue:
		queued[t] = int(queued.get(t, 0)) + 1
	var first := {queue[0]: float(s.progress)} if not queue.is_empty() else {}
	production_picker.set_options(s.name, options, queued, first)
	var names: Array[String] = []
	for i in queue.size():
		var text := sim_host.unit_type_name(queue[i])
		if i == 0:
			text += " %d %%" % int(float(s.progress) * 100.0)
		names.append(text)
	if float(s.built) < 1.0:
		names = ["under construction %d %%" % int(float(s.built) * 100.0)]
	_queue_label.text = "Queue: " + (", ".join(names) if not names.is_empty() else "empty")
	_cancel.disabled = queue.is_empty()


## Repair and Sell of the selected own building (prompt 036).
func _refresh_service() -> void:
	_repair.get_parent().visible = service_building >= 0
	if service_building < 0:
		return
	var s: Dictionary = base_view.structures.get(service_building, {})
	var site := float(s.get("built", 1.0)) < 1.0
	_sell.text = "Cancel site" if site else "Sell"
	_sell.tooltip_text = "Cancel the construction site (75 % back)" if site else "Sell the building (half its price back)"
	var hp := 0
	var max_hp := 1
	var index := sim_host.unit_ids().find(service_building)
	if index >= 0:
		var info := sim_host.unit_info(service_building)
		hp = int(info.get("hp", 0))
		max_hp = maxi(int(info.get("max_hp", 1)), 1)
	var repairing := sim_host.building_repairing(service_building)
	_repair.text = "Stop repair" if repairing else "Repair"
	_repair.disabled = site or (hp >= max_hp and not repairing)
