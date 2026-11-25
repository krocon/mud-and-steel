// Fields (prompt 088), included by terrain.gdshader: ripe grain, green
// fields in rows and ploughed soil. Masks and row direction come from
// `field_tex` (crates/bridge/src/terrain_layers.rs: R grain, G green, B
// ploughed, A direction of the rows, the long axis of each field, 0..180°).
// Rows and furrows run along that direction; every stripe is filtered for
// the pixel footprint (`field_band`) and fades into its mean color before it
// gets smaller than a pixel, so nothing shimmers far away or when the camera
// moves. No textures. Colors are linear (the shader's ALBEDO), not sRGB.
// Presentation only.

uniform sampler2D field_tex : filter_linear, repeat_disable;
// Base colors per biome (prompt 120, BiomePreset field_*; linear): ripe
// grain or stubble, leaves of green fields, the soil between their rows and
// ploughed soil.
uniform vec3 field_grain = vec3(0.6, 0.39, 0.09);
uniform vec3 field_leaf = vec3(0.04, 0.13, 0.015);
uniform vec3 field_soil = vec3(0.1, 0.055, 0.026);
uniform vec3 field_earth = vec3(0.14, 0.065, 0.032);

// Share of a stripe of half width `half_w` around centers `period` apart at
// coordinate `u`, filtered for a pixel `px` meters wide.
float field_band(float u, float period, float half_w, float px) {
	float d = abs(fract(u / period + 0.5) - 0.5) * period;
	float sharp = clamp((half_w - d) / max(px, 1e-4) + 0.5, 0.0, 1.0);
	float mean = 2.0 * half_w / period;
	return mix(sharp, mean, smoothstep(period * 0.2, period * 0.6, px));
}

// Sparse small irregular stubble/weed tufts in a grain/stubble field
// (prompt 218), strictly antialiased and faded out before they could shimmer.
float field_stubble_dots(vec2 p, vec2 along, float fine, float px) {
	if (px > 0.42) {
		return 0.0;
	}
	float fade = 1.0 - smoothstep(0.14, 0.42, px);
	float cell_m = 2.4;
	vec2 g = p / cell_m;
	vec2 id = floor(g);
	vec2 h = fract(sin(vec2(dot(id, vec2(127.1, 311.7)), dot(id, vec2(269.5, 183.3)))) * 43758.5453);
	if (h.x < 0.80) {
		return 0.0;
	}
	vec2 center = (id + 0.18 + 0.64 * h) * cell_m;
	vec2 delta = p - center;
	// Slightly elongated along the crop row so it reads as a stray tuft, not a dot.
	vec2 across = vec2(-along.y, along.x);
	vec2 local = vec2(dot(delta, along) * 0.55, dot(delta, across) * 1.25);
	float r = 0.08 + 0.07 * fract(h.y * 17.3);
	float d = length(local);
	float re = max(r, px * 0.6);
	float cov = clamp((re - d) / max(px, 1e-4) + 0.5, 0.0, 1.0) * (r * r) / (re * re);
	return cov * fade * mix(0.6, 1.0, fine);
}

