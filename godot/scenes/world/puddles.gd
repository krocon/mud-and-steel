extends Node3D
## Standing water, smoke and mist of a battlefield map (prompt 124).
## Presentation only: no rule knows any of it.
##
## - Craters: the prepared craters of the map (`SimHost.map_craters`) are
##   dug into the ground by the simulation, but at 2 m between height
##   samples a shell hole hardly shows. Each gets a skin draped over the
##   ground (the terrain's own height texture, so it follows the bowl and
##   later changes): dark wet mud in the bowl, a rim of thrown-up earth,
##   fading out at the end of the rim. One MultiMesh per 256 m tile.
## - Puddles: brown rain water in the bowls of the craters that have water
##   and in the wet hollows of `map.toml` (`[[puddle]]`;
##   `SimHost.map_puddles`), as flat glossy discs a little above the bottom
##   (one MultiMesh, no shadows). The ground under them is mud (the crater
##   and the hollow make it so).
## - Smoke of fires: a smoke column (`EffectLibrary` "smoke_column") at
##   every map marker `smoke_<n>`, n = 1, 2, ...
## - Mist: a low, slowly drifting sheet of mist (noise, soft where it meets
##   the ground) around every map marker `mist_<n>`.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

## Water in a crater: this share of the bowl's depth above its bottom, out
## to this share of the bowl's radius (a parabolic bowl: sqrt of the fill).
const CRATER_FILL := 0.4
const CRATER_REACH := 0.66
## Water in a hollow (dug 0.3 m by the generator): above its lowest point.
const HOLLOW_LEVEL_M := 0.12
const HOLLOW_REACH := 0.85
## Mist sheets: radius (m), height above the ground at the marker (m).
const MIST_RADIUS_M := 330.0
const MIST_LIFT_M := 1.6

const WATER_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx, shadows_disabled;

uniform vec4 water : source_color = vec4(0.16, 0.12, 0.075, 1.0);
uniform vec4 shallow : source_color = vec4(0.3, 0.24, 0.16, 1.0);

void fragment() {
	// UV: 0..1 over the disc; the edge thins out into the mud.
	float r = length(UV - vec2(0.5)) * 2.0;
	float edge = 1.0 - smoothstep(0.72, 1.0, r);
	ALBEDO = mix(water.rgb, shallow.rgb, smoothstep(0.4, 1.0, r));
	ROUGHNESS = mix(0.06, 0.4, smoothstep(0.6, 1.0, r));
	SPECULAR = 0.55;
	ALPHA = edge * 0.92;
}
"""

## Crater skins: tiles of instances (frustum culling), subdivisions of the
## disc, lift over the ground (m) and how far they are drawn (m).
const TILE_M := 256.0
const SKIN_SUBDIVISIONS := 11
const SKIN_LIFT_M := 0.09
const SKIN_RANGE_M := 2600.0

const SKIN_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx, shadows_disabled;

uniform sampler2D height_tex : filter_nearest, repeat_disable;
uniform ivec2 samples = ivec2(2, 2);
uniform vec2 half_size = vec2(1.0);
uniform float cell = 2.0;
uniform float lift = 0.07;
// Bowl radius over outer radius (where the rim ends).
uniform float bowl = 0.667;
uniform vec4 bowl_color : source_color = vec4(0.13, 0.1, 0.07, 1.0);
uniform vec4 rim_color : source_color = vec4(0.36, 0.29, 0.2, 1.0);

varying vec2 v_world;

float sample_h(ivec2 s) {
	return texelFetch(height_tex, clamp(s, ivec2(0), samples - 1), 0).r;
}

// The ground as the terrain mesh draws it near the camera: two triangles
// per cell, the diagonal alternating like terrain.gd's chunks.
float ground(vec2 xz) {
	vec2 g = (xz + half_size) / cell;
	ivec2 i = ivec2(floor(g));
	vec2 f = g - vec2(i);
	float h00 = sample_h(i);
	float h10 = sample_h(i + ivec2(1, 0));
	float h01 = sample_h(i + ivec2(0, 1));
	float h11 = sample_h(i + ivec2(1, 1));
	if ((i.x + i.y) % 2 == 0) {
		if (f.x + f.y <= 1.0) {
			return h00 + (h10 - h00) * f.x + (h01 - h00) * f.y;
		}
		return h11 + (h01 - h11) * (1.0 - f.x) + (h10 - h11) * (1.0 - f.y);
	}
	if (f.x >= f.y) {
		return h00 + (h10 - h00) * f.x + (h11 - h10) * f.y;
	}
	return h00 + (h01 - h00) * f.y + (h11 - h01) * f.x;
}

float h1(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vn(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(h1(i), h1(i + vec2(1, 0)), f.x), mix(h1(i + vec2(0, 1)), h1(i + vec2(1, 1)), f.x), f.y);
}

void vertex() {
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_world = w.xz;
	w.y = ground(w.xz) + lift;
	VERTEX = (inverse(MODEL_MATRIX) * vec4(w, 1.0)).xyz;
}

void fragment() {
	float r = length(UV - vec2(0.5)) * 2.0;
	float n = vn(v_world * 1.7) * 0.6 + vn(v_world * 4.3 + 7.0) * 0.4;
	float edge = r + (n - 0.5) * 0.18;
	// Bowl: dark wet mud; rim: lighter thrown-up earth in clods; out to
	// nothing at the end of the rim.
	float in_bowl = 1.0 - smoothstep(bowl - 0.08, bowl + 0.04, edge);
	float rim = smoothstep(bowl - 0.12, bowl + 0.02, edge) * (1.0 - smoothstep(bowl + 0.05, 1.0, edge));
	vec3 c = mix(rim_color.rgb * (0.8 + 0.4 * n), bowl_color.rgb * (0.85 + 0.3 * n), in_bowl);
	ALBEDO = c;
	ROUGHNESS = mix(0.9, 0.45, in_bowl);
	ALPHA = clamp(in_bowl * 0.95 + rim * (0.55 + 0.4 * n), 0.0, 1.0) * (1.0 - smoothstep(0.92, 1.0, r));
}
"""

