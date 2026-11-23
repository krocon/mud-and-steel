extends Control
## Map picker (prompt 201): a dialog over the screen with a table of the
## maps on the left (name, size, biome, starts, start layouts, missions,
## origin; sortable by a click on a column title, filtered by a search
## field, biome and players) and the marked map on the right (top view
## with starts and supply points of the chosen start layout, or the 3D
## relief; description, size, weather, hints).
##
## `MapPicker.open(parent, current, filter)` adds it to `parent` and
## returns it; it emits `map_chosen(name)` (Enter, double click, Choose)
## or `cancelled` (Esc, Cancel) and then frees itself. `filter`:
## `min_starts` (only maps with at least that many starts in some layout),
## `maps` (the names to offer; default `MatchSetup.available_maps()`),
## `title` (translation key of the heading).
## Used by the single-player setup, the LAN and Steam lobbies and the map
## editor's "Open".

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const DialogFrame := preload("res://scenes/menu/dialog_frame.gd")
const MapPreview := preload("res://scenes/menu/map_preview.gd")
const Language := preload("res://scenes/menu/language.gd")

signal map_chosen(map_name: String)
signal cancelled

## Table columns: id → title key.
const COLUMNS: Dictionary[String, String] = {
	"title": "MAP_PICKER_NAME",
	"size": "MAP_PICKER_SIZE",
	"biome": "MAP_PICKER_BIOME",
	"starts": "MAP_PICKER_STARTS",
	"layouts": "MAP_PICKER_LAYOUTS",
	"missions": "MAP_PICKER_MISSIONS",
	"origin": "MAP_PICKER_ORIGIN",
}
const BIOME_FILTERS: Array[String] = ["", "forest", "desert", "winter", "mediterranean"]
const PLAYER_FILTERS: Array[int] = [0, 2, 4, 6]
## Below this width the details go under the table.
const WIDE_PX := 1280.0

## Time from `open` to the first frame with the table, ms (self-test).
static var last_open_ms := 0.0

## Rows of the table (`entry`), in table order after filtering.
var rows: Array[Dictionary] = []
var table: Tree
var preview: MapPreview
var search: LineEdit
var biome_filter: OptionButton
var players_filter: OptionButton
var layout_option: OptionButton
var view_button: Button
var exaggerate_box: CheckBox
var choose_button: Button
var cancel_button: Button
var title_label: Label
var description: Label
var facts: Label

var _entries: Array[Dictionary] = []
var _filter: Dictionary = {}
var _current := ""
var _sort := "title"
var _ascending := true
var _split: BoxContainer
var _panel: PanelContainer
var _layouts: Array[Dictionary] = []
var _opened_us := 0


## Opens the picker over `parent` with `current` marked.
static func open(parent: Node, current := "", filter := {}) -> Control:
	var picker: Control = load("res://scenes/menu/map_picker.gd").new()
	picker.setup(current, filter)
	parent.add_child(picker)
	return picker


func setup(current: String, filter: Dictionary) -> void:
	_current = current
	_filter = filter
	_opened_us = Time.get_ticks_usec()


