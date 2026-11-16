extends Node3D
## Battle sound (prompt 029): weapons, impacts, explosions, burning wrecks,
## collapsing buildings, falling trees, craters, engines and tracks, plus
## the short notices of the match (order confirmed, building finished, unit
## ready, under attack, unit lost, victory, defeat).
##
## Presentation only, like `world/effects.gd`: reads the combat, object and
## crater events and the snapshot; nothing reaches the simulation, so
## Godot randomness is fine. Created by `main.gd`.
##
## Hearing: an `AudioListener3D` stands above the camera's ground focus (not
## at the camera, else every sound is equally far away from high above) and
## faces the camera's view on the ground, so left and right match the
## screen. Zooming out widens the hearing range (`hearing_scale`) and lifts
## the listener a little: the overview hears the whole battle, quieter.
##
## Budget: each sound class belongs to a voice group with a fixed pool of
## players (`assets/audio/sounds.json`). A new sound takes a free player,
## else steals the weakest one (priority, then estimated loudness at the
## listener, then age) if it is not weaker itself; at most `per_frame` new
## sounds per group and frame. Loops (engines, tracks, fires) go to the
## `ENGINE_VOICES` vehicles and `FIRE_VOICES` wrecks nearest the listener.
##
## Fog of war: `visible_at` (set by `main.gd` like the effects') decides;
## sounds at points the player does not see play on the `Muffled` bus
## (low-pass, quieter) and a little off their true position: battle noise
## tells that something happens, not exactly where.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundClasses := preload("res://scenes/audio/sound_classes.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")
const BaseView := preload("res://scenes/world/base_view.gd")
const Ambience := preload("res://scenes/audio/ambience.gd")
const MatchMusic := preload("res://scenes/audio/match_music.gd")

## Camera distance up to which the hearing range is the manifest's.
const HEARING_REF_DISTANCE := 150.0
const MAX_HEARING_SCALE := 4.0
## Listener height above the focus, as a share of the camera distance.
const LISTENER_HEIGHT_SHARE := 0.12
const MIN_LISTENER_HEIGHT := 6.0
## Vehicles with their own engine loop (nearest to the listener).
const ENGINE_VOICES := 6
const FIRE_VOICES := 4
## How often the loops are reassigned (s).
const LOOP_INTERVAL := 0.2
## Burning wrecks crackle this long, then quieter while they smolder (the
## times of `effects.gd`).
const BURN_SECONDS := 60.0
const SMOLDER_SECONDS := 90.0
const SMOLDER_DB := -12.0
## Sounds in the fog: quieter (on top of the Muffled bus) and moved by up
## to this share of their distance to the listener.
const FOG_DB := -3.0
const FOG_JITTER_SHARE := 0.15
## Engine: pitch and volume between standing and full road speed.
const ENGINE_PITCH_IDLE := 0.8
const ENGINE_PITCH_FULL := 1.2
const ENGINE_IDLE_DB := -9.0
const TRACKS_IDLE_DB := -40.0
## How fast loop volume and pitch follow (1/s).
const LOOP_SMOOTHING := 6.0
## `weapon` of shot events for indirect fire (prompt 032).
const INDIRECT_WEAPON := 255
## Smoke shells burst with a small charge.
const SMOKE_BURST_DB := -8.0
## Notices: minimum seconds between two of the same kind.
const NOTICE_COOLDOWN: Dictionary[String, float] = {
	"ui_under_attack": 20.0, "ui_unit_lost": 6.0, "ui_unit_ready": 1.0, "ui_build_complete": 1.0,
	"ui_order": 0.12, "ui_select": 0.12,
}
## New units in the first seconds are the start forces, not production.
const START_GRACE_SECONDS := 2.0

## Visibility of a world point (fog of war); default: everything visible.
var visible_at: Callable = func(_point: Vector3) -> bool: return true
## Statistics since the start (self-test, gallery, benchmark): sounds
## played per class, dropped per group (pool or frame budget), culled
## (beyond hearing range), muffled (in the fog), highest number of busy
## voices per group in one frame.
var played: Dictionary[String, int] = {}
## Sounds asked for per class, played or not (self-test: event wiring).
var requested: Dictionary[String, int] = {}
var dropped: Dictionary[String, int] = {}
var culled := 0
var muffled := 0
var peak_voices: Dictionary[String, int] = {}
var notices: Dictionary[String, int] = {}
## Time of the last `_process` in microseconds (benchmark).
var process_usec := 0

