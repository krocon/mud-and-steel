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
const EffectLibrary := preload("res://scenes/world/effect_library.gd")
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
## Incoming and full-arc artillery/rocket puff trails (prompts 220 & 225):
## dotted chains of bright light-grey smoke puffs along indirect shell arcs.
## Heavy indirect shells (>= PUFF_FULL_ARC_CALIBER) and rockets trail puffs
## from launch all the way through the arc into impact; lighter shells (75-99 mm)
## show puffs only on steep final descent. Deterministic from shell id and
## flight time; off at low quality (`fx_shell_trails`) and hidden in fog of war.
## [provisional]
const PUFF_MIN_CALIBER := 75.0
const PUFF_FULL_ARC_CALIBER := 100.0
const PUFF_WINDOW_S := 2.4
const PUFF_MIN_DESCENT_MPS := 4.0
const PUFF_SPACING_M := 4.6
const PUFF_ARC_SPACING_M := 5.0
const PUFF_STEP_MIN_S := 0.035
const PUFF_STEP_MAX_S := 0.105
const PUFF_CHAIN_COUNT := 14.0
const PUFF_ARC_LIFE_MIN_S := 2.8
const PUFF_ARC_LIFE_MAX_S := 4.4
const PUFF_MAX_INSTANCES := 2048
const PUFF_COLOR := Color(0.95, 0.94, 0.91)
const PUFF_SKY_LIFT := {"day": 0.28, "morning": 0.12, "evening": 0.12, "night": 0.0}
const PUFF_DAY_ALPHA := {"day": 0.92, "morning": 0.76, "evening": 0.72, "night": 0.22}
const PUFF_WEATHER_ALPHA := {"rain": 0.72, "snow": 0.72, "fog": 0.48}
const INCOMING_PUFF_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled;
uniform sampler2D tex : source_color, filter_linear_mipmap, repeat_disable;
uniform float sky_lift = 0.22;
uniform float alpha_scale = 0.86;
varying float v_warm;
varying vec2 v_uv;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float size = length(MODEL_MATRIX[0].xyz);
	float a = INSTANCE_CUSTOM.x;
	float ca = cos(a);
	float sa = sin(a);
	vec2 q = vec2(VERTEX.x * ca - VERTEX.y * sa, VERTEX.x * sa + VERTEX.y * ca) * size;
	vec3 world = origin + INV_VIEW_MATRIX[0].xyz * q.x + INV_VIEW_MATRIX[1].xyz * q.y;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	v_warm = INSTANCE_CUSTOM.y;
	v_uv = UV;
}

