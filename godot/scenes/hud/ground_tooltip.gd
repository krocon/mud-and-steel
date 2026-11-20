extends PanelContainer
## Ground tooltip (prompt 100): when the mouse rests on the map for
## `REST_SECONDS`, a small box beside the pointer names the ground or the
## map object under it and says what it does:
##
## - movement, separately for vehicles and infantry (or for the running gear
##   of the selected type when all selected units share one): a word and the
##   percent of the cross-country speed, with the soft ground of the
##   weather; "road speed" on roads; impassable;
## - the slope when it slows uphill movement noticeably;
## - concealment (the vision cell: woods, solid obstacles, smoke);
## - cover (an object that stops bullets, soft cover of an object, a dug
##   position, a crater), kept apart from concealment.
##
## Every value comes from `SimHost.ground_info`, which reads the rules
## (`data/terrain.toml`, `weather.toml`, `vision.toml`, `infantry.toml`,
## `craters.toml`); nothing is a hand-kept text table. Field works count
## only where the player sees them.
##
## The box does not show while a box selection or a right-drag runs, the
## camera is dragged, a building is placed, or the pointer lies on a HUD
## panel; it never moves while the pointer rests, so it cannot flicker at a
## terrain border. Setting `ground_tooltip` (`settings.gd`): `on` (default),
## `key` (only while `ground_tooltip` is held, then at once) or `off`.

const Settings := preload("res://scenes/menu/settings.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")

## Seconds the pointer must rest before the box shows.
const REST_SECONDS := 0.4
## Pointer movement (pixels) that counts as moving.
const MOVE_PX := 3.0
## Offset of the box from the pointer.
const OFFSET := Vector2(20, 22)
## Speed words: from these percent of the cross-country speed on.
const FAST_PCT := 110
const NORMAL_PCT := 85
const SLOW_PCT := 50
## A slope factor below this (percent) is mentioned.
const SLOPE_NOTE_PCT := 90
## Cover in percent from which soft cover counts as heavy.
const HEAVY_COVER_PCT := 50

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## What the box shows (self-test): the title and the lines.
var title := ""
var lines: PackedStringArray = []
## Ground lookups so far (self-test: one per rest).
var lookups := 0
## Point to look at instead of the pointer (self-test; null: the pointer).
var forced_point: Variant = null

var _title: Label
var _body: Label
var _rest := 0.0
var _last_mouse := Vector2(-1, -1)
var _shown_at := Vector2(-1e9, -1e9)


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "GroundTooltip"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	add_theme_stylebox_override(&"panel", HudTheme.panel_box())
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_theme_constant_override(&"separation", 1)
	add_child(box)
	_title = Label.new()
	_title.add_theme_color_override(&"font_color", HudTheme.GOLD)
	_title.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	box.add_child(_title)
	_body = Label.new()
	_body.add_theme_color_override(&"font_color", HudTheme.TEXT)
	_body.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	box.add_child(_body)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_update(delta)
	usec += Time.get_ticks_usec() - began


func _update(delta: float) -> void:
	var mode: String = Settings.get_value("ground_tooltip")
	var key_held := InputMap.has_action("ground_tooltip") and Input.is_action_pressed("ground_tooltip")
	if mode == "off" or (mode == "key" and not key_held) or _busy():
		_hide()
		return
	var mouse := get_viewport().get_mouse_position()
	if mouse.distance_to(_last_mouse) > MOVE_PX:
		_last_mouse = mouse
		_rest = 0.0
		_hide()
		return
	_rest += delta
	if _rest < (0.0 if mode == "key" else REST_SECONDS) or _shown_at == mouse:
		return
	_shown_at = mouse
	var point: Variant = forced_point if forced_point != null else GroundPick.from_screen(main.camera, mouse)
	if point == null:
		return
	show_at(point as Vector3, mouse)


## Fills the box for the ground at `point` and places it beside `screen`.
func show_at(point: Vector3, screen: Vector2) -> void:
	var info := sim_host.ground_info(point.x, point.z)
	lookups += 1
	if info.is_empty():
		_hide()
		return
	_fill(info)
	_title.add_theme_font_size_override(&"font_size", HudTheme.font(15))
	_body.add_theme_font_size_override(&"font_size", HudTheme.font(13))
	_title.text = title
	_body.text = "\n".join(lines)
	visible = true
	reset_size()
	_place(screen)


func _hide() -> void:
	if visible:
		visible = false
	_shown_at = Vector2(-1e9, -1e9)


## Whether the player is busy with something the box would get in the way of.
func _busy() -> bool:
	var selection: Control = main.selection
	if selection._dragging or selection._right_pressing:
		return true
	var rig: Node = main.camera_rig
	if rig != null and rig.get("_dragging") == true:
		return true
	var base_hud: Node = main.get("base_hud")
	var placing: Variant = base_hud.get("placing") if base_hud != null else null
	if placing != null and int(placing) >= 0:
		return true
	if forced_point != null:
		return false
	var mouse := get_viewport().get_mouse_position()
	var hud: Node = main.get("hud")
	if hud != null and hud.covers(mouse):
		return true
	return get_viewport().gui_get_hovered_control() != null


func _place(screen: Vector2) -> void:
	var view := get_viewport_rect().size
	var at := screen + OFFSET
	if at.x + size.x > view.x - 8.0:
		at.x = screen.x - OFFSET.x - size.x
	if at.y + size.y > view.y - 8.0:
		at.y = screen.y - OFFSET.y - size.y
	position = at.clamp(Vector2(8, 8), view - size - Vector2(8, 8))


