// Foliage cards: leaves, needles, palm fronds, grass, flowers, twigs.
//
// The card pattern is chosen by the integer part of UV.x (virtual atlas,
// tools/blender/kit/env.py CARD_*), the fraction is the position on the card.
// Everything is procedural (no textures), alpha-scissored and double sided.
// Wind: vertex color G = weight, B = phase. Biome tint via instance uniforms.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled, diffuse_burley, specular_schlick_ggx;

// Clouds dim the direct sunlight (prompt 087): light() of the include.
#define CLOUD_BACKLIGHT
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

#include "res://assets/shaders/env_common.gdshaderinc"

uniform float alpha_cut = 0.5;
// Beyond this camera distance (m), and in directional shadow passes, leaf
// and needle cells use a cheap one-hash pattern instead of the 3 x 3 cell
// search: the cells are a few pixels small there, and over a wood the full
// pattern made the shadow passes cost most of the frame (prompt 013).
uniform float detail_distance = 70.0;

varying vec3 v_local;
varying vec3 v_world_normal;
// Normal of the standing model: snow stays on fallen trees (prompt 056).
varying vec3 v_rest_normal;
varying float v_tint;

void vertex() {
	// A bare tree (prompt 085): every card collapses to one point, so its
	// triangles have no area and draw nothing (cheaper than discarding).
	// Checked before wind_offset() (prompt 205).
	if (env_bare > 0.5) {
		VERTEX = vec3(0.0);
	} else {
		bool shadow_pass = PROJECTION_MATRIX[3][3] > 0.5;
		vec3 inst = MODEL_MATRIX[3].xyz;
		// Shadow passes skip high-frequency leaf flutter (prompt 205).
		VERTEX += wind_offset(VERTEX, COLOR.g, COLOR.b, MODEL_MATRIX, TIME, shadow_pass ? 0.0 : 1.0) * (1.0 - env_fallen);
		v_tint = hash1(inst * 0.37 + vec3(COLOR.b * 13.0));
		if (!shadow_pass) {
			v_local = VERTEX;
			v_world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
			v_rest_normal = normalize(NORMAL);
		}
	}
}

// Fast sin-free 2D hash (prompt 205): much cheaper than sin() on shadow passes.
float hash2_fast(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * 0.1031);
	q += dot(q, q.yzx + 33.33);
	return fract((q.x + q.y) * q.z);
}

// Cheap stand-in for leaves(): about 80 % of the cells filled, no search.
// Each filled cell holds a round blob a little off its center (prompt 057:
// square cells cast checkerboard shadows onto the meadow).
vec2 leaves_cheap(vec2 uv, float cells) {
	vec2 g = uv * cells;
	vec2 id = floor(g);
	float h = hash2_fast(id);
	vec2 d = fract(g) - 0.5 - (vec2(h, fract(h * 13.7)) - 0.5) * 0.28;
	float rad = 0.56 + 0.14 * fract(h * 3.1);
	float blob = step(dot(d, d), rad * rad);
	return vec2(step(0.18, h) * blob, 0.72 + 0.54 * fract(h * 7.31));
}

// Shadow-only alpha test for leaf/needle cells (prompt 205): no shade, no sqrt, no sin.
float leaves_shadow(vec2 uv, float cells, float empty_cut) {
	vec2 g = uv * cells;
	float h = hash2_fast(floor(g));
	vec2 d = fract(g) - 0.5 - (vec2(h, fract(h * 13.7)) - 0.5) * 0.28;
	float rad = 0.58 + 0.14 * fract(h * 3.1);
	return step(empty_cut, h) * step(dot(d, d), rad * rad);
}

// One leaf per grid cell (random offset and angle); returns (alpha, shade).
vec2 leaves(vec2 uv, float cells, float aspect) {
	vec2 g = uv * cells;
	vec2 id = floor(g);
	float best = 0.0;
	float shade = 0.0;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 c = id + vec2(float(x), float(y));
			float h = hash2(c);
			vec2 center = c + vec2(hash2(c + 3.1), hash2(c + 7.7));
			float a = h * 6.2831;
			vec2 d = g - center;
			d = vec2(cos(a) * d.x + sin(a) * d.y, -sin(a) * d.x + cos(a) * d.y);
			float e = length(d * vec2(1.0, aspect));
			float inside = 1.0 - smoothstep(0.66, 0.72, e);
			if (inside > best) {
				best = inside;
				shade = 0.74 + 0.52 * hash2(c + 1.3) - abs(d.x) * 0.28;
			}
		}
	}
	return vec2(best, shade);
}

