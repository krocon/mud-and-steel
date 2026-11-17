// Opaque environment material: buildings, props, rocks, trunks, fortifications.
//
// One ShaderMaterial per slot (godot/assets/materials/env/*.tres, assigned on
// import by assets/import/unit_post_import.gd for models with env_kit_version).
// Each slot has up to four material kinds (vertex color A, see
// tools/blender/kit/env.py SLOT_KINDS); all patterns are procedural, there are
// no texture files. Biome look via instance uniforms (EnvLook.apply()).
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

#include "res://assets/shaders/env_common.gdshaderinc"

// 0 Paint, 1 Metal, 2 Rubber, 3 Wood, 4 Canvas, 5 Glass,
// 10 Bark, 12 Stone, 13 Plaster, 14 Ground, 15 Roof, 16 Rock
uniform int slot = 12;

varying vec3 v_local;
// World position (Rock: the pattern runs on across neighbouring blocks).
varying vec3 v_world;
varying vec3 v_world_normal;
// Normal of the standing model: snow stays on fallen trees (prompt 056).
varying vec3 v_rest_normal;
// Per object, from its position (prompt 056): picks the paint of plaster,
// the roof covering and tints of brick and boards, so no two neighbouring
// houses look alike. Deterministic: the same house always looks the same.
varying float v_seed;

void vertex() {
	v_local = VERTEX;
	vec3 inst = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	if (slot == 10 && COLOR.g > 0.0) {
		VERTEX += wind_offset(VERTEX, COLOR.g, COLOR.b, inst, TIME) * 0.5 * (1.0 - env_fallen);
	}
	v_world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_rest_normal = normalize(NORMAL);
	v_seed = hash2(floor(inst.xz * 0.5) + vec2(0.5, 0.5));
}

// Paint of plaster walls (prompt 056): whitewash, cream, pale yellow, light
// ochre, pale pink, light grey-green, grey (a palette of rural Central
// European houses, judgement values).
vec3 plaster_paint(float seed) {
	vec3 c[7] = {vec3(0.88, 0.87, 0.83), vec3(0.84, 0.79, 0.66), vec3(0.86, 0.78, 0.52),
		vec3(0.78, 0.62, 0.40), vec3(0.84, 0.70, 0.64), vec3(0.70, 0.74, 0.66), vec3(0.66, 0.66, 0.63)};
	return c[int(floor(seed * 7.0)) % 7];
}

// Grown rock (prompt 074): the base color of the terrain's rock layer
// (gen_textures.rs `rock`, about sRGB 107/102/92) times the biome's
// rock tint (godot/assets/biomes/*.tres layer_tint[8]), so rock objects on
// a rock slope read as one piece. The biome is recognized by the look
// uniforms: dust = desert, snow = winter.
vec3 rock_base() {
	vec3 base = vec3(0.147, 0.133, 0.107);
	vec3 tint = mix(vec3(1.0), vec3(1.12, 0.98, 0.82), clamp(env_dust / 0.45, 0.0, 1.0));
	tint = mix(tint, vec3(0.85, 0.88, 0.92), clamp(env_snow / 0.85, 0.0, 1.0));
	return base * tint;
}