var main: Node3D
var sim_host: SimHost
var listener: AudioListener3D
var hearing_scale := 1.0
## Wind, birds, water and the rest of the landscape; calm or combat music
## (prompt 030).
var ambience: Ambience
var match_music: MatchMusic
var _time := 0.0
## Group -> Array of voices `{player, priority, score, started, ends}`.
var _pools: Dictionary[String, Array] = {}
var _frame_budget: Dictionary[String, int] = {}
## Unit type index -> `unit_type_sound` (cached).
var _type_info: Dictionary[int, Dictionary] = {}
## "type/weapon" -> shot class (cached).
var _shot_classes: Dictionary[String, String] = {}
var _index_cache: Dictionary[int, int] = {}
var _index_frame := -1
var _types := PackedInt32Array()
var _owners := PackedInt32Array()
## Loops: players and what they play.
var _engine_slots: Array[Dictionary] = []
var _fire_slots: Array[Dictionary] = []
var _loop_timer := 0.0
## Unit id -> last position and time (engine speed).
var _last_seen: Dictionary[int, Vector3] = {}
## Burning wrecks: `{position, age}`.
var _wrecks: Array[Dictionary] = []
## Notices.
var _notice_at: Dictionary[String, float] = {}
var _known_units: Dictionary[int, bool] = {}
var _built: Dictionary[int, bool] = {}
var _selection_key := ""
var _result_timer := 0.0
var _result_played := false


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	SoundBuses.ensure()
	listener = AudioListener3D.new()
	listener.name = "Listener"
	add_child(listener)
	listener.make_current()
	for group: String in SoundLibrary.groups():
		if group == "ui":
			continue
		var pool: Array = []
		for i in int(SoundLibrary.groups()[group].get("voices", 4)):
			pool.append({"player": _new_player(), "priority": -1, "score": -INF, "started": 0.0, "ends": 0.0})
		_pools[group] = pool
		dropped[group] = 0
		peak_voices[group] = 0
	for i in ENGINE_VOICES:
		_engine_slots.append({"engine": _new_player(), "tracks": _new_player(), "unit": -1, "sound": ""})
	for i in FIRE_VOICES:
		_fire_slots.append({"player": _new_player(), "wreck": null})
	main.combat_view.combat_event.connect(on_combat_event)
	# Indirect fire (prompt 032), if present: the whistle before a shell lands.
	var artillery: Object = main.get("artillery_view")
	if artillery != null and artillery.has_signal(&"shell_incoming"):
		artillery.connect(&"shell_incoming", shell_incoming)
	var selection: Node = main.selection
	selection.move_ordered.connect(func(_point: Vector3) -> void: notice("ui_order"))
	selection.attack_ordered.connect(func(_target: int) -> void: notice("ui_order"))
	selection.garrison_ordered.connect(func(_building: int) -> void: notice("ui_order"))
	_update_listener()
	ambience = Ambience.new()
	ambience.name = "Ambience"
	add_child(ambience)
	ambience.setup(main, self)
	match_music = MatchMusic.new()
	match_music.name = "MatchMusic"
	add_child(match_music)
	match_music.setup(main, self)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	var began := Time.get_ticks_usec()
	_time += delta
	for group: String in _pools:
		var busy := 0
		for voice: Dictionary in _pools[group]:
			if voice.ends > _time:
				busy += 1
		peak_voices[group] = maxi(peak_voices[group], busy)
		_frame_budget[group] = int(SoundLibrary.groups()[group].get("per_frame", 3))
	_update_listener()
	_age_wrecks(delta)
	_loop_timer -= delta
	if _loop_timer <= 0.0:
		_loop_timer = LOOP_INTERVAL
		_assign_engines()
		_assign_fires()
		_watch_notices()
	_update_loops(delta)
	process_usec = Time.get_ticks_usec() - began


func _exit_tree() -> void:
	SoundBuses.release(find_children("*", "AudioStreamPlayer3D", false, false))
	SoundLibrary.clear_cache()


# --- Events -----------------------------------------------------------------


