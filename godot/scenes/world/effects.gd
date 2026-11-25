extends Node3D
## Battle effects (prompt 019): muzzle flashes and powder smoke, impacts
## (earth fountains, water fountains that shoot up and fall back, sparks), explosions with fireball, light
## flash, debris and camera shake, burning wrecks with smoke columns, dust
## trails and exhaust smoke behind vehicles, and scorch decals.
##
## Ground effects of driving vehicles (prompt 104, `drive_effect`): dust on
## dry ground, heavy dark clods on mud and on soaked earth (rain), fine
## snow dust on snow, bow spray and a foam wake in a ford.
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
##
## Kill sequences (prompt 105, `kill_sequence.gd`): a destroyed vehicle plays
## a few stages (flash, smoke, fire, cook-off or a fuel flare) over 2-5 s
## instead of one explosion; `kill_stage` tells the sound. The wreck exists
## from the killing tick (smoke only), its flames start with the `fire`
## stage. Timing and kind come from the unit id and the tick, the same in
## every replay. The rules do not wait for it: the unit is dead at once.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const RocketEffects := preload("res://scenes/world/rocket_effects.gd")
const BlastEffects := preload("res://scenes/world/blast_effects.gd")
const KillSequence := preload("res://scenes/world/kill_sequence.gd")

## A stage of a kill sequence started (prompt 105): `unit`, the stage name
## (`kill_sequence.gd`), the wreck's ground point, its size (1 = medium
## tank) and whether the simulation blew its ammunition up. Sent also while
## the point is not visible (the sound plays muffled there).
signal kill_stage(unit: int, stage: String, at: Vector3, size: float, ammo: bool)

## Emitters per burst kind at quality "high" (scaled by fx_pools). Sized so
## 50 explosions at once fit at high.
const POOL_SIZES: Dictionary[String, int] = {
	"flash": 32, "powder": 40, "blast_dust": 56, "fountain": 48, "spray": 16,
	"impact_dust": 48, "puff": 48, "sparks": 56, "fireball": 56,
	"explosion_smoke": 56, "smoke_collar": 28, "ground_haze": 12, "debris": 56,
	# Leaf fall of trees stripped by a big blast (prompt 085).
	"leaves": 32,
	"core": 48, "rays": 32, "streaks": 48, "earth_cloud": 48, "embers": 56, "dust_cloud": 24,
	# Two salvos at once: four launches a second each, 3.2 s of smoke.
	"rocket_blast": 26,
	# Muzzle blast (prompt 077): 40 cannon shots a second plus autocannon
	# bursts; the core and tongue live < 0.1 s, the gas 0.45 s; the powder
	# smoke (3 s) and the dust (2.6 s) of the oldest shots are cut short.
	"muzzle_core": 24, "muzzle_tongue": 40, "muzzle_gas": 32, "muzzle_dust": 40, "muzzle_puffs": 32,
	# Shells in water: column, drops and base surge (3.5 s each).
	"water_column": 24, "water_jets": 24, "water_drops": 16, "water_ring": 16,
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
	# Shell in water from 37 mm: the column shoots up and falls back, drops
	# fly wider, the base surge rolls out as it lands, mist stays.
	# After the user's reference of an underwater blast: spiky jets fanning
	# out, a dense white column, grey tops (dark layers), a wide white base.
	"water_hit": [
		["water_column", 0.0, 1.0, 0.0, ""],
		["water_jets", 0.0, 1.0, 0.2, ""],
		["water_jets", 0.05, 0.85, 0.6, "dark"],
		["water_column", 0.08, 0.75, 1.2, "dark"],
		["water_drops", 0.0, 1.0, 0.3, ""],
		["water_ring", 0.0, 0.8, 0.1, ""],
		["water_ring", 0.9, 1.4, 0.1, ""],
		["spray", 1.4, 1.6, 0.0, ""],
	],
	# Destroyed vehicle (scale = vehicle length / 7 m; prompt 220: pale
	# sandy-beige ground dust ring, warm-lit bright-grey lower smoke collar,
	# velvety deep-black upper oil-smoke mushroom, and low ground haze veil).
	"vehicle": [
		["rays", 0.0, 1.0, 1.5, ""],
		["fireball", 0.0, 1.05, 1.5, ""],
		["sparks", 0.0, 1.3, 1.2, ""],
		["debris", 0.0, 1.0, 1.2, ""],
		["blast_dust", 0.0, 1.25, 0.2, "sand_dust"],
		["ground_haze", 0.1, 1.0, 0.9, "haze"],
		["smoke_collar", 0.15, 1.05, 1.3, ""],
		["explosion_smoke", 0.45, 1.15, 3.6, ""],
		["embers", 1.0, 1.0, 2.8, ""],
	],
}
## Mix of the ground colour with grey for thrown-up earth.
const EARTH_GREY := Color(0.55, 0.52, 0.48)
## Pale beige-grey mix for low ground haze veils (prompt 220). [provisional]
const HAZE_BEIGE := Color(0.84, 0.81, 0.75)
## Heavy impact threshold (mm) for two-stage artillery smoke and ground haze
## veils (prompt 220). [provisional]
const HEAVY_IMPACT_MM := 100.0
## Ground look per terrain id (data/terrain.toml) and biome: dust color kind
## and dust trail strength (0..1). Desert draws grass and snow as sand, winter
## grass and sand as snow (like the terrain shader, prompt 015).
## Trees whose stage changed that get bursts per frame (prompt 085: a salvo
## into a wood changes dozens in a few ticks); the rest change silently.
const TREE_EFFECTS_PER_FRAME := 6
## Leaves, needles and frond bits of a stripped tree (prompt 085).
const LEAF_COLORS: Dictionary[String, Color] = {
	"env_tree_deciduous": Color(0.36, 0.42, 0.16), "env_tree_birch": Color(0.5, 0.55, 0.2),
	"env_tree_conifer": Color(0.2, 0.3, 0.14), "env_tree_palm": Color(0.42, 0.45, 0.2),
	"env_tree_stone_pine": Color(0.18, 0.27, 0.12), "env_tree_cypress": Color(0.16, 0.24, 0.11),
	"env_tree_olive": Color(0.38, 0.42, 0.3),
}
const GROUND_COLORS: Dictionary[String, Color] = {
	"earth": Color(0.37, 0.3, 0.22), "sand": Color(0.82, 0.72, 0.54), "snow": Color(0.93, 0.95, 0.99),
	"mud": Color(0.21, 0.17, 0.13), "road": Color(0.55, 0.53, 0.5), "water": Color(0.88, 0.92, 0.96),
	"object": Color(0.5, 0.46, 0.4),
}
const TERRAIN_WATER_SHALLOW := 6
const TERRAIN_WATER_DEEP := 7
## Tall grass (prompt 073): dust and trails like grass.
const TERRAIN_TALL_GRASS := 8
## Ground effects of driving (prompt 104) besides the dust trail, and the
## emitters of each (effect_library.gd kinds).
const GROUND_EFFECTS: Dictionary[String, Array] = {
	"mud": ["mud_clods"], "snow": ["snow_dust"], "wake": ["bow_spray", "wake_foam"],
}
## Without `fx_ground_fx` (low quality): what is drawn instead ("" nothing).
const PLAIN_EFFECTS: Dictionary[String, String] = {"snow": "dust", "wake": "dust", "mud": ""}
## Terrain wetness (weather_look.gd) from which bare earth and meadow throw
## mud instead of dust: rain soaks them (fog only dampens, < 0.25).
const WET_GROUND := 0.3
## Below this caliber (mm) a round is small arms: tiny flashes and puffs.
const SMALL_CALIBER := 20.0
## `weapon` of shot events of indirect fire (prompt 032).
const INDIRECT_WEAPON := 255
## Bursts smaller than this on screen (pixels) are skipped.
const MIN_PIXELS := 3.0
## A burst of this many pixels gets all its particles; smaller ones fewer.
const FULL_PIXELS := 60.0
## Small details (glowing embers, sparks, the core flash) only start when
## their explosion is at least this big on screen (prompt 072): from afar
## they are a few pixels, but each is one more emitter to process.
const DETAIL_PIXELS := 50.0
const DETAIL_KINDS: Array[String] = ["embers", "sparks", "core"]
## Burning wrecks: fire and dense smoke, then smoke only, then nothing.
const BURN_SECONDS := 60.0
const SMOLDER_SECONDS := 90.0
## A hit counts as the killing one for this long before the destroyed
## event (s), and an ammunition explosion waits this long for its destroyed
## event before it plays alone (prompt 105).
const KILL_HIT_S := 1.0
const AMMO_WAIT_S := 0.5
## Smoke share of a wreck before its fire takes hold.
const SMOKE_ONLY := 0.5
## Camera shake: offset per unit of shake as a share of the camera distance,
## and how fast it decays (1/s). Triggered only by massive blasts
## (`blast_effects.gd` `SHAKE_CALIBER` and up) so ordinary fire keeps the
## camera steady.
const SHAKE_SCALE := 0.0022
const SHAKE_DECAY := 10.0
## Decals fade out beyond this camera distance (meters).
const DECAL_FADE_M := 450.0
const FIRE_LIGHT_COLOR := Color(1.0, 0.55, 0.2)
## Soldier hits (prompt 048): dust and cloth, then a short dark puff.
const SOLDIER_HIT_DUST := Color(0.5, 0.46, 0.38)
const SOLDIER_HIT_DARK := Color(0.12, 0.1, 0.09)
const FLASH_LIGHT_COLOR := Color(1.0, 0.72, 0.4)

