// Map surround water (prompt 222): lightweight sea/river continuation beyond
// coastal and river map borders. Presentation only.
//
// No screen-texture refraction pass (zero extra screen copy): uses the biome's
// shallow/deep/shore colors and the same animated wind ripple normals as
// `water.gdshader` so ocean and river exits flow seamlessly out into the haze.
shader_type spatial;
render_mode blend_mix, depth_draw_always, cull_back, specular_schlick_ggx;

#include "res://assets/shaders/cloud_shadow.gdshaderinc"

uniform sampler2D normal_a : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D normal_b : hint_normal, filter_linear_mipmap, repeat_enable;

uniform vec4 shallow_color : source_color = vec4(0.08, 0.30, 0.50, 1.0);
uniform vec4 deep_color : source_color = vec4(0.03, 0.14, 0.34, 1.0);
uniform vec4 shore_color : source_color = vec4(0.42, 0.66, 0.78, 1.0);
uniform float shore_depth = 1.2;
uniform float normal_tile = 7.0;
uniform float ripple_tile = 31.0;
uniform float normal_strength = 0.50;
uniform float surround_desat = 0.16;
uniform float surround_dim = 0.92;

varying vec3 v_world;
// x = outward distance from map border (m), y = water depth over surround bed (m)
varying vec2 v_dist_depth;

vec3 unpack(vec4 n) {
	return vec3(n.xy * 2.0 - 1.0, 0.0);
}

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_dist_depth = UV;
}

void fragment() {
	CLOUD_FRAGMENT(0.35)
	vec2 p = v_world.xz;
	float d = v_dist_depth.x;
	float depth = max(v_dist_depth.y, 0.0);

	// Two gentle wind-driven ripple layers (world-space coordinates only).
	vec3 n0 = unpack(texture(normal_a, p / normal_tile + TIME * vec2(0.018, 0.011)));
	vec3 n1 = unpack(texture(normal_b, p / ripple_tile - TIME * vec2(0.011, 0.007)));
	vec3 n = (n0 * 0.55 + n1 * 0.55) * normal_strength;
	// Calm ripples gently in the far distance against aliasing.
	float cam_dist = length(v_world - CAMERA_POSITION_WORLD);
	n.xy *= mix(1.0, 0.35, smoothstep(120.0, 700.0, cam_dist));
	vec3 world_n = normalize(vec3(n.x, 1.0, n.y));
	NORMAL = normalize((VIEW_MATRIX * vec4(world_n, 0.0)).xyz);

	float deep = 1.0 - exp(-depth / 1.2);
	vec3 water = mix(shallow_color.rgb, deep_color.rgb, deep);
	float shore = pow(1.0 - smoothstep(0.0, shore_depth, depth), 1.5) * 0.75;
	vec3 col = mix(water, shore_color.rgb, shore);

	// Slight calm desaturation beyond the border without a bright seam line.
	float out_fade = smoothstep(0.0, 28.0, d);
	float luma = dot(col, vec3(0.2126, 0.7152, 0.0722));
	col = mix(col, vec3(luma), surround_desat * out_fade);
	col *= mix(1.0, surround_dim, out_fade);

	ALBEDO = col;
	ROUGHNESS = 0.09;
	METALLIC = 0.0;
	SPECULAR = 0.35;
	ALPHA = 1.0;
}
