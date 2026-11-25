extends Node3D
## Rocket salvo (prompt 076): launch, flame and smoke trail of rockets, for
## the heavy strike's battery position (prompt 054) and the vehicle rocket
## launchers (prompt 062). Owned by `effects.gd` (`effects.rockets`).
##
## Two calls make a shell a rocket:
##
## - `note(projectile, at, station)`: the shell `projectile` is a rocket
##   that leaves the rail at `at`; `station` groups the launches of one
##   launcher (its unit id) for the light and the haze. `effects.gd` calls it
##   for every indirect shot of a unit whose model has `Socket_Rocket_*`
##   sockets, one socket after the other.
## - `flight(id, at, velocity)`: `artillery_view.gd` hands over every noted
##   shell each frame instead of drawing it as a ball. The first call fires
##   `launch()` in the shell's direction.
##
## `launch(at, direction, caliber, station)` alone gives the launch effects
## at any point (flare, back blast, thrown-up ground, the station's light).
##
## Layers: a slim body; while the motor burns an elongated additive flame
## and a halo at its tail (MultiMesh, three draw calls for all rockets); a
## smoke trail per rocket: a camera-facing ribbon through points left in the
## world while the motor burns (no puffs, so it cannot tear at any speed),
## widened, faded and drifted with the wind in the shader by each point's
## age, warm and bright near the flame (emission), else lit by sun and
## ambient light (dark at night). At the launcher: a flare that collapses
## between two launches, back blast smoke, ground dust in the ground colour,
## one flickering light per position and haze over it that drifts off.
##
## Presentation only: the rockets fly and land where the simulation says.
## The drawn rocket starts at its rail socket and blends onto the
## simulation's path within BLEND_S (a fading offset, the path's end is
## untouched). Godot randomness never reaches the simulation.

const BlastEffects := preload("res://scenes/world/blast_effects.gd")
const EffectLibrary := preload("res://scenes/world/effect_library.gd")

## Motor burn time in game seconds. The M-13 burns 0.85 s (missilery.info,
## docs/SOURCES.md), 0.43 s at time factor 2; judged: 1.0 s, so four to five
## flames per salvo are in the air at once like in the reference pictures
## (with 0.43 s there were only two and the trails stayed short).
const BURN_S := 1.0
## Body: the M-13 is 1.41 m long and 132 mm thick (missilery.info); drawn
## at its real size: the reference shows the flame, hardly the rocket.
const BODY_LENGTH_M := 1.41
const BODY_RADIUS_M := 0.066
const BODY_COLOR := Color(0.16, 0.16, 0.14)
## Flame at the tail (reference: a drop "some meters" long, white-yellow
## core, orange rim, soft halo). Judged.
const FLAME_LENGTH_M := 5.0
const FLAME_WIDTH_M := 1.5
const FLAME_ENERGY := 5.0
const HALO_M := 3.5
const HALO_ENERGY := 1.1
## Reference caliber of the sizes: the M-13 (132 mm); other rockets scale
## with their caliber (Wurfgranate 41: 150 mm, M8: 114 mm, data/strike.toml).
const CALIBER_MM := 132.0
## The drawn rocket blends from its socket onto the simulated path (s).
const BLEND_S := 0.35
## Trail: points at least this far apart (m), at most this many per trail.
const TRAIL_SPACING_M := 5.0
const TRAIL_POINTS := 96
## Width when it leaves the flame and growth per second of age (m, m/s),
## opacity, colour; rise (m/s) and share of the biome's wind it drifts with.
## Judged from the reference: about a launcher wide after two seconds.
const TRAIL_WIDTH_M := 0.6
const TRAIL_GROWTH_M := 1.0
const TRAIL_ALPHA := 0.6
const TRAIL_COLOR := Color(0.93, 0.92, 0.9)
const TRAIL_RISE := 0.25
const TRAIL_WIND := 0.7
## The trail glows warm this far behind a burning flame (m).
const TRAIL_HEAT_M := 12.0
const TRAIL_HEAT_COLOR := Color(1.0, 0.62, 0.3)
## Launch flare at the rail's rear end: as big as the launcher, overexposed
## (reference), collapsing with this time constant so it drops to a few
## percent between two launches 0.25 s apart (readability).
const FLARE_M := 10.0
const FLARE_DECAY_S := 0.085
const FLARE_STRENGTH := 2.4
## Flares at once (two salvos, a few launchers each).
const FLARES := 12
## Light of a firing position: lights the ground 10-15 m around (reference),
## stronger at dusk and at night (BlastEffects.DAYTIME_*).
const LIGHT_COLOR := Color(1.0, 0.7, 0.4)
const LIGHT_ENERGY := 6.0
const LIGHT_RANGE_M := 14.0
## Seconds the haze keeps rising after the last launch.
const HAZE_LINGER_S := 3.0

