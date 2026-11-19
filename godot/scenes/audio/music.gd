extends Node
## Music (prompt 030): one node under the scene tree's root for the whole
## run, so a piece keeps playing across scene changes (menu → loading
## screen → match → result → menu) and changes only when the mood does.
##
## Pieces are `assets/audio/music/<mood>_NN.mp3` (made by
## `tools/audio/music.py`, `just music`). `priv_<mood>_*` pieces (private
## sound repository) replace the public pieces of their mood. MP3 is decoded while it plays;
## only the compressed file is in memory. Moods:
##
## | Mood | Who sets it | After a piece |
## |---|---|---|
## | `menu` | `menu/main_menu.gd` | next menu piece right away |
## | `calm` | `audio/match_music.gd` | 20–60 s of silence (only ambience), then the next |
## | `combat` | `audio/match_music.gd` | next combat piece right away |
## | `victory`, `defeat` | `audio/match_music.gd` | silence |
##
## A mood change cross-fades between two players on the `Music` bus.
## Pieces fade out at their end themselves and are not looped; the next one
## starts a little before the end of the last. `-- --no-sound` keeps the
## music silent like every other sound. Static entry points like
## `ui_sound.gd`, no autoload.

const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const DIR := "res://assets/audio/music/"
const PRIVATE_PREFIX := "priv_"
const EXTENSIONS: Array[String] = ["mp3", "ogg", "wav"]
## Level of the music under the bus slider: music sits below the battle.
## [provisional] Set without listening.
const MUSIC_DB := -8.0
const SILENT_DB := -60.0
## Seconds of cross-fade between moods, and between two pieces of a mood.
const MOOD_FADE := 4.0
const PIECE_OVERLAP := 2.0
## Silence between two calm pieces (s).
const CALM_GAP: Vector2 = Vector2(20.0, 60.0)
## Moods whose pieces play once, then silence.
const ONCE: Array[String] = ["victory", "defeat"]
const MOODS: Array[String] = ["menu", "calm", "combat", "victory", "defeat"]

static var _instance: Node
## Mood asked for before the node is in the tree.
static var _pending := ""
static var _tracks: Dictionary[String, Array] = {}

## Current mood ("" = none) and every mood change since the start
## (self-test).
var mood := ""
var history: PackedStringArray = []
## Pieces started, and the path of the current one.
var pieces_started := 0
var current_piece := ""

var _players: Array[AudioStreamPlayer] = []
var _active := 0
## Per player: target volume (dB) and dB change per second.
var _target: PackedFloat32Array = [SILENT_DB, SILENT_DB]
var _rate: PackedFloat32Array = [0.0, 0.0]
var _time := 0.0
## When the current piece ends (s, own clock), when the next starts.
var _piece_ends := INF
var _next_at := INF
var _last_path := ""


## Adds the shared node to the tree once (idempotent).
static func install() -> void:
	if is_instance_valid(_instance):
		return
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return
	SoundBuses.ensure()
	var node: Node = load("res://scenes/audio/music.gd").new()
	node.name = "Music"
	node.process_mode = Node.PROCESS_MODE_ALWAYS
	_instance = node
	tree.root.add_child.call_deferred(node)


## The shared node (null before `install`).
static func instance() -> Node:
	return _instance if is_instance_valid(_instance) else null


## Switches to `mood` (one of MOODS) with a cross-fade; nothing happens if
## it already plays. Installs the node if needed.
static func play(new_mood: String) -> void:
	if "--no-sound" in OS.get_cmdline_user_args():
		return
	install()
	var node := instance()
	if node != null and node.is_inside_tree():
		node.set_mood(new_mood)
	else:
		_pending = new_mood


## Fades the music out (draw, tools).
static func stop() -> void:
	var node := instance()
	if node != null and node.is_inside_tree():
		node.set_mood("")
	_pending = ""


## The mood playing now ("" = silence or not installed).
static func current_mood() -> String:
	var node := instance()
	return node.mood if node != null else ""


