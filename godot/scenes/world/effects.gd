extends Node3D
## Battle effects (prompt 019): muzzle flashes and powder smoke, impacts
## (earth fountains, water spray, sparks), explosions with fireball, light
## flash, debris and camera shake, burning wrecks with smoke columns, dust
## trails and exhaust smoke behind vehicles, and scorch decals.
##
## Presentation only. Triggered by the combat events of the simulation
## (`combat_view.combat_event`) and by the snapshot (positions,
## `unit_moving`, terrain under the units). Nothing here is read by the
## simulation; Godot randomness is fine because it never reaches the state
## hash.
##
## Budget: pools of particle systems per effect kind (the oldest burst is
## reused when a pool is exhausted), a cap on light flashes, scorch decals
## and burning wrecks, all scaled by the graphics quality
## (`graphics_quality.gd`, keys `fx_*`). LOD: bursts that would be only a few
## pixels on screen are skipped, small ones get fewer particles; dust and
## exhaust run only within `fx_vehicle_m` of the camera.
##
## Fog of war (prompt 018) can set `visible_at`: new effects only appear at
## visible points, and burning wrecks and vehicle effects are hidden while
## their position is not visible.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

## Emitters per burst kind at quality "high" (scaled by fx_pools). Sized so
## 50 explosions at once fit at high.
const POOL_SIZES: Dictionary[String, int] = {
	"flash": 32, "powder": 40, "blast_dust": 56, "fountain": 48, "spray": 16,
	"impact_dust": 48, "puff": 48, "sparks": 56, "fireball": 56,
	"explosion_smoke": 56, "debris": 56,
}
## Ground look per terrain id (data/terrain.toml) and biome: dust color kind
## and dust trail strength (0..1). Desert draws grass and snow as sand, winter
## grass and sand as snow (like the terrain shader, prompt 015).
const GROUND_COLORS: Dictionary[String, Color] = {
	"earth": Color(0.37, 0.3, 0.22), "sand": Color(0.82, 0.72, 0.54), "snow": Color(0.93, 0.95, 0.99),
	"mud": Color(0.21, 0.17, 0.13), "road": Color(0.55, 0.53, 0.5), "water": Color(0.88, 0.92, 0.96),
	"object": Color(0.5, 0.46, 0.4),
}
const TERRAIN_WATER_SHALLOW := 6
const TERRAIN_WATER_DEEP := 7
## Below this caliber (mm) a round is small arms: tiny flashes and puffs.
const SMALL_CALIBER := 20.0
## Bursts smaller than this on screen (pixels) are skipped.
const MIN_PIXELS := 3.0
## A burst of this many pixels gets all its particles; smaller ones fewer.
const FULL_PIXELS := 60.0
## Burning wrecks: fire and dense smoke, then smoke only, then nothing.
const BURN_SECONDS := 60.0
const SMOLDER_SECONDS := 90.0
## Camera shake: offset per unit of shake as a share of the camera distance,
## and how fast it decays (1/s).
const SHAKE_SCALE := 0.003
const SHAKE_DECAY := 8.0
## Decals fade out beyond this camera distance (meters).
const DECAL_FADE_M := 450.0
const FIRE_LIGHT_COLOR := Color(1.0, 0.55, 0.2)
const FLASH_LIGHT_COLOR := Color(1.0, 0.72, 0.4)

## Visibility of a world point for new and persistent effects (fog of war,
## prompt 018). Default: everything is visible.
var visible_at: Callable = func(_point: Vector3) -> bool: return true
## Camera shake on/off (a later setting).
var shake_enabled := true
## Bursts, lights, decals and wrecks started since the start (self-test,
## tools); key = effect kind.
var counts: Dictionary[String, int] = {}

var main: Node3D
var sim_host: SimHost
var _settings: Dictionary = {}
var _config_key := ""
var _pools: Dictionary[String, Array] = {}
var _pool_next: Dictionary[String, int] = {}
var _lights: Array[Dictionary] = []
var _decals: Array[Decal] = []
var _decal_next := 0
var _wrecks: Array[Dictionary] = []
## Unit id -> {dust, exhausts, kind, strength, last, frame}.
var _vehicles: Dictionary[int, Dictionary] = {}
## Unit id -> sockets of the model (muzzles, fires, exhausts).
var _sockets: Dictionary[int, Dictionary] = {}
var _index_cache: Dictionary[int, int] = {}
var _index_frame := -1
var _shake := 0.0
var _time := 0.0
var _visibility_timer := 0.0
var _bursts_root: Node3D


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_bursts_root = Node3D.new()
	_bursts_root.name = "Bursts"
	add_child(_bursts_root)
	_configure()
	main.combat_view.combat_event.connect(on_combat_event)


