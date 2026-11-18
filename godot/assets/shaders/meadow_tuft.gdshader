// Grass tufts of the meadow (prompt 057): single blades as thin opaque
// triangles (scenes/world/meadow_tufts.gd builds the meshes), drawn as
// MultiMesh decoration by map_objects.gd in the forest biome.
//
// The color comes from the same palette as the ground under the tuft
// (meadow_palette.gdshaderinc, read per instance in the vertex shader), so
// tufts melt into the meadow: dark at the root, lighter and a little
// yellower at the tip. The normals point mostly up (set by the mesh), so
// blades are lit like the ground and show no dark back faces.
//
// Vertex color: r = height on the blade (0 root .. 1 tip), g = wind weight,
// b = random shade per blade.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;

// Clouds dim the direct sunlight (prompt 087): light() of the include.
#define CLOUD_BACKLIGHT
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;
uniform float wind_strength = 0.25;
uniform vec2 wind_direction = vec2(1.0, 0.3);
uniform float wind_speed = 1.2;

#include "res://assets/shaders/meadow_palette.gdshaderinc"

varying vec3 v_base;
varying float v_t;
varying float v_shade;

void vertex() {
	vec3 inst = MODEL_MATRIX[3].xyz;
	vec2 p = inst.xz;
	float dry;
	// The terrain's macro_far is macro_tex at p / 420.
	float far_g = textureLod(macro_tex, p / 420.0, 0.0).g;
	vec4 m_mid = meadow_mid(p);
	vec4 m_small = meadow_small(p);
	v_base = meadow_base(p, far_g, m_mid, m_small, m_vnoise(p / 0.3), dry);
	// Every tuft a little different: some drier, some darker.
	vec2 rh = m_hash2(p * 7.31);
	v_base = mix(v_base, meadow_dry.rgb, rh.x * rh.x * 0.6) * mix(0.8, 1.15, rh.y);
	v_t = COLOR.r;
	v_shade = COLOR.b;
	// Wind: sway of the whole tuft and a flutter per blade, more at the tip.
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	float t = TIME * wind_speed + dot(p, vec2(0.13, 0.07));
	float sway = sin(t) * 0.6 + sin(t * 2.3 + 1.1) * 0.25;
	float flutter = sin(TIME * 6.0 + COLOR.b * 6.2831 + p.x * 3.0) * 0.15;
	float h = COLOR.g * COLOR.g;
	vec3 off = vec3(dir.x, 0.0, dir.y) * (sway + flutter) * h * wind_strength * 0.35;
	// Offset in world space, turned into the instance's local frame.
	VERTEX += (inverse(mat3(MODEL_MATRIX)) * off);
}

void fragment() {
	CLOUD_FRAGMENT(0.3)
	vec3 tip = v_base * vec3(1.45, 1.38, 1.0);
	vec3 c = mix(v_base * 0.6, tip, smoothstep(0.0, 0.8, v_t));
	ALBEDO = c * mix(0.85, 1.12, v_shade);
	ROUGHNESS = 0.85;
	SPECULAR = 0.3;
	// Roots sit in the shade of the tuft.
	AO = mix(0.55, 1.0, v_t);
	AO_LIGHT_AFFECT = 0.3;
	BACKLIGHT = c * 0.25;
}
