extends Node
## Acoustics (prompt 053): how the battle reaches the ear. Created and
## driven by `battle_sound.gd` (`update` from its `_process`, so its time
## counts in `process_usec`); presentation only.
##
## - **Speed of sound.** 343 m/s real × time factor 2 = `SPEED_OF_SOUND`
##   game m/s. Sounds from `play_far` wait for their ground distance to the
##   camera focus / speed (at most `MAX_DELAY`): the far gun flashes, its
##   report comes later. A sorted queue of at most `QUEUE_MAX`; when full,
##   the weakest entry (priority) gives way.
## - **Supersonic crack.** Once per simulation tick every projectile faster
##   than sound whose closest point to the hearing center (focus + 1.5 m)
##   lies on this tick's stretch of its flight and nearer than
##   `CRACK_RADIUS_BULLET` / `_SHELL` snaps at that point, at once: the
##   crack comes before the delayed muzzle report, as in reality. Only up
##   to `CRACK_MAX_CAMERA` m camera distance.
## - **Reverb by surroundings.** Every `ENV_INTERVAL` s a 5 × 5 grid of wood
##   density and the buildings near the focus (`SimHost.buildings_near`)
##   give shares of forest, village and open field; one `AudioEffectReverb`
##   on the Effects bus blends their presets.
## - **Zoom.** A low-pass on the Effects bus falls from 20 kHz close up to
##   `FAR_CUTOFF_HZ` in the overview, and small-arms classes (priority ≤ 1)
##   lose up to `DETAIL_FAR_DB`: close up the details, far a dull rumble.
## - **Aircraft.** The `AIRCRAFT_VOICES` nearest aircraft get an engine
##   loop (`engine_bomber` from `BOMBER_WINGSPAN_M`, else
##   `engine_aircraft`), pitch by Doppler from their radial speed; flak,
##   strafing and crashes from `air_view.air_event`.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")

## Game meters per second: 343 m/s real × time factor 2 (docs/SCALE.md).
const SPEED_OF_SOUND := 686.0
## [provisional] Longest delay, so the overview stays readable.
const MAX_DELAY := 2.5
## Below this delay a sound plays at once.
const MIN_DELAY := 0.04
const QUEUE_MAX := 192
## Supersonic crack.
const CRACK_RADIUS_BULLET := 12.0
const CRACK_RADIUS_SHELL := 25.0
const CRACK_MAX_CAMERA := 200.0
const CRACK_FAR_DB := -10.0
const EAR_HEIGHT := 1.5
const SHELL_MM := 20.0
## Surroundings.
const ENV_INTERVAL := 0.5
const ENV_GRID := 5
const ENV_RADIUS := 35.0
## Wood density (0..1) that counts as full forest, buildings for a village.
const FOREST_DENSITY := 0.6
const VILLAGE_BUILDINGS := 8.0
## How fast the reverb follows a new surrounding (1/s).
const ENV_SMOOTHING := 1.5
## [provisional] Reverb presets, set without listening.
const REVERB := {
	"open": {"room_size": 0.25, "damping": 0.5, "wet": 0.04, "predelay_msec": 80.0, "hipass": 0.2},
	"forest": {"room_size": 0.65, "damping": 0.85, "wet": 0.16, "predelay_msec": 25.0, "hipass": 0.1},
	"village": {"room_size": 0.35, "damping": 0.25, "wet": 0.2, "predelay_msec": 10.0, "hipass": 0.15},
}
## Zoom: camera distances of the near and far mix.
const NEAR_CAMERA := 40.0
const FAR_CAMERA := 450.0
const NEAR_CUTOFF_HZ := 20000.0
const FAR_CUTOFF_HZ := 2200.0
const DETAIL_FAR_DB := -8.0
## Aircraft.
const AIRCRAFT_VOICES := 3
const AIRCRAFT_INTERVAL := 0.2
const BOMBER_WINGSPAN_M := 18.0
const DOPPLER_MIN := 0.75
const DOPPLER_MAX := 1.35
const FALLING_PITCH := 1.15
const AIR_SMOOTHING := 4.0

