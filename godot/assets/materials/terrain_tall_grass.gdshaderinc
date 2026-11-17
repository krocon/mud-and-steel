// Tall grass on the ground (prompt 073), included by terrain.gdshader after
// the meadow include (uses its m_vnoise). Presentation only.
//
// The mask comes from the A channel of the track raster (terrain_layers.rs;
// tall grass counts as grass in the splat weights). Near the camera the
// plants of tall_grass.gd stand here and the ground under them is dark
// green litter, so no light soil shows through gaps. Towards `tall_range`
// the plants shrink into the ground and this canopy look fades in: darker,
// rougher and richer than the meadow, light blade tips as strokes in cells
// fixed to the world, clumps of about a meter, the gusts of the plants,
// and a shadow seam at the edge of the stand, so it reads as taller than
// the meadow at 110 and 350 m. Every texture coordinate depends on the
// world position only (rule of prompt 071); the camera distance and the
// pixel footprint only steer weights.

#include "res://assets/shaders/tall_grass_common.gdshaderinc"

// 0: tall grass drawn as plain meadow (benchmark A/B).
uniform float tall_strength = 1.0;

// Frayed mask 0..1 from the raw mask (track.a), with noise of a few meters
// so the edge does not follow the 2 m grid.
float tall_mask(vec2 p, float raw, vec4 macro_near) {
	float n = m_vnoise(p / 1.7) * 0.6 + macro_near.a * 0.4;
	return smoothstep(0.32, 0.68, raw + (n - 0.5) * 0.45) * tall_strength;
}

// Shadow seam just outside the stand (at a distance, where the plants are
// gone): the taller grass shades the ground beside it.
float tall_seam(float raw) {
	return smoothstep(0.03, 0.22, raw) * (1.0 - smoothstep(0.3, 0.55, raw)) * tall_strength;
}

// Tufts seen from above, as the plants look from the game camera: cells of
// 0.62 m with one tuft each (jittered; the 2 x 2 nearest cells), blades
// radiating from its center as strokes, lighter towards their tips, dark
// gaps between the tufts. Antialiased by the pixel footprint `px`; where a
// pixel is too large for single blades, their coverage is used.
// Returns (brightness of the tuft 0..1, blade coverage 0..1, position
// along the blade 0..1).
vec3 tall_tufts(vec2 p, float px, float size) {
	vec2 g = p / size;
	vec2 id = floor(g);
	vec2 f = g - id;
	vec2 side = vec2(f.x < 0.5 ? -1.0 : 1.0, f.y < 0.5 ? -1.0 : 1.0);
	float best = 9.0;
	vec2 center = vec2(0.0);
	float seed = 0.0;
	for (int k = 0; k < 4; k++) {
		vec2 c = id + vec2(float(k & 1), float(k >> 1)) * side;
		vec2 h = vec2(tg_hash(c), tg_hash(c + 17.3));
		vec2 at = (c + 0.15 + 0.7 * h) * size;
		float d = length(p - at);
		if (d < best) {
			best = d;
			center = at;
			seed = h.x;
		}
	}
	float body = 1.0 - smoothstep(0.1, 0.55, best / size * 0.62);
	if (px > 0.16) {
		return vec3(body, 0.45 * body, 0.5);
	}
	// Blades: 13 sectors around the center, each blade a stroke out to
	// 0.2..0.42 m, about 4.5 cm wide.
	vec2 d = p - center;
	float sectors = 13.0;
	float a = atan(d.y, d.x) / 6.2831853 * sectors + seed * 7.0;
	float sector = floor(a);
	float along = best / mix(0.22, 0.5, tg_hash(vec2(sector, seed * 31.0)));
	float across = abs(fract(a) - 0.5) * 6.2831853 * best / sectors;
	float w = 0.045 * (1.0 - 0.6 * clamp(along, 0.0, 1.0));
	float we = max(w, px * 0.5);
	float cov = clamp((we - across) / px + 0.5, 0.0, 1.0) * (w / we) * step(along, 1.0) * step(0.08, along);
	float fine = 1.0 - smoothstep(0.08, 0.16, px);
	return vec3(body, mix(0.45 * body, cov, fine), clamp(along, 0.0, 1.0));
}

