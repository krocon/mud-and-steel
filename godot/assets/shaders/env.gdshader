// Opaque environment material: buildings, props, rocks, trunks, fortifications.
//
// One ShaderMaterial per slot (godot/assets/materials/env/*.tres, assigned on
// import by assets/import/unit_post_import.gd for models with env_kit_version).
// Each slot has up to four material kinds (vertex color A, see
// tools/blender/kit/env.py SLOT_KINDS); all patterns are procedural, there are
// no texture files. Biome look via instance uniforms (EnvLook.apply()).
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Clouds dim the direct sunlight (prompt 087): light() of the include.
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

#include "res://assets/shaders/env_common.gdshaderinc"
// Filtered thresholds and lines, relief from a height (prompt 203).
#include "res://assets/shaders/relief.gdshaderinc"
// Roof patina (prompt 075): RoofFrame, roof_clay/slate/thatch/planks,
// sheet_patina (corrugated Metal), flat_roof_patina (flat Plaster roofs).
#include "res://assets/shaders/env_roof.gdshaderinc"
// Roof snow (prompt 080): roof_snow(), snow_albedo().
#include "res://assets/shaders/env_snow.gdshaderinc"
// Storey-by-storey collapse (prompt 197): collapse_vertex().
#include "res://assets/shaders/env_collapse.gdshaderinc"

// A map's look (prompt 122, BiomePreset.roof_tint): clay roofs drawn
// towards this color (linear), `a` = how much; 0 on every other map.
global uniform vec4 env_roof_tint;

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
	// Bark (prompt 086): the same wind as the foliage, without the leaf
	// flutter, so trunk and crown move together; the trunk itself (G = 0)
	// bends with the height when the object is a tree (env_bend).
	if (slot == 10 && (COLOR.g > 0.0 || env_bend > 0.0)) {
		VERTEX += wind_offset(VERTEX, COLOR.g, COLOR.b, MODEL_MATRIX, TIME, 0.0) * (1.0 - env_fallen);
	}
	// A collapsing building (prompt 197); the pattern stays on v_local.
	VERTEX = collapse_vertex(VERTEX, MODEL_MATRIX);
	v_world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_rest_normal = normalize(NORMAL);
	v_seed = hash2(floor(inst.xz * 0.5) + vec2(0.5, 0.5));
}

