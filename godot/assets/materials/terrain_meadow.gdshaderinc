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
// - Blades (near only): short strokes in 3.6 cm cells,
//   bright tips, dark gaps, a common lean that changes over tens of meters
//   (flattened along field tracks). Antialiased by the pixel footprint and
//   skipped where a pixel is larger than about 2.5 cm.
// - Relief (middle and far, prompt 191): grass clumps in three scales
//   (0.45, 1.1 and 2.6 m) with dark gaps, lit through the normal, lower
//   ambient occlusion in the gaps and a short fake shadow away from the sun
//   (`ground_sun`, `ground_relief`, set by atmosphere.gd by quality; off
//   where they are 0). Each scale fades
//   in where a pixel resolves it and out before it shimmers, so the ground
//   stays tufty from the middle zoom to the overview.
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

// Grass clumps (prompt 191): terrain_clumps.png from `just terrain-textures`,
// R height, GB gradient per cell (0.5 + g / 8), 32 x 32 cells per tile.
// Mipmapped: far away height and gradient average out by themselves.
uniform sampler2D clump_tex : filter_linear_mipmap, repeat_enable;

// Height of grass clumps in cells of `size` meters, the tile turned by
// `angle` (radians) against visible repeats; dx, dy: screen derivatives
// of p. Returns (height 0..1,
// gradient per meter in x and z).
vec3 m_clump_h(vec2 p, vec2 dx, vec2 dy, float size, float angle) {
	float ca = cos(angle);
	float sa = sin(angle);
	vec2 q = vec2(ca * p.x - sa * p.y, sa * p.x + ca * p.y);
	float k = 1.0 / (32.0 * size);
	vec2 qdx = vec2(ca * dx.x - sa * dx.y, sa * dx.x + ca * dx.y) * k;
	vec2 qdy = vec2(ca * dy.x - sa * dy.y, sa * dy.x + ca * dy.y) * k;
	// Explicit derivatives: called inside branches.
	vec4 t = textureGrad(clump_tex, q * k, qdx, qdy);
	vec2 gq = (t.gb - 0.5) * 8.0 / size;
	// Gradient back from the turned frame.
	vec2 g = vec2(ca * gq.x + sa * gq.y, -sa * gq.x + ca * gq.y);
	return vec3(t.r, g);
}

// Global sun for the fake clump shadows (prompt 191, set by atmosphere.gd):
// xyz direction towards the sun, w strength (0 = off: thumbnails, editor,
// quality below high).
global uniform vec4 ground_sun;
// Clump relief on (1) or off (0: thumbnails, editor, low quality).
global uniform float ground_relief;

// One scale of the relief: returns the albedo factor, adds to the normal
// detail and lowers the ambient occlusion. `height` is the clump height
// in meters, `fade` its weight at this pixel size.
float m_relief_layer(vec2 p, vec2 dx, vec2 dy, float size, float height, float angle, float fade, float sun_k,
		inout vec3 detail, inout float ao) {
	if (fade <= 0.0) {
		return 1.0;
	}
	vec3 c = m_clump_h(p, dx, dy, size, angle);
	// Gaps between clumps are dark (the ground there lies in their shade),
	// tops a little brighter.
	float shade = mix(0.66, 1.16, smoothstep(0.0, 0.7, c.x));
	ao *= mix(1.0, mix(0.9, 1.0, c.x), fade);
	// Lit through the normal: slope of the dome times its height.
	detail += vec3(-c.y, 0.0, -c.z) * height * 0.5 * fade;
	// Fake shadow: a clump towards the sun hides this point.
	if (sun_k > 0.0) {
		vec2 to_sun = ground_sun.xz;
		float hl = length(to_sun);
		if (hl > 1e-3) {
			float tan_el = max(ground_sun.y, 0.08) / hl;
			float reach = min(height / tan_el, size * 0.7);
			vec3 o = m_clump_h(p + to_sun / hl * reach, dx, dy, size, angle);
			float shadow = clamp((o.x - c.x) * 2.2, 0.0, 1.0);
			shade *= 1.0 - shadow * 0.55 * sun_k;
		}
	}
	// Keep the mean brightness: gaps, shadows and the tilted normals take about 20 %.
	return mix(1.0, shade * 1.22, fade);
}