var battle_sound: Node3D
var main: Node3D
var sim_host: SimHost
## Shares of the surroundings now (sum 1) and the blended reverb values.
var surroundings := {"open": 1.0, "forest": 0.0, "village": 0.0}
var reverb_now := {}
## Statistics (self-test, benchmark): sounds delayed, dropped from a full
## queue, cracks played, aircraft loops started, air events heard.
var delayed := 0
var queue_dropped := 0
var cracks := 0
var aircraft_loops := 0
var air_events := 0
## 0 close up .. 1 overview (camera distance).
var far := 0.0
## The camera's ground focus and distance, read once per frame.
var focus := Vector3.ZERO
var camera_distance := 0.0

var _time := 0.0
## [due, class, at, gain, pitch, priority], sorted by due.
var _queue: Array = []
var _by_due := func(a: Array, b: Array) -> bool: return a[0] < b[0]
var _last_tick := -1
var _env_timer := 0.0
var _env_target := {"open": 1.0, "forest": 0.0, "village": 0.0}
var _low_pass: AudioEffectLowPassFilter
var _reverb: AudioEffectReverb
var _air_timer := 0.0
var _air_slots: Array[Dictionary] = []
var _wingspans: Dictionary[String, float] = {}


func setup(sound: Node3D) -> void:
	battle_sound = sound
	main = sound.main
	sim_host = sound.sim_host
	var chain := SoundBuses.ensure_effects()
	_low_pass = chain[0]
	_reverb = chain[1]
	reverb_now = REVERB.open.duplicate()
	focus = main.camera_rig.focus
	camera_distance = main.camera_rig.distance
	for i in AIRCRAFT_VOICES:
		_air_slots.append({"player": battle_sound._new_player(), "aircraft": -1, "sound": "", "pitch": 1.0})
	var air: Object = main.get("air_view")
	if air != null and air.has_signal(&"air_event"):
		air.connect(&"air_event", on_air_event)


func _exit_tree() -> void:
	SoundBuses.reset_effects()


## Called by `battle_sound._process` after the frame budgets are set.
func update(delta: float) -> void:
	_time += delta
	focus = main.camera_rig.focus
	camera_distance = main.camera_rig.distance
	var distance := camera_distance
	far = clampf(log(maxf(distance, 1.0) / NEAR_CAMERA) / log(FAR_CAMERA / NEAR_CAMERA), 0.0, 1.0)
	_low_pass.cutoff_hz = NEAR_CUTOFF_HZ * pow(FAR_CUTOFF_HZ / NEAR_CUTOFF_HZ, far)
	_flush()
	var tick := sim_host.current_tick()
	if tick != _last_tick:
		if _last_tick >= 0 and distance <= CRACK_MAX_CAMERA:
			var dt := float(clampi(tick - _last_tick, 1, 3)) / float(sim_host.tick_rate())
			scan_cracks(sim_host.projectile_positions(), sim_host.projectile_velocities(), sim_host.projectile_calibers(), dt)
		_last_tick = tick
	_env_timer -= delta
	if _env_timer <= 0.0:
		_env_timer = ENV_INTERVAL
		_sample_surroundings()
	_blend_reverb(delta)
	_air_timer -= delta
	if _air_timer <= 0.0:
		_air_timer = AIRCRAFT_INTERVAL
		_assign_aircraft()
	_update_aircraft(delta)


# --- Speed of sound -------------------------------------------------------------


## Seconds a sound at `at` needs to the camera focus.
func delay_of(at: Vector3) -> float:
	return minf(Vector2(at.x - focus.x, at.z - focus.z).length() / SPEED_OF_SOUND, MAX_DELAY)


## Queues a sound for its travel time; false if it should play now.
func queue(sound_class: String, at: Vector3, gain_db: float, pitch: float, priority: int) -> bool:
	var wait := delay_of(at)
	if wait < MIN_DELAY:
		return false
	if _queue.size() >= QUEUE_MAX:
		# Small arms never push anything out: no scan for them.
		if priority <= 1:
			queue_dropped += 1
			return true
		var weakest := 0
		for i in _queue.size():
			if int(_queue[i][5]) < int(_queue[weakest][5]):
				weakest = i
		if int(_queue[weakest][5]) > priority:
			queue_dropped += 1
			return true
		_queue.remove_at(weakest)
		queue_dropped += 1
	var entry := [_time + wait, sound_class, at, gain_db, pitch, priority]
	var index := _queue.bsearch_custom(entry, _by_due)
	_queue.insert(index, entry)
	delayed += 1
	return true