## Plays the sound of one combat event (`SimHost.take_combat_events`).
func on_combat_event(event: Dictionary) -> void:
	match event["type"]:
		"shot":
			var unit: int = event["unit"]
			var sound_class := _shot_class(unit, int(event.get("weapon", 0)), float(event["caliber_mm"]))
			play_at(sound_class, event["position"])
		"hit":
			var at: Vector3 = event["position"]
			var target: int = event.get("target", -1)
			var infantry := false
			if target >= 0:
				var index := _index_of(target)
				infantry = index >= 0 and str(_info(_types[index]).get("category", "")).begins_with("inf_")
				if index >= 0 and _owners[index] == sim_host.local_player():
					notice("ui_under_attack")
			var water := false
			if event["surface"] == "ground" and float(event["caliber_mm"]) >= SoundClasses.SMALL_ARMS_MM:
				water = sim_host.terrain_at(at.x, at.z) >= 6
			var picked: Array = SoundClasses.hit_class(event, infantry, water)
			if not (picked[0] as String).is_empty():
				play_at(picked[0], at, picked[1])
		"destroyed":
			_destroyed(event["unit"], event["position"])
		"smoke":
			play_at("hit_shell_small", event.get("position", Vector3.ZERO), SMOKE_BURST_DB)


## A shell lands in about 1.5 s (`artillery_view.shell_incoming`).
func shell_incoming(at: Vector3, caliber_mm: float, smoke: bool) -> void:
	play_at("shell_incoming", at, SoundClasses.caliber_gain_db(caliber_mm) + (SMOKE_BURST_DB if smoke else 0.0))


## A map object changed its stage (`destruction_view.gd`).
func object_event(event: Dictionary) -> void:
	var sound_class := SoundClasses.object_class(event)
	if sound_class.is_empty():
		return
	var at: Vector3 = sim_host.object_position(event.object)
	at.y = sim_host.height_at(at.x, at.z)
	play_at(sound_class, at)


## A crater appeared (`crater_view.gd`): earth rains down.
func crater_event(event: Dictionary) -> void:
	# Engineers digging (prompt 035) change the ground without a blast.
	if float(event.get("caliber", 75.0)) <= 0.0:
		return
	var at: Vector3 = event.get("position", Vector3.ZERO)
	play_at("crater", at, SoundClasses.caliber_gain_db(float(event.get("caliber", 75.0))))


func _destroyed(unit: int, at: Vector3) -> void:
	var index := _index_of(unit)
	var type_id: String = str(_info(_types[index]).get("id", "")) if index >= 0 else ""
	if index >= 0 and _owners[index] == sim_host.local_player():
		notice("ui_unit_lost")
	if type_id.contains("_inf_"):
		return
	if BaseView.is_building(type_id):
		play_at("building_collapse", at)
		return
	var size := clampf((sim_host.unit_footprints()[index] if index >= 0 else 6.0) / 7.0, 0.6, 1.4)
	play_at("explosion_vehicle", at, 20.0 * log(size) / log(10.0))
	_wrecks.append({"position": at + Vector3.UP, "age": 0.0})


# --- One-shot voices ----------------------------------------------------------


## Plays `sound_class` at `at` with `gain_db` on top of its manifest level.
## Returns whether a voice started (else: out of range, budget or pool).
func play_at(sound_class: String, at: Vector3, gain_db := 0.0) -> bool:
	var spec := SoundLibrary.spec(sound_class)
	if spec.is_empty():
		return false
	requested[sound_class] = requested.get(sound_class, 0) + 1
	var group: String = spec.group
	var pool: Array = _pools.get(group, [])
	if pool.is_empty():
		return false
	var reach: float = float(spec.get("range_m", 300.0)) * hearing_scale
	var ref: float = float(spec.get("ref_m", 10.0)) * hearing_scale
	var distance := listener.global_position.distance_to(at)
	if distance > reach:
		culled += 1
		return false
	if _frame_budget.get(group, 0) <= 0:
		dropped[group] += 1
		return false
	var hidden: bool = not visible_at.call(at)
	var volume: float = float(spec.get("volume_db", 0.0)) + gain_db + (FOG_DB + SoundBuses.MUFFLED_DB if hidden else 0.0)
	var loudness := volume + 20.0 * log(ref / maxf(distance, ref)) / log(10.0)
	var priority: int = spec.get("priority", 1)
	var score := priority * 12.0 + loudness / 3.0
	var voice := _victim(pool, score)
	if voice.is_empty():
		dropped[group] += 1
		return false
	var stream := SoundLibrary.pick(sound_class)
	if stream == null:
		return false
	_frame_budget[group] -= 1
	var player: AudioStreamPlayer3D = voice.player
	var spread: float = spec.get("pitch_spread", 0.0)
	player.stop()
	player.stream = stream
	player.bus = "Muffled" if hidden else str(spec.get("bus", "Effects"))
	player.volume_db = volume - (SoundBuses.MUFFLED_DB if hidden else 0.0)
	player.pitch_scale = 1.0 + randf_range(-spread, spread)
	player.unit_size = ref
	player.max_distance = reach
	var position := at
	if hidden:
		muffled += 1
		var off := distance * FOG_JITTER_SHARE
		position += Vector3(randf_range(-off, off), 0.0, randf_range(-off, off))
	player.global_position = position
	player.play()
	voice.priority = priority
	voice.score = score
	voice.started = _time
	voice.ends = _time + stream.get_length() / player.pitch_scale
	played[sound_class] = played.get(sound_class, 0) + 1
	return true


