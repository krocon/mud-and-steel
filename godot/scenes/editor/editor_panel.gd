extends Control
## Tool panel of the map editor (prompt 028): file buttons, tools, brush
## settings, the status line and the New / Open / Save dialogs. Only UI; the
## actions live in `map_editor.gd` (`main`).

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const EditorData := preload("res://scenes/editor/editor_data.gd")
const Language := preload("res://scenes/menu/language.gd")

var main: Node3D
var title: Label
var status: Label
var undo_button: Button
var redo_button: Button
var tool_buttons: Dictionary[String, Button] = {}
var radius_slider: HSlider
var strength_slider: HSlider
var density_slider: HSlider
var terrain_option: OptionButton
var preset_option: OptionButton
var place_option: OptionButton
var heading_spin: SpinBox
## Team of the start a start tool places (prompt 037).
var start_team_option: OptionButton
## Dialogs by name: "new", "open", "save".
var dialogs: Dictionary[String, PanelContainer] = {}
var new_name: LineEdit
var new_size: OptionButton
var new_biome: OptionButton
var open_option: OptionButton
var save_name: LineEdit
## Default weather and time of day of matches on the map (prompt 040).
var save_weather: OptionButton
var save_daytime: OptionButton

var _settings: Dictionary[String, Control] = {}


func _ready() -> void:
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var side := PanelContainer.new()
	side.position = Vector2(8, 8)
	side.custom_minimum_size = Vector2(300, 0)
	add_child(side)
	var box := VBoxContainer.new()
	side.add_child(box)
	title = MenuTheme.title("", MenuTheme.FONT_SIZE + 4)
	box.add_child(title)

	var files := HFlowContainer.new()
	box.add_child(files)
	_button(files, "EDITOR_NEW", func() -> void: show_dialog("new"))
	_button(files, "EDITOR_OPEN", func() -> void: show_dialog("open"))
	_button(files, "COMMON_SAVE", func() -> void: show_dialog("save"))
	_button(files, "EDITOR_CHECK", main.validate_map)
	undo_button = _button(files, "EDITOR_UNDO", main.undo)
	redo_button = _button(files, "EDITOR_REDO", main.redo)
	_button(files, "EDITOR_MENU", main.back_to_menu)

	var tools := GridContainer.new()
	tools.columns = 2
	box.add_child(tools)
	var group := ButtonGroup.new()
	for id in EditorData.TOOLS:
		var button := _button(tools, EditorData.TOOLS[id], select_tool.bind(id))
		button.toggle_mode = true
		button.button_group = group
		button.custom_minimum_size = Vector2(140, 32)
		# German labels are longer: trim them, the tooltip shows the whole name.
		button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		button.tooltip_text = EditorData.TOOLS[id]
		tool_buttons[id] = button

	radius_slider = _slider(box, "radius", "EDITOR_RADIUS", 2.0, 120.0, main.radius, func(v: float) -> void: main.radius = v)
	strength_slider = _slider(box, "strength", "EDITOR_STRENGTH", 0.05, 1.0, main.strength, func(v: float) -> void: main.strength = v)
	density_slider = _slider(box, "density", "EDITOR_DENSITY", 10.0, 1500.0, main.density, func(v: float) -> void: main.density = v)

	terrain_option = OptionButton.new()
	for name in main.editor.terrain_names():
		terrain_option.add_item(Language.code_text("EDITOR_TERRAIN_", name, name.replace("_", " ")))
	terrain_option.select(main.terrain_id)
	terrain_option.item_selected.connect(func(i: int) -> void: main.terrain_id = i)
	_setting(box, "terrain", "EDITOR_TERRAIN", terrain_option)

	preset_option = OptionButton.new()
	for name: String in EditorData.SCATTER_PRESETS:
		preset_option.add_item(EditorData.SCATTER_LABELS.get(name, name))
		preset_option.set_item_metadata(preset_option.item_count - 1, name)
	preset_option.item_selected.connect(func(i: int) -> void: main.scatter_preset = preset_option.get_item_metadata(i))
	_setting(box, "preset", "EDITOR_OBJECTS", preset_option)

	place_option = OptionButton.new()
	# Grouped (prompt 041): a separator with the group name before each group.
	var groups: Dictionary[String, Array] = {}
	for kind in main.editor.object_catalog():
		var group_of := EditorData.object_group(kind)
		if not groups.has(group_of):
			groups[group_of] = []
		groups[group_of].append(kind)
	var order: Array = EditorData.OBJECT_GROUPS.map(func(g: Array) -> String: return g[0])
	order.append(EditorData.OTHER_GROUP)
	for group_name: String in order:
		if not groups.has(group_name):
			continue
		place_option.add_separator(group_name)
		for kind: String in groups[group_name]:
			place_option.add_item(kind)
			if kind == main.place_kind:
				place_option.select(place_option.item_count - 1)
	place_option.item_selected.connect(func(i: int) -> void: main.place_kind = place_option.get_item_text(i))
	_setting(box, "place", "EDITOR_OBJECT", place_option)
	heading_spin = SpinBox.new()
	heading_spin.max_value = 359
	heading_spin.step = 15
	heading_spin.value_changed.connect(func(v: float) -> void: main.place_heading = int(v))
	_setting(box, "heading", "EDITOR_HEADING", heading_spin)
	# Prompt 059: compounds of data/compounds.toml.
	var compound_option := OptionButton.new()
	for entry: String in main.editor.compound_names():
		compound_option.add_item(Language.code_text("EDITOR_COMPOUND_", entry.get_slice(":", 0), entry.get_slice(":", 1)))
		compound_option.set_item_metadata(compound_option.item_count - 1, entry.get_slice(":", 0))
		if entry.get_slice(":", 0) == main.compound_name:
			compound_option.select(compound_option.item_count - 1)
	compound_option.item_selected.connect(func(i: int) -> void: main.compound_name = compound_option.get_item_metadata(i))
	_setting(box, "compound", "EDITOR_COMPOUND", compound_option)
	start_team_option = OptionButton.new()
	for label in EditorData.START_TEAMS:
		start_team_option.add_item(label)
	start_team_option.item_selected.connect(func(i: int) -> void: main.set_start_team(i - 1))
	_setting(box, "start_team", "EDITOR_TEAM", start_team_option)
	# Prompt 039: name of the next trigger area or spawn point.
	var point_name := LineEdit.new()
	point_name.placeholder_text = "EDITOR_NUMBERED"
	point_name.text_changed.connect(func(text: String) -> void: main.mission_point_name = text)
	_setting(box, "point_name", "EDITOR_NAME", point_name)

	status = MenuTheme.hint("")
	status.custom_minimum_size = Vector2(300, 0)
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(status)

	_build_dialogs()
	select_tool(main.tool)
	update_title()
	update_history()


