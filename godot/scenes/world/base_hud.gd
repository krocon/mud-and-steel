extends Control
## Base building HUD (prompt 023): resources, build menu, placement preview,
## production of the selected building and its rally point. Presentation
## only: every order goes to `SimHost`, which checks it again.
##
## - The stocks of fuel, ammunition and manpower show in the top bar of the
##   match HUD (`hud/top_bar.gd`, prompt 095), which also has the button
##   that opens the build menu.
## - The build menu lists what the player can build with its
##   price; a greyed-out button says why not. Clicking one starts the
##   placement: a box of the building's size follows the mouse, green where
##   the simulation would accept it, red otherwise (the reason is shown).
##   Left click places it (Shift keeps placing), right click or Esc cancels.
## - With one own producing building selected, the production bar shows
##   what it makes and the queue with the progress of the first order.
##   Right click on the ground sets the building's rally point.
## - Prompt 036: with one own building or construction site selected it can
##   be repaired (start / stop, paid while it runs) and sold (a site:
##   cancelled); see docs/GAME_DESIGN.md "Supply and repair".
## - Prompt 095: both panels sit in the dock above the order grid
##   (`hud.gd`); while the build menu is open it takes the place of the
##   production bar. "Repair", "Sell" and "Cancel last" are tiles of the
##   order grid (`command_bar.gd`, page "building"), which calls
##   `toggle_repair`, `sell`, `cancel_last` and reads `service_state`.
##
## Hidden in matches without base building. Created by `main.gd` in the HUD.
## Prompt 058: both show picture tiles in tabs (`hud/picker.gd`); Alt + 1…0
## picks a tile of the production bar if it shows, else of the build menu.

const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
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

var _build_panel: PanelContainer
var _production_panel: PanelContainer
var _production_box: VBoxContainer
## Picture tiles of the build menu and the production bar (prompt 058).
var build_picker: Picker
var production_picker: Picker
var _queue_label: Label
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
	_queue_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_production_box.add_child(_queue_label)
	_production_panel.visible = false
	add_child(_production_panel)

	_hint = Label.new()
	_hint.name = "BuildHint"
	_hint.set_anchors_preset(Control.PRESET_CENTER_TOP)
	_hint.offset_left = -300
	_hint.offset_right = 300
	_hint.offset_top = 46
	_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_hint.add_theme_color_override(&"font_outline_color", Color.BLACK)
	_hint.add_theme_constant_override(&"outline_size", 5)
	add_child(_hint)

	_ghost = MeshInstance3D.new()
	_ghost.mesh = BoxMesh.new()
	_ghost_material = StandardMaterial3D.new()
	_ghost_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_ghost_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_ghost_material.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
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


## Moves the build menu and the production bar into `dock` (above the
## order grid) and the placement hint into `hints`;
## `bar.build_menu_open()` decides whether the build menu shows.
func dock_into(dock: Control, bar: Node, hints: Control = null) -> void:
	_command_bar = bar
	for panel: Control in [_production_panel, _build_panel]:
		remove_child(panel)
		panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
		panel.grow_vertical = Control.GROW_DIRECTION_END
		dock.add_child(panel)
	if hints != null:
		remove_child(_hint)
		hints.add_child(_hint)
	apply_scale()
	_refresh()


## Look and sizes for the HUD factor (`hud.gd` calls it when it changed).
func apply_scale() -> void:
	for panel: Control in [_production_panel, _build_panel]:
		panel.add_theme_stylebox_override(&"panel", HudTheme.panel_box())
	_queue_label.add_theme_font_size_override(&"font_size", HudTheme.font(12))
	_hint.add_theme_font_size_override(&"font_size", HudTheme.font(14))


## Starts or stops the repair of the selected own building (prompt 036).
func toggle_repair() -> void:
	if service_building >= 0:
		sim_host.repair_building(service_building)


## Sells the selected own building (a site: cancels it).
func sell() -> void:
	if service_building >= 0:
		sim_host.sell_building(service_building)
		main.selection.set_selection([])


## Cancels the last order of the production queue (full refund).
func cancel_last() -> void:
	if production_building >= 0:
		sim_host.cancel_production(production_building)


## What the order grid shows for the selected own building: whether there
## is one to serve (`service`) or to cancel orders of (`production`),
## `site`, `repairing`, `can_repair`, `can_cancel`.
func service_state() -> Dictionary:
	var state := {"service": service_building >= 0, "production": production_building >= 0,
		"site": false, "repairing": false, "can_repair": false, "can_cancel": false}
	if production_building >= 0:
		var queue: PackedInt32Array = base_view.structures[production_building].queue
		state.can_cancel = not queue.is_empty()
	if service_building < 0:
		return state
	var s: Dictionary = base_view.structures.get(service_building, {})
	state.site = float(s.get("built", 1.0)) < 1.0
	var hp := 0
	var max_hp := 1
	if sim_host.unit_ids().find(service_building) >= 0:
		var info := sim_host.unit_info(service_building)
		hp = int(info.get("hp", 0))
		max_hp = maxi(int(info.get("max_hp", 1)), 1)
	state.repairing = sim_host.building_repairing(service_building)
	state.can_repair = not state.site and (hp < max_hp or state.repairing)
	return state


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
		_hint.text = tr("HUD_CANNOT_BUILD") % Picker.blocked_text(reason)
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
	_hint.text = tr("HUD_PLACE_HINT") if placement_reason.is_empty() else Picker.blocked_text(placement_reason)


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
	# Not over a panel: a right click on a tile asks for its unit card
	# (prompt 066).
	if button != null and button.pressed and button.button_index == MOUSE_BUTTON_RIGHT \
			and not button.ctrl_pressed and not button.meta_pressed and not _over_panel(button.position):
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
	# Own construction sites per type, and the progress of the furthest.
	var sites := {}
	var site_progress := {}
	var me := sim_host.local_player()
	for s: Dictionary in base_view.structures.values():
		if int(s.owner) == me and float(s.built) < 1.0:
			var t := int(s.type)
			sites[t] = int(sites.get(t, 0)) + 1
			site_progress[t] = maxf(float(site_progress.get(t, 0.0)), float(s.built))
	build_picker.set_options(tr("HUD_CMD_BUILD"), sim_host.build_options(), sites, site_progress)
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
	# The open build menu takes the place of the production bar.
	_production_panel.visible = production_building >= 0 and not _build_panel.visible
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
		names = [tr("HUD_UNDER_CONSTRUCTION") % int(float(s.built) * 100.0)]
	_queue_label.text = tr("HUD_QUEUE") % (", ".join(names) if not names.is_empty() else tr("HUD_QUEUE_EMPTY"))