void fragment() {
	vec4 t = texture(tex, v_uv);
	vec2 p = v_uv * 2.0 - 1.0;
	NORMAL = normalize(vec3(p.x * 0.45, -p.y * 0.45 + 0.35, 0.9));
	vec3 col = t.rgb * COLOR.rgb;
	ALBEDO = col;
	EMISSION = col * sky_lift + vec3(1.0, 0.56, 0.20) * v_warm;
	ALPHA = clamp(t.a * COLOR.a * alpha_scale, 0.0, 1.0);
}
"""
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
## Puffs per cloud and their look. Prompt 200: at medium and high quality
## (`fx_clouds`) the puffs are soft cloud puffs lit as one body
## (`effect_library.gd` "cloud"), else the plain smoke texture.
const PUFFS := 9
const SMOKE_COLOR := Color(0.78, 0.78, 0.76)
## The same smoke drawn as a lit cloud (prompt 200): a little darker, so
## the sun side does not burn out to white.
const CLOUD_SMOKE_COLOR := Color(0.66, 0.66, 0.65)
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
## Prompt 200: the cloud puff (one material; color and opacity per puff).
var _cloud_mesh: QuadMesh
var _clouds_on := true
## Cloud id -> its puffs (MeshInstance3D) and their offsets (x, y, z in
## units of the radius).
var _clouds: Dictionary[int, Array] = {}
var _whistled: Dictionary[int, bool] = {}
## Prompt 220: single MultiMesh for all incoming artillery/rocket puff trails.
var _incoming_puffs: MultiMeshInstance3D
var _incoming_puff_shader: Shader
var _incoming_puff_material: ShaderMaterial
## Shell id -> {next_flown, puffs ([{pos, born, life, size, angle, warm}]), owner}.
var _puff_trails: Dictionary[int, Dictionary] = {}
var _sim_time := 0.0
var _puff_count := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_streak_mesh = LightStreak.mesh()
	_shell_material = LightStreak.material(SHELL_COLOR, SHELL_ENERGY.day, SHELL_MIN_PX, SHELL_MAX_PX, 0.25)
	_trail_material = LightStreak.material(TRAIL_COLOR, TRAIL_ENERGY.day, TRAIL_MIN_PX, TRAIL_MAX_PX, 0.0)
	_setup_incoming_puffs()
	_marker_material = _marker_material_of(RelationColors.own_or(MARKER_COLOR))
	_marker_smoke_material = _marker_material_of(MARKER_SMOKE_COLOR)
	_puff_mesh = QuadMesh.new()
	_puff_mesh.size = Vector2(1, 1)
	var puff := StandardMaterial3D.new()
	puff.albedo_texture = EffectTextures.smoke()
	puff.albedo_color = SMOKE_COLOR
	puff.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	puff.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	# Keep the node's scale: without it every puff was 1 m (prompt 200).
	puff.billboard_keep_scale = true
	puff.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	puff.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	_puff_mesh.material = puff
	_dark_puff_mesh = QuadMesh.new()
	_dark_puff_mesh.size = Vector2(1, 1)
	var dark := puff.duplicate() as StandardMaterial3D
	dark.albedo_color = FIRE_SMOKE_COLOR
	_dark_puff_mesh.material = dark
	_cloud_mesh = QuadMesh.new()
	_cloud_mesh.size = Vector2(1, 1)
	_cloud_mesh.material = EffectLibrary._material("cloud", EffectTextures.cloud(), 0.0, true)


func _setup_incoming_puffs() -> void:
	_incoming_puff_shader = Shader.new()
	_incoming_puff_shader.code = INCOMING_PUFF_SHADER
	_incoming_puff_material = ShaderMaterial.new()
	_incoming_puff_material.shader = _incoming_puff_shader
	_incoming_puff_material.set_shader_parameter("tex", EffectTextures.smoke())
	_incoming_puff_material.set_shader_parameter("sky_lift", PUFF_SKY_LIFT.day)
	_incoming_puff_material.set_shader_parameter("alpha_scale", PUFF_DAY_ALPHA.day)
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	quad.material = _incoming_puff_material
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.use_custom_data = true
	mm.mesh = quad
	mm.instance_count = PUFF_MAX_INSTANCES
	mm.visible_instance_count = 0
	_incoming_puffs = MultiMeshInstance3D.new()
	_incoming_puffs.name = "IncomingPuffs"
	_incoming_puffs.multimesh = mm
	_incoming_puffs.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_incoming_puffs.custom_aabb = AABB(Vector3(-1e5, -1e4, -1e5), Vector3(2e5, 2e4, 2e5))
	add_child(_incoming_puffs)


func _exit_tree() -> void:
	_incoming_puff_shader = null


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
	if _incoming_puff_material != null:
		_incoming_puff_material.set_shader_parameter("sky_lift", float(PUFF_SKY_LIFT.get(daytime, PUFF_SKY_LIFT.day)))
		_incoming_puff_material.set_shader_parameter("alpha_scale",
				float(PUFF_DAY_ALPHA.get(daytime, PUFF_DAY_ALPHA.day)) * float(PUFF_WEATHER_ALPHA.get(weather, 1.0)))
	var settings := GraphicsQuality.settings(quality)
	_trails_on = bool(settings.get("fx_shell_trails", true))
	if not _trails_on and _incoming_puffs != null:
		_puff_trails.clear()
		_incoming_puffs.multimesh.visible_instance_count = 0
	var clouds := bool(settings.get("fx_clouds", true))
	if clouds != _clouds_on:
		_clouds_on = clouds
		# Built again with the other look.
		for id in _clouds.keys():
			for puff: Array in _clouds[id]:
				(puff[0] as Node).queue_free()
		_clouds.clear()


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
	var speed_scale := clampf(sim_host.speed, 0.0, 20.0)
	var frame_s := clampf(delta, STREAK_FRAME_MIN_S, STREAK_FRAME_MAX_S) * speed_scale
	_sim_time += delta * speed_scale
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
		var is_rocket: bool = rockets != null and rockets.is_rocket(ids[i])
		var visible_shell: bool = owners[i] == me or effects == null or effects.visible_at.call(at)
		if is_rocket:
			_shells[i].visible = false
			at = rockets.flight(ids[i], at, velocity)
		elif not visible_shell:
			# Enemy shells only where the local player sees (prompt 098).
			_shells[i].visible = false
		if not _first_left.has(ids[i]):
			_first_left[ids[i]] = left[i]
		var left_now := maxf(left[i] - ahead, 0.0)
		var flown := maxf(_first_left[ids[i]] - left_now, 0.0)
		if _shells[i].visible:
			var width := caliber * SHELL_WIDTH_PER_MM
			_shells[i].transform = LightStreak.transform(at, at - velocity * frame_s, width)
			_shells[i].set_instance_shader_parameter("glow", shell_glow(ids[i], flown, left_now))
			if i < _trails.size():
				_trails[i].visible = true
				_trails[i].transform = LightStreak.transform(at, at - velocity * TRAIL_SECONDS, width)
		if _trails_on and visible_shell and not smoke and (caliber >= PUFF_MIN_CALIBER or is_rocket):
			var full_arc: bool = caliber >= PUFF_FULL_ARC_CALIBER or is_rocket
			if full_arc or (velocity.y <= -PUFF_MIN_DESCENT_MPS and left_now <= PUFF_WINDOW_S):
				_update_shell_puffs(ids[i], at, velocity, caliber, flown, left_now, owners[i], full_arc)
		if owners[i] == me and markers_enabled:
			var alpha := marker_alpha(left_now)
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
	if _trails_on:
		_render_incoming_puffs(effects, me)
	shells_drawn = positions.size()
	_draw_markers(markers, shown_now, delta)


## Deterministic hash in [0, 1) for incoming puff variation (presentation only).
static func _puff_hash(id: int, index: int) -> float:
	var h := ((id * 2654435761) ^ (index * 2246822519)) & 0xffffffff
	return float(h) / 4294967296.0


## Records world-space smoke puffs behind an indirect shell/rocket (prompt 220
## final descent, prompt 225 full arc from launch for heavy shells >= 100 mm).
func _update_shell_puffs(id: int, at: Vector3, velocity: Vector3, caliber: float, flown: float,
		left_now: float, owner: int, full_arc: bool) -> void:
	var speed := maxf(velocity.length(), 1.0)
	var spacing := PUFF_ARC_SPACING_M if full_arc else PUFF_SPACING_M
	var step_s := clampf(spacing / speed, PUFF_STEP_MIN_S, PUFF_STEP_MAX_S)
	var life_s := clampf(step_s * 36.0, PUFF_ARC_LIFE_MIN_S, PUFF_ARC_LIFE_MAX_S) if full_arc else step_s * PUFF_CHAIN_COUNT
	var cal_scale := clampf(caliber / 105.0, 0.85, 1.38)
	if not _puff_trails.has(id):
		var puffs: Array[Dictionary] = []
		var phase := _puff_hash(id, 0) * step_s
		var back_limit := minf(flown, life_s * 0.9) if full_arc else minf(maxf(flown, 0.45), life_s * 0.85)
		var k := int(floor(back_limit / step_s))
		# Pre-seed backward along the arc so stills/fast-forward frames show complete trails.
		for j in range(k, 0, -1):
			var back_t := float(j) * step_s + phase * 0.5
			if back_t > back_limit:
				continue
			# Approximate game gravity (~4.9 m/s^2) along the recent arc segment.
			var p := at - velocity * back_t + Vector3(0.0, 2.45 * back_t * back_t, 0.0)
			var left_then := left_now + back_t
			var flown_then := maxf(flown - back_t, 0.0)
			var launch_k := clampf(1.0 - flown_then / 0.65, 0.0, 1.0) if full_arc else 0.0
			var descent_t := clampf(1.0 - left_then / PUFF_WINDOW_S, 0.0, 1.0)
			var base_size := lerpf(1.65, 2.35, maxf(descent_t, launch_k * 0.8)) if full_arc else lerpf(1.05, 1.75, descent_t)
			puffs.append({
				"pos": p, "born": _sim_time - back_t, "life": life_s * lerpf(1.0, 1.18, launch_k),
				"size": base_size * cal_scale,
				"angle": _puff_hash(id, j + 17) * TAU,
				"warm": maxf(clampf(1.0 - left_then / 0.45, 0.0, 1.0) * 0.55, launch_k * 0.25),
			})
		_puff_trails[id] = {"next_flown": flown, "puffs": puffs, "seq": k + 1, "owner": owner}
	var trail: Dictionary = _puff_trails[id]
	var puffs: Array[Dictionary] = trail.puffs
	var spawned := 0
	while flown >= float(trail.next_flown) and spawned < 10:
		var lag := clampf(flown - float(trail.next_flown), 0.0, life_s)
		var p := at - velocity * lag + Vector3(0.0, 2.45 * lag * lag, 0.0)
		var left_then := left_now + lag
		var flown_then := maxf(flown - lag, 0.0)
		var launch_k := clampf(1.0 - flown_then / 0.65, 0.0, 1.0) if full_arc else 0.0
		var descent_t := clampf(1.0 - left_then / PUFF_WINDOW_S, 0.0, 1.0)
		var base_size := lerpf(1.65, 2.35, maxf(descent_t, launch_k * 0.8)) if full_arc else lerpf(1.05, 1.75, descent_t)
		var seq := int(trail.seq)
		trail.seq = seq + 1
		puffs.append({
			"pos": p, "born": _sim_time - lag, "life": life_s * lerpf(1.0, 1.18, launch_k),
			"size": base_size * cal_scale,
			"angle": _puff_hash(id, seq) * TAU,
			"warm": maxf(clampf(1.0 - left_then / 0.45, 0.0, 1.0) * 0.55, launch_k * 0.25),
		})
		# Slightly denser bead spacing right out of the barrel (launch zone).
		var eff_step := step_s * lerpf(1.0, 0.65, launch_k)
		trail.next_flown = float(trail.next_flown) + eff_step
		spawned += 1
	if flown - float(trail.next_flown) > life_s:
		trail.next_flown = flown + step_s


## Uploads all living incoming smoke puffs to the shared MultiMesh (1 draw call).
func _render_incoming_puffs(effects: Effects, me: int) -> void:
	if _incoming_puffs == null:
		return
	var mm := _incoming_puffs.multimesh
	var wind: Vector2 = effects._wind() * 0.35 if effects != null else Vector2.ZERO
	var count := 0
	for id: int in _puff_trails.keys():
		var trail: Dictionary = _puff_trails[id]
		var puffs: Array[Dictionary] = trail.puffs
		var enemy: bool = int(trail.owner) != me
		for j in range(puffs.size() - 1, -1, -1):
			var puff: Dictionary = puffs[j]
			var age := _sim_time - float(puff.born)
			var life := float(puff.life)
			if age >= life or age < -0.1:
				puffs.remove_at(j)
				continue
			if count >= PUFF_MAX_INSTANCES:
				continue
			var pos: Vector3 = puff.pos + Vector3(wind.x * age, 0.25 * age, wind.y * age)
			if enemy and effects != null and not effects.visible_at.call(pos):
				continue
			var t := clampf(age / life, 0.0, 1.0)
			# Crisp fade-in right behind the shell, gentle dissolve as it lingers.
			var alpha := smoothstep(0.0, 0.08, t) * pow(1.0 - t, 1.35)
			var size := float(puff.size) * lerpf(0.85, 1.45, t)
			var warm := float(puff.warm) * (1.0 - t)
			var tint := PUFF_COLOR.lerp(Color(1.0, 0.84, 0.62), warm * 0.6)
			mm.set_instance_transform(count, Transform3D(Basis.IDENTITY.scaled(Vector3.ONE * size), pos))
			mm.set_instance_color(count, Color(tint.r, tint.g, tint.b, alpha))
			mm.set_instance_custom_data(count, Color(float(puff.angle), warm, 0.0, 0.0))
			count += 1
		if puffs.is_empty():
			_puff_trails.erase(id)
	mm.visible_instance_count = count
	_puff_count = count


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
		var is_dark := i < dark.size() and dark[i] != 0
		var body := center + Vector3.UP * 0.35 * radius
		for puff: Array in _clouds[id]:
			var node: MeshInstance3D = puff[0]
			var offset: Vector3 = puff[1]
			node.position = center + Vector3(offset.x * radius, (0.35 + offset.y) * radius, offset.z * radius)
			if _clouds_on:
				# Bigger, overlapping puffs; colour and opacity per puff, lit
				# as one cloud around its body.
				node.scale = Vector3.ONE * radius * (1.5 + 0.3 * offset.y)
				var color := FIRE_SMOKE_COLOR if is_dark else CLOUD_SMOKE_COLOR
				node.set_instance_shader_parameter(&"tint", Color(color, SMOKE_OPACITY * fade))
				node.set_instance_shader_parameter(&"cloud_center", Vector4(body.x, body.y, body.z, radius))
			else:
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
		node.mesh = _cloud_mesh if _clouds_on else (_dark_puff_mesh if is_dark else _puff_mesh)
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		var angle := rng.randf() * TAU
		var distance := sqrt(rng.randf()) * 0.6 if k > 0 else 0.0
		puffs.append([node, Vector3(cos(angle) * distance, rng.randf() * 0.4, sin(angle) * distance)])
	return puffs
