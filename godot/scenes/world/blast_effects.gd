extends Node3D
## Big blasts (prompt 054): heavy shells, bombs, rockets, exploding dumps
## and tank ammunition explosions get a white shock-wave dome that expands
## and fades (the condensation of the shock front), a flat pressure ring
## and a dust ring racing over the ground, a bright flash light that lights
## up the ground and the buildings around (much stronger at night), and a
## stronger camera shake. A tank whose ammunition explodes
## (`StrikeEvent::AmmoExplosion`, rule of prompt 048) throws its turret.
##
## Presentation only, on top of the ordinary effects of `effects.gd` (which
## still draw fountain, fire and smoke): triggered by the combat events
## (`hit` with a caliber of at least BIG_CALIBER) and by `strike_view.gd`.
## Godot randomness is fine here: it never reaches the simulation.
##
## Budget: pooled domes, rings and dust rings; flash lights per graphics
## quality (LIGHTS); at most FLYING_TURRETS turrets in the air and
## LYING_TURRETS on the ground. Like `effects.gd`, nothing appears where the
## local player does not see (`effects.visible_at`).

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const EffectTextures := preload("res://scenes/world/effect_textures.gd")

## From this caliber (mm) a ground or object hit is a big blast: 12.2 cm
## howitzers and up, bombs (data/air.toml: 15 cm and more), rockets of the
## heavy strike, exploding dumps (data/fire.toml). [provisional]
const BIG_CALIBER := 110.0
## Shock dome radius per mm of caliber (m) and how long it grows (s).
const DOME_M_PER_MM := 0.075
const DOME_SECONDS := 0.32
## The pressure ring runs out to this multiple of the dome radius.
const RING_FACTOR := 2.0
const RING_SECONDS := 0.55
const POOL := 12
## Flash lights at once per graphics quality.
const LIGHTS := {"low": 1, "medium": 2, "high": 4}
const FLASH_COLOR := Color(1.0, 0.78, 0.5)
const FLASH_SECONDS := 0.32
## Flash light strength by time of day (prompt 040): at night it lights up
## the whole neighbourhood.
const DAYTIME_LIGHT := {"day": 1.0, "morning": 1.7, "evening": 1.7, "night": 3.0}
const DAYTIME_RANGE := {"day": 1.0, "morning": 1.2, "evening": 1.2, "night": 1.45}
const GRAVITY := 9.81
const FLYING_TURRETS := 6
const LYING_TURRETS := 16

const DOME_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_back, unshaded, shadows_disabled;
instance uniform float fade = 1.0;
uniform vec3 color : source_color = vec3(0.96, 0.96, 0.94);
void fragment() {
	// Bright rim, nearly clear middle: a thin shell of condensation.
	float facing = abs(dot(normalize(NORMAL), normalize(VIEW)));
	float rim = pow(1.0 - facing, 2.2);
	ALBEDO = color;
	// Fainter since prompt 072: the reference shows the dust dome, hardly
	// a condensation shell.
	ALPHA = clamp((0.03 + 0.45 * rim) * fade, 0.0, 1.0);
}
"""
const RING_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;
instance uniform float fade = 1.0;
uniform vec3 color : source_color = vec3(0.90, 0.86, 0.78);
void fragment() {
	float r = length(UV * 2.0 - 1.0);
	// Soft sandy-beige ground shockwave dust band (never a hard wireframe ring).
	float band = smoothstep(0.42, 0.82, r) * (1.0 - smoothstep(0.84, 1.0, r));
	float haze = 0.08 * (1.0 - smoothstep(0.0, 0.85, r));
	ALBEDO = color;
	ALPHA = clamp((band * 0.24 + haze) * fade, 0.0, 1.0);
}
"""

var main: Node3D
var sim_host: SimHost
## Started effects per kind (self-test, tools).
var counts: Dictionary[String, int] = {}