# --- Muzzle blast (prompt 077) -----------------------------------------------
## Layers of a gun's shot per class: factors on the scale (caliber / 75 mm)
## of the core, the fire tongue's length, the glowing gas, the powder smoke,
## the ground dust, the light, the camera shake, the recoil and the hull's
## rocking. Judged against the reference picture (docs/ARCHITECTURE.md
## "Muzzle blast (prompt 077)"): machine cannons small quick flashes on
## every round of a burst, no dust; heavy guns a longer tongue and more gas
## and dust; howitzers more smoke and dust, a shorter tongue; mortars (an
## indirect weapon without a `Gun` node) a flash and smoke only.
const MUZZLE_CLASSES: Dictionary[String, Dictionary] = {
	"autocannon": {"core": 1.4, "tongue": 1.0, "gas": 0.6, "powder": 0.35, "dust": 0.0, "light": 0.6, "shake": 0.0,
		"recoil": 0.8, "rock": 0.0},
	"gun": {"core": 1.0, "tongue": 1.0, "gas": 1.0, "powder": 1.0, "dust": 1.0, "light": 1.0, "shake": 1.0,
		"recoil": 1.0, "rock": 1.0},
	"heavy": {"core": 1.0, "tongue": 1.1, "gas": 1.15, "powder": 1.2, "dust": 1.3, "light": 1.3, "shake": 1.3,
		"recoil": 1.0, "rock": 1.2},
	"howitzer": {"core": 1.0, "tongue": 0.55, "gas": 1.25, "powder": 1.6, "dust": 1.6, "light": 1.1, "shake": 1.2,
		"recoil": 1.0, "rock": 0.8},
	"mortar": {"core": 0.6, "tongue": 0.0, "gas": 0.5, "powder": 1.0, "dust": 0.6, "light": 0.4, "shake": 0.2,
		"recoil": 0.0, "rock": 0.0},
}
## Calibers (mm) where the classes of direct fire begin: below 40 machine
## cannons (20-37 mm), from 85 heavy guns.
const GUN_CALIBER := 40.0
const HEAVY_CALIBER := 85.0
## Scale of a blast: caliber / 75 mm within these bounds (20 mm 0.27,
## 152 mm 2.0; before prompt 077 0.4 to 1.6).
const MUZZLE_SCALE_MIN := 0.25
const MUZZLE_SCALE_MAX := 2.2
## Length of the fire tongue at scale 1 (m): 40 calibers of a 7.5 cm gun.
## The intermediate flash stands 20-25 calibers before the muzzle of large
## guns, the secondary flash beyond it (docs/SOURCES.md); 40 calibers is a
## judged middle of the secondary flash.
const TONGUE_M := 3.0
## The tongue's length follows the real muzzle velocity: × v / 750 m/s
## within these bounds (a 7.5 cm L/24 short, a 8.8 cm L/71 long). The
## simulation's shells fly at the real velocity × the time factor 2
## (data/scale.toml).
const TONGUE_VELOCITY_REF := 750.0
const TONGUE_VELOCITY_MIN := 0.65
const TONGUE_VELOCITY_MAX := 1.35
const TIME_FACTOR := 2.0
## Side blasts of a muzzle brake: share of the tongue's length and of the
## gas cloud (judged).
const BRAKE_TONGUE := 0.35
const BRAKE_GAS := 0.45
## Clouds (prompt 200, `effect_library.gd`): radius of a burst's cloud at
## scale 1 and of a wreck's smoke column, for lighting the puffs as one body.
const CLOUD_RADIUS_M := 3.0
const COLUMN_RADIUS_M := 3.0
## Light flash (m, energy, s) at scale 1; × the daytime factors of
## blast_effects.gd (night × 3). Judged: lights the ground and the own
## vehicle as in the reference, as long as the core.
const MUZZLE_LIGHT_ENERGY := 4.0
const MUZZLE_LIGHT_RANGE_M := 9.0
const MUZZLE_LIGHT_SECONDS := 0.08
## Brightness of the (unshaded) glowing gas by time of day (judged).
const GAS_DAYTIME := {"day": 1.0, "morning": 0.75, "evening": 0.75, "night": 0.45}
## The core stays a bright point at least this big on screen however far
## away (it shows who fires): no LOD for it.
const CORE_MIN_PIXELS := 5.0
## Blown-up dust: the ground colour mixed with this light beige (judged
## from the reference's light brown dust over a meadow).
const DUST_LIGHT := Color(0.78, 0.68, 0.52)
## Barrels raised more than this (sine of the elevation) blow the dust all
## round the gun instead of fanning it ahead.
const RAISED_BARREL := 0.35
## Recoil: the `Gun` node runs back along its axis (m per mm of caliber,
## at most RECOIL_MAX_M), in RECOIL_BACK_S, and forward again in
## RECOIL_RETURN_S. Real recoil lengths are 0.3-0.6 m for 7.5-12.2 cm guns
## (judged from the order of magnitude; the counter-recoil is slowed down to
## be seen).
const RECOIL_M_PER_MM := 0.005
const RECOIL_MAX_M := 0.6
const RECOIL_BACK_S := 0.05
const RECOIL_RETURN_S := 0.7
## The hull rocks: degrees at scale 1, frequency (Hz), decay time (s);
## judged.
const ROCK_DEG := 0.8
const ROCK_HZ := 1.6
const ROCK_DECAY_S := 0.35
## A howitzer's barrel stays raised this long after its last round (s),
## or until the gun moves, then sinks at ELEVATE_DEG_PER_S.
const ELEVATION_HOLD_S := 25.0
const ELEVATE_DEG_PER_S := 12.0

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
var _tree_effects_left := TREE_EFFECTS_PER_FRAME

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
## Unit id -> {dust, exhausts, ground (effect -> emitters), kind, strength,
## effect, length, last, frame}.
var _vehicles: Dictionary[int, Dictionary] = {}
## Unit id -> sockets of the model (muzzles, fires, exhausts, rocket racks).
var _sockets: Dictionary[int, Dictionary] = {}
## Rocket launch, flame and trail (prompt 076, `rocket_effects.gd`).
var rockets: RocketEffects
## Unit id -> its next rail socket (prompt 076).
var _rail_next: Dictionary[int, int] = {}
var _index_cache: Dictionary[int, int] = {}
var _index_frame := -1
var _shake := 0.0
var _time := 0.0
var _visibility_timer := 0.0
var _bursts_root: Node3D
## Layers waiting for their delay: {t, kind, at, s, tint} (prompt 072).
var _pending: Array[Dictionary] = []
## Unit id -> its gun's kick (prompt 077, `_gun_of`): {gun, rest, model,
## model_rest, recoil, t, rock, axis, elevation, target, last, active}.
var _guns: Dictionary[int, Dictionary] = {}
## Indirect shells whose barrel elevation is not known yet: projectile id ->
## {unit, t} (the shell shows up in a later snapshot).
var _pending_elevations: Dictionary[int, Dictionary] = {}
## Projectile and shell velocities of this frame (id -> m/s), read once.
var _velocities: Dictionary[int, Vector3] = {}
var _velocities_frame := -1
## Recoils started since the start (self-test, tools).
var recoils := 0
## Kill sequences under way (prompt 105): {unit, at, top, s, profile,
## stages ([[s, stage], ...]), next, start, ammo, log}.
var _kills: Array[Dictionary] = []
## Every kill sequence started, newest last (self-test, at most 64):
## {unit, tick, profile, planned (stage names), done (stage names)}.
var kill_log: Array[Dictionary] = []
## Unit id -> the last hit on it: {caliber_mm, t} (the killing hit).
var _last_hits: Dictionary[int, Dictionary] = {}
## Unit id -> an ammunition explosion of the simulation waiting for its
## destroyed event: {t, at}.
var _ammo_waiting: Dictionary[int, Dictionary] = {}
## Catalog category per unit type index (cached).
var _categories: Dictionary[int, String] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_bursts_root = Node3D.new()
	_bursts_root.name = "Bursts"
	add_child(_bursts_root)
	_configure()
	rockets = RocketEffects.new()
	rockets.setup(self)
	add_child(rockets)
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
	_tree_effects_left = TREE_EFFECTS_PER_FRAME
	_configure()
	_start_pending()
	_update_kills()
	_update_lights(delta)
	_update_wrecks(delta)
	_update_vehicles(delta)
	_update_shake(delta)
	_update_guns(delta)
	_visibility_timer -= delta
	if _visibility_timer <= 0.0:
		_visibility_timer = 0.5
		for wreck in _wrecks:
			(wreck.root as Node3D).visible = visible_at.call(wreck.position)
		_prune_caches()


