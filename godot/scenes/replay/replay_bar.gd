extends CanvasLayer
## Controls of the replay viewer (prompt 038), added by `main.gd` while a
## replay plays: play/pause, speeds 1×/2×/4×/8×, a timeline to jump, the
## perspective (one player with that player's fog of war, or everything
## visible) and back to the menu.
##
## A jump or a new perspective rebuilds `main.tscn` from the replay's
## nearest earlier snapshot (`GameFiles.watch_replay`), because terrain,
## craters and map objects are built from the simulation at the start.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")

var main: Node3D

var play_button: Button
var speed_buttons: Array[Button] = []
var timeline: HSlider
var time_label: Label
var view_option: OptionButton
var exit_button: Button
## Player id per entry of `view_option` (-1: everything visible).
var view_ids: Array[int] = []

var _ticks := 0
var _dragging := false


func _ready() -> void:
	layer = 40
	var root := PanelContainer.new()
	root.theme = MenuTheme.get_theme()
	root.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
	root.offset_top = -64
	add_child(root)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	root.add_child(row)
	var label := Label.new()
	label.text = "REPLAY_LABEL"
	label.add_theme_color_override("font_color", MenuTheme.ACCENT)
	row.add_child(label)
	play_button = _button(row, "REPLAY_PAUSE", toggle_pause)
	for speed in GameFiles.SPEEDS:
		var button := _button(row, "%d×" % int(speed), func() -> void: set_speed(speed))
		button.toggle_mode = true
		speed_buttons.append(button)
	var status: Dictionary = main.sim_host.replay_status()
	_ticks = status["ticks"]
	timeline = HSlider.new()
	timeline.min_value = 0
	timeline.max_value = maxi(_ticks, 1)
	timeline.step = 1
	timeline.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	timeline.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	timeline.drag_started.connect(func() -> void: _dragging = true)
	timeline.drag_ended.connect(func(changed: bool) -> void:
		_dragging = false
		if changed:
			jump_to(int(timeline.value)))
	row.add_child(timeline)
	time_label = Label.new()
	time_label.custom_minimum_size = Vector2(110, 0)
	row.add_child(time_label)
	view_option = OptionButton.new()
	view_option.add_item("REPLAY_VIEW_ALL")
	view_ids.append(-1)
	for player: Dictionary in GameFiles.replay_player.info()["players"]:
		view_option.add_item(GameFiles.faction_name(player["faction"], player["id"]))
		view_ids.append(player["id"])
	view_option.select(maxi(view_ids.find(status["view"]), 0))
	view_option.item_selected.connect(func(index: int) -> void: set_view(view_ids[index]))
	row.add_child(view_option)
	exit_button = _button(row, "REPLAY_EXIT", leave)
	_show_speed()


func _process(_delta: float) -> void:
	var status: Dictionary = main.sim_host.replay_status()
	if status.is_empty():
		return
	var tick: int = status["tick"]
	if not _dragging:
		timeline.set_value_no_signal(tick)
	var rate := maxi(main.sim_host.tick_rate(), 1)
	time_label.text = "%s / %s" % [_clock(tick / rate), _clock(_ticks / rate)]
	if status["ended"]:
		play_button.text = "REPLAY_ENDED"


func toggle_pause() -> void:
	GameFiles.replay_paused = not GameFiles.replay_paused
	_show_speed()


func set_speed(speed: float) -> void:
	GameFiles.replay_speed = speed
	GameFiles.replay_paused = false
	_show_speed()


## Rebuilds the match at `tick` (the timeline).
func jump_to(tick: int) -> void:
	_rebuild(clampi(tick, 0, _ticks), view_ids[view_option.selected])


## Shows the match as `player` sees it (-1: everything) from the current
## tick on.
func set_view(player: int) -> void:
	_rebuild(main.sim_host.current_tick(), player)


func leave() -> void:
	GameFiles.replay_player = null
	get_tree().paused = false
	get_tree().change_scene_to_file(MatchSetup.MENU_SCENE)


func _rebuild(tick: int, view: int) -> void:
	print("Replay: jump to tick %d, view %d" % [tick, view])
	GameFiles.watch_replay(tick, view)
	get_tree().paused = false
	get_tree().change_scene_to_file(MatchSetup.MAIN_SCENE)


func _show_speed() -> void:
	main.sim_host.speed = 0.0 if GameFiles.replay_paused else GameFiles.replay_speed
	play_button.text = "REPLAY_PLAY" if GameFiles.replay_paused else "REPLAY_PAUSE"
	for i in speed_buttons.size():
		speed_buttons[i].set_pressed_no_signal(not GameFiles.replay_paused and is_equal_approx(GameFiles.SPEEDS[i], GameFiles.replay_speed))


static func _clock(seconds: int) -> String:
	return "%d:%02d" % [seconds / 60, seconds % 60]


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(56, 40)
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(action)
	parent.add_child(button)
	return button