## The voice a new sound of `score` takes: a free one, else the weakest
## busy one (score lowered as it fades out), or {} if all are stronger.
func _victim(pool: Array, score: float) -> Dictionary:
	var best: Dictionary = {}
	var best_value := INF
	for voice: Dictionary in pool:
		if voice.ends <= _time:
			return voice
		var length: float = maxf(voice.ends - voice.started, 0.001)
		var faded: float = (_time - voice.started) / length
		var value: float = voice.score - 6.0 * faded
		if value < best_value:
			best_value = value
			best = voice
	return best if best_value <= score else {}


## Busy one-shot voices per group right now (self-test, gallery).
func busy_voices() -> Dictionary[String, int]:
	var out: Dictionary[String, int] = {}
	for group: String in _pools:
		var busy := 0
		for voice: Dictionary in _pools[group]:
			if voice.ends > _time:
				busy += 1
		out[group] = busy
	return out


func pool_size(group: String) -> int:
	return _pools.get(group, []).size()


func _new_player() -> AudioStreamPlayer3D:
	var player := AudioStreamPlayer3D.new()
	player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
	player.doppler_tracking = AudioStreamPlayer3D.DOPPLER_TRACKING_DISABLED
	player.max_polyphony = 1
	player.bus = "Effects"
	add_child(player)
	return player


# --- Listener -----------------------------------------------------------------


func _update_listener() -> void:
	var rig: Node3D = main.camera_rig
	var distance: float = rig.distance
	hearing_scale = clampf(distance / HEARING_REF_DISTANCE, 1.0, MAX_HEARING_SCALE)
	var forward: Vector3 = -main.camera.global_basis.z
	forward.y = 0.0
	if forward.length() < 0.01:
		forward = Vector3.FORWARD.rotated(Vector3.UP, rig.yaw)
	var position: Vector3 = rig.focus + Vector3.UP * maxf(MIN_LISTENER_HEIGHT, distance * LISTENER_HEIGHT_SHARE)
	listener.global_transform = Transform3D(Basis.looking_at(forward.normalized(), Vector3.UP), position)


# --- Loops: engines, tracks, fires ---------------------------------------------


## Gives the nearest vehicles to the listener an engine (and tracks) loop.
func _assign_engines() -> void:
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var alive := sim_host.unit_alive()
	var types := sim_host.unit_types()
	var hear := listener.global_position
	var candidates: Array = []
	for i in ids.size():
		if alive[i] == 0:
			continue
		var info := _info(types[i])
		var engine := SoundClasses.engine_class(str(info.get("category", "")), int(info.get("mass_kg", 0)))
		if engine.is_empty():
			continue
		var reach: float = float(SoundLibrary.spec(engine).get("range_m", 200.0)) * hearing_scale
		var d := hear.distance_to(positions[i])
		if d < reach:
			candidates.append([d, ids[i], engine, info])
	candidates.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var wanted: Dictionary[int, Array] = {}
	for c: Array in candidates.slice(0, ENGINE_VOICES):
		wanted[c[1]] = c
	# Slots keep their unit while it stays among the nearest.
	var free: Array[Dictionary] = []
	for slot in _engine_slots:
		if wanted.has(slot.unit):
			wanted.erase(slot.unit)
		else:
			free.append(slot)
	for slot in free:
		if wanted.is_empty():
			_stop_slot(slot)
			continue
		var unit: int = wanted.keys()[0]
		var c: Array = wanted[unit]
		wanted.erase(unit)
		_start_engine(slot, unit, c[2], c[3])