## Removes cached unit metadata for freed/removed unit nodes (prompt 206).
func _prune_caches() -> void:
	if main == null:
		return
	var unit_nodes: Dictionary = main._unit_nodes
	for id: int in _infantry_ids.keys():
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node):
			_infantry_ids.erase(id)
	for id: int in _sockets.keys():
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node):
			_sockets.erase(id)
	for id: int in _guns.keys():
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node):
			_guns.erase(id)


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
					_last_hits[int(event["target"])] = {"caliber_mm": float(event["caliber_mm"]), "t": _time}
					if _last_hits.size() > 512:
						_last_hits.clear()
					if _is_infantry(int(event["target"])):
						soldier_hit(event["position"], event["caliber_mm"])
					else:
						unit_hit(event["position"], event["caliber_mm"], event["penetrated"])
				_:
					object_hit(event["position"], event["caliber_mm"])
		"destroyed":
			destroyed(event["unit"], event["position"], int(event.get("tick", 0)))


func _shot(unit: int, sim_muzzle: Vector3, caliber: float, weapon := -1, projectile := -1) -> void:
	var index := _index_of(unit)
	var yaw: float = _aim_yaws[index] if index >= 0 and index < _aim_yaws.size() else 0.0
	var direction := Vector3(-sin(yaw), 0.0, -cos(yaw))
	# Rockets (prompts 062, 076): indirect shots of a unit whose model has
	# rails (`Socket_Rocket_*`: vehicle launchers, the strike's battery
	# position) leave the next rail, one after the other.
	var racks: Array = _sockets_of(unit).rockets
	if weapon == INDIRECT_WEAPON and not racks.is_empty():
		if rockets.enabled:
			var next: int = _rail_next.get(unit, 0)
			_rail_next[unit] = (next + 1) % racks.size()
			var rail: Node3D = racks[next]
			if projectile >= 0:
				rockets.note(projectile, rail.global_position, unit, caliber)
			else:
				rockets.launch(rail.global_position, direction, caliber, unit)
		else:
			# Rockets drawn as plain shells (bench comparison): the small
			# flash of before prompt 076, never a gun's fire tongue.
			var muzzle := _muzzle_of(unit, sim_muzzle, caliber)
			_simple_flash(muzzle.global_position if muzzle != null else sim_muzzle, direction, caliber)
		return
	if caliber < SMALL_CALIBER:
		var socket := _muzzle_of(unit, sim_muzzle)
		var from := socket.global_position if socket != null else sim_muzzle
		_simple_flash(from, direction, caliber)
		_note_tracer(projectile, weapon, from, sim_muzzle)
		return
	var velocity := _velocity_of(projectile) if projectile >= 0 else Vector3.ZERO
	var indirect := weapon == INDIRECT_WEAPON
	var gun := _gun_of(unit)
	# Mortars: an indirect weapon without a gun, of a squad, or whose barrel
	# is modeled standing steeply.
	var mortar := gun.is_empty() or _is_infantry(unit) or bool(gun.steep)
	var kind := _muzzle_class(caliber, indirect, mortar)
	if indirect and not mortar:
		# The barrel rises to the shell's elevation (prompt 077); the shell
		# may only show up in a later snapshot.
		if velocity != Vector3.ZERO:
			_raise(gun, velocity, true)
		elif projectile >= 0:
			_pending_elevations[projectile] = {"unit": unit, "t": _time}
		gun.last = _time
	var socket := _muzzle_of(unit, sim_muzzle, caliber)
	var at := sim_muzzle
	if socket != null:
		at = socket.global_position
		# Forward is the barrel from the gun's trunnion to its muzzle (the
		# real, possibly raised barrel), else the socket's -Z (+Y in
		# Blender).
		direction = -socket.global_basis.z.normalized()
		if not gun.is_empty() and socket.get_parent() == gun.gun:
			var axis: Vector3 = socket.global_position - (gun.gun as Node3D).global_position
			if axis.length() > 0.5:
				direction = axis.normalized()
	elif velocity != Vector3.ZERO:
		direction = velocity.normalized()
	var speed := velocity.length() / TIME_FACTOR if not indirect else 0.0
	var brake: bool = socket != null and not mortar and socket.get_parent() == gun.get("gun") and gun.get("brake", false)
	muzzle_flash(at, direction, caliber, kind, speed, brake, socket.global_basis.x if socket != null else Vector3.ZERO)
	_note_tracer(projectile, weapon, at, sim_muzzle)
	if not gun.is_empty() and socket != null and socket.get_parent() == gun.gun:
		_kick(gun, caliber, kind, direction)


## Small arms (and rockets drawn as balls): a small flash; from 20 mm the
## flash, powder smoke and light of before prompt 077.
func _simple_flash(at: Vector3, direction: Vector3, caliber: float) -> void:
	if not visible_at.call(at):
		return
	var facing := _facing(direction)
	if caliber < SMALL_CALIBER:
		_burst("flash", at, facing, 0.25)
		return
	var s := clampf(caliber / 75.0, 0.4, 1.6)
	_burst("flash", at, facing, s)
	_light(at, 3.0 * s, 6.0 + 8.0 * s, 0.08)


## The tracer of a direct shell starts at the model's muzzle (prompt 077).
func _note_tracer(projectile: int, weapon: int, from: Vector3, sim_muzzle: Vector3) -> void:
	if projectile < 0 or weapon == INDIRECT_WEAPON or main.get("combat_view") == null:
		return
	main.combat_view.note_muzzle(projectile, from, sim_muzzle)


## Class of a shot (MUZZLE_CLASSES).
func _muzzle_class(caliber: float, indirect: bool, no_gun: bool) -> String:
	if indirect:
		return "mortar" if no_gun else "howitzer"
	if caliber < GUN_CALIBER:
		return "autocannon"
	return "heavy" if caliber >= HEAVY_CALIBER else "gun"


## The muzzle blast of a gun (prompt 077) at `at` along the barrel
## `direction`: core, fire tongue, glowing gas, powder smoke, side blasts of
## a muzzle brake (`side`: the barrel's horizontal side axis), ground dust,
## light and camera shake, sized by `caliber` and the class `kind`;
## `speed` is the real muzzle velocity (m/s, 0 = unknown).
func muzzle_flash(at: Vector3, direction: Vector3, caliber: float, kind := "", speed := 0.0, brake := false,
		side := Vector3.ZERO) -> void:
	if not visible_at.call(at):
		return
	if caliber < SMALL_CALIBER:
		_simple_flash(at, direction, caliber)
		return
	if kind.is_empty():
		kind = _muzzle_class(caliber, false, false)
	var c: Dictionary = MUZZLE_CLASSES[kind]
	direction = direction.normalized() if direction.length() > 0.01 else Vector3.FORWARD
	var facing := _facing(direction)
	var s := clampf(caliber / 75.0, MUZZLE_SCALE_MIN, MUZZLE_SCALE_MAX)
	var velocity_factor := 1.0
	if speed > 0.0:
		velocity_factor = clampf(speed / TONGUE_VELOCITY_REF, TONGUE_VELOCITY_MIN, TONGUE_VELOCITY_MAX)
	_burst("muzzle_core", at + direction * 0.3 * s, facing, s * float(c.core))
	var tongue := TONGUE_M * s * float(c.tongue) * velocity_factor
	if tongue > 0.0:
		_burst("muzzle_tongue", at, facing, s, Color.WHITE, tongue)
	var detail: bool = _settings.get("fx_muzzle_detail", true)
	var daytime := str(main.atmosphere.daytime) if main.atmosphere != null else "day"
	# The glowing gas is unshaded: dimmer in the dark, so it glows only
	# while hot and does not shine as pale smoke at night.
	var dim := float(GAS_DAYTIME.get(daytime, 1.0))
	var gas_tint := Color(dim, dim, dim)
	if detail:
		_burst("muzzle_gas", at, facing, s * float(c.gas), gas_tint)
		if kind in ["gun", "heavy", "howitzer"]:
			# Prompt 220: compact chain of 4-6 round light-grey smoke puffs
			# drifting forward from cannon muzzles (off on low).
			_burst("muzzle_puffs", at + direction * 0.6 * s, facing, s * float(c.powder))
	_burst("powder", at, facing, s * float(c.powder))
	if brake and detail and side != Vector3.ZERO:
		side = side.normalized()
		for k: float in [-1.0, 1.0]:
			var out := _facing((side * k + direction * 0.25).normalized())
			_burst("muzzle_tongue", at, out, s * 0.7, Color.WHITE, tongue * BRAKE_TONGUE)
			_burst("muzzle_gas", at, out, s * BRAKE_GAS, gas_tint)
	if float(c.dust) > 0.0:
		_muzzle_dust(at, direction, s * float(c.dust))
	_light(at + direction * 1.5 * s, MUZZLE_LIGHT_ENERGY * s * float(c.light) * float(BlastEffects.DAYTIME_LIGHT.get(daytime, 1.0)),
			(MUZZLE_LIGHT_RANGE_M * (0.5 + 0.5 * s)) * float(BlastEffects.DAYTIME_RANGE.get(daytime, 1.0)), MUZZLE_LIGHT_SECONDS)


