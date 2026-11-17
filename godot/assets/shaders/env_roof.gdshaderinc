// Roof patina (prompt 075): weathered, stained, patched and overgrown roof
// coverings, included by env.gdshader after env_common.gdshaderinc.
//
// Everything is fixed to the house: patterns depend only on the position on
// the model (`p`, model space, Y up), its normal and the per-house seed
// (`v_seed`), so the same house always looks the same and nothing wanders
// with the camera. Fine layers (tile rows, single tiles, lichen dots, fibres,
// cracks) fade to their mean by the pixel footprint (`RoofFrame.fw`) before
// they can flicker; the coarse layers (age, stains, streaks, the ridge-to-
// eave gradient) carry the far view.
//
// Layer order inside a covering: base color per house -> per tile tone ->
// repairs -> rows/joints -> stains -> streaks -> ridge-to-eave gradient ->
// growth (moss, lichen) -> biome (desert bleach and sand). env.gdshader
// then adds edge wear, dust, snow and soot on top, in that order.
//
// Hook for snow (prompt 080): `roof_frame()` gives the slope coordinates
// (`uv`), the distance from the ridge (`ridge`) and the pixel footprint, so
// a snow layer can follow the same rows (snow lying in the courses, thinner
// near the eave) without recomputing them.
//
// All colors are judgement values for rural Central and Eastern European
// roofs around 1940 (burnt clay, slate, straw and reed, grey boards,
// galvanized sheet); [provisional]. The coverings are composed in sRGB
// (colors and contrasts as they look) and converted to linear albedo at the
// end (`roof_linear`); the old roof colors were linear values that read
// pale salmon on screen.

// The roof plane at a fragment. Stage 1 of prompt 075: derived from what the
// model already has (position and normal); the models carry no ridge or
// eave data.
struct RoofFrame {
	// x: along the ridge (m), y: down the slope (m), both on the plane;
	// flat faces use the model's x / z, vertical faces x and depth.
	vec2 uv;
	// Distance down the slope from the ridge (m). Approximated: houses are
	// built with their ridge above the model origin (kit/buildings.py
	// gable_roof), so the horizontal distance from the origin along the
	// slope direction is the distance from the ridge. 0 on flat faces.
	float ridge;
	// 0..1: the face looks away from the sun (north, -Z), moss grows there.
	float north;
	// 0..1: the face looks west (-X), the weather side.
	float west;
	// Meters per pixel on the roof plane (for fading the fine layers).
	float fw;
	// 1 on the sloped or flat top of the covering, 0 on vertical edges and
	// undersides (they get the base color and tile tone only).
	float top;
};

RoofFrame roof_frame(vec3 p, vec3 n, vec3 wn) {
	RoofFrame f;
	float sa = length(n.xz);
	vec2 down = sa > 0.05 ? n.xz / sa : vec2(0.0, 1.0);
	vec2 across = vec2(-down.y, down.x);
	float horiz = dot(p.xz, down);
	if (n.y > 0.97) {
		// flat (flat roofs, ridge cap)
		f.uv = p.xz;
		f.ridge = 0.0;
		f.top = 1.0;
	} else if (n.y > 0.15) {
		// sloped plane: exact plane coordinates, increasing down the slope
		f.uv = vec2(dot(p.xz, across), n.y * horiz - sa * p.y);
		f.ridge = max(horiz, 0.0) / max(n.y, 0.3);
		f.top = 1.0;
	} else {
		// slab edges, eave faces, undersides: rows by height
		f.uv = vec2(dot(p.xz, across), -p.y);
		f.ridge = max(horiz, 0.0);
		f.top = 0.0;
	}
	vec2 wh = wn.xz / max(length(wn.xz), 1e-3);
	float tilt = smoothstep(0.02, 0.3, length(wn.xz));
	f.north = smoothstep(-0.3, 0.7, -wh.y) * tilt;
	f.west = smoothstep(-0.3, 0.7, -wh.x) * tilt;
	f.fw = max(max(fwidth(f.uv.x), fwidth(f.uv.y)), 1e-4);
	return f;
}