func _start_engine(slot: Dictionary, unit: int, engine: String, info: Dictionary) -> void:
	slot.unit = unit
	slot.sound = engine
	_last_seen.erase(unit)
	slot.road_speed = maxf(float(info.get("road_speed", 10.0)), 1.0)
	slot.level = 0.0
	slot.speed = 0.0
	var tracked := SoundClasses.has_tracks(str(info.get("category", "")))
	_start_loop(slot.engine, engine, randf() * 3.0)
	if tracked:
		_start_loop(slot.tracks, "tracks", randf() * 3.0)
	else:
		(slot.tracks as AudioStreamPlayer3D).stop()
	slot.tracked = tracked


func _stop_slot(slot: Dictionary) -> void:
	slot.unit = -1
	slot.sound = ""
	(slot.engine as AudioStreamPlayer3D).stop()
	(slot.tracks as AudioStreamPlayer3D).stop()


func _start_loop(player: AudioStreamPlayer3D, sound_class: String, offset: float) -> void:
	var spec := SoundLibrary.spec(sound_class)
	var stream := SoundLibrary.pick(sound_class)
	if stream == null:
		return
	player.stream = stream
	player.bus = str(spec.get("bus", "Effects"))
	player.unit_size = float(spec.get("ref_m", 10.0)) * hearing_scale
	player.max_distance = float(spec.get("range_m", 200.0)) * hearing_scale
	player.volume_db = -60.0
	player.play(fmod(offset, maxf(stream.get_length(), 0.01)))
	played[sound_class] = played.get(sound_class, 0) + 1


## Gives the nearest burning wrecks a fire loop.
func _assign_fires() -> void:
	var hear := listener.global_position
	var reach: float = float(SoundLibrary.spec("fire").get("range_m", 250.0)) * hearing_scale
	var near: Array = []
	for wreck in _wrecks:
		var d := hear.distance_to(wreck.position)
		if d < reach:
			near.append([d, wreck])
	near.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var wanted: Array = near.slice(0, FIRE_VOICES).map(func(entry: Array) -> Dictionary: return entry[1])
	var free: Array[Dictionary] = []
	for slot in _fire_slots:
		if slot.wreck != null and wanted.has(slot.wreck):
			wanted.erase(slot.wreck)
		else:
			free.append(slot)
	for slot in free:
		var player: AudioStreamPlayer3D = slot.player
		if wanted.is_empty():
			slot.wreck = null
			player.stop()
			continue
		slot.wreck = wanted.pop_front()
		player.global_position = slot.wreck.position
		_start_loop(player, "fire", randf() * 5.0)


func _age_wrecks(delta: float) -> void:
	for i in range(_wrecks.size() - 1, -1, -1):
		_wrecks[i].age += delta
		if _wrecks[i].age > BURN_SECONDS + SMOLDER_SECONDS:
			_wrecks.remove_at(i)


## Follows the vehicles with their loops; volume and pitch by speed.
func _update_loops(delta: float) -> void:
	var smooth := clampf(delta * LOOP_SMOOTHING, 0.0, 1.0)
	for slot in _engine_slots:
		if slot.unit < 0:
			continue
		var node: Node3D = main._unit_nodes.get(slot.unit)
		if node == null or not node.is_inside_tree():
			_stop_slot(slot)
			continue
		var position := node.global_position
		var last: Vector3 = _last_seen.get(slot.unit, position)
		_last_seen[slot.unit] = position
		var speed := position.distance_to(last) / maxf(delta, 0.001)
		slot.speed = lerpf(slot.speed, speed, smooth)
		var share := clampf(slot.speed / slot.road_speed, 0.0, 1.0)
		var hidden: bool = not visible_at.call(position)
		var engine: AudioStreamPlayer3D = slot.engine
		var spec := SoundLibrary.spec(slot.sound)
		engine.global_position = position
		engine.pitch_scale = lerpf(ENGINE_PITCH_IDLE, ENGINE_PITCH_FULL, share)
		var fog := FOG_DB if hidden else 0.0
		engine.volume_db = lerpf(engine.volume_db, float(spec.get("volume_db", 0.0)) + lerpf(ENGINE_IDLE_DB, 0.0, share) + fog, smooth)
		engine.bus = "Muffled" if hidden else "Effects"
		if slot.tracked:
			var tracks: AudioStreamPlayer3D = slot.tracks
			tracks.global_position = position
			tracks.pitch_scale = lerpf(0.7, 1.3, share)
			var tracks_db := float(SoundLibrary.spec("tracks").get("volume_db", 0.0)) + lerpf(TRACKS_IDLE_DB, 0.0, sqrt(share)) + fog
			tracks.volume_db = lerpf(tracks.volume_db, tracks_db, smooth)
			tracks.bus = engine.bus
	for slot in _fire_slots:
		if slot.wreck == null:
			continue
		var player: AudioStreamPlayer3D = slot.player
		var wreck: Dictionary = slot.wreck
		var level := float(SoundLibrary.spec("fire").get("volume_db", 0.0))
		if wreck.age > BURN_SECONDS:
			level += SMOLDER_DB
		var hidden: bool = not visible_at.call(wreck.position)
		player.bus = "Muffled" if hidden else "Effects"
		player.volume_db = lerpf(player.volume_db, level + (FOG_DB if hidden else 0.0), smooth)