func select_tool(id: String) -> void:
	main.tool = id
	tool_buttons[id].set_pressed_no_signal(true)
	var uses: Dictionary[String, Array] = {
		"radius": ["raise", "lower", "smooth", "flatten", "paint", "river", "scatter", "erase", "village",
			"mission_area"] + EditorData.MOUNTAIN_TOOLS,
		"strength": ["raise", "lower", "smooth", "flatten", "river", "ridge", "valley", "terrace"],
		"density": ["scatter"], "preset": ["scatter"], "terrain": ["paint", "road"],
		"place": ["place"], "heading": ["place", "compound"], "compound": ["compound"],
		"start_team": EditorData.START_TOOLS,
		"point_name": EditorData.MISSION_TOOLS,
	}
	for key in _settings:
		_settings[key].visible = id in uses[key]
	if id in EditorData.START_TOOLS:
		start_team_option.select(main.start_team_of(int(id.right(1))) + 1)


func set_radius(value: float) -> void:
	radius_slider.value = clampf(value, radius_slider.min_value, radius_slider.max_value)


func set_status(text: String, error := false) -> void:
	status.text = text
	status.add_theme_color_override("font_color", MenuTheme.DEFEAT if error else MenuTheme.TEXT_DIM)


func update_title() -> void:
	var size: Vector2 = main.editor.map_size()
	title.text = "%s  (%d × %d m)" % [main.editor.map_name(), size.x, size.y]


func update_history() -> void:
	undo_button.disabled = main.editor.undo_steps() == 0
	redo_button.disabled = main.editor.redo_steps() == 0


func show_dialog(dialog: String) -> void:
	for key in dialogs:
		dialogs[key].visible = key == dialog
	match dialog:
		"open":
			open_option.clear()
			for name in MatchSetup.available_maps():
				open_option.add_item(name)
		"save":
			save_name.text = main.editor.map_name()
			var current: PackedStringArray = main.editor.map_weather()
			if current.size() == 2:
				save_weather.select(maxi(MatchSetup.WEATHERS.keys().find(current[0]) - 1, 0))
				save_daytime.select(maxi(MatchSetup.DAYTIMES.keys().find(current[1]) - 1, 0))


