extends Node3D
## Burning map objects (prompt 041). Presentation only.
##
## Reads what burns from the simulation (`SimHost.fires`, `fire_keys`,
## `fire_life`) a few times per second: every burning object gets flames
## sized to the object, a dark smoke column and, up to the quality budget,
## a flickering light that carries at night. Flames die down in the last
## fifth of the burn. When a fire burns out, `map_objects.gd` draws the
## object charred (`set_charred`). Wreck fires are drawn by `effects.gd`
## (prompt 019) and skipped here; the dark smoke clouds that block sight are
## simulation clouds drawn by `artillery_view.gd`. Godot randomness only
## varies the flicker. Created by `main.gd`.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const LIGHT_COLOR := Color(1.0, 0.5, 0.18)
## Seconds between reads of the simulation's fires.
const POLL_SECONDS := 0.2
## Most burning objects with particles per `fx_fires` of the quality.
const FLAMES_PER_BUDGET := 3

var main: Node3D
var sim_host: SimHost
## Object index -> {root, fires, smoke, light, position, size}.
var _fires: Dictionary[int, Dictionary] = {}
## Every object seen burning (drawn or over the budget).
var _known: Dictionary[int, bool] = {}
var _poll := 0.0
var _time := 0.0
var _settings: Dictionary = {}
## Fires started and burned out since the start (self-test, tools).
var started := 0
var burnt_out := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_settings = GraphicsQuality.settings(_quality())
	# A loaded game: objects that burned out before are charred.
	for index: int in sim_host.burnt_objects():
		main.map_objects.set_charred(index, 1.0)


func _quality() -> String:
	var atmosphere: Node = main.atmosphere
	return atmosphere.quality if atmosphere != null else GraphicsQuality.DEFAULT


## Burning objects drawn with flames right now (self-test).
func flames() -> int:
	return _fires.size()


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_time += delta
	_poll -= delta
	if _poll <= 0.0:
		_poll = POLL_SECONDS
		_settings = GraphicsQuality.settings(_quality())
		_sync()
	_flicker()


func _sync() -> void:
	var fires := sim_host.fires()
	var keys := sim_host.fire_keys()
	var life := sim_host.fire_life()
	var live: Dictionary[int, bool] = {}
	var budget := int(_settings.get("fx_fires", 10)) * FLAMES_PER_BUDGET
	var visible_at: Callable = main.effects.visible_at if main.effects != null else Callable()
	for i in keys.size():
		var key := keys[i]
		if key < 0:
			continue  # Wrecks: effects.gd.
		live[key] = true
		if not _known.has(key):
			_known[key] = true
			started += 1
		var f := fires[i]
		if not _fires.has(key):
			if _fires.size() >= budget:
				continue
			_fires[key] = _start(key, Vector3(f.x, f.y, f.z), f.w)
		var entry: Dictionary = _fires[key]
		# Flames die down in the last fifth of the burn.
		var strength := clampf(life[i] * 5.0, 0.15, 1.0)
		for fire: GPUParticles3D in entry.fires:
			fire.amount_ratio = strength
		(entry.smoke as GPUParticles3D).amount_ratio = clampf(strength, 0.3, 1.0)
		entry.strength = strength
		if visible_at.is_valid():
			(entry.root as Node3D).visible = visible_at.call(entry.position)
	for key: int in _known.keys():
		if not live.has(key):
			_stop(key)


## Flames over the object's outline, a smoke column, maybe a light.
func _start(index: int, at: Vector3, size: float) -> Dictionary:
	var root := Node3D.new()
	root.name = "Fire%d" % index
	add_child(root)
	root.global_position = at
	var wind := _wind()
	var fires: Array = []
	# One flame emitter per ~4 m of the object, at most four.
	var count := clampi(int(size / 4.0) + 1, 1, 4)
	var scale := clampf(size / 2.5, 0.8, 3.0)
	for k in count:
		var fire := EffectLibrary.build("fire", _settings.fx_particles, _settings.fx_soft, wind)
		root.add_child(fire)
		var angle := TAU * float(k) / float(count) + 0.6
		var r := size * 0.45 if count > 1 else 0.0
		fire.position = Vector3(cos(angle) * r, 0.4 + size * 0.3, sin(angle) * r)
		fire.scale = Vector3.ONE * scale
		fire.emitting = true
		fires.append(fire)
	var smoke := EffectLibrary.build("smoke_column", _settings.fx_particles, _settings.fx_soft, wind)
	root.add_child(smoke)
	smoke.position = Vector3(0, 1.0 + size * 0.5, 0)
	smoke.scale = Vector3.ONE * clampf(size / 4.0, 0.8, 2.5)
	smoke.emitting = true
	var light: OmniLight3D = null
	var lit := 0
	for entry: Dictionary in _fires.values():
		if entry.light != null:
			lit += 1
	if lit < int(_settings.get("fx_fire_lights", 0)):
		light = OmniLight3D.new()
		light.light_color = LIGHT_COLOR
		light.omni_range = 10.0 + size * 2.0
		light.light_energy = 2.0
		light.shadow_enabled = false
		root.add_child(light)
		light.position = Vector3(0, 1.5 + size * 0.3, 0)
	return {"root": root, "fires": fires, "smoke": smoke, "light": light, "position": at,
		"strength": 1.0, "phase": randf() * TAU}


func _stop(index: int) -> void:
	_known.erase(index)
	if _fires.has(index):
		var entry: Dictionary = _fires[index]
		_fires.erase(index)
		(entry.root as Node).queue_free()
	if main.map_objects != null:
		main.map_objects.set_charred(index, 1.0)
	burnt_out += 1


func _flicker() -> void:
	for entry: Dictionary in _fires.values():
		var light: OmniLight3D = entry.light
		if light == null:
			continue
		var p: float = entry.phase
		light.light_energy = 2.4 * float(entry.strength) * (0.75 + 0.25 * sin(_time * 11.0 + p) * sin(_time * 6.7 + p * 2.0))


func _wind() -> Vector2:
	var atmosphere: Node = main.atmosphere
	if atmosphere == null or atmosphere.preset == null:
		return Vector2.ZERO
	var direction: Vector2 = atmosphere.preset.wind_direction
	return direction.normalized() * (0.5 + 2.0 * atmosphere.preset.wind_strength)