// Rock pattern in world space: bright warm tops, cooler darker flanks
// (by the normal), large tone patches, bedding bands on the flanks,
// cracks, dark water streaks running down, lichen on top (not in the
// desert), a darker foot. `up` = world normal y, `h` = height above the
// model's ground (m).
vec3 rock_albedo(vec3 w, float up, float h, int kind) {
	vec3 base = rock_base();
	float top = smoothstep(0.3, 0.85, up);
	vec3 col = base * mix(vec3(0.95, 0.98, 1.04), vec3(1.95, 1.85, 1.66), top);
	float tone = fbm3(w * 0.33);
	col *= 0.78 + 0.44 * tone;
	col *= 0.9 + 0.2 * vnoise(w * 4.3);
	// bedding: horizontal bands on the flanks
	float band = sin(w.y * 2.1 + tone * 4.0) * 0.5 + 0.5;
	col *= mix(1.0, 0.86 + 0.2 * band, 1.0 - top);
	// cracks: thin dark lines where a noise crosses its middle
	float cr = abs(vnoise(w * vec3(0.8, 1.3, 0.8) + vec3(3.1, 0.0, 7.7)) * 2.0 - 1.0);
	float cr2 = abs(vnoise(w * 2.2 + vec3(11.0, 5.0, 2.0)) * 2.0 - 1.0);
	float crack = (1.0 - smoothstep(0.0, 0.045, cr)) + (1.0 - smoothstep(0.0, 0.03, cr2)) * 0.5;
	col *= 1.0 - 0.5 * clamp(crack, 0.0, 1.0);
	// water streaks down the flanks
	float streak = smoothstep(0.55, 0.85, vnoise(vec3(w.x * 2.2, w.y * 0.22, w.z * 2.2)));
	col *= 1.0 - 0.38 * streak * (1.0 - top);
	// lichen and moss on the tops (forest), pale lichen spots everywhere
	float lichen = smoothstep(0.52, 0.72, fbm3(w * 1.4 + vec3(5.0))) * top * (1.0 - env_dry);
	col = mix(col, vec3(0.16, 0.19, 0.08), lichen * 0.55);
	float spots = smoothstep(0.8, 0.88, vnoise(w * 3.7 + vec3(1.3)));
	col = mix(col, base * vec3(2.6, 2.6, 2.3), spots * 0.3);
	// darker foot where soil and damp touch the rock
	col *= mix(0.6, 1.0, smoothstep(0.0, 0.9, h));
	if (kind == 1) {
		col *= 0.85;  // scree: loose, darker stones
	}
	return col;
}

// mortar lines of a running bond (brick / ashlar), size in meters
float bond(vec2 uv, vec2 size, float joint) {
	vec2 q = uv / size;
	q.x += step(1.0, mod(floor(q.y), 2.0)) * 0.5;
	vec2 f = fract(q);
	vec2 j = joint / size;
	return step(j.x, f.x) * step(j.y, f.y);
}