## Dust blown off the ground by the blast: fanned ahead of a level barrel,
## all round a raised one; colour and amount by the ground under it.
func _muzzle_dust(at: Vector3, direction: Vector3, s: float) -> void:
	var flat := Vector3(direction.x, 0.0, direction.z)
	var raised := direction.y > RAISED_BARREL or flat.length() < 0.1
	var ground := at
	if not raised:
		ground += flat.normalized() * 1.5 * s
	ground.y = sim_host.height_at(ground.x, ground.z)
	var terrain := sim_host.terrain_at(ground.x, ground.z)
	var kind := _ground_kind(terrain)
	# Dust trails' strength (sand and tracks much, grass little, roads
	# hardly); water throws mist.
	var strength := 0.8 if kind == "water" else _trail_strength(terrain)
	if strength <= 0.05:
		return
	# Higher muzzles stir less (a tank's muzzle stands ~2 m high).
	var height := clampf(1.0 - (at.y - ground.y - 2.0) / 6.0, 0.4, 1.0)
	var amount := clampf(strength * 1.6 * height, 0.25, 1.0)
	# Blown dust is lighter than the ground it rises from.
	var tint: Color = GROUND_COLORS[kind].lerp(DUST_LIGHT, 0.4)
	if raised:
		_burst("blast_dust", ground + Vector3.UP * 0.2, Basis.IDENTITY, s * 1.2, tint, 0.0, amount)
	else:
		_burst("muzzle_dust", ground + Vector3.UP * 0.3, _facing(flat), s * (0.7 + 0.4 * strength), tint, 0.0, amount)


func _facing(direction: Vector3) -> Basis:
	var up := Vector3.UP if absf(direction.normalized().y) < 0.98 else Vector3.BACK
	return Basis.looking_at(direction, up)


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
		# On the water surface, not the river bed.
		var level := sim_host.water_level_at(at.x, at.z)
		var surface := Vector3(at.x, level, at.z) if not is_nan(level) else at
		if caliber < 37.0:
			_burst("spray", surface, Basis.IDENTITY, s, color)
			_burst("puff", surface, Basis.IDENTITY, s * 2.0, color)
		else:
			# A fountain of water that falls back (column, drops, surge).
			layers("water_hit", surface, s, color)
			_light(surface + Vector3.UP * 1.5, 2.5 * s, 10.0 * s, 0.08)
			counts["water_hit"] = counts.get("water_hit", 0) + 1
		return
	if caliber < 37.0:
		_burst("fountain", at, Basis.IDENTITY, s, color.darkened(0.15))
		_burst("impact_dust", at, Basis.IDENTITY, s, color)
	else:
		# Prompt 072: flash, earth fan, clods and a cloud of earth lumps.
		layers("ground_hit", at, s, color)
		if caliber >= HEAVY_IMPACT_MM:
			# Prompt 220: two-stage artillery smoke plume + broad low ground haze veil.
			var hs := clampf(caliber / 120.0, 0.85, 1.65)
			if bool(_settings.get("fx_clouds", true)):
				_burst("ground_haze", at + Vector3.UP * 0.9, Basis.IDENTITY, hs, color.lerp(HAZE_BEIGE, 0.72))
			_pending.append({"t": _time + 0.1, "kind": "smoke_collar", "at": at + Vector3.UP * (1.2 * hs),
				"s": 0.95 * hs, "tint": Color.WHITE})
			_pending.append({"t": _time + 0.35, "kind": "explosion_smoke", "at": at + Vector3.UP * (3.2 * hs),
				"s": 0.9 * hs, "tint": Color.WHITE})
		_decal(at, 1.4 + 1.6 * s)
		# The flash lights up the surroundings for one or two frames.
		_light(at + Vector3.UP * 1.5, 4.0 * s, 14.0 * s, 0.12)


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
	if caliber >= HEAVY_IMPACT_MM:
		var hs := clampf(caliber / 120.0, 0.85, 1.5)
		if bool(_settings.get("fx_clouds", true)):
			_burst("ground_haze", Vector3(at.x, sim_host.height_at(at.x, at.z) + 0.9, at.z), Basis.IDENTITY, hs,
				color.lerp(HAZE_BEIGE, 0.72))
		_burst("smoke_collar", at + Vector3.UP * 0.8, Basis.IDENTITY, 0.85 * hs)
		_pending.append({"t": _time + 0.3, "kind": "explosion_smoke", "at": at + Vector3.UP * (2.6 * hs),
			"s": 0.8 * hs, "tint": Color.WHITE})


## A unit is destroyed at `tick`: vehicles play their kill sequence
## (prompt 105) and keep burning; other units explode at once (fireball,
## light, debris, smoke, scorch mark, camera shake) and burn; infantry
## leaves dust.
func destroyed(unit: int, at: Vector3, tick := 0) -> void:
	var index := _index_of(unit)
	var type_id: String = sim_host.unit_type_ids()[index] if index >= 0 else ""
	if type_id.contains("_inf_"):
		if visible_at.call(at):
			_burst("impact_dust", at, Basis.IDENTITY, 0.6, _ground_color(at))
		return
	var size := clampf((sim_host.unit_footprints()[index] if index >= 0 else 6.0) / 7.0, 0.6, 1.4)
	var hit: Dictionary = _last_hits.get(unit, {})
	_last_hits.erase(unit)
	var ammo := _ammo_waiting.has(unit)
	_ammo_waiting.erase(unit)
	var heavy := ammo or (not hit.is_empty() and _time - float(hit.t) < KILL_HIT_S
			and float(hit.caliber_mm) >= KillSequence.HEAVY_KILL_MM)
	var profile := KillSequence.profile(_category_of(index), heavy, unit, tick)
	if profile.is_empty():
		explosion(at, size)
		_start_wreck(unit, at)
		return
	_start_wreck(unit, at, false)
	var ground := at
	ground.y = sim_host.height_at(at.x, at.z)
	var fires: Array = _sockets_of(unit).fires
	var top: Vector3 = (fires[0] as Node3D).global_position if not fires.is_empty() else ground + Vector3.UP * 1.8 * size
	var stages := KillSequence.stages(profile, unit, tick)
	var entry := {"unit": unit, "tick": tick, "profile": profile, "done": [],
		"planned": stages.map(func(st: Array) -> String: return st[1])}
	kill_log.append(entry)
	if kill_log.size() > 64:
		kill_log.pop_front()
	_kills.append({"unit": unit, "at": ground, "top": top, "s": size, "profile": profile, "stages": stages,
		"next": 0, "start": _time, "ammo": ammo, "log": entry})
	counts["kill_" + profile] = counts.get("kill_" + profile, 0) + 1
	_update_kills()


## Catalog category of the unit at `index` ("" if unknown).
func _category_of(index: int) -> String:
	if index < 0:
		return ""
	var unit_type: int = sim_host.unit_types()[index]
	if not _categories.has(unit_type):
		_categories[unit_type] = str(sim_host.unit_type_sound(unit_type).get("category", ""))
	return _categories[unit_type]


## The simulation blew up a destroyed vehicle's ammunition
## (`StrikeEvent::AmmoExplosion`, `strike_view.gd`): its kill sequence
## cooks off right away with the big blast and the flying turret. Without
## a sequence it waits AMMO_WAIT_S for the destroyed event, then plays alone.
func note_ammo_explosion(unit: int, at: Vector3) -> void:
	for kill in _kills:
		if kill.unit != unit:
			continue
		kill.ammo = true
		var age := _time - float(kill.start)
		var stages: Array = kill.stages
		var soon := age + 0.15
		# Like a heavy kill: no smoke stage left, the cook-off next, the
		# fire after it.
		var rest: Array = []
		var fire := false
		for k in range(kill.next, stages.size()):
			var st: Array = stages[k]
			if st[1] == "cookoff":
				continue
			if st[1] == "fire":
				fire = true
				st[0] = maxf(float(st[0]), soon + 0.3)
			if st[1] != "smoke":
				rest.append(st)
		rest.append([soon, "cookoff"])
		if not fire and not (kill.log.done as Array).has("fire"):
			rest.append([soon + 0.4, "fire"])
		rest.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
		stages.resize(kill.next)
		stages.append_array(rest)
		kill.log.planned = stages.map(func(st: Array) -> String: return st[1])
		return
	_ammo_waiting[unit] = {"t": _time, "at": at}