// Blends the fields into albedo, detail normal (world xz) and roughness.
// Returns how much row pattern the point shows (for the snow on top).
// `fine`: a noise of a few meters (texture of the caller) for mottling.
float field_apply(vec2 p, vec3 m, float angle01, vec4 macro_far, vec4 macro_near, float fine, float px,
		inout vec3 albedo, inout vec3 detail, inout float rough) {
	float total = m.r + m.g + m.b;
	float a = angle01 * PI;
	vec2 along = vec2(cos(a), sin(a));
	vec2 across = vec2(-along.y, along.x);
	float u = dot(p, across);
	float vary = macro_near.g - 0.5;
	vec3 across3 = vec3(across.x, 0.0, across.y);
	vec3 col = vec3(0.0);
	vec3 dn = vec3(0.0);
	float r = 0.0;
	float rows = 0.0;
	if (m.r > 0.0) {
		// Grain / stubble (prompt 088, 218): warm pale straw with crisp regular
		// harvest rows legible from game height, fine drill rows close up,
		// tramlines every 20 m and sparse subtle stubble tufts.
		float drill = field_band(u, 0.30, 0.07, px);
		float swath = field_band(u, 1.40, 0.26, px);
		float tram = max(field_band(u, 20.0, 0.22, px), field_band(u + 1.8, 20.0, 0.22, px));
		vec3 grain = field_grain * (0.92 + 0.16 * macro_far.r) + vec3(0.035, 0.025, -0.008) * vary;
		grain *= 1.0 - 0.11 * drill - 0.24 * swath;
		grain = mix(grain, field_soil * 1.25, tram * 0.55);
		grain *= 1.0 - 0.09 * smoothstep(0.68, 0.82, macro_near.r);
		// Ears and stalks: mottled light and shade of a few meters.
		grain *= mix(0.90, 1.07, fine);
		float dots = field_stubble_dots(p, along, fine, px);
		grain = mix(grain, field_soil * 0.65, dots * 0.42);
		col += grain * m.r;
		r += 0.86 * m.r;
		rows += (0.3 * drill + 0.55 * swath + tram) * m.r;
		float swath_n = (swath - 0.37) * 0.38 * (1.0 - smoothstep(0.25, 0.78, px));
		dn += across3 * ((drill - 0.5) * 0.28 * (1.0 - smoothstep(0.05, 0.2, px)) + swath_n) * m.r;
	}
	if (m.g > 0.0) {
		// Green field (prompt 088, 218): fresh lighter green crop rows over
		// darker soil furrows, legible from game height without shimmering.
		float row = field_band(u, 1.10, 0.34, px);
		float fine_row = field_band(u, 0.36, 0.10, px);
		vec3 leaf = field_leaf * (0.90 + 0.22 * macro_far.g) * mix(0.90, 1.10, fine) + vec3(0.012, 0.018, 0.004) * vary;
		leaf *= 1.0 - 0.08 * fine_row;
		vec3 soil = mix(field_soil, field_leaf * 0.48, 0.42);
		col += mix(soil, leaf, row) * m.g;
		r += mix(0.93, 0.74, row) * m.g;
		rows += row * m.g;
		dn += across3 * cos(u / 1.10 * 6.2832) * 0.32 * (1.0 - smoothstep(0.16, 0.55, px)) * m.g;
	}
	if (m.b > 0.0) {
		// Ploughed soil (prompt 088, 218): rich dark furrows with medium ridges
		// visible from game height, fine ridges up close and broad plough passes.
		float phase = u / 0.36 * 6.2832;
		float fade = 1.0 - smoothstep(0.07, 0.25, px);
		float ridge = 0.5 + 0.5 * sin(phase);
		float furrow = field_band(u, 1.25, 0.34, px);
		vec3 earth = field_earth * (0.86 + 0.24 * macro_far.r) + vec3(0.015, 0.005, -0.006) * vary;
		earth *= mix(0.92, 1.06, mix(0.5, ridge, fade));
		earth *= mix(0.84, 1.12, furrow);
		// Broad passes of the plough, seen from afar.
		earth *= 1.0 - 0.07 * field_band(u, 5.5, 1.4, px);
		col += earth * m.b;
		r += 0.95 * m.b;
		rows += max(ridge * fade, furrow * 0.75) * m.b;
		float fur_n = (furrow - 0.54) * 0.45 * (1.0 - smoothstep(0.18, 0.65, px));
		dn += across3 * (cos(phase) * 0.55 * fade + fur_n) * m.b;
	}
	// A crisp rectangular edge (unwarped field_tex, 3 x 3 tent filter).
	float k = smoothstep(0.42, 0.58, total);
	float inv = 1.0 / max(total, 1e-4);
	albedo = mix(albedo, col * inv, k);
	rough = mix(rough, r * inv, k);
	detail = mix(detail, dn * inv, k);
	return clamp(rows * inv, 0.0, 1.0) * k;
}
