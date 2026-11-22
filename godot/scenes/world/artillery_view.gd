extends Node3D
## Indirect fire and smoke (prompt 032): shells in their arc, impact
## markers for the local player's own shells, and smoke clouds.
##
## Presentation only: everything comes from SimHost (`shell_*`,
## `smoke_*`); the simulation decides where shells land and which cells the
## smoke hides. Muzzle flashes and explosions come from the ordinary `shot`
## and `hit` events (`effects.gd`). Rockets (prompt 076: shells that
## `effects.rockets` noted at a launcher's rail) are drawn by
## `rocket_effects.gd` with body, flame and trail instead of a ball. Sound
## (prompt 029) can hook into `shell_incoming` for the whistle of a shell about to land. Godot
## randomness only places smoke puffs and never reaches the simulation.

## Shells in flight (prompt 199) have no body: each is a small warm light
## that its speed stretches into a short line along the flight (the
## distance it covers in one frame, like motion blur), with a faint vapour
## trail behind it (`light_streak.gd`). The light flashes up after the
## launch, glints now and then in mid-flight and grows in the last seconds
## before the impact, the same for the same shell id in every replay.

## Emitted once per shell shortly before it lands (whistle).
signal shell_incoming(position: Vector3, caliber_mm: float, smoke: bool)

