extends Node
## Music of a match (prompt 030): calm or combat by how much fighting the
## player sees, victory or defeat at the end. Created by
## `battle_sound.gd`; plays through the shared `music.gd`.
##
## Intensity: every combat event the player sees (`battle_sound.visible_at`)
## adds a weight (small-arms shot 0.5, gun shot 1.5, heavy hit 1, destroyed
## unit 6) and the sum decays with `DECAY_SECONDS`. Combat starts at
## `COMBAT_ABOVE` and ends below `CALM_BELOW`, not before
## `MIN_COMBAT_SECONDS`: the hysteresis keeps a single shot from switching
## and a pause in the fight from switching back at once. Fighting in the fog
## is only heard (muffled), it does not change the music. The menu music
## plays on behind the loading screen; calm starts with the match.
## [provisional] Weights and thresholds set without listening.

const Music := preload("res://scenes/audio/music.gd")

const DECAY_SECONDS := 10.0
const COMBAT_ABOVE := 12.0
const CALM_BELOW := 4.0
const MIN_COMBAT_SECONDS := 25.0
const WEIGHT_SMALL_SHOT := 0.5
const WEIGHT_GUN_SHOT := 1.5
const WEIGHT_HEAVY_HIT := 1.0
const WEIGHT_DESTROYED := 6.0
## Calibers from this size count as guns and heavy hits (mm).
const GUN_MM := 20.0
## How often the match result is read (s).
const RESULT_INTERVAL := 1.0

var main: Node3D
var sim_host: SimHost
var battle_sound: Node3D
## Current intensity and the highest one so far (self-test).
var intensity := 0.0
var peak_intensity := 0.0
## "calm", "combat" or the result; changes since the start (self-test).
var state := ""
var switches: PackedStringArray = []
var _time := 0.0
var _combat_since := 0.0
var _result_timer := 0.0


func setup(main_node: Node3D, sound: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	battle_sound = sound
	main.combat_view.combat_event.connect(on_combat_event)


func _process(delta: float) -> void:
	advance(delta)


## Moves the music's clock by `seconds` (also used by the self-test).
func advance(seconds: float) -> void:
	_time += seconds
	intensity *= exp(-seconds / DECAY_SECONDS)
	# The menu music plays on behind the loading screen.
	if state.is_empty():
		if main.get(&"_loaded") == false:
			return
		_set_state("calm")
	_result_timer -= seconds
	if _result_timer <= 0.0 and state in ["calm", "combat"]:
		_result_timer = RESULT_INTERVAL
		var result := sim_host.match_result()
		if result.get("decided", false):
			var winner := int(result.get("winner", -1))
			if winner == sim_host.local_player():
				_set_state("victory")
			elif winner >= 0:
				_set_state("defeat")
			else:
				_set_state("draw")
			return
	if state == "calm" and intensity >= COMBAT_ABOVE:
		_set_state("combat")
	elif state == "combat" and intensity < CALM_BELOW and _time - _combat_since >= MIN_COMBAT_SECONDS:
		_set_state("calm")


## Weighs one combat event (`combat_view.combat_event`).
func on_combat_event(event: Dictionary) -> void:
	var at: Vector3 = event.get("position", Vector3.ZERO)
	if not battle_sound.visible_at.call(at):
		return
	var caliber := float(event.get("caliber_mm", 0.0))
	match event.get("type", ""):
		"shot":
			add(WEIGHT_GUN_SHOT if caliber >= GUN_MM else WEIGHT_SMALL_SHOT)
		"hit":
			if caliber >= GUN_MM:
				add(WEIGHT_HEAVY_HIT)
		"destroyed":
			add(WEIGHT_DESTROYED)


func add(weight: float) -> void:
	intensity += weight
	peak_intensity = maxf(peak_intensity, intensity)


func _set_state(new_state: String) -> void:
	if new_state == state:
		return
	state = new_state
	switches.append(new_state)
	if new_state == "combat":
		_combat_since = _time
	if new_state == "draw":
		Music.stop()
	else:
		Music.play(new_state)