## Sounds waiting for their travel time (self-test).
func queued() -> int:
	return _queue.size()


func _flush() -> void:
	while not _queue.is_empty() and float(_queue[0][0]) <= _time:
		var entry: Array = _queue.pop_front()
		battle_sound._play(entry[1], entry[2], entry[3], entry[4])


# --- Supersonic crack ------------------------------------------------------------


## Plays a crack for every supersonic projectile that passes the hearing
## center on its next `dt` seconds of flight (public for the self-test).
func scan_cracks(positions: PackedVector3Array, velocities: PackedVector3Array, calibers: PackedFloat32Array, dt: float) -> void:
	var ear: Vector3 = focus + Vector3.UP * EAR_HEIGHT
	var limit := SPEED_OF_SOUND * SPEED_OF_SOUND
	var gain := CRACK_FAR_DB * clampf((camera_distance - NEAR_CAMERA) / (CRACK_MAX_CAMERA - NEAR_CAMERA), 0.0, 1.0)
	for i in positions.size():
		var v := velocities[i]
		var speed2 := v.length_squared()
		if speed2 <= limit:
			continue
		var s := (ear - positions[i]).dot(v) / speed2
		if s < 0.0 or s >= dt:
			continue
		var closest := positions[i] + v * s
		var shell := calibers[i] >= SHELL_MM
		var radius := CRACK_RADIUS_SHELL if shell else CRACK_RADIUS_BULLET
		if closest.distance_squared_to(ear) > radius * radius:
			continue
		if battle_sound.play_at("crack_shell" if shell else "crack_bullet", closest, gain):
			cracks += 1


# --- Surroundings and zoom ---------------------------------------------------------


func _sample_surroundings() -> void:
	var radius: float = ENV_RADIUS * clampf(battle_sound.hearing_scale, 1.0, 2.0)
	var wood := 0.0
	for gx in ENV_GRID:
		for gz in ENV_GRID:
			var x := focus.x + (float(gx) / float(ENV_GRID - 1) * 2.0 - 1.0) * radius
			var z := focus.z + (float(gz) / float(ENV_GRID - 1) * 2.0 - 1.0) * radius
			wood += sim_host.wood_density_at(x, z)
	wood /= float(ENV_GRID * ENV_GRID)
	var forest := clampf(wood / FOREST_DENSITY, 0.0, 1.0)
	var village := clampf(float(sim_host.buildings_near(focus.x, focus.z, radius)) / VILLAGE_BUILDINGS, 0.0, 1.0)
	var total := forest + village
	if total > 1.0:
		forest /= total
		village /= total
	_env_target = {"open": 1.0 - forest - village, "forest": forest, "village": village}


func _blend_reverb(delta: float) -> void:
	var k := clampf(delta * ENV_SMOOTHING, 0.0, 1.0)
	for key: String in surroundings:
		surroundings[key] = lerpf(surroundings[key], _env_target[key], k)
	for param: String in REVERB.open:
		var value := 0.0
		for key: String in surroundings:
			value += float(REVERB[key][param]) * float(surroundings[key])
		reverb_now[param] = value
		_reverb.set(param, value)


## Extra level of a class for the zoom: small arms fade in the overview.
func detail_db(spec: Dictionary) -> float:
	return DETAIL_FAR_DB * far if int(spec.get("priority", 1)) <= 1 else 0.0


# --- Aircraft ----------------------------------------------------------------------