// 1 while a pattern of period `size` (m) covers at least ~5 pixels, 0 below
// ~2 pixels: fine layers fade to their mean before they flicker.
float roof_fine(float fw, float size) {
	return 1.0 - smoothstep(0.2 * size, 0.45 * size, fw);
}

// Box-filtered joint lines: the fraction of the pixel footprint
// [x - w/2, x + w/2] (in periods) that lies in a joint [n, n + jw). Exact
// for any footprint, so the rows stay steady under motion; their contrast
// fades out only once a period is below ~2 pixels (`roof_rows`).
float roof_joint(float x, float w, float jw) {
	w = max(w, 1e-4);
	float a = x - 0.5 * w;
	float b = x + 0.5 * w;
	float fa = floor(a) * jw + min(fract(a), jw);
	float fb = floor(b) * jw + min(fract(b), jw);
	return (fb - fa) / w;
}

// Joint darkness 0..1 of rows along `x` (periods), faded to its mean `jw`
// when a period covers fewer than ~2.5 pixels.
float roof_rows(float x, float jw) {
	float w = fwidth(x);
	return mix(jw, roof_joint(x, w, jw), 1.0 - smoothstep(0.35, 0.65, w));
}

// Independent per-house random numbers from the seed.
float roof_rand(float seed, float k) {
	return hash2(vec2(seed * 91.7 + k * 13.1, k * 7.3 + 1.7));
}

// Age of the roof: few new (0.1-0.2), many weathered (0.4-0.75), some very
// old (0.8-1.0). Scales every other layer.
float roof_age(float seed) {
	float h = roof_rand(seed, 1.0);
	return h < 0.12 ? 0.1 + h : (h < 0.8 ? 0.4 + 0.35 * (h - 0.12) / 0.68 : 0.8 + (h - 0.8));
}

float roof_luma(vec3 c) {
	return dot(c, vec3(0.3, 0.59, 0.11));
}

// sRGB color -> linear albedo.
vec3 roof_linear(vec3 c) {
	return pow(clamp(c, 0.0, 1.0), vec3(2.2));
}

// Coarse layers shared by all coverings (sRGB): large stains (darkened,
// bleached, damp-green), dark streaks running down the slope (denser under
// the ridge), darker and dirtier towards the eave, a washed-out ridge.
// `strength` scales the effect per covering.
vec3 roof_weather(vec3 col, RoofFrame f, float seed, float age, float dry, float strength) {
	float a = age * strength * f.top;
	vec3 s = vec3(seed * 17.0, seed * 5.0, seed * 11.0);
	// stains, 3-6 m
	float st = fbm3(vec3(f.uv * 0.28, s.x));
	col *= 1.0 - 0.25 * a * smoothstep(0.5, 0.7, st);
	float bleach = smoothstep(0.42, 0.28, st) * a;
	col = mix(col, vec3(roof_luma(col)) * 1.1 + 0.03, 0.4 * bleach);
	float damp = smoothstep(0.55, 0.75, vnoise(vec3(f.uv * 0.45, s.y + 3.0))) * a * (1.0 - dry);
	col = mix(col, col * vec3(0.78, 0.86, 0.7), 0.6 * damp);
	// streaks down the slope: broad ones carry the far view, narrow ones fade
	float wide = smoothstep(0.55, 0.85, vnoise(vec3(f.uv.x * 0.9, f.uv.y * 0.05, s.z)));
	float narrow = smoothstep(0.6, 0.85, vnoise(vec3(f.uv.x * 2.6, f.uv.y * 0.12, s.z + 5.0)));
	narrow *= roof_fine(f.fw, 0.4);
	float under_ridge = mix(1.0, 0.6, smoothstep(0.5, 3.5, f.ridge));
	col *= 1.0 - a * (0.25 * wide + 0.22 * narrow) * under_ridge;
	// ridge to eave: washed-out ridge, darker dirtier eave
	float eave = smoothstep(0.8, 4.5, f.ridge);
	col *= mix(1.0, 0.82, eave * a);
	col = mix(col, col * vec3(0.92, 0.9, 0.84), eave * a);
	float wash = (1.0 - smoothstep(0.1, 1.0, f.ridge)) * a * step(0.001, f.ridge);
	col = mix(col, vec3(roof_luma(col)) * 1.15, 0.3 * wash);
	return col;
}

