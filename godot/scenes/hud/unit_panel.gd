extends PanelContainer
## Unit panel (prompt 031), bottom center.
##
## - One unit: portrait (`unit_portrait.gd`), name, hit points, armor
##   (hull and turret front / side / rear), weapons with caliber and range,
##   soldiers of a squad, stance and fire mode.
## - Several units: a grid of tiles with a short name and a health bar;
##   click selects that unit alone, Shift + click removes it.
## - Hidden without a selection.

const UnitPortrait := preload("res://scenes/hud/unit_portrait.gd")
const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const REFRESH := 0.2
## Tiles shown at most; the rest is counted.
const MAX_TILES := 40
const TILE_SIZE := Vector2(74, 40)
const STANCES := {-1: "", 0: "standing", 1: "kneeling", 2: "prone"}

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Unit ids of the tiles (self-test).
var tile_ids: Array[int] = []

var _portraits: UnitPortrait
var _single: HBoxContainer
var _portrait: TextureRect
var _title: Label
var _health: ProgressBar
var _details: Label
var _grid: GridContainer
var _more: Label
var _shown: Array[int] = []
var _refresh_left := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "UnitPanel"
	set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	grow_horizontal = Control.GROW_DIRECTION_BOTH
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	offset_left = -250
	offset_right = 250
	offset_bottom = -8
	offset_top = -8
	custom_minimum_size = Vector2(500, 0)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_portraits = UnitPortrait.new()
	_portraits.name = "Portraits"
	add_child(_portraits)
	var box := VBoxContainer.new()
	add_child(box)

	_single = HBoxContainer.new()
	box.add_child(_single)
	_portrait = TextureRect.new()
	_portrait.custom_minimum_size = Vector2(UnitPortrait.SIZE)
	_portrait.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_portrait.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_single.add_child(_portrait)
	var text := VBoxContainer.new()
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_single.add_child(text)
	_title = Label.new()
	text.add_child(_title)
	_health = ProgressBar.new()
	_health.custom_minimum_size = Vector2(0, 14)
	_health.show_percentage = false
	text.add_child(_health)
	_details = Label.new()
	_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_details.add_theme_font_size_override(&"font_size", 13)
	text.add_child(_details)

	_grid = GridContainer.new()
	_grid.columns = 6
	box.add_child(_grid)
	_more = Label.new()
	box.add_child(_more)
	visible = false


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	var selected: Array[int] = main.selection.selected
	if selected != _shown or _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh(selected)
	usec += Time.get_ticks_usec() - began


func _refresh(selected: Array[int]) -> void:
	var changed := selected != _shown
	_shown = selected.duplicate()
	visible = not selected.is_empty()
	if selected.is_empty():
		return
	if selected.size() == 1:
		_single.visible = true
		_grid.visible = false
		_more.visible = false
		tile_ids.clear()
		_show_single(selected[0])
		return
	_single.visible = false
	_grid.visible = true
	if changed:
		_build_tiles(selected)
	_update_tiles()