## Drops the cached shaders and textures (else they leak at exit).
func _exit_tree() -> void:
	EffectLibrary.clear_cache()
	EffectTextures.clear_cache()


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_time += delta
	_configure()
	_update_lights(delta)
	_update_wrecks(delta)
	_update_vehicles(delta)
	_update_shake(delta)
	_visibility_timer -= delta
	if _visibility_timer <= 0.0:
		_visibility_timer = 0.5
		for wreck in _wrecks:
			(wreck.root as Node3D).visible = visible_at.call(wreck.position)


# --- Events -----------------------------------------------------------------


## Turns one combat event (see `SimHost.take_combat_events`) into effects.
func on_combat_event(event: Dictionary) -> void:
	match event["type"]:
		"shot":
			_shot(event["unit"], event["position"], event["caliber_mm"])
		"hit":
			match event["surface"]:
				"ground":
					ground_hit(event["position"], event["caliber_mm"])
				"unit":
					unit_hit(event["position"], event["caliber_mm"], event["penetrated"])
				_:
					object_hit(event["position"], event["caliber_mm"])
		"destroyed":
			destroyed(event["unit"], event["position"])


func _shot(unit: int, sim_muzzle: Vector3, caliber: float) -> void:
	var muzzle := _muzzle_of(unit, sim_muzzle)
	var index := _index_of(unit)
	var yaw: float = sim_host.unit_aim_yaws()[index] if index >= 0 else 0.0
	muzzle_flash(muzzle, Vector3(-sin(yaw), 0.0, -cos(yaw)), caliber)


## Muzzle flash, powder smoke and (large guns) dust blown off the ground.
func muzzle_flash(at: Vector3, direction: Vector3, caliber: float) -> void:
	if not visible_at.call(at):
		return
	var facing := Basis.looking_at(direction.normalized() if direction.length() > 0.01 else Vector3.FORWARD)
	if caliber < SMALL_CALIBER:
		_burst("flash", at, facing, 0.25)
		return
	var s := clampf(caliber / 75.0, 0.4, 1.6)
	_burst("flash", at, facing, s)
	_burst("powder", at, facing, s)
	_light(at, 3.0 * s, 6.0 + 8.0 * s, 0.08)
	if caliber >= 37.0:
		var ground := at + direction * 2.5 * s
		ground.y = sim_host.height_at(ground.x, ground.z) + 0.2
		_burst("blast_dust", ground, Basis.IDENTITY, s * 0.8, _ground_color(ground))
	_add_shake(at, 0.08 * s)


## Impact on the ground: earth fountain and dust (water: spray), a scorch
## mark for shells.
func ground_hit(at: Vector3, caliber: float) -> void:
	if not visible_at.call(at):
		return
	var kind := _ground_kind(sim_host.terrain_at(at.x, at.z))
	var color: Color = GROUND_COLORS[kind]
	if caliber < SMALL_CALIBER:
		_burst("puff", at, Basis.IDENTITY, 0.8, color)
		return
	var s := clampf(caliber / 75.0, 0.35, 1.8)
	if kind == "water":
		_burst("spray", at, Basis.IDENTITY, s, color)
		_burst("puff", at, Basis.IDENTITY, s * 2.0, color)
		return
	_burst("fountain", at, Basis.IDENTITY, s, color.darkened(0.15))
	_burst("impact_dust", at, Basis.IDENTITY, s, color)
	if caliber >= 37.0:
		_burst("sparks", at, Basis.IDENTITY, s * 0.5)
		_decal(at, 1.4 + 1.6 * s)
		_light(at + Vector3.UP, 2.0 * s, 8.0 * s, 0.1)
	_add_shake(at, 0.25 * s)


