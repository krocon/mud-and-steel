// Snow on roofs (prompt 080), included by env.gdshader after
// env_roof.gdshaderinc. Replaces snow_cover() on roof pixels: the Roof slot
// (clay, slate, thatch, boards), the sloped corrugated sheet of the Metal
// slot and the flat Plaster roofs.
//
// The snow is a blanket or nothing: a signed distance `d` (m, > 0 in the
// snow) decides, its edge is smoothed over the pixel footprint only. The
// blanket stays along the ridge and along the eave and has slid off in
// between, in lanes; the lower edge of the ridge band is jagged with
// tongues hanging down, single floes lie on the free roof. How much stays
// depends on the amount (`env_snow`), the pitch (steep holds less), the
// covering, the sun side (less), the lee side (more) and the house
// (`v_seed`). Everything depends only on the position on the model, its
// normal and the seed: the same house always looks the same, nothing moves
// with the camera.
//
// Thickness is faked: the edge faces of the roof slabs (eave and verge)
// turn white under snow and read as the blanket's front, a dark line runs
// under the edge of the blanket on the free roof, and the normal bends over
// at the edge so the light draws the blanket as a body.

// The horizontal bounds of the mesh in model space (min x, min z, max x,
// max z; m), set per mesh by EnvLook.roof_box(): how far the eave lies from
// the ridge. Zero: unknown, a 4 m half width is assumed.
instance uniform vec4 env_roof_box = vec4(0.0);

// Direction towards the sun, horizontal (x, z): south-east, as the winter
// biome (azimuth 140 deg). The sunny side holds a little less snow.
const vec2 SNOW_SUN = vec2(0.64, 0.77);
// Snow albedo (linear): the same white as snow_cover(), a touch lower so a
// roof in the noon sun does not burn out.
const vec3 SNOW_WHITE = vec3(0.86, 0.88, 0.92);

struct RoofSnow {
	// 0..1 snow coverage (edge smoothed over the pixel footprint)
	float cover;
	// 0..1 dark line under the edge of the blanket, on the free roof
	float shade;
	// -1..1: how far the normal bends over at the edge, + downhill
	float bend;
};

// Horizontal distance from the model origin (the ridge) to the edge of the
// mesh bounds along `dir`.
float snow_eave(vec2 dir) {
	vec4 b = env_roof_box;
	float e = (dir.x > 0.0 ? b.z : -b.x) * abs(dir.x) + (dir.y > 0.0 ? b.w : -b.y) * abs(dir.y);
	return (b.z - b.x) > 0.5 ? max(e, 0.5) : 4.0;
}

