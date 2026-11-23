extends Node3D
## Waterfalls (prompt 122): falling water, spray and sound between the map
## markers `waterfall_<n>_top` (the rock edge) and `waterfall_<n>_foot`
## (where it meets the water), n = 1, 2, ... Presentation only: no rule
## knows them, the ground and the water stay as the simulation has them.
##
## The falling water is a ribbon over the ground from the top to the foot
## (a little above it, wider at the foot) with streaks that run down; the
## foot has white mist (`EffectLibrary` "waterfall_mist") and the river
## sound of the ambience as a 3D loop on the `Ambience` bus.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const Ambience := preload("res://scenes/audio/ambience.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

## Width of the falling water at the top and at the foot (m).
const WIDTH_M := Vector2(7.0, 12.0)
## Height of the ribbon above the ground (m).
const LIFT_M := 0.6
## Samples along the fall.
const STEPS := 24
## Loudness of the fall at its foot and the distance over which it fades.
const SOUND_DB := -2.0
const SOUND_RANGE_M := 160.0

const SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, diffuse_burley, specular_schlick_ggx;

// UV.x across (0..1), UV.y along the fall in meters from the top.
uniform float speed = 7.0;

float h1(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vn(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(h1(i), h1(i + vec2(1, 0)), f.x), mix(h1(i + vec2(0, 1)), h1(i + vec2(1, 1)), f.x), f.y);
}

void fragment() {
	float across = UV.x;
	float along = UV.y;
	// Streaks: noise stretched along the fall, running down.
	vec2 q = vec2(across * 9.0, along * 0.35 - TIME * speed * 0.35);
	float streak = vn(q) * 0.65 + vn(q * vec2(2.3, 1.7) + 3.1) * 0.35;
	float edge = smoothstep(0.0, 0.22, across) * smoothstep(1.0, 0.78, across);
	float foam = smoothstep(0.15, 0.6, streak);
	vec3 water = vec3(0.55, 0.72, 0.78);
	ALBEDO = mix(water, vec3(0.95, 0.97, 0.98), foam);
	ROUGHNESS = 0.35;
	ALPHA = edge * mix(0.55, 0.95, foam) * COLOR.a;
}
"""

var count := 0


## Builds every waterfall of the map (`host`: SimHost) for the graphics
## quality `level`.
func build(host: Object, level: String) -> void:
	var material := ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = SHADER
	material.shader = shader
	var settings: Dictionary = GraphicsQuality.settings(level)
	var n := 1
	while true:
		var top: Variant = host.map_marker("waterfall_%d_top" % n)
		var foot: Variant = host.map_marker("waterfall_%d_foot" % n)
		if top == null or foot == null:
			break
		_add(host, top as Vector3, foot as Vector3, material, settings, n)
		n += 1
	count = n - 1
	if count > 0:
		print("waterfalls: %d" % count)


func _add(host: Object, top: Vector3, foot: Vector3, material: ShaderMaterial, settings: Dictionary, n: int) -> void:
	var flat := Vector3(foot.x - top.x, 0.0, foot.z - top.z)
	var length := flat.length()
	if length < 1.0:
		return
	var dir := flat / length
	var side := Vector3(-dir.z, 0.0, dir.x)
	var level: float = host.water_level_at(foot.x, foot.z)
	if is_nan(level):
		level = host.height_at(foot.x, foot.z)
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var colors := PackedColorArray()
	var indices := PackedInt32Array()
	var along := 0.0
	var last := Vector3.ZERO
	for i in STEPS + 1:
		var t := float(i) / STEPS
		var p := top + flat * t
		var ground: float = host.height_at(p.x, p.z)
		var y := maxf(ground + LIFT_M * (1.0 - t), level + 0.05)
		var c := Vector3(p.x, y, p.z)
		if i > 0:
			along += c.distance_to(last)
		last = c
		var half := lerpf(WIDTH_M.x, WIDTH_M.y, t) * 0.5
		for s in [-1.0, 1.0]:
			verts.append(c + side * half * s)
			uvs.append(Vector2(0.5 + 0.5 * s, along))
			# Fades in at the top edge and out into the pool.
			colors.append(Color(1, 1, 1, smoothstep(0.0, 0.06, t) * smoothstep(1.0, 0.9, t)))
		if i > 0:
			var k := i * 2
			indices.append_array([k - 2, k - 1, k, k - 1, k + 1, k])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var ribbon := MeshInstance3D.new()
	ribbon.name = "Waterfall%d" % n
	ribbon.mesh = mesh
	ribbon.material_override = material
	ribbon.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(ribbon)
	var at := Vector3(foot.x, level, foot.z)
	var mist := EffectLibrary.build("waterfall_mist", float(settings.get("fx_particles", 1.0)),
		bool(settings.get("fx_soft", true)), Vector2(0.4, 0.2))
	mist.name = "Mist%d" % n
	add_child(mist)
	mist.global_position = at
	mist.emitting = true
	var sounds: Array = Ambience.streams("water")
	if not sounds.is_empty():
		var player := AudioStreamPlayer3D.new()
		player.name = "Sound%d" % n
		player.bus = "Ambience"
		player.stream = sounds[0]
		player.volume_db = SOUND_DB
		player.pitch_scale = 0.8
		player.unit_size = 25.0
		player.max_distance = SOUND_RANGE_M
		player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
		player.doppler_tracking = AudioStreamPlayer3D.DOPPLER_TRACKING_DISABLED
		add_child(player)
		player.global_position = at + Vector3(0.0, 2.0, 0.0)
		player.play(0.0)