const MIST_SHADER := """
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

uniform vec4 tint : source_color = vec4(0.66, 0.66, 0.63, 1.0);
uniform float density = 0.16;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;

float h1(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vn(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(h1(i), h1(i + vec2(1, 0)), f.x), mix(h1(i + vec2(0, 1)), h1(i + vec2(1, 1)), f.x), f.y);
}

varying vec3 world;

void vertex() {
	world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 q = world.xz * 0.012 + vec2(TIME * 0.006, TIME * 0.002);
	float n = vn(q) * 0.55 + vn(q * 2.7 + 5.3) * 0.3 + vn(q * 6.1 - 2.1) * 0.15;
	float r = length(UV - vec2(0.5)) * 2.0;
	float outline = 1.0 - smoothstep(0.55, 1.0, r);
	// Soft where the sheet meets the ground (and fades with distance to it).
	float depth = textureLod(depth_tex, SCREEN_UV, 0.0).r;
	vec4 view = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	view.xyz /= view.w;
	float soft = clamp((VERTEX.z - view.z) / 4.0, 0.0, 1.0);
	ALBEDO = tint.rgb;
	ALPHA = density * smoothstep(0.35, 0.8, n) * outline * soft;
}
"""

## Counters for tools and the self-test.
var crater_skins := 0
var puddles := 0
var smoke_columns := 0
var mist_sheets := 0


## Builds everything for the map of `host` (SimHost) at graphics quality
## `level`.
func build(host: Object, level: String, terrain: Node = null) -> void:
	var craters: Array = host.map_craters() if host.has_method("map_craters") else []
	_build_skins(host, craters, terrain)
	_build_puddles(host, craters)
	var settings: Dictionary = GraphicsQuality.settings(level)
	_build_smoke(host, settings)
	_build_mist(host)
	if crater_skins + puddles + smoke_columns + mist_sheets > 0:
		print("crater skins: %d, puddles: %d, smoke columns: %d, mist sheets: %d" % [
			crater_skins, puddles, smoke_columns, mist_sheets])