func _ready() -> void:
	name = "MapPicker"
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var dim := ColorRect.new()
	dim.color = Color(0, 0, 0, 0.55)
	dim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	dim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(dim)
	var frame := DialogFrame.new(true)
	add_child(frame)
	_panel = PanelContainer.new()
	frame.add_child(_panel)
	var box := VBoxContainer.new()
	_panel.add_child(box)
	box.add_child(MenuTheme.title(_filter.get("title", "MAP_PICKER_TITLE"), MenuTheme.HEADING_SIZE))

	var filters := HBoxContainer.new()
	box.add_child(filters)
	search = LineEdit.new()
	search.placeholder_text = "MAP_PICKER_SEARCH"
	search.clear_button_enabled = true
	search.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	search.text_changed.connect(func(_t: String) -> void: _fill())
	filters.add_child(search)
	biome_filter = OptionButton.new()
	biome_filter.add_item("MAP_PICKER_ALL_BIOMES")
	for biome in BIOME_FILTERS.slice(1):
		biome_filter.add_item(_biome_name(biome))
	biome_filter.item_selected.connect(func(_i: int) -> void: _fill())
	filters.add_child(biome_filter)
	players_filter = OptionButton.new()
	players_filter.add_item("MAP_PICKER_ALL_PLAYERS")
	for n in PLAYER_FILTERS.slice(1):
		players_filter.add_item(tr("MAP_PICKER_MIN_PLAYERS") % n)
	players_filter.item_selected.connect(func(_i: int) -> void: _fill())
	filters.add_child(players_filter)

	_split = BoxContainer.new()
	_split.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_split.add_theme_constant_override(&"separation", 14)
	box.add_child(_split)
	table = Tree.new()
	table.columns = COLUMNS.size()
	table.column_titles_visible = true
	table.hide_root = true
	table.select_mode = Tree.SELECT_ROW
	table.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	table.size_flags_vertical = Control.SIZE_EXPAND_FILL
	table.custom_minimum_size = Vector2(420, 200)
	table.size_flags_stretch_ratio = 1.6
	table.item_selected.connect(_on_selected)
	table.item_activated.connect(choose)
	table.column_title_clicked.connect(_on_title_clicked)
	_split.add_child(table)

	# The details scroll on a low window instead of pushing the buttons off.
	var details_scroll := ScrollContainer.new()
	details_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	details_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	details_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	details_scroll.custom_minimum_size = Vector2(340, 160)
	_split.add_child(details_scroll)
	var details := VBoxContainer.new()
	details.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	details.size_flags_vertical = Control.SIZE_EXPAND_FILL
	details_scroll.add_child(details)
	title_label = MenuTheme.title("", MenuTheme.FONT_SIZE + 6)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	details.add_child(title_label)
	preview = MapPreview.new()
	preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	preview.custom_minimum_size = Vector2(260, 180)
	details.add_child(preview)
	var view_row := HBoxContainer.new()
	details.add_child(view_row)
	view_button = Button.new()
	view_button.text = "MAP_PICKER_VIEW_3D"
	view_button.pressed.connect(func() -> void: set_relief(not preview.show_relief))
	view_row.add_child(view_button)
	exaggerate_box = CheckBox.new()
	exaggerate_box.text = "MAP_PICKER_EXAGGERATE"
	exaggerate_box.button_pressed = true
	exaggerate_box.visible = false
	exaggerate_box.toggled.connect(func(on: bool) -> void: preview.exaggeration = 2.0 if on else 1.0)
	view_row.add_child(exaggerate_box)
	layout_option = OptionButton.new()
	layout_option.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	layout_option.item_selected.connect(func(i: int) -> void:
		preview.show_map(preview.map_name, _layouts[i]["name"]))
	view_row.add_child(layout_option)
	description = MenuTheme.hint("")
	details.add_child(description)
	facts = Label.new()
	facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	details.add_child(facts)

	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	cancel_button = _button(buttons, "COMMON_CANCEL", cancel)
	choose_button = _button(buttons, "MAP_PICKER_CHOOSE", func() -> void: choose())

	var names: PackedStringArray = _filter.get("maps", MatchSetup.available_maps())
	for map_name in names:
		_entries.append(entry(map_name))
	_fill()
	resized.connect(_fit)
	_fit()
	table.grab_focus.call_deferred()
	_measure_open.call_deferred()


func _measure_open() -> void:
	last_open_ms = (Time.get_ticks_usec() - _opened_us) / 1000.0


func _input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		cancel()


## Chooses the marked map.
func choose() -> void:
	var item := table.get_selected()
	if item == null:
		return
	var map_name: String = item.get_metadata(0)
	map_chosen.emit(map_name)
	queue_free()


func cancel() -> void:
	cancelled.emit()
	queue_free()


## Marks `map_name` in the table (false if it is not shown).
func select(map_name: String) -> bool:
	var item := table.get_root().get_first_child() if table.get_root() != null else null
	while item != null:
		if item.get_metadata(0) == map_name:
			item.select(0)
			table.scroll_to_item(item)
			return true
		item = item.get_next()
	return false


## Shows the 3D relief (true) or the top view.
func set_relief(on: bool) -> void:
	preview.show_relief = on
	view_button.text = "MAP_PICKER_VIEW_TOP" if on else "MAP_PICKER_VIEW_3D"
	exaggerate_box.visible = on


