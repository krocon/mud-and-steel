extends VBoxContainer
## Picture tiles of the build menu and the production bar (prompt 058,
## [provisional] layout and keys).
##
## Data-driven: it shows whatever `SimHost.build_options()` or
## `production_options()` returns, so new buildables (pioneer obstacles,
## supply trucks, ...) appear without changes here. Each option becomes a
## tile with its pre-rendered picture (`thumbnail.gd`) on a faction-tinted
## backdrop, name, costs (fuel, ammunition, manpower), build time, hotkey,
## a count of queued orders or construction sites and the progress of the
## first as a bar over the picture. Unavailable tiles are dimmed with the
## reason. The tooltip lists catalog values (`SimHost.catalog_card`).
##
## Tabs group the tiles by kind: base buildings, other placeable things
## (defense), then by catalog group: infantry, vehicles, guns, aircraft.
## Empty tabs are hidden. Alt + 1…9, 0 picks the tiles of the open tab
## (plain digits are control groups, Shift + digits air support).
##
## Presentation only: `picked` hands the option to `base_hud.gd`, which
## sends the order through `SimHost` (the simulation checks it again).

signal picked(option: Dictionary)

const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
## Pixel sizes at UI scale 1 (prompt 043 will scale the whole HUD).
const TILE := Vector2(116, 124)
const PICTURE_HEIGHT := 77.0
const COLUMNS := 4
const VISIBLE_ROWS := 2
const GAP := 4
## Tab id, label (translation key); in this order.
const TABS: Array[Array] = [
	["base", "HUD_TAB_BASE"],
	["defense", "HUD_TAB_DEFENSE"],
	["infantry", "HUD_TAB_INFANTRY"],
	["vehicles", "HUD_TAB_VEHICLES"],
	["guns", "HUD_TAB_GUNS"],
	["air", "HUD_TAB_AIR"],
	["other", "HUD_TAB_OTHER"],
]
const HOTKEYS: Array[Key] = [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0]
const BLOCKED_SHADE := Color(0.05, 0.05, 0.05, 0.62)
const PROGRESS_COLOR := MenuTheme.ACCENT

## Open tab id.
var tab := ""
## Tile per unit type of the open tab (self-test).
var tiles: Dictionary[int, Button] = {}

var _title: Label
var _tab_bar: HBoxContainer
var _tab_buttons: Dictionary[String, Button] = {}
var _scroll: ScrollContainer
var _grid: GridContainer
var _options: Array = []
var _signature := ""
var _counts := {}
var _progress := {}
## Catalog cards per id, read once per run.
static var _cards: Dictionary[String, Dictionary] = {}


func _init() -> void:
	add_theme_constant_override(&"separation", GAP)
	_title = Label.new()
	_title.add_theme_color_override(&"font_color", MenuTheme.ACCENT)
	_title.add_theme_font_size_override(&"font_size", 14)
	add_child(_title)
	_tab_bar = HBoxContainer.new()
	_tab_bar.add_theme_constant_override(&"separation", 2)
	add_child(_tab_bar)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(_scroll)
	_grid = GridContainer.new()
	_grid.columns = COLUMNS
	_grid.add_theme_constant_override(&"h_separation", GAP)
	_grid.add_theme_constant_override(&"v_separation", GAP)
	_scroll.add_child(_grid)


## Shows `options` (dictionaries of `SimHost.build_options`) under `title`.
## `counts` maps a unit type to its queued orders or sites, `progress` to
## the progress (0..1) of the first of them.
func set_options(title: String, options: Array, counts: Dictionary, progress: Dictionary) -> void:
	_title.text = title
	var ids: Array[String] = []
	for o: Dictionary in options:
		ids.append(str(o.id))
	var signature := title + ",".join(ids)
	_options = options
	_counts = counts
	_progress = progress
	if signature != _signature:
		_signature = signature
		_rebuild_tabs()
	# Rebuilt only when the tab or the list changed, so a click is never
	# lost; costs, blocks, counts and progress update in place.
	_update_tiles()