## Kill sequences still running (self-test).
func kills_running() -> int:
	return _kills.size()


## Plays the stages that are due; ammunition explosions without a
## destroyed event play alone after AMMO_WAIT_S.
func _update_kills() -> void:
	for i in range(_kills.size() - 1, -1, -1):
		var kill: Dictionary = _kills[i]
		var age := _time - float(kill.start)
		var stages: Array = kill.stages
		while kill.next < stages.size() and float(stages[kill.next][0]) <= age:
			var stage: String = stages[kill.next][1]
			kill.next += 1
			_kill_stage(kill, stage)
		if kill.next >= stages.size():
			_kills.remove_at(i)
	for unit: int in _ammo_waiting.keys():
		var waiting: Dictionary = _ammo_waiting[unit]
		if _time - float(waiting.t) > AMMO_WAIT_S:
			_ammo_waiting.erase(unit)
			if main.get("blast_effects") != null:
				main.blast_effects.ammo_explosion(unit, waiting.at)


## One stage of a kill sequence (table in `kill_sequence.gd`).
func _kill_stage(kill: Dictionary, stage: String) -> void:
	var s: float = kill.s
	var at: Vector3 = kill.at
	var top: Vector3 = kill.top
	(kill.log.done as Array).append(stage)
	counts["kill_stage_" + stage] = counts.get("kill_stage_" + stage, 0) + 1
	kill_stage.emit(kill.unit, stage, at, s, kill.ammo)
	var wreck := _wreck_of(kill.unit)
	var shown: bool = visible_at.call(at)
	match stage:
		"flash":
			if shown:
				# The jet of flame out of the hatches.
				_burst("muzzle_tongue", top, _facing(Vector3.UP), 1.1 * s, Color.WHITE, 4.5 * s)
				_burst("sparks", top, Basis.IDENTITY, 0.8 * s)
				_light(top, 6.0 * s, 16.0 * s, 0.2)
		"smoke":
			if not wreck.is_empty():
				_smoke_wreck(wreck)
			if shown:
				# Dark smoke pours out (a puff, not a fireball's smoke).
				_burst("puff", top, Basis.IDENTITY, 1.6 * s, Color(0.16, 0.15, 0.14))
				_burst("puff", top + Vector3.UP * 0.8, Basis.IDENTITY, 1.1 * s, Color(0.3, 0.29, 0.27))
		"fire":
			if not wreck.is_empty():
				_ignite_wreck(wreck)
			# Trucks: the fuel catches with a whoosh; the others' flames
			# simply start (the wreck's fire and light).
			if shown and kill.profile.begins_with("soft"):
				_burst("fireball", top, Basis.IDENTITY, 0.8 * s)
		"flare":
			if shown:
				# Smaller than a cook-off: a gout of flame, no blast.
				_burst("fireball", top + Vector3.UP * 0.5, Basis.IDENTITY, 0.7 * s)
				_burst("embers", top, Basis.IDENTITY, 0.6 * s)
				_burst("explosion_smoke", top + Vector3.UP, Basis.IDENTITY, 0.5 * s)
				_light(top, 4.0 * s, 12.0 * s, 0.3)
		"pop":
			explosion(at, 0.5 * s)
		"cookoff":
			if kill.ammo and main.get("blast_effects") != null:
				main.blast_effects.ammo_explosion(kill.unit, at)
			else:
				explosion(at, s)
			if not wreck.is_empty():
				_ignite_wreck(wreck)


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
	if str(event.remains) == "fallen":
		# A salvo into a wood changes dozens of trees in a few ticks: only
		# the first few per frame get their bursts (prompt 085).
		if _tree_effects_left <= 0:
			counts["object_tree_skipped"] = counts.get("object_tree_skipped", 0) + 1
			return
		_tree_effects_left -= 1
	if event.stage == "damaged" and str(event.remains) == "fallen":
		# Stripped by a big blast (prompt 085): leaves, needles or fronds
		# fall; a winter tree that had none drops only a few twigs.
		var leafless: bool = str(event.id) == "env_tree_deciduous" and str(event.variant) == "Bare"
		if not leafless:
			_burst("leaves", at + Vector3.UP * 9.0, Basis.IDENTITY, 1.0, _leaf_color(str(event.id)))
		_burst("debris", at + Vector3.UP * 6.0, Basis.IDENTITY, 0.5)
		return
	if event.stage == "damaged":
		var s := 1.2
		_burst("debris", at + Vector3.UP * 3.0, Basis.IDENTITY, s)
		_burst("impact_dust", at + Vector3.UP * 2.0, Basis.IDENTITY, s * 1.5, dust)
		return
	match str(event.remains):
		"ruin" when main != null and main.get("map_objects") != null \
				and main.map_objects.collapsing.has(int(event.object)):
			# Falls storey by storey (prompt 197): the hit here, dust and
			# debris per storey in collapse_stage.
			_burst("debris", at + Vector3.UP * 4.0, Basis.IDENTITY, 1.4)
			_burst("impact_dust", at + Vector3.UP * 5.0, Basis.IDENTITY, 2.0, dust)
		"ruin":
			var s := 2.2
			_burst("debris", at + Vector3.UP * 4.0, Basis.IDENTITY, s)
			_burst("dust_cloud", at + Vector3.UP * 3.0, Basis.IDENTITY, s * 0.8, dust.lightened(0.15))
			_burst("blast_dust", at + Vector3.UP * 0.3, Basis.IDENTITY, s * 1.4, _ground_color(at))
			_burst("impact_dust", at + Vector3.UP * 5.0, Basis.IDENTITY, s * 1.5, dust)
			_decal(at, 9.0)
		"fallen":
			var heading := deg_to_rad(float(event.fall_deg))
			var crown := at + Vector3(sin(heading), 0.0, -cos(heading)) * 12.0
			crown.y = sim_host.height_at(crown.x, crown.z) + 0.3
			_burst("debris", at + Vector3.UP * 1.5, Basis.IDENTITY, 0.6)
			_burst("impact_dust", at + Vector3.UP * 0.5, Basis.IDENTITY, 0.7, dust)
			_burst("blast_dust", crown, Basis.IDENTITY, 1.1, _ground_color(crown))
			# Pushed over by a vehicle (prompt 106): the shove shakes the
			# crown (leaves, needles or fronds) and the roots tear up earth.
			if str(event.get("cause", "")) == "crushed":
				counts["object_tree_pushed"] = counts.get("object_tree_pushed", 0) + 1
				var leafless: bool = str(event.id) == "env_tree_deciduous" and str(event.variant) == "Bare"
				if not leafless:
					var mid := at.lerp(crown, 0.6) + Vector3.UP * 3.0
					_burst("leaves", mid, Basis.IDENTITY, 1.2, _leaf_color(str(event.id)))
				_burst("blast_dust", at + Vector3.UP * 0.3, Basis.IDENTITY, 0.8, _ground_color(at))
		"gone":
			_burst("debris", at + Vector3.UP, Basis.IDENTITY, 0.9)
			_burst("impact_dust", at + Vector3.UP * 0.6, Basis.IDENTITY, 1.2, dust)
			_burst("blast_dust", at + Vector3.UP * 0.2, Basis.IDENTITY, 0.8, _ground_color(at))
		_:
			_burst("puff", at + Vector3.UP * 0.3, Basis.IDENTITY, 1.2, _ground_color(at))


## A band of a collapsing building starts to fall (prompt 197,
## building_collapse.gd): debris bursts off the outside of the walls at that
## height, a dust cloud rises there and dust rolls down the walls, so the
## cloud grows from the top down; the last band (the ground storey) raises
## the ground dust, the scorch mark and the shake of the old one-step ruin.
func collapse_stage(stage: Dictionary, last: bool) -> void:
	var top: Vector3 = stage.center
	var ground := top
	ground.y = sim_host.height_at(top.x, top.z)
	if not visible_at.call(ground):
		return
	counts["collapse_stage"] = counts.get("collapse_stage", 0) + 1
	var half: Vector2 = stage.half
	var size := clampf(maxf(half.x, half.y) / 3.0, 0.8, 3.0)
	var dust: Color = GROUND_COLORS["object"].lightened(0.15)
	var mid := top + Vector3.DOWN * float(stage.height) * 0.5
	_burst("debris", top, Basis.IDENTITY, size)
	_burst("dust_cloud", mid, Basis.IDENTITY, size * 0.7, dust)
	# Dust rolling down the outside of two walls.
	for side: Vector3 in [Vector3(half.x + 0.8, 0.0, 0.0), Vector3(0.0, 0.0, -half.y - 0.8)]:
		var down: Vector3 = mid + side
		down.y = lerpf(mid.y, ground.y + 1.0, 0.5)
		_burst("impact_dust", down, Basis.IDENTITY, size * 1.1, dust)
	if last:
		_burst("blast_dust", ground + Vector3.UP * 0.3, Basis.IDENTITY, size * 1.4, _ground_color(ground))
		_burst("dust_cloud", ground + Vector3.UP * 2.5, Basis.IDENTITY, size * 0.9, dust)
		_decal(ground, 9.0)


