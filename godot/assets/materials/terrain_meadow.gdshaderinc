// Meadow look of the grass layer (prompt 057), included by terrain.gdshader.
// Presentation only, everything procedural.
//
// `meadow_apply` runs after the layers are blended and before wet ground and
// snow (so weather drawn later in terrain.gdshader still lies on top). It
// works only where grass is drawn (`grass` share) and only in biomes with
// `meadow_strength` > 0 (forest; desert and winter draw the grass layer as
// sand or snow):
//
// - Color in four scales: the dry/lush areas of the macro noise (hundreds of
//   meters), clouds of tens of meters, blotches of a few meters and speckles
//   of single tufts. The grass texture keeps only its light/dark pattern
//   (blades and gaps, contrast raised), the hue comes from a palette:
//   lush green, dry straw, damp dark green.
// - Bare soil patches of a few meters and frayed soil at the edges of field
//   tracks.
// - Blades (near only): two layers of short strokes in 4.5 / 3.2 cm cells,
//   bright tips, dark gaps, a common lean that changes over tens of meters
//   (flattened along field tracks). Antialiased by the pixel footprint and
//   skipped where a pixel is larger than about 2.5 cm.
// - Flowers (near and middle): white daisies and yellow dandelions as tiny
//   flat dots in loose groups, `flower_density` per biome (0 in desert and
//   winter). Skipped where a pixel is larger than about 6 cm.

uniform float meadow_strength = 1.0;
uniform float flower_density = 1.0;
uniform vec4 meadow_soil : source_color = vec4(0.25, 0.19, 0.13, 1.0);
// Contrast of the grass texture's light/dark pattern (blades vs. gaps).
uniform float meadow_contrast = 1.5;
// Share of the ground that is bare soil in patches, 0..1.
uniform float meadow_soil_amount = 0.5;

#include "res://assets/shaders/meadow_palette.gdshaderinc"

// One layer of blades: cells of `size` meters, one stroke per cell from a
// random root along `lean` (+- spread). Looks at the cell of the pixel and
// the three neighbors towards it. Returns (coverage, position along the
// blade 0 root..1 tip, signed distance across in widths).
vec3 m_blades(vec2 q, float size, vec2 lean, float spread, float px, float seed) {
	vec2 g = q / size;
	vec2 id = floor(g);
	vec2 f = g - id;
	vec2 side = vec2(f.x < 0.5 ? -1.0 : 1.0, f.y < 0.5 ? -1.0 : 1.0);
	vec3 best = vec3(0.0, 0.0, 0.0);
	for (int k = 0; k < 4; k++) {
		vec2 c = id + vec2(float(k & 1), float(k >> 1)) * side;
		vec2 h = m_hash2(c + seed);
		vec2 root = (c + h) * size;
		float a = (h.x - 0.5) * spread + (h.y - 0.5) * 0.6;
		vec2 dir = vec2(lean.x * cos(a) - lean.y * sin(a), lean.x * sin(a) + lean.y * cos(a));
		float len = size * (1.2 + 0.9 * fract(h.y * 7.13));
		vec2 d = q - root;
		float t = clamp(dot(d, dir) / len, 0.0, 1.0);
		vec2 off = d - dir * t * len;
		float across = off.x * dir.y - off.y * dir.x;
		float w = size * 0.06 * (1.0 - 0.75 * t);
		// Thin blades fade to their share of the pixel instead of flickering.
		float we = max(w, px * 0.5);
		float cov = clamp((we - abs(across)) / px + 0.5, 0.0, 1.0) * (w / we);
		if (cov > best.x) {
			best = vec3(cov, t, across / we);
		}
	}
	return best;
}

// Clumps of grass on the ground: cells of `size` meters, bright in the
// middle of a clump, dark in the gaps between (2 x 2 nearest cells).
float m_clumps(vec2 q, float size) {
	vec2 g = q / size;
	vec2 id = floor(g);
	vec2 f = g - id;
	vec2 side = vec2(f.x < 0.5 ? -1.0 : 1.0, f.y < 0.5 ? -1.0 : 1.0);
	float d = 9.0;
	for (int k = 0; k < 4; k++) {
		vec2 c = id + vec2(float(k & 1), float(k >> 1)) * side;
		vec2 center = c + 0.15 + 0.7 * m_hash2(c + 5.7);
		d = min(d, length(g - center));
	}
	return 1.0 - smoothstep(0.15, 0.75, d);
}

// Flowers: at most one per cell of 0.35 m, in loose groups. Returns
// (coverage, kind 0 daisy / 1 dandelion, center share).
vec3 m_flowers(vec2 p, float groups, float px) {
	float fsize = 0.35;
	vec2 id = floor(p / fsize);
	vec2 h = m_hash2(id * 1.7 + 11.0);
	if (h.x > groups) {
		return vec3(0.0);
	}
	vec2 center = (id + 0.2 + 0.6 * m_hash2(id + 3.3)) * fsize;
	float r = 0.016 + 0.012 * fract(h.y * 5.31);
	float kind = step(0.62, fract(h.y * 13.7));
	r *= mix(1.0, 1.25, kind);
	float d = length(p - center);
	float re = max(r, px * 0.5);
	float cov = clamp((re - d) / px + 0.5, 0.0, 1.0) * (r * r) / (re * re);
	return vec3(cov, kind, 1.0 - smoothstep(r * 0.3, r * 0.45, d));
}