## Impact on a unit: sparks; a penetration also a short burst of fire.
func unit_hit(at: Vector3, caliber: float, penetrated: bool) -> void:
	if not visible_at.call(at):
		return
	if caliber < SMALL_CALIBER:
		_burst("sparks", at, Basis.IDENTITY, 0.25)
		return
	var s := clampf(caliber / 75.0, 0.35, 1.8)
	_burst("sparks", at, Basis.IDENTITY, s)
	_light(at, 2.5 * s, 7.0 * s, 0.09)
	if penetrated:
		_burst("fireball", at, Basis.IDENTITY, 0.5 * s)
		_burst("impact_dust", at, Basis.IDENTITY, 0.5 * s, Color(0.25, 0.24, 0.23))


## Impact on a map object (tree, wall, building): splinters and dust.
func object_hit(at: Vector3, caliber: float) -> void:
	if not visible_at.call(at):
		return
	var color: Color = GROUND_COLORS["object"]
	if caliber < SMALL_CALIBER:
		_burst("puff", at, Basis.IDENTITY, 0.6, color)
		return
	var s := clampf(caliber / 75.0, 0.35, 1.8)
	_burst("fountain", at, Basis.IDENTITY, 0.6 * s, color.darkened(0.3))
	_burst("impact_dust", at, Basis.IDENTITY, s, color)


## A unit is destroyed: vehicles explode (fireball, light, debris, smoke,
## scorch mark, camera shake) and keep burning; infantry leaves dust.
func destroyed(unit: int, at: Vector3) -> void:
	var index := _index_of(unit)
	var type_id: String = sim_host.unit_type_ids()[index] if index >= 0 else ""
	if type_id.contains("_inf_"):
		if visible_at.call(at):
			_burst("impact_dust", at, Basis.IDENTITY, 0.6, _ground_color(at))
		return
	var size := clampf((sim_host.unit_footprints()[index] if index >= 0 else 6.0) / 7.0, 0.6, 1.4)
	explosion(at, size)
	_start_wreck(unit, at)


## A map object changed its stage (prompt 020, `SimHost.take_object_events`):
## buildings throw debris and a dust cloud (more when they collapse), trees
## splinter at the trunk and raise dust where the crown lands, walls and
## hedges burst apart, crushed fences and wire puff a little dust.
func object_event(event: Dictionary) -> void:
	var at: Vector3 = sim_host.object_position(event.object)
	at.y = sim_host.height_at(at.x, at.z)
	if not visible_at.call(at):
		return
	var dust: Color = GROUND_COLORS["object"]
	var key := "object_" + str(event.remains if event.stage == "destroyed" else event.stage)
	counts[key] = counts.get(key, 0) + 1
	if event.stage == "damaged":
		var s := 1.2
		_burst("debris", at + Vector3.UP * 3.0, Basis.IDENTITY, s)
		_burst("impact_dust", at + Vector3.UP * 2.0, Basis.IDENTITY, s * 1.5, dust)
		_add_shake(at, 0.3)
		return
	match str(event.remains):
		"ruin":
			var s := 2.2
			_burst("debris", at + Vector3.UP * 4.0, Basis.IDENTITY, s)
			_burst("explosion_smoke", at + Vector3.UP * 3.0, Basis.IDENTITY, s, dust.lightened(0.15))
			_burst("blast_dust", at + Vector3.UP * 0.3, Basis.IDENTITY, s * 1.4, _ground_color(at))
			_burst("impact_dust", at + Vector3.UP * 5.0, Basis.IDENTITY, s * 1.5, dust)
			_decal(at, 9.0)
			_add_shake(at, 1.2)
		"fallen":
			var heading := deg_to_rad(float(event.fall_deg))
			var crown := at + Vector3(sin(heading), 0.0, -cos(heading)) * 12.0
			crown.y = sim_host.height_at(crown.x, crown.z) + 0.3
			_burst("debris", at + Vector3.UP * 1.5, Basis.IDENTITY, 0.6)
			_burst("impact_dust", at + Vector3.UP * 0.5, Basis.IDENTITY, 0.7, dust)
			_burst("blast_dust", crown, Basis.IDENTITY, 1.1, _ground_color(crown))
		"gone":
			_burst("debris", at + Vector3.UP, Basis.IDENTITY, 0.9)
			_burst("impact_dust", at + Vector3.UP * 0.6, Basis.IDENTITY, 1.2, dust)
			_burst("blast_dust", at + Vector3.UP * 0.2, Basis.IDENTITY, 0.8, _ground_color(at))
		_:
			_burst("puff", at + Vector3.UP * 0.3, Basis.IDENTITY, 1.2, _ground_color(at))