## The pieces of a mood, loaded (empty if there are none).
static func tracks(of_mood: String) -> Array:
	if _tracks.has(of_mood):
		return _tracks[of_mood]
	var list: Array = []
	var files := PackedStringArray()
	var private := PackedStringArray()
	for file in ResourceLoader.list_directory(DIR):
		if file.get_extension().to_lower() not in EXTENSIONS:
			continue
		var is_private := file.begins_with(PRIVATE_PREFIX)
		var stem := file.get_basename().trim_prefix(PRIVATE_PREFIX) if is_private else file.get_basename()
		if stem.rsplit("_", true, 1)[0] == of_mood:
			files.append(file)
			if is_private:
				private.append(file)
	# Private pieces (`priv_<mood>_*`, from the private sound repository,
	# never in git) replace the public pieces of their mood where they exist.
	if not private.is_empty():
		files = private
	files.sort()
	for file in files:
		var stream := load(DIR + file) as AudioStream
		if stream == null:
			push_error("cannot load music %s" % (DIR + file))
			continue
		stream.set_meta(&"path", DIR + file)
		list.append(stream)
	_tracks[of_mood] = list
	return list


## Problems of the music library (self-test): moods without a loadable
## piece, looping pieces, pieces held uncompressed in memory.
static func check() -> PackedStringArray:
	var problems := PackedStringArray()
	for each in MOODS:
		var list := tracks(each)
		if list.is_empty():
			problems.append("music mood '%s' has no piece in %s" % [each, DIR])
		for stream: AudioStream in list:
			if stream.get_length() < 10.0:
				problems.append("music piece %s is only %.1f s" % [stream.get_meta(&"path"), stream.get_length()])
			if stream is AudioStreamMP3:
				var mp3 := stream as AudioStreamMP3
				if mp3.loop:
					problems.append("music piece %s loops" % stream.get_meta(&"path"))
				# Compressed in memory: far below 16-bit stereo PCM.
				if mp3.data.size() > stream.get_length() * 44100 * 4 / 4:
					problems.append("music piece %s is not compressed" % stream.get_meta(&"path"))
			elif not stream is AudioStreamOggVorbis:
				problems.append("music piece %s is not streamed (%s)" % [stream.get_meta(&"path"), stream.get_class()])
	return problems


func _ready() -> void:
	for i in 2:
		var player := AudioStreamPlayer.new()
		player.bus = "Music"
		player.volume_db = SILENT_DB
		add_child(player)
		_players.append(player)
	if not _pending.is_empty():
		set_mood(_pending)
		_pending = ""


func _exit_tree() -> void:
	if _instance == self:
		_instance = null
	for player in _players:
		player.stop()
		player.stream = null
	_tracks.clear()


func _process(delta: float) -> void:
	_time += delta
	for i in _players.size():
		var player := _players[i]
		if not player.playing:
			continue
		var target := _target[i]
		var step := _rate[i] * delta
		player.volume_db = minf(player.volume_db + step, target) if player.volume_db < target else maxf(player.volume_db - step, target)
		if (i != _active or mood.is_empty()) and player.volume_db <= SILENT_DB + 0.5:
			player.stop()
	if mood.is_empty():
		return
	if _time >= _piece_ends - PIECE_OVERLAP and _next_at == INF:
		_piece_ends = INF
		if mood in ONCE:
			return
		var gap := 0.0
		if mood == "calm":
			gap = randf_range(CALM_GAP.x, CALM_GAP.y)
		_next_at = _time + gap
	if _time >= _next_at:
		_next_at = INF
		_start_piece(PIECE_OVERLAP)


## Changes the mood; "" fades out.
func set_mood(new_mood: String) -> void:
	if new_mood == mood:
		return
	mood = new_mood
	history.append(new_mood)
	_next_at = INF
	_piece_ends = INF
	if new_mood.is_empty():
		_fade(_active, SILENT_DB, MOOD_FADE)
		return
	_start_piece(MOOD_FADE)


func _start_piece(fade: float) -> void:
	var list := tracks(mood)
	if list.is_empty():
		_fade(_active, SILENT_DB, fade)
		return
	# Another piece than the last one, if the mood has more.
	var choices := list.filter(func(s: AudioStream) -> bool: return s.get_meta(&"path") != _last_path)
	if choices.is_empty():
		choices = list
	var stream: AudioStream = choices[randi() % choices.size()]
	_last_path = stream.get_meta(&"path")
	current_piece = _last_path
	_fade(_active, SILENT_DB, fade)
	_active = 1 - _active
	var player := _players[_active]
	player.stop()
	player.stream = stream
	player.volume_db = SILENT_DB
	player.play()
	_fade(_active, MUSIC_DB, fade)
	_piece_ends = _time + stream.get_length()
	pieces_started += 1


func _fade(index: int, target_db: float, seconds: float) -> void:
	_target[index] = target_db
	_rate[index] = absf(_players[index].volume_db - target_db) / maxf(seconds, 0.01)