const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const Effects := preload("res://scenes/world/effects.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const LightStreak := preload("res://scenes/world/light_streak.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
## Seconds before the impact at which `shell_incoming` fires.
const WHISTLE_SECONDS := 1.5
## Shell light (prompt 199): real width per mm of caliber, held on screen
## between SHELL_MIN_PX and SHELL_MAX_PX (at 1080 lines) by the material.
const SHELL_WIDTH_PER_MM := 0.004
const SHELL_MIN_PX := 2.2
const SHELL_MAX_PX := 4.0
const SHELL_COLOR := Color(1.0, 0.7, 0.3)
## Brightness (HDR, above 1.1 it glows) per time of day: the light stands
## out more in the dark.
const SHELL_ENERGY := {"day": 3.6, "morning": 3.8, "evening": 3.8, "night": 4.4}
## The streak covers one frame of flight; the frame time is held in this
## range so a slow frame does not draw a long bar.
const STREAK_FRAME_MIN_S := 1.0 / 120.0
const STREAK_FRAME_MAX_S := 1.0 / 30.0
## Brightness over the flight: full right after the launch, falling over
## LAUNCH_GLOW_S; in mid-flight a faint MID_FLOOR with a glint up to
## MID_GLOW that comes and goes; rising to full over the last FINAL_GLOW_S.
const LAUNCH_GLOW_S := 0.6
const FINAL_GLOW_S := 2.0
const MID_GLOW := 0.6
const MID_FLOOR := 0.2
## Vapour trail: a thin, pale line over the last TRAIL_SECONDS of flight,
## fading toward its end; hardly there at night, weaker in rain, snow and
## fog (the haze swallows it). Off at low quality (`fx_shell_trails`).
const TRAIL_SECONDS := 0.3
const TRAIL_COLOR := Color(0.9, 0.9, 0.86)
const TRAIL_MIN_PX := 1.2
const TRAIL_MAX_PX := 2.4
const TRAIL_ENERGY := {"day": 0.24, "morning": 0.18, "evening": 0.15, "night": 0.03}
const TRAIL_WEATHER := {"rain": 0.6, "snow": 0.6, "fog": 0.4}
## Impact markers (prompt 084): a thin, faint ring where one of the local
## player's own shells will land, a hint seen in passing, not a signal.
## They appear together with the whistle (the last WHISTLE_SECONDS of the
## flight: the player hears the shell coming and sees where; a howitzer's
## long flight would otherwise lay rings for many seconds), fade in over
## MARKER_FADE_IN and fade out over MARKER_FADE_OUT after the impact instead
## of vanishing with the shell. No pulsing.
const MARKER_LEAD_SECONDS := WHISTLE_SECONDS
const MARKER_FADE_IN := 0.5
const MARKER_FADE_OUT := 0.4
## Width of the ring line in meters.
const MARKER_LINE := 0.25
## Ring radius as a share of the fragmentation radius of the simulation
## (`[blast]`; the hit chance falls linearly to 0 there): 0.4 is the core in
## which a soldier in the open is still hit at 60 % of the point-blank
## chance. Never larger than the old ring (caliber × 0.1, at least 2 m).
const MARKER_CORE_SHARE := 0.4
## Markers of the same kind whose centers lie closer than this share of the
## radius are drawn as one (a salvo on one point is one ring, not a carpet).
const MARKER_MERGE_SHARE := 0.6
## Pale, lit, without emission: dims with the light at dusk and night. In
## the standard palette a pale khaki that sits in the meadow; the palettes
## for color vision deficiencies use their own color (`relation_colors.gd`).
const MARKER_COLOR := Color(0.86, 0.88, 0.72, 0.35)
const MARKER_SMOKE_COLOR := Color(0.85, 0.85, 0.85, 0.3)
## Puffs per cloud and their look.
const PUFFS := 9
const SMOKE_COLOR := Color(0.78, 0.78, 0.76)
## Fire smoke (prompt 041): dark, oily.
const FIRE_SMOKE_COLOR := Color(0.16, 0.15, 0.14)
const SMOKE_OPACITY := 0.85

var main: Node3D
var sim_host: SimHost
## Shells drawn in the last frame and smoke clouds shown (self-test).
var shells_drawn := 0
## Impact markers drawn in the last frame (self-test).
var markers_drawn := 0
## Off for pictures without interface (`trailer_shots.gd`).
var markers_enabled := true
var clouds_drawn := 0
var incoming := 0

var _shells: Array[MeshInstance3D] = []
var _trails: Array[MeshInstance3D] = []
var _markers: Array[MeshInstance3D] = []
var _streak_mesh: QuadMesh
var _shell_material: ShaderMaterial
var _trail_material: ShaderMaterial
## Daytime, weather and quality the materials were set for.
var _look_key := ""
var _trails_on := true
## Seconds left of each shell when it was first seen (time since launch).
var _first_left: Dictionary[int, float] = {}
## Ring mesh per radius in half meters (the line keeps its width).
var _marker_meshes: Dictionary[int, TorusMesh] = {}
var _marker_material: StandardMaterial3D
var _marker_smoke_material: StandardMaterial3D
## Fragmentation radius per caliber in mm×10 (from SimHost).
var _blast_radii: Dictionary[int, float] = {}
## Markers shown last frame (shell id -> [target, radius, smoke, alpha]) and
## markers fading out after their impact ([target, radius, smoke, alpha]).
var _shown_markers: Dictionary[int, Array] = {}
var _fading_markers: Array[Array] = []
var _puff_mesh: QuadMesh
var _dark_puff_mesh: QuadMesh
## Cloud id -> its puffs (MeshInstance3D) and their offsets (x, y, z in
## units of the radius).
var _clouds: Dictionary[int, Array] = {}
var _whistled: Dictionary[int, bool] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_streak_mesh = LightStreak.mesh()
	_shell_material = LightStreak.material(SHELL_COLOR, SHELL_ENERGY.day, SHELL_MIN_PX, SHELL_MAX_PX, 0.25)
	_trail_material = LightStreak.material(TRAIL_COLOR, TRAIL_ENERGY.day, TRAIL_MIN_PX, TRAIL_MAX_PX, 0.0)
	_marker_material = _marker_material_of(RelationColors.own_or(MARKER_COLOR))
	_marker_smoke_material = _marker_material_of(MARKER_SMOKE_COLOR)
	_puff_mesh = QuadMesh.new()
	_puff_mesh.size = Vector2(1, 1)
	var puff := StandardMaterial3D.new()
	puff.albedo_texture = EffectTextures.smoke()
	puff.albedo_color = SMOKE_COLOR
	puff.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	puff.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	puff.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	puff.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	_puff_mesh.material = puff
	_dark_puff_mesh = QuadMesh.new()
	_dark_puff_mesh.size = Vector2(1, 1)
	var dark := puff.duplicate() as StandardMaterial3D
	dark.albedo_color = FIRE_SMOKE_COLOR
	_dark_puff_mesh.material = dark


## Lit (no emission), half transparent, no specular shine.
func _marker_material_of(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.roughness = 1.0
	m.metallic_specular = 0.0
	m.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
	return m


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_draw_shells(delta)
	_draw_smoke()


## Brightness of the shell `id` (0..1) `flown_s` seconds after its launch
## with `left_s` seconds to its impact (game time, prompt 199).
static func shell_glow(id: int, flown_s: float, left_s: float) -> float:
	var launch := clampf(1.0 - flown_s / LAUNCH_GLOW_S, 0.0, 1.0)
	var final := clampf(1.0 - left_s / FINAL_GLOW_S, 0.0, 1.0)
	var h := float((id * 2654435761) & 0xffffffff) / 4294967296.0
	var wave := sin(flown_s * (4.0 + 3.0 * h) + h * TAU) * sin(flown_s * (1.1 + h) + h * 17.0)
	var mid := lerpf(MID_FLOOR, MID_GLOW, clampf(wave * 1.6 + 0.2, 0.0, 1.0))
	return maxf(maxf(launch, final * final), mid)


## Material brightness for the time of day, weather and quality.
func _update_look() -> void:
	var atmosphere: Node = main.get("atmosphere") if main != null else null
	var daytime := str(atmosphere.daytime) if atmosphere != null else "day"
	var weather := str(atmosphere.weather) if atmosphere != null else "clear"
	var quality := str(atmosphere.quality) if atmosphere != null else GraphicsQuality.DEFAULT
	var key := "%s/%s/%s" % [daytime, weather, quality]
	if key == _look_key:
		return
	_look_key = key
	_shell_material.set_shader_parameter("energy", float(SHELL_ENERGY.get(daytime, SHELL_ENERGY.day)))
	_trail_material.set_shader_parameter("energy",
			float(TRAIL_ENERGY.get(daytime, TRAIL_ENERGY.day)) * float(TRAIL_WEATHER.get(weather, 1.0)))
	_trails_on = bool(GraphicsQuality.settings(quality).get("fx_shell_trails", true))


## Shells along their arc (advanced by the time since the last tick) and,
## for the local player's shells, a ring where they will land.
func _draw_shells(delta: float) -> void:
	var positions := sim_host.shell_positions()
	var velocities := sim_host.shell_velocities()
	var targets := sim_host.shell_targets()
	var owners := sim_host.shell_owners()
	var ids := sim_host.shell_ids()
	var left := sim_host.shell_seconds_left()
	var calibers := sim_host.shell_calibers()
	var me := sim_host.local_player()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	_update_look()
	_grow(_shells, positions.size(), _shell_material)
	_grow(_trails, positions.size() if _trails_on else 0, _trail_material)
	# The streak covers one frame of flight at the game's speed.
	var frame_s := clampf(delta, STREAK_FRAME_MIN_S, STREAK_FRAME_MAX_S) * clampf(sim_host.speed, 0.0, 20.0)
	var live: Dictionary[int, bool] = {}
	var markers: Array[Array] = []
	var shown_now: Dictionary[int, Array] = {}
	var effects := main.get("effects") as Effects if main != null else null
	var rockets: Node3D = effects.rockets if effects != null else null
	if rockets != null:
		rockets.begin_frame()
	for i in _trails.size():
		_trails[i].visible = false
	for i in _shells.size():
		var shown := i < positions.size()
		_shells[i].visible = shown
		if not shown:
			continue
		var caliber := absf(calibers[i])
		var smoke := calibers[i] < 0.0
		var velocity := velocities[i]
		var at := positions[i] + velocity * ahead
		if rockets != null and rockets.is_rocket(ids[i]):
			_shells[i].visible = false
			at = rockets.flight(ids[i], at, velocity)
		elif owners[i] != me and effects != null and not effects.visible_at.call(at):
			# Enemy shells only where the local player sees (prompt 098).
			_shells[i].visible = false
		if not _first_left.has(ids[i]):
			_first_left[ids[i]] = left[i]
		if _shells[i].visible:
			var width := caliber * SHELL_WIDTH_PER_MM
			_shells[i].transform = LightStreak.transform(at, at - velocity * frame_s, width)
			_shells[i].set_instance_shader_parameter("glow",
					shell_glow(ids[i], _first_left[ids[i]] - left[i], left[i] - ahead))
			if i < _trails.size():
				_trails[i].visible = true
				_trails[i].transform = LightStreak.transform(at, at - velocity * TRAIL_SECONDS, width)
		if owners[i] == me and markers_enabled:
			var alpha := marker_alpha(left[i] - ahead)
			if alpha > 0.0:
				var marker := [targets[i], _marker_radius(caliber), smoke, alpha]
				markers.append(marker)
				shown_now[ids[i]] = marker
		live[ids[i]] = true
		if left[i] <= WHISTLE_SECONDS and not _whistled.has(ids[i]):
			_whistled[ids[i]] = true
			incoming += 1
			shell_incoming.emit(targets[i], caliber, smoke)
	for id in _whistled.keys():
		if not live.has(id):
			_whistled.erase(id)
	for id in _first_left.keys():
		if not live.has(id):
			_first_left.erase(id)
	if rockets != null:
		rockets.end_frame()
	shells_drawn = positions.size()
	_draw_markers(markers, shown_now, delta)


## Opacity factor of the marker of a shell landing in `seconds_left`: 0
## before MARKER_LEAD_SECONDS, then rising to 1 over MARKER_FADE_IN.
static func marker_alpha(seconds_left: float) -> float:
	if seconds_left > MARKER_LEAD_SECONDS:
		return 0.0
	return clampf((MARKER_LEAD_SECONDS - seconds_left) / MARKER_FADE_IN, 0.0, 1.0)


## Markers (`[target, radius, smoke, alpha]`) with near ones of the same
## kind drawn as one: the first keeps its place, the larger radius and the
## highest alpha.
static func merge_markers(markers: Array[Array]) -> Array[Array]:
	var kept: Array[Array] = []
	for m in markers:
		var merged := false
		for k in kept:
			if k[2] == m[2] and (k[0] as Vector3).distance_to(m[0]) < MARKER_MERGE_SHARE * maxf(k[1], m[1]):
				k[3] = maxf(k[3], m[3])
				k[1] = maxf(k[1], m[1])
				merged = true
				break
		if not merged:
			kept.append(m.duplicate())
	return kept


func _marker_radius(caliber_mm: float) -> float:
	var key := roundi(caliber_mm * 10.0)
	if not _blast_radii.has(key):
		_blast_radii[key] = sim_host.shell_blast_radius(caliber_mm)
	var old := maxf(2.0, caliber_mm * 0.1)
	var core := _blast_radii[key] * MARKER_CORE_SHARE
	return clampf(core, 1.5, old) if core > 0.0 else old


func _marker_mesh(radius: float) -> TorusMesh:
	var key := maxi(1, roundi(radius * 2.0))
	if not _marker_meshes.has(key):
		var mesh := TorusMesh.new()
		mesh.outer_radius = key * 0.5
		mesh.inner_radius = key * 0.5 - MARKER_LINE
		mesh.rings = 48
		mesh.ring_segments = 3
		_marker_meshes[key] = mesh
	return _marker_meshes[key]


## Draws this frame's markers plus those fading out after their impact
## (the fade follows game time: it stands while the match is paused).
func _draw_markers(markers: Array[Array], shown_now: Dictionary[int, Array], delta: float) -> void:
	for id in _shown_markers.keys():
		if not shown_now.has(id) and markers_enabled:
			_fading_markers.append(_shown_markers[id].duplicate())
	_shown_markers = shown_now
	var step := delta * clampf(sim_host.speed, 0.0, 20.0) / MARKER_FADE_OUT
	for i in range(_fading_markers.size() - 1, -1, -1):
		_fading_markers[i][3] -= step
		if _fading_markers[i][3] <= 0.0 or not markers_enabled:
			_fading_markers.remove_at(i)
	var all: Array[Array] = []
	if markers_enabled:
		all.append_array(markers)
	all.append_array(_fading_markers)
	var drawn := merge_markers(all)
	while _markers.size() < drawn.size():
		var node := MeshInstance3D.new()
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		_markers.append(node)
	for i in _markers.size():
		var node := _markers[i]
		node.visible = i < drawn.size()
		if not node.visible:
			continue
		var m: Array = drawn[i]
		node.mesh = _marker_mesh(m[1])
		node.material_override = _marker_smoke_material if m[2] else _marker_material
		node.transparency = 1.0 - clampf(m[3], 0.0, 1.0)
		# Flat on the ground, just above it so short grass does not hide it.
		node.transform = Transform3D(Basis.IDENTITY.scaled(Vector3(1.0, 0.25, 1.0)), (m[0] as Vector3) + Vector3.UP * 0.25)
	markers_drawn = drawn.size()


func _grow(pool: Array[MeshInstance3D], count: int, material: Material) -> void:
	while pool.size() < count:
		var node := LightStreak.node(_streak_mesh, material)
		add_child(node)
		pool.append(node)


## Every cloud as a few billboard puffs that drift with it and fade out at
## the end of its life.
func _draw_smoke() -> void:
	var clouds := sim_host.smoke_clouds()
	var ids := sim_host.smoke_ids()
	var life := sim_host.smoke_life()
	var dark := sim_host.smoke_dark()
	var live: Dictionary[int, bool] = {}
	for i in clouds.size():
		var id := ids[i]
		live[id] = true
		if not _clouds.has(id):
			_clouds[id] = _make_cloud(id, i < dark.size() and dark[i] != 0)
		var c := clouds[i]
		var center := Vector3(c.x, c.y, c.z)
		var radius := c.w
		var fade := clampf(life[i] * 4.0, 0.0, 1.0)
		for puff: Array in _clouds[id]:
			var node: MeshInstance3D = puff[0]
			var offset: Vector3 = puff[1]
			node.position = center + Vector3(offset.x * radius, (0.35 + offset.y) * radius, offset.z * radius)
			node.scale = Vector3.ONE * radius * (1.1 + 0.3 * offset.y)
			node.transparency = 1.0 - SMOKE_OPACITY * fade
	for id in _clouds.keys():
		if not live.has(id):
			for puff: Array in _clouds[id]:
				(puff[0] as Node).queue_free()
			_clouds.erase(id)
	clouds_drawn = clouds.size()


func _make_cloud(id: int, is_dark := false) -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = id * 7919 + 17
	var puffs: Array = []
	for k in PUFFS:
		var node := MeshInstance3D.new()
		node.mesh = _dark_puff_mesh if is_dark else _puff_mesh
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		var angle := rng.randf() * TAU
		var distance := sqrt(rng.randf()) * 0.6 if k > 0 else 0.0
		puffs.append([node, Vector3(cos(angle) * distance, rng.randf() * 0.4, sin(angle) * distance)])
	return puffs