## Color of the leaves a tree of catalog `id` drops (prompt 085).
func _leaf_color(id: String) -> Color:
	return LEAF_COLORS.get(id, LEAF_COLORS["env_tree_deciduous"])


## A vehicle explosion of relative size `s` (1 = medium tank).
func explosion(at: Vector3, s := 1.0) -> void:
	if not visible_at.call(at):
		return
	var center := at + Vector3.UP * 1.2 * s
	var ground := at
	ground.y = sim_host.height_at(at.x, at.z)
	# Prompt 072 & 220: flash and rays, fireball, flat ground dust/shockwave
	# ring, bright warm-lit lower collar, velvety black smoke mushroom, haze.
	layers("vehicle", ground, s, _ground_color(ground))
	if main.get("blast_effects") != null:
		main.blast_effects.ground_ring(ground, 7.5 * s)
	# The flash; a longer glow (1.4 s) cost 4 FPS at "high" with 50
	# explosions (prompt 072), the video lights the ground only in the flash.
	_light(center + Vector3.UP, 10.0 * s, 28.0 * s, 0.4)
	_decal(ground, 7.0 * s)


# --- Bursts -----------------------------------------------------------------


## Starts the layers of EXPLOSION_LAYERS[profile] on the ground point `at`
## with scale `s`; `ground` is the colour of the ground there. Layers with a
## delay wait in `_pending` (the caller checked the visibility).
func layers(profile: String, at: Vector3, s: float, ground: Color) -> void:
	for layer: Array in EXPLOSION_LAYERS[profile]:
		var kind := str(layer[0])
		if kind == "ground_haze" and not bool(_settings.get("fx_clouds", true)):
			continue
		var tint := Color.WHITE
		match str(layer[4]):
			"ground":
				tint = ground
			"earth":
				tint = ground.lerp(EARTH_GREY, 0.5)
			"dark":
				tint = ground.darkened(0.3)
			"sand_dust":
				tint = ground.lerp(DUST_LIGHT, 0.65)
			"haze":
				tint = ground.lerp(HAZE_BEIGE, 0.72)
		var point := at + Vector3.UP * (float(layer[3]) * s)
		var size := float(layer[2]) * s
		if float(layer[1]) <= 0.0:
			_burst(kind, point, Basis.IDENTITY, size, tint)
		else:
			_pending.append({"t": _time + float(layer[1]), "kind": kind, "at": point, "s": size, "tint": tint})


func _start_pending() -> void:
	for i in range(_pending.size() - 1, -1, -1):
		var entry: Dictionary = _pending[i]
		if entry.t <= _time:
			_pending.remove_at(i)
			_burst(entry.kind, entry.at, Basis.IDENTITY, entry.s, entry.tint)



## Fires the next emitter of a pool at `at`, scaled by `s`, tinted.
## `reach` (m, prompt 077): how far the particles fly in their life (the fire
## tongue's length); `amount` scales the particle count (ground dust).
func _burst(kind: String, at: Vector3, basis: Basis, s: float, tint := Color.WHITE, reach := 0.0,
		amount := 1.0) -> void:
	var ratio := _lod(at, 3.0 * s)
	if kind == "muzzle_core":
		# The core shows who fires, from any distance (prompt 077): a
		# bright point of at least CORE_MIN_PIXELS.
		var pixels := _pixels(at, 3.0 * s)
		if pixels < CORE_MIN_PIXELS * 3.0:
			s *= CORE_MIN_PIXELS * 3.0 / maxf(pixels, 0.01)
		ratio = 1.0
	if ratio <= 0.0 or (kind in DETAIL_KINDS and ratio * FULL_PIXELS < DETAIL_PIXELS):
		return
	var pool: Array = _pools[kind]
	var next: int = _pool_next[kind]
	var p: GPUParticles3D = pool[next]
	_pool_next[kind] = (next + 1) % pool.size()
	p.visible = true
	p.global_transform = Transform3D(basis.scaled(Vector3.ONE * s), at)
	if reach > 0.0:
		# Speeds are in the emitter's scaled space.
		var pm := p.process_material as ParticleProcessMaterial
		pm.initial_velocity_max = reach / (s * p.lifetime)
		pm.initial_velocity_min = pm.initial_velocity_max * 0.15
	p.amount_ratio = ratio * amount
	p.set_instance_shader_parameter(&"tint", tint)
	# The cloud the puffs belong to (prompt 200): lit as one body.
	p.set_instance_shader_parameter(&"cloud_center", Vector4(at.x, at.y, at.z, CLOUD_RADIUS_M * s))
	p.restart()
	counts[kind] = counts.get(kind, 0) + 1


## Share of particles for an effect of `size_m` at `at`: 0 = skip (too small
## on screen), up to 1.
func _lod(at: Vector3, size_m: float) -> float:
	var pixels := _pixels(at, size_m)
	if pixels < MIN_PIXELS:
		return 0.0
	return clampf(pixels / FULL_PIXELS, 0.35, 1.0)


## Size in pixels on screen of something `size_m` across at `at`.
func _pixels(at: Vector3, size_m: float) -> float:
	var camera: Camera3D = main.camera
	var distance := camera.global_position.distance_to(at)
	# Headless runs have a tiny dummy window: assume full HD.
	var height := get_viewport().get_visible_rect().size.y
	if DisplayServer.get_name() == "headless":
		height = 1080.0
	return size_m * height / (2.0 * distance * tan(deg_to_rad(camera.fov) * 0.5) + 0.001)


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
	# Tall grass burns down to stubble under the scorch mark (prompt 073).
	if main.get("grass_trails") != null:
		main.grass_trails.burn(at, size_m * 0.35)
	# ... and a big blast sets the stand on fire (prompt 082).
	if main.get("grass_fire") != null:
		main.grass_fire.ignite(at, size_m)
	counts["decal"] = counts.get("decal", 0) + 1


func _add_shake(at: Vector3, strength: float) -> void:
	# Screen shake can be switched off in the settings (prompt 043).
	if not shake_enabled or not Settings.get_value("screen_shake"):
		return
	var rig: Node3D = main.camera_rig
	var reach: float = rig.distance * 1.5 + 50.0
	var falloff := clampf(1.0 - (rig.focus as Vector3).distance_to(at) / reach, 0.0, 1.0)
	# Take the strongest jolt instead of summing linearly so barrages never build
	# up into continuous vibration.
	_shake = maxf(_shake, minf(strength * falloff, 1.0))


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


## Starts the wreck of `unit`: fire, smoke column and light; with `lit`
## false (kill sequences, prompt 105) everything waits for
## `_smoke_wreck` and `_ignite_wreck`.
func _start_wreck(unit: int, at: Vector3, lit := true) -> void:
	var root := Node3D.new()
	root.name = "Wreck%d" % unit
	_bursts_root.add_child(root)
	# Hidden at once in the fog, not only at the next visibility pass (prompt 098).
	root.visible = visible_at.call(at)
	var points: Array[Vector3] = []
	for socket: Node3D in _sockets_of(unit).fires:
		points.append(socket.global_position)
	if points.is_empty():
		points.append(at + Vector3.UP * 1.5)
	var wreck := {"unit": unit, "root": root, "position": at, "age": 0.0, "fires": [], "light": null, "smoke": null,
		"collar": null, "lit": lit, "smoking": lit}
	var wind := _wind()
	for point in points.slice(0, 2):
		var fire := EffectLibrary.build("fire", _settings.fx_particles, _settings.fx_soft, wind)
		root.add_child(fire)
		fire.global_position = point
		fire.emitting = lit
		wreck.fires.append(fire)
	var use_collar := bool(_settings.get("fx_clouds", true))
	if use_collar:
		var collar := EffectLibrary.build("wreck_collar", _settings.fx_particles, _settings.fx_soft, wind)
		root.add_child(collar)
		collar.global_position = points[0] + Vector3.UP * 0.5
		var cbase: Vector3 = collar.global_position
		collar.set_instance_shader_parameter(&"cloud_center", Vector4(cbase.x, cbase.y, cbase.z, COLUMN_RADIUS_M))
		collar.emitting = lit
		wreck.collar = collar
	var smoke := EffectLibrary.build("smoke_column", _settings.fx_particles, _settings.fx_soft, wind)
	root.add_child(smoke)
	smoke.global_position = points[0] + Vector3.UP * (2.2 if use_collar else 0.8)
	# A column (prompt 200): lit by the side its puffs drift to.
	var base: Vector3 = smoke.global_position
	smoke.set_instance_shader_parameter(&"cloud_center", Vector4(base.x, base.y, base.z, -COLUMN_RADIUS_M))
	smoke.emitting = lit
	wreck.smoke = smoke
	var lights := 0
	for other in _wrecks:
		if other.light != null:
			lights += 1
	if lights < int(_settings.fx_fire_lights):
		var light := OmniLight3D.new()
		light.light_color = FIRE_LIGHT_COLOR
		light.omni_range = 12.0
		light.light_energy = 2.0
		light.shadow_enabled = false
		root.add_child(light)
		light.global_position = points[0] + Vector3.UP * 1.0
		light.visible = lit
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