func _update_tiles() -> void:
	for o: Dictionary in _options:
		var tile: Button = tiles.get(int(o.type))
		if tile != null:
			_update_tile(tile, o, int(_counts.get(int(o.type), 0)), float(_progress.get(int(o.type), -1.0)))


## Opens tab `id` (also used by the self-test).
func open_tab(id: String) -> void:
	tab = id
	for key in _tab_buttons:
		_tab_buttons[key].set_pressed_no_signal(key == id)
	_rebuild_tiles()


## The tab ids that have tiles, in order.
func tab_ids() -> Array[String]:
	var present: Array[String] = []
	for spec in TABS:
		for o: Dictionary in _options:
			if tab_of(o) == spec[0]:
				present.append(spec[0])
				break
	return present


## Presses the tile of the open tab with hotkey number `index` (0 = key 1);
## returns whether there was an enabled one.
func press_index(index: int) -> bool:
	var shown := _tab_options()
	if index < 0 or index >= shown.size():
		return false
	return press_type(int(shown[index].type))


## Presses the tile of `unit_type` like a click; false if it is not shown
## or blocked.
func press_type(unit_type: int) -> bool:
	var tile: Button = tiles.get(unit_type)
	if tile == null or tile.disabled:
		return false
	tile.pressed.emit()
	return true


## Hotkey number of a key event (Alt + 1…9, 0), or -1.
static func hotkey_index(key: InputEventKey) -> int:
	if key == null or not key.pressed or key.echo or not key.alt_pressed \
			or key.ctrl_pressed or key.meta_pressed or key.shift_pressed:
		return -1
	return HOTKEYS.find(key.physical_keycode if key.physical_keycode != KEY_NONE else key.keycode)


## Tab of an option: base buildings, other structures, else by catalog group.
static func tab_of(option: Dictionary) -> String:
	var card := card_of(str(option.id))
	var group: String = card.get("group", "")
	if group == "building":
		return "base"
	if option.get("structure", false):
		return "defense"
	match group:
		"infantry":
			return "infantry"
		"vehicle":
			return "vehicles"
		"gun":
			return "guns"
		"aircraft":
			return "air"
	return "other"


## Why an option is blocked, in the interface language: `blocked` is a
## code of the simulation ("not enough resources", "too steep", ...).
static func blocked_text(blocked: String) -> String:
	return Language.code_text("HUD_BLOCKED_", blocked.replace(" ", "_"), blocked)


static func card_of(id: String) -> Dictionary:
	if not _cards.has(id):
		_cards[id] = SimHost.catalog_card(id)
	return _cards[id]


func _tab_options() -> Array:
	return _options.filter(func(o: Dictionary) -> bool: return tab_of(o) == tab)


func _rebuild_tabs() -> void:
	for child in _tab_bar.get_children():
		_tab_bar.remove_child(child)
		child.queue_free()
	_tab_buttons.clear()
	var present := tab_ids()
	for spec in TABS:
		if not present.has(spec[0]):
			continue
		var b := Button.new()
		b.name = "Tab_" + spec[0]
		b.text = spec[1]
		b.toggle_mode = true
		b.focus_mode = Control.FOCUS_NONE
		b.add_theme_font_size_override(&"font_size", 13)
		b.pressed.connect(open_tab.bind(spec[0]))
		_tab_bar.add_child(b)
		_tab_buttons[spec[0]] = b
	_tab_bar.visible = present.size() > 1
	open_tab(tab if present.has(tab) else (present[0] if not present.is_empty() else ""))


func _rebuild_tiles() -> void:
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	tiles.clear()
	var shown := _tab_options()
	for i in shown.size():
		var tile := _make_tile(shown[i], i)
		_grid.add_child(tile)
		tiles[int(shown[i].type)] = tile
	_update_tiles()
	var rows := ceili(float(shown.size()) / COLUMNS)
	_scroll.custom_minimum_size = Vector2(
			COLUMNS * TILE.x + (COLUMNS - 1) * GAP,
			mini(rows, VISIBLE_ROWS) * (TILE.y + GAP) - GAP)