var _root: Node3D
var _domes: Array[Dictionary] = []
var _rings: Array[Dictionary] = []
var _dust: Array[GPUParticles3D] = []
var _dust_next := 0
var _lights: Array[Dictionary] = []
var _flying: Array[Dictionary] = []
var _lying: Array[Node3D] = []
var _quality := ""
var _dome_material: ShaderMaterial
var _ring_material: ShaderMaterial


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_root = Node3D.new()
	_root.name = "BigBlasts"
	add_child(_root)
	var dome_shader := Shader.new()
	dome_shader.code = DOME_SHADER
	_dome_material = ShaderMaterial.new()
	_dome_material.shader = dome_shader
	var ring_shader := Shader.new()
	ring_shader.code = RING_SHADER
	_ring_material = ShaderMaterial.new()
	_ring_material.shader = ring_shader
	var dome_mesh := SphereMesh.new()
	dome_mesh.radius = 1.0
	dome_mesh.height = 1.0
	dome_mesh.is_hemisphere = true
	dome_mesh.radial_segments = 32
	dome_mesh.rings = 12
	dome_mesh.material = _dome_material
	var ring_mesh := PlaneMesh.new()
	ring_mesh.size = Vector2(2, 2)
	ring_mesh.material = _ring_material
	for i in POOL:
		_domes.append(_pooled(dome_mesh))
		_rings.append(_pooled(ring_mesh))
	_configure()
	main.combat_view.combat_event.connect(_on_combat_event)


func _pooled(mesh: Mesh) -> Dictionary:
	var m := MeshInstance3D.new()
	m.mesh = mesh
	m.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	m.visible = false
	_root.add_child(m)
	return {"node": m, "age": 0.0, "seconds": 1.0, "radius": 1.0, "busy": false}


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_configure()
	_grow(_domes, delta, false)
	_grow(_rings, delta, true)
	_update_lights(delta)
	_update_turrets(delta)


func _on_combat_event(event: Dictionary) -> void:
	if event["type"] != "hit" or float(event["caliber_mm"]) < BIG_CALIBER:
		return
	var at: Vector3 = event["position"]
	if event["surface"] == "ground" and sim_host.terrain_at(at.x, at.z) == main.effects.TERRAIN_WATER_DEEP:
		return
	big_blast(at, float(event["caliber_mm"]))


## A big blast of `caliber` mm at `at`: shock dome, pressure and dust ring,
## flash light, camera shake.
func big_blast(at: Vector3, caliber: float) -> void:
	if not main.effects.visible_at.call(at):
		return
	var ground := Vector3(at.x, sim_host.height_at(at.x, at.z), at.z)
	var radius := maxf(caliber, BIG_CALIBER) * DOME_M_PER_MM
	_start(_domes, ground, radius, DOME_SECONDS)
	_start(_rings, ground + Vector3.UP * 0.25, radius * RING_FACTOR, RING_SECONDS)
	var dust: Color = main.effects._ground_color(ground)
	_dust_ring(ground, radius, dust.lerp(Color(0.84, 0.78, 0.68), 0.52))
	var s := clampf(caliber / 100.0, 1.0, 2.2)
	# On top of the ordinary impact of `effects.gd` (prompt 072, after the
	# reference video): a grey-beige dome of dust ~16 m across within 0.5 s
	# (15 cm), short flame tongues that turn into dark smoke, and debris.
	var fx: Node3D = main.effects
	fx._burst("dust_cloud", ground + Vector3.UP * 1.0 * s, Basis.IDENTITY, s, _cloud_tint(dust))
	fx._burst("fireball", ground + Vector3.UP * 1.2 * s, Basis.IDENTITY, 0.5 * s)
	fx._burst("debris", ground + Vector3.UP, Basis.IDENTITY, s)
	flash(ground + Vector3.UP * 4.0, s)
	main.effects._add_shake(ground, 0.9 * s)
	counts["big_blast"] = counts.get("big_blast", 0) + 1


## Flat expanding ground pressure/dust ring (prompt 220, used by vehicle hits
## and cook-offs).
func ground_ring(ground: Vector3, radius: float) -> void:
	if _rings.is_empty():
		return
	_start(_rings, ground + Vector3.UP * 0.25, radius, RING_SECONDS)


## Grey-beige: earth thrown up with the powder smoke (the reference video's
## cloud measured #b59e88 at 0.1 s, #867466 at 1.7 s, prompt 072).
static func _cloud_tint(dust: Color) -> Color:
	return dust.lerp(Color(0.62, 0.58, 0.53), 0.7)