// Relief of the meadow beyond the blades: three scales of clumps, each
// faded in where a pixel resolves it and out before it shimmers.
// `patch`: smooth noise of tens of meters (0..1), the relief is stronger in
// some places than in others.
float meadow_relief(vec2 p, vec2 dx, vec2 dy, float px, float grass, float patch, float sun_k,
		inout vec3 detail, inout float ao) {
	// A clump stays about 6 to 15 pixels wide: each scale takes over from
	// the finer one as the pixels grow (at most two at a time, each costs
	// two texture reads). Larger on screen they read as bumps, not grass.
	float f1 = smoothstep(0.012, 0.025, px) * (1.0 - smoothstep(0.06, 0.09, px));
	float f2 = smoothstep(0.06, 0.09, px) * (1.0 - smoothstep(0.16, 0.24, px));
	float f3 = smoothstep(0.16, 0.24, px) * (1.0 - smoothstep(0.6, 1.0, px));
	float vary = mix(0.6, 1.2, smoothstep(0.3, 0.7, patch));
	f1 *= vary;
	f2 *= vary;
	f3 *= vary;
	vec3 d = vec3(0.0);
	float a = 1.0;
	float k = 1.0;
	k *= m_relief_layer(p + vec2(0.31, 0.77), dx, dy, 0.45, 0.12, 0.3, f1, sun_k, d, a);
	k *= m_relief_layer(p, dx, dy, 1.1, 0.25, 1.9, f2, sun_k, d, a);
	k *= m_relief_layer(p + vec2(3.7, 1.9), dx, dy, 2.6, 0.45, 4.1, f3 * 0.75, sun_k, d, a);
	detail += d * grass;
	ao = mix(ao, ao * max(a, 0.7), grass);
	// The scales overlap: never darker than 0.6 in all their gaps at once.
	return clamp(k, 0.6, 1.35);
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
// samples; px: pixel footprint in meters; dpdx, dpdy: screen derivatives of
// p; sun_k: strength of the fake clump
// shadows (0..1, direct sunlight here); relief_w: weight of the clump
// relief (0 under tall grass).
void meadow_apply(vec2 p, float grass, vec3 grass_tex, vec3 grass_mean, float track_w, vec2 along,
		vec4 macro_far, vec4 macro_near, float px, vec2 dpdx, vec2 dpdy, float sun_k, float relief_w, inout vec3 albedo, inout vec3 detail,
		inout float rough, inout float ao) {
	if (meadow_strength <= 0.0 || grass <= 0.0) {
		return;
	}
	vec4 m_mid = meadow_mid(p);
	// Blotches of a few meters only where they are larger than a few pixels
	// (overview: two texture reads less per pixel).
	vec4 m_small = px < 0.25 ? mix(meadow_small(p), vec4(0.5), smoothstep(0.15, 0.25, px)) : vec4(0.5);
	// Speckles and the fine soil noise only where a pixel resolves them
	// (they would shimmer beyond and cost for nothing).
	float speck = px < 0.3 ? mix(m_vnoise(p / 0.3), 0.5, smoothstep(0.1, 0.3, px)) : 0.5;
	float dry;
	vec3 base = meadow_base(p, macro_far.g, m_mid, m_small, speck, dry);

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
		vec3 b = m_blades(p, 0.036, lean, spread, px, 0.0);
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

	// Relief of the middle and far zoom (prompt 191): clumps of 0.45, 1.1
	// and 2.6 m with dark gaps, lit through the normal and casting a short
	// fake shadow away from the sun (the blades above take over nearer).
	if (px > 0.012 && ground_relief > 0.0) {
		meadow *= mix(1.0, meadow_relief(p, dpdx, dpdy, px, grass * relief_w, m_mid.b, sun_k, detail, ao), relief_w);
	}

	// Bare soil: patches of a few meters, frayed edges of field tracks.
	float soil_fine = px < 0.6 ? mix(m_vnoise(p / 0.9), 0.5, smoothstep(0.3, 0.6, px)) : 0.5;
	float soil_n = m_small.b * 0.65 + soil_fine * 0.35;
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
