// Ground scatter (prompt 191): low shrubs, fern and herb clumps, dry
// stalks and field stones, drawn as MultiMesh decoration by map_objects.gd
// (layers "scatter_*"; meshes from scenes/world/ground_scatter.gd).
// Presentation only.
//
// Vertex color: r = height share on the plant (0 ground .. 1 top), g = wind
// weight, b = random shade, a = part (see KIND_*).
//
// Per instance in the vertex shader:
// - color from the meadow palette at the instance (meadow_palette.gdshaderinc),
//   so the scatter follows the ground's lush and dry areas; herbs keep to
//   the lush and damp areas, stalks to the dry ones (they shrink away
//   elsewhere), like the moisture the ground color stands for;
// - shrinking into the ground towards `scatter_range` from the game camera
//   (also in shadow passes), so nothing pops in or out;
// - vehicle tracks (ground_tracks.gd raster, same as the terrain's): shrubs
//   are pressed flat and brown, everything else disappears under them.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
// Shrubs hide units like grass (unit_xray.gdshader, prompt 165).
stencil_mode write, compare_always, 1;

#define CLOUD_BACKLIGHT
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;
uniform float wind_strength = 0.25;
uniform vec2 wind_direction = vec2(1.0, 0.3);
uniform float wind_speed = 1.2;
// Biome: 0 temperate meadow, 1 desert, 2 winter (ground_scatter.gd apply_look).
uniform int scatter_look = 0;
// Distance from the game camera where the scatter has shrunk to nothing (m).
uniform float scatter_range = 150.0;
// Vehicle tracks (prompt 104): R ruts, G churned, B worn; world rectangle.
uniform sampler2D scatter_tracks : hint_default_black, filter_linear, repeat_disable;
uniform vec2 scatter_track_origin = vec2(0.0);
uniform vec2 scatter_track_size = vec2(1.0);
uniform bool scatter_has_tracks = false;

#include "res://assets/shaders/meadow_palette.gdshaderinc"

const float KIND_SHRUB = 0.0;
const float KIND_HERB = 0.25;
const float KIND_STALK = 0.5;
const float KIND_HEAD = 0.75;
const float KIND_STONE = 1.0;

varying vec3 v_color;
varying vec3 v_local;
varying float v_t;
varying float v_kind;
varying float v_crushed;

bool is_kind(float a, float k) {
	return abs(a - k) < 0.12;
}

// How much a vehicle drove over `p` (0..1): the ruts' raster read across
// the width of a hull (the ruts are only the two tracks).
float crushed_at(vec2 p) {
	if (!scatter_has_tracks) {
		return 0.0;
	}
	float k = 0.0;
	for (int i = 0; i < 5; i++) {
		vec2 o = i == 0 ? vec2(0.0) : vec2(i == 1 ? 0.9 : (i == 2 ? -0.9 : 0.0), i == 3 ? 0.9 : (i == 4 ? -0.9 : 0.0));
		vec2 uv = (p + o - scatter_track_origin) / scatter_track_size;
		if (uv.x > 0.0 && uv.y > 0.0 && uv.x < 1.0 && uv.y < 1.0) {
			vec3 t = textureLod(scatter_tracks, uv, 0.0).rgb;
			k = max(k, max(t.r * 1.6, max(t.g, t.b * 0.8)));
		}
	}
	return smoothstep(0.12, 0.45, k);
}