## A short bright light of relative strength `s`.
func flash(at: Vector3, s: float) -> void:
	if _lights.is_empty():
		return
	var best: Dictionary = _lights[0]
	for entry in _lights:
		if entry.left < best.left:
			best = entry
	var daytime := _daytime()
	var light: OmniLight3D = best.light
	light.global_position = at
	light.omni_range = 30.0 * s * float(DAYTIME_RANGE.get(daytime, 1.0))
	best.energy = 9.0 * s * float(DAYTIME_LIGHT.get(daytime, 1.0))
	light.light_energy = best.energy
	light.visible = true
	best.left = FLASH_SECONDS
	counts["big_flash"] = counts.get("big_flash", 0) + 1


## A destroyed tank's ammunition explodes (`StrikeEvent::AmmoExplosion`):
## a bigger fireball and blast, and the turret flies off.
func ammo_explosion(unit: int, at: Vector3) -> void:
	if not main.effects.visible_at.call(at):
		return
	main.effects.explosion(at, 1.5)
	big_blast(at, 150.0)
	counts["ammo_explosion"] = counts.get("ammo_explosion", 0) + 1
	var node: Node3D = main._unit_nodes.get(unit)
	if node == null:
		return
	var turret := node.find_child("Turret", true, false) as Node3D
	if turret == null or turret.has_meta(&"blown"):
		return
	var xform := turret.global_transform
	turret.get_parent().remove_child(turret)
	_root.add_child(turret)
	turret.global_transform = xform
	turret.set_meta(&"blown", true)
	# The turret no longer follows the aim; it burns out like the hull.
	var combat: Node = main.combat_view
	if combat._turrets.has(unit):
		combat._turrets[unit] = null
	combat._tint(turret, combat._wreck_material)
	var side := Vector2(randf_range(-1.0, 1.0), randf_range(-1.0, 1.0)).normalized() * randf_range(1.5, 4.0)
	_flying.append({
		"node": turret,
		"velocity": Vector3(side.x, randf_range(10.0, 15.0), side.y),
		"axis": Vector3(randf_range(-1, 1), randf_range(-0.3, 0.3), randf_range(-1, 1)).normalized(),
		"spin": randf_range(3.0, 8.0),
	})
	counts["turret"] = counts.get("turret", 0) + 1
	while _flying.size() > FLYING_TURRETS:
		_land(_flying.pop_front())


func _start(pool: Array[Dictionary], at: Vector3, radius: float, seconds: float) -> void:
	var best: Dictionary = pool[0]
	for entry in pool:
		if not entry.busy:
			best = entry
			break
		if entry.age / entry.seconds > best.age / best.seconds:
			best = entry
	best.busy = true
	best.age = 0.0
	best.seconds = seconds
	best.radius = radius
	var node: MeshInstance3D = best.node
	node.global_position = at
	node.scale = Vector3.ONE * 0.05
	node.visible = true
	node.set_instance_shader_parameter(&"fade", 1.0)


## Domes grow fast and fade; rings run out flat.
func _grow(pool: Array[Dictionary], delta: float, flat: bool) -> void:
	for entry in pool:
		if not entry.busy:
			continue
		entry.age += delta
		var t: float = entry.age / entry.seconds
		var node: MeshInstance3D = entry.node
		if t >= 1.0:
			entry.busy = false
			node.visible = false
			continue
		# Fast at first, slowing down (the shock front loses speed).
		var grow := 1.0 - pow(1.0 - t, 3.0)
		var r: float = maxf(0.05, entry.radius * grow)
		node.scale = Vector3(r, 1.0, r) if flat else Vector3(r, r * 0.85, r)
		node.set_instance_shader_parameter(&"fade", (1.0 - t) * (1.0 - t) * (0.6 + 0.4 * minf(t * 8.0, 1.0)))


func _dust_ring(at: Vector3, radius: float, color: Color) -> void:
	if _dust.is_empty():
		return
	var p: GPUParticles3D = _dust[_dust_next]
	_dust_next = (_dust_next + 1) % _dust.size()
	p.global_position = at + Vector3.UP * 0.4
	var s := radius / 12.0
	p.scale = Vector3.ONE * s
	p.set_instance_shader_parameter(&"tint", color)
	p.visible = true
	p.restart()


func _update_lights(delta: float) -> void:
	for entry in _lights:
		if entry.left <= 0.0:
			continue
		entry.left -= delta
		var light: OmniLight3D = entry.light
		if entry.left <= 0.0:
			light.visible = false
		else:
			var t: float = entry.left / FLASH_SECONDS
			light.light_energy = entry.energy * t * t


