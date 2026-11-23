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
		// Grain: warm yellow, fine drill rows close up, tramlines (two
		// wheel tracks 1.8 m apart every 20 m), a few lodged patches.
		float fine = field_band(u, 0.25, 0.06, px);
		float tram = max(field_band(u, 20.0, 0.22, px), field_band(u + 1.8, 20.0, 0.22, px));
		vec3 grain = field_grain * (0.88 + 0.22 * macro_far.r) + vec3(0.05, 0.03, -0.01) * vary;
		grain *= 1.0 - 0.14 * fine;
		grain = mix(grain, vec3(0.13, 0.095, 0.035), tram * 0.75);
		grain *= 1.0 - 0.16 * smoothstep(0.64, 0.78, macro_near.r);
		// Ears and stalks: mottled light and shade of a few meters.
		grain *= mix(0.84, 1.1, fine);
		col += grain * m.r;
		r += 0.85 * m.r;
		rows += (0.4 * fine + tram) * m.r;
		dn += across3 * (fine - 0.5) * 0.3 * (1.0 - smoothstep(0.05, 0.2, px)) * m.r;
	}
	if (m.g > 0.0) {
		// Green field: low rows of leaves over brown soil.
		float row = field_band(u, 0.62, 0.15, px);
		vec3 leaf = field_leaf * (0.85 + 0.3 * macro_far.g) * mix(0.85, 1.12, fine) + vec3(0.01, 0.015, 0.0) * vary;
		vec3 soil = field_soil;
		col += mix(soil, leaf, row) * m.g;
		r += mix(0.95, 0.72, row) * m.g;
		rows += row * m.g;
		dn += across3 * cos(u / 0.62 * 6.2832) * 0.3 * (1.0 - smoothstep(0.08, 0.3, px)) * m.g;
	}
	if (m.b > 0.0) {
		// Ploughed soil: furrows whose ridges tilt the normal across them,
		// so they throw light and shade in a raking sun.
		float phase = u / 0.36 * 6.2832;
		float fade = 1.0 - smoothstep(0.07, 0.25, px);
		float ridge = 0.5 + 0.5 * sin(phase);
		vec3 earth = field_earth * (0.84 + 0.26 * macro_far.r) + vec3(0.02, 0.0, -0.01) * vary;
		earth *= mix(0.92, 1.06, mix(0.5, ridge, fade));
		// Broad passes of the plough, seen from afar.
		earth *= 1.0 - 0.05 * field_band(u, 6.0, 1.5, px);
		col += earth * m.b;
		r += 0.95 * m.b;
		rows += ridge * fade * m.b;
		dn += across3 * cos(phase) * 0.6 * fade * m.b;
	}
	// A clear edge (the masks are filtered over 2 m samples).
	float k = smoothstep(0.35, 0.65, total);
	float inv = 1.0 / max(total, 1e-4);
	albedo = mix(albedo, col * inv, k);
	rough = mix(rough, r * inv, k);
	detail = mix(detail, dn * inv, k);
	return clamp(rows * inv, 0.0, 1.0) * k;
}
