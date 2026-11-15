extends Node
## Interface sounds (prompt 029): a small pool of non-positional players on
## the UI bus, a click for every button of every screen, and the mute while
## the window has no focus.
##
## One instance lives under the scene tree's root for the whole run
## (`install()`, called by `menu/settings.gd` when the settings are
## applied), so menus, matches, lobbies and the editor share it without an
## autoload. Other scripts play through the static `play(class)`.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")

static var _instance: Node

## Sounds started and dropped (pool busy with more important ones), per
## class (self-test, gallery).
var played: Dictionary[String, int] = {}
var dropped := 0

var _players: Array[AudioStreamPlayer] = []
## Per player: priority and time when it is free again.
var _priority: PackedInt32Array = []
var _ends: PackedFloat64Array = []
var _time := 0.0


## Adds the shared node to the tree once.
static func install() -> void:
	if is_instance_valid(_instance):
		return
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return
	SoundBuses.ensure()
	var node: Node = load("res://scenes/audio/ui_sound.gd").new()
	node.name = "UiSound"
	node.process_mode = Node.PROCESS_MODE_ALWAYS
	_instance = node
	tree.root.add_child.call_deferred(node)


## The shared node (null before `install`).
static func instance() -> Node:
	return _instance if is_instance_valid(_instance) else null


## Plays a UI sound class (e.g. "ui_click"). Returns whether it started.
static func play(sound_class: String) -> bool:
	var node := instance()
	return node != null and node.play_class(sound_class)


func _ready() -> void:
	var voices: int = SoundLibrary.groups().get("ui", {}).get("voices", 6)
	for i in voices:
		var player := AudioStreamPlayer.new()
		player.bus = "UI"
		add_child(player)
		_players.append(player)
		_priority.append(0)
		_ends.append(0.0)
	get_tree().node_added.connect(_on_node_added)
	_hook_buttons(get_tree().root)


func _exit_tree() -> void:
	if _instance == self:
		_instance = null
	SoundBuses.release(_players)
	SoundLibrary.clear_cache()


func _process(delta: float) -> void:
	_time += delta


func _notification(what: int) -> void:
	match what:
		NOTIFICATION_APPLICATION_FOCUS_OUT:
			SoundBuses.set_focused(false)
		NOTIFICATION_APPLICATION_FOCUS_IN:
			SoundBuses.set_focused(true)


func play_class(sound_class: String) -> bool:
	var spec := SoundLibrary.spec(sound_class)
	var stream := SoundLibrary.pick(sound_class)
	if stream == null:
		return false
	var priority: int = spec.get("priority", 1)
	# A free player, else the one with the lowest priority (oldest first).
	var best := -1
	for i in _players.size():
		if _time >= _ends[i]:
			best = i
			break
		if best < 0 or _priority[i] < _priority[best] or (_priority[i] == _priority[best] and _ends[i] < _ends[best]):
			best = i
	if best < 0 or (_time < _ends[best] and _priority[best] > priority):
		dropped += 1
		return false
	var player := _players[best]
	var spread: float = spec.get("pitch_spread", 0.0)
	player.stream = stream
	player.volume_db = spec.get("volume_db", 0.0)
	player.pitch_scale = 1.0 + randf_range(-spread, spread)
	player.bus = spec.get("bus", "UI")
	player.play()
	_priority[best] = priority
	_ends[best] = _time + stream.get_length() / player.pitch_scale
	played[sound_class] = played.get(sound_class, 0) + 1
	return true


func _on_node_added(node: Node) -> void:
	if node is BaseButton:
		_hook(node)


func _hook_buttons(node: Node) -> void:
	if node is BaseButton:
		_hook(node)
	for child in node.get_children():
		_hook_buttons(child)


func _hook(button: BaseButton) -> void:
	if button.has_meta(&"ui_sound"):
		return
	button.set_meta(&"ui_sound", true)
	button.pressed.connect(func() -> void: play_class("ui_click"))