func _update_turrets(delta: float) -> void:
	for i in range(_flying.size() - 1, -1, -1):
		var f: Dictionary = _flying[i]
		var node: Node3D = f.node
		if not is_instance_valid(node):
			_flying.remove_at(i)
			continue
		var v: Vector3 = f.velocity
		v.y -= GRAVITY * delta
		f.velocity = v
		node.global_position += v * delta
		node.global_rotate(f.axis, float(f.spin) * delta)
		var ground := sim_host.height_at(node.global_position.x, node.global_position.z)
		if v.y < 0.0 and node.global_position.y <= ground + 0.4:
			_flying.remove_at(i)
			_land(f)


func _land(f: Dictionary) -> void:
	var node: Node3D = f.node
	if not is_instance_valid(node):
		return
	var p := node.global_position
	p.y = sim_host.height_at(p.x, p.z) + 0.4
	node.global_position = p
	main.effects._burst("impact_dust", p, Basis.IDENTITY, 1.2, main.effects._ground_color(p))
	main.effects._add_shake(p, 0.3)
	_lying.append(node)
	while _lying.size() > LYING_TURRETS:
		var old: Node3D = _lying.pop_front()
		if is_instance_valid(old):
			old.queue_free()


## Turrets in the air (self-test).
func flying_turrets() -> int:
	return _flying.size()


func _daytime() -> String:
	var atmosphere: Node = main.atmosphere
	return str(atmosphere.daytime) if atmosphere != null else "day"


## (Re)builds lights and dust rings when the graphics quality changed.
func _configure() -> void:
	var quality: String = main.atmosphere.quality if main.atmosphere != null else "medium"
	if quality == _quality:
		return
	_quality = quality
	for entry in _lights:
		(entry.light as Node).queue_free()
	_lights.clear()
	for i in int(LIGHTS.get(quality, 2)):
		var light := OmniLight3D.new()
		light.light_color = FLASH_COLOR
		light.shadow_enabled = false
		light.omni_attenuation = 1.2
		light.visible = false
		_root.add_child(light)
		_lights.append({"light": light, "left": 0.0, "energy": 0.0})
	for p in _dust:
		p.queue_free()
	_dust.clear()
	var factor: float = {"low": 0.4, "medium": 0.7}.get(quality, 1.0)
	for i in (4 if quality == "low" else 8):
		var p := _build_dust_ring(factor, quality == "high")
		p.visible = false
		_root.add_child(p)
		_dust.append(p)


## A ring of dust thrown outwards along the ground (about 14 m at scale 1).
static func _build_dust_ring(factor: float, soft: bool) -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.name = "dust_ring"
	p.local_coords = false
	p.one_shot = true
	p.explosiveness = 0.95
	p.emitting = false
	p.lifetime = 2.6
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.visibility_aabb = AABB(Vector3(-60, -5, -60), Vector3(120, 30, 120))
	p.amount = maxi(8, int(round(36 * factor)))
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_RING
	pm.emission_ring_axis = Vector3.UP
	pm.emission_ring_radius = 1.5
	pm.emission_ring_inner_radius = 1.0
	pm.emission_ring_height = 0.2
	pm.direction = Vector3.UP
	pm.spread = 8.0
	pm.initial_velocity_min = 0.5
	pm.initial_velocity_max = 2.0
	pm.radial_velocity_min = 10.0
	pm.radial_velocity_max = 14.0
	pm.damping_min = 6.0
	pm.damping_max = 7.0
	pm.gravity = Vector3(0, 0.3, 0)
	pm.angle_min = 0.0
	pm.angle_max = 360.0
	pm.angular_velocity_min = -20.0
	pm.angular_velocity_max = 20.0
	pm.scale_curve = EffectLibrary._curve([Vector2(0, 0.6), Vector2(0.3, 1.6), Vector2(1, 2.6)])
	pm.color_ramp = EffectLibrary._ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.5), Color(1, 1, 1, 0.22), Color(1, 1, 1, 0)], [0.0, 0.05, 0.4, 1.0])
	p.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(2.6, 2.6)
	quad.material = EffectLibrary._material("lit", EffectTextures.smoke(), 0.15, soft)
	p.draw_pass_1 = quad
	return p