const FLAME_SHADER := """
shader_type spatial;
render_mode blend_add, unshaded, depth_draw_never, cull_disabled, shadows_disabled;
uniform float energy = 7.0;
varying float v_strength;

void vertex() {
	// Axial billboard: the quad lies along the instance's Y axis (the
	// flame, length = its scale) and turns around it to face the camera.
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec3 axis = MODEL_MATRIX[1].xyz;
	float width = length(MODEL_MATRIX[0].xyz);
	vec3 side = cross(axis, INV_VIEW_MATRIX[3].xyz - origin);
	float l = length(side);
	side = l > 0.0001 ? side / l : INV_VIEW_MATRIX[0].xyz;
	vec3 world = origin + axis * (VERTEX.y + 0.5) + side * VERTEX.x * width;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	v_strength = COLOR.a;
}

void fragment() {
	// v: 0 at the rocket's tail, 1 at the flame's end (the quad's top);
	// a drop that is widest near the tail.
	float v = 1.0 - UV.y;
	float x = abs(UV.x * 2.0 - 1.0);
	float r = v < 0.16 ? sqrt(v / 0.16) : pow(max(1.0 - (v - 0.16) / 0.84, 0.0), 0.9);
	float d = x / max(r, 0.001);
	float body = 1.0 - smoothstep(0.3, 1.0, d);
	float core = (1.0 - smoothstep(0.0, 0.55, d)) * (1.0 - smoothstep(0.05, 0.55, v));
	vec3 rim = mix(vec3(1.0, 0.36, 0.06), vec3(1.0, 0.72, 0.3), body);
	ALBEDO = (rim * body + vec3(1.0, 0.93, 0.72) * core * 1.6) * energy * v_strength;
	ALPHA = clamp(body + core, 0.0, 1.0);
}
"""
const HALO_SHADER := """
shader_type spatial;
render_mode blend_add, unshaded, depth_draw_never, cull_disabled, shadows_disabled;
uniform float energy = 1.6;
varying float v_strength;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float size = length(MODEL_MATRIX[0].xyz);
	vec3 world = origin + (INV_VIEW_MATRIX[0].xyz * VERTEX.x + INV_VIEW_MATRIX[1].xyz * VERTEX.y) * size;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	v_strength = COLOR.a;
}

void fragment() {
	float d = length(UV * 2.0 - 1.0);
	float glow = pow(max(1.0 - d, 0.0), 2.4);
	float hot = pow(max(1.0 - d * 2.2, 0.0), 2.0);
	ALBEDO = (vec3(1.0, 0.48, 0.14) * glow + vec3(1.0, 0.88, 0.62) * hot * 2.5) * energy * v_strength;
	ALPHA = clamp(glow + hot, 0.0, 1.0);
}
"""
## Trail ribbon: VERTEX = the point where it was left, NORMAL = the trail's
## direction there, UV = (side -1/+1, distance from the start in m), UV2.x =
## its birth in s after the trail's start.
const TRAIL_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled;
uniform vec3 color : source_color = vec3(0.88, 0.86, 0.82);
uniform vec3 heat_color : source_color = vec3(1.0, 0.62, 0.3);
uniform float alpha = 0.6;
uniform float width = 0.6;
uniform float growth = 1.0;
uniform float life = 9.0;
uniform float heat_m = 22.0;
uniform vec3 drift = vec3(0.0, 0.25, 0.0);
// Seconds since the trail's start, its length (m) and how much its head
// still burns (0..1); per trail.
instance uniform float age0 = 0.0;
instance uniform float head = 0.0;
instance uniform float heat = 0.0;
instance uniform float seed = 0.0;
varying float v_age;
varying float v_side;
varying float v_dist;
varying float v_heat;
varying vec3 v_normal;

