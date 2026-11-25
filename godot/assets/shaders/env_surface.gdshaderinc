// Surfaces per building (prompt 186), included by env.gdshader after
// env_roof.gdshaderinc.
//
// A blueprint may set a `[surface]` table (docs/ASSETS.md, "Surfaces per
// building"); tools/blender/build_env.py stores it as the glTF extra
// `env_surface`, EnvLook.surface() packs it into two instance uniforms and
// sets them on the building's meshes (EnvLook.apply, map_objects.gd per
// kind). All zero / default = the look before prompt 186.
//
//   env_surface_a: x plaster color (0 = per house as before, 1.. =
//                  EnvLook.PLASTERS), y spalled plaster share 0..1, z brick
//                  color (0 = as before, 1.. = EnvLook.BRICKS), w bond
//                  (0 stretcher, 1 English)
//   env_surface_b: x roof covering (0 = as before, 1.. = EnvLook.ROOFS),
//                  y course height in m (0 = the covering's own), z share
//                  of missing tiles 0..0.4, w moss and lichen factor (1)
//
// Fine relief (tile courses, curved tiles, mortar joints, spall edges) is a
// normal perturbation from a height in meters (`surface_bump`,
// relief.gdshaderinc), never geometry; it fades with the pixel footprint
// like the fine color layers.

instance uniform vec4 env_surface_a = vec4(0.0, 0.0, 0.0, 0.0);
instance uniform vec4 env_surface_b = vec4(0.0, 0.0, 0.0, 1.0);

// Plaster colors by index (EnvLook.PLASTERS: lime, cream, ochre, ochre red,
// pink, grey green, white), linear albedo like the Plaster slot's own colors
// (lime, ochre and white are those; the others are judgement values).
vec3 surface_plaster(float index) {
	vec3 c[7] = {vec3(0.80, 0.77, 0.69), vec3(0.80, 0.70, 0.50), vec3(0.76, 0.60, 0.36),
		vec3(0.47, 0.16, 0.08), vec3(0.70, 0.40, 0.33), vec3(0.42, 0.47, 0.37), vec3(0.90, 0.89, 0.85)};
	return c[clamp(int(index + 0.5) - 1, 0, 6)];
}

// Brick colors by index (EnvLook.BRICKS: red, yellow, dark); `fallback` for
// index 0 (the per-house brick of the Stone slot).
vec3 surface_brick_color(float index, vec3 fallback) {
	vec3 c[3] = {vec3(0.40, 0.12, 0.06), vec3(0.62, 0.45, 0.24), vec3(0.20, 0.07, 0.05)};
	return index < 0.5 ? fallback : c[clamp(int(index + 0.5) - 1, 0, 2)];
}

// Brickwork on wall coordinates `w` (m): 25 x 7.5 cm bricks, 1.2 cm joints;
// English bond (`english` 1): every other course of headers (12.5 cm).
// Returns brick albedo, `joint` = 1 in the mortar.
vec3 surface_brickwork(vec2 w, vec3 brick, float english, out float joint) {
	float course = floor(w.y / 0.075);
	vec2 size = vec2(english > 0.5 && mod(course, 2.0) > 0.5 ? 0.125 : 0.25, 0.075);
	// joints and single bricks fade to their mean before they flicker
	float k = roof_fine(max(fwidth(w.x), fwidth(w.y)), 0.075);
	float m = mix(0.84, bond(w, size, 0.012), k);
	joint = 1.0 - m;
	float tone = 1.0 + (0.4 * hash2(floor(vec2(w.x / size.x + 0.5 * mod(course, 2.0), course))) - 0.2) * k;
	return mix(vec3(0.62, 0.60, 0.55), brick * tone, m);
}

// Coordinates on a wall: along it (m) and up (m), from the rest normal.
vec2 surface_wall_uv(vec3 p, vec3 n) {
	vec2 t = normalize(vec2(-n.z, n.x) + vec2(1e-4, 0.0));
	return vec2(dot(p.xz, t), p.y);
}

// Where plaster has fallen off: 1 = bare brick. More near the ground
// (`low`, meters above it) and at edges (`edge`, the baked edge wear);
// `rim` = the broken plaster edge around a hole.
float surface_spall(vec2 w, float amount, float low, float edge, float seed, out float rim) {
	float n = fbm3(vec3(w * 0.8, seed * 13.0)) + 0.2 * vnoise(vec3(w * 3.5, seed * 7.0));
	float bias = (0.3 * (1.0 - smoothstep(0.0, 2.5, low)) + 0.25 * edge) * amount;
	float t = 1.0 - 0.62 * amount - bias;
	float bare = smoothstep(t, t + 0.025, n);
	rim = smoothstep(t - 0.05, t, n) * (1.0 - bare);
	return bare;
}