// Paint of plaster walls (prompt 056): whitewash, cream, pale yellow, light
// ochre, pale pink, light grey-green, grey (a palette of rural Central
// European houses, judgement values).
// Wall colors in the desert (prompt 089): sun-dried mud brick and mud
// plaster in the tones of the sand, a few houses whitewashed.
vec3 plaster_mud(float seed) {
	vec3 c[6] = {vec3(0.55, 0.26, 0.105), vec3(0.62, 0.32, 0.13), vec3(0.47, 0.22, 0.09),
		vec3(0.66, 0.37, 0.165), vec3(0.58, 0.30, 0.14), vec3(0.78, 0.66, 0.48)};
	return c[int(floor(seed * 6.0)) % 6];
}

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
// model's ground (m), `px` = meters per pixel. `relief` (prompt 203): the
// height of the surface in meters, cracks cut in, the bedding and the grain
// of the stone standing out a little.
vec3 rock_albedo(vec3 w, float up, float h, int kind, float px, out float relief) {
	vec3 base = rock_base();
	float top = smoothstep(0.3, 0.85, up);
	vec3 col = base * mix(vec3(0.95, 0.98, 1.04), vec3(1.95, 1.85, 1.66), top);
	float tone = fbm3(w * 0.33);
	col *= 0.78 + 0.44 * tone;
	// the grain of the stone (23 cm), gone before it flickers
	float grit = mix(0.5, vnoise(w * 4.3), roof_fine(px, 0.23));
	col *= 0.9 + 0.2 * grit;
	// bedding: horizontal bands on the flanks
	float band = sin(w.y * 2.1 + tone * 4.0) * 0.5 + 0.5;
	col *= mix(1.0, 0.86 + 0.2 * band, 1.0 - top);
	// cracks: thin dark lines where a noise crosses its middle, wider and
	// fainter as the pixel grows
	float cr = abs(vnoise(w * vec3(0.8, 1.3, 0.8) + vec3(3.1, 0.0, 7.7)) * 2.0 - 1.0);
	float cr2 = abs(vnoise(w * 2.2 + vec3(11.0, 5.0, 2.0)) * 2.0 - 1.0);
	float crack = clamp(aa_line(cr, 0.045) + aa_line(cr2, 0.03) * 0.5, 0.0, 1.0);
	col *= 1.0 - 0.5 * crack;
	relief = 0.02 * grit + 0.05 * band * (1.0 - top) - 0.03 * crack;
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

// Apartment blocks (prompt 078): faces whose UV.x is 100 + across carry a
// pattern instead of box-projected meters (tools/blender/kit/blocks.py);
// UV.y runs from 0 at the top to 1 at the bottom.
// Plaster 100: dirt streak under a window sill, 101: soot over a burnt-out
// window. The card lies 1 cm in front of the wall and repeats its plaster,
// so only the stain shows.
vec3 wall_stain(vec3 albedo, vec2 uv, vec3 p) {
	float code = floor(uv.x);
	float fx = fract(uv.x);
	float fy = clamp(uv.y, 0.0, 1.0);
	float n = vnoise(vec3(fx * 9.0 + p.x * 0.7 + p.z * 0.7, fy * 2.5, p.y * 0.3));
	if (code < 100.5) {
		float edge = smoothstep(0.0, 0.22, fx) * smoothstep(1.0, 0.78, fx);
		float a = pow(1.0 - fy, 1.3) * edge * (0.45 + 0.9 * n);
		albedo = mix(albedo, albedo * vec3(0.48, 0.45, 0.40), clamp(a, 0.0, 1.0));
	} else {
		float dist = abs(fx - 0.5);
		float limit = 0.2 + 0.28 * (1.0 - fy) + (n - 0.5) * 0.14;
		float a = smoothstep(limit, limit - 0.16, dist) * (0.35 + 0.65 * fy) * (0.75 + 0.5 * n);
		albedo = mix(albedo, vec3(0.035, 0.033, 0.03), clamp(a, 0.0, 0.92));
	}
	return albedo;
}

// Glass 102: a sash window (white frame, mullion, transom, glazing bars),
// one quad per window. Frame color and curtains vary per window.
vec3 sash_window(vec2 uv, vec3 world, inout float rough, inout float spec) {
	float fx = fract(uv.x);
	float fy = clamp(uv.y, 0.0, 1.0);
	vec2 fw = max(fwidth(vec2(fx, fy)), vec2(1e-4));
	float id = hash2(floor(world.xz * 0.8) + vec2(floor(world.y / 2.8) * 7.1, 3.3));
	// bars: distance to the nearest bar line in each direction, anti-aliased
	float frame = max(1.0 - smoothstep(0.045, 0.045 + fw.x, min(fx, 1.0 - fx)),
		1.0 - smoothstep(0.035, 0.035 + fw.y, min(fy, 1.0 - fy)));
	float mull = 1.0 - smoothstep(0.022, 0.022 + fw.x, abs(fx - 0.5));
	float trans = 1.0 - smoothstep(0.02, 0.02 + fw.y, abs(fy - 0.28));
	float lower = step(0.28, fy);
	float bars = lower * (1.0 - smoothstep(0.011, 0.011 + fw.y, min(abs(fy - 0.52), abs(fy - 0.76))));
	bars = max(bars, (1.0 - lower) * (1.0 - smoothstep(0.011, 0.011 + fw.x, min(abs(fx - 0.25), abs(fx - 0.75)))));
	float wood = clamp(max(max(frame, mull), max(trans, bars)), 0.0, 1.0);
	vec3 paint = id < 0.7 ? vec3(0.80, 0.79, 0.74) : (id < 0.85 ? vec3(0.30, 0.38, 0.30) : vec3(0.40, 0.30, 0.22));
	paint *= 0.85 + 0.15 * hash2(vec2(id * 13.0, 1.0));
	// panes: dark glass with a faint sky sheen, curtains in some windows
	vec3 glass = mix(vec3(0.03, 0.04, 0.05), vec3(0.16, 0.18, 0.20), (1.0 - fy) * 0.5 * hash2(vec2(id, 5.0)));
	float curtain = step(0.6, hash2(vec2(id, 9.0))) * lower;
	// folds of the curtain (6 per window), gone before they flicker
	float folds = sin(fx * 40.0) * roof_fine(fw.x, 0.157);
	glass = mix(glass, vec3(0.42, 0.38, 0.30) * (0.8 + 0.2 * folds), curtain * 0.85);
	rough = mix(0.06, 0.7, max(wood, curtain));
	spec = mix(0.75, 0.5, max(wood, curtain));
	return mix(glass, paint, wood);
}

// Glass 103: Dutch/Flemish white cross-mullion window (prompt 224) — bold
// off-white frame, central vertical mullion and high transom readable from
// 50-140 m against dark red brick walls.
vec3 cross_window(vec2 uv, vec3 world, inout float rough, inout float spec) {
	float fx = fract(uv.x);
	float fy = clamp(uv.y, 0.0, 1.0);
	vec2 fw = max(fwidth(vec2(fx, fy)), vec2(1e-4));
	float id = hash2(floor(world.xz * 0.8) + vec2(floor(world.y / 2.8) * 7.1, 5.9));
	float frame = max(1.0 - smoothstep(0.065, 0.065 + fw.x, min(fx, 1.0 - fx)),
		1.0 - smoothstep(0.052, 0.052 + fw.y, min(fy, 1.0 - fy)));
	float mull = 1.0 - smoothstep(0.040, 0.040 + fw.x, abs(fx - 0.5));
	float trans = 1.0 - smoothstep(0.038, 0.038 + fw.y, abs(fy - 0.34));
	float wood = clamp(max(frame, max(mull, trans)), 0.0, 1.0);
	vec3 paint = vec3(0.86, 0.85, 0.81) * (0.90 + 0.12 * hash2(vec2(id * 11.0, 2.0)));
	vec3 glass = mix(vec3(0.025, 0.032, 0.042), vec3(0.11, 0.13, 0.16), (1.0 - fy) * 0.45);
	float curtain = step(0.68, hash2(vec2(id, 9.0))) * step(0.34, fy);
	glass = mix(glass, vec3(0.34, 0.31, 0.26), curtain * 0.72);
	rough = mix(0.06, 0.68, max(wood, curtain));
	spec = mix(0.75, 0.5, max(wood, curtain));
	return mix(glass, paint, wood);
}

// Mortar lines of a running bond (brick / ashlar), size in meters: 1 on the
// stone, 0 in the joint, box-filtered for the pixel (prompt 203), so a joint
// thinner than a pixel dims it by its share instead of flickering.
float bond(vec2 uv, vec2 size, float joint) {
	vec2 q = uv / size;
	vec2 w = fwidth(uv) / size;
	q.x += step(1.0, mod(floor(q.y), 2.0)) * 0.5;
	vec2 j = joint / size;
	return (1.0 - roof_joint(q.x, w.x, j.x)) * (1.0 - roof_joint(q.y, w.y, j.y));
}

// Surfaces per building (prompt 186): plaster, spalling, brick, roof
// coverings, relief from normals (env_surface.gdshaderinc, needs bond()).
#include "res://assets/shaders/env_surface.gdshaderinc"

void fragment() {
	CLOUD_FRAGMENT(0.5)
	vec4 vc = COLOR;
	float ao = vc.r;
	int kind = kind_of(vc.a);
	vec3 p = v_local;
	vec3 wn = normalize(v_world_normal);
	// Meters per pixel on the surface (prompt 203): patterns fade into their
	// mean before they get smaller than a pixel.
	vec3 pw = fwidth(p);
	float px = max(max(pw.x, pw.y), pw.z);
	// Two noises shared by several slots, only where a slot uses them
	// (prompt 203; `big` alone is 32 hashes per pixel).
	float grain = 0.5;
	float big = 0.5;
	if (slot == 0 || slot == 1 || slot == 4 || slot == 13 || slot == 14 || env_dust > 0.0
			|| (slot == 10 && kind == 2) || (slot == 12 && kind == 0)) {
		grain = vnoise(p * 6.0);
	}
	if (slot == 4 || slot == 13 || slot == 14 || (slot == 12 && (kind == 0 || kind == 2))) {
		big = fbm3(p * 0.7);
	}
	vec3 albedo = vec3(0.5);
	float rough = 0.9;
	float metal = 0.0;
	float spec = 0.5;
	// Roof snow (prompt 080): which covering this pixel is, -1 = no roof.
	// 0 tiles, slate, boards; 1 thatch; 2 corrugated sheet; 3 flat roof.
	int snow_roof = -1;
	RoofFrame srf;
	// Relief height in meters (prompt 186), turned into a normal at the end.
	float bump_h = 0.0;
	if (slot == 0) {  // Paint
		vec3 c[4] = {env_paint.rgb, vec3(0.62, 0.07, 0.05), vec3(0.86, 0.86, 0.82), vec3(0.36, 0.20, 0.11)};
		albedo = c[kind] * (0.9 + 0.2 * grain);
		rough = 0.7;
		float rust = smoothstep(0.62, 0.8, fbm3(p * 9.0)) * vc.g;
		albedo = mix(albedo, vec3(0.3, 0.16, 0.08), rust);
	} else if (slot == 1) {  // Metal
		vec3 c[4] = {vec3(0.27, 0.27, 0.27), vec3(0.33, 0.18, 0.09), vec3(0.55, 0.56, 0.56), vec3(0.12, 0.12, 0.12)};
		albedo = c[kind] * (0.85 + 0.3 * grain);
		rough = kind == 1 ? 0.85 : 0.5;
		metal = kind == 1 ? 0.2 : 0.6;
		if (kind == 1 || kind == 2) {
			// Corrugated sheet (prompt 056, steel hall, depot roof): 7 ribs
			// per meter along the sheets, like their seams (up the wall, down
			// the roof slope); their shading and relief (prompt 203) fade
			// before a rib is a pixel wide.
			RoofFrame rf = roof_frame(p, normalize(v_rest_normal), wn);
			float ribs = roof_fine(rf.fw, 0.143);
			float u = rf.uv.x * 7.0;
			albedo *= 0.95 + 0.2 * (abs(fract(u) - 0.5) - 0.25) * ribs;
			bump_h = 0.006 * cos(u * 6.2832) * ribs;
			if (v_rest_normal.y > -0.2) {
				// Patina of the sheets (prompt 075): rust runs, matte and bright sheets.
				albedo = sheet_patina(albedo, rf, v_seed, roof_age(v_seed), env_dry, rough, metal);
				if (v_rest_normal.y > 0.15) {
					snow_roof = 2;
					srf = rf;
				}
			}
		}
	} else if (slot == 2) {  // Rubber
		albedo = vec3(0.05);
		rough = 0.92;
	} else if (slot == 3) {  // Wood: weathered, fresh, dark (creosote), charred
		vec3 c[4] = {vec3(0.286, 0.224, 0.163), vec3(0.394, 0.286, 0.170), vec3(0.136, 0.102, 0.075), vec3(0.034, 0.031, 0.027)};
		// Boards 0.2 m wide: upright on walls (Y is up), along z on floors
		// and decks; the grain runs along them.
		bool wall = abs(wn.y) < 0.7;
		vec3 along = wall ? vec3(24.0, 1.5, 24.0) : vec3(24.0, 24.0, 1.5);
		float planks = vnoise(p * along) * 0.45 + vnoise(p * along * 0.3 + vec3(7.3)) * 0.25;
		float u = wall ? (p.x + p.z) * 5.0 : p.x * 5.0;
		// dark seams 2 cm wide, filtered for the pixel; the tone of single
		// boards fades before it flickers (prompt 203)
		float seam = roof_rows(u + 0.05, 0.1);
		float board = 1.0 + (0.3 * hash2(vec2(floor(u), 1.0)) - 0.15) * roof_fine(px, 0.2);
		albedo = c[kind] * (0.7 + 0.6 * planks) * board * (1.0 - 0.45 * seam);
		albedo *= 0.88 + 0.24 * fract(v_seed * 9.17);
		bump_h = -0.006 * seam;
		rough = 0.9;
	} else if (slot == 4) {  // Canvas / hessian (sandbags)
		albedo = vec3(0.50, 0.44, 0.32) * (0.85 + 0.3 * grain) * (0.9 + 0.2 * big);
		if (kind == 1) {
			// Tent canvas (prompt 202): olive drab, far darker than hessian.
			albedo = vec3(0.115, 0.125, 0.07) * (0.85 + 0.3 * grain) * (0.9 + 0.2 * big);
		}
		rough = 0.97;
	} else if (slot == 5) {  // Glass
		// A dielectric (prompt 203): no metal, a strong mirror reflection.
		albedo = vec3(0.04, 0.06, 0.07);
		rough = 0.06;
		spec = 0.75;
		if (UV.x > 102.5) {
			albedo = cross_window(UV, v_world, rough, spec);
		} else if (UV.x > 101.5) {
			albedo = sash_window(UV, v_world, rough, spec);
		}
	} else if (slot == 10) {  // Bark: oak, birch, palm, pine
		// Y is up the trunk (prompt 203): furrows run up, rings and the marks
		// of the birch across. Furrows and their relief fade with the pixel.
		float fine = roof_fine(px, 0.12);
		if (kind == 1) {
			// birch: white and smooth, dark marks across the trunk, a dark foot
			float n = vnoise(p * vec3(5.0, 9.0, 5.0)) + vnoise(p * vec3(2.0, 18.0, 2.0)) * 0.3;
			float marks = mix(0.15, aa_step(0.8, n), roof_fine(px, 0.06));
			albedo = mix(vec3(0.82, 0.80, 0.74), vec3(0.08, 0.07, 0.06), marks);
			float foot = 1.0 - smoothstep(0.4, 1.4, p.y);
			albedo = mix(albedo, vec3(0.15, 0.12, 0.10), foot);
		} else if (kind == 2) {
			// palm: the scars of fallen fronds, three rings per meter
			float rings = mix(0.2, smoothstep(0.3, 0.5, abs(fract(p.y * 3.0) - 0.5)), roof_fine(px, 0.33));
			albedo = mix(vec3(0.30, 0.25, 0.18), vec3(0.48, 0.40, 0.29), rings) * (0.85 + 0.3 * grain);
			bump_h = 0.012 * rings;
		} else if (kind == 3) {
			// pine: grey-brown plates below, thin red bark from 8 m up
			float n = mix(0.5, vnoise(p * vec3(8.0, 1.5, 8.0)), fine);
			float high = smoothstep(8.0, 20.0, p.y);
			albedo = mix(vec3(0.24, 0.18, 0.14), vec3(0.62, 0.36, 0.20), high);
			albedo *= 0.8 + 0.4 * n;
			bump_h = 0.016 * n * (1.0 - 0.6 * high);
		} else {
			// oak: deep furrows
			float n = mix(0.5, vnoise(p * vec3(10.0, 1.2, 10.0)), fine);
			albedo = vec3(0.26, 0.21, 0.16) * (0.65 + 0.7 * n);
			bump_h = 0.018 * n;
		}
		rough = 0.95;
		// Dark damp autumn bark so trunks and twigs stand out against bright foliage (prompt 223).
		if (env_autumn > 0.001 && kind != 2) {
			albedo *= mix(1.0, kind == 1 ? 0.72 : 0.26, env_autumn);
			rough = mix(rough, 0.74, env_autumn);
		}
	} else if (slot == 12) {  // Stone: fieldstone, ashlar, concrete, brick
		vec2 uv = UV;
		if (kind == 3) {
			vec3 brick = mix(vec3(0.50, 0.22, 0.15), vec3(0.58, 0.32, 0.20), fract(v_seed * 3.7));
			// The blueprint's brick and bond (prompt 186) or, without one,
			// the house's own brick in a stretcher bond; joints and single
			// bricks fade with the pixel either way (prompt 203).
			float joint;
			albedo = surface_brickwork(uv, surface_brick_color(env_surface_a.z, brick), env_surface_a.w, joint);
			// recessed mortar joints (prompt 186), gone before they flicker
			bump_h = -0.006 * joint * roof_fine(max(fwidth(uv.x), fwidth(uv.y)), 0.075);
		} else if (kind == 1) {
			// Ashlar: blocks of 60 x 30 cm; joints, block tones and the
			// recessed joints fade with the pixel (prompt 203).
			vec2 size = vec2(0.6, 0.3);
			float k = roof_fine(max(fwidth(uv.x), fwidth(uv.y)), size.y);
			float m = mix(0.9, bond(uv, size, 0.02), k);
			float course = floor(uv.y / size.y);
			float tone = 1.0 + (0.24 * hash2(vec2(floor(uv.x / size.x + 0.5 * mod(course, 2.0)), course)) - 0.12) * k;
			albedo = mix(vec3(0.45, 0.43, 0.40), vec3(0.66, 0.62, 0.54) * tone, m);
			bump_h = -0.008 * (1.0 - m) * k;
		} else if (kind == 2) {
			// Concrete: joints of the formwork every 62.5 cm up the wall (Y
			// is up, prompt 203), none on top; filtered for the pixel.
			float side = 1.0 - smoothstep(0.5, 0.8, abs(wn.y));
			float lines = aa_line(abs(fract(p.y * 1.6 + 0.5) - 0.5), 0.02) * side;
			albedo = vec3(0.58, 0.58, 0.55) * (0.85 + 0.25 * big) * (1.0 - lines * 0.15);
			bump_h = -0.004 * lines;
		} else {
			albedo = mix(vec3(0.42, 0.41, 0.38), vec3(0.55, 0.52, 0.46), big) * (0.85 + 0.3 * grain);
			albedo = mix(albedo, vec3(0.25, 0.32, 0.18), smoothstep(0.65, 0.8, fbm3(p * 3.0)) * (1.0 - env_dry) * 0.6 * max(wn.y, 0.0));
		}
		// Ground-contact damp/shadow darkening at the foot of plinths and stone walls (prompt 219).
		albedo *= mix(0.68, 1.0, smoothstep(0.0, 0.85, vc.b * 4.0));
		rough = 0.93;
	} else if (slot == 13) {  // Plaster: lime, adobe, whitewash, ochre
		vec3 c[4] = {vec3(0.80, 0.77, 0.69), vec3(0.62, 0.48, 0.33), vec3(0.90, 0.89, 0.85), vec3(0.76, 0.60, 0.36)};
		vec3 base = c[kind];
		if (env_surface_a.x > 0.5) {
			// the blueprint's plaster color (prompt 186), a little per house
			base = surface_plaster(env_surface_a.x) * (0.96 + 0.08 * fract(v_seed * 5.31));
		} else if (kind == 0 || kind == 2) {
			// Most houses get a paint of their own, a few stay bare lime.
			base = v_seed < 0.15 ? base : plaster_paint(fract(v_seed * 7.13));
		} else {
			base *= 0.9 + 0.2 * fract(v_seed * 5.31);
		}
		if (env_dust > 0.0 && env_surface_a.x < 0.5) {
			base = mix(base, plaster_mud(fract(v_seed * 7.13)), clamp(env_dust / 0.45, 0.0, 1.0));
		}
		albedo = base * (0.88 + 0.16 * big) * (0.95 + 0.1 * grain);
		// grime rising from the ground
		float h = vc.b * 4.0;
		albedo *= mix(0.72, 1.0, smoothstep(0.0, 1.2 + 0.6 * big, h));
		if (UV.x > 99.5) {
			albedo = wall_stain(albedo, UV, p);
		}
		if (env_surface_a.y > 0.0 && abs(v_rest_normal.y) < 0.5) {
			// Spalled plaster (prompt 186): brick shows through, the broken
			// edge is darker and stands 1.5 cm proud of the brick.
			vec2 w = surface_wall_uv(p, normalize(v_rest_normal));
			float rim;
			float bare = surface_spall(w, env_surface_a.y, h, vc.g, v_seed, rim);
			float joint;
			vec3 fallback = mix(vec3(0.50, 0.22, 0.15), vec3(0.58, 0.32, 0.20), fract(v_seed * 3.7));
			vec3 brick = surface_brickwork(w, surface_brick_color(env_surface_a.z, fallback), env_surface_a.w, joint);
			brick *= mix(0.72, 1.0, smoothstep(0.0, 1.2, h));
			albedo = mix(albedo * (1.0 - 0.35 * rim), brick, bare);
			float fw = max(fwidth(w.x), fwidth(w.y));
			bump_h = (1.0 - bare) * 0.015 * roof_fine(fw, 0.3) - 0.006 * joint * bare * roof_fine(fw, 0.075);
		}
		if (v_rest_normal.y > 0.9) {
			// Flat roofs (prompt 075): grime rims, stains, cracks; the wall
			// colors stay as they are.
			RoofFrame rf = roof_frame(p, normalize(v_rest_normal), wn);
			albedo = flat_roof_patina(albedo, rf, v_seed, roof_age(v_seed), ao, env_dust);
			snow_roof = 3;
			srf = rf;
		}
		rough = 0.95;
	} else if (slot == 14) {  // Ground: earth, spoil, mud, sand
		vec3 c[4] = {vec3(0.33, 0.26, 0.18), vec3(0.42, 0.34, 0.24), vec3(0.22, 0.18, 0.13), vec3(0.66, 0.58, 0.42)};
		albedo = c[kind] * (0.8 + 0.4 * big) * (0.9 + 0.2 * grain);
		albedo = mix(albedo, vec3(0.56, 0.48, 0.34), env_dry * 0.5);
		rough = 1.0;
	} else if (slot == 16) {  // Rock: bedrock, scree (prompt 074)
		albedo = rock_albedo(v_world, wn.y, vc.b * 4.0, kind, px, bump_h);
		rough = 0.92;
	} else {  // 15 Roof: clay tile, slate, thatch, planks
		// Patina per covering (prompt 075, env_roof.gdshaderinc), all from
		// the position on the model and the per-house seed.
		RoofFrame rf = roof_frame(p, normalize(v_rest_normal), wn);
		float age = roof_age(v_seed);
		// Snow on the top faces and on the outward edge faces of the slabs
		// (eave, verge, ridge cap), not on the undersides.
		vec3 rn = normalize(v_rest_normal);
		if (rf.top > 0.5 || dot(rn.xz, p.xz) > 0.0) {
			snow_roof = kind == 2 ? 1 : 0;
			srf = rf;
		}
		// Roof variants (prompt 056): about every fifth clay roof has dark
		// cement tiles, drawn like slate.
		if (kind == 0 && fract(v_seed * 11.3) > 0.82) {
			kind = 1;
		}
		if (env_surface_b.x > 0.5) {
			// The blueprint's covering (prompt 186).
			int covering = int(env_surface_b.x + 0.5);
			albedo = surface_roof(rf, v_seed, age, env_dry, covering, env_surface_b.y, env_surface_b.z,
				env_surface_b.w, bump_h);
			rough = covering == 5 ? 0.7 : 0.85;
		} else if (kind == 1) {
			albedo = roof_slate(rf, v_seed, age, env_dry, bump_h);
			rough = 0.7;
		} else if (kind == 2) {
			albedo = roof_thatch(rf, v_seed, age, env_dry);
			rough = 1.0;
		} else if (kind == 3) {
			albedo = roof_planks(rf, v_seed, age, env_dry);
			rough = 0.9;
		} else {
			albedo = roof_clay(rf, v_seed, age, env_dry, bump_h);
			rough = 0.85;
		}
		// Tiled roofs of a colorful map look (prompt 122): clay, cement
		// and slate tiles turn red, their own light and dark kept;
		// thatch and boards stay.
		bool tiled = env_surface_b.x > 0.5 || kind == 0 || kind == 1;
		if (env_roof_tint.a > 0.0 && tiled) {
			vec3 lw = vec3(0.3, 0.55, 0.15);
			float l = dot(albedo, lw) / max(dot(env_roof_tint.rgb, lw), 1e-3);
			albedo = mix(albedo, env_roof_tint.rgb * l, env_roof_tint.a);
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
		albedo = mix(albedo, vec3(0.70, 0.38, 0.15), clamp(dust, 0.0, 0.7));
		if (snow_roof < 0) {
			albedo = snow_cover(albedo, rough, normalize(v_rest_normal), p, env_snow);
		}
	}
	// Roof snow (prompt 080): after patina and dust, before soot; none where
	// the roof burns or is charred.
	float roof_amount = env_snow * (1.0 - smoothstep(0.05, 0.4, env_char));
	float snow_cover_share = 0.0;
	if (snow_roof >= 0 && roof_amount > 0.0) {
		// hold, ridge band, eave band, softness per covering
		vec4 cfg = snow_roof == 1 ? vec4(1.35, 1.2, 1.2, 0.06)
			: (snow_roof == 2 ? vec4(0.7, 0.45, 1.1, 0.0) : vec4(1.0, 1.0, 1.0, 0.0));
		RoofSnow rs = roof_snow(p, normalize(v_rest_normal), wn, srf, v_seed, roof_amount,
			cfg.x, cfg.y, cfg.z, cfg.w, snow_roof == 3 ? 1.0 : 0.0);
		albedo *= 1.0 - 0.65 * rs.shade;
		albedo = mix(albedo, snow_albedo(p, wn, srf), rs.cover);
		snow_cover_share = rs.cover;
		rough = mix(rough, 0.8, rs.cover);
		metal = mix(metal, 0.0, rs.cover);
		if (abs(rs.bend) > 0.0) {
			vec3 wdown = normalize(vec3(wn.x, 0.0, wn.z) + vec3(1e-4, 0.0, 0.0));
			NORMAL = normalize(NORMAL + (VIEW_MATRIX * vec4(wdown * rs.bend * 0.7, 0.0)).xyz);
		}
	}
	// Relief (prompt 186): none under snow. Taken here, outside every
	// branch, so the derivatives are defined.
	NORMAL = surface_bump(NORMAL, VERTEX, bump_h * (1.0 - snow_cover_share));
	albedo = charred(albedo, env_char);
	// The baked occlusion dims the light, not the color (prompt 203): the
	// albedo stays the material's own (metal reflects with it), ambient and
	// direct light take the shade. The two factors give the brightness the
	// darkened albedo had (ao * mix(0.45, 1, ao) in ambient light, about
	// mix(0.45, 1, ao) * mix(1, ao, 0.3) in direct light).
	ALBEDO = albedo;
	AO = ao * mix(0.45, 1.0, ao);
	AO_LIGHT_AFFECT = 0.6;
	ROUGHNESS = rough;
	METALLIC = metal;
	SPECULAR = spec;
	cloud_specular = spec;
}