## Engines and fires playing right now (self-test, gallery).
func active_loops() -> int:
	var n := 0
	for slot in _engine_slots:
		n += 1 if slot.unit >= 0 else 0
	for slot in _fire_slots:
		n += 1 if slot.wreck != null else 0
	return n


# --- Notices ------------------------------------------------------------------


## Plays a notice (UI sound) unless the same one played within its cooldown.
func notice(sound_class: String) -> void:
	var cooldown: float = NOTICE_COOLDOWN.get(sound_class, 0.0)
	if _time - float(_notice_at.get(sound_class, -INF)) < cooldown:
		return
	_notice_at[sound_class] = _time
	notices[sound_class] = notices.get(sound_class, 0) + 1
	UiSound.play(sound_class)


## Polled with the loops: selection, new own units, finished buildings, the
## result of the match.
func _watch_notices() -> void:
	var selected: Array = main.selection.selected
	var key := str(selected.size()) + ":" + (str(selected[0]) if not selected.is_empty() else "")
	if key != _selection_key:
		_selection_key = key
		if not selected.is_empty():
			notice("ui_select")
	var me := sim_host.local_player()
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var structures: Dictionary = main.base_view.structures if main.base_view != null else {}
	for i in ids.size():
		var id := ids[i]
		if owners[i] != me or _known_units.has(id):
			continue
		_known_units[id] = true
		if _time > START_GRACE_SECONDS and not structures.has(id):
			notice("ui_unit_ready")
	for id: int in structures:
		var s: Dictionary = structures[id]
		if int(s.get("owner", -1)) != me:
			continue
		var done: bool = float(s.get("built", 1.0)) >= 1.0
		if done and _built.get(id, true) == false:
			notice("ui_build_complete")
		_built[id] = done
	_result_timer -= LOOP_INTERVAL
	if not _result_played and _result_timer <= 0.0:
		_result_timer = 1.0
		var result := sim_host.match_result()
		if result.get("decided", false):
			_result_played = true
			notice("ui_victory" if int(result.get("winner", -1)) == me else "ui_defeat")


# --- Helpers ------------------------------------------------------------------


func _info(unit_type: int) -> Dictionary:
	if not _type_info.has(unit_type):
		_type_info[unit_type] = sim_host.unit_type_sound(unit_type)
	return _type_info[unit_type]


func _shot_class(unit: int, weapon: int, caliber: float) -> String:
	var index := _index_of(unit)
	if index < 0:
		return SoundClasses.shot_class_by_caliber(caliber)
	var unit_type := _types[index]
	var key := "%d/%d" % [unit_type, weapon]
	if not _shot_classes.has(key):
		var info := _info(unit_type)
		var weapons: Array = info.get("weapons", [])
		var sound_class := SoundClasses.shot_class_by_caliber(caliber)
		if weapon == INDIRECT_WEAPON:
			sound_class = SoundClasses.indirect_shot_class(str(info.get("category", "")))
		elif weapon >= 0 and weapon < weapons.size():
			sound_class = SoundClasses.shot_class(str(info.get("category", "")), weapons[weapon])
		_shot_classes[key] = sound_class
	return _shot_classes[key]


## Index of a unit id in this frame's snapshot arrays (read once per frame).
func _index_of(unit: int) -> int:
	var frame := Engine.get_process_frames()
	if frame != _index_frame:
		_index_frame = frame
		_index_cache.clear()
		var ids := sim_host.unit_ids()
		for i in ids.size():
			_index_cache[ids[i]] = i
		_types = sim_host.unit_types()
		_owners = sim_host.unit_owners()
	return _index_cache.get(unit, -1)