void vertex() {
	float age = max(age0 - UV2.x, 0.0);
	vec3 p = VERTEX + drift * age;
	vec3 tangent = normalize(NORMAL);
	vec3 to_cam = normalize(INV_VIEW_MATRIX[3].xyz - p);
	vec3 side = cross(tangent, to_cam);
	float l = length(side);
	side = l > 0.0001 ? side / l : INV_VIEW_MATRIX[0].xyz;
	float behind = max(head - UV.y, 0.0);
	// Thin where it leaves the flame, wider with age.
	float w = (width + growth * age) * mix(0.3, 1.0, smoothstep(0.0, 8.0, behind));
	VERTEX = p + side * UV.x * w * 0.5;
	// A rounded fake normal across the ribbon (a tube of smoke), tilted up
	// to the sky; set in fragment() so the back side is not flipped dark.
	v_normal = (VIEW_MATRIX * vec4(normalize(to_cam + side * UV.x * 0.6 + vec3(0.0, 0.5, 0.0)), 0.0)).xyz;
	v_age = age;
	v_side = UV.x;
	v_dist = UV.y;
	v_heat = heat * exp(-behind / heat_m);
}

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	float across = 1.0 - abs(v_side);
	float n = noise(vec2(v_dist * 0.12 + seed, v_side * 1.3 + v_age * 0.25)) * 0.6
		+ noise(vec2(v_dist * 0.35 - seed, v_side * 2.7 - v_age * 0.4)) * 0.4;
	float fade = 1.0 - smoothstep(life * 0.35, life, v_age);
	// Soft, ragged edges: the noise eats into the rim.
	float a = alpha * smoothstep(0.0, 1.0, across * (0.55 + 0.9 * n)) * mix(0.5, 1.0, n) * fade;
	// Fresh smoke just behind the flame is denser.
	a = min(a * (1.0 + 0.6 * v_heat), 1.0);
	NORMAL = normalize(v_normal);
	ALBEDO = color * mix(0.85, 1.0, n);
	EMISSION = heat_color * v_heat * 1.4;
	ALPHA = a;
}
"""

var effects: Node3D
## Off: rockets are drawn as plain shells with a muzzle flash, as before
## prompt 076 (the effects bench measures both).
var enabled := true
## Rockets started since the start (self-test, tools).
var launched := 0
## Rockets with a burning motor in the last frame (tools).
var burning := 0

var _time := 0.0
## Projectile id -> {t, from, station, caliber, offset, seen, trail, last}.
var _rockets: Dictionary[int, Dictionary] = {}
## Station key -> {at, dir, pulse, last, light, haze}.
var _stations: Dictionary[int, Dictionary] = {}
var _bodies: MultiMeshInstance3D
var _flames: MultiMeshInstance3D
var _halos: MultiMeshInstance3D
var _trail_material: ShaderMaterial
## Trail pool: {node, mesh, t0, points, births, dists, live, burn, seed}.
var _trails: Array[Dictionary] = []
var _seen_frame: Dictionary[int, bool] = {}
## Launch flares: {at, pulse}.
var _flares: Array[Dictionary] = []
var _shaders: Array[Shader] = []


func setup(owner_effects: Node3D) -> void:
	effects = owner_effects
	name = "Rockets"
	var body := CylinderMesh.new()
	body.top_radius = BODY_RADIUS_M * 0.7
	body.bottom_radius = BODY_RADIUS_M
	body.height = BODY_LENGTH_M
	body.radial_segments = 6
	body.rings = 1
	var body_material := StandardMaterial3D.new()
	body_material.albedo_color = BODY_COLOR
	body_material.roughness = 0.6
	body.material = body_material
	_bodies = _multimesh("Bodies", body, false)
	_flames = _multimesh("Flames", _quad(_shader_material(FLAME_SHADER, FLAME_ENERGY)), true)
	_halos = _multimesh("Halos", _quad(_shader_material(HALO_SHADER, HALO_ENERGY)), true)
	var trail_shader := Shader.new()
	trail_shader.code = TRAIL_SHADER
	_shaders.append(trail_shader)
	_trail_material = ShaderMaterial.new()
	_trail_material.shader = trail_shader
	_trail_material.set_shader_parameter("color", TRAIL_COLOR)
	_trail_material.set_shader_parameter("heat_color", TRAIL_HEAT_COLOR)
	_trail_material.set_shader_parameter("alpha", TRAIL_ALPHA)
	_trail_material.set_shader_parameter("growth", TRAIL_GROWTH_M)
	_trail_material.set_shader_parameter("heat_m", TRAIL_HEAT_M)


## Drops the shaders (else they leak at exit, like EffectLibrary's).
func _exit_tree() -> void:
	_shaders.clear()


func _multimesh(node_name: String, mesh: Mesh, colors: bool) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = colors
	mm.mesh = mesh
	mm.instance_count = 160
	mm.visible_instance_count = 0
	var node := MultiMeshInstance3D.new()
	node.name = node_name
	node.multimesh = mm
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# The billboards are placed in the shader: never cull the whole set.
	node.custom_aabb = AABB(Vector3(-1e5, -1e4, -1e5), Vector3(2e5, 2e4, 2e5))
	add_child(node)
	return node


func _quad(material: Material) -> QuadMesh:
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	quad.material = material
	return quad


func _shader_material(code: String, energy: float) -> ShaderMaterial:
	var shader := Shader.new()
	shader.code = code
	_shaders.append(shader)
	var material := ShaderMaterial.new()
	material.shader = shader
	material.set_shader_parameter("energy", energy)
	return material


# --- Calls ----------------------------------------------------------------------


## The shell `projectile` is a rocket leaving the rail at `at`; `station`
## names its launcher (unit id). The launch effects follow with its first
## `flight()`, in its direction.
func note(projectile: int, at: Vector3, station: int, caliber: float) -> void:
	_rockets[projectile] = {"t": _time, "from": at, "station": station, "caliber": caliber,
		"offset": Vector3.ZERO, "seen": false, "trail": -1, "last": at}


func is_rocket(projectile: int) -> bool:
	return _rockets.has(projectile)


## Seconds since rocket `projectile` was noted (INF: not a rocket).
func age(projectile: int) -> float:
	return _time - float(_rockets[projectile].t) if _rockets.has(projectile) else INF


## Launch effects of one rocket at `at` flying along `direction`: the flare
## and the light of `station`, back blast smoke and dust thrown up behind.
func launch(at: Vector3, direction: Vector3, caliber: float, station: int) -> void:
	launched += 1
	effects.counts["rocket_launch"] = effects.counts.get("rocket_launch", 0) + 1
	var dir := direction.normalized() if direction.length() > 0.01 else Vector3.FORWARD
	var s := clampf(caliber / CALIBER_MM, 0.7, 1.3)
	var st := _station(station, at, dir)
	st.pulse = 1.0
	st.last = _time
	st.at = at
	st.dir = dir
	if not effects.visible_at.call(at):
		return
	var back := -dir
	var rear := at + back * 1.2
	# Each rack flares on its own (the reference: one ball per launcher).
	_flares.append({"at": rear, "pulse": 1.0})
	if _flares.size() > FLARES:
		_flares.pop_front()
	effects._burst("core", rear, Basis.IDENTITY, 1.3 * s)
	effects._burst("flash", rear, Basis.looking_at(back, _up(back)), 1.6 * s)
	# The jet points back and down at the ground behind the launcher.
	var blast_dir := (back + Vector3.DOWN * 0.35).normalized()
	effects._burst("rocket_blast", rear, Basis.looking_at(blast_dir, _up(blast_dir)), s)
	var ground := at + Vector3(back.x, 0.0, back.z).normalized() * 4.0
	ground.y = effects.sim_host.height_at(ground.x, ground.z) + 0.2
	effects._burst("blast_dust", ground, Basis.IDENTITY, 1.3 * s, effects._ground_color(ground))
	if st.haze == null and bool(effects._settings.get("fx_salvo_smoke", true)):
		var haze: GPUParticles3D = EffectLibrary.build("salvo_smoke", effects._settings.fx_particles,
			effects._settings.fx_soft, effects._wind())
		add_child(haze)
		haze.global_position = Vector3(at.x, ground.y + 2.0, at.z)
		# One haze over the position (prompt 200): lit as one cloud.
		haze.set_instance_shader_parameter(&"cloud_center", Vector4(at.x, ground.y + 2.0, at.z, 8.0))
		haze.emitting = true
		st.haze = haze


## One frame of rocket `id` at the simulated point `at` with `velocity`:
## body, flame and trail. Returns the drawn point.
func flight(id: int, at: Vector3, velocity: Vector3) -> Vector3:
	var r: Dictionary = _rockets[id]
	_seen_frame[id] = true
	var t: float = _time - float(r.t)
	var dir := velocity.normalized() if velocity.length() > 0.01 else Vector3.UP
	if not r.seen:
		r.seen = true
		# Where the simulation launched it (back along its path).
		r.offset = (r.from as Vector3) - (at - velocity * t)
		launch(r.from, dir, r.caliber, r.station)
		r.trail = _new_trail(r.from)
	var blend := 1.0 - smoothstep(0.0, BLEND_S, t)
	var drawn := at + (r.offset as Vector3) * blend
	r.last = drawn
	var s := clampf(float(r.caliber) / CALIBER_MM, 0.7, 1.3)
	var tail := drawn - dir * BODY_LENGTH_M * 0.5 * s
	_add_body(drawn, dir, s)
	if t < BURN_S:
		var flicker := 0.85 + 0.15 * sin(_time * 61.0 + id) * sin(_time * 37.0 + id * 1.7)
		# The flame shrinks in the last tenth of the burn.
		var strength := flicker * clampf((BURN_S - t) / (BURN_S * 0.1), 0.0, 1.0)
		_add_flame(tail, -dir, s, strength)
		_trail_point(int(r.trail), tail, true)
		burning += 1
	elif int(r.trail) >= 0:
		_trail_point(int(r.trail), tail, false)
		r.trail = -1
	return drawn


# --- Per frame ------------------------------------------------------------------


var _body_count := 0
var _flame_count := 0
var _halo_count := 0


## Called by `artillery_view.gd` before the first `flight()` of a frame.
func begin_frame() -> void:
	_body_count = 0
	_flame_count = 0
	_halo_count = 0
	burning = 0
	_seen_frame.clear()


## Called after the last `flight()`: rockets not seen any more landed; the
## launch flares are added and the frame's instances shown.
func end_frame() -> void:
	for flare: Dictionary in _flares:
		var pulse: float = flare.pulse
		_add_halo(flare.at, FLARE_M * (0.55 + 0.45 * pulse), pulse * FLARE_STRENGTH)
	_bodies.multimesh.visible_instance_count = _body_count
	_flames.multimesh.visible_instance_count = _flame_count
	_halos.multimesh.visible_instance_count = _halo_count
	for id: int in _rockets.keys():
		var r: Dictionary = _rockets[id]
		if not _seen_frame.has(id) and (r.seen or _time - float(r.t) > 5.0):
			if int(r.trail) >= 0:
				_trail_point(int(r.trail), r.last, false)
			_rockets.erase(id)


func _process(delta: float) -> void:
	_time += delta
	_update_stations(delta)
	_update_trails()
	for i in range(_flares.size() - 1, -1, -1):
		var flare: Dictionary = _flares[i]
		flare.pulse = float(flare.pulse) * exp(-delta / FLARE_DECAY_S)
		if flare.pulse < 0.02:
			_flares.remove_at(i)


func _add_body(at: Vector3, dir: Vector3, s: float) -> void:
	if _body_count >= _bodies.multimesh.instance_count:
		return
	var basis := Basis(_perp(dir), dir, dir.cross(_perp(dir))).orthonormalized().scaled(Vector3.ONE * s)
	_bodies.multimesh.set_instance_transform(_body_count, Transform3D(basis, at))
	_body_count += 1


func _add_flame(tail: Vector3, back: Vector3, s: float, strength: float) -> void:
	if _flame_count < _flames.multimesh.instance_count:
		var side := _perp(back)
		var basis := Basis(side * FLAME_WIDTH_M * s, back * FLAME_LENGTH_M * s, side.cross(back))
		_flames.multimesh.set_instance_transform(_flame_count, Transform3D(basis, tail))
		_flames.multimesh.set_instance_color(_flame_count, Color(1, 1, 1, strength))
		_flame_count += 1
	_add_halo(tail + back * 0.6 * s, HALO_M * s, strength)


func _add_halo(at: Vector3, size: float, strength: float) -> void:
	if _halo_count >= _halos.multimesh.instance_count:
		return
	_halos.multimesh.set_instance_transform(_halo_count, Transform3D(Basis.IDENTITY.scaled(Vector3.ONE * size), at))
	_halos.multimesh.set_instance_color(_halo_count, Color(1, 1, 1, strength))
	_halo_count += 1


# --- Firing positions -------------------------------------------------------------


func _station(key: int, at: Vector3, dir: Vector3) -> Dictionary:
	if _stations.has(key):
		return _stations[key]
	var st := {"at": at, "dir": dir, "pulse": 0.0, "last": _time, "light": null, "haze": null}
	var lit := 0
	for other: Dictionary in _stations.values():
		if other.light != null:
			lit += 1
	if lit < int(effects._settings.get("fx_rocket_lights", 1)):
		var light := OmniLight3D.new()
		light.light_color = LIGHT_COLOR
		light.shadow_enabled = false
		light.omni_attenuation = 1.3
		light.visible = false
		add_child(light)
		st.light = light
	_stations[key] = st
	return st


## Flare and light collapse after each launch; the haze stops some seconds
## after the last one and the station goes once it has drifted off.
func _update_stations(delta: float) -> void:
	var daytime := "day"
	var atmosphere: Node = effects.main.atmosphere
	if atmosphere != null:
		daytime = str(atmosphere.daytime)
	for key: int in _stations.keys():
		var st: Dictionary = _stations[key]
		st.pulse = float(st.pulse) * exp(-delta / FLARE_DECAY_S)
		var pulse: float = st.pulse
		var rear: Vector3 = st.at - (st.dir as Vector3) * 1.2
		var shown: bool = effects.visible_at.call(st.at)
		if st.light != null:
			var light: OmniLight3D = st.light
			light.visible = pulse > 0.02 and shown
			light.global_position = rear + Vector3.UP * 1.5
			light.light_energy = LIGHT_ENERGY * pulse * float(BlastEffects.DAYTIME_LIGHT.get(daytime, 1.0))
			light.omni_range = LIGHT_RANGE_M * float(BlastEffects.DAYTIME_RANGE.get(daytime, 1.0))
		var idle: float = _time - float(st.last)
		if st.haze != null:
			var haze: GPUParticles3D = st.haze
			if idle > HAZE_LINGER_S and haze.emitting:
				haze.emitting = false
			if idle > HAZE_LINGER_S + haze.lifetime:
				haze.queue_free()
				st.haze = null
		if idle > HAZE_LINGER_S and st.haze == null:
			if st.light != null:
				(st.light as Node).queue_free()
			_stations.erase(key)


# --- Trails -----------------------------------------------------------------------


## A trail from `from`, reusing the oldest one when all are in use.
func _new_trail(from: Vector3) -> int:
	var limit := int(effects._settings.get("fx_rocket_trails", 48))
	var index := -1
	if _trails.size() < limit:
		var node := MeshInstance3D.new()
		node.mesh = ArrayMesh.new()
		node.material_override = _trail_material
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.extra_cull_margin = 60.0
		add_child(node)
		_trails.append({"node": node})
		index = _trails.size() - 1
	else:
		var oldest := INF
		for i in mini(_trails.size(), limit):
			var t0: float = _trails[i].get("t0", -INF) if _trails[i].get("live", false) else -INF
			if t0 < oldest:
				oldest = t0
				index = i
	var trail: Dictionary = _trails[index]
	trail.t0 = _time
	trail.points = PackedVector3Array([from])
	trail.births = PackedFloat32Array([0.0])
	trail.live = true
	trail.burn = 1.0
	trail.burning = true
	trail.end = _time
	trail.seed = randf() * 100.0
	(trail.node as MeshInstance3D).set_instance_shader_parameter(&"seed", trail.seed)
	(trail.node as MeshInstance3D).visible = true
	# Trails go on top of trails of the same frame: fewer sort jumps.
	return index


## The trail's head at `at`: a new point once it is TRAIL_SPACING_M from the
## last fixed one, else the head moves. `burning` false ends the trail.
func _trail_point(index: int, at: Vector3, still_burning: bool) -> void:
	if index < 0 or index >= _trails.size():
		return
	var trail: Dictionary = _trails[index]
	if not trail.live or not trail.burning:
		return
	var points: PackedVector3Array = trail.points
	var births: PackedFloat32Array = trail.births
	var born: float = _time - float(trail.t0)
	if points.size() < 2 or (points[points.size() - 2].distance_to(at) >= TRAIL_SPACING_M and points.size() < TRAIL_POINTS):
		points.append(at)
		births.append(born)
	else:
		points[points.size() - 1] = at
		births[births.size() - 1] = born
	trail.points = points
	trail.births = births
	trail.burning = still_burning
	trail.end = _time
	_build_trail(trail)


func _build_trail(trail: Dictionary) -> void:
	var points: PackedVector3Array = trail.points
	var births: PackedFloat32Array = trail.births
	var n := points.size()
	var mesh: ArrayMesh = (trail.node as MeshInstance3D).mesh
	mesh.clear_surfaces()
	if n < 2:
		return
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	vertices.resize(n * 2)
	normals.resize(n * 2)
	uvs.resize(n * 2)
	uv2s.resize(n * 2)
	var distance := 0.0
	for i in n:
		if i > 0:
			distance += points[i].distance_to(points[i - 1])
		var tangent := points[mini(i + 1, n - 1)] - points[maxi(i - 1, 0)]
		if tangent.length() < 0.001:
			tangent = Vector3.UP
		for k in 2:
			vertices[i * 2 + k] = points[i]
			normals[i * 2 + k] = tangent.normalized()
			uvs[i * 2 + k] = Vector2(-1.0 if k == 0 else 1.0, distance)
			uv2s[i * 2 + k] = Vector2(births[i], 0.0)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLE_STRIP, arrays)
	trail.length = distance


## Ages every trail; trails older than their life are hidden for reuse.
func _update_trails() -> void:
	var life := float(effects._settings.get("fx_trail_life", 9.0))
	var wind: Vector2 = effects._wind() * TRAIL_WIND
	_trail_material.set_shader_parameter("life", life)
	_trail_material.set_shader_parameter("width", TRAIL_WIDTH_M * float(effects._settings.get("fx_trail_width", 1.0)))
	_trail_material.set_shader_parameter("growth", TRAIL_GROWTH_M * float(effects._settings.get("fx_trail_width", 1.0)))
	_trail_material.set_shader_parameter("drift", Vector3(wind.x, TRAIL_RISE, wind.y))
	var limit := int(effects._settings.get("fx_rocket_trails", 48))
	for i in _trails.size():
		var trail: Dictionary = _trails[i]
		if not trail.get("live", false):
			continue
		var node: MeshInstance3D = trail.node
		if i >= limit or _time - float(trail.end) > life:
			trail.live = false
			node.visible = false
			continue
		if not trail.burning:
			trail.burn = maxf(0.0, float(trail.burn) - get_process_delta_time() / 0.3)
		node.set_instance_shader_parameter(&"age0", _time - float(trail.t0))
		node.set_instance_shader_parameter(&"head", float(trail.get("length", 0.0)))
		node.set_instance_shader_parameter(&"heat", float(trail.burn))


## Trails in use (tools, self-test).
func trails_live() -> int:
	var n := 0
	for trail in _trails:
		if trail.get("live", false):
			n += 1
	return n


static func _perp(v: Vector3) -> Vector3:
	var p := v.cross(Vector3.UP)
	if p.length() < 0.001:
		p = v.cross(Vector3.RIGHT)
	return p.normalized()


static func _up(v: Vector3) -> Vector3:
	return Vector3.UP if absf(v.y) < 0.99 else Vector3.RIGHT
