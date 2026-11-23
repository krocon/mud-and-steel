extends Node3D
## Ambience (prompt 030): wind, insects, rustling woods, rivers, rain,
## birds and distant rumble on the `Ambience` bus. Created by
## `battle_sound.gd`; presentation only.
##
## Files: `assets/audio/ambience/<layer>/*.wav` (`tools/audio/music.py`,
## `just music`). Beds loop, birds and rumble are one-shots. `priv_*` files
## of a layer (private sound repository) replace its public ones.
##
## - Biome (`main.atmosphere.preset.id`, followed when it changes): which
##   wind, insects and birds play (`BIOMES`).
## - Map: every `SAMPLE_INTERVAL` a 7 × 7 grid around the listener (radius
##   `GRID_RADIUS` × hearing scale) counts water samples (`terrain_at`) and
##   wood density (`SimHost.wood_density_at`). Water and leaves are one
##   player each at the center of their samples, loud by their share: a
##   river to the left is heard on the left, without a player per tree.
## - Camera height: close up the details (insects, leaves, water, birds)
##   are full and the wind is lower; in the overview the details fade and
##   the wind takes over (`NEAR_DISTANCE`, `FAR_DISTANCE`).
## - Birds: a call every few seconds at a wooded sample (biome interval),
##   silent for `BIRD_SILENCE` seconds after combat near the listener, then
##   back over `BIRD_RETURN` seconds.
## - Rumble: a distant roll every 40–120 s from a random direction.
## - Rain: `rain` (0..1) for the weather of a later prompt; damps insects
##   and birds.

const DIR := "res://assets/audio/ambience/"
const PRIVATE_PREFIX := "priv_"
const LAYERS: Array[String] = [
	"wind_forest", "wind_desert", "wind_winter", "insects_meadow", "insects_desert", "leaves", "water", "rain",
	"birds_forest", "birds_desert", "birds_winter", "rumble",
]
const LOOPS: Array[String] = [
	"wind_forest", "wind_desert", "wind_winter", "insects_meadow", "insects_desert", "leaves", "water", "rain",
]
## Per biome: wind and insect beds, birds, seconds between bird calls and
## levels (dB) on top of the base levels below. [provisional] All levels
## set without listening.
const BIOMES := {
	"forest": {"wind": "wind_forest", "insects": "insects_meadow", "insects_db": -2.0, "birds": "birds_forest",
		"bird_every": Vector2(2.0, 7.0), "leaves_db": 0.0},
	"desert": {"wind": "wind_desert", "insects": "insects_desert", "insects_db": 0.0, "birds": "birds_desert",
		"bird_every": Vector2(8.0, 20.0), "leaves_db": -6.0},
	"winter": {"wind": "wind_winter", "insects": "", "insects_db": 0.0, "birds": "birds_winter",
		"bird_every": Vector2(9.0, 24.0), "leaves_db": -12.0},
	# Prompt 120: cicadas by day (the synthesized cicada bed of the desert,
	# louder), the mild wind of the forest, songbirds less often; the sea is
	# the water layer like every water. [provisional]
	"mediterranean": {"wind": "wind_forest", "insects": "insects_desert", "insects_db": 3.0, "birds": "birds_forest",
		"bird_every": Vector2(5.0, 14.0), "leaves_db": -4.0},
}
const WIND_DB := -16.0
const INSECTS_DB := -24.0
const LEAVES_DB := -20.0
const WATER_DB := -16.0
const RAIN_DB := -12.0
const BIRD_DB := -14.0
const RUMBLE_DB := -16.0
const SILENT_DB := -80.0
## Camera distances (m) for the near and far mix, and the levels at far.
const NEAR_DISTANCE := 60.0
const FAR_DISTANCE := 450.0
const FAR_DETAIL_DB := -24.0
const NEAR_WIND_DB := -8.0
## Sampling grid around the listener.
const GRID := 7
const GRID_RADIUS := 40.0
const SAMPLE_INTERVAL := 0.5
## Terrain ids of water (`data/terrain.toml`: shallow, deep; tall grass,
## id 8 since prompt 073, is not water).
const WATER_TERRAIN := 6
const WATER_TERRAIN_DEEP := 7
## Wood density from which a sample counts as wood for birds.
const BIRD_WOOD := 0.2
## Birds: silence after combat within this radius (× hearing scale).
const BIRD_SILENCE := 60.0
const BIRD_RETURN := 20.0
const BIRD_SILENCE_RADIUS := 250.0
const BIRD_VOICES := 3
## Rumble: seconds between two, distance of the source.
const RUMBLE_EVERY := Vector2(40.0, 120.0)
const RUMBLE_DISTANCE := 400.0
## How fast bed volumes follow their targets (dB per second).
const FADE_DB_PER_SECOND := 12.0

