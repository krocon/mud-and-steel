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
const Settings := preload("res://scenes/menu/settings.gd")

## Emitters per burst kind at quality "high" (scaled by fx_pools). Sized so
## 50 explosions at once fit at high.
const POOL_SIZES: Dictionary[String, int] = {
	"flash": 32, "powder": 40, "blast_dust": 56, "fountain": 48, "spray": 16,
	"impact_dust": 48, "puff": 48, "sparks": 56, "fireball": 56,
	"explosion_smoke": 56, "debris": 56,
	"core": 48, "rays": 32, "streaks": 48, "earth_cloud": 48, "embers": 56, "dust_cloud": 24,
}
## Layers of an explosion (prompt 072, after the reference video; profiles
## in docs/ARCHITECTURE.md): burst kind, delay in s, size × the explosion's
## scale, height above the ground in m × scale, and tint: "" none, "ground"
## the ground colour, "earth" ground mixed with grey (thrown-up soil and
## dust), "dark" the ground darkened. The flash, the earth fan and the
## fireball start together; the black smoke grows out of the fireball from
## 0.9 s, glowing embers show in it from 1.2 s.
const EXPLOSION_LAYERS: Dictionary[String, Array] = {
	# Shell on the ground from 37 mm (scale caliber / 75 mm).
	"ground_hit": [
		["core", 0.0, 0.7, 0.5, ""],
		["streaks", 0.0, 1.0, 0.0, "dark"],
		["fountain", 0.0, 1.0, 0.0, "dark"],
		["earth_cloud", 0.03, 1.0, 0.6, "earth"],
		["sparks", 0.0, 0.5, 0.3, ""],
	],
	# Destroyed vehicle (scale = vehicle length / 7 m).
	"vehicle": [
		["rays", 0.0, 1.0, 1.5, ""],
		["fireball", 0.0, 1.0, 1.6, ""],
		["sparks", 0.0, 1.3, 1.2, ""],
		["debris", 0.0, 1.0, 1.2, ""],
		["blast_dust", 0.0, 0.9, 0.2, "dark"],
		["explosion_smoke", 0.9, 1.0, 3.0, ""],
		["embers", 1.2, 1.0, 3.0, ""],
	],
}
## Mix of the ground colour with grey for thrown-up earth.
const EARTH_GREY := Color(0.55, 0.52, 0.48)
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
## `weapon` of shot events of indirect fire (prompt 032).
const INDIRECT_WEAPON := 255
## Rocket ids older than this (s) are forgotten (prompt 062).
const ROCKET_FORGET_S := 120.0
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
## Soldier hits (prompt 048): dust and cloth, then a short dark puff.
const SOLDIER_HIT_DUST := Color(0.5, 0.46, 0.38)
const SOLDIER_HIT_DARK := Color(0.12, 0.1, 0.09)
const FLASH_LIGHT_COLOR := Color(1.0, 0.72, 0.4)

## Time of the last `_process` in microseconds (benchmark, prompt 016).
var process_usec := 0
## Whether a unit id is infantry (no dust trail, no exhaust).
var _infantry_ids: Dictionary[int, bool] = {}
## Turret directions of this frame's snapshot (with `_index_cache`).
var _aim_yaws := PackedFloat32Array()
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
## Unit id -> sockets of the model (muzzles, fires, exhausts, rocket racks).
var _sockets: Dictionary[int, Dictionary] = {}
## Rockets of vehicle launchers in flight (prompt 062): projectile id ->
## launch time (`_time`). `artillery_view.gd` draws their smoke trail.
var rocket_projectiles: Dictionary[int, float] = {}
var _index_cache: Dictionary[int, int] = {}
var _index_frame := -1
var _shake := 0.0
var _time := 0.0
var _visibility_timer := 0.0
var _bursts_root: Node3D
## Layers waiting for their delay: {t, kind, at, s, tint} (prompt 072).
var _pending: Array[Dictionary] = []


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
	var began := Time.get_ticks_usec()
	_process_effects(delta)
	process_usec = Time.get_ticks_usec() - began