## A vehicle explosion of relative size `s` (1 = medium tank).
func explosion(at: Vector3, s := 1.0) -> void:
	if not visible_at.call(at):
		return
	var center := at + Vector3.UP * 1.2 * s
	_burst("fireball", center, Basis.IDENTITY, s)
	_burst("explosion_smoke", center, Basis.IDENTITY, s)
	_burst("sparks", center, Basis.IDENTITY, s * 1.3)
	_burst("debris", center, Basis.IDENTITY, s)
	var ground := at
	ground.y = sim_host.height_at(at.x, at.z) + 0.2
	_burst("blast_dust", ground, Basis.IDENTITY, s * 0.9, _ground_color(ground).darkened(0.2))
	_light(center + Vector3.UP, 10.0 * s, 28.0 * s, 0.4)
	_decal(ground, 7.0 * s)
	_add_shake(at, 1.0 * s)


# --- Bursts -----------------------------------------------------------------


## Fires the next emitter of a pool at `at`, scaled by `s`, tinted.
func _burst(kind: String, at: Vector3, basis: Basis, s: float, tint := Color.WHITE) -> void:
	var ratio := _lod(at, 3.0 * s)
	if ratio <= 0.0:
		return
	var pool: Array = _pools[kind]
	var next: int = _pool_next[kind]
	var p: GPUParticles3D = pool[next]
	_pool_next[kind] = (next + 1) % pool.size()
	p.visible = true
	p.global_transform = Transform3D(basis.scaled(Vector3.ONE * s), at)
	p.amount_ratio = ratio
	p.set_instance_shader_parameter(&"tint", tint)
	p.restart()
	counts[kind] = counts.get(kind, 0) + 1


## Share of particles for an effect of `size_m` at `at`: 0 = skip (too small
## on screen), up to 1.
func _lod(at: Vector3, size_m: float) -> float:
	var camera: Camera3D = main.camera
	var distance := camera.global_position.distance_to(at)
	# Headless runs have a tiny dummy window: assume full HD.
	var height := get_viewport().get_visible_rect().size.y
	if DisplayServer.get_name() == "headless":
		height = 1080.0
	var pixels := size_m * height / (2.0 * distance * tan(deg_to_rad(camera.fov) * 0.5) + 0.001)
	if pixels < MIN_PIXELS:
		return 0.0
	return clampf(pixels / FULL_PIXELS, 0.35, 1.0)


func _light(at: Vector3, energy: float, light_range: float, seconds: float) -> void:
	if _lights.is_empty():
		return
	# Reuse the light that has the least time left.
	var best: Dictionary = _lights[0]
	for entry in _lights:
		if entry.left < best.left:
			best = entry
	var light: OmniLight3D = best.light
	light.global_position = at
	light.omni_range = light_range
	light.light_color = FLASH_LIGHT_COLOR
	light.light_energy = energy
	light.visible = true
	best.left = seconds
	best.duration = seconds
	best.energy = energy
	counts["light"] = counts.get("light", 0) + 1


func _update_lights(delta: float) -> void:
	for entry in _lights:
		if entry.left <= 0.0:
			continue
		entry.left -= delta
		var light: OmniLight3D = entry.light
		if entry.left <= 0.0:
			light.visible = false
		else:
			var t: float = entry.left / entry.duration
			light.light_energy = entry.energy * t * t


func _decal(at: Vector3, size_m: float) -> void:
	var limit: int = _settings.fx_decals
	if limit <= 0:
		return
	var decal: Decal
	if _decals.size() < limit:
		decal = Decal.new()
		decal.texture_albedo = EffectTextures.scorch()
		decal.upper_fade = 0.4
		decal.lower_fade = 0.4
		decal.distance_fade_enabled = true
		decal.distance_fade_begin = DECAL_FADE_M
		decal.distance_fade_length = 100.0
		decal.cull_mask = 1
		_bursts_root.add_child(decal)
		_decals.append(decal)
	else:
		decal = _decals[_decal_next]
		_decal_next = (_decal_next + 1) % _decals.size()
	decal.size = Vector3(size_m, 2.5, size_m)
	decal.global_transform = Transform3D(Basis(Vector3.UP, randf() * TAU), at)
	counts["decal"] = counts.get("decal", 0) + 1