// grass: share of the grass layer (0..1); grass_tex: blended grass texture
// color here; grass_mean: its average color; track_w: weight of field
// tracks; along: track direction; macro_far/near: the shader's macro
// samples; px: pixel footprint in meters.
void meadow_apply(vec2 p, float grass, vec3 grass_tex, vec3 grass_mean, float track_w, vec2 along,
		vec4 macro_far, vec4 macro_near, float px, inout vec3 albedo, inout vec3 detail,
		inout float rough) {
	if (meadow_strength <= 0.0 || grass <= 0.0) {
		return;
	}
	vec4 m_mid = meadow_mid(p);
	vec4 m_small = meadow_small(p);
	float speck = m_vnoise(p / 0.3);
	float dry;
	vec3 base = meadow_base(p, macro_far.g, m_mid, m_small, dry);

	// Light/dark pattern of the texture (blades, gaps), contrast raised.
	float shade = m_luma(grass_tex) / max(m_luma(grass_mean), 1e-3);
	shade = pow(max(shade, 0.0), meadow_contrast);
	vec3 meadow = base * shade;

	// Blades near the camera.
	if (px < 0.025) {
		float fade = 1.0 - smoothstep(0.01, 0.025, px);
		// Common lean over tens of meters; along field tracks the grass is
		// pressed flat in the driving direction.
		float lean_a = (m_mid.g - 0.5) * 9.0 + (macro_near.a - 0.5) * 3.0;
		vec2 lean = vec2(cos(lean_a), sin(lean_a));
		float flat_ground = smoothstep(0.15, 0.5, track_w);
		lean = normalize(mix(lean, along * sign(dot(along, lean) + 1e-3), flat_ground));
		float spread = mix(1.6, 0.5, flat_ground);
		// Clumps: the meadow is tufty, not a lawn.
		float clump = m_clumps(p, 0.16);
		meadow *= mix(1.0, mix(0.6, 1.2, clump), fade);
		vec3 b1 = m_blades(p, 0.045, lean, spread, px, 0.0);
		vec3 b2 = m_blades(p + vec2(0.017, 0.031), 0.032, lean, spread, px, 17.0);
		vec3 b = b1.x >= b2.x ? b1 : b2;
		float cov = b.x * fade;
		// Gaps darker (the ground between blades lies in their shade),
		// blades lighter towards the tip, a little yellower at the tip.
		vec3 tip = base * vec3(1.25, 1.2, 0.95);
		vec3 blade = mix(base * 0.8, tip, b.y) * mix(0.9, 1.1, fract(b.y * 31.7 + b.z));
		meadow = mix(meadow * mix(1.0, 0.55, fade), blade * 1.2, cov);
		// Rounded blades: tilt the normal across the blade.
		vec2 across_dir = vec2(-lean.y, lean.x);
		vec2 dn = across_dir * clamp(b.z, -1.0, 1.0) * 0.5 * cov;
		detail += vec3(dn.x, 0.0, dn.y) * grass;
	}

	// Tufts of the middle zoom: clumps of about 0.45 m, bright tops and dark
	// gaps, where a pixel is 1.5 to 8 cm (the blades above take over nearer).
	if (px > 0.012 && px < 0.08) {
		float mid_fade = smoothstep(0.012, 0.025, px) * (1.0 - smoothstep(0.05, 0.08, px));
		float tuft = m_clumps(p + vec2(0.31, 0.77), 0.45);
		meadow *= mix(1.0, mix(0.72, 1.12, tuft), mid_fade);
	}

	// Bare soil: patches of a few meters, frayed edges of field tracks.
	float soil_n = m_small.b * 0.65 + m_vnoise(p / 0.9) * 0.35;
	float soil = smoothstep(0.66, 0.72, soil_n + (meadow_soil_amount - 0.5) * 0.2) * meadow_soil_amount;
	float fray = smoothstep(0.08, 0.3, track_w) * (1.0 - smoothstep(0.45, 0.7, track_w));
	soil = max(soil, fray * smoothstep(0.45, 0.65, speck * 0.6 + m_small.g * 0.4));
	vec3 soil_col = meadow_soil.rgb * mix(0.75, 1.15, speck) * mix(0.8, 1.1, shade);
	// A few blades stay standing in the soil.
	meadow = mix(meadow, soil_col, soil * smoothstep(1.4, 0.7, shade));

	// Flowers.
	if (px < 0.06 && flower_density > 0.0) {
		float groups = smoothstep(0.45, 0.75, m_mid.a * 0.4 + m_small.a * 0.6) * 0.7 + 0.05;
		groups *= flower_density * (1.0 - dry * 0.7) * (1.0 - soil) * (1.0 - smoothstep(0.1, 0.3, track_w));
		vec3 fl = m_flowers(p, groups, px);
		float cov = fl.x * (1.0 - smoothstep(0.03, 0.06, px));
		vec3 petals = mix(vec3(0.85, 0.85, 0.8), vec3(0.85, 0.66, 0.05), fl.y);
		vec3 heart = mix(vec3(0.8, 0.58, 0.05), vec3(0.75, 0.52, 0.03), fl.y);
		meadow = mix(meadow, mix(petals, heart, fl.z), cov);
	}

	float k = grass * meadow_strength;
	albedo = mix(albedo, meadow, k);
	rough = mix(rough, 0.92, k);
}