func _assign_aircraft() -> void:
	var ids := sim_host.aircraft_ids()
	var positions := sim_host.aircraft_positions()
	var types := sim_host.aircraft_types()
	var hear: Vector3 = battle_sound.listener.global_position
	var candidates: Array = []
	for i in ids.size():
		var sound_class := _aircraft_class(types[i])
		var reach: float = float(SoundLibrary.spec(sound_class).get("range_m", 1200.0)) * battle_sound.hearing_scale
		var d := hear.distance_to(positions[i])
		if d < reach:
			candidates.append([d, ids[i], sound_class])
	candidates.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var wanted: Dictionary[int, String] = {}
	for c: Array in candidates.slice(0, AIRCRAFT_VOICES):
		wanted[c[1]] = c[2]
	var free: Array[Dictionary] = []
	for slot in _air_slots:
		if wanted.has(slot.aircraft):
			wanted.erase(slot.aircraft)
		else:
			free.append(slot)
	for slot in free:
		var player: AudioStreamPlayer3D = slot.player
		if wanted.is_empty():
			slot.aircraft = -1
			player.stop()
			continue
		var id: int = wanted.keys()[0]
		slot.aircraft = id
		slot.sound = wanted[id]
		wanted.erase(id)
		var stream := SoundLibrary.pick(slot.sound)
		if stream == null:
			slot.aircraft = -1
			continue
		var spec := SoundLibrary.spec(slot.sound)
		player.stream = stream
		player.bus = "Effects"
		player.unit_size = float(spec.get("ref_m", 60.0)) * battle_sound.hearing_scale
		player.max_distance = float(spec.get("range_m", 1200.0)) * battle_sound.hearing_scale
		player.volume_db = -40.0
		player.play(randf() * maxf(stream.get_length() - 0.1, 0.0))
		aircraft_loops += 1
		battle_sound.played[slot.sound] = battle_sound.played.get(slot.sound, 0) + 1


func _update_aircraft(delta: float) -> void:
	var busy := false
	for slot in _air_slots:
		busy = busy or slot.aircraft >= 0
	if not busy:
		return
	var ids := sim_host.aircraft_ids()
	var positions := sim_host.aircraft_positions()
	var velocities := sim_host.aircraft_velocities()
	var health := sim_host.aircraft_health()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	var hear: Vector3 = battle_sound.listener.global_position
	var smooth := clampf(delta * AIR_SMOOTHING, 0.0, 1.0)
	for slot in _air_slots:
		if slot.aircraft < 0:
			continue
		var index := ids.find(slot.aircraft)
		var player: AudioStreamPlayer3D = slot.player
		if index < 0:
			slot.aircraft = -1
			player.stop()
			continue
		var velocity := velocities[index]
		var at := positions[index] + velocity * ahead
		player.global_position = at
		var toward := (hear - at).normalized()
		var radial := velocity.dot(toward)
		var pitch := clampf(SPEED_OF_SOUND / maxf(SPEED_OF_SOUND - radial, 1.0), DOPPLER_MIN, DOPPLER_MAX)
		if health[index] < 0.0:
			pitch *= FALLING_PITCH
		slot.pitch = lerpf(slot.pitch, pitch, smooth)
		player.pitch_scale = slot.pitch
		player.volume_db = lerpf(player.volume_db, float(SoundLibrary.spec(slot.sound).get("volume_db", 0.0)), smooth)


func _aircraft_class(type_id: String) -> String:
	if not _wingspans.has(type_id):
		_wingspans[type_id] = sim_host.aircraft_wingspan(type_id)
	return "engine_bomber" if _wingspans[type_id] >= BOMBER_WINGSPAN_M else "engine_aircraft"


## Aircraft engines playing right now (self-test).
func aircraft_playing() -> int:
	var n := 0
	for slot in _air_slots:
		n += 1 if slot.aircraft >= 0 else 0
	return n


## An air event (`air_view.air_event`): flak, strafing, crashes.
func on_air_event(event: Dictionary) -> void:
	match str(event.get("type", "")):
		"flak":
			air_events += 1
			var at: Vector3 = event["position"]
			match str(event.get("class", "")):
				"heavy":
					battle_sound.play_far("flak_burst", at)
				"light":
					battle_sound.play_far("shot_cannon_small", _gun_or(int(event.get("unit", -1)), at), -3.0)
				_:
					battle_sound.play_far("shot_mg", _gun_or(int(event.get("unit", -1)), at), -3.0)
		"strafe":
			air_events += 1
			battle_sound.play_far("shot_mg", event.get("muzzle", Vector3.ZERO), 3.0, 0.9)
		"crashed":
			air_events += 1
			var at: Vector3 = event["position"]
			battle_sound.play_far("explosion_vehicle", at)
			battle_sound.play_far("explosion_tail", at)


func _gun_or(unit: int, fallback: Vector3) -> Vector3:
	var index: int = battle_sound._index_of(unit) if unit >= 0 else -1
	return sim_host.current_positions()[index] + Vector3.UP if index >= 0 else fallback