func _add_shake(at: Vector3, strength: float) -> void:
	if not shake_enabled:
		return
	var rig: Node3D = main.camera_rig
	var reach: float = rig.distance * 1.5 + 50.0
	var falloff := clampf(1.0 - (rig.focus as Vector3).distance_to(at) / reach, 0.0, 1.0)
	_shake = minf(_shake + strength * falloff, 1.5)


func _update_shake(delta: float) -> void:
	var camera: Camera3D = main.camera
	if _shake < 0.01:
		if _shake > 0.0:
			_shake = 0.0
			camera.h_offset = 0.0
			camera.v_offset = 0.0
		return
	var amplitude: float = _shake * SHAKE_SCALE * main.camera_rig.distance
	camera.h_offset = amplitude * 0.5 * (sin(_time * 47.0) + sin(_time * 31.7))
	camera.v_offset = amplitude * 0.5 * (sin(_time * 41.3) + sin(_time * 27.1))
	_shake *= exp(-delta * SHAKE_DECAY)


# --- Burning wrecks -----------------------------------------------------------


func _start_wreck(unit: int, at: Vector3) -> void:
	var root := Node3D.new()
	root.name = "Wreck%d" % unit
	_bursts_root.add_child(root)
	var points: Array[Vector3] = []
	for socket: Node3D in _sockets_of(unit).fires:
		points.append(socket.global_position)
	if points.is_empty():
		points.append(at + Vector3.UP * 1.5)
	var wreck := {"unit": unit, "root": root, "position": at, "age": 0.0, "fires": [], "light": null, "smoke": null}
	var wind := _wind()
	for point in points.slice(0, 2):
		var fire := EffectLibrary.build("fire", _settings.fx_particles, _settings.fx_soft, wind)
		root.add_child(fire)
		fire.global_position = point
		fire.emitting = true
		wreck.fires.append(fire)
	var smoke := EffectLibrary.build("smoke_column", _settings.fx_particles, _settings.fx_soft, wind)
	root.add_child(smoke)
	smoke.global_position = points[0] + Vector3.UP * 0.8
	smoke.emitting = true
	wreck.smoke = smoke
	var lit := 0
	for other in _wrecks:
		if other.light != null:
			lit += 1
	if lit < int(_settings.fx_fire_lights):
		var light := OmniLight3D.new()
		light.light_color = FIRE_LIGHT_COLOR
		light.omni_range = 12.0
		light.light_energy = 2.0
		light.shadow_enabled = false
		root.add_child(light)
		light.global_position = points[0] + Vector3.UP * 1.0
		wreck.light = light
	_wrecks.append(wreck)
	counts["wreck"] = counts.get("wreck", 0) + 1
	# Over the budget: the oldest burning wrecks only smolder.
	var burning := 0
	for i in range(_wrecks.size() - 1, -1, -1):
		if _wrecks[i].age < BURN_SECONDS:
			burning += 1
			if burning > int(_settings.fx_fires):
				_wrecks[i].age = BURN_SECONDS


## Fire, then smoke only (thinner, lighter), then nothing.
func _update_wrecks(delta: float) -> void:
	for i in range(_wrecks.size() - 1, -1, -1):
		var wreck: Dictionary = _wrecks[i]
		wreck.age += delta
		var age: float = wreck.age
		var smoke: GPUParticles3D = wreck.smoke
		if age < BURN_SECONDS:
			# The fire grows in the first seconds.
			smoke.amount_ratio = clampf(age / 3.0, 0.3, 1.0)
			if wreck.light != null:
				var light: OmniLight3D = wreck.light
				light.light_energy = 2.2 * (0.75 + 0.25 * sin(_time * 13.0 + i) * sin(_time * 7.3 + i * 2.0))
		elif age < BURN_SECONDS + SMOLDER_SECONDS:
			for fire: GPUParticles3D in wreck.fires:
				fire.emitting = false
			if wreck.light != null:
				(wreck.light as Node).queue_free()
				wreck.light = null
			smoke.amount_ratio = 0.3
		elif smoke.emitting:
			smoke.emitting = false
		elif age > BURN_SECONDS + SMOLDER_SECONDS + smoke.lifetime:
			(wreck.root as Node).queue_free()
			_wrecks.remove_at(i)


