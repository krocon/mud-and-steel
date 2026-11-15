extends RefCounted
## Particle systems of the effects (prompt 019). `build(kind, ...)` returns a
## fresh `GPUParticles3D` for one effect kind; `effects.gd` keeps pools of
## them. Sizes are meters for a 7.5 cm gun; the emitter's node scale scales
## an effect (particles inherit the emission transform, size and speed).
##
## All particles are camera-facing quads drawn with one small spatial shader
## (`_shader`): texture × particle color × per-instance `tint` (ground color),
## lit with a rounded fake normal so puffs read as volumes in the sun, or
## unshaded (fire, flashes, sparks; HDR values feed the glow). Soft
## particles fade where they cut into the ground (quality "fx_soft").
## Presentation only.

const EffectTextures := preload("res://scenes/world/effect_textures.gd")

## Kinds that are fired as one-shot bursts (pooled).
const BURSTS: Array[String] = [
	"flash", "powder", "blast_dust", "fountain", "spray", "impact_dust", "puff",
	"sparks", "fireball", "explosion_smoke", "debris",
]
## Kinds that run continuously (wreck fires, vehicles).
const CONTINUOUS: Array[String] = ["fire", "smoke_column", "dust_trail", "exhaust"]

const SHADER_TEMPLATE := """
shader_type spatial;
render_mode %s, depth_draw_never, cull_disabled, shadows_disabled%s;

uniform sampler2D tex : source_color, filter_linear_mipmap, repeat_disable;
uniform float emission = 0.0;
uniform float soft_distance = 0.0;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
instance uniform vec4 tint : source_color = vec4(1.0);

void vertex() {
	// Face the camera like BILLBOARD_PARTICLES: keep the particle's position
	// and scale, spin it around the view axis by its angle (CUSTOM.x).
	mat4 world = mat4(normalize(INV_VIEW_MATRIX[0]), normalize(INV_VIEW_MATRIX[1]),
		normalize(INV_VIEW_MATRIX[2]), MODEL_MATRIX[3]);
	float a = INSTANCE_CUSTOM.x;
	world = world * mat4(vec4(cos(a), -sin(a), 0.0, 0.0), vec4(sin(a), cos(a), 0.0, 0.0),
		vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_MATRIX = VIEW_MATRIX * world * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
}

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = t.a * c.a;
	if (soft_distance > 0.0) {
		float depth = textureLod(depth_tex, SCREEN_UV, 0.0).r;
		vec4 scene = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
		alpha *= clamp((VERTEX.z - scene.z / scene.w) / soft_distance, 0.0, 1.0);
	}
	%s
	ALPHA = alpha;
}
"""
const LIT_BODY := """vec2 p = UV * 2.0 - 1.0;
	NORMAL = normalize(vec3(p.x, -p.y, 1.1));
	ALBEDO = t.rgb * c.rgb;
	EMISSION = ALBEDO * emission;"""
const UNSHADED_BODY := "ALBEDO = t.rgb * c.rgb * (1.0 + emission);"

static var _shaders: Dictionary[String, Shader] = {}