static var _streams: Dictionary[String, Array] = {}

var main: Node3D
var sim_host: SimHost
var battle_sound: Node3D
## Rain 0..1 (weather, later prompt).
var rain := 0.0
## Biome whose layers play.
var biome := ""
## Last grid result: water share and mean wood density 0..1.
var water_share := 0.0
var wood_share := 0.0
## Near mix 1 (close up) .. 0 (overview).
var near := 1.0
## Statistics (self-test): birds called, rumbles, birds left out because of
## combat, the time until birds are silent.
var birds_played := 0
var birds_silenced := 0
var rumbles := 0
var silent_until := -INF
## Grid center override for tools and the self-test (null: listener).
var center_override: Variant = null
## Camera distance override for the self-test (null: the camera's).
var distance_override: Variant = null

var _time := 0.0
var _sample_timer := 0.0
var _beds: Dictionary[String, Node] = {}
var _target_db: Dictionary[String, float] = {}
var _birds: Array[AudioStreamPlayer3D] = []
var _bird_next := 0
var _next_bird_at := 3.0
var _next_rumble_at := 0.0
var _rumble: AudioStreamPlayer3D
var _wood_points: PackedVector3Array = []
var _water_center := Vector3.ZERO
var _wood_center := Vector3.ZERO


## The files of a layer, loaded; loops loop (empty if there are none).
static func streams(layer: String) -> Array:
	if _streams.has(layer):
		return _streams[layer]
	var list: Array = []
	var dir := DIR + layer + "/"
	var files := PackedStringArray()
	var private := PackedStringArray()
	for file in ResourceLoader.list_directory(dir):
		if file.get_extension().to_lower() in ["wav", "ogg", "mp3"]:
			files.append(file)
			if file.begins_with(PRIVATE_PREFIX):
				private.append(file)
	# Private files (`priv_*`, from the private sound repository, never in
	# git) replace the public files of their layer where they exist.
	if not private.is_empty():
		files = private
	files.sort()
	for file in files:
		var stream := load(dir + file) as AudioStream
		if stream == null:
			push_error("cannot load ambience %s" % (dir + file))
			continue
		var wav := stream as AudioStreamWAV
		if layer in LOOPS and wav != null and wav.loop_mode == AudioStreamWAV.LOOP_DISABLED:
			wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
			wav.loop_begin = 0
			wav.loop_end = int(wav.get_length() * wav.mix_rate)
		list.append(stream)
	_streams[layer] = list
	return list


## Problems of the ambience files (self-test): layers without a file,
## loops that do not loop.
static func check() -> PackedStringArray:
	var problems := PackedStringArray()
	for layer in LAYERS:
		var list := streams(layer)
		if list.is_empty():
			problems.append("ambience layer '%s' has no loadable file in %s%s/" % [layer, DIR, layer])
		for stream: AudioStream in list:
			var wav := stream as AudioStreamWAV
			if layer in LOOPS and wav != null and wav.loop_mode == AudioStreamWAV.LOOP_DISABLED:
				problems.append("ambience loop '%s' does not loop" % layer)
	return problems


