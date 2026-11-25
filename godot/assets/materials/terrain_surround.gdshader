// Map surround terrain (prompt 222): low-cost ground skirt continuing out from
// the four map edges into the distance haze. Presentation only.
//
// Vertex: Ring 0 vertices (UV2.x >= 0) sit on the exact map border samples and
// snap their height to the adjacent playable chunk's current LOD chord
// (`edge_lod_tex`), closing every seam at every LOD level and distance.
//
// Fragment: world-space macro biome look (no meadow blades, no grass clumps,
// no tall grass, no tracks, no puddles) matching the playable edge at d = 0,
// then gently calming and desaturating beyond the map border with a soft,
// antialiased boundary line so the playable area stays clearly readable.
shader_type spatial;
render_mode cull_back;

#define CLOUD_SUN_IN_FRAGMENT
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

const int LAYER_COUNT = 10;
const int CHUNK_CELLS = 64;

uniform sampler2D height_tex : filter_nearest, repeat_disable;
uniform sampler2D edge_lod_tex : filter_nearest, repeat_disable;
uniform ivec2 samples = ivec2(2, 2);
uniform ivec2 chunks = ivec2(1, 1);

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;

// Biome look (set by map_surround.gd from BiomePreset).
uniform bool use_biome = false;
uniform int layer_source[LAYER_COUNT];
uniform vec4 layer_tint[LAYER_COUNT] : source_color;
uniform float meadow_strength = 1.0;
uniform float grass_variation = 1.0;
uniform float snow_cover = 0.0;
uniform float wetness = 0.0;
uniform float mountain = 0.0;
uniform float snow_line = 1000000.0;
uniform float snow_line_fade = 25.0;
// Desaturation and darkening of the non-playable surround so the playable map
// border reads cleanly even with fog of war off (editor, replay, HUD off).
uniform float surround_desat = 0.24;
uniform float surround_dim = 0.88;
uniform vec3 border_line_color : source_color = vec3(0.55, 0.50, 0.38);
uniform float border_line_strength = 0.22;

#include "res://assets/shaders/meadow_palette.gdshaderinc"

varying vec3 v_world;
varying vec3 v_normal_w;
// x = outward distance from map border (m), y = water bed mask (0..1)
varying vec2 v_dist_water;
// r = sand, g = earth/track, b = rock/slope, a = extra dry/field blend
varying vec4 v_mat;

float edge_height(int edge_id, int idx) {
	ivec2 s;
	if (edge_id == 0) {
		s = ivec2(clamp(idx, 0, samples.x - 1), 0);
	} else if (edge_id == 1) {
		s = ivec2(samples.x - 1, clamp(idx, 0, samples.y - 1));
	} else if (edge_id == 2) {
		s = ivec2(clamp(idx, 0, samples.x - 1), samples.y - 1);
	} else {
		s = ivec2(0, clamp(idx, 0, samples.y - 1));
	}
	return texelFetch(height_tex, s, 0).r;
}

void vertex() {
	vec3 pos = VERTEX;
	int edge_id = int(round(UV2.x));
	if (edge_id >= 0) {
		int idx = int(round(UV2.y));
		bool horizontal = (edge_id == 0 || edge_id == 2);
		int last_s = (horizontal ? samples.x : samples.y) - 1;
		int max_c = (horizontal ? chunks.x : chunks.y) - 1;
		int c = min(idx / CHUNK_CELLS, max_c);
		int stride = max(int(round(texelFetch(edge_lod_tex, ivec2(c, edge_id), 0).r * 255.0)), 1);
		int s0 = c * CHUNK_CELLS;
		int t = idx - s0;
		int t0 = (t / stride) * stride;
		int a = min(s0 + t0, last_s);
		int b = min(s0 + t0 + stride, last_s);
		float f = (b > a) ? float(idx - a) / float(b - a) : 0.0;
		pos.y = mix(edge_height(edge_id, a), edge_height(edge_id, b), f);
	}
	VERTEX = pos;
	v_world = (MODEL_MATRIX * vec4(pos, 1.0)).xyz;
	v_normal_w = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_dist_water = UV;
	v_mat = COLOR;
}