## A new particle system of `kind`. `factor` scales particle counts
## (quality), `soft` turns on soft particles, `wind` (x, z) drifts smoke.
static func build(kind: String, factor: float, soft: bool, wind: Vector2) -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.name = kind
	p.local_coords = false
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.one_shot = kind in BURSTS
	p.emitting = false
	p.explosiveness = 0.9 if p.one_shot else 0.0
	p.fixed_fps = 0
	p.interpolate = false
	p.visibility_aabb = AABB(Vector3(-25, -5, -25), Vector3(50, 40, 50))
	var pm := ParticleProcessMaterial.new()
	pm.angle_min = 0.0
	pm.angle_max = 360.0
	var drift := Vector3(wind.x, 0.0, wind.y)
	var amount := 8
	var size := 1.0
	var material: ShaderMaterial
	match kind:
		"flash":
			amount = 5
			p.lifetime = 0.08
			p.explosiveness = 1.0
			size = 1.5
			_cone(pm, Vector3(0, 0, -1), 6.0, 3.0, 9.0)
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 1.0), Vector2(1, 0.4)])
			pm.color_ramp = _ramp([Color(1, 0.95, 0.75, 1), Color(1, 0.6, 0.2, 0.9), Color(1, 0.4, 0.1, 0)], [0.0, 0.5, 1.0])
			material = _material("add", EffectTextures.flash(), 3.0, false)
		"powder":
			amount = 14
			p.lifetime = 3.5
			size = 1.6
			_cone(pm, Vector3(0, 0, -1), 22.0, 3.0, 10.0)
			pm.damping_min = 3.0
			pm.damping_max = 5.0
			pm.gravity = Vector3(0, 0.25, 0) + drift * 0.4
			pm.scale_min = 0.7
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.35), Vector2(0.25, 1.4), Vector2(1, 2.4)])
			pm.angular_velocity_min = -25.0
			pm.angular_velocity_max = 25.0
			pm.color_ramp = _ramp([Color(0.96, 0.95, 0.92, 0), Color(0.96, 0.95, 0.92, 0.8), Color(0.9, 0.89, 0.86, 0.45), Color(0.88, 0.87, 0.85, 0)], [0.0, 0.06, 0.4, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, soft)
		"blast_dust":
			amount = 16
			p.lifetime = 2.6
			size = 1.8
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_RING
			pm.emission_ring_axis = Vector3.UP
			pm.emission_ring_radius = 0.8
			pm.emission_ring_inner_radius = 0.2
			pm.emission_ring_height = 0.1
			pm.direction = Vector3.UP
			pm.spread = 10.0
			pm.initial_velocity_min = 0.3
			pm.initial_velocity_max = 1.2
			pm.radial_velocity_min = 3.0
			pm.radial_velocity_max = 8.0
			pm.damping_min = 3.0
			pm.damping_max = 5.0
			pm.gravity = Vector3(0, 0.2, 0) + drift * 0.3
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 2.4)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.5), Color(1, 1, 1, 0)], [0.0, 0.08, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"fountain":
			amount = 44
			p.lifetime = 1.8
			size = 0.6
			_cone(pm, Vector3.UP, 16.0, 7.0, 19.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.4
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.4
			pm.scale_max = 1.3
			pm.angular_velocity_min = -300.0
			pm.angular_velocity_max = 300.0
			pm.color_ramp = _ramp([Color(0.75, 0.75, 0.75, 1), Color(0.75, 0.75, 0.75, 1), Color(0.75, 0.75, 0.75, 0)], [0.0, 0.8, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"spray":
			amount = 22
			p.lifetime = 1.8
			size = 1.0
			_cone(pm, Vector3.UP, 9.0, 7.0, 15.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.3
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 1.8)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.9), Color(1, 1, 1, 0.6), Color(1, 1, 1, 0)], [0.0, 0.6, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"impact_dust":
			amount = 18
			p.lifetime = 4.0
			size = 2.0
			_cone(pm, Vector3.UP, 18.0, 2.0, 11.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.6
			pm.damping_min = 2.5
			pm.damping_max = 3.5
			pm.gravity = Vector3(0, 0.25, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(1, 2.8)])
			pm.angular_velocity_min = -15.0
			pm.angular_velocity_max = 15.0
			pm.color_ramp = _ramp([Color(0.8, 0.8, 0.8, 0), Color(0.8, 0.8, 0.8, 0.95), Color(1, 1, 1, 0.45), Color(1, 1, 1, 0)], [0.0, 0.04, 0.45, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"puff":
			amount = 4
			p.lifetime = 1.0
			size = 0.7
			_cone(pm, Vector3.UP, 50.0, 0.5, 1.6)
			pm.damping_min = 1.0
			pm.damping_max = 2.0
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 1.6)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.7), Color(1, 1, 1, 0)], [0.0, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, false)
		"sparks":
			amount = 24
			p.lifetime = 0.5
			p.explosiveness = 1.0
			size = 0.3
			_cone(pm, Vector3.UP, 75.0, 5.0, 18.0)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.2
			pm.color_ramp = _ramp([Color(1, 0.95, 0.8, 1), Color(1, 0.6, 0.15, 1), Color(0.8, 0.2, 0.05, 0)], [0.0, 0.4, 1.0])
			material = _material("add", EffectTextures.spark(), 6.0, false)
		"fireball":
			amount = 20
			p.lifetime = 1.3
			size = 2.4
			_cone(pm, Vector3.UP, 180.0, 2.0, 7.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.9
			pm.damping_min = 3.0
			pm.damping_max = 5.0
			pm.gravity = Vector3(0, 2.5, 0)
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.45), Vector2(0.25, 1.4), Vector2(1, 1.2)])
			pm.angular_velocity_min = -60.0
			pm.angular_velocity_max = 60.0
			pm.color_ramp = _ramp([
				Color(1, 0.78, 0.4, 1), Color(1, 0.42, 0.08, 1), Color(0.5, 0.12, 0.03, 0.9),
				Color(0.08, 0.06, 0.05, 0.7), Color(0.06, 0.05, 0.05, 0)], [0.0, 0.2, 0.45, 0.7, 1.0])
			material = _material("unshaded", EffectTextures.fire(), 1.0, soft)
		"explosion_smoke":
			amount = 30
			p.lifetime = 9.0
			p.explosiveness = 0.75
			size = 3.2
			_cone(pm, Vector3.UP, 40.0, 2.0, 6.5)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 1.2
			pm.damping_min = 0.8
			pm.damping_max = 1.6
			pm.gravity = Vector3(0, 0.8, 0) + drift * 0.6
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(0.3, 2.0), Vector2(1, 4.0)])
			pm.angular_velocity_min = -12.0
			pm.angular_velocity_max = 12.0
			pm.color_ramp = _ramp([
				Color(0.04, 0.035, 0.03, 0), Color(0.05, 0.045, 0.04, 0.97), Color(0.13, 0.12, 0.115, 0.75),
				Color(0.26, 0.25, 0.24, 0)], [0.0, 0.05, 0.45, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, soft)
		"debris":
			# Dark tumbling fragments (sprites: a mesh with a StandardMaterial3D
			# leaked its shader at exit).
			amount = 14
			p.lifetime = 2.0
			p.explosiveness = 1.0
			size = 0.35
			_cone(pm, Vector3.UP, 55.0, 5.0, 14.0)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.4
			pm.angular_velocity_min = -500.0
			pm.angular_velocity_max = 500.0
			pm.color_ramp = _ramp([Color(0.12, 0.11, 0.1, 1), Color(0.12, 0.11, 0.1, 1), Color(0.12, 0.11, 0.1, 0)], [0.0, 0.85, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"fire":
			amount = 26
			p.lifetime = 0.9
			size = 1.3
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(0.6, 0.15, 0.6)
			_cone(pm, Vector3.UP, 12.0, 1.2, 2.6)
			pm.gravity = Vector3(0, 1.8, 0) + drift * 0.2
			pm.scale_min = 0.6
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.7), Vector2(0.3, 1.0), Vector2(1, 0.2)])
			pm.angular_velocity_min = -90.0
			pm.angular_velocity_max = 90.0
			pm.color_ramp = _ramp([Color(1, 0.85, 0.45, 0.9), Color(1, 0.5, 0.12, 0.85), Color(0.6, 0.15, 0.04, 0)], [0.0, 0.45, 1.0])
			material = _material("unshaded", EffectTextures.fire(), 1.2, soft)
		"smoke_column":
			amount = 64
			p.lifetime = 14.0
			size = 3.2
			p.visibility_aabb = AABB(Vector3(-60, -5, -60), Vector3(120, 90, 120))
			_cone(pm, Vector3.UP, 8.0, 3.5, 5.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.8
			pm.damping_min = 0.2
			pm.damping_max = 0.35
			pm.gravity = Vector3(0, 0.2, 0) + drift * 0.3
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(0.3, 2.0), Vector2(1, 6.0)])
			pm.angular_velocity_min = -8.0
			pm.angular_velocity_max = 8.0
			pm.color_ramp = _ramp([
				Color(0.03, 0.028, 0.025, 0), Color(0.035, 0.032, 0.03, 0.97), Color(0.09, 0.085, 0.08, 0.85),
				Color(0.2, 0.19, 0.185, 0)], [0.0, 0.03, 0.4, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, soft)
		"dust_trail":
			amount = 48
			p.lifetime = 3.2
			size = 2.2
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.4, 0.15, 0.5)
			_cone(pm, Vector3.UP, 45.0, 0.6, 2.2)
			pm.damping_min = 0.8
			pm.damping_max = 1.4
			pm.gravity = Vector3(0, 0.15, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 3.0)])
			pm.angular_velocity_min = -15.0
			pm.angular_velocity_max = 15.0
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.55), Color(1, 1, 1, 0)], [0.0, 0.12, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"exhaust":
			amount = 16
			p.lifetime = 2.2
			size = 0.7
			_cone(pm, Vector3.UP, 20.0, 0.5, 1.2)
			pm.gravity = Vector3(0, 0.4, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.3), Vector2(1, 1.8)])
			pm.color_ramp = _ramp([Color(0.22, 0.21, 0.2, 0), Color(0.24, 0.23, 0.22, 0.5), Color(0.4, 0.39, 0.38, 0)], [0.0, 0.1, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, false)
		_:
			push_error("unknown effect kind '%s'" % kind)
	p.amount = maxi(1, int(round(amount * factor)))
	p.process_material = pm
	if material != null:
		var quad := QuadMesh.new()
		quad.size = Vector2(size, size)
		quad.material = material
		p.draw_pass_1 = quad
	return p


## Emission along `direction` within `spread` degrees.
static func _cone(pm: ParticleProcessMaterial, direction: Vector3, spread: float, v_min: float, v_max: float) -> void:
	pm.direction = direction
	pm.spread = spread
	pm.initial_velocity_min = v_min
	pm.initial_velocity_max = v_max


static func _curve(points: Array[Vector2]) -> CurveTexture:
	var curve := Curve.new()
	var top := 1.0
	for point in points:
		top = maxf(top, point.y)
	curve.max_value = top
	for point in points:
		curve.add_point(point)
	var texture := CurveTexture.new()
	texture.curve = curve
	return texture


static func _ramp(colors: Array[Color], offsets: Array[float]) -> GradientTexture1D:
	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array(offsets)
	gradient.colors = PackedColorArray(colors)
	var texture := GradientTexture1D.new()
	texture.gradient = gradient
	return texture


## mode: "lit" (alpha blend, sun light), "unshaded" (alpha blend, glowing)
## or "add" (additive, glowing).
static func _material(mode: String, texture: Texture2D, emission: float, soft: bool) -> ShaderMaterial:
	if not _shaders.has(mode):
		var shader := Shader.new()
		var blend := "blend_add" if mode == "add" else "blend_mix"
		var extra := "" if mode == "lit" else ", unshaded"
		shader.code = SHADER_TEMPLATE % [blend, extra, LIT_BODY if mode == "lit" else UNSHADED_BODY]
		_shaders[mode] = shader
	var material := ShaderMaterial.new()
	material.shader = _shaders[mode]
	material.set_shader_parameter("tex", texture)
	material.set_shader_parameter("emission", emission)
	material.set_shader_parameter("soft_distance", 0.8 if soft else 0.0)
	return material


static func clear_cache() -> void:
	_shaders.clear()