func setup(main_node: Node3D, sound: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	battle_sound = sound
	for bed in ["wind", "insects", "rain"]:
		var player := AudioStreamPlayer.new()
		player.bus = "Ambience"
		player.volume_db = SILENT_DB
		add_child(player)
		_beds[bed] = player
	for bed in ["leaves", "water"]:
		var player := _new_player3d(false)
		_beds[bed] = player
		_start_bed(bed, bed)
	_start_bed("rain", "rain")
	for i in BIRD_VOICES:
		_birds.append(_new_player3d(true))
	_rumble = _new_player3d(false)
	_next_rumble_at = randf_range(RUMBLE_EVERY.x * 0.3, RUMBLE_EVERY.y * 0.5)
	main.combat_view.combat_event.connect(on_combat_event)
	_follow_biome()
	sample()


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_time += delta
	_follow_biome()
	_sample_timer -= delta
	if _sample_timer <= 0.0:
		_sample_timer = SAMPLE_INTERVAL
		sample()
	update_targets()
	for bed: String in _beds:
		var player: Node = _beds[bed]
		var target: float = _target_db.get(bed, SILENT_DB)
		var volume: float = player.get(&"volume_db")
		player.set(&"volume_db", move_toward(volume, target, FADE_DB_PER_SECOND * delta))
	if _time >= _next_bird_at:
		var every: Vector2 = BIOMES.get(biome, BIOMES.forest).bird_every
		_next_bird_at = _time + randf_range(every.x, every.y) * (1.0 + 3.0 * rain)
		try_bird()
	if _time >= _next_rumble_at:
		_next_rumble_at = _time + randf_range(RUMBLE_EVERY.x, RUMBLE_EVERY.y)
		play_rumble()


func _exit_tree() -> void:
	var players: Array = find_children("*", "AudioStreamPlayer", false, false)
	players.append_array(find_children("*", "AudioStreamPlayer3D", false, false))
	for player: Node in players:
		player.stop()
		player.set(&"stream", null)
	_streams.clear()


## Combat near the listener silences the birds for a while.
func on_combat_event(event: Dictionary) -> void:
	var at: Vector3 = event.get("position", Vector3.ZERO)
	var reach := BIRD_SILENCE_RADIUS * _hearing_scale()
	if _center().distance_to(Vector3(at.x, 0.0, at.z)) <= reach:
		silent_until = _time + BIRD_SILENCE


## Plays a bird call if the birds are not silent. Returns whether one
## started.
func try_bird() -> bool:
	var layer: String = BIOMES.get(biome, BIOMES.forest).birds
	if layer.is_empty():
		return false
	# Silent after combat, then a growing chance to sing again.
	var since := _time - silent_until
	if since < 0.0 or randf() > clampf(since / BIRD_RETURN, 0.0, 1.0):
		birds_silenced += 1
		return false
	var stream := _pick(layer)
	if stream == null:
		return false
	var at: Vector3
	if not _wood_points.is_empty():
		at = _wood_points[randi() % _wood_points.size()]
	else:
		var angle := randf() * TAU
		at = _center() + Vector3(cos(angle), 0.0, sin(angle)) * GRID_RADIUS * _hearing_scale() * randf_range(0.3, 1.0)
	at.y = sim_host.height_at(at.x, at.z) + randf_range(3.0, 10.0)
	var player := _birds[_bird_next]
	_bird_next = (_bird_next + 1) % _birds.size()
	player.stream = stream
	player.global_position = at
	player.unit_size = 20.0 * _hearing_scale()
	player.max_distance = GRID_RADIUS * 4.0 * _hearing_scale()
	player.volume_db = BIRD_DB + _detail_db() + (6.0 if not _wood_points.is_empty() else 0.0) - 12.0 * rain
	player.pitch_scale = randf_range(0.94, 1.06)
	player.play()
	birds_played += 1
	return true


## A distant rumble from a random direction.
func play_rumble() -> bool:
	var stream := _pick("rumble")
	if stream == null:
		return false
	var angle := randf() * TAU
	_rumble.stream = stream
	_rumble.global_position = _listener_position() + Vector3(cos(angle), 0.0, sin(angle)) * RUMBLE_DISTANCE
	_rumble.volume_db = RUMBLE_DB + randf_range(-4.0, 2.0)
	_rumble.pitch_scale = randf_range(0.85, 1.1)
	_rumble.play()
	rumbles += 1
	return true


## Volume (dB) each bed is heading to right now (self-test, tools).
func targets() -> Dictionary[String, float]:
	return _target_db.duplicate()


## Name of the layer a bed plays ("" = none).
func bed_layer(bed: String) -> String:
	var player: Node = _beds.get(bed)
	return str(player.get_meta(&"layer", "")) if player != null else ""


func _new_player3d(attenuate: bool) -> AudioStreamPlayer3D:
	var player := AudioStreamPlayer3D.new()
	player.bus = "Ambience"
	player.doppler_tracking = AudioStreamPlayer3D.DOPPLER_TRACKING_DISABLED
	player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE if attenuate else AudioStreamPlayer3D.ATTENUATION_DISABLED
	# Volume comes from the shares; position only gives the direction.
	player.attenuation_filter_db = 0.0
	player.volume_db = SILENT_DB
	add_child(player)
	return player


func _start_bed(bed: String, layer: String) -> void:
	var player: Node = _beds[bed]
	if str(player.get_meta(&"layer", "")) == layer:
		return
	player.set_meta(&"layer", layer)
	player.call(&"stop")
	if layer.is_empty():
		return
	var stream := _pick(layer)
	if stream == null:
		return
	player.set(&"stream", stream)
	player.call(&"play", randf() * stream.get_length())


func _follow_biome() -> void:
	var atmosphere: Node = main.get(&"atmosphere")
	var id: String = atmosphere.preset.id if atmosphere != null and atmosphere.preset != null else "forest"
	if id == biome:
		return
	biome = id if BIOMES.has(id) else "forest"
	var spec: Dictionary = BIOMES[biome]
	_start_bed("wind", spec.wind)
	_start_bed("insects", spec.insects)


func sample() -> void:
	var center := _center()
	var radius := GRID_RADIUS * _hearing_scale()
	var water := 0
	var wood_sum := 0.0
	var water_center := Vector3.ZERO
	var wood_center := Vector3.ZERO
	_wood_points.clear()
	for j in GRID:
		for i in GRID:
			var x := center.x + (float(i) / (GRID - 1) * 2.0 - 1.0) * radius
			var z := center.z + (float(j) / (GRID - 1) * 2.0 - 1.0) * radius
			var point := Vector3(x, 0.0, z)
			if is_water(sim_host.terrain_at(x, z)):
				water += 1
				water_center += point
				continue
			var wood := sim_host.wood_density_at(x, z)
			wood_sum += wood
			wood_center += point * wood
			if wood >= BIRD_WOOD:
				_wood_points.append(point)
	var count := float(GRID * GRID)
	water_share = water / count
	wood_share = wood_sum / count
	_water_center = water_center / water if water > 0 else center
	_wood_center = wood_center / wood_sum if wood_sum > 0.0 else center
	for bed in ["water", "leaves"]:
		var player: AudioStreamPlayer3D = _beds[bed]
		var at := _water_center if bed == "water" else _wood_center
		at.y = sim_host.height_at(at.x, at.z) + 1.0
		player.global_position = at


func update_targets() -> void:
	var distance: float = distance_override if distance_override is float else main.camera_rig.distance
	near = clampf(inverse_lerp(log(FAR_DISTANCE), log(NEAR_DISTANCE), log(maxf(distance, 1.0))), 0.0, 1.0)
	var spec: Dictionary = BIOMES.get(biome, BIOMES.forest)
	var detail := _detail_db()
	_target_db["wind"] = WIND_DB + lerpf(0.0, NEAR_WIND_DB, near) + 3.0 * rain
	_target_db["insects"] = INSECTS_DB + float(spec.insects_db) + detail - 18.0 * rain if not str(spec.insects).is_empty() else SILENT_DB
	_target_db["leaves"] = _share_db(LEAVES_DB + float(spec.leaves_db) + detail, wood_share)
	_target_db["water"] = _share_db(WATER_DB + detail, water_share)
	_target_db["rain"] = RAIN_DB + linear_to_db(rain) if rain > 0.01 else SILENT_DB


## Level of a source by its share of the grid: quieter with a smaller
## share, silent below 2 %.
func _share_db(level: float, share: float) -> float:
	return level + 10.0 * log(share) / log(10.0) if share >= 0.02 else SILENT_DB


func _detail_db() -> float:
	return lerpf(FAR_DETAIL_DB, 0.0, near)


func _pick(layer: String) -> AudioStream:
	var list := streams(layer)
	return null if list.is_empty() else list[randi() % list.size()]


func _hearing_scale() -> float:
	return float(battle_sound.get(&"hearing_scale")) if battle_sound != null else 1.0


func _listener_position() -> Vector3:
	var listener: Node3D = battle_sound.get(&"listener") if battle_sound != null else null
	return listener.global_position if listener != null else Vector3.ZERO


## Center of the grid on the ground (listener, or `center_override`).
func _center() -> Vector3:
	if center_override is Vector3:
		return center_override
	var at := _listener_position()
	return Vector3(at.x, 0.0, at.z)


## Whether terrain id `terrain` is water (shallow or deep).
static func is_water(terrain: int) -> bool:
	return terrain == WATER_TERRAIN or terrain == WATER_TERRAIN_DEEP
