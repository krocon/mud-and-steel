extends Node3D
## Sound gallery (prompt 029): plays every sound class once, one after the
## other, then waits; a click on a class in the list plays it again (a new
## random variant each time). Start with `just sound-gallery` (the main
## menu forwards `-- --sound-gallery` here). `--sound-gallery-once` quits
## after the round with exit code 1 if a class has no loadable file
## (headless check).
##
## Positional sounds play 40 m in front of the listener, slightly to the
## left or right, so panning is audible; loops play for `LOOP_SECONDS`.
## UI sounds play through `audio/ui_sound.gd` on the UI bus.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")

const DISTANCE_M := 40.0
const LOOP_SECONDS := 4.0
const GAP_SECONDS := 0.4

var _names: Array[String] = []
var _list: ItemList
var _label: Label
var _player: AudioStreamPlayer3D
var _once := "--sound-gallery-once" in OS.get_cmdline_user_args()


func _ready() -> void:
	Settings.ensure_loaded()
	SoundBuses.ensure()
	var camera := Camera3D.new()
	add_child(camera)
	camera.position = Vector3(0, 2, 0)
	var listener := AudioListener3D.new()
	add_child(listener)
	listener.make_current()
	_player = AudioStreamPlayer3D.new()
	_player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
	add_child(_player)
	for name: String in SoundLibrary.classes():
		_names.append(name)
	_build_ui()
	var problems := SoundLibrary.check()
	for problem in problems:
		push_error(problem)
	await _play_all()
	if _once:
		# The audio thread lets go of stopped streams on its next mix.
		await get_tree().create_timer(0.2).timeout
		print("sound gallery: %d classes played, %d problems" % [_names.size(), problems.size()])
		get_tree().quit(0 if problems.is_empty() else 1)


func _build_ui() -> void:
	var layer := CanvasLayer.new()
	add_child(layer)
	var panel := PanelContainer.new()
	panel.theme = MenuTheme.get_theme()
	panel.set_anchors_and_offsets_preset(Control.PRESET_LEFT_WIDE)
	panel.custom_minimum_size = Vector2(380, 0)
	layer.add_child(panel)
	var box := VBoxContainer.new()
	panel.add_child(box)
	box.add_child(MenuTheme.title("Sound gallery", MenuTheme.HEADING_SIZE))
	_label = Label.new()
	_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_label)
	_list = ItemList.new()
	_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	for name in _names:
		var spec := SoundLibrary.spec(name)
		_list.add_item("%s  (%d files, %s)" % [name, SoundLibrary.streams(name).size(), spec.get("group", "")])
	_list.item_clicked.connect(func(index: int, _at: Vector2, _button: int) -> void: play(_names[index]))
	box.add_child(_list)


func _play_all() -> void:
	for i in _names.size():
		_list.select(i)
		var seconds := play(_names[i])
		await get_tree().create_timer(seconds + GAP_SECONDS).timeout
	_player.stop()
	_label.text = "Done. Click a class to hear it again."


## Plays one class; returns how long it lasts.
func play(sound_class: String) -> float:
	var spec := SoundLibrary.spec(sound_class)
	var stream := SoundLibrary.pick(sound_class)
	_label.text = "%s\nvolume %.0f dB, range %s m, priority %s" % [
		sound_class, spec.get("volume_db", 0.0), spec.get("range_m", "–"), spec.get("priority", 0)
	]
	if stream == null:
		return 0.0
	if spec.get("bus", "Effects") == "UI":
		_player.stop()
		UiSound.play(sound_class)
		return stream.get_length()
	_player.stop()
	_player.stream = stream
	_player.bus = spec.get("bus", "Effects")
	_player.volume_db = spec.get("volume_db", 0.0)
	_player.unit_size = float(spec.get("ref_m", 10.0))
	_player.max_distance = float(spec.get("range_m", 300.0)) * 2.0
	var side := randf_range(-15.0, 15.0)
	_player.position = Vector3(side, 0.0, -DISTANCE_M)
	_player.play()
	return LOOP_SECONDS if spec.get("loop", false) else stream.get_length()