func _make_tile(option: Dictionary, index: int) -> Button:
	var id := str(option.id)
	var tile := Button.new()
	tile.name = "Tile_" + id
	tile.custom_minimum_size = TILE
	tile.focus_mode = Control.FOCUS_NONE
	tile.clip_contents = true
	var picture_rect := Rect2(3, 3, TILE.x - 6, PICTURE_HEIGHT)
	var backdrop := ColorRect.new()
	backdrop.color = Thumbnail.backdrop(id)
	_place(tile, backdrop, picture_rect)
	var texture := Thumbnail.texture(id)
	if texture != null:
		var picture := TextureRect.new()
		picture.texture = texture
		picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		_place(tile, picture, picture_rect)
	else:
		var missing := _label(str(option.name), 11, MenuTheme.TEXT_DIM)
		missing.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		missing.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		missing.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		_place(tile, missing, picture_rect)
	var track := ColorRect.new()
	track.name = "Track"
	track.color = Color(0.0, 0.0, 0.0, 0.6)
	_place(tile, track, Rect2(picture_rect.position.x, picture_rect.end.y - 6, picture_rect.size.x, 6))
	var progress := ColorRect.new()
	progress.name = "Progress"
	progress.color = PROGRESS_COLOR
	_place(tile, progress, Rect2(picture_rect.position.x, picture_rect.end.y - 6, 0, 6))
	var shade := ColorRect.new()
	shade.name = "Shade"
	shade.color = BLOCKED_SHADE
	_place(tile, shade, Rect2(Vector2.ZERO, TILE))
	if index < HOTKEYS.size():
		var key := _badge(OS.get_keycode_string(HOTKEYS[index]))
		key.name = "Key"
		_place(tile, key, Rect2(5, 5, 16, 16))
	var count := _badge("")
	count.name = "Count"
	count.add_theme_color_override(&"font_color", MenuTheme.ACCENT)
	_place(tile, count, Rect2(TILE.x - 31, 5, 26, 16))
	var time := _badge(tr("COMMON_SECONDS") % int(option.time))
	time.name = "Time"
	_place(tile, time, Rect2(TILE.x - 41, PICTURE_HEIGHT - 17, 36, 16))
	var name_label := _label(str(option.name), 12, MenuTheme.TEXT)
	name_label.name = "Name"
	name_label.clip_text = true
	name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_place(tile, name_label, Rect2(5, PICTURE_HEIGHT + 4, TILE.x - 10, 18))
	var cost := _label("", 11, MenuTheme.TEXT_DIM)
	cost.name = "Cost"
	_place(tile, cost, Rect2(5, PICTURE_HEIGHT + 22, TILE.x - 10, 16))
	var reason := _label("", 11, Color(0.95, 0.75, 0.6))
	reason.name = "Reason"
	reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	reason.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	reason.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_place(tile, reason, Rect2(4, 18, TILE.x - 8, PICTURE_HEIGHT - 34))
	tile.set_meta(&"index", index)
	var tile_option := option
	tile.pressed.connect(func() -> void: picked.emit(tile_option))
	return tile


func _update_tile(tile: Button, option: Dictionary, count: int, progress: float) -> void:
	var blocked := str(option.blocked)
	tile.disabled = not blocked.is_empty()
	(tile.get_node("Shade") as Control).visible = tile.disabled
	var reason := blocked_text(blocked) if tile.disabled else ""
	(tile.get_node("Reason") as Label).text = (reason[0].to_upper() + reason.substr(1)) if not reason.is_empty() else ""
	(tile.get_node("Cost") as Label).text = tr("HUD_COST_SHORT") % [option.fuel, option.ammo, option.manpower]
	var count_label := tile.get_node("Count") as Label
	count_label.text = "×%d" % count
	count_label.visible = count > 0
	var bar := tile.get_node("Progress") as Control
	bar.visible = progress >= 0.0 and count > 0
	(tile.get_node("Track") as Control).visible = bar.visible
	bar.size.x = (TILE.x - 6) * clampf(progress, 0.0, 1.0)
	var signature := "%s|%d|%d|%d|%s|%s" % [option.time, option.fuel, option.ammo, option.manpower, blocked, TranslationServer.get_locale()]
	if tile.get_meta(&"tooltip_for", "") != signature:
		tile.set_meta(&"tooltip_for", signature)
		tile.tooltip_text = _tooltip(option, int(tile.get_meta(&"index", -1)))