// Moss in the joints and along the eave (more on the north side and on old
// roofs), pale grey-yellow lichen dots (sRGB). `joint` = joint darkness.
vec3 roof_growth(vec3 col, RoofFrame f, float seed, float age, float dry, float joint) {
	float alive = (1.0 - dry) * f.top;
	float amount = alive * smoothstep(0.25, 0.9, age) * (0.3 + 0.7 * f.north);
	float eave = smoothstep(1.5, 5.0, f.ridge);
	float m = fbm3(vec3(f.uv * 1.3, seed * 23.0)) + 0.3 * eave + 0.3 * joint - 0.15 * (1.0 - age);
	float moss = smoothstep(0.6, 0.8, m) * amount;
	col = mix(col, vec3(0.30, 0.36, 0.17), 0.7 * moss);
	// lichen: small dots, faded to a faint mean lightening at distance
	float k = roof_fine(f.fw, 0.07);
	float dots = smoothstep(0.86, 0.92, vnoise(vec3(f.uv * 16.0, seed * 31.0)));
	float lichen = mix(0.025, dots, k) * alive * age;
	col = mix(col, vec3(0.70, 0.70, 0.56), 0.4 * lichen);
	return col;
}

// Desert (sRGB): bleached, sand in the joints instead of growth.
vec3 roof_desert(vec3 col, float dry, float joint) {
	col = mix(col, vec3(roof_luma(col)) * 1.1 + vec3(0.06, 0.04, 0.0), 0.3 * dry);
	return mix(col, vec3(0.80, 0.71, 0.55), 0.8 * joint * dry);
}

// Base clay color per house (sRGB): bright brick red, orange red, red
// brown, dark brown, weathered grey brown; a continuous blend.
vec3 roof_clay_base(float seed) {
	vec3 c[5] = {vec3(0.74, 0.37, 0.24), vec3(0.77, 0.45, 0.27), vec3(0.65, 0.34, 0.24),
		vec3(0.54, 0.33, 0.25), vec3(0.60, 0.47, 0.40)};
	float h = roof_rand(seed, 2.0) * 4.999;
	int i = int(floor(h));
	return mix(c[i], c[min(i + 1, 4)], fract(h) * 0.5);
}

