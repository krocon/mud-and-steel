// Water surface of rivers and lakes (prompt 014). Drawn by
// scenes/world/water.gd on a mesh built in Rust (crates/bridge/src/water.rs):
// one vertex per map sample in and next to the water.
//
// Vertex data: UV = flow velocity (x, z) in m/s, UV2 = (distance to the
// shore or a stone in m, water depth in m; negative on the land ring).
//
// - Flow map: two normal samples shifted along the flow with phases half a
//   cycle apart, blended so neither jumps when its phase wraps; plus a slow
//   large ripple layer that does not depend on the flow.
// - Foam: near the banks and stones, in very shallow water and where the
//   screen depth shows the water getting thin; the foam pattern is shifted
//   along the flow like the normals.
// - Color: refraction of the opaque scene (screen texture, offset by the
//   normals, not where that would pick up something above the water), then
//   absorption over the view distance through the water (screen depth)
//   towards the shallow and the deep color of the biome.
// - Reflection: Godot's own sky radiance and sun specular on a smooth
//   surface (Fresnel included); no screen-space reflections.
shader_type spatial;
render_mode blend_mix, depth_draw_always, cull_back, specular_schlick_ggx;

// Clouds dim the direct sunlight (prompt 087): light() of the include.
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap, repeat_disable;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest, repeat_disable;
uniform sampler2D normal_a : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D normal_b : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D foam_tex : filter_linear_mipmap, repeat_enable;

// Biome look (BiomePreset, water group).
uniform vec4 shallow_color : source_color = vec4(0.08, 0.30, 0.50, 1.0);
uniform vec4 deep_color : source_color = vec4(0.03, 0.14, 0.34, 1.0);
uniform vec4 foam_color : source_color = vec4(0.85, 0.88, 0.84, 1.0);
// Meters of water after which 63 % of the green light is absorbed; red
// goes a little farther, blue much less (humic river water).
uniform float clarity = 0.8;
// How far each color reaches through the water, relative to `clarity`:
// red is absorbed first, so the bed shows through bluish (prompt 091).
uniform vec3 absorb_tint = vec3(0.45, 0.85, 1.0);
// Pale band along every bank (prompt 091): full at the water's edge, gone
// at `shore_depth` meters of water or `shore_width` meters from the bank.
uniform vec4 shore_color : source_color = vec4(0.42, 0.66, 0.78, 1.0);
uniform float shore_depth = 1.2;
uniform float shore_width = 7.0;

uniform float normal_tile = 7.0;
uniform float ripple_tile = 31.0;
uniform float normal_strength = 0.55;
// Seconds per flow-map phase.
uniform float cycle = 2.4;
uniform float foam_tile = 3.2;
uniform float refraction = 0.035;

varying vec2 v_flow;
varying float v_shore;
varying float v_depth;
varying vec3 v_world;

void vertex() {
	v_flow = UV;
	v_shore = UV2.x;
	v_depth = UV2.y;
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

vec3 unpack(vec4 n) {
	return vec3(n.xy * 2.0 - 1.0, 0.0);
}

// Linear view-space distance of the opaque scene at `uv`.
float scene_distance(vec2 uv, mat4 inv_projection) {
	float depth = texture(depth_tex, uv).r;
	vec4 view = inv_projection * vec4(uv * 2.0 - 1.0, depth, 1.0);
	return length(view.xyz / view.w);
}

void fragment() {
	CLOUD_FRAGMENT(0.35)
	vec2 p = v_world.xz;
	float speed = length(v_flow);
	// Flow map: two phases of the same pattern, shifted along the flow.
	float phase0 = fract(TIME / cycle);
	float phase1 = fract(TIME / cycle + 0.5);
	float w0 = 1.0 - abs(1.0 - 2.0 * phase0);
	vec2 shift0 = v_flow * phase0 * cycle;
	vec2 shift1 = v_flow * phase1 * cycle;
	vec3 n0 = unpack(texture(normal_a, (p - shift0) / normal_tile));
	vec3 n1 = unpack(texture(normal_a, (p - shift1) / normal_tile + vec2(0.37, 0.61)));
	vec3 flow_n = mix(n1, n0, w0);
	// Slow large ripples (wind), independent of the flow.
	vec3 ripple = unpack(texture(normal_b, p / ripple_tile + TIME * vec2(0.011, 0.007)));
	vec3 n = flow_n * (0.6 + 0.6 * clamp(speed, 0.0, 1.0)) + ripple * 0.5;
	n.xy *= normal_strength;
	vec3 world_n = normalize(vec3(n.x, 1.0, n.y));
	NORMAL = normalize((VIEW_MATRIX * vec4(world_n, 0.0)).xyz);

	// Thickness of the water along the view ray (screen depth).
	float surface = length(VERTEX);
	float behind = scene_distance(SCREEN_UV, INV_PROJECTION_MATRIX);
	float thickness = max(behind - surface, 0.0);
	// Refraction, unless the offset point lies in front of the water.
	vec2 offset = world_n.xz * refraction * clamp(thickness, 0.0, 1.0);
	vec2 uv = SCREEN_UV + offset;
	float behind_off = scene_distance(uv, INV_PROJECTION_MATRIX);
	if (behind_off < surface) {
		uv = SCREEN_UV;
		behind_off = behind;
	}
	float through = max(behind_off - surface, 0.0);
	vec3 below = textureLod(screen_tex, uv, 0.0).rgb;
	vec3 transmit = exp(-through / (max(clarity, 0.01) * absorb_tint));
	float deep = 1.0 - exp(-max(v_depth, 0.0) / 1.2);
	vec3 water = mix(shallow_color.rgb, deep_color.rgb, deep);
	// Shore band from the bed (depth, distance to the bank), not from the
	// screen depth: it must not change with the view or the zoom.
	float shore = (1.0 - smoothstep(0.0, shore_depth, max(v_depth, 0.0)))
		* (1.0 - smoothstep(0.0, shore_width, max(v_shore, 0.0)));
	shore = pow(shore, 1.5) * 0.85;

	// Foam: banks and stones, very shallow water, thin water on screen.
	float edge = smoothstep(2.4, 0.2, v_shore) * 0.85 + smoothstep(0.15, 0.0, v_depth) * 0.6
		+ smoothstep(0.2, 0.0, thickness) * 0.4;
	vec2 fshift0 = v_flow * phase0 * cycle;
	vec2 fshift1 = v_flow * phase1 * cycle;
	float f0 = texture(foam_tex, (p - fshift0) / foam_tile).r;
	float f1 = texture(foam_tex, (p - fshift1) / foam_tile + vec2(0.5, 0.25)).r;
	float pattern = mix(f1, f0, w0);
	// Streaks behind stones and along fast water.
	float streak = texture(foam_tex, (p - v_flow * TIME * 0.6) / vec2(foam_tile * 0.6)).r;
	float foam = clamp(edge, 0.0, 1.0) * smoothstep(0.5, 0.8, pattern + edge * 0.2) * 0.85;
	foam = max(foam, smoothstep(0.8, 0.95, streak) * clamp(speed - 0.45, 0.0, 1.0) * 0.5);

	// The lit part is the water's own color; the scene below comes in as
	// emission so it is not lit twice.
	vec3 own = mix(water * (1.0 - transmit.g), shore_color.rgb, shore);
	ALBEDO = mix(own, foam_color.rgb, foam);
	EMISSION = below * transmit * (1.0 - foam) * (1.0 - shore);
	ROUGHNESS = mix(0.09, 0.7, foam);
	METALLIC = 0.0;
	SPECULAR = 0.35;
	ALPHA = 1.0;
}