vec2 rot2(vec2 p, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

vec3 sample_layer(vec2 p, int layer, float tile) {
	float src = use_biome ? float(layer_source[layer]) : float(layer);
	vec3 a = texture(albedo_array, vec3(p / tile, src)).rgb;
	vec3 b = texture(albedo_array, vec3(rot2(p, 1.1 + float(layer)) / (tile * 4.5) + vec2(0.31, 0.71), src)).rgb;
	vec3 c = mix(a, b, 0.55);
	if (use_biome) {
		c *= layer_tint[layer].rgb;
	}
	return c;
}

void fragment() {
	CLOUD_FRAGMENT(0.35)
	cloud_share = cloud_sun(v_world);
	vec2 p = v_world.xz;
	float d = v_dist_water.x;
	float water_mask = v_dist_water.y;

	vec4 macro_far = texture(macro_tex, p / 420.0);
	vec4 macro_near = texture(macro_tex, p / 53.0 + 0.37);

	vec3 n_w = normalize(v_normal_w);
	float grade = length(n_w.xz) / max(n_w.y, 0.05);

	// Base biome ground color: uses the shared meadow palette in grassy biomes
	// so the surround joins the playable meadow without a hue jump, keeping
	// fine world-space grain from the base texture so the near border stays crisp.
	vec3 base = sample_layer(p, 0, 4.0);
	base *= mix(0.82, 1.14, macro_far.r) * mix(0.93, 1.05, macro_near.g);
	if (meadow_strength > 0.0) {
		float dry = 0.0;
		vec3 m_col = meadow_base(p, macro_far.g, meadow_mid(p), vec4(0.5), 0.5, dry);
		float src0 = use_biome ? float(layer_source[0]) : 0.0;
		vec3 tint0 = use_biome ? layer_tint[0].rgb : vec3(1.0);
		float mean_luma = max(m_luma(textureLod(albedo_array, vec3(0.5, 0.5, src0), 12.0).rgb * tint0), 0.05);
		float tex_luma = clamp(m_luma(base) / mean_luma, 0.62, 1.38);
		base = mix(base, m_col * mix(1.0, tex_luma, 0.72), meadow_strength);
	} else {
		float greenness = clamp((base.g - max(base.r, base.b)) * 25.0, 0.0, 1.0);
		vec3 dry_c = base * vec3(1.35, 1.08, 0.7);
		vec3 lush_c = base * vec3(0.8, 0.95, 0.75);
		base = mix(base, dry_c, smoothstep(0.5, 0.8, macro_far.g) * 0.6 * greenness * grass_variation);
		base = mix(base, lush_c, smoothstep(0.45, 0.2, macro_far.g) * 0.5 * greenness * grass_variation);
	}

	// Sand / beach continuation along coastal borders.
	float sand_w = clamp(v_mat.r, 0.0, 1.0);
	if (sand_w > 0.01) {
		vec3 sand = sample_layer(p, 3, 10.0) * mix(0.90, 1.08, macro_far.r);
		base = mix(base, sand, sand_w);
	}

	// Bare earth / slope / rock continuation (slightly gentler threshold than
	// playable hills so grassy coastal dunes stay grassy).
	vec2 earth_g = vec2(0.28, 0.46) + mountain * vec2(0.20, 0.24);
	vec2 rock_g = vec2(0.48, 0.68) + mountain * vec2(0.16, 0.20);
	float earth_w = clamp(max(smoothstep(earth_g.x, earth_g.y, grade), v_mat.g), 0.0, 1.0);
	float rock_w = clamp(max(smoothstep(rock_g.x, rock_g.y, grade + (macro_near.r - 0.5) * 0.1), v_mat.b), 0.0, 1.0);
	if (earth_w > 0.01) {
		vec3 earth = sample_layer(p, 9, 8.0) * mix(0.85, 1.12, macro_far.r);
		base = mix(base, earth, earth_w * (1.0 - rock_w));
	}
	if (rock_w > 0.01) {
		vec3 rock = sample_layer(p, 8, 14.0) * mix(0.85, 1.12, macro_far.b);
		if (mountain > 0.0) {
			float band = sin(v_world.y * 0.9 + macro_near.r * 4.0 + macro_far.g * 9.0);
			float band2 = sin(v_world.y * 0.23 + macro_far.b * 6.0);
			rock *= mix(1.0, 0.86 + 0.1 * band + 0.12 * band2, mountain);
		}
		base = mix(base, rock, rock_w);
	}

	// Subtle seabed darkening under coastal surround water.
	if (water_mask > 0.01) {
		base *= mix(1.0, 0.45, smoothstep(0.0, 0.6, water_mask));
	}

	// Rain wetness.
	if (wetness > 0.0) {
		base *= mix(1.0, 0.65, wetness);
	}

	// Snow cover (winter biome and alpine snow line).
	float alpine = smoothstep(snow_line - snow_line_fade, snow_line + snow_line_fade,
		v_world.y + (macro_far.b - 0.5) * snow_line_fade * 2.0);
	if (snow_cover > 0.0 || alpine > 0.0) {
		float flat_g = smoothstep(0.55, 0.2, grade);
		float s = max(snow_cover * flat_g, alpine * smoothstep(1.1, 0.6, grade)) * (1.0 - water_mask);
		s = smoothstep(0.25, 0.65, s + (macro_near.r - 0.5) * 0.45 + (macro_far.b - 0.5) * 0.25);
		vec3 snow = sample_layer(p, 5, 10.0);
		base = mix(base, snow, s);
	}

	// Calm visual distinction of the non-playable surround + subtle antialiased
	// ground line along the playable border (d = 0..1.4 m).
	float out_fade = smoothstep(0.0, 14.0, d);
	float luma = dot(base, vec3(0.2126, 0.7152, 0.0722));
	base = mix(base, vec3(luma), surround_desat * out_fade);
	base *= mix(1.0, surround_dim, out_fade);

	float fw = max(fwidth(d), 0.15);
	float line_band = (1.0 - smoothstep(0.0, max(1.1, fw * 1.4), d)) * (1.0 - water_mask);
	float cam_dist = length(v_world - CAMERA_POSITION_WORLD);
	float line_vis = (1.0 - smoothstep(180.0, 650.0, cam_dist));
	base = mix(base, border_line_color, line_band * border_line_strength * line_vis);

	ALBEDO = base;
	ROUGHNESS = mix(0.88, 0.42, wetness * 0.6);
	SPECULAR = 0.28;
}