void fragment() {
	vec4 vc = COLOR;
	float ao = vc.r;
	int kind = kind_of(vc.a);
	vec3 p = v_local;
	vec3 wn = normalize(v_world_normal);
	float grain = vnoise(p * 6.0);
	float big = fbm3(p * 0.7);
	vec3 albedo = vec3(0.5);
	float rough = 0.9;
	float metal = 0.0;
	if (slot == 0) {  // Paint
		vec3 c[4] = {env_paint.rgb, vec3(0.62, 0.07, 0.05), vec3(0.86, 0.86, 0.82), vec3(0.36, 0.20, 0.11)};
		albedo = c[kind] * (0.9 + 0.2 * grain);
		rough = 0.7;
		float rust = smoothstep(0.62, 0.8, fbm3(p * 9.0)) * vc.g;
		albedo = mix(albedo, vec3(0.3, 0.16, 0.08), rust);
	} else if (slot == 1) {  // Metal
		vec3 c[4] = {vec3(0.27, 0.27, 0.27), vec3(0.33, 0.18, 0.09), vec3(0.55, 0.56, 0.56), vec3(0.12, 0.12, 0.12)};
		albedo = c[kind] * (0.85 + 0.3 * grain);
		if (kind == 1 || kind == 2) {
			// Corrugated sheet (prompt 056, steel hall, depot roof): ribs
			// across the wall, down the roof slope.
			float u = wn.y > 0.5 ? p.z : p.x + p.z;
			albedo *= 0.9 + 0.2 * abs(fract(u * 7.0) - 0.5);
		}
		rough = kind == 1 ? 0.85 : 0.5;
		metal = kind == 1 ? 0.2 : 0.6;
	} else if (slot == 2) {  // Rubber
		albedo = vec3(0.05);
		rough = 0.92;
	} else if (slot == 3) {  // Wood: weathered, fresh, dark (creosote), charred
		vec3 c[4] = {vec3(0.42, 0.33, 0.24), vec3(0.58, 0.42, 0.25), vec3(0.20, 0.15, 0.11), vec3(0.05, 0.045, 0.04)};
		float planks = vnoise(p * vec3(1.5, 1.5, 24.0)) * 0.35 + vnoise(p * vec3(24.0, 24.0, 1.5)) * 0.35;
		// board seams every 0.2 m on vertical faces
		float u = abs(wn.y) < 0.7 ? (p.x + p.y) * 5.0 : p.x * 5.0;
		float seam = 1.0 - 0.35 * (1.0 - smoothstep(0.0, 0.06, abs(fract(u) - 0.5) * 2.0 - 0.9));
		float board = 0.85 + 0.3 * hash2(vec2(floor(u), 1.0));
		albedo = c[kind] * (0.7 + 0.6 * planks) * board * seam;
		albedo *= 0.88 + 0.24 * fract(v_seed * 9.17);
		rough = 0.9;
	} else if (slot == 4) {  // Canvas / hessian (sandbags)
		albedo = vec3(0.50, 0.44, 0.32) * (0.85 + 0.3 * grain) * (0.9 + 0.2 * big);
		albedo = mix(albedo, albedo * vec3(0.9, 0.95, 0.8), env_dry * 0.0);
		rough = 0.97;
	} else if (slot == 5) {  // Glass
		albedo = vec3(0.04, 0.06, 0.07);
		rough = 0.06;
		metal = 0.3;
	} else if (slot == 10) {  // Bark: oak, birch, palm, pine
		if (kind == 1) {
			float marks = step(0.72, vnoise(vec3(p.x * 6.0, p.y * 6.0, p.z * 1.2)) + vnoise(p * vec3(2.0, 2.0, 18.0)) * 0.3);
			albedo = mix(vec3(0.82, 0.80, 0.74), vec3(0.08, 0.07, 0.06), marks);
			albedo = mix(vec3(0.15, 0.12, 0.10), albedo, smoothstep(0.4, 1.4, p.z));
		} else if (kind == 2) {
			float rings = smoothstep(0.3, 0.5, abs(fract(p.z * 3.0) - 0.5));
			albedo = mix(vec3(0.30, 0.25, 0.18), vec3(0.48, 0.40, 0.29), rings) * (0.85 + 0.3 * grain);
		} else if (kind == 3) {
			albedo = mix(vec3(0.24, 0.18, 0.14), vec3(0.62, 0.36, 0.20), smoothstep(8.0, 20.0, p.z));
			albedo *= 0.8 + 0.4 * vnoise(p * vec3(8.0, 8.0, 1.5));
		} else {
			albedo = vec3(0.26, 0.21, 0.16) * (0.65 + 0.7 * vnoise(p * vec3(10.0, 10.0, 1.2)));
		}
		rough = 0.95;
	} else if (slot == 12) {  // Stone: fieldstone, ashlar, concrete, brick
		vec2 uv = UV;
		if (kind == 3) {
			float m = bond(uv, vec2(0.25, 0.075), 0.012);
			vec3 brick = mix(vec3(0.50, 0.22, 0.15), vec3(0.58, 0.32, 0.20), fract(v_seed * 3.7));
			albedo = mix(vec3(0.62, 0.60, 0.55), brick * (0.8 + 0.4 * hash2(floor(uv / vec2(0.25, 0.075)))), m);
		} else if (kind == 1) {
			float m = bond(uv, vec2(0.6, 0.3), 0.02);
			albedo = mix(vec3(0.45, 0.43, 0.40), vec3(0.66, 0.62, 0.54) * (0.88 + 0.24 * hash2(floor(uv / vec2(0.6, 0.3)))), m);
		} else if (kind == 2) {
			float lines = smoothstep(0.0, 0.02, abs(fract(p.z * 1.6) - 0.5) - 0.48);
			albedo = vec3(0.58, 0.58, 0.55) * (0.85 + 0.25 * big) * (1.0 - lines * 0.15);
		} else {
			albedo = mix(vec3(0.42, 0.41, 0.38), vec3(0.55, 0.52, 0.46), big) * (0.85 + 0.3 * grain);
			albedo = mix(albedo, vec3(0.25, 0.32, 0.18), smoothstep(0.65, 0.8, fbm3(p * 3.0)) * (1.0 - env_dry) * 0.6 * max(wn.y, 0.0));
		}
		rough = 0.93;
	} else if (slot == 13) {  // Plaster: lime, adobe, whitewash, ochre
		vec3 c[4] = {vec3(0.80, 0.77, 0.69), vec3(0.62, 0.48, 0.33), vec3(0.90, 0.89, 0.85), vec3(0.76, 0.60, 0.36)};
		vec3 base = c[kind];
		if (kind == 0 || kind == 2) {
			// Most houses get a paint of their own, a few stay bare lime.
			base = v_seed < 0.15 ? base : plaster_paint(fract(v_seed * 7.13));
		} else {
			base *= 0.9 + 0.2 * fract(v_seed * 5.31);
		}
		albedo = base * (0.88 + 0.16 * big) * (0.95 + 0.1 * grain);
		// grime rising from the ground
		float h = vc.b * 4.0;
		albedo *= mix(0.72, 1.0, smoothstep(0.0, 1.2 + 0.6 * big, h));
		rough = 0.95;
	} else if (slot == 14) {  // Ground: earth, spoil, mud, sand
		vec3 c[4] = {vec3(0.33, 0.26, 0.18), vec3(0.42, 0.34, 0.24), vec3(0.22, 0.18, 0.13), vec3(0.66, 0.58, 0.42)};
		albedo = c[kind] * (0.8 + 0.4 * big) * (0.9 + 0.2 * grain);
		albedo = mix(albedo, vec3(0.56, 0.48, 0.34), env_dry * 0.5);
		rough = 1.0;
	} else if (slot == 16) {  // Rock: bedrock, scree (prompt 074)
		albedo = rock_albedo(v_world, wn.y, vc.b * 4.0, kind);
		rough = 0.92;
	} else {  // 15 Roof: clay tile, slate, thatch, planks
		float rows = smoothstep(0.0, 0.08, fract(p.z * 6.0)) * 0.25 + 0.75;
		float cols = 0.9 + 0.2 * hash2(floor(vec2(p.x * 4.0 + p.y * 4.0, p.z * 6.0)));
		// Roof variants (prompt 056): clay tiles red, brown or weathered
		// grey-brown, a few houses with dark cement tiles; slate blue-grey
		// or nearly black.
		float r_pick = fract(v_seed * 11.3);
		if (kind == 0 && r_pick > 0.82) {
			kind = 1;
		}
		if (kind == 1) {
			vec3 slate = mix(vec3(0.22, 0.24, 0.27), vec3(0.15, 0.15, 0.16), fract(v_seed * 4.7));
			albedo = slate * rows * cols;
			rough = 0.7;
		} else if (kind == 2) {
			float straw = vnoise(p * vec3(30.0, 30.0, 3.0));
			albedo = mix(vec3(0.52, 0.44, 0.24), vec3(0.70, 0.62, 0.36), straw) * (0.85 + 0.25 * big);
			rough = 1.0;
		} else if (kind == 3) {
			albedo = vec3(0.30, 0.25, 0.20) * rows * cols;
		} else {
			vec3 tile = r_pick < 0.4 ? vec3(0.56, 0.26, 0.17)
				: (r_pick < 0.65 ? vec3(0.48, 0.28, 0.20) : vec3(0.45, 0.36, 0.30));
			albedo = tile * rows * cols * (0.85 + 0.25 * big);
			albedo = mix(albedo, vec3(0.25, 0.30, 0.16), smoothstep(0.7, 0.85, fbm3(p * 2.0)) * 0.5 * (1.0 - env_dry));
			rough = 0.85;
		}
	}
	// edge wear (bevel strips) brightens a little, desert dust, snow on top faces
	if (slot != 10) {
		albedo = mix(albedo, albedo * 1.15, vc.g * 0.5);
	}
	if (slot != 5 && slot != 14) {
		float dust = env_dust * (0.35 + 0.65 * max(wn.y, 0.0)) * (0.6 + 0.4 * grain);
		// grown rock takes its desert color from the tint, less dust
		dust *= slot == 16 ? 0.45 : 1.0;
		albedo = mix(albedo, vec3(0.70, 0.62, 0.46), clamp(dust, 0.0, 0.7));
		albedo = snow_cover(albedo, rough, normalize(v_rest_normal), p, env_snow);
	}
	albedo = charred(albedo, env_char);
	ALBEDO = albedo * mix(0.45, 1.0, ao);
	AO = ao;
	AO_LIGHT_AFFECT = 0.3;
	ROUGHNESS = rough;
	METALLIC = metal;
}