func _process_effects(delta: float) -> void:
	_time += delta
	_configure()
	_start_pending()
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
			_shot(event["unit"], event["position"], event["caliber_mm"], int(event.get("weapon", -1)),
					int(event.get("projectile", -1)))
		"hit":
			match event["surface"]:
				"ground":
					ground_hit(event["position"], event["caliber_mm"])
				"unit":
					if _is_infantry(int(event["target"])):
						soldier_hit(event["position"], event["caliber_mm"])
					else:
						unit_hit(event["position"], event["caliber_mm"], event["penetrated"])
				_:
					object_hit(event["position"], event["caliber_mm"])
		"destroyed":
			destroyed(event["unit"], event["position"])


func _shot(unit: int, sim_muzzle: Vector3, caliber: float, weapon := -1, projectile := -1) -> void:
	var index := _index_of(unit)
	var yaw: float = _aim_yaws[index] if index >= 0 and index < _aim_yaws.size() else 0.0
	var direction := Vector3(-sin(yaw), 0.0, -cos(yaw))
	# Rocket launchers (prompt 062): indirect shots leave the rack
	# (`Socket_Rocket_*`), with a back blast and a smoke trail.
	var racks: Array = _sockets_of(unit).rockets
	if weapon == INDIRECT_WEAPON and not racks.is_empty():
		var rack: Node3D = racks[0]
		rocket_launch(rack.global_position, direction, caliber)
		if projectile >= 0:
			_note_rocket(projectile)
		return
	muzzle_flash(_muzzle_of(unit, sim_muzzle), direction, caliber)


## Launch of one rocket of a salvo (prompt 062): the flash and powder smoke
## of a gun plus the back blast of the motor, which raises dust behind the
## launcher. Reuses the bursts of prompt 019; prompt 076 may replace it.
func rocket_launch(at: Vector3, direction: Vector3, caliber: float) -> void:
	muzzle_flash(at, direction, caliber * 0.6)
	if not visible_at.call(at):
		return
	var back := -direction.normalized() if direction.length() > 0.01 else Vector3.BACK
	var s := clampf(caliber / 100.0, 0.6, 1.4)
	_burst("powder", at + back * 1.5, Basis.looking_at(back), s)
	var ground := at + back * 3.0
	ground.y = sim_host.height_at(ground.x, ground.z) + 0.2
	_burst("blast_dust", ground, Basis.IDENTITY, s * 0.7, _ground_color(ground))


## One puff of a rocket's smoke trail at `at` (called by
## `artillery_view.gd` while the motor burns).
func rocket_trail(at: Vector3, velocity: Vector3) -> void:
	if not visible_at.call(at):
		return
	var back := -velocity.normalized() if velocity.length() > 0.01 else Vector3.UP
	_burst("powder", at, Basis.looking_at(back, Vector3.UP if absf(back.y) < 0.99 else Vector3.RIGHT), 0.45)


## Seconds since rocket `projectile` was launched (INF: not a rocket).
func rocket_age(projectile: int) -> float:
	if not rocket_projectiles.has(projectile):
		return INF
	return _time - rocket_projectiles[projectile]


func _note_rocket(projectile: int) -> void:
	rocket_projectiles[projectile] = _time
	if rocket_projectiles.size() > 256:
		for id: int in rocket_projectiles.keys():
			if _time - rocket_projectiles[id] > ROCKET_FORGET_S:
				rocket_projectiles.erase(id)


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
	if caliber < 37.0:
		_burst("fountain", at, Basis.IDENTITY, s, color.darkened(0.15))
		_burst("impact_dust", at, Basis.IDENTITY, s, color)
	else:
		# Prompt 072: flash, earth fan, clods and a cloud of earth lumps.
		layers("ground_hit", at, s, color)
		_decal(at, 1.4 + 1.6 * s)
		# The flash lights up the surroundings for one or two frames.
		_light(at + Vector3.UP * 1.5, 4.0 * s, 14.0 * s, 0.12)
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


## A hit on a soldier (prompt 048): restrained on purpose, no blood: a puff
## of dust and cloth and a short dark puff. [provisional: the user decides
## on more severity, age rating]
func soldier_hit(at: Vector3, caliber: float) -> void:
	if not visible_at.call(at):
		return
	var s := 0.35 if caliber < SMALL_CALIBER else clampf(caliber / 75.0, 0.5, 1.2)
	_burst("puff", at, Basis.IDENTITY, s, SOLDIER_HIT_DUST)
	_burst("puff", at, Basis.IDENTITY, s * 0.6, SOLDIER_HIT_DARK)
	counts["soldier_hit"] = counts.get("soldier_hit", 0) + 1


