extends Node
## Photo mode (prompt 103), single player and replays: `photo_mode` (F2)
## stops the game (the scene tree, like the tactical pause), hides the HUD
## and every mark (`view_toggles.gd`), frees the camera (closer, flatter and
## steeper than in the game, a field of view), offers the depth of field
## modes and, in a replay or after the match is decided, the fog of war off,
## and saves the picture as a PNG in `user://photos/`. Leaving (the key
## again, the button or Esc) restores camera, settings and fog exactly.
## Not in network matches.

const Settings := preload("res://scenes/menu/settings.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")

const PHOTO_DIR := "user://photos"
## Free camera of the photo mode.
const MIN_DISTANCE := 4.0
const TILT_MIN_DEG := 4.0
const TILT_MAX_DEG := 89.0
const FOV_RANGE := Vector2(12.0, 70.0)
const DOF_CYCLE: Array[String] = ["off", "subtle", "miniature"]

var main: Node3D
var active := false
## Path of the last saved picture ("" if none).
var last_path := ""
var _layer: CanvasLayer
var _panel: PanelContainer
var _fov: HSlider
var _dof_button: Button
var _fog_box: CheckBox
var _saved_label: Label
var _saved := {}


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "PhotoMode"
	process_mode = Node.PROCESS_MODE_ALWAYS
	_layer = CanvasLayer.new()
	_layer.layer = 45
	_layer.visible = false
	add_child(_layer)
	_build_panel()


## Single player or a replay; never in a network match.
func available() -> bool:
	return not main.sim_host.is_networked()


## The fog may go: in a replay or after the match is decided.
func fog_allowed() -> bool:
	var host: SimHost = main.sim_host
	return host.is_replay() or bool(host.match_result().get("decided", false))


func toggle() -> void:
	if active:
		leave()
	else:
		enter()


func enter() -> void:
	if active or not available():
		return
	var rig: Node3D = main.camera_rig
	active = true
	_saved = {
		"paused": get_tree().paused, "rig_mode": rig.process_mode,
		"min": rig.min_distance, "tilt_min": rig.tilt_min_deg, "tilt_max": rig.tilt_max_deg,
		"fov": rig.fov_deg, "dof": Settings.get_value("depth_of_field"),
		"fog": main.fog_of_war.enabled if main.get("fog_of_war") != null else true,
		"hidden": main.view_toggles.hidden,
		"focus": rig.focus, "distance": rig.target_distance, "yaw": rig.target_yaw, "tilt": rig.target_tilt,
	}
	get_tree().paused = true
	rig.process_mode = Node.PROCESS_MODE_ALWAYS
	rig.min_distance = MIN_DISTANCE
	rig.tilt_min_deg = TILT_MIN_DEG
	rig.tilt_max_deg = TILT_MAX_DEG
	main.view_toggles.set_hidden(true)
	_fov.set_value_no_signal(rig.fov_deg)
	_fog_box.visible = fog_allowed()
	_fog_box.set_pressed_no_signal(false)
	_saved_label.text = ""
	_update_dof_label()
	_layer.visible = true


func leave() -> void:
	if not active:
		return
	var rig: Node3D = main.camera_rig
	active = false
	_layer.visible = false
	rig.min_distance = _saved.min
	rig.tilt_min_deg = _saved.tilt_min
	rig.tilt_max_deg = _saved.tilt_max
	_set_fov(_saved.fov)
	Settings.set_value("depth_of_field", _saved.dof)
	if main.get("fog_of_war") != null:
		main.fog_of_war.set_fog_enabled(_saved.fog)
	rig.focus = _saved.focus
	rig.target_distance = _saved.distance
	rig.distance = _saved.distance
	rig.target_yaw = _saved.yaw
	rig.yaw = _saved.yaw
	rig.target_tilt = _saved.tilt
	rig.tilt = _saved.tilt
	rig.process_mode = _saved.rig_mode
	main.view_toggles.set_hidden(_saved.hidden)
	get_tree().paused = _saved.paused