func _build_dialogs() -> void:
	var box := _dialog("new", "EDITOR_NEW_MAP")
	new_name = LineEdit.new()
	new_name.text = "my_map"
	box.add_child(_row("EDITOR_NAME", new_name))
	new_size = OptionButton.new()
	for size in EditorData.SIZES:
		new_size.add_item("%d m" % size)
	new_size.select(1)
	box.add_child(_row("EDITOR_SIZE", new_size))
	new_biome = OptionButton.new()
	for biome in EditorData.BIOMES:
		new_biome.add_item("EDITOR_BIOME_" + biome.to_upper())
	box.add_child(_row("EDITOR_BIOME", new_biome))
	_dialog_buttons(box, "EDITOR_CREATE", _create_map)

	box = _dialog("open", "EDITOR_OPEN_MAP")
	open_option = OptionButton.new()
	box.add_child(_row("EDITOR_MAP", open_option))
	_dialog_buttons(box, "EDITOR_OPEN", _open_map)

	box = _dialog("save", "EDITOR_SAVE_MAP")
	save_name = LineEdit.new()
	box.add_child(_row("EDITOR_NAME", save_name))
	# Without the "Map default" entry of the setup screen.
	save_weather = OptionButton.new()
	for id: String in MatchSetup.WEATHERS:
		if not id.is_empty():
			save_weather.add_item(MatchSetup.WEATHERS[id])
	box.add_child(_row("EDITOR_WEATHER", save_weather))
	save_daytime = OptionButton.new()
	for id: String in MatchSetup.DAYTIMES:
		if not id.is_empty():
			save_daytime.add_item(MatchSetup.DAYTIMES[id])
	box.add_child(_row("EDITOR_DAYTIME", save_daytime))
	box.add_child(MenuTheme.hint("EDITOR_SAVE_HINT"))
	_dialog_buttons(box, "COMMON_SAVE", _save_map)


func _create_map() -> void:
	main.open_new(new_name.text.strip_edges(), EditorData.SIZES[new_size.selected], EditorData.BIOMES[new_biome.selected])


func _open_map() -> void:
	if open_option.selected >= 0:
		main.open_map(open_option.get_item_text(open_option.selected))


func _save_map() -> void:
	main.editor.set_map_weather(MatchSetup.WEATHERS.keys()[save_weather.selected + 1], MatchSetup.DAYTIMES.keys()[save_daytime.selected + 1])
	if main.save_as(save_name.text):
		show_dialog("")


func _dialog(key: String, heading: String) -> VBoxContainer:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(center)
	var frame := PanelContainer.new()
	frame.visible = false
	center.add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(420, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title(heading, MenuTheme.HEADING_SIZE))
	dialogs[key] = frame
	return box


func _dialog_buttons(box: VBoxContainer, text: String, action: Callable) -> void:
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(row)
	_button(row, "COMMON_CANCEL", func() -> void: show_dialog(""))
	_button(row, text, action)


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(action)
	parent.add_child(button)
	return button


func _slider(parent: Control, key: String, text: String, lo: float, hi: float, value: float, changed: Callable) -> HSlider:
	var slider := HSlider.new()
	slider.min_value = lo
	slider.max_value = hi
	slider.step = 0.01
	slider.value = value
	slider.focus_mode = Control.FOCUS_NONE
	slider.custom_minimum_size = Vector2(150, 24)
	var label := Label.new()
	slider.value_changed.connect(func(v: float) -> void: label.text = _number(v, hi))
	slider.value_changed.connect(changed)
	label.text = _number(value, hi)
	var row := _setting(parent, key, text, slider)
	row.add_child(label)
	return slider


func _setting(parent: Control, key: String, text: String, control: Control) -> HBoxContainer:
	var row := _row(text, control)
	parent.add_child(row)
	_settings[key] = row
	return row


func _row(text: String, control: Control) -> HBoxContainer:
	var row := HBoxContainer.new()
	var label := Label.new()
	label.text = text
	label.custom_minimum_size = Vector2(100, 0)
	row.add_child(label)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(control)
	return row


static func _number(value: float, hi: float) -> String:
	return "%.2f" % value if hi <= 1.0 else "%d" % roundi(value)