## Table width and details side by side on a wide window, one above the
## other on a narrow one.
func _fit() -> void:
	var room := get_viewport_rect().size
	_panel.custom_minimum_size.x = minf(room.x * 0.94, 1600.0)
	_split.vertical = room.x < WIDE_PX
	table.custom_minimum_size.y = 160.0 if _split.vertical else 200.0


## What the table and the details show of a map: `name`, `title`,
## `size_m`, `biome`, `starts` (standard layout), `max_starts` (any
## layout), `layouts`, `missions` (mission scripts and battles on it),
## `origin` (translation key), `weather`, `daytime`.
static func entry(map_name: String) -> Dictionary:
	var info := MapPreview.info(map_name)
	var layouts: Array = info.get("layouts", [])
	var starts := 0
	var most := 0
	for l: Dictionary in layouts:
		var n: int = (l["starts"] as Array).size()
		if starts == 0:
			starts = n
		most = maxi(most, n)
	return {
		"name": map_name,
		"title": MatchSetup.map_title(map_name),
		"size_m": info.get("size_m", Vector2.ZERO),
		"biome": info.get("biome", ""),
		"weather": info.get("weather", ""),
		"daytime": info.get("daytime", ""),
		"starts": starts,
		"max_starts": most,
		"layouts": layouts.size(),
		"missions": _count_files(GamePaths.map_dir(map_name).path_join("missions"), ".toml")
			+ _count_files(GamePaths.data_dir().path_join("battles").path_join(map_name), ".toml"),
		"origin": origin(map_name),
	}


## Translation key of how a map was made, from `generator` in its
## `map.toml`: geodata, a picture sketch, a generator, or by hand.
static func origin(map_name: String) -> String:
	var text := FileAccess.get_file_as_string(GamePaths.map_dir(map_name).path_join("map.toml"))
	for line in text.split("\n"):
		if line.begins_with("["):
			break
		if line.begins_with("generator"):
			if line.contains("geo_import"):
				return "MAP_ORIGIN_GEO"
			if line.contains("gen-sketch"):
				return "MAP_ORIGIN_SKETCH"
			return "MAP_ORIGIN_GENERATED"
	return "MAP_ORIGIN_HAND"


## Size as text: meters below a kilometer, else kilometers.
static func size_text(size_m: Vector2) -> String:
	if maxf(size_m.x, size_m.y) < 1000.0:
		return "%d × %d m" % [roundi(size_m.x), roundi(size_m.y)]
	return "%.1f × %.1f km" % [size_m.x / 1000.0, size_m.y / 1000.0]


static func _biome_name(biome: String) -> String:
	return Language.code_text("MATCH_BIOME_", biome).capitalize() if not biome.is_empty() else ""


static func _count_files(folder: String, ext: String) -> int:
	var dir := DirAccess.open(folder)
	if dir == null:
		return 0
	return Array(dir.get_files()).filter(func(f: String) -> bool: return f.ends_with(ext)).size()


func _passes(e: Dictionary) -> bool:
	var needle := search.text.strip_edges().to_lower()
	if not needle.is_empty() and not (String(e["title"]).to_lower().contains(needle) or String(e["name"]).contains(needle)):
		return false
	var biome := BIOME_FILTERS[maxi(biome_filter.selected, 0)]
	if not biome.is_empty() and e["biome"] != biome:
		return false
	var players := maxi(PLAYER_FILTERS[maxi(players_filter.selected, 0)], int(_filter.get("min_starts", 0)))
	return e["max_starts"] >= players


func _sort_value(e: Dictionary) -> Variant:
	match _sort:
		"size":
			var s: Vector2 = e["size_m"]
			return s.x * s.y
		"biome":
			return _biome_name(e["biome"])
		"origin":
			return tr(e["origin"])
		"title":
			return String(e["title"]).to_lower()
	return e[_sort]