// Roof coverings of `env_surface_b.x` (EnvLook.ROOFS: 1 clay, 2 mission
// (monk and nun), 3 beaver tail, 4 Spanish / S pan, 5 slate): albedo in
// linear space and the relief height `h` (m). `row` = course height (m,
// 0 = the covering's own), `missing` = share of missing tiles, `moss` =
// growth factor. Clay and slate draw the patina of roof_clay / roof_slate;
// the others build on the same layers.
vec3 surface_roof(RoofFrame f, float seed, float age, float dry, int kind, float row, float missing,
		float moss, out float h) {
	// course height, tile width (m) and relief per covering
	float course = kind == 2 ? 0.36 : (kind == 3 ? 0.15 : (kind == 4 ? 0.33 : (kind == 5 ? 0.2 : 0.17)));
	float width = kind == 2 ? 0.2 : (kind == 3 ? 0.18 : (kind == 4 ? 0.26 : (kind == 5 ? 0.28 : 0.21)));
	if (row > 0.01) {
		width *= row / course;
		course = row;
	}
	float k = roof_fine(f.fw, course);
	float r = f.uv.y / course + 0.12 * sin(f.uv.x * 0.8 + seed * 40.0);
	float c = f.uv.x / width;
	// mission and S pans run in columns (no half bond); flat tiles in a half bond
	bool columns = kind == 2 || kind == 4;
	float cc = columns ? c : c + 0.5 * mod(floor(r), 2.0);
	float u = fract(cc) * 2.0 - 1.0;
	// beaver tails: rounded lower ends shift the course edge per tile
	float rr = kind == 3 ? r - 0.28 * (1.0 - sqrt(max(1.0 - u * u, 0.0))) : r;
	vec2 id = vec2(floor(cc), floor(rr));
	float h1 = hash2(id + vec2(seed * 17.3, 3.0));
	float h2 = hash2(id + vec2(5.0, seed * 29.1));
	vec3 col = kind == 5 ? mix(vec3(0.46, 0.48, 0.52), vec3(0.33, 0.33, 0.35), roof_rand(seed, 4.0))
		: roof_clay_base(seed);
	col *= 1.0 + (h1 - 0.5) * (kind == 5 ? 0.25 : 0.4) * k;
	col *= mix(vec3(1.0), h2 > 0.5 ? vec3(1.08, 1.04, 0.92) : vec3(0.9, 0.84, 0.82), abs(h2 - 0.5) * 1.4 * k);
	col *= 0.9 + 0.2 * vnoise(vec3(f.uv * 1.1, seed * 3.0));
	// relief: each course thickens towards its lower edge (a step at the
	// joint), curved tiles add their profile across
	float along = fract(rr);
	h = along * (kind == 5 ? 0.014 : 0.03);
	float joint = roof_rows(rr, kind == 5 ? 0.12 : 0.16);
	float side = 0.0;
	if (kind == 2) {
		// monk and nun: convex covers (monk) over concave pans (nun)
		float monk = step(0.5, fract(c * 0.5));
		float prof = cos(u * 1.5708);
		h += mix(-0.035 * prof, 0.05 * prof, monk);
		// pans lie in the shade of the covers, covers catch the light on top
		col *= mix(0.62 + 0.18 * prof, 0.95 + 0.2 * prof, monk);
		side = roof_rows(c * 0.5 + 0.5, 0.05) + roof_rows(c * 0.5, 0.05);
	} else if (kind == 4) {
		// S pan: a wave across, one crest and one trough per tile
		float wave = sin(fract(c) * 6.2832);
		h += 0.03 * wave;
		col *= 0.92 + 0.12 * wave;
		side = roof_rows(c + 0.02, 0.04);
	} else {
		side = roof_rows(cc + 0.47, kind == 5 ? 0.04 : 0.06);
	}
	// curved tiles overlap smoothly down the slope: weak course joints,
	// their profile carries the pattern
	col *= 1.0 - (columns ? 0.18 : 0.42) * joint - 0.25 * side;
	// missing tiles: the battens and the dark roof space between them
	float gone = step(hash2(id + vec2(seed * 3.7, 23.0)), missing) * f.top;
	if (gone > 0.5) {
		float batten = 1.0 - smoothstep(0.0, 0.06, abs(fract(rr) - 0.15));
		col = mix(vec3(0.09, 0.07, 0.055), vec3(0.36, 0.27, 0.18), batten * k);
		h = -0.04 + 0.03 * batten;
		joint = 0.0;
	}
	h *= k;
	float a = age * (1.0 - 0.8 * gone);
	col = roof_weather(col, f, seed, a, dry, kind == 5 ? 0.8 : 1.0);
	col = roof_growth(col, f, seed, clamp(a * moss, 0.0, 1.0), dry, joint);
	return roof_linear(roof_desert(col, dry, joint));
}