## Burning wrecks right now (self-test).
func burning_wrecks() -> int:
	var n := 0
	for wreck in _wrecks:
		if wreck.age < BURN_SECONDS:
			n += 1
	return n


# --- Vehicles: dust trails and exhaust ----------------------------------------


func _update_vehicles(delta: float) -> void:
	var ids := sim_host.unit_ids()
	var moving := sim_host.unit_moving()
	var alive := sim_host.unit_alive()
	var types := sim_host.unit_type_ids()
	var camera_position: Vector3 = main.camera.global_position
	var reach: float = _settings.fx_vehicle_m
	for i in ids.size():
		var id := ids[i]
		if alive[i] == 0 or types[i].contains("_inf_"):
			if _vehicles.has(id):
				_remove_vehicle(id)
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var position := node.global_position
		var near: bool = camera_position.distance_to(position) < reach and visible_at.call(position)
		if not near and not _vehicles.has(id):
			continue
		var fx := _vehicle(id, node, i)
		var speed := position.distance_to(fx.last) / maxf(delta, 0.001)
		fx.last = position
		fx.frame += 1
		if fx.frame % 8 == 1:
			var terrain := sim_host.terrain_at(position.x, position.z)
			fx.kind = _ground_kind(terrain)
			fx.strength = _trail_strength(terrain)
			(fx.dust as GPUParticles3D).set_instance_shader_parameter(&"tint", GROUND_COLORS[fx.kind])
		var driving: bool = near and moving[i] != 0 and speed > 0.5
		var dust: GPUParticles3D = fx.dust
		dust.emitting = driving and fx.strength > 0.0
		dust.amount_ratio = clampf(fx.strength * speed / 6.0, 0.1, 1.0)
		for exhaust: GPUParticles3D in fx.exhausts:
			exhaust.emitting = near
			exhaust.amount_ratio = 1.0 if driving else 0.3


func _vehicle(id: int, node: Node3D, index: int) -> Dictionary:
	if _vehicles.has(id):
		return _vehicles[id]
	var wind := _wind()
	var dust := EffectLibrary.build("dust_trail", _settings.fx_particles, _settings.fx_soft, wind)
	node.add_child(dust)
	# Models face -Z: the dust rises behind the tracks.
	var length := sim_host.unit_footprints()[index]
	dust.position = Vector3(0.0, 0.3, length * 0.4)
	var exhausts: Array[GPUParticles3D] = []
	for socket: Node3D in _sockets_of(id).exhausts:
		# On the unit node (the socket may be turned): smoke rises straight up.
		var exhaust := EffectLibrary.build("exhaust", _settings.fx_particles, false, wind)
		node.add_child(exhaust)
		exhaust.position = node.to_local(socket.global_position)
		exhausts.append(exhaust)
	var fx := {"dust": dust, "exhausts": exhausts, "kind": "earth", "strength": 0.0, "last": node.global_position, "frame": 0}
	_vehicles[id] = fx
	return fx


func _remove_vehicle(id: int) -> void:
	var fx: Dictionary = _vehicles[id]
	(fx.dust as Node).queue_free()
	for exhaust: Node in fx.exhausts:
		exhaust.queue_free()
	_vehicles.erase(id)


## Dust color kind of a terrain id in the current biome.
func _ground_kind(terrain: int) -> String:
	var biome := _biome()
	match terrain:
		0:
			return {"desert": "sand", "winter": "snow"}.get(biome, "earth")
		1:
			return {"desert": "sand", "winter": "mud"}.get(biome, "earth")
		2:
			return "sand" if biome == "desert" else "road"
		3:
			return "snow" if biome == "winter" else "sand"
		4:
			return "earth" if biome == "desert" else "mud"
		5:
			return "sand" if biome == "desert" else "snow"
		TERRAIN_WATER_SHALLOW, TERRAIN_WATER_DEEP:
			return "water"
	return "earth"