## Fills the table from the entries that pass the filters, sorted.
func _fill() -> void:
	var marked := _current
	var selected := table.get_selected()
	if selected != null:
		marked = selected.get_metadata(0)
	rows.assign(_entries.filter(_passes))
	rows.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var va: Variant = _sort_value(a)
		var vb: Variant = _sort_value(b)
		if va == vb:
			return String(a["title"]) < String(b["title"])
		return (va < vb) == _ascending)
	table.clear()
	var root := table.create_item()
	var index := 0
	for id in COLUMNS:
		var arrow := ("  ▲" if _ascending else "  ▼") if id == _sort else ""
		table.set_column_title(index, tr(COLUMNS[id]) + arrow)
		table.set_column_expand(index, id == "title")
		table.set_column_clip_content(index, id == "title")
		index += 1
	for e in rows:
		var item := table.create_item(root)
		item.set_metadata(0, e["name"])
		item.set_text(0, e["title"])
		item.set_tooltip_text(0, e["name"])
		item.set_text(1, size_text(e["size_m"]))
		item.set_text(2, _biome_name(e["biome"]))
		item.set_text(3, str(e["starts"]) if e["starts"] == e["max_starts"] else "%d–%d" % [e["starts"], e["max_starts"]])
		item.set_text(4, str(e["layouts"]))
		item.set_text(5, str(e["missions"]) if e["missions"] > 0 else "–")
		item.set_text(6, tr(e["origin"]))
		for c in range(1, COLUMNS.size() - 1):
			item.set_text_alignment(c, HORIZONTAL_ALIGNMENT_CENTER)
	_fit_columns()
	if not select(marked) and root.get_first_child() != null:
		root.get_first_child().select(0)
	choose_button.disabled = table.get_selected() == null
	if table.get_selected() == null:
		_show_details("")


## Every column but the name as wide as its widest text.
func _fit_columns() -> void:
	var font := table.get_theme_font(&"font")
	var title_font := table.get_theme_font(&"title_button_font")
	var font_size := table.get_theme_font_size(&"font_size")
	for c in COLUMNS.size():
		var widest := (title_font if title_font != null else font).get_string_size(table.get_column_title(c), HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
		var item := table.get_root().get_first_child()
		while item != null:
			widest = maxf(widest, font.get_string_size(item.get_text(c), HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x)
			item = item.get_next()
		table.set_column_custom_minimum_width(c, ceili(widest) + 20 if c > 0 else 160)


func _on_title_clicked(column: int, _mouse_button: int) -> void:
	var id: String = COLUMNS.keys()[column]
	if id == _sort:
		_ascending = not _ascending
	else:
		_sort = id
		_ascending = true
	_fill()


func _on_selected() -> void:
	var item := table.get_selected()
	choose_button.disabled = item == null
	_show_details(item.get_metadata(0) if item != null else "")


func _show_details(map_name: String) -> void:
	var e: Dictionary = {}
	for row in rows:
		if row["name"] == map_name:
			e = row
	title_label.text = e.get("title", "")
	layout_option.clear()
	_layouts.assign(MapPreview.info(map_name).get("layouts", []) if not e.is_empty() else [])
	for v in _layouts:
		var label: String = v["title"] if not String(v["title"]).is_empty() else v["name"]
		layout_option.add_item(tr("MATCH_VARIANT_STANDARD") if v["name"] == "standard" else label)
	layout_option.disabled = _layouts.size() <= 1
	preview.show_map(map_name)
	if e.is_empty():
		description.text = ""
		facts.text = ""
		return
	var desc_key := "MAP_%s_DESC" % map_name.to_upper()
	description.text = tr(desc_key) if Language.has(desc_key) else MatchSetup.describe_map(map_name)
	var lines: PackedStringArray = [
		"%s: %s, %s" % [tr("MAP_PICKER_SIZE"), size_text(e["size_m"]), _biome_name(e["biome"])],
		"%s: %s, %s" % [tr("MAP_PICKER_DEFAULT_WEATHER"),
			Language.code_text("MATCH_WEATHER_", e["weather"]), Language.code_text("MATCH_DAYTIME_", e["daytime"])],
		"%s: %d   %s: %d   %s: %d" % [tr("MAP_PICKER_STARTS"), e["max_starts"], tr("MAP_PICKER_LAYOUTS"), e["layouts"], tr("MAP_PICKER_MISSIONS"), e["missions"]],
		"%s: %s" % [tr("MAP_PICKER_ORIGIN"), tr(e["origin"])],
	]
	if MatchSetup.is_large(map_name):
		lines.append(tr("MATCH_LARGE_MAP"))
	facts.text = "\n".join(lines)


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
