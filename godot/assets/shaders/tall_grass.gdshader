// Tall grass plants (prompt 073): blades, leaves, fronds and seed heads as
// opaque triangles (scenes/world/tall_grass.gd builds the meshes), drawn as
// MultiMesh decoration by map_objects.gd on terrain type `tall_grass`.
//
// Color: strong gradient from almost black roots to sunlit yellow-green
// tips, the base varying in patches (tall_grass_common.gdshaderinc), herbs
// darker green, shrubs ocher, seed heads brown. Gusts run over the stand as
// light bands and bend the blades downwind. Vehicles press the grass flat
// and fire leaves black stubble (track raster). Towards `tall_range` the
// plants shrink into the ground, where the terrain's look takes over.
// Blades are widened so none is drawn thinner than about half a pixel.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;

#include "res://assets/shaders/tall_grass_common.gdshaderinc"

varying vec3 v_base;
varying float v_t;
varying float v_shade;
varying float v_kind;
varying float v_gust;
varying vec2 v_trail;
varying float v_dry;

void vertex() {
	vec3 inst = MODEL_MATRIX[3].xyz;
	vec2 p = inst.xz;
	float scale = length(MODEL_MATRIX[0].xyz);
	vec4 patch = tall_patch(p);
	v_base = patch.rgb;
	v_dry = patch.w;
	v_t = COLOR.r;
	v_shade = COLOR.b;
	v_kind = COLOR.a;
	v_trail = tall_trail(p);
	// Distance to the game camera, also in shadow passes (whose camera is
	// the light's).
	float dist = length(inst - MAIN_CAM_INV_VIEW_MATRIX[3].xyz);
	bool shadow_pass = PROJECTION_MATRIX[3][3] > 0.5;

	// Fewer blades with distance (UV2.y: the blade's rank 0..1): beyond
	// `tall_near` they thin out to `tall_far_share` at `tall_switch` (the
	// far mesh holds just those), the rest are widened. Dropped blades
	// collapse to a point (no pixels).
	float keep = mix(1.0, tall_far_share, smoothstep(tall_near, tall_switch, dist));

	// Height: biome, distance (shrink into the ground before the terrain
	// takes over), tracks (pressed flat) and fire (stubble).
	float shrink = 1.0 - smoothstep(tall_range * 0.75, tall_range, dist);
	float flat_k = v_trail.x;
	float h = tall_height * shrink * mix(1.0, 0.18, flat_k) * mix(1.0, 0.1, v_trail.y);
	// Winter: broken blades hang over and lie lower.
	h *= 1.0 - tall_bent * 0.35 * COLOR.r;
	vec2 out_dir = VERTEX.xz;
	VERTEX.y *= h;
	// Flattened blades lie outwards; bent ones too, a little.
	VERTEX.xz += out_dir * (flat_k * 1.4 + tall_bent * 0.4 * COLOR.r);

	// Never thinner than about half a pixel: widen across the blade; the
	// remaining blades of a thinned tuft are wider.
	if (!shadow_pass) {
		float px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
		float half_w = UV2.x * scale * inversesqrt(keep);
		float extra = (max(px * 0.55, half_w) - UV2.x * scale) / scale;
		VERTEX.xz += UV * extra;
	}

	// Wind: gust lean, sway of the tuft and a flutter per blade, more at the tip.
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	v_gust = tall_gust(p, TIME);
	float t = TIME * wind_speed + dot(p, vec2(0.13, 0.07));
	float sway = sin(t) * 0.5 + sin(t * 2.3 + 1.1) * 0.2;
	float flutter = sin(TIME * 5.0 + COLOR.b * 6.2831 + p.x * 3.0) * 0.12;
	float w = COLOR.g * COLOR.g * (1.0 - flat_k);
	vec3 off = vec3(dir.x, 0.0, dir.y) * (v_gust * 1.3 + sway + flutter) * w * wind_strength * 0.55;
	// Bent towards the wind, the blade also sinks a little.
	off.y = -abs(v_gust) * w * wind_strength * 0.25;
	// World to the instance frame: the instance only turns about Y and
	// scales uniformly, so the inverse is the transpose over the scale².
	VERTEX += transpose(mat3(MODEL_MATRIX)) * off / (scale * scale);
	if (UV2.y > keep) {
		VERTEX = vec3(0.0);
	}
}

void fragment() {
	float t = v_t;
	vec3 c;
	if (v_kind < 0.17) {
		// Grass: black roots, the patch color in the middle, sunlit tips.
		vec3 tip = mix(tall_tip.rgb, tall_dry.rgb, v_dry * 0.5) * mix(0.85, 1.1, v_shade);
		c = mix(tall_root.rgb, v_base, smoothstep(0.05, 0.6, t));
		c = mix(c, tip, smoothstep(0.5, 1.0, t));
	} else if (v_kind < 0.5) {
		// Herbs and fronds: broad, darker green leaves.
		c = mix(tall_root.rgb * 2.0, tall_herb.rgb * mix(0.8, 1.2, v_shade), smoothstep(0.0, 0.6, t));
		c = mix(c, tall_herb.rgb * 1.6, smoothstep(0.7, 1.0, t) * 0.5);
	} else if (v_kind < 0.84) {
		// Shrub leaves: olive brown, some yellow-green at the top.
		c = tall_shrub.rgb * mix(0.75, 1.2, v_shade) * mix(0.7, 1.05, t);
		c = mix(c, tall_tip.rgb * 0.9, step(0.6, v_shade) * smoothstep(0.55, 0.9, t));
	} else {
		// Stalks and seed heads.
		c = tall_seed.rgb * mix(0.7, 1.2, v_shade) * mix(0.5, 1.05, t);
	}
	// Gusts: bent blades catch the light.
	c *= 1.0 + v_gust * 0.28 * t;
	// Pressed flat: paler, yellower; burnt: black stubble.
	c = mix(c, tall_dry.rgb * 0.8, v_trail.x * 0.55);
	c = mix(c, vec3(0.025, 0.022, 0.02), v_trail.y);
	// Snow on the upper parts (winter).
	c = mix(c, vec3(0.86, 0.88, 0.92), tall_snow * smoothstep(0.55, 0.95, t) * step(0.45, v_shade) * 0.7);
	ALBEDO = c;
	ROUGHNESS = 0.82;
	SPECULAR = 0.3;
	// Deep in the stand it is dark.
	AO = mix(0.15, 1.0, smoothstep(0.0, 0.75, t));
	AO_LIGHT_AFFECT = 0.5;
	BACKLIGHT = c * 0.3;
}