// Clay tiles: courses of 17 cm (6 / m), tiles 21 cm wide in a half bond,
// slightly wavy rows, a few slipped tiles, strong tile-to-tile tone, groups
// of patched tiles, then all shared layers. Returns linear albedo.
vec3 roof_clay(RoofFrame f, float seed, float age, float dry) {
	float k = roof_fine(f.fw, 0.19);
	float wave = 0.22 * sin(f.uv.x * 0.8 + seed * 40.0) + 0.3 * (vnoise(vec3(f.uv.x * 0.6, seed * 13.0, 0.5)) - 0.5);
	float r = f.uv.y * 6.0 + wave;
	float row = floor(r);
	float cx = f.uv.x * 4.7;
	float c = cx + 0.5 * mod(row, 2.0);
	vec2 id = vec2(floor(c), row);
	float h1 = hash2(id + vec2(seed * 17.3, 3.0));
	float h2 = hash2(id + vec2(5.0, seed * 29.1));
	float h3 = hash2(id + vec2(seed * 7.7, 11.0));
	vec3 col = roof_clay_base(seed);
	// tile tone: lighter orange or darker brown, a few nearly burnt tiles
	col *= 1.0 + (h1 - 0.5) * 0.4 * k;
	col *= mix(vec3(1.0), h2 > 0.5 ? vec3(1.08, 1.04, 0.92) : vec3(0.9, 0.84, 0.82), abs(h2 - 0.5) * 2.0 * k);
	col *= h3 < 0.05 ? mix(1.0, 0.7, k) : 1.0;
	// groups of about 1 m: unevenness that stays when single tiles are gone
	col *= 0.9 + 0.2 * vnoise(vec3(f.uv * 1.1, seed * 3.0));
	// repairs: groups of tiles in fresher or foreign tone (none on new roofs)
	vec2 q = mix(vec2(c, r), id + 0.5, k);
	float patch = vnoise(vec3(q.x * 0.22, q.y * 0.3, seed * 37.0));
	float repair = smoothstep(0.0, 0.03, patch - mix(1.1, 0.8, smoothstep(0.3, 0.6, age))) * f.top;
	vec3 fresh = roof_rand(seed, 3.0) < 0.7 ? vec3(0.74, 0.37, 0.21) : vec3(0.54, 0.30, 0.21);
	col = mix(col, fresh * (0.92 + 0.16 * h1 * k), repair);
	// course joints (the lower edge of each course shades the next; a
	// slipped tile shifts it) and the side joints between tiles
	float joint = roof_rows(r - (h3 > 0.975 ? 0.2 * k : 0.0), 0.2);
	float side = roof_rows(c + 0.47, 0.06);
	col *= 1.0 - 0.42 * joint - 0.25 * side;
	col = roof_weather(col, f, seed, age * (1.0 - 0.6 * repair), dry, 1.0);
	col = roof_growth(col, f, seed, age * (1.0 - 0.7 * repair), dry, joint);
	return roof_linear(roof_desert(col, dry, joint));
}

// Slate (and dark cement tiles): plates 28 x 20 cm in grey tones, pale lime
// and lichen spots, dark water runs, single replaced plates.
vec3 roof_slate(RoofFrame f, float seed, float age, float dry) {
	float k = roof_fine(f.fw, 0.24);
	float r = f.uv.y * 5.0 + 0.15 * sin(f.uv.x * 0.7 + seed * 30.0);
	float row = floor(r);
	float c = f.uv.x * 3.6 + 0.5 * mod(row, 2.0);
	vec2 id = vec2(floor(c), row);
	float h1 = hash2(id + vec2(seed * 13.1, 2.0));
	float h2 = hash2(id + vec2(9.0, seed * 3.3));
	vec3 col = mix(vec3(0.46, 0.48, 0.52), vec3(0.33, 0.33, 0.35), roof_rand(seed, 4.0));
	col *= 1.0 + (h1 - 0.5) * 0.25 * k;
	col *= mix(vec3(1.0), h2 > 0.5 ? vec3(0.96, 1.0, 1.06) : vec3(1.04, 1.03, 0.95), abs(h2 - 0.5) * k);
	col *= 0.92 + 0.16 * vnoise(vec3(f.uv * 1.2, seed * 5.0));
	// replaced plates: darker, bluer, clean
	float replaced = step(1.0 - 0.06 * age, hash2(id + vec2(seed * 2.9, 17.0))) * k * f.top;
	col = mix(col, vec3(0.27, 0.30, 0.36), replaced);
	float joint = roof_rows(r, 0.12);
	float side = roof_rows(c + 0.48, 0.04);
	col *= 1.0 - 0.45 * joint - 0.25 * side;
	col = roof_weather(col, f, seed, age * (1.0 - replaced), dry, 0.8);
	// pale lime and lichen patches (slate carries more of them than clay)
	float lime = smoothstep(0.62, 0.82, fbm3(vec3(f.uv * 0.9, seed * 41.0))) * age * f.top;
	col = mix(col, vec3(0.62, 0.62, 0.55), 0.3 * lime * (1.0 - 0.5 * dry));
	col = roof_growth(col, f, seed, age * 0.8, dry, joint);
	return roof_linear(roof_desert(col, dry, joint));
}

