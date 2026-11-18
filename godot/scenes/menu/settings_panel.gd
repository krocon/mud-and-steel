extends PanelContainer
## Settings screen (prompt 027), used by the main menu and the pause menu.
##
## Tabs: Graphics (quality level, depth of field; window, resolution, VSync,
## frame limit, render scale, prompt 043), Controls (keyboard bindings of
## camera and orders), Audio (volume per bus, mute without focus), Camera
## (pan and zoom sensitivity, edge scrolling), Interface (language, UI scale,
## font size, color palette, subtitles, screen shake (prompt 043), health
## bars (prompt 031)). All texts are translation keys (`locale/settings.csv`).
## Changes apply at once; "Save" writes them to the settings
## file, "Cancel" reloads the file. Emits `changed` after every change (the
## match re-applies quality and camera) and `closed` when the screen closes.

signal changed
signal closed

const Settings := preload("res://scenes/menu/settings.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
## Bus volume sliders below the master volume: settings key -> label.
const BUS_SLIDERS: Dictionary[String, String] = {
	"effects_volume": "SET_VOLUME_EFFECTS", "ui_volume": "SET_VOLUME_UI", "music_volume": "SET_VOLUME_MUSIC",
	"ambience_volume": "SET_VOLUME_AMBIENCE",
}
## Option lists: settings key -> [values, label keys].
const OPTIONS := {
	"window_mode": [["windowed", "fullscreen", "exclusive"], ["SET_WINDOWED", "SET_FULLSCREEN", "SET_EXCLUSIVE"]],
	"upscaler": [["bilinear", "fsr1", "fsr2"], ["SET_UPSCALER_BILINEAR", "SET_UPSCALER_FSR1", "SET_UPSCALER_FSR2"]],
	"color_palette": [["standard", "red_green", "blue_yellow"],
		["SET_PALETTE_STANDARD", "SET_PALETTE_RED_GREEN", "SET_PALETTE_BLUE_YELLOW"]],
	"font_size": [[0.85, 1.0, 1.15, 1.3], ["SET_FONT_SMALL", "SET_FONT_NORMAL", "SET_FONT_LARGE", "SET_FONT_HUGE"]],
}

var quality_option: OptionButton
var volume_slider: HSlider
## Volume sliders of the other buses (prompt 029), by settings key.
var bus_sliders: Dictionary[String, HSlider] = {}
var mute_unfocused_box: CheckBox
var pan_slider: HSlider
var zoom_slider: HSlider
var edge_scroll_box: CheckBox
var dof_option: OptionButton
var health_bar_option: OptionButton
## Option buttons of OPTIONS, by settings key (prompt 043).
var options: Dictionary[String, OptionButton] = {}
var language_option: OptionButton
var resolution_option: OptionButton
var fps_option: OptionButton
var vsync_box: CheckBox
var render_scale_slider: HSlider
var ui_scale_slider: HSlider
var subtitles_box: CheckBox
var shake_box: CheckBox
var cloud_box: CheckBox
var save_button: Button
var cancel_button: Button
## Action whose key is being captured, or "".
var capturing := ""

var _key_buttons: Dictionary[String, Button] = {}


func _ready() -> void:
	Settings.ensure_loaded()
	theme = MenuTheme.get_theme()
	custom_minimum_size = Vector2(700, 0)
	var box := VBoxContainer.new()
	add_child(box)
	box.add_child(MenuTheme.title("COMMON_SETTINGS", MenuTheme.HEADING_SIZE))
	var tabs := TabContainer.new()
	tabs.custom_minimum_size = Vector2(0, 420)
	box.add_child(tabs)
	tabs.add_child(_graphics_tab())
	tabs.add_child(_controls_tab())
	tabs.add_child(_audio_tab())
	tabs.add_child(_camera_tab())
	tabs.add_child(_interface_tab())

	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	var defaults := Button.new()
	defaults.text = "COMMON_DEFAULTS"
	defaults.pressed.connect(func() -> void:
		Settings.reset()
		_refresh()
		changed.emit())
	buttons.add_child(defaults)
	cancel_button = Button.new()
	cancel_button.text = "COMMON_CANCEL"
	cancel_button.pressed.connect(cancel)
	buttons.add_child(cancel_button)
	save_button = Button.new()
	save_button.text = "COMMON_SAVE"
	save_button.pressed.connect(save)
	buttons.add_child(save_button)
	_refresh()


func save() -> void:
	Settings.save_file()
	closed.emit()


func cancel() -> void:
	capturing = ""
	Settings.load_file()
	changed.emit()
	closed.emit()


## Starts capturing the next key for `action` (also used by the self-test).
func capture(action: String) -> void:
	capturing = action
	_key_buttons[action].text = tr("SET_PRESS_KEY")


func _input(event: InputEvent) -> void:
	if capturing.is_empty() or not visible:
		return
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo:
		return
	get_viewport().set_input_as_handled()
	# A modifier alone waits for the key it goes with (Shift+1).
	if key.physical_keycode in [KEY_SHIFT, KEY_CTRL, KEY_ALT, KEY_META]:
		return
	if key.physical_keycode != KEY_ESCAPE:
		Settings.bind(capturing, key.get_physical_keycode_with_modifiers())
		changed.emit()
	capturing = ""
	_refresh()


func _graphics_tab() -> Control:
	var tab := _tab("SET_TAB_GRAPHICS")
	quality_option = OptionButton.new()
	for level in GraphicsQuality.LEVELS:
		quality_option.add_item("SET_QUALITY_" + level.to_upper())
	quality_option.item_selected.connect(func(index: int) -> void:
		_change("quality", GraphicsQuality.LEVELS[index]))
	tab.add_child(_row("SET_QUALITY", quality_option))
	tab.add_child(MenuTheme.hint("SET_QUALITY_HINT"))
	dof_option = OptionButton.new()
	for label in ["COMMON_OFF", "SET_DOF_SUBTLE", "SET_DOF_MINIATURE"]:
		dof_option.add_item(label)
	dof_option.item_selected.connect(func(index: int) -> void:
		_change("depth_of_field", Settings.DOF_MODES[index]))
	tab.add_child(_row("SET_DOF", dof_option))
	tab.add_child(MenuTheme.hint("SET_DOF_HINT"))
	cloud_box = CheckBox.new()
	cloud_box.text = "SET_CLOUD_SHADOWS"
	cloud_box.toggled.connect(func(on: bool) -> void: _change("cloud_shadows", on))
	tab.add_child(cloud_box)
	tab.add_child(MenuTheme.hint("SET_CLOUD_SHADOWS_HINT"))
	tab.add_child(_row("SET_WINDOW_MODE", _option("window_mode")))
	resolution_option = OptionButton.new()
	resolution_option.add_item("SET_RESOLUTION_KEEP")
	for r in Settings.RESOLUTIONS:
		resolution_option.add_item(r.replace("x", " × "))
	resolution_option.item_selected.connect(func(index: int) -> void:
		_change("resolution", "" if index == 0 else Settings.RESOLUTIONS[index - 1]))
	tab.add_child(_row("SET_RESOLUTION", resolution_option))
	vsync_box = CheckBox.new()
	vsync_box.text = "SET_VSYNC"
	vsync_box.toggled.connect(func(on: bool) -> void: _change("vsync", on))
	tab.add_child(vsync_box)
	fps_option = OptionButton.new()
	for limit in Settings.FPS_LIMITS:
		fps_option.add_item("SET_FPS_NONE" if limit == 0 else str(limit))
	fps_option.item_selected.connect(func(index: int) -> void:
		_change("fps_limit", Settings.FPS_LIMITS[index]))
	tab.add_child(_row("SET_FPS_LIMIT", fps_option))
	render_scale_slider = _slider(0.5, 1.0, 0.05, "render_scale")
	tab.add_child(_row("SET_RENDER_SCALE", render_scale_slider))
	tab.add_child(_row("SET_UPSCALER", _option("upscaler")))
	tab.add_child(MenuTheme.hint("SET_RENDER_SCALE_HINT"))
	return _scrolled(tab)


func _controls_tab() -> Control:
	# Camera and order keys (prompt 031) do not fit without scrolling.
	var tab := _tab("SET_TAB_CONTROLS")
	for action in Settings.BINDABLE:
		var button := Button.new()
		button.custom_minimum_size = Vector2(160, 0)
		button.pressed.connect(capture.bind(action))
		_key_buttons[action] = button
		tab.add_child(_row(Settings.BINDABLE[action], button))
	tab.add_child(MenuTheme.hint("SET_CONTROLS_HINT"))
	return _scrolled(tab)


func _audio_tab() -> Control:
	var tab := _tab("SET_TAB_AUDIO")
	volume_slider = _slider(0.0, 1.0, 0.05, "master_volume")
	tab.add_child(_row("SET_VOLUME_MASTER", volume_slider))
	for key: String in BUS_SLIDERS:
		bus_sliders[key] = _slider(0.0, 1.0, 0.05, key)
		tab.add_child(_row(BUS_SLIDERS[key], bus_sliders[key]))
	mute_unfocused_box = CheckBox.new()
	mute_unfocused_box.text = "SET_MUTE_UNFOCUSED"
	mute_unfocused_box.toggled.connect(func(on: bool) -> void: _change("mute_unfocused", on))
	tab.add_child(mute_unfocused_box)
	tab.add_child(MenuTheme.hint("SET_AUDIO_HINT"))
	return tab


func _camera_tab() -> Control:
	var tab := _tab("SET_TAB_CAMERA")
	pan_slider = _slider(0.25, 3.0, 0.05, "pan_sensitivity")
	tab.add_child(_row("SET_PAN_SPEED", pan_slider))
	zoom_slider = _slider(0.25, 3.0, 0.05, "zoom_sensitivity")
	tab.add_child(_row("SET_ZOOM_SPEED", zoom_slider))
	edge_scroll_box = CheckBox.new()
	edge_scroll_box.text = "SET_EDGE_SCROLL"
	edge_scroll_box.toggled.connect(func(on: bool) -> void: _change("edge_scroll", on))
	tab.add_child(edge_scroll_box)
	return tab


func _interface_tab() -> Control:
	var tab := _tab("SET_TAB_INTERFACE")
	language_option = OptionButton.new()
	language_option.add_item("SET_LANGUAGE_AUTO")
	for code in Language.LANGUAGES:
		language_option.add_item(Language.LANGUAGES[code])
	language_option.item_selected.connect(func(index: int) -> void:
		_change("language", "auto" if index == 0 else Language.LANGUAGES.keys()[index - 1]))
	tab.add_child(_row("SET_LANGUAGE", language_option))
	# Applied when the drag ends: scaling while dragging moves the slider
	# away from the mouse.
	ui_scale_slider = HSlider.new()
	ui_scale_slider.min_value = 0.75
	ui_scale_slider.max_value = 1.5
	ui_scale_slider.step = 0.05
	ui_scale_slider.custom_minimum_size = Vector2(240, 24)
	ui_scale_slider.drag_ended.connect(func(_moved: bool) -> void: _change("ui_scale", ui_scale_slider.value))
	tab.add_child(_row("SET_UI_SCALE", ui_scale_slider))
	tab.add_child(_row("SET_FONT_SIZE", _option("font_size")))
	tab.add_child(_row("SET_PALETTE", _option("color_palette")))
	tab.add_child(MenuTheme.hint("SET_PALETTE_HINT"))
	subtitles_box = CheckBox.new()
	subtitles_box.text = "SET_SUBTITLES"
	subtitles_box.toggled.connect(func(on: bool) -> void: _change("subtitles", on))
	tab.add_child(subtitles_box)
	shake_box = CheckBox.new()
	shake_box.text = "SET_SCREEN_SHAKE"
	shake_box.toggled.connect(func(on: bool) -> void: _change("screen_shake", on))
	tab.add_child(shake_box)
	health_bar_option = OptionButton.new()
	for label in ["SET_HEALTH_DAMAGED", "SET_HEALTH_SELECTED", "SET_HEALTH_NEVER"]:
		health_bar_option.add_item(label)
	health_bar_option.item_selected.connect(func(index: int) -> void:
		_change("health_bars", Settings.HEALTH_BAR_MODES[index]))
	tab.add_child(_row("SET_HEALTH_BARS", health_bar_option))
	return _scrolled(tab)


func _refresh() -> void:
	health_bar_option.select(maxi(Settings.HEALTH_BAR_MODES.find(Settings.get_value("health_bars")), 0))
	quality_option.select(maxi(GraphicsQuality.LEVELS.find(Settings.get_value("quality")), 0))
	volume_slider.set_value_no_signal(Settings.get_value("master_volume"))
	for key in bus_sliders:
		bus_sliders[key].set_value_no_signal(Settings.get_value(key))
	mute_unfocused_box.set_pressed_no_signal(Settings.get_value("mute_unfocused"))
	pan_slider.set_value_no_signal(Settings.get_value("pan_sensitivity"))
	zoom_slider.set_value_no_signal(Settings.get_value("zoom_sensitivity"))
	edge_scroll_box.set_pressed_no_signal(Settings.get_value("edge_scroll"))
	dof_option.select(maxi(Settings.DOF_MODES.find(Settings.get_value("depth_of_field")), 0))
	var language: String = Settings.get_value("language")
	language_option.select(0 if language == "auto" else 1 + Language.LANGUAGES.keys().find(language))
	ui_scale_slider.set_value_no_signal(Settings.get_value("ui_scale"))
	render_scale_slider.set_value_no_signal(Settings.get_value("render_scale"))
	vsync_box.set_pressed_no_signal(Settings.get_value("vsync"))
	subtitles_box.set_pressed_no_signal(Settings.get_value("subtitles"))
	shake_box.set_pressed_no_signal(Settings.get_value("screen_shake"))
	cloud_box.set_pressed_no_signal(Settings.get_value("cloud_shadows"))
	fps_option.select(maxi(Settings.FPS_LIMITS.find(Settings.get_value("fps_limit")), 0))
	var resolution: String = Settings.get_value("resolution")
	resolution_option.select(0 if resolution.is_empty() else 1 + Settings.RESOLUTIONS.find(resolution))
	for key in options:
		options[key].select(maxi(OPTIONS[key][0].find(Settings.get_value(key)), 0))
	for action in _key_buttons:
		_key_buttons[action].text = Settings.key_name(Settings.bindings.get(action, 0))


func _change(key: String, value: Variant) -> void:
	Settings.set_value(key, value)
	changed.emit()


## An option button over OPTIONS[key].
func _option(key: String) -> OptionButton:
	var option := OptionButton.new()
	for label: String in OPTIONS[key][1]:
		option.add_item(label)
	option.item_selected.connect(func(index: int) -> void: _change(key, OPTIONS[key][0][index]))
	options[key] = option
	return option


## `tab` in a vertical scroll container that carries the tab's name (the
## translation key of its title).
func _scrolled(tab: VBoxContainer) -> Control:
	var scroll := ScrollContainer.new()
	scroll.name = tab.name
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tab.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(tab)
	return scroll


func _tab(title: String) -> VBoxContainer:
	var tab := VBoxContainer.new()
	tab.name = title
	return tab


func _row(text: String, control: Control) -> Control:
	var row := HBoxContainer.new()
	var label := Label.new()
	label.text = text
	label.custom_minimum_size = Vector2(230, 0)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	row.add_child(label)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(control)
	return row


func _slider(from: float, to: float, step: float, key: String) -> HSlider:
	var slider := HSlider.new()
	slider.min_value = from
	slider.max_value = to
	slider.step = step
	slider.custom_minimum_size = Vector2(240, 24)
	slider.value_changed.connect(func(value: float) -> void: _change(key, value))
	return slider