func _tooltip(option: Dictionary, index: int) -> String:
	var card := card_of(str(option.id))
	var lines: Array[String] = []
	var title := str(option.name)
	if not str(card.get("designation", "")).is_empty() and card.designation != title:
		title += "  (%s)" % card.designation
	lines.append(title)
	lines.append(tr("HUD_COST_LINE") % [option.fuel, option.ammo, option.manpower, int(option.time)])
	if card.get("group", "") == "building":
		lines.append(tr("HUD_BUILDING_STATS") % [card.hp, card.armor_mm])
		var income: PackedInt64Array = card.income
		if income.size() == 3 and (income[0] + income[1] + income[2]) > 0:
			lines.append(tr("HUD_INCOME") % [income[0], income[1], income[2]])
		if not str(card.produces).is_empty():
			lines.append(tr("HUD_PRODUCES") % Language.code_text("HUD_PRODUCES_", str(card.produces)))
		var requires: PackedStringArray = card.requires
		if not requires.is_empty():
			lines.append(tr("HUD_REQUIRES") % ", ".join(requires))
	elif not card.is_empty():
		var men := tr("HUD_SQUAD_OF") % card.squad_size if int(card.squad_size) > 0 else tr("HUD_CREW") % card.crew
		var body: Array[String] = [men]
		if float(card.mass_t) > 0.0:
			body.append("%.1f t" % card.mass_t)
		if float(card.speed_road_kmh) > 0.0:
			body.append(tr("HUD_ROAD_SPEED") % roundi(card.speed_road_kmh))
		if int(card.passengers) > 0:
			body.append(tr("HUD_SEATS") % card.passengers)
		if int(card.year) > 0:
			body.append(tr("HUD_FROM_YEAR") % card.year)
		lines.append(" · ".join(body))
		var hull: PackedInt64Array = card.armor_hull
		var turret: PackedInt64Array = card.armor_turret
		if hull.size() == 3 and hull[0] > 0:
			var armor := tr("HUD_ARMOR_HULL") % [hull[0], hull[1], hull[2]]
			if turret.size() == 3 and turret[0] > 0:
				armor += ", " + tr("HUD_ARMOR_TURRET") % [turret[0], turret[1], turret[2]]
			lines.append(armor + " " + tr("HUD_ARMOR_SIDES"))
		for w: Dictionary in card.weapons:
			var text := ("%d× " % w.count if int(w.count) > 1 else "") + str(w.name)
			if float(w.caliber_mm) > 0.0:
				text += " (%s mm)" % String.num(w.caliber_mm, 1).trim_suffix(".0")
			if int(w.range_m) > 0:
				text += ", " + tr("HUD_EFFECTIVE_RANGE") % w.range_m
			lines.append(text)
	if index >= 0 and index < HOTKEYS.size():
		lines.append(tr("HUD_KEY_ALT") % OS.get_keycode_string(HOTKEYS[index]))
	if not str(option.blocked).is_empty():
		lines.append(tr("HUD_NOT_AVAILABLE") % blocked_text(str(option.blocked)))
	return "\n".join(lines)


func _place(tile: Control, child: Control, rect: Rect2) -> void:
	child.mouse_filter = Control.MOUSE_FILTER_IGNORE
	child.position = rect.position
	child.size = rect.size
	tile.add_child(child)


func _label(text: String, size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override(&"font_size", size)
	label.add_theme_color_override(&"font_color", color)
	return label


func _badge(text: String) -> Label:
	var label := _label(text, 11, MenuTheme.TEXT)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.08, 0.08, 0.07, 0.72)
	style.set_corner_radius_all(3)
	label.add_theme_stylebox_override(&"normal", style)
	return label