## Saves the picture without the panel; returns the file's path.
func save_photo() -> String:
	DirAccess.make_dir_recursive_absolute(PHOTO_DIR)
	var stamp := Time.get_datetime_string_from_system().replace(":", "-").replace("T", "_")
	var path := "%s/photo_%s.png" % [PHOTO_DIR, stamp]  # l10n-ignore (file name)
	_layer.visible = false
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var error := image.save_png(path)
	_layer.visible = active
	last_path = ProjectSettings.globalize_path(path) if error == OK else ""
	_saved_label.text = tr("PHOTO_SAVED") % last_path if error == OK else tr("PHOTO_SAVE_FAILED")
	return last_path


func _input(event: InputEvent) -> void:
	if not active:
		return
	var key := event as InputEventKey
	if key != null and key.pressed and not key.echo and key.keycode == KEY_ESCAPE:
		leave()
		get_viewport().set_input_as_handled()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("photo_mode", false, true) and available():
		toggle()
		get_viewport().set_input_as_handled()


func _set_fov(value: float) -> void:
	var rig: Node3D = main.camera_rig
	rig.fov_deg = value
	rig.camera.fov = value
	rig._apply()


func _cycle_dof() -> void:
	var now: String = Settings.get_value("depth_of_field")
	Settings.set_value("depth_of_field", DOF_CYCLE[(DOF_CYCLE.find(now) + 1) % DOF_CYCLE.size()])
	_update_dof_label()


func _update_dof_label() -> void:
	_dof_button.text = tr("PHOTO_DOF") % tr("PHOTO_DOF_" + str(Settings.get_value("depth_of_field")).to_upper())


func _build_panel() -> void:
	_panel = PanelContainer.new()
	_panel.theme = MenuTheme.get_theme()
	_panel.add_theme_stylebox_override(&"panel", HudTheme.panel_box())
	_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM)
	_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	_panel.offset_bottom = -12
	_layer.add_child(_panel)
	var box := VBoxContainer.new()
	_panel.add_child(box)
	var row := HBoxContainer.new()
	row.add_theme_constant_override(&"separation", 10)
	box.add_child(row)
	var title := Label.new()
	title.text = "PHOTO_TITLE"
	title.add_theme_color_override(&"font_color", HudTheme.GOLD)
	row.add_child(title)
	var fov_label := Label.new()
	fov_label.text = "PHOTO_FOV"
	row.add_child(fov_label)
	_fov = HSlider.new()
	_fov.min_value = FOV_RANGE.x
	_fov.max_value = FOV_RANGE.y
	_fov.step = 1.0
	_fov.custom_minimum_size = Vector2(160, 20)
	_fov.value_changed.connect(_set_fov)
	row.add_child(_fov)
	_dof_button = Button.new()
	_dof_button.focus_mode = Control.FOCUS_NONE
	_dof_button.pressed.connect(_cycle_dof)
	row.add_child(_dof_button)
	_fog_box = CheckBox.new()
	_fog_box.text = "PHOTO_NO_FOG"
	_fog_box.focus_mode = Control.FOCUS_NONE
	_fog_box.toggled.connect(func(on: bool) -> void:
		if fog_allowed() and main.get("fog_of_war") != null:
			main.fog_of_war.set_fog_enabled(not on))
	row.add_child(_fog_box)
	var save := Button.new()
	save.text = "PHOTO_SAVE"
	save.focus_mode = Control.FOCUS_NONE
	save.pressed.connect(save_photo)
	row.add_child(save)
	var close := Button.new()
	close.text = "PHOTO_LEAVE"
	close.focus_mode = Control.FOCUS_NONE
	close.pressed.connect(leave)
	row.add_child(close)
	var hint := Label.new()
	hint.text = "PHOTO_HINT"
	hint.add_theme_color_override(&"font_color", HudTheme.TEXT_DIM)
	box.add_child(hint)
	_saved_label = Label.new()
	_saved_label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	box.add_child(_saved_label)
