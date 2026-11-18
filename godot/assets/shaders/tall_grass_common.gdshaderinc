// Tall grass (prompt 073), shared by the tufts (tall_grass.gdshader) and the
// terrain's look of tall grass (terrain_tall_grass.gdshaderinc), so both
// agree on palette, gusts, vehicle tracks and the distance at which the
// terrain takes over. Needs `macro_tex` (terrain_macro.png) declared before
// the include. Palette and range are set by scenes/world/tall_grass.gd
// (`apply_look`, `apply_range`), the wind by atmosphere.gd / terrain.gd, the
// track raster by scenes/world/grass_trails.gd.

uniform vec4 tall_tip : source_color = vec4(0.62, 0.66, 0.2, 1.0);
uniform vec4 tall_mid : source_color = vec4(0.19, 0.33, 0.08, 1.0);
uniform vec4 tall_root : source_color = vec4(0.02, 0.04, 0.012, 1.0);
uniform vec4 tall_dry : source_color = vec4(0.58, 0.52, 0.2, 1.0);
uniform vec4 tall_herb : source_color = vec4(0.15, 0.3, 0.1, 1.0);
uniform vec4 tall_shrub : source_color = vec4(0.5, 0.35, 0.1, 1.0);
uniform vec4 tall_seed : source_color = vec4(0.42, 0.3, 0.16, 1.0);
// Snow on the tips (winter), bent and broken blades (winter), height factor.
uniform float tall_snow = 0.0;
uniform float tall_bent = 0.0;
uniform float tall_height = 1.0;
// Tufts are drawn up to this distance from the camera (m); they shrink into
// the ground over the last quarter, while the terrain's look fades in.
uniform float tall_range = 65.0;
// Beyond `tall_near` (m) tufts thin out to `tall_far_share` of their
// blades (by rank), reached at `tall_switch`, where the far meshes begin.
uniform float tall_near = 18.0;
uniform float tall_switch = 36.0;
uniform float tall_far_share = 0.3;

uniform float wind_strength = 0.25;
uniform vec2 wind_direction = vec2(1.0, 0.3);
uniform float wind_speed = 1.2;
#include "res://assets/shaders/wind_common.gdshaderinc"

// Tracks (grass_trails.gd): R pressed flat by vehicles, G burnt. World
// rectangle: origin (x, z) and size in meters.
uniform sampler2D trail_tex : hint_default_black, filter_linear, repeat_disable;
uniform vec2 trail_origin = vec2(0.0);
uniform vec2 trail_size = vec2(1.0);

float tg_hash(vec2 p) {
	vec3 q = fract(vec3(p.xyx) * 0.1031);
	q += dot(q, q.yzx + 33.33);
	return fract((q.x + q.y) * q.z);
}

// Gusts: the shared wave (wind_common.gdshaderinc, prompt 086), so trees
// and grass follow the same gust.
float tall_gust(vec2 p, float t) {
	return wind_gust(p, t);
}

// Track raster at a world point: x = flattened, y = burnt (0..1).
vec2 tall_trail(vec2 p) {
	vec2 uv = (p - trail_origin) / trail_size;
	return textureLod(trail_tex, uv, 0.0).rg;
}

// Patch color of the stand at `p`: fresh green, yellowish and dark patches
// of a few to tens of meters (macro noise), as the base of a tuft.
// Returns (mid color, dryness 0..1 in w).
vec4 tall_patch(vec2 p) {
	vec4 a = texture(macro_tex, p / 23.0 + vec2(0.17, 0.61));
	vec4 b = texture(macro_tex, p / 6.3 + vec2(0.53, 0.29));
	float dry = smoothstep(0.52, 0.72, a.r * 0.7 + b.g * 0.3);
	float dark = smoothstep(0.5, 0.3, a.b * 0.65 + b.r * 0.35);
	vec3 c = mix(tall_mid.rgb, tall_dry.rgb * 0.75, dry * 0.7);
	c *= mix(1.0, 0.62, dark) * mix(0.88, 1.12, b.a);
	return vec4(c, dry);
}