// Thatch and reed (and hay): courses of about 35 cm with uneven lower
// edges, fibres down the slope, lighter towards the ridge, dark and damp at
// the eave, moss cushions, frayed dark patches; old thatch turns silver
// grey.
vec3 roof_thatch(RoofFrame f, float seed, float age, float dry) {
	vec3 fresh = vec3(0.72, 0.63, 0.45);
	vec3 aged = vec3(0.56, 0.51, 0.43);
	vec3 old = vec3(0.53, 0.52, 0.49);
	vec3 col = age < 0.5 ? mix(fresh, aged, age * 2.0) : mix(aged, old, (age - 0.5) * 2.0);
	col *= 0.92 + 0.16 * roof_rand(seed, 5.0);
	// courses with frayed lower edges
	float edge = 0.35 * (vnoise(vec3(f.uv.x * 3.0, seed * 3.0, 0.0)) - 0.5);
	float r = f.uv.y / 0.35 + edge;
	float k = roof_fine(f.fw, 0.35);
	float course = floor(r);
	float bundle = floor(f.uv.x / 1.4 + hash2(vec2(course, seed * 7.0)));
	col *= 1.0 + (hash2(vec2(course, bundle) + seed * 3.1) - 0.5) * 0.2 * k;
	col *= 1.0 - 0.18 * roof_rows(r, 0.3);
	// fibres down the slope (2-3 cm), gone early
	float fibre = vnoise(vec3(f.uv.x * 38.0, f.uv.y * 2.5, seed));
	col *= mix(1.0, 0.85 + 0.3 * fibre, roof_fine(f.fw, 0.05));
	col *= 0.88 + 0.24 * vnoise(vec3(f.uv * 0.9, seed * 9.0));
	col = roof_weather(col, f, seed, age, dry, 0.9);
	// frayed dark patches
	float fray = smoothstep(0.66, 0.8, fbm3(vec3(f.uv * 0.7, seed * 19.0))) * age * f.top;
	col *= 1.0 - 0.35 * fray;
	// moss cushions: larger and thicker than on tiles
	float cushion = smoothstep(0.6, 0.72, fbm3(vec3(f.uv * 0.8, seed * 27.0)) + 0.2 * smoothstep(1.0, 4.0, f.ridge));
	col = mix(col, vec3(0.31, 0.37, 0.18), 0.45 * cushion * age * (1.0 - dry) * (0.4 + 0.6 * f.north) * f.top);
	return roof_linear(roof_desert(col, dry, 0.0));
}

// Boards and shingles: boards 20 cm wide running down the slope, dark
// seams, butt joints, silver-grey weathered wood, single darker or newer
// boards, green growth on the weather side.
vec3 roof_planks(RoofFrame f, float seed, float age, float dry) {
	float k = roof_fine(f.fw, 0.2);
	float b = f.uv.x * 5.0;
	float board = floor(b);
	float len = 2.5 + 1.5 * hash2(vec2(board, seed * 4.0));
	float along = (f.uv.y + hash2(vec2(board, seed * 9.0)) * len) / len;
	vec2 id = vec2(board, floor(along));
	vec3 grey = vec3(0.62, 0.60, 0.56);
	vec3 brown = vec3(0.56, 0.44, 0.33);
	vec3 col = mix(brown, grey, smoothstep(0.15, 0.6, age));
	col *= 0.92 + 0.16 * roof_rand(seed, 6.0);
	float h = hash2(id + seed * 5.3);
	col *= 1.0 + (h - 0.5) * 0.3 * k;
	// a few newer, browner boards
	float newer = step(0.93, hash2(id + vec2(3.0, seed))) * k * f.top;
	col = mix(col, vec3(0.52, 0.42, 0.32), newer);
	// grain along the boards
	col *= mix(1.0, 0.88 + 0.24 * vnoise(vec3(f.uv.x * 30.0, f.uv.y * 1.5, seed)), roof_fine(f.fw, 0.06));
	float seam = roof_rows(b + 0.06, 0.12);
	float butt = mix(0.0, 1.0 - smoothstep(0.0, 0.03 / len, min(fract(along), 1.0 - fract(along))), k);
	float joint = clamp(seam + butt, 0.0, 1.0);
	col *= 1.0 - 0.5 * joint;
	col = roof_weather(col, f, seed, age * (1.0 - newer), dry, 0.8);
	// green on the weather side
	float green = smoothstep(0.62, 0.8, fbm3(vec3(f.uv * 0.8, seed * 15.0)) + 0.2 * f.west) * age * (1.0 - dry) * f.top;
	col = mix(col, vec3(0.33, 0.38, 0.22), 0.4 * green * (0.4 + 0.6 * max(f.west, f.north)));
	col = roof_growth(col, f, seed, age * 0.6, dry, joint);
	return roof_linear(roof_desert(col, dry, joint));
}

