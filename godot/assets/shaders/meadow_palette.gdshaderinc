// Meadow palette (prompt 057), shared by the terrain (terrain_meadow.gdshaderinc)
// and the grass tufts (meadow_tuft.gdshader), so tufts take the color of the
// ground they stand on. Needs `macro_tex` (terrain_macro.png) declared before
// the include.

uniform vec4 meadow_lush : source_color = vec4(0.31, 0.43, 0.16, 1.0);
uniform vec4 meadow_dry : source_color = vec4(0.60, 0.55, 0.32, 1.0);
uniform vec4 meadow_damp : source_color = vec4(0.19, 0.30, 0.12, 1.0);

float m_hash(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * 0.1031);
	q += dot(q, q.yzx + 33.33);
	return fract((q.x + q.y) * q.z);
}

vec2 m_hash2(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
	q += dot(q, q.yzx + 33.33);
	return fract((q.xx + q.yz) * q.zy);
}

float m_vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = m_hash(i);
	float b = m_hash(i + vec2(1.0, 0.0));
	float c = m_hash(i + vec2(0.0, 1.0));
	float d = m_hash(i + vec2(1.0, 1.0));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float m_luma(vec3 c) {
	return dot(c, vec3(0.3, 0.59, 0.11));
}

// Macro samples of the palette: two de-correlated samples each (turned and
// of incommensurate size), so the clouds of tens of meters and the blotches
// of a few meters show no repeat even in the overview.
vec4 meadow_mid(vec2 p) {
	vec2 q = vec2(0.8 * p.x - 0.6 * p.y, 0.6 * p.x + 0.8 * p.y);
	return 0.5 * (texture(macro_tex, p / 29.0 + vec2(0.61, 0.13))
		+ texture(macro_tex, q / 47.0 + vec2(0.17, 0.42)));
}

vec4 meadow_small(vec2 p) {
	vec2 q = vec2(0.6 * p.x - 0.8 * p.y, 0.8 * p.x + 0.6 * p.y);
	return 0.5 * (texture(macro_tex, p / 3.7 + vec2(0.29, 0.83))
		+ texture(macro_tex, q / 6.1 + vec2(0.71, 0.36)));
}

// Grass color at `p` in four scales: macro_far_g (hundreds of meters, the
// terrain's macro_far.g), m_mid (tens of meters), m_small (a few meters) and
// speckles of single tufts (`speck`, m_vnoise(p / 0.3), 0.5 where a pixel is
// larger than the speckles). Averaged samples have less contrast, hence the
// narrow ramps. Returns the color; `dry` gets the dryness 0..1.
vec3 meadow_base(vec2 p, float macro_far_g, vec4 m_mid, vec4 m_small, float speck, out float dry) {
	dry = smoothstep(0.5, 0.8, macro_far_g) * 0.7
		+ smoothstep(0.5, 0.68, m_mid.r) * 0.45
		+ smoothstep(0.5, 0.7, m_small.g) * 0.15
		+ (speck - 0.5) * 0.2;
	float damp = smoothstep(0.42, 0.22, macro_far_g) * 0.6
		+ smoothstep(0.46, 0.32, m_mid.b) * 0.6;
	dry = clamp(dry, 0.0, 1.0);
	damp = clamp(damp, 0.0, 1.0) * (1.0 - dry);
	vec3 base = mix(meadow_lush.rgb, meadow_dry.rgb, dry);
	base = mix(base, meadow_damp.rgb, damp);
	return base * mix(0.88, 1.1, m_small.r) * mix(0.9, 1.08, speck) * mix(0.9, 1.08, m_mid.a);
}
