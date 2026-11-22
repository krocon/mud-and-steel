extends VBoxContainer
## Riders of one own vehicle in the unit panel (prompt 109): one tile per
## squad aboard and per towed gun, with picture, short name, strength and
## wounded men. Presentation only; the orders are the existing
## `Disembark` / `Unlimber` with single unit ids.
##
## - Click on a tile: exactly this squad gets off (the gun is unhitched)
##   where everybody gets off with J. Once it stands outside, it alone is
##   selected and the vehicle leaves the selection.
## - Shift + click: this one gets off too; the selection switches once all
##   collected ones stand outside.
## - The pending switch is dropped when the selection changes otherwise.
## - J (`transport_orders.gd`) still unloads everybody.
## - Shown for the single selected unit, or with several selected for the
##   active one (prompt 094); never for vehicles of other players.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const TILE_SIZE := Vector2(108, 40)
const COLUMNS := 4

var main: Node3D
var sim_host: SimHost
## Vehicle whose riders are shown, -1 without one.
var vehicle := -1
## Unit ids of the tiles (self-test).
var rider_ids: Array[int] = []
## Riders ordered off by a tile click whose selection is still pending.
var leaving: Array[int] = []
var _leaving_from: Array[int] = []
var _heading: Label
var _grid: GridContainer


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "RiderTiles"
	add_theme_constant_override(&"separation", 2)
	_heading = Label.new()
	_heading.add_theme_color_override(&"font_color", HudTheme.TEXT_DIM)
	_heading.clip_text = true
	add_child(_heading)
	_grid = GridContainer.new()
	_grid.columns = COLUMNS
	_grid.add_theme_constant_override(&"h_separation", 3)
	_grid.add_theme_constant_override(&"v_separation", 3)
	add_child(_grid)
	visible = false
	apply_scale()


func apply_scale() -> void:
	_heading.add_theme_font_size_override(&"font_size", HudTheme.font(11))
	for tile: Control in _grid.get_children():
		tile.custom_minimum_size = (TILE_SIZE * HudTheme.scale).round()


## Shows the riders of `id` (-1: none). Called by the unit panel on every
## refresh.
func show_for(id: int) -> void:
	var riders := _riders_of(id)
	vehicle = id if not riders.is_empty() else -1
	visible = vehicle >= 0
	if riders != rider_ids:
		_build(riders)
	if visible:
		_heading.text = tr("HUD_RIDERS") % str(sim_host.unit_info(id).get("name", ""))
		_update()


## Every frame: switches the selection to the riders ordered off once all
## of them stand outside.
func follow_leaving() -> void:
	if leaving.is_empty():
		return
	var selection: Control = main.selection
	if selection.selected != _leaving_from:
		leaving.clear()
		return
	var ids := sim_host.unit_ids()
	var carriers := sim_host.unit_carriers()
	var alive := sim_host.unit_alive()
	var out: Array[int] = []
	for id: int in leaving:
		var i := ids.find(id)
		if i < 0 or alive[i] == 0:
			continue
		if carriers[i] >= 0:
			return
		out.append(id)
	leaving.clear()
	if not out.is_empty():
		selection.set_selection(out)


## Click on the tile of `id` (also used by the self-test).
func order_off(id: int, shift: bool) -> void:
	if sim_host.is_towed_gun(id):
		sim_host.unlimber_units(PackedInt64Array([id]))
	else:
		sim_host.disembark_units(PackedInt64Array([id]))
	if not shift:
		leaving.clear()
	if not leaving.has(id):
		leaving.append(id)
	_leaving_from = main.selection.selected.duplicate()


## Living units riding in or towed by own vehicle `id`: squads first, then
## guns, each by id.
func _riders_of(id: int) -> Array[int]:
	var result: Array[int] = []
	if id < 0:
		return result
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var index := ids.find(id)
	if index < 0 or owners[index] != sim_host.local_player():
		return result
	var carriers := sim_host.unit_carriers()
	var alive := sim_host.unit_alive()
	var stances := sim_host.unit_stances()
	var guns: Array[int] = []
	for i in carriers.size():
		if carriers[i] == id and alive[i] != 0:
			if stances[i] >= 0:
				result.append(ids[i])
			else:
				guns.append(ids[i])
	result.append_array(guns)
	return result


func _build(riders: Array[int]) -> void:
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	rider_ids = riders.duplicate()
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	var key := Settings.action_key_name("order_disembark")
	for id: int in riders:
		var index := ids.find(id)
		var type_id: String = types[index] if index >= 0 else ""
		var tile := RiderTile.new()
		tile.custom_minimum_size = (TILE_SIZE * HudTheme.scale).round()
		tile.unit = id
		tile.gun = sim_host.is_towed_gun(id)
		tile.label = _short_name(type_id)
		tile.picture = Thumbnail.texture(type_id)
		tile.backdrop = Thumbnail.backdrop(type_id)
		tile.action_tip = str(sim_host.unit_info(id).get("name", "")) + "\n" \
				+ (tr("HUD_TIP_RIDER_GUN") if tile.gun else tr("HUD_TIP_RIDER") % key)
		tile.tooltip_text = tile.action_tip
		tile.pressed_with.connect(order_off)
		_grid.add_child(tile)