func _build_skins(host: Object, craters: Array, terrain: Node) -> void:
	if craters.is_empty() or terrain == null:
		return
	var textures: Variant = terrain.get("_textures")
	if not (textures is Dictionary) or not (textures as Dictionary).has("height"):
		return
	var material := ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = SKIN_SHADER
	material.shader = shader
	var size: Vector2 = host.map_size()
	material.set_shader_parameter("height_tex", textures["height"])
	material.set_shader_parameter("samples", host.grid_samples(1))
	material.set_shader_parameter("half_size", size * 0.5)
	material.set_shader_parameter("cell", float(host.cell_size()))
	material.set_shader_parameter("lift", SKIN_LIFT_M)
	var disc := PlaneMesh.new()
	disc.size = Vector2(2.0, 2.0)
	disc.subdivide_width = SKIN_SUBDIVISIONS
	disc.subdivide_depth = SKIN_SUBDIVISIONS
	disc.material = material
	var tiles := {}
	for d: Dictionary in craters:
		var at: Vector3 = d.position
		var key := Vector2i(floori(at.x / TILE_M), floori(at.z / TILE_M))
		if not tiles.has(key):
			tiles[key] = []
		tiles[key].append(d)
	for key: Vector2i in tiles:
		var list: Array = tiles[key]
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = disc
		mm.instance_count = list.size()
		for i in list.size():
			var d: Dictionary = list[i]
			var at: Vector3 = d.position
			var outer: float = d.outer
			var yaw := fmod(at.x * 0.731 + at.z * 1.37, TAU)
			var basis := Basis(Vector3.UP, yaw).scaled(Vector3(outer, 1.0, outer))
			mm.set_instance_transform(i, Transform3D(basis, Vector3(at.x, at.y, at.z)))
		var node := MultiMeshInstance3D.new()
		node.name = "CraterSkins_%d_%d" % [key.x, key.y]
		node.multimesh = mm
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.visibility_range_end = SKIN_RANGE_M
		# The skins follow the ground: a box over the tile's craters, tall
		# enough for any relief.
		node.custom_aabb = AABB(Vector3(key.x * TILE_M - 20.0, -200.0, key.y * TILE_M - 20.0),
			Vector3(TILE_M + 40.0, 400.0, TILE_M + 40.0))
		add_child(node)
		node.global_transform = Transform3D.IDENTITY
		crater_skins += list.size()


func _build_puddles(host: Object, craters: Array) -> void:
	var list: Array = []
	for d: Dictionary in craters:
		if d.water:
			list.append(d)
	if host.has_method("map_puddles"):
		list.append_array(host.map_puddles())
	if list.is_empty():
		return
	var material := ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = WATER_SHADER
	material.shader = shader
	var disc := PlaneMesh.new()
	disc.size = Vector2(2.0, 2.0)
	disc.material = material
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = disc
	mm.instance_count = list.size()
	for i in list.size():
		var d: Dictionary = list[i]
		var at: Vector3 = d.position
		var radius: float = d.radius
		var depth: float = d.get("depth", 0.0)
		var level: float
		var reach: float
		if depth > 0.0:
			level = at.y + depth * CRATER_FILL
			reach = radius * CRATER_REACH
		else:
			level = at.y + HOLLOW_LEVEL_M
			reach = radius * HOLLOW_REACH
		var yaw := float(i % 17) * 0.37
		var basis := Basis(Vector3.UP, yaw).scaled(Vector3(reach, 1.0, reach * (0.85 + 0.03 * float(i % 6))))
		mm.set_instance_transform(i, Transform3D(basis, Vector3(at.x, level, at.z)))
	var node := MultiMeshInstance3D.new()
	node.name = "Puddles"
	node.multimesh = mm
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(node)
	puddles = list.size()


func _build_smoke(host: Object, settings: Dictionary) -> void:
	var n := 1
	while true:
		var at: Variant = host.map_marker("smoke_%d" % n)
		if at == null:
			break
		var smoke := EffectLibrary.build("smoke_column", float(settings.get("fx_particles", 1.0)),
			bool(settings.get("fx_soft", true)), Vector2(0.5, 0.15))
		smoke.name = "Smoke%d" % n
		add_child(smoke)
		smoke.global_position = at as Vector3
		smoke.emitting = true
		n += 1
	smoke_columns = n - 1


func _build_mist(host: Object) -> void:
	var material: ShaderMaterial = null
	var n := 1
	while true:
		var at: Variant = host.map_marker("mist_%d" % n)
		if at == null:
			break
		if material == null:
			material = ShaderMaterial.new()
			var shader := Shader.new()
			shader.code = MIST_SHADER
			material.shader = shader
		var sheet := MeshInstance3D.new()
		sheet.name = "Mist%d" % n
		var plane := PlaneMesh.new()
		plane.size = Vector2(MIST_RADIUS_M * 2.0, MIST_RADIUS_M * 2.0)
		sheet.mesh = plane
		sheet.material_override = material
		sheet.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(sheet)
		sheet.global_position = (at as Vector3) + Vector3(0.0, MIST_LIFT_M, 0.0)
		n += 1
	mist_sheets = n - 1