## Dust trail strength (0..1) of a terrain id in the current biome.
func _trail_strength(terrain: int) -> float:
	var biome := _biome()
	match terrain:
		0:
			return {"desert": 1.0, "winter": 0.6}.get(biome, 0.45)
		1:
			return {"winter": 0.35}.get(biome, 1.0)
		2:
			return 0.4 if biome == "desert" else 0.12
		3:
			return 1.0 if biome != "winter" else 0.6
		4:
			return 0.9 if biome == "desert" else 0.4
		5:
			return 1.0 if biome == "desert" else 0.7
		TERRAIN_WATER_SHALLOW:
			return 0.9
		TERRAIN_WATER_DEEP:
			return 0.0
	return 0.3


func _ground_color(at: Vector3) -> Color:
	return GROUND_COLORS[_ground_kind(sim_host.terrain_at(at.x, at.z))]


# --- Helpers ------------------------------------------------------------------


func _biome() -> String:
	var atmosphere: Node = main.atmosphere
	return atmosphere.preset.id if atmosphere != null and atmosphere.preset != null else "forest"


func _wind() -> Vector2:
	var atmosphere: Node = main.atmosphere
	if atmosphere == null or atmosphere.preset == null:
		return Vector2.ZERO
	var direction: Vector2 = atmosphere.preset.wind_direction
	return direction.normalized() * (0.5 + 2.0 * atmosphere.preset.wind_strength)


## (Re)builds the pools when the graphics quality or the biome changed.
func _configure() -> void:
	var quality: String = main.atmosphere.quality if main.atmosphere != null else GraphicsQuality.DEFAULT
	var key := "%s/%s" % [quality, _biome()]
	if key == _config_key:
		return
	_config_key = key
	_settings = GraphicsQuality.settings(quality)
	for pool: Array in _pools.values():
		for p: Node in pool:
			p.queue_free()
	_pools.clear()
	var wind := _wind()
	for kind: String in POOL_SIZES:
		var pool: Array = []
		for i in maxi(2, int(round(POOL_SIZES[kind] * float(_settings.fx_pools)))):
			var p := EffectLibrary.build(kind, _settings.fx_particles, _settings.fx_soft, wind)
			p.visible = false
			_bursts_root.add_child(p)
			pool.append(p)
		_pools[kind] = pool
		_pool_next[kind] = 0
	for entry in _lights:
		(entry.light as Node).queue_free()
	_lights.clear()
	for i in int(_settings.fx_lights):
		var light := OmniLight3D.new()
		light.shadow_enabled = false
		light.visible = false
		light.omni_attenuation = 1.5
		_bursts_root.add_child(light)
		_lights.append({"light": light, "left": 0.0, "duration": 1.0, "energy": 0.0})
	while _decals.size() > int(_settings.fx_decals):
		_decals.pop_back().queue_free()
	_decal_next = 0
	for id: int in _vehicles.keys():
		_remove_vehicle(id)


func _index_of(unit: int) -> int:
	var frame := Engine.get_process_frames()
	if frame != _index_frame:
		_index_frame = frame
		_index_cache.clear()
		var ids := sim_host.unit_ids()
		for i in ids.size():
			_index_cache[ids[i]] = i
	return _index_cache.get(unit, -1)


## Sockets of a unit's model, found once.
func _sockets_of(unit: int) -> Dictionary:
	if _sockets.has(unit):
		return _sockets[unit]
	var entry := {"muzzles": [], "fires": [], "exhausts": []}
	var node: Node3D = main._unit_nodes.get(unit)
	var model := node.get_node_or_null("Model") if node != null else null
	if model != null:
		entry.muzzles = model.find_children("Socket_Muzzle*", "", true, false)
		entry.fires = model.find_children("Socket_Fire_*", "", true, false)
		entry.exhausts = model.find_children("Socket_Exhaust_*", "", true, false)
	_sockets[unit] = entry
	return entry


## The model's muzzle socket nearest to the simulation's muzzle, or the
## simulation's point if the model has none near it.
func _muzzle_of(unit: int, sim_muzzle: Vector3) -> Vector3:
	var best := sim_muzzle
	var best_distance := 6.0
	for socket: Node3D in _sockets_of(unit).muzzles:
		var d := socket.global_position.distance_to(sim_muzzle)
		if d < best_distance:
			best_distance = d
			best = socket.global_position
	return best