## The wreck of `unit` ({} if none).
func _wreck_of(unit: int) -> Dictionary:
	for i in range(_wrecks.size() - 1, -1, -1):
		if _wrecks[i].unit == unit:
			return _wrecks[i]
	return {}


## Smoke pours out of a wreck whose fire has not taken hold yet.
func _smoke_wreck(wreck: Dictionary) -> void:
	if wreck.lit or wreck.smoking:
		return
	wreck.smoking = true
	var smoke: GPUParticles3D = wreck.smoke
	smoke.amount_ratio = SMOKE_ONLY
	smoke.emitting = true


## The fire takes hold: flames, light, the full smoke column; the burn
## time starts now. A wreck pushed over the fire budget only smolders.
func _ignite_wreck(wreck: Dictionary) -> void:
	if wreck.lit:
		return
	wreck.lit = true
	wreck.smoking = true
	(wreck.smoke as GPUParticles3D).emitting = true
	if wreck.age >= BURN_SECONDS:
		return
	for fire: GPUParticles3D in wreck.fires:
		fire.emitting = true
	if wreck.collar != null:
		(wreck.collar as GPUParticles3D).emitting = true
	if wreck.light != null:
		(wreck.light as OmniLight3D).visible = true


## Fire, then smoke only (thinner, lighter), then nothing.
func _update_wrecks(delta: float) -> void:
	for i in range(_wrecks.size() - 1, -1, -1):
		var wreck: Dictionary = _wrecks[i]
		if not wreck.lit:
			# Before the fire of a kill sequence: smoke only, no burn time.
			if wreck.smoking:
				(wreck.smoke as GPUParticles3D).amount_ratio = SMOKE_ONLY
			continue
		wreck.age += delta
		var age: float = wreck.age
		var smoke: GPUParticles3D = wreck.smoke
		if age < BURN_SECONDS:
			# The fire grows in the first seconds.
			var ramp := clampf(age / 3.0, 0.3, 1.0)
			smoke.amount_ratio = ramp
			if wreck.collar != null:
				(wreck.collar as GPUParticles3D).amount_ratio = ramp
			if wreck.light != null:
				var light: OmniLight3D = wreck.light
				light.light_energy = 2.2 * (0.75 + 0.25 * sin(_time * 13.0 + i) * sin(_time * 7.3 + i * 2.0))
		elif age < BURN_SECONDS + SMOLDER_SECONDS:
			for fire: GPUParticles3D in wreck.fires:
				fire.emitting = false
			if wreck.collar != null:
				(wreck.collar as GPUParticles3D).emitting = false
			if wreck.light != null:
				(wreck.light as Node).queue_free()
				wreck.light = null
			smoke.amount_ratio = 0.3
		elif smoke.emitting:
			smoke.emitting = false
			if wreck.collar != null:
				(wreck.collar as GPUParticles3D).emitting = false
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
	var wet := wet()
	var ground_fx: bool = _settings.get("fx_ground_fx", true)
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
			fx.effect = drive_effect(terrain, wet)
			if not ground_fx and fx.effect != "dust":
				fx.effect = PLAIN_EFFECTS.get(fx.effect, "")
			(fx.dust as GPUParticles3D).set_instance_shader_parameter(&"tint", GROUND_COLORS[fx.kind])
			if GROUND_EFFECTS.has(fx.effect):
				_place_ground(fx, node, position)
		var driving: bool = near and moving[i] != 0 and speed > 0.5
		var dust: GPUParticles3D = fx.dust
		dust.emitting = driving and fx.effect == "dust" and fx.strength > 0.0
		# Prompt 104: the vehicles drive slowly (prompt 097); the dust shows
		# from walking pace on.
		dust.amount_ratio = clampf(fx.strength * (0.3 + speed / 4.0), 0.15, 1.0)
		for effect: String in GROUND_EFFECTS:
			var on: bool = driving and fx.effect == effect
			if not on and not (fx.ground as Dictionary).has(effect):
				continue
			for emitter: GPUParticles3D in _ground_emitters(fx, node, effect):
				emitter.emitting = on
				emitter.amount_ratio = clampf(0.35 + speed / 4.0, 0.35, 1.0)
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
	var fx := {"dust": dust, "exhausts": exhausts, "ground": {}, "kind": "earth", "strength": 0.0, "effect": "dust",
		"length": length, "last": node.global_position, "frame": 0}
	_vehicles[id] = fx
	return fx


## The emitters of a ground effect of a vehicle, built when first needed
## (most vehicles never drive through a ford).
func _ground_emitters(fx: Dictionary, node: Node3D, effect: String) -> Array:
	var ground: Dictionary = fx.ground
	if ground.has(effect):
		return ground[effect]
	var length: float = fx.length
	var emitters: Array[GPUParticles3D] = []
	for kind: String in GROUND_EFFECTS[effect]:
		var emitter := EffectLibrary.build(kind, _settings.fx_particles, _settings.fx_soft, _wind())
		node.add_child(emitter)
		# Models face -Z: clods and snow fly off behind the tracks, spray
		# at the bow, the wake trails behind.
		emitter.position = Vector3(0.0, 0.3, length * (-0.45 if kind == "bow_spray" else 0.4))
		emitters.append(emitter)
	ground[effect] = emitters
	_place_ground(fx, node, node.global_position)
	return emitters


## Tint of the ground effect; spray and wake float on the water surface.
func _place_ground(fx: Dictionary, node: Node3D, position: Vector3) -> void:
	var ground: Dictionary = fx.ground
	if not ground.has(fx.effect):
		return
	var tint: Color = GROUND_COLORS[fx.kind]
	if fx.effect == "mud" and fx.kind != "mud":
		# Soaked earth or sand: darker than its dust.
		tint = GROUND_COLORS["mud"] if fx.kind == "earth" else tint.darkened(0.45)
	var level := sim_host.water_level_at(position.x, position.z)
	for emitter: GPUParticles3D in ground[fx.effect]:
		emitter.set_instance_shader_parameter(&"tint", tint)
		if fx.effect == "wake" and not is_nan(level):
			var lift := 0.15 if emitter.name == "bow_spray" else 0.04
			emitter.position.y = (level - position.y) / maxf(node.scale.y, 0.01) + lift


func _remove_vehicle(id: int) -> void:
	var fx: Dictionary = _vehicles[id]
	(fx.dust as Node).queue_free()
	for emitters: Array in (fx.ground as Dictionary).values():
		for emitter: Node in emitters:
			emitter.queue_free()
	for exhaust: Node in fx.exhausts:
		exhaust.queue_free()
	_vehicles.erase(id)


## Ground effect of a vehicle driving on terrain id `terrain` (prompt 104):
## "dust", "mud" (clods), "snow" (snow dust), "wake" (ford) or "" (none:
## deep water, a wet road). `wet`: the ground is soaked (`wet()`), so bare
## earth, meadow and sand throw mud instead of dust.
func drive_effect(terrain: int, wet: bool) -> String:
	if terrain == TERRAIN_WATER_DEEP:
		return ""
	match _ground_kind(terrain):
		"water":
			return "wake"
		"mud":
			return "mud"
		"snow":
			return "snow"
		"road":
			return "" if wet else "dust"
	return "mud" if wet else "dust"


## Whether rain soaked the ground (terrain wetness of weather_look.gd).
func wet() -> bool:
	var look: Node = main.get("weather_look")
	return look != null and look.wetness >= WET_GROUND


## Dust color kind of a terrain id in the current biome.
func _ground_kind(terrain: int) -> String:
	var biome := _biome()
	match terrain:
		0, TERRAIN_TALL_GRASS:
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
		0, TERRAIN_TALL_GRASS:
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
	EffectLibrary.use_clouds(bool(_settings.get("fx_clouds", true)))
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


