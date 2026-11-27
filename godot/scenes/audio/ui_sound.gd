extends Node
## Interface sounds (prompt 029): a small pool of non-positional players on
## the UI bus, a click for every button of every screen, and the mute while
## the window has no focus.
##
## One instance lives under the scene tree's root for the whole run
## (`install()`, called by `menu/settings.gd` when the settings are
## applied), so menus, matches, lobbies and the editor share it without an
## autoload. Other scripts play through the static `play(class)`.
##
## Signals (prompt 192): sirens and alarms (`SoundClasses.SIGNAL_CLASSES`,
## the rocket strike's siren of `strike_view.gd`) share one player for the
## whole run, so at most one signal sounds at a time in every scene. A
## trigger while one sounds starts nothing (no restart mid-tone); a held
## signal (`hold_signal`, looped) keeps looping while anyone holds it and,
## released, fades out at the end of its cycle. A held signal waits for a
## sounding one-shot alarm to end. Two alarms or sirens at once only beat
## against each other and say nothing new; battle sounds overlap as before.
##
## Announcements (prompt 244): spoken messages ("We are under attack!",
## `battle_sound.announce`) have one player of their own (`speak`), so never
## two speak at once; one asked for while another speaks is left out. They
## are no signals: a message is spoken over a sounding siren.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const SoundClasses := preload("res://scenes/audio/sound_classes.gd")

## A released siren fades out over this long before its cycle ends (s);
## with less than SIGNAL_MIN_TAIL left it runs one more cycle.
const SIGNAL_FADE := 0.6
const SIGNAL_MIN_TAIL := 0.3

static var _instance: Node

## Sounds started and dropped (pool busy with more important ones), per
## class (self-test, gallery).
var played: Dictionary[String, int] = {}
var dropped := 0
## Signals started, triggers absorbed by a sounding signal, holds
## released with a fade (self-test).
var signals_started: Dictionary[String, int] = {}
var signals_absorbed := 0
var signals_faded := 0
## Announcements spoken and left out because one was speaking (self-test).
var spoken := 0
var speech_dropped := 0

var _players: Array[AudioStreamPlayer] = []
## Per player: priority and time when it is free again.
var _priority: PackedInt32Array = []
var _ends: PackedFloat64Array = []
var _time := 0.0
## The one signal player, what it plays ("" idle), whether that is a held
## loop, the holders (key -> {stream, volume_db, bus}), the fade left (s,
## negative: not releasing) and every stream it ever played (self-test).
var _signal: AudioStreamPlayer
var _signal_class := ""
var _signal_held := false
var _signal_db := 0.0
var _holds: Dictionary[String, Dictionary] = {}
var _release_left := -1.0
var _signal_streams: Array[AudioStream] = []
## The one player of spoken announcements.
var _speech: AudioStreamPlayer


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


## Plays a UI sound class (e.g. "ui_click"). Returns whether it started
## (a signal class also counts as played while another signal sounds).
static func play(sound_class: String) -> bool:
	var node := instance()
	if node == null:
		return false
	if SoundClasses.is_signal(sound_class):
		return node.play_signal(sound_class)
	return node.play_class(sound_class)


## Speaks an announcement (a keyed name like `announce@de/under_attack`).
## Returns whether it started: not without files or while one speaks.
static func speak(sound_class: String) -> bool:
	var node := instance()
	return node != null and node.speak_class(sound_class)


## Holds (`on`) or releases the looped signal `key` (e.g. the rocket
## strike's siren): it sounds while at least one key holds it.
static func hold_signal(key: String, on: bool, stream: AudioStream = null, volume_db := 0.0,
		bus := &"Effects") -> void:
	var node := instance()
	if node != null:
		node.set_hold(key, on, stream, volume_db, bus)


func _ready() -> void:
	var voices: int = SoundLibrary.groups().get("ui", {}).get("voices", 6)
	for i in voices:
		var player := AudioStreamPlayer.new()
		player.bus = "UI"
		add_child(player)
		_players.append(player)
		_priority.append(0)
		_ends.append(0.0)
	_signal = AudioStreamPlayer.new()
	_signal.name = "Signal"
	# Pauses with the match like the strike siren did under `main`.
	_signal.process_mode = Node.PROCESS_MODE_PAUSABLE
	add_child(_signal)
	_speech = AudioStreamPlayer.new()
	_speech.name = "Speech"
	_speech.bus = "UI"
	_speech.process_mode = Node.PROCESS_MODE_PAUSABLE
	add_child(_speech)
	get_tree().node_added.connect(_on_node_added)
	_hook_buttons(get_tree().root)