## Whether a unit is an infantry squad (unit types never change: cached).
func _is_infantry(unit: int) -> bool:
	if not _infantry_ids.has(unit):
		if _index_of(unit) < 0:
			return false
		var types := sim_host.unit_type_ids()
		var ids := sim_host.unit_ids()
		for i in ids.size():
			_infantry_ids[ids[i]] = (types[i] as String).contains("_inf_")
	return _infantry_ids.get(unit, false)


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
			_burst("dust_cloud", at + Vector3.UP * 3.0, Basis.IDENTITY, s * 0.8, dust.lightened(0.15))
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
	var ground := at
	ground.y = sim_host.height_at(at.x, at.z)
	# Prompt 072: flash and rays, a fireball that turns into black smoke,
	# embers, sparks, debris and dust.
	layers("vehicle", ground, s, _ground_color(ground))
	# The flash; a longer glow (1.4 s) cost 4 FPS at "high" with 50
	# explosions (prompt 072), the video lights the ground only in the flash.
	_light(center + Vector3.UP, 10.0 * s, 28.0 * s, 0.4)
	_decal(ground, 7.0 * s)
	_add_shake(at, 1.0 * s)


# --- Bursts -----------------------------------------------------------------


## Starts the layers of EXPLOSION_LAYERS[profile] on the ground point `at`
## with scale `s`; `ground` is the colour of the ground there. Layers with a
## delay wait in `_pending` (the caller checked the visibility).
func layers(profile: String, at: Vector3, s: float, ground: Color) -> void:
	for layer: Array in EXPLOSION_LAYERS[profile]:
		var tint := Color.WHITE
		match str(layer[4]):
			"ground":
				tint = ground
			"earth":
				tint = ground.lerp(EARTH_GREY, 0.5)
			"dark":
				tint = ground.darkened(0.3)
		var point := at + Vector3.UP * (float(layer[3]) * s)
		var size := float(layer[2]) * s
		if float(layer[1]) <= 0.0:
			_burst(str(layer[0]), point, Basis.IDENTITY, size, tint)
		else:
			_pending.append({"t": _time + float(layer[1]), "kind": layer[0], "at": point, "s": size, "tint": tint})


func _start_pending() -> void:
	for i in range(_pending.size() - 1, -1, -1):
		var entry: Dictionary = _pending[i]
		if entry.t <= _time:
			_pending.remove_at(i)
			_burst(entry.kind, entry.at, Basis.IDENTITY, entry.s, entry.tint)



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
	# Screen shake can be switched off in the settings (prompt 043).
	if not shake_enabled or not Settings.get_value("screen_shake"):
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
	var camera_position: Vector3 = main.camera.global_position
	var reach: float = _settings.fx_vehicle_m
	# Unit types never change: which ids are infantry is read once per unit
	# instead of every frame (prompt 016).
	var types := PackedStringArray()
	for i in ids.size():
		var id := ids[i]
		if not _infantry_ids.has(id):
			if types.is_empty():
				types = sim_host.unit_type_ids()
			_infantry_ids[id] = types[i].contains("_inf_")
		if alive[i] == 0 or _infantry_ids[id]:
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
		# Read once per frame, not once per shot (prompt 016).
		_aim_yaws = sim_host.unit_aim_yaws()
	return _index_cache.get(unit, -1)


## Sockets of a unit's model, found once.
func _sockets_of(unit: int) -> Dictionary:
	if _sockets.has(unit):
		return _sockets[unit]
	var entry := {"muzzles": [], "fires": [], "exhausts": [], "rockets": []}
	var node: Node3D = main._unit_nodes.get(unit)
	var model := node.get_node_or_null("Model") if node != null else null
	if model != null:
		entry.muzzles = model.find_children("Socket_Muzzle*", "", true, false)
		entry.fires = model.find_children("Socket_Fire_*", "", true, false)
		entry.exhausts = model.find_children("Socket_Exhaust_*", "", true, false)
		entry.rockets = model.find_children("Socket_Rocket_*", "", true, false)
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