void vertex() {
	vec3 inst = MODEL_MATRIX[3].xyz;
	vec2 p = inst.xz;
	v_t = COLOR.r;
	v_kind = COLOR.a;
	v_local = VERTEX;
	vec2 rh = m_hash2(p * 3.17);

	// Ground color at the instance and its dryness (same as the terrain).
	float dry;
	float far_g = textureLod(macro_tex, p / 420.0, 0.0).g;
	vec4 m_mid = meadow_mid(p);
	vec3 meadow = meadow_base(p, far_g, m_mid, meadow_small(p), 0.5, dry);
	float stone_tone = mix(0.8, 1.15, rh.x);
	vec3 c;
	float keep = 1.0;
	if (is_kind(COLOR.a, KIND_SHRUB)) {
		// Dark, a little bluer than the grass; dry areas browner.
		c = meadow * vec3(0.34, 0.4, 0.36) * mix(0.75, 1.1, rh.y);
		c = mix(c, vec3(0.13, 0.12, 0.07), dry * 0.4);
	} else if (is_kind(COLOR.a, KIND_HERB)) {
		// Fern and dock leaves: fresher green, only where the meadow is not dry.
		c = mix(meadow * vec3(0.7, 0.9, 0.55), vec3(0.14, 0.24, 0.07), 0.45 * rh.y);
		keep = 1.0 - smoothstep(0.35, 0.7, dry);
	} else if (is_kind(COLOR.a, KIND_STALK)) {
		// Dry stalks and dead leaves: pale straw, more in dry areas.
		c = mix(vec3(0.62, 0.55, 0.38), vec3(0.5, 0.42, 0.28), rh.y);
		keep = mix(0.45, 1.0, smoothstep(0.1, 0.5, dry));
	} else if (is_kind(COLOR.a, KIND_HEAD)) {
		// Seed heads of dock and thistle: rust brown to dark brown.
		c = mix(vec3(0.36, 0.2, 0.1), vec3(0.2, 0.14, 0.1), rh.x);
		keep = mix(0.45, 1.0, smoothstep(0.1, 0.5, dry));
	} else {
		c = vec3(0.19, 0.18, 0.16) * stone_tone;
	}
	if (scatter_look == 1) {
		// Desert: grey-olive scrub, bleached stalks, sandy stones.
		c = is_kind(COLOR.a, KIND_STONE) ? vec3(0.62, 0.5, 0.38) * stone_tone
			: mix(vec3(dot(c, vec3(0.33))), vec3(0.5, 0.46, 0.34), 0.6);
	} else if (scatter_look == 2) {
		// Winter: bare dark twigs and dead stalks out of the snow.
		c = is_kind(COLOR.a, KIND_STONE) ? vec3(0.36, 0.36, 0.37) * stone_tone
			: mix(vec3(dot(c, vec3(0.33))), vec3(0.3, 0.24, 0.18), 0.6);
	}
	v_color = c;

	// Shrink towards the range (distance to the game camera, also in the
	// shadow passes) and where the plant does not grow (keep).
	float dist = length(inst - MAIN_CAM_INV_VIEW_MATRIX[3].xyz);
	float shrink = 1.0 - smoothstep(scatter_range * 0.72, scatter_range, dist);
	// Keep is per instance: a random threshold, not a smaller plant.
	shrink *= step(rh.x * 0.999, keep);
	v_crushed = crushed_at(p);
	if (is_kind(COLOR.a, KIND_SHRUB)) {
		// Pressed flat: lower and wider.
		VERTEX.y *= mix(1.0, 0.12, v_crushed);
		VERTEX.xz *= mix(1.0, 1.35, v_crushed);
	} else {
		shrink *= 1.0 - v_crushed;
	}
	VERTEX *= shrink;

	// Wind: plants sway with the height, stones stand still.
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	float t = TIME * wind_speed + dot(p, vec2(0.13, 0.07));
	float sway = sin(t) * 0.6 + sin(t * 2.1 + rh.y * 6.0) * 0.3;
	float h = COLOR.g * COLOR.g * (1.0 - v_crushed);
	vec3 off = vec3(dir.x, 0.0, dir.y) * sway * h * wind_strength * 0.22;
	VERTEX += inverse(mat3(MODEL_MATRIX)) * off;
}

void fragment() {
	CLOUD_FRAGMENT(0.3)
	vec3 c = v_color;
	float ao = 1.0;
	if (is_kind(v_kind, KIND_SHRUB)) {
		// Leaf speckle on the model (sticks to it), dark inside and below.
		float leaf = m_vnoise(v_local.xz * 22.0 + v_local.y * 17.0);
		c *= mix(0.45, 1.35, leaf * leaf);
		ao = mix(0.35, 1.0, smoothstep(0.0, 0.9, v_t));
		c = mix(c, vec3(0.24, 0.18, 0.1) * 0.6, v_crushed * 0.6);
	} else if (is_kind(v_kind, KIND_STONE)) {
		float grain = m_vnoise(v_local.xz * 40.0 + v_local.y * 31.0);
		c *= mix(0.85, 1.1, grain);
		ao = mix(0.5, 1.0, smoothstep(0.0, 0.6, v_t));
		if (scatter_look == 2) {
			// Snow on the tops of stones.
			c = mix(c, vec3(0.9, 0.92, 0.96), smoothstep(0.55, 0.85, v_t));
		}
	} else {
		ao = mix(0.5, 1.0, v_t);
		c *= mix(0.75, 1.1, v_t);
	}
	ALBEDO = c;
	ROUGHNESS = is_kind(v_kind, KIND_STONE) ? 0.9 : 0.85;
	SPECULAR = 0.3;
	AO = ao;
	AO_LIGHT_AFFECT = 0.3;
	BACKLIGHT = is_kind(v_kind, KIND_STONE) ? vec3(0.0) : c * 0.2;
}