// The snow on a sloped or flat roof face, or on an edge face of the roof
// slabs. `n`: model normal, `wn`: world normal, `amount`: env_snow,
// `hold`: how well the covering holds snow (thatch more, sheet less),
// `top` / `eave`: scale of the ridge and eave bands, `soft`: extra edge
// softness (m), `is_flat`: 1 = a flat roof (closed blanket).
RoofSnow roof_snow(vec3 p, vec3 n, vec3 wn, RoofFrame f, float seed, float amount,
		float hold, float top, float eave, float soft, float is_flat) {
	RoofSnow r;
	float sa = length(n.xz);
	bool edge = f.top < 0.5;
	// downhill (horizontal) and the position on the slope: u = 0 at the
	// ridge, 1 at the eave. Edge faces take it across the ridge (houses
	// slope along x, kit/buildings.py gable_roof).
	vec2 down = sa > 0.05 ? n.xz / sa : vec2(1.0, 0.0);
	float horiz = dot(p.xz, down);
	if (edge) {
		down = vec2(p.x >= 0.0 ? 1.0 : -1.0, 0.0);
		horiz = abs(p.x);
	}
	float half_w = snow_eave(down);
	float u = clamp(horiz / half_w, 0.0, 1.0);
	float cosp = edge ? 0.75 : max(n.y, 0.3);
	float len = half_w / cosp;  // slope length ridge to eave (m)
	float s = u * len;
	float x = edge ? p.z : f.uv.x;
	// per house and roof side
	float side = seed * 7.31 + (down.x > 0.0 ? 0.37 : 0.0) + (down.y > 0.0 ? 0.71 : 0.0);
	float k = amount / 0.85;
	// how much stays: covering, pitch, sun, lee, house
	vec2 wh = wn.xz / max(length(wn.xz), 1e-3);
	float tilt = smoothstep(0.02, 0.3, length(wn.xz));
	float sun = smoothstep(0.0, 0.8, dot(wh, SNOW_SUN)) * tilt;
	vec2 wd = normalize(wind_direction + vec2(1e-4));
	float lee = dot(wh, wd) * tilt;
	float pitch = edge ? 1.0 : smoothstep(0.2, 0.72, n.y);
	float c = hold * pitch * k * (1.0 - 0.22 * sun) * (1.0 + 0.15 * lee);
	c *= 0.78 + 0.44 * roof_rand(seed, 40.0 + side);
	// fine layers (tongues, floes) fade before they are smaller than a pixel
	float fine = roof_fine(f.fw, 0.7);
	// ridge band: its lower edge jagged and bayed, tongues hanging down
	float n1 = vnoise(vec3(x * 0.55, side * 13.0, 0.5));
	float n2 = vnoise(vec3(x * 1.6, side * 5.0, 2.5));
	float tongue = pow(n2, 3.0) * fine;
	// small bites and clumps along the edge (20-40 cm)
	float bite = (vnoise(vec3(x * 4.5, s * 1.5, side * 11.0)) - 0.5) * roof_fine(f.fw, 0.3);
	float top_w = (0.06 + 0.34 * c) * top;
	float d_top = (top_w + 0.16 * (n1 - 0.5)) * len + (0.6 + 0.5 * c) * tongue + 0.35 * bite - s;
	// eave band: a broad strip, its upper edge wavy
	float n3 = vnoise(vec3(x * 0.8, side * 9.0, 4.5));
	float eave_w = (0.05 + 0.24 * c) * eave;
	float d_eave = s - (1.0 - eave_w - 0.1 * (n3 - 0.5)) * len + 0.25 * bite;
	// lanes where nothing slid (bands joined from ridge to eave)
	float lane = vnoise(vec3(x * 0.32, s * 0.12, side * 3.0 + 6.0));
	float d_lane = (lane - (1.12 - 0.42 * c)) * 3.0;
	// floes on the free roof, and a filled middle with a lot of snow
	float floe = vnoise(vec3(f.uv.x * 0.9, s * 1.3, side * 17.0 + 1.0));
	float d_floe = (floe - (0.86 - 0.1 * c)) * 1.6 * fine - (1.0 - fine) * 0.2;
	float d_fill = (c - 1.02) * 4.0 + (k - 1.05) * 6.0 + (floe - 0.5) * 1.2;
	float d = max(max(d_top, d_eave), max(max(d_lane, d_floe), d_fill));
	// flat roofs: a closed blanket
	d = mix(d, 0.5 + (k - 0.8) * 1.5 + (floe - 0.5) * 0.6, is_flat);
	// haystacks, bales and other small things: a cap on the flatter top
	vec2 size = env_roof_box.zw - env_roof_box.xy;
	bool small = min(size.x, size.y) < 4.8;
	d = small ? (n.y - 0.78 + 0.3 * c + (floe - 0.5) * 0.25) * 1.5 : d;
	// thin snow: holes in the blanket
	float hole = vnoise(vec3(f.uv * 1.1, side * 23.0 + 3.0));
	d = min(d, (0.3 + 0.75 * smoothstep(0.25, 1.0, k) - hole) * 2.0);
	// no snow at all without any amount; edge faces only on buildings (a
	// hay bale or haystack has no slab edges, its sides stay bare)
	d = k <= 0.0 || (edge && small) ? -1.0 : d;
	float w = f.fw * 1.2 + soft;
	r.cover = smoothstep(-w, w, d);
	r.shade = (1.0 - r.cover) * (1.0 - smoothstep(0.0, 0.1 + w, -d)) * roof_fine(f.fw, 0.2);
	// bend: over the lower edge of the ridge band downhill, over the upper
	// edge of the eave band uphill
	float body = 1.0 - smoothstep(0.0, 0.35, d);
	float which = d_top >= d_eave ? 1.0 : -1.0;
	r.bend = r.cover * body * which * (1.0 - is_flat) * roof_fine(f.fw, 0.3) * (edge ? 0.0 : 1.0);
	return r;
}

// Albedo of the snow (linear): a little grain, bluish on the shady side,
// warm on the sunny side, brighter on the body.
vec3 snow_albedo(vec3 p, vec3 wn, RoofFrame f) {
	float grain = vnoise(p * 7.0) - 0.5;
	vec3 col = SNOW_WHITE * (1.0 + 0.05 * grain * roof_fine(f.fw, 0.15));
	vec2 wh = wn.xz / max(length(wn.xz), 1e-3);
	float tilt = smoothstep(0.02, 0.3, length(wn.xz));
	float sunny = dot(wh, SNOW_SUN) * tilt;
	col *= mix(vec3(1.0), sunny > 0.0 ? vec3(1.02, 1.0, 0.96) : vec3(0.95, 0.98, 1.03), abs(sunny));
	return col;
}