void fragment() {
	int pattern = int(floor(UV.x));
	vec2 uv = vec2(fract(UV.x), UV.y);
	vec2 c = uv - 0.5;
	// Directional shadow passes use an orthographic projection (prompt 205):
	// fast 2D silhouette + hash mask only, skipping 3D vnoise, albedo, snow and cloud lookups.
	if (PROJECTION_MATRIX[3][3] > 0.5) {
		if (pattern != 7 && pattern != 11) {
			float s_alpha = 0.0;
			if (pattern == 0 || pattern == 5 || pattern == 8 || pattern == 10 || pattern == 13 || pattern == 12) {
				float cells = pattern == 5 ? 11.0 : (pattern == 8 ? 26.0 : (pattern == 10 ? 15.0 : (pattern == 13 ? 7.0 : (pattern == 12 ? 10.0 : 8.0))));
				// Cheap organic lobe edge without 3D vnoise.
				float lobe = (c.x * c.y * 4.0) * (v_tint - 0.5) + (c.x * c.x - c.y * c.y) * 1.4;
				float edge = (pattern == 12 ? 0.43 : 0.46) + 0.055 * clamp(lobe, -1.0, 1.0);
				float cut = pattern == 10 ? 0.42 : (pattern == 12 ? 0.12 : 0.18);
				s_alpha = step(dot(c, c), edge * edge) * leaves_shadow(uv, cells, cut);
			} else if (pattern == 1 || pattern == 9) {
				float side = pattern == 9 ? abs(uv.x - 0.5) : abs(uv.y - 0.5);
				float along = pattern == 9 ? uv.y : uv.x;
				float w = pattern == 9 ? 0.5 * (1.0 - along) : 0.46 * (1.0 - along * 0.5);
				float ragged = 0.80 + 0.30 * hash2_fast(floor(uv * vec2(7.0, 11.0)) + v_tint);
				float tuft_cells = pattern == 9 ? 30.0 : 16.0;
				s_alpha = step(side, w * ragged) * leaves_shadow(uv, tuft_cells, 0.06);
				s_alpha = max(s_alpha, step(side, 0.015) * step(along, 0.97) * float(pattern == 1));
			} else if (pattern == 2) {
				float side = abs(uv.x - 0.5) * 2.0;
				float width = sin(3.1416 * clamp(uv.y * 1.05, 0.0, 1.0));
				float leaflets = abs(fract(uv.y * 34.0 + side * 3.5) - 0.5);
				s_alpha = max(step(side, width * 0.98) * step(leaflets, 0.38), step(side, 0.05));
			} else if (pattern == 3 || pattern == 4) {
				float n = 14.0;
				float col = floor(uv.x * n);
				float h = 0.55 + 0.45 * hash2_fast(vec2(col, v_tint * 7.0));
				float bend = (hash2_fast(vec2(col, 3.0)) - 0.5) * 0.8;
				float x = fract(uv.x * n + bend * uv.y * uv.y) - 0.5;
				float width = 0.42 * (1.0 - uv.y / h);
				s_alpha = step(abs(x), width) * step(uv.y, h);
			} else if (pattern == 6) {
				s_alpha = step(dot(c, c), 0.19) * step(0.62, hash2_fast(floor(uv * 10.0) + v_tint));
			}
			if (s_alpha < alpha_cut) {
				discard;
			}
		}
	} else {
	CLOUD_FRAGMENT(0.5)
	float r = length(c);
	vec3 p = v_local;
	float ao = COLOR.r;
	int kind = kind_of(COLOR.a);
	bool cheap = length(VERTEX) > detail_distance;
	float alpha = 0.0;
	float shade = 1.0;
	vec3 base = env_foliage.rgb;
	if (kind == 1) {
		base = env_foliage.rgb * vec3(0.7, 0.85, 0.85);
	} else if (kind == 2) {
		base = vec3(0.45, 0.36, 0.20);
	} else if (kind == 3) {
		// Dark dense hedgerow / bocage foliage (SLOT_KINDS Foliage[3] = "dark").
		base = env_foliage.rgb * vec3(0.72, 0.80, 0.66);
	}
	if (pattern == 0 || pattern == 5 || pattern == 8 || pattern == 10 || pattern == 13) {
		// broadleaf / shrub / far crown / olive / vine: leaf cells inside a lumpy cluster
		float cells = pattern == 5 ? 11.0 : (pattern == 8 ? 26.0 : (pattern == 10 ? 15.0 : (pattern == 13 ? 7.0 : 8.0)));
		float edge = 0.42 + 0.11 * vnoise(vec3(uv * 5.0, v_tint * 9.0));
		float cluster = 1.0 - smoothstep(edge - 0.05, edge, r);
		vec2 lf = cheap ? leaves_cheap(uv, cells) : leaves(uv, cells, 1.8);
		alpha = lf.x * cluster;
		shade = lf.y * (0.75 + 0.35 * (1.0 - r * 1.6));
		if (pattern == 13) {
			// Vine (prompt 202): large leaves in the fresh green of a tended
			// vineyard, whatever the biome's leaf color.
			base = mix(base, vec3(0.085, 0.20, 0.03), 0.7);
		}
		if (pattern == 10) {
			// Olive (prompt 120): small leaves, grey-green above and silver
			// below; the crown stays loose (fewer cells survive).
			base = mix(base, vec3(0.30, 0.34, 0.25), 0.7) * mix(0.9, 1.25, lf.y);
			alpha *= step(0.3, hash2(floor(uv * cells) + v_tint * 3.0));
		}
	} else if (pattern == 1 || pattern == 9) {
		// needle spray (1, along U) or far conifer silhouette (9, cone along V):
		// thin needle tufts in random directions inside a ragged outline
		float side = pattern == 9 ? abs(uv.x - 0.5) : abs(uv.y - 0.5);
		float along = pattern == 9 ? uv.y : uv.x;
		float w = pattern == 9 ? 0.5 * pow(1.0 - along, 0.9) : 0.46 * (1.0 - along * 0.5);
		float ragged = 0.7 + 0.45 * vnoise(vec3(uv * vec2(7.0, 11.0), v_tint * 5.0));
		float outline = step(side, w * ragged);
		float tuft_cells = pattern == 9 ? 30.0 : 16.0;
		vec2 tufts = cheap ? leaves_cheap(uv, tuft_cells) : leaves(uv, tuft_cells, 4.5);
		vec2 tufts2 = cheap ? leaves_cheap(uv + vec2(0.37, 0.11), tuft_cells)
				: leaves(uv + vec2(0.37, 0.11), tuft_cells, 4.5);
		alpha = max(tufts.x, tufts2.x) * outline;
		alpha = max(alpha, step(side, 0.015) * step(along, 0.97) * float(pattern == 1));
		shade = (0.7 + 0.4 * max(tufts.y, tufts2.y)) * (0.85 + 0.25 * (1.0 - side * 2.0));
		base *= vec3(0.62, 0.82, 0.78);
	} else if (pattern == 2) {
		// palm frond: rib along V, leaflets to both sides
		float side = abs(uv.x - 0.5) * 2.0;
		float width = sin(3.1416 * clamp(uv.y * 1.05, 0.0, 1.0));
		float leaflets = abs(fract(uv.y * 34.0 + side * 3.5) - 0.5);
		alpha = step(side, width * 0.98) * step(leaflets, 0.38);
		alpha = max(alpha, step(side, 0.05));
		shade = 0.75 + 0.35 * side;
		// Date palms are evergreen (prompt 089): a fresh green of their own
		// that the biome's dryness hardly touches (see `dryness` below).
		base = mix(base, vec3(0.16, 0.34, 0.08), 0.65);
	} else if (pattern == 3 || pattern == 4) {
		// grass blades (and flower heads)
		float n = 14.0;
		float col = floor(uv.x * n);
		float h = 0.55 + 0.45 * hash2(vec2(col, v_tint * 7.0));
		float bend = (hash2(vec2(col, 3.0)) - 0.5) * 0.8;
		float x = fract(uv.x * n + bend * uv.y * uv.y) - 0.5;
		float width = 0.42 * (1.0 - uv.y / h);
		alpha = step(abs(x), width) * step(uv.y, h);
		shade = 0.7 + 0.5 * uv.y;
		base = mix(base * 0.85, base * 1.15 + vec3(0.04, 0.03, 0.0), uv.y);
		if (pattern == 4) {
			vec2 g = uv * vec2(6.0, 6.0);
			vec2 id = floor(g);
			float pick = hash2(id + v_tint);
			vec2 d = fract(g) - 0.5;
			float head = step(length(d), 0.22) * step(0.55, uv.y) * step(0.45, pick);
			if (head > 0.5) {
				alpha = 1.0;
				base = pick > 0.8 ? vec3(0.85, 0.18, 0.12) : (pick > 0.62 ? vec3(0.9, 0.9, 0.85) : vec3(0.9, 0.75, 0.15));
				shade = 1.0;
			}
		}
	} else if (pattern == 6) {
		// bare twigs: thin ridged lines
		float n1 = abs(vnoise(vec3(uv * vec2(5.0, 9.0), v_tint * 3.0)) - 0.5);
		float n2 = abs(vnoise(vec3(uv * vec2(11.0, 7.0), v_tint * 3.0 + 4.0)) - 0.5);
		float edge = 1.0 - smoothstep(0.38, 0.48, r);
		alpha = max(step(n1, 0.035), step(n2, 0.025)) * edge;
		base = vec3(0.20, 0.16, 0.12);
		shade = 1.0;
	} else if (pattern == 11) {
		// Succulents (prompt 202): solid pads and leaves. Kind 0 prickly
		// pear green, 1 agave grey-green, 2 dry stalk, 3 fruit (linear
		// colors, like every color in this shader).
		alpha = 1.0;
		shade = 0.82 + 0.22 * vnoise(vec3(uv * 7.0, v_tint * 4.0));
		if (kind == 0) {
			base = vec3(0.070, 0.165, 0.038);
		} else if (kind == 1) {
			base = vec3(0.135, 0.215, 0.170);
		} else if (kind == 3) {
			base = vec3(0.45, 0.035, 0.02);
		}
	} else if (pattern == 12) {
		// Stone pine (prompt 202): round tufts of long needles, one per
		// cell, inside a lumpy cluster; reads as needles, not as leaves.
		float edge = 0.40 + 0.10 * vnoise(vec3(uv * 5.0, v_tint * 9.0));
		float cluster = 1.0 - smoothstep(edge - 0.05, edge, r);
		vec2 g = uv * 10.0;
		vec2 id = floor(g);
		float h = hash2(id + v_tint * 3.0);
		vec2 d = fract(g) - 0.5 - (vec2(h, hash2(id + 5.3)) - 0.5) * 0.25;
		float rr = length(d);
		float spoke = abs(fract(atan(d.y, d.x) / 6.2831 * 11.0 + h) - 0.5);
		// Far away and in shadow passes the needles are a little wider,
		// so the crown keeps its density.
		float tuft = step(rr, 0.55) * max(step(rr, 0.16), step(spoke, cheap ? 0.33 : 0.26));
		alpha = tuft * cluster * step(0.12, h);
		shade = (0.72 + 0.4 * h - rr * 0.35) * (0.8 + 0.3 * (1.0 - r * 1.6));
		base *= vec3(0.62, 0.82, 0.78);
	} else {
		// 7: solid hedge core (prompt 218: mottled organic leaf texture, no box stripes)
		alpha = 1.0;
		float lf_core = cheap ? leaves_cheap(uv * 2.4, 7.0).y : leaves(uv * 2.4, 7.0, 1.5).y;
		float lumps = vnoise(p * 3.2 + vec3(v_tint * 7.0));
		shade = (0.58 + 0.32 * lf_core) * mix(0.76, 1.14, lumps);
	}
	if (alpha < alpha_cut) {
		discard;
	}
	// biome: dryness pushes greens to olive/brown
	float dryness = env_dry * ((pattern == 2 || pattern == 11 || pattern == 13) ? 0.2 : 1.0);
	vec3 dry = mix(base, vec3(0.50, 0.44, 0.26) * (0.8 + 0.4 * v_tint), dryness);
	vec3 albedo = (pattern == 4 && shade == 1.0 && alpha == 1.0) ? base : dry;
	albedo *= shade * (0.85 + 0.3 * v_tint);
	// Volumetric crown & hedgerow shading (prompt 218): COLOR.g encodes baked
	// height fraction (0.35..1.0 from bottom to top). Upper sunlit surfaces
	// warm gently toward olive-green, while undersides and hedge bases drop
	// into deep shade so crowns and bocage walls read as solid volumes.
	vec3 rest = normalize(v_rest_normal);
	float crown_h = clamp((COLOR.g - 0.35) / 0.65, 0.0, 1.0);
	bool volumetric = pattern == 0 || pattern == 5 || pattern == 7 || pattern == 8;
	if (volumetric) {
		float top_w = smoothstep(-0.25, 0.72, rest.y) * smoothstep(0.22, 0.90, crown_h);
		albedo *= mix(vec3(0.84, 0.92, 0.90), vec3(1.14, 1.12, 0.88), top_w);
		// Stronger contact shading near the foot of hedges and low shrubs.
		float base_ao = (pattern == 7 || pattern == 5) ? mix(0.42, 1.0, smoothstep(0.0, 0.55, crown_h)) : mix(0.68, 1.0, crown_h);
		ao *= base_ao;
	}
	float rough = 0.85;
	// snow sits on needles and on the tops of dense cards only
	// (prompt 056: needle branches hold more, on all but their undersides;
	// the normal of the standing tree keeps the snow on a fallen one)
	// Far crowns (9) are flat cards: less snow, or they read as white flags.
	bool needles = pattern == 1 || pattern == 12;
	float snow_amount = env_snow * (needles ? 1.0 : (pattern == 9 ? 0.5 : 0.45));
	vec3 up = needles ? vec3(0.0, 0.62 + 0.4 * max(rest.y, 0.0), 0.0) : rest * vec3(1.0, 0.85, 1.0);
	albedo = snow_cover(albedo, rough, up, p, snow_amount);
	albedo = charred(albedo, env_char);
	ALBEDO = albedo * mix(0.28, 1.05, ao);
	AO = ao;
	AO_LIGHT_AFFECT = 0.45;
	ROUGHNESS = rough;
	BACKLIGHT = albedo * (volumetric ? 0.22 * (0.35 + 0.65 * crown_h) : 0.35);
	}
}