// tall: frayed mask; seam: tall_seam of the raw mask; dist: camera
// distance (m); px: pixel footprint (m).
void tall_apply(vec2 p, float tall, float seam, float dist, float px, float time,
		inout vec3 albedo, inout vec3 detail, inout float rough) {
	// Hand-over: plants up to tall_range (shrinking from 0.75 of it on),
	// canopy look beyond. tall_range 0: no plants, canopy everywhere.
	float far_k = tall_range <= 0.0 ? 1.0 : smoothstep(tall_range * 0.7, tall_range * 0.97, dist);
	if (seam > 0.0 && far_k > 0.0) {
		albedo *= 1.0 - 0.32 * seam * far_k * (1.0 - tall);
	}
	if (tall <= 0.002) {
		return;
	}
	vec4 patch = tall_patch(p);
	vec2 trail = tall_trail(p);
	float gust = tall_gust(p, time);

	// Ground under the plants: the meadow below in the deep shade of the
	// stand (keeps its blades and texture), turned towards dark litter.
	vec3 under = mix(albedo * 0.3, tall_root.rgb * 1.5, 0.45);

	// Canopy: tufts with radiating blades over dark gaps, larger clumps
	// of a few meters lighter or darker.
	vec4 cl = texture(macro_tex, p / 2.9 + vec2(0.41, 0.13));
	float clumps = smoothstep(0.3, 0.72, cl.r * 0.6 + cl.g * 0.4);
	// Two layers of tufts (different cells, turned) overlap, so the stand
	// shows no cells.
	vec3 ta = tall_tufts(p, px, 0.62);
	vec2 q = vec2(0.8 * p.x - 0.6 * p.y, 0.6 * p.x + 0.8 * p.y) + vec2(13.1, 7.7);
	vec3 tb = tall_tufts(q, px, 0.53);
	vec3 t = vec3(max(ta.x, tb.x * 0.9), max(ta.y, tb.y), ta.y >= tb.y ? ta.z : tb.z);
	vec3 tip_col = mix(tall_tip.rgb, tall_dry.rgb, patch.w * 0.5);
	vec3 gap = mix(tall_root.rgb * 1.5, patch.rgb * 0.3, 0.4);
	vec3 body = mix(gap, patch.rgb * 0.55, t.x * (0.6 + 0.4 * clumps));
	vec3 blade = mix(patch.rgb * 0.8, tip_col, smoothstep(0.2, 1.0, t.z)) * mix(0.85, 1.1, clumps);
	// Seen from afar the stand is darker and more saturated than the
	// meadow: most of what shows is the shade between the blades.
	vec3 canopy = mix(body, blade, t.y) * mix(1.0, 0.78, smoothstep(0.05, 0.3, px));
	canopy *= 1.0 + gust * 0.18;

	// Pressed flat by vehicles: paler; burnt: black stubble.
	canopy = mix(canopy, tall_dry.rgb * 0.8, trail.x * 0.6);
	under = mix(under, tall_dry.rgb * 0.55, trail.x * 0.5);
	canopy = mix(canopy, vec3(0.025, 0.022, 0.02), trail.y);
	under = mix(under, vec3(0.02, 0.018, 0.016), trail.y);

	vec3 c = mix(under, canopy, far_k);
	albedo = mix(albedo, c, tall);
	rough = mix(rough, 0.95, tall);
	// The canopy hides the ground's relief detail; a rough surface of its own.
	detail *= 1.0 - 0.7 * tall;
	detail += vec3(cl.g - 0.5, 0.0, cl.b - 0.5) * 0.35 * tall * far_k;
}
