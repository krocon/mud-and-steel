extends PanelContainer
## Settings screen (prompt 027), used by the main menu and the pause menu.
##
## Tabs: Graphics (quality level), Controls (keyboard bindings of the camera),
## Audio (volume per bus, mute without focus), Camera (pan and zoom
## sensitivity, edge scrolling), Interface (health bars, prompt 031).
## Changes apply at once; "Save" writes them to the settings
## file, "Cancel" reloads the file. Emits `changed` after every change (the
## match re-applies quality and camera) and `closed` when the screen closes.

signal changed
signal closed

const Settings := preload("res://scenes/menu/settings.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
## Bus volume sliders below the master volume: settings key -> label.
const BUS_SLIDERS: Dictionary[String, String] = {
	"effects_volume": "Effects", "ui_volume": "Interface", "music_volume": "Music", "ambience_volume": "Ambience",
}

var quality_option: OptionButton
var volume_slider: HSlider
## Volume sliders of the other buses (prompt 029), by settings key.
var bus_sliders: Dictionary[String, HSlider] = {}
var mute_unfocused_box: CheckBox
var pan_slider: HSlider
var zoom_slider: HSlider
var edge_scroll_box: CheckBox
var health_bar_option: OptionButton
var save_button: Button
var cancel_button: Button
## Action whose key is being captured, or "".
var capturing := ""

var _key_buttons: Dictionary[String, Button] = {}


func _ready() -> void:
	Settings.ensure_loaded()
	theme = MenuTheme.get_theme()
	custom_minimum_size = Vector2(620, 0)
	var box := VBoxContainer.new()
	add_child(box)
	box.add_child(MenuTheme.title("Settings", MenuTheme.HEADING_SIZE))
	var tabs := TabContainer.new()
	tabs.custom_minimum_size = Vector2(0, 330)
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
	defaults.text = "Defaults"
	defaults.pressed.connect(func() -> void:
		Settings.reset()
		_refresh()
		changed.emit())
	buttons.add_child(defaults)
	cancel_button = Button.new()
	cancel_button.text = "Cancel"
	cancel_button.pressed.connect(cancel)
	buttons.add_child(cancel_button)
	save_button = Button.new()
	save_button.text = "Save"
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
	_key_buttons[action].text = "Press a key …"


func _input(event: InputEvent) -> void:
	if capturing.is_empty() or not visible:
		return
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo:
		return
	get_viewport().set_input_as_handled()
	if key.physical_keycode != KEY_ESCAPE:
		Settings.bind(capturing, key.physical_keycode)
		changed.emit()
	capturing = ""
	_refresh()


func _graphics_tab() -> Control:
	var tab := _tab("Graphics")
	quality_option = OptionButton.new()
	for level in GraphicsQuality.LEVELS:
		quality_option.add_item(level.capitalize())
	quality_option.item_selected.connect(func(index: int) -> void:
		_change("quality", GraphicsQuality.LEVELS[index]))
	tab.add_child(_row("Quality", quality_option))
	tab.add_child(MenuTheme.hint("Low: hard shadows, no ambient occlusion, no snowfall. Medium: soft shadows. High: finest shadows and anti-aliasing. `--quality=<level>` on the command line overrides this."))
	return tab


func _controls_tab() -> Control:
	# Camera and order keys (prompt 031) do not fit without scrolling.
	var scroll := ScrollContainer.new()
	scroll.name = "Controls"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var tab := _tab("Controls")
	tab.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(tab)
	for action in Settings.BINDABLE:
		var button := Button.new()
		button.custom_minimum_size = Vector2(160, 0)
		button.pressed.connect(capture.bind(action))
		_key_buttons[action] = button
		tab.add_child(_row(Settings.BINDABLE[action], button))
	tab.add_child(MenuTheme.hint("Click a binding and press the new key (Esc cancels). Arrow keys also pan; the mouse wheel zooms and the middle button drags the map. Groups: Ctrl+1…9 stores, 1…9 recalls."))
	return scroll


func _audio_tab() -> Control:
	var tab := _tab("Audio")
	volume_slider = _slider(0.0, 1.0, 0.05, "master_volume")
	tab.add_child(_row("Master volume", volume_slider))
	for key: String in BUS_SLIDERS:
		bus_sliders[key] = _slider(0.0, 1.0, 0.05, key)
		tab.add_child(_row(BUS_SLIDERS[key], bus_sliders[key]))
	mute_unfocused_box = CheckBox.new()
	mute_unfocused_box.text = "Mute when the window is in the background"
	mute_unfocused_box.toggled.connect(func(on: bool) -> void: _change("mute_unfocused", on))
	tab.add_child(mute_unfocused_box)
	tab.add_child(MenuTheme.hint("Effects: weapons, impacts, engines. Interface: clicks and notices. Music and ambience come with a later update."))
	return tab


func _camera_tab() -> Control:
	var tab := _tab("Camera")
	pan_slider = _slider(0.25, 3.0, 0.05, "pan_sensitivity")
	tab.add_child(_row("Pan speed", pan_slider))
	zoom_slider = _slider(0.25, 3.0, 0.05, "zoom_sensitivity")
	tab.add_child(_row("Zoom speed", zoom_slider))
	edge_scroll_box = CheckBox.new()
	edge_scroll_box.text = "Pan at the screen edge"
	edge_scroll_box.toggled.connect(func(on: bool) -> void: _change("edge_scroll", on))
	tab.add_child(edge_scroll_box)
	return tab


func _interface_tab() -> Control:
	var tab := _tab("Interface")
	health_bar_option = OptionButton.new()
	for label in ["Damaged and selected units", "Selected units only", "Never"]:
		health_bar_option.add_item(label)
	health_bar_option.item_selected.connect(func(index: int) -> void:
		_change("health_bars", Settings.HEALTH_BAR_MODES[index]))
	tab.add_child(_row("Health bars", health_bar_option))
	return tab


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
	for action in _key_buttons:
		_key_buttons[action].text = Settings.key_name(Settings.bindings.get(action, 0))


func _change(key: String, value: Variant) -> void:
	Settings.set_value(key, value)
	changed.emit()


func _tab(title: String) -> VBoxContainer:
	var tab := VBoxContainer.new()
	tab.name = title
	return tab


func _row(text: String, control: Control) -> Control:
	var row := HBoxContainer.new()
	var label := Label.new()
	label.text = text
	label.custom_minimum_size = Vector2(200, 0)
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