## Strength, wounded men and loading state of the tiles.
func _update() -> void:
	var ids := sim_host.unit_ids()
	var health := sim_host.unit_health()
	var loading := sim_host.unit_loading()
	var wounded: Dictionary = sim_host.unit_wounded_counts()
	for tile: RiderTile in _grid.get_children():
		var index := ids.find(tile.unit)
		if index < 0:
			continue
		var info := sim_host.unit_info(tile.unit)
		var strength := ""
		if info.has("soldiers"):
			strength = "%d/%d" % [info.soldiers, info.squad_size]
		var hurt: Vector2i = wounded.get(tile.unit, Vector2i.ZERO)
		var leaving_now := loading[index] != 0 or leaving.has(tile.unit)
		if strength != tile.strength or hurt != tile.wounded or leaving_now != tile.leaving \
				or not is_equal_approx(health[index], tile.health):
			tile.strength = strength
			tile.wounded = hurt
			tile.leaving = leaving_now
			tile.health = health[index]
			tile.tooltip_text = tile.action_tip
			if hurt != Vector2i.ZERO:
				tile.tooltip_text += "\n" + tr("HUD_WOUNDED") % [hurt.x, hurt.y]
			tile.queue_redraw()


## `de_inf_mg42_team` -> `MG42`: the catalog id without faction, `inf`
## and `team` / `squad`, short enough for a tile (the full name is in the
## tooltip).
static func _short_name(type_id: String) -> String:
	var parts := type_id.split("_")
	if parts.size() > 1:
		parts.remove_at(0)
	if parts.size() > 1 and parts[0] == "inf":
		parts.remove_at(0)
	if parts.size() > 1 and parts[parts.size() - 1] in ["team", "squad"]:
		parts.remove_at(parts.size() - 1)
	return " ".join(parts).to_upper().left(14)


## One rider: picture on its faction backdrop at the left, short name,
## soldiers and wounded men at the right, a health bar at the bottom. Dim
## while it is getting off.
class RiderTile:
	extends Control

	signal pressed_with(id: int, shift: bool)

	var unit := -1
	var gun := false
	var label := ""
	var picture: Texture2D
	var backdrop := Color.BLACK
	var strength := ""
	## Lightly / heavily wounded (untreated) men.
	var wounded := Vector2i.ZERO
	var health := 1.0
	var leaving := false
	## What a click does (the tooltip, plus wounded men).
	var action_tip := ""
	var _hover := false

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_entered.connect(func() -> void:
			_hover = true
			queue_redraw())
		mouse_exited.connect(func() -> void:
			_hover = false
			queue_redraw())

	func _gui_input(event: InputEvent) -> void:
		var button := event as InputEventMouseButton
		if button != null and button.pressed and button.button_index == MOUSE_BUTTON_LEFT:
			pressed_with.emit(unit, button.shift_pressed)
			accept_event()

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		draw_style_box(HudTheme.plate(HudTheme.GHOST_HOVER if _hover else HudTheme.GHOST), rect)
		var side := size.y - 6.0
		var art := Rect2(3.0, 3.0, side * 1.25, side)
		draw_rect(art, Color(backdrop, 0.6))
		if picture != null:
			draw_texture_rect(picture, art, false)
		var x := art.end.x + 4.0
		var room := size.x - x - 3.0
		var font_size := HudTheme.font(10)
		var text_color := HudTheme.TEXT_DIM if leaving else HudTheme.TEXT
		draw_string(ThemeDB.fallback_font, Vector2(x, 3.0 + font_size), label, HORIZONTAL_ALIGNMENT_LEFT, room, font_size, text_color)
		var line := strength
		if leaving:
			line = tr("HUD_RIDER_LEAVING")
		draw_string(ThemeDB.fallback_font, Vector2(x, 5.0 + font_size * 2.0), line, HORIZONTAL_ALIGNMENT_LEFT, room, font_size, text_color)
		if wounded != Vector2i.ZERO and not leaving:
			var mark := "%d" % (wounded.x + wounded.y)
			var at := Vector2(size.x - 5.0 - font_size * 0.7 * mark.length(), 5.0 + font_size * 2.0)
			draw_string(ThemeDB.fallback_font, at, mark, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.95, 0.6, 0.2))
		var bar := Rect2(x, size.y - 7.0, room, 4.0)
		draw_rect(bar, Color(0, 0, 0, 0.7))
		var color := Color(0.3, 0.9, 0.3) if health > 0.5 else (Color(0.95, 0.75, 0.2) if health > 0.25 else Color(0.95, 0.3, 0.2))
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * health, bar.size.y)), Color(color, 0.5 if leaving else 1.0))
		if leaving:
			draw_rect(rect, Color(0, 0, 0, 0.35))