// Corrugated sheet (Metal slot, rust and galvanized kinds; linear albedo in
// and out): sheets 0.9 m wide and 2.4 m long, matte and bright sheets, rust
// along the seams and in runs below the fastener rows (every 1.2 m down
// the slope or wall). `rough` and `metal` are adjusted per sheet.
vec3 sheet_patina(vec3 col, RoofFrame f, float seed, float age, float dry, inout float rough, inout float metal) {
	vec2 sheet = floor(vec2(f.uv.x / 0.9, f.uv.y / 2.4));
	float k = roof_fine(f.fw, 0.9);
	float h = mix(0.5, hash2(sheet + seed * 7.1), k);
	float bright = smoothstep(0.45, 0.65, h);
	col *= mix(0.75, 1.15, bright) * (0.92 + 0.16 * mix(0.5, hash2(sheet + 3.3), k));
	rough = mix(rough + 0.15, rough - 0.1, bright);
	metal = mix(metal * 0.6, metal, bright);
	float seam = roof_rows(f.uv.x / 0.9 + 0.03, 0.06);
	float below = fract(f.uv.y / 1.2);
	float runs = smoothstep(0.55, 0.85, vnoise(vec3(f.uv.x * 2.5, f.uv.y * 0.3, seed * 3.0)));
	runs *= 1.0 - smoothstep(0.0, 1.0, below);
	runs = mix(0.15, runs, roof_fine(f.fw, 0.4));
	float patches = smoothstep(0.6, 0.8, fbm3(vec3(f.uv * 0.4, seed * 11.0)));
	float rust = clamp((2.0 * seam + runs + 0.6 * patches) * (0.3 + age), 0.0, 1.0) * (1.0 - 0.4 * dry);
	col = mix(col, vec3(0.20, 0.075, 0.03) * (0.8 + 0.4 * h), 0.75 * rust);
	rough = mix(rough, 0.9, rust);
	metal = mix(metal, 0.1, rust);
	return roof_weather(col, f, seed, age, dry, 0.5);
}

// Flat plaster roofs (desert houses, upward faces of the Plaster slot;
// linear albedo in and out): grime rims along the parapet and dust in the
// corners (from the baked occlusion), water stains with darker rims, fine
// cracks.
vec3 flat_roof_patina(vec3 col, RoofFrame f, float seed, float age, float ao, float dust) {
	float rim = 1.0 - smoothstep(0.5, 0.95, ao);
	col *= 1.0 - 0.3 * rim * (0.5 + age);
	col = mix(col, vec3(0.70, 0.60, 0.44), clamp(rim * (0.3 + dust), 0.0, 0.6));
	float w = fbm3(vec3(f.uv * 0.35, seed * 13.0));
	float stain = smoothstep(0.55, 0.6, w) * (1.0 - 0.4 * smoothstep(0.6, 0.72, w));
	col *= 1.0 - 0.18 * stain * age;
	float cr = abs(vnoise(vec3(f.uv * 1.6, seed * 5.0)) * 2.0 - 1.0);
	float crack = (1.0 - smoothstep(0.0, 0.035, cr)) * roof_fine(f.fw, 0.3);
	col *= 1.0 - 0.3 * crack * age;
	return col;
}