## Title and lines from a `ground_info` dictionary.
func _fill(info: Dictionary) -> void:
	lines = PackedStringArray()
	var terrain := _terrain_name(info)
	var object: String = info.get("object", "")
	var work: String = info.get("work", "")
	var sight: String = info.get("sight", "clear")
	if not work.is_empty():
		title = tr("HUD_WORK_" + work.to_upper())
	elif not object.is_empty():
		title = _object_name(object)
		if info.get("ruin", false):
			title = tr("GROUND_RUIN") % title
	else:
		title = terrain
	if sight == "foliage" and object.begins_with("env_tree"):
		title = tr("GROUND_WOODS")
	if title != terrain:
		lines.append(terrain)
	_movement(info)
	var slope: int = info.get("slope_pct", 100)
	var grade: int = info.get("grade_pct", 0)
	if slope < 0:
		lines.append(tr("GROUND_SLOPE_STEEP") % grade)
	elif slope < SLOPE_NOTE_PCT:
		lines.append(tr("GROUND_SLOPE") % [grade, slope])
	lines.append(tr("GROUND_SIGHT") % _sight_word(info))
	lines.append(tr("GROUND_COVER") % _cover_word(info))
	if info.get("building", false):
		lines.append(tr("GROUND_OCCUPY"))


func _terrain_name(info: Dictionary) -> String:
	var key := "GROUND_T_" + str(info.get("terrain_name", "")).to_upper()
	var text := tr(key)
	return text if text != key else str(info.get("terrain_name", ""))


func _object_name(id: String) -> String:
	var key := "GROUND_O_" + id.to_upper()
	var text := tr(key)
	return text if text != key else id


## One line per running gear that matters: the gear of the selected units
## when they share one, else vehicles (tracks; wheels when they differ)
## and infantry.
func _movement(info: Dictionary) -> void:
	var gears: Dictionary = info.get("gears", {})
	var gear := _selected_gear()
	if not gear.is_empty():
		lines.append(tr("GROUND_MOVE") % [tr("GROUND_GEAR_" + gear.to_upper()), _speed_word(info, gear)])
		return
	var tracked := _speed_word(info, "tracked")
	var wheeled := _speed_word(info, "wheeled")
	var vehicles := tracked
	if wheeled != tracked and gears.has("wheeled"):
		vehicles = "%s, %s: %s" % [tracked, tr("GROUND_GEAR_WHEELED"), wheeled]  # l10n-ignore
	lines.append(tr("GROUND_MOVE") % [tr("GROUND_VEHICLES"), vehicles])
	lines.append(tr("GROUND_MOVE") % [tr("GROUND_INFANTRY"), _speed_word(info, "foot")])


## Running gear shared by every selected unit, or "".
func _selected_gear() -> String:
	var shared := ""
	for id: int in main.selection.selected:
		var gear := sim_host.unit_gear(id)
		if gear.is_empty() or (not shared.is_empty() and gear != shared):
			return ""
		shared = gear
	return shared


## Movement word for `gear` (`foot`: infantry, small size class).
func _speed_word(info: Dictionary, gear: String) -> String:
	var foot := gear == "foot"
	if not info.get("passable_small" if foot else "passable_large", false):
		return tr("GROUND_MOVE_NONE_FOOT" if foot else "GROUND_MOVE_NONE")
	var soft: int = (info.get("gears", {}) as Dictionary).get(gear, 100)
	var pct: int = int(info.get("speed_pct", 100)) * soft / 100
	if info.get("road", false) and not foot:
		return tr("GROUND_MOVE_ROAD") if soft >= 100 else tr("GROUND_MOVE_ROAD_SLOW") % soft
	if pct >= FAST_PCT:
		return tr("GROUND_MOVE_FAST") % pct
	if pct >= NORMAL_PCT:
		return tr("GROUND_MOVE_NORMAL") % pct
	if pct >= SLOW_PCT:
		return tr("GROUND_MOVE_SLOW") % pct
	return tr("GROUND_MOVE_VERY_SLOW") % pct


## Concealment: smoke and solid obstacles and woods hide what is behind
## them (vision grid); open ground hides nothing.
func _sight_word(info: Dictionary) -> String:
	if info.get("smoke", false):
		return tr("GROUND_SIGHT_SMOKE")
	match str(info.get("sight", "clear")):
		"solid", "foliage":
			return tr("GROUND_STRONG")
	return tr("GROUND_NONE")


## Cover: the strongest of the object (stops bullets, or soft cover), a dug
## position and a crater.
func _cover_word(info: Dictionary) -> String:
	if info.get("hard_cover", false):
		return tr("GROUND_COVER_HARD")
	var soft: int = info.get("object_cover_pct", 0)
	var ground: int = info.get("ground_cover_pct", 0)
	if soft <= 0 and ground <= 0:
		return tr("GROUND_COVER_NONE")
	var level := tr("GROUND_HEAVY") if maxi(soft, ground) >= HEAVY_COVER_PCT else tr("GROUND_LIGHT")
	if soft >= ground:
		return tr("GROUND_COVER_PCT") % [level, soft]
	if not str(info.get("work", "")).is_empty():
		return tr("GROUND_COVER_DUG") % [level, ground]
	return tr("GROUND_COVER_CRATER") % [level, ground]
