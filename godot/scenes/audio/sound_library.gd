extends RefCounted
## Sound library (prompt 029): the manifest `assets/audio/sounds.json` and
## the audio files of every sound class, loaded once and shared.
##
## A class plays one of the files in `assets/audio/<class>/` (any origin:
## synthesized by `tools/audio/synth.py` or prepared from
## `assets-src/audio/`, see docs/ASSETS.md). Classes with `"loop": true`
## loop their files. Static, like `menu/settings.gd`: no autoload.
##
## Keyed names (prompt 053) `<class>@<key>` play the files of
## `assets/audio/<keyed_dir>/<key>/` with the settings of `<class>`:
## `shot_mg@mg_42` (real recordings of one weapon type, `keyed_dir`
## "weapon" by default) or `voice@de/contact` (battle voices; the manifest's
## top-level `voice` spec, `keyed_dir` "voice"). A keyed name without files
## has no stream (`has_files`), so the caller falls back.
##
## A folder with files named `priv_*` plays only those (prompt 069): sounds
## that may ship in the game but not as single files in the public
## repository (docs/AUDIO_LIBRARY.md).

const DIR := "res://assets/audio/"
const MANIFEST := DIR + "sounds.json"
const EXTENSIONS: Array[String] = ["wav", "ogg", "mp3"]
const PRIVATE_PREFIX := "priv_"

static var _manifest := {}
static var _streams: Dictionary[String, Array] = {}


## The parsed manifest: `{groups: {name: {voices, per_frame}}, classes:
## {name: {group, bus, volume_db, pitch_spread, range_m, ref_m, priority,
## variants, loop}}}`.
static func manifest() -> Dictionary:
	if _manifest.is_empty():
		var text := FileAccess.get_file_as_string(MANIFEST)
		var parsed: Variant = JSON.parse_string(text)
		if parsed is Dictionary:
			_manifest = parsed
		else:
			push_error("sound manifest %s is missing or not valid JSON" % MANIFEST)
			_manifest = {"groups": {}, "classes": {}}
	return _manifest


static func classes() -> Dictionary:
	return manifest()["classes"]


static func groups() -> Dictionary:
	return manifest()["groups"]


## Settings of a sound class (empty for an unknown class); keyed names
## take the settings of their class.
static func spec(sound_class: String) -> Dictionary:
	var at := sound_class.find("@")
	if at < 0:
		return classes().get(sound_class, {})
	var base := sound_class.substr(0, at)
	if base == "voice":
		return manifest().get("voice", {})
	return classes().get(base, {})


## Whether a class or keyed name has at least one file.
static func has_files(sound_class: String) -> bool:
	return not streams(sound_class).is_empty()


## The files of a class, loaded (empty if it has none).
static func streams(sound_class: String) -> Array:
	if _streams.has(sound_class):
		return _streams[sound_class]
	var loaded: Array = []
	var looping: bool = spec(sound_class).get("loop", false)
	for path in _files(sound_class):
		var stream := load(path) as AudioStream
		if stream == null:
			push_error("cannot load sound %s" % path)
			continue
		if looping:
			_make_loop(stream)
		loaded.append(stream)
	_streams[sound_class] = loaded
	return loaded


## A random file of a class, or null.
static func pick(sound_class: String) -> AudioStream:
	var list := streams(sound_class)
	return null if list.is_empty() else list[randi() % list.size()]


## Problems of the library: classes without a loadable file, unknown
## groups (self-test, sound gallery).
static func check() -> PackedStringArray:
	var problems := PackedStringArray()
	for name: String in classes():
		var spec_of: Dictionary = classes()[name]
		var group: String = spec_of.get("group", "")
		if group != "loop" and not groups().has(group):
			problems.append("sound class '%s' has unknown group '%s'" % [name, group])
		var list := streams(name)
		if list.is_empty():
			problems.append("sound class '%s' has no loadable file in %s%s/" % [name, DIR, name])
		for stream: AudioStream in list:
			if stream.get_length() <= 0.0:
				problems.append("sound class '%s' has an empty file" % name)
	return problems


## Drops the loaded files (else they are reported as leaked at exit).
static func clear_cache() -> void:
	_streams.clear()


static func _files(sound_class: String) -> PackedStringArray:
	var dir := DIR + sound_class + "/"
	var at := sound_class.find("@")
	if at >= 0:
		dir = DIR + str(spec(sound_class).get("keyed_dir", "weapon")) + "/" + sound_class.substr(at + 1) + "/"
	var out := PackedStringArray()
	# `list_directory` also works in exported games, where only the
	# imported versions of the files exist.
	for file in ResourceLoader.list_directory(dir):
		if file.get_extension().to_lower() in EXTENSIONS:
			out.append(dir + file)
	# Private sounds (`priv_*`, from the private sound repository, never in
	# git) replace the public files of their folder where they exist.
	var private := PackedStringArray()
	for path in out:
		if path.get_file().begins_with(PRIVATE_PREFIX):
			private.append(path)
	if not private.is_empty():
		out = private
	out.sort()
	return out


static func _make_loop(stream: AudioStream) -> void:
	var wav := stream as AudioStreamWAV
	if wav != null and wav.loop_mode == AudioStreamWAV.LOOP_DISABLED:
		wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
		wav.loop_begin = 0
		wav.loop_end = int(wav.get_length() * wav.mix_rate)
	elif stream is AudioStreamOggVorbis:
		(stream as AudioStreamOggVorbis).loop = true
	elif stream is AudioStreamMP3:
		(stream as AudioStreamMP3).loop = true