## Sockets of a unit's model, found once per model (a base building's
## borrowed model replaces its placeholder, prompt 076).
func _sockets_of(unit: int) -> Dictionary:
	var node: Node3D = main._unit_nodes.get(unit)
	var model := node.get_node_or_null("Model") if node != null else null
	if _sockets.has(unit) and _sockets[unit].model == model:
		return _sockets[unit]
	var entry := {"muzzles": [], "fires": [], "exhausts": [], "rockets": [], "model": model}
	if model != null:
		entry.muzzles = model.find_children("Socket_Muzzle*", "", true, false)
		entry.fires = model.find_children("Socket_Fire_*", "", true, false)
		entry.exhausts = model.find_children("Socket_Exhaust_*", "", true, false)
		entry.rockets = model.find_children("Socket_Rocket_*", "", true, false)
	_sockets[unit] = entry
	return entry


## The model's muzzle socket nearest to the simulation's muzzle (within
## 6 m), or null. Cannon rounds (from 20 mm) come from the main gun's
## `Socket_Muzzle` if it is near, small arms from the machine guns'
## `Socket_Muzzle_NN` (prompt 077: the simulation's muzzle often lies
## nearer to the bow MG than to the gun's muzzle).
func _muzzle_of(unit: int, sim_muzzle: Vector3, caliber := 0.0) -> Node3D:
	var main_gun := caliber >= SMALL_CALIBER
	var best: Node3D = null
	var best_distance := 6.0
	var any: Node3D = null
	var any_distance := 6.0
	for entry: Variant in _sockets_of(unit).muzzles:
		# A socket of a part removed from the model (a gun or MG lost or
		# swapped while the model stays) is freed: skip it.
		if not is_instance_valid(entry):
			continue
		var socket := entry as Node3D
		var d := socket.global_position.distance_to(sim_muzzle)
		if d < any_distance:
			any_distance = d
			any = socket
		if (socket.name == &"Socket_Muzzle") == main_gun and d < best_distance:
			best_distance = d
			best = socket
	return best if best != null else any


# --- Guns: recoil, rocking hull, raised barrels (prompt 077) ---------------------


## The kick state of a unit's `Gun` node, found once per model ({} when the
## model has none). Presentation only: the simulation and the mouse
## selection know the unit node, which never moves here.
func _gun_of(unit: int) -> Dictionary:
	var node: Node3D = main._unit_nodes.get(unit)
	var model := node.get_node_or_null("Model") as Node3D if node != null else null
	if _guns.has(unit):
		var entry: Dictionary = _guns[unit]
		if entry.model == model and is_instance_valid(entry.gun):
			return entry
		forget_unit(unit)
	if model == null:
		return {}
	var gun := model.find_child("Gun", true, false) as Node3D
	if gun == null:
		return {}
	# A barrel modeled standing steeply (mortars): its muzzle well above
	# the trunnion.
	var tip := Vector3.ZERO
	var socket := gun.get_node_or_null("Socket_Muzzle") as Node3D
	if socket != null:
		tip = socket.position
	var steep := tip.length() > 0.1 and tip.y / tip.length() > 0.5
	var entry := {"gun": gun, "rest": gun.transform, "model": model, "model_rest": model.transform, "steep": steep,
		"brake": bool(model.get_meta(&"muzzle_brake", false)), "recoil": 0.0, "t": 100.0, "rock": 0.0,
		"axis": Vector3.RIGHT, "elevation": 0.0, "target": 0.0, "last": -100.0, "active": false}
	_guns[unit] = entry
	return entry


## Puts a unit's gun and hull back at rest and drops its state (the model
## was replaced, tools).
func forget_unit(unit: int) -> void:
	if not _guns.has(unit):
		return
	var entry: Dictionary = _guns[unit]
	if is_instance_valid(entry.gun):
		(entry.gun as Node3D).transform = entry.rest
	if is_instance_valid(entry.model):
		(entry.model as Node3D).transform = entry.model_rest
	_guns.erase(unit)


## Starts the recoil of the gun and the rocking of the hull.
func _kick(entry: Dictionary, caliber: float, kind: String, direction: Vector3) -> void:
	var c: Dictionary = MUZZLE_CLASSES[kind]
	var s := clampf(caliber / 75.0, MUZZLE_SCALE_MIN, MUZZLE_SCALE_MAX)
	entry.recoil = minf(caliber * RECOIL_M_PER_MM * float(c.recoil), RECOIL_MAX_M)
	entry.rock = ROCK_DEG * s * float(c.rock)
	# Rocks about the horizontal axis across the fire: the muzzle side
	# rises first.
	var model: Node3D = entry.model
	var local := model.global_basis.inverse() * direction
	local.y = 0.0
	entry.axis = local.normalized().cross(Vector3.UP) if local.length() > 0.01 else Vector3.RIGHT
	entry.t = 0.0
	entry.active = true
	recoils += 1


## The barrel's elevation from a shell's launch velocity; `snap`: at once
## (the gun was laid before it fired).
func _raise(entry: Dictionary, velocity: Vector3, snap: bool) -> void:
	var flat := Vector2(velocity.x, velocity.z).length()
	entry.target = clampf(atan2(velocity.y, flat), 0.0, deg_to_rad(80.0))
	if snap:
		entry.elevation = entry.target
	entry.active = true
	_pose(entry)


func _update_guns(delta: float) -> void:
	if not _pending_elevations.is_empty():
		for projectile: int in _pending_elevations.keys():
			var wait: Dictionary = _pending_elevations[projectile]
			var velocity := _velocity_of(projectile)
			if velocity != Vector3.ZERO:
				var entry := _gun_of(int(wait.unit))
				if not entry.is_empty():
					_raise(entry, velocity, false)
				_pending_elevations.erase(projectile)
			elif _time - float(wait.t) > 2.0:
				_pending_elevations.erase(projectile)
	for unit: int in _guns.keys():
		var entry: Dictionary = _guns[unit]
		if not entry.active:
			continue
		if not is_instance_valid(entry.gun) or not is_instance_valid(entry.model):
			_guns.erase(unit)
			continue
		entry.t += delta
		if entry.target > 0.0:
			var index := _index_of(unit)
			var moving := index >= 0 and sim_host.unit_moving()[index] != 0
			if moving or _time - float(entry.last) > ELEVATION_HOLD_S:
				entry.target = 0.0
		entry.elevation = move_toward(entry.elevation, entry.target, deg_to_rad(ELEVATE_DEG_PER_S) * delta)
		_pose(entry)
		if entry.t > RECOIL_BACK_S + RECOIL_RETURN_S and entry.t > ROCK_DECAY_S * 5.0 \
				and entry.elevation == 0.0 and entry.target == 0.0:
			entry.active = false
			(entry.gun as Node3D).transform = entry.rest
			(entry.model as Node3D).transform = entry.model_rest


## Sets the gun (elevation, recoil) and the hull (rocking) for the state.
func _pose(entry: Dictionary) -> void:
	var t: float = entry.t
	var back := 0.0
	if t < RECOIL_BACK_S:
		back = t / RECOIL_BACK_S
	elif t < RECOIL_BACK_S + RECOIL_RETURN_S:
		back = 1.0 - smoothstep(0.0, 1.0, (t - RECOIL_BACK_S) / RECOIL_RETURN_S)
	var rest: Transform3D = entry.rest
	var basis := rest.basis * Basis(Vector3.RIGHT, entry.elevation)
	# Back along the barrel (+Z of the gun: forward is -Z).
	(entry.gun as Node3D).transform = Transform3D(basis, rest.origin + basis * Vector3(0.0, 0.0, back * float(entry.recoil)))
	var angle := deg_to_rad(float(entry.rock)) * exp(-t / ROCK_DECAY_S) * sin(TAU * ROCK_HZ * t)
	var model_rest: Transform3D = entry.model_rest
	(entry.model as Node3D).transform = Transform3D(model_rest.basis * Basis(entry.axis, angle), model_rest.origin)


## Velocity (m/s) of a direct projectile or an indirect shell of this
## frame's snapshot, or ZERO.
func _velocity_of(projectile: int) -> Vector3:
	var frame := Engine.get_process_frames()
	if frame != _velocities_frame:
		_velocities_frame = frame
		_velocities.clear()
		var ids := sim_host.projectile_ids()
		var velocities := sim_host.projectile_velocities()
		for i in mini(ids.size(), velocities.size()):
			_velocities[ids[i]] = velocities[i]
		ids = sim_host.shell_ids()
		velocities = sim_host.shell_velocities()
		for i in mini(ids.size(), velocities.size()):
			_velocities[ids[i]] = velocities[i]
	return _velocities.get(projectile, Vector3.ZERO)