func _show_single(id: int) -> void:
	var info := sim_host.unit_info(id)
	if info.is_empty():
		return
	var type_id: String = info.type_id
	_portrait.texture = _portraits.texture_for(type_id, func() -> Node3D:
		var unit := UNIT_SCENE.instantiate() as Node3D
		main._use_catalog_model(unit, type_id, 0)
		var ring := unit.get_node_or_null("SelectionRing") as Node3D
		if ring != null:
			ring.visible = false
		return unit)
	_title.text = info.name
	_health.max_value = maxi(int(info.max_hp), 1)
	_health.value = int(info.hp)
	var lines: Array[String] = []
	var hp_line := "Hit points %d / %d" % [info.hp, info.max_hp]
	if info.has("soldiers"):
		hp_line += "   ·   soldiers %d / %d" % [info.soldiers, info.squad_size]
	lines.append(hp_line)
	var hull: PackedInt64Array = info.armor_hull
	var turret: PackedInt64Array = info.armor_turret
	if hull.size() == 3 and hull[0] > 0:
		var armor := "Armor hull %d / %d / %d mm" % [hull[0], hull[1], hull[2]]
		if turret.size() == 3:
			armor += ", turret %d / %d / %d mm" % [turret[0], turret[1], turret[2]]
		lines.append(armor + " (front / side / rear)")
	var weapons: Array[String] = []
	for w: Dictionary in info.weapons:
		var count := "%d× " % w.count if int(w.count) > 1 else ""
		var caliber: float = w.caliber_mm
		var mm := "%d" % roundi(caliber) if is_equal_approx(caliber, roundf(caliber)) else "%.1f" % caliber
		weapons.append("%s%s (%s mm, %d m)" % [count, w.name, mm, int(w.range_m)])
	lines.append("Weapons: " + (", ".join(weapons) if not weapons.is_empty() else "none"))
	var index := sim_host.unit_ids().find(id)
	if index >= 0:
		var state: Array[String] = []
		var stance: int = sim_host.unit_stances()[index]
		if stance >= 0:
			state.append("Stance %s" % STANCES.get(stance, "?"))
		state.append("holding fire" if sim_host.unit_hold_fire()[index] != 0 else "fire at will")
		var queued := sim_host.unit_queue_length(id)
		if sim_host.unit_patrols(id):
			state.append("patrolling")
		elif queued > 0:
			state.append("%d waypoint%s queued" % [queued, "" if queued == 1 else "s"])
		lines.append(", ".join(state))
	_details.text = "\n".join(lines)


func _build_tiles(selected: Array[int]) -> void:
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	tile_ids.clear()
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	for id: int in selected.slice(0, MAX_TILES):
		var index := ids.find(id)
		var tile := Tile.new()
		tile.custom_minimum_size = TILE_SIZE
		tile.unit = id
		tile.label = _short_name(types[index] if index >= 0 else "")
		tile.pressed_with.connect(_on_tile)
		_grid.add_child(tile)
		tile_ids.append(id)
	_more.visible = selected.size() > MAX_TILES
	_more.text = "+%d more" % (selected.size() - MAX_TILES)


func _update_tiles() -> void:
	var ids := sim_host.unit_ids()
	var health := sim_host.unit_health()
	for tile: Tile in _grid.get_children():
		var index := ids.find(tile.unit)
		var value := health[index] if index >= 0 else 0.0
		if not is_equal_approx(value, tile.health):
			tile.health = value
			tile.queue_redraw()


## Click selects only that unit, Shift + click removes it (also used by
## the self-test).
func _on_tile(id: int, shift: bool) -> void:
	var selection: Control = main.selection
	if shift:
		selection.set_selection(selection.selected.filter(func(other: int) -> bool: return other != id))
	else:
		selection.set_selection([id])


## `de_pz4_h` -> `PZ4 H`: the catalog id without faction, short enough for
## a tile.
static func _short_name(type_id: String) -> String:
	var parts := type_id.split("_")
	if parts.size() > 1:
		parts.remove_at(0)
	return " ".join(parts).to_upper().left(12)


## One unit of a multiple selection.
class Tile:
	extends Control

	signal pressed_with(id: int, shift: bool)

	var unit := -1
	var label := ""
	var health := 1.0

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		tooltip_text = "Click: select only this unit. Shift + click: remove it."

	func _gui_input(event: InputEvent) -> void:
		var button := event as InputEventMouseButton
		if button != null and button.pressed and button.button_index == MOUSE_BUTTON_LEFT:
			pressed_with.emit(unit, button.shift_pressed)
			accept_event()

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		draw_rect(rect, Color(0.12, 0.13, 0.11, 0.9))
		draw_rect(rect, Color(0.5, 0.55, 0.45, 0.8), false, 1.0)
		draw_string(ThemeDB.fallback_font, Vector2(4, 17), label, HORIZONTAL_ALIGNMENT_LEFT, size.x - 8, 12)
		var bar := Rect2(4, size.y - 10, size.x - 8, 5)
		draw_rect(bar, Color(0, 0, 0, 0.7))
		var color := Color(0.3, 0.9, 0.3) if health > 0.5 else (Color(0.95, 0.75, 0.2) if health > 0.25 else Color(0.95, 0.3, 0.2))
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * health, bar.size.y)), color)