func _exit_tree() -> void:
	if _instance == self:
		_instance = null
	SoundBuses.release(_players)
	SoundBuses.release([_signal, _speech])
	SoundLibrary.clear_cache()


func _process(delta: float) -> void:
	_time += delta
	_update_signal(delta)


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


## A one-shot signal class (alarm): starts unless a signal sounds.
func play_signal(sound_class: String) -> bool:
	if _signal == null:
		return false
	if _signal.playing:
		signals_absorbed += 1
		return true
	var stream := SoundLibrary.pick(sound_class)
	if stream == null:
		return false
	var spec := SoundLibrary.spec(sound_class)
	_start_signal(sound_class, stream, spec.get("volume_db", 0.0), spec.get("bus", "UI"), false)
	played[sound_class] = played.get(sound_class, 0) + 1
	return true


func speak_class(sound_class: String) -> bool:
	if _speech == null:
		return false
	var stream := SoundLibrary.pick(sound_class)
	if stream == null:
		return false
	if _speech.playing:
		speech_dropped += 1
		return false
	var spec := SoundLibrary.spec(sound_class)
	_speech.stream = stream
	_speech.volume_db = spec.get("volume_db", 0.0)
	_speech.bus = spec.get("bus", "UI")
	_speech.play()
	spoken += 1
	played[sound_class] = played.get(sound_class, 0) + 1
	return true


## Whether an announcement is being spoken (self-test).
func speaking() -> bool:
	return _speech != null and _speech.playing


func set_hold(key: String, on: bool, stream: AudioStream, volume_db: float, bus: StringName) -> void:
	if not on:
		_holds.erase(key)
	elif stream != null:
		_holds[key] = {"stream": stream, "volume_db": volume_db, "bus": bus}
	if is_inside_tree():
		_update_signal(0.0)


## Whether a signal sounds, and what (self-test, `blast_shots.gd`).
func signal_playing() -> String:
	return _signal_class if _signal != null and _signal.playing else ""


## Every stream the signal player played, and the signal classes' files:
## a player anywhere playing one of these plays a signal (self-test).
func signal_streams() -> Array[AudioStream]:
	var all: Array[AudioStream] = _signal_streams.duplicate()
	for name: String in SoundClasses.SIGNAL_CLASSES:
		for stream: AudioStream in SoundLibrary.streams(name):
			if not all.has(stream):
				all.append(stream)
	return all


func _start_signal(sound_class: String, stream: AudioStream, volume_db: float, bus: StringName,
		held: bool) -> void:
	_signal.stream = stream
	_signal.volume_db = volume_db
	_signal.bus = bus if AudioServer.get_bus_index(bus) >= 0 else &"Master"
	_signal.pitch_scale = 1.0
	_signal.play()
	_signal_class = sound_class
	_signal_held = held
	_signal_db = volume_db
	_release_left = -1.0
	if not _signal_streams.has(stream):
		_signal_streams.append(stream)
	signals_started[sound_class] = signals_started.get(sound_class, 0) + 1


## Starts a held signal when the player is free, keeps it looping while
## held and fades it out at the end of its cycle once nobody holds it.
func _update_signal(delta: float) -> void:
	if _signal == null or not _signal.can_process():
		return
	if not _signal.playing:
		_signal_class = ""
		_release_left = -1.0
		if not _holds.is_empty():
			var keys: Array = _holds.keys()
			keys.sort()
			var h: Dictionary = _holds[keys[0]]
			_start_signal(keys[0], h.stream, h.volume_db, h.bus, true)
		return
	if not _signal_held:
		return
	if not _holds.is_empty():
		if _release_left >= 0.0:
			_release_left = -1.0
			_signal.volume_db = _signal_db
		return
	if _release_left < 0.0:
		var length := _signal.stream.get_length()
		_release_left = length - fmod(_signal.get_playback_position(), length)
		if _release_left < SIGNAL_MIN_TAIL:
			_release_left += length
		signals_faded += 1
	_release_left -= delta
	if _release_left <= 0.0:
		_signal.stop()
		_signal.volume_db = _signal_db
		_signal_class = ""
		_release_left = -1.0
		return
	_signal.volume_db = _signal_db + linear_to_db(clampf(_release_left / SIGNAL_FADE, 0.001, 1.0))


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
