// Terrain surface (prompt 011). Drawn by scenes/terrain/terrain.gd as chunk
// grids in MultiMeshes, one per LOD level.
//
// Vertex: every vertex sits exactly on a map sample. Its height comes from
// `height_tex` (one texel per sample, meters, read with texelFetch, so the
// surface matches the simulation's samples). Edges next to a coarser chunk
// are snapped onto the coarser edge (neighbor strides in INSTANCE_CUSTOM),
// so there are no cracks between LOD levels.
//
// Fragment: normal from `normal_tex` (per pixel, independent of the mesh
// LOD), terrain weights from the splat textures (soft 3 x 3 filtered ids)
// plus optical variation (slope -> earth/rock, macro noise), height-based
// blending between layers, two de-correlated samples per layer against
// visible tiling, field-track ruts turned along the track direction.
//
// Biome (prompt 015): every material layer draws the texture layer named in
// `layer_source`, multiplied by `layer_tint` (desert: grass drawn as dry
// sand-earth; winter: grass drawn as snow). `snow_cover` lays snow over flat
// ground, less on roads and tracks. Set by terrain.gd `apply_biome`.
//
// Water (prompt 014): `water_tex` holds the water level per sample (on water
// and on land near it, -10000 elsewhere). Ground just above the waterline is
// darker and glossier (wet band); the river bed under water is only a little
// darker, the water surface (water.gdshader) tints it.
//
// Weather (prompt 040): `wetness` (0..1, set by world/weather_look.gd)
// darkens the ground, makes it glossy and fills puddles on flat soft
// ground (tracks, mud, bare earth first).
//
// Tall grass (prompt 073): its mask is the A channel of `track_tex` (it
// counts as grass in the splat weights). Near the camera the plants of
// world/tall_grass.gd stand on a dark ground; beyond their range the
// shader draws the stand itself (terrain_tall_grass.gdshaderinc).
//
// Vehicle tracks (prompt 104): `ground_track_tex` (world/ground_tracks.gd)
// holds ruts (R), churned ground (G) and worn ground (B) in a window around
// the camera. Drawn on soft ground only (none on roads, rock and water),
// darker on dark ground (mud) than on bright (sand, snow), browner on
// meadow, with a little relief at the rut edges; puddles gather in them in
// the rain, and snow is pressed grey.
//
// Mountains (prompt 060): on maps with a lot of relief (`mountain`, set by
// terrain.gd from the height range) scree covers slopes between bare earth
// and rock, and snow lies above `snow_line` on all but the steepest rock.
//
// Rule (prompt 071): every texture coordinate of the ground depends on the
// world position only. Values that depend on the camera (distance, `far`,
// pixel footprint) only steer weights: cross-fades, fading in and out. A
// texture scale that followed the camera distance slid the texture over the
// ground and sheared it into 1 px streaks whenever the camera moved
// (`just ground-motion` measures it).
//
// Layer order: see LAYERS in crates/cli/src/gen_textures.rs.
shader_type spatial;
render_mode cull_back;

// Clouds dim the direct sunlight (prompt 087): light() of the include.
#include "res://assets/shaders/cloud_shadow.gdshaderinc"

const int LAYER_COUNT = 10;
const int TRACK = 1;
const int ROAD = 2;
const int SNOW = 5;
const int SHALLOW = 6;
const int DEEP = 7;
const int ROCK = 8;
const int EARTH = 9;
// Size of the second (rotated) sample of every layer against tiling, as a
// factor on the layer's tile: near, and far away (from 250 m on); cross-faded
// between 40 and 250 m.
const float B_SCALE_NEAR = 1.37;
const float B_SCALE_FAR = 1.37 * 4.0;

// Map data (set by terrain.gd).
uniform sampler2D height_tex : filter_nearest, repeat_disable;
uniform sampler2D normal_tex : filter_linear, repeat_disable;
uniform sampler2D splat0_tex : filter_linear, repeat_disable;
uniform sampler2D splat1_tex : filter_linear, repeat_disable;
uniform sampler2D track_tex : filter_linear, repeat_disable;
uniform ivec2 samples = ivec2(2, 2);
uniform vec2 half_size = vec2(1.0);
uniform float cell = 2.0;

// Materials.
uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray normal_array : filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;
uniform float layer_tile[LAYER_COUNT];
uniform float detail_normal_strength = 1.0;
uniform float blend_contrast = 0.35;
// Slope (rise/run) where bare earth and rock start to show.
uniform vec2 earth_grade = vec2(0.20, 0.34);
uniform vec2 rock_grade = vec2(0.45, 0.65);
// Biome look (prompt 015).
// Without a biome (use_biome false) every layer draws its own texture.
uniform bool use_biome = false;
uniform int layer_source[LAYER_COUNT];
uniform vec4 layer_tint[LAYER_COUNT] : source_color;
uniform float snow_cover = 0.0;
uniform float grass_variation = 1.0;
// Rain-soaked ground (prompt 040), 0..1.
uniform float wetness = 0.0;
// Mountain look (prompt 060), 0 = off.
uniform float mountain = 0.0;
// Slope (rise/run) where scree starts and covers the ground.
uniform vec2 scree_grade = vec2(0.42, 0.62);
// Height (m) above which snow lies (mountains), and its fade.
uniform float snow_line = 1000000.0;
uniform float snow_line_fade = 25.0;
// 1: tint every chunk by its LOD level (debug menu).
uniform bool show_lod = false;
uniform sampler2D ground_track_tex : hint_default_black, filter_linear, repeat_disable;
uniform vec2 ground_track_origin = vec2(0.0);
uniform vec2 ground_track_size = vec2(1.0);
uniform bool has_ground_tracks = false;
// Brightness factor of a full rut on dark (x) and bright ground (y).
uniform vec2 track_dark = vec2(0.5, 0.8);
// Strength of the rut edges in the normal.
uniform float track_relief = 1.2;
uniform sampler2D water_tex : filter_linear, repeat_disable;
uniform bool has_water = false;

varying vec3 world_pos;
varying float lod_stride;

// Meadow look of the grass layer (prompt 057).
#include "res://assets/materials/terrain_meadow.gdshaderinc"
// Tall grass (prompt 073): dark ground under the plants, canopy far away.
#include "res://assets/materials/terrain_tall_grass.gdshaderinc"
#include "res://assets/materials/terrain_fields.gdshaderinc"

float height_at(ivec2 s) {
	return texelFetch(height_tex, clamp(s, ivec2(0), samples - 1), 0).r;
}

// Height along a chunk edge that runs from sample `a` in direction `dir`,
// interpolated on the grid of the coarser neighbor with `coarse` samples.
float snapped(ivec2 origin, ivec2 dir, int t, int coarse) {
	int t0 = (t / coarse) * coarse;
	float f = float(t - t0) / float(coarse);
	return mix(height_at(origin + dir * t0), height_at(origin + dir * (t0 + coarse)), f);
}

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	int stride = int(UV2.x + 0.5);
	int quads = int(UV2.y + 0.5);
	ivec2 g = ivec2(UV + 0.5);
	ivec2 s0 = ivec2(round((origin.xz + half_size) / cell));
	ivec2 s = min(s0 + g * stride, samples - 1);
	float h = height_at(s);
	// Neighbor strides: x = north (-z), y = east (+x), z = south (+z), w = west (-x).
	ivec4 nb = ivec4(INSTANCE_CUSTOM + 0.5);
	int t_x = g.x * stride;
	int t_z = g.y * stride;
	if (g.y == 0 && nb.x > stride) {
		h = snapped(s0, ivec2(1, 0), t_x, nb.x);
	} else if (g.y == quads && nb.z > stride) {
		h = snapped(s0 + ivec2(0, quads * stride), ivec2(1, 0), t_x, nb.z);
	} else if (g.x == 0 && nb.w > stride) {
		h = snapped(s0, ivec2(0, 1), t_z, nb.w);
	} else if (g.x == quads && nb.y > stride) {
		h = snapped(s0 + ivec2(quads * stride, 0), ivec2(0, 1), t_z, nb.y);
	}
	// Vertices beyond the map edge collapse onto it.
	VERTEX = vec3(vec2(s) * cell - half_size - origin.xz, 0.0).xzy;
	VERTEX.y = h;
	vec2 n = texelFetch(normal_tex, s, 0).rg * 2.0 - 1.0;
	NORMAL = vec3(n.x, sqrt(max(1.0 - dot(n, n), 0.0)), n.y);
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	lod_stride = float(stride);
}

vec2 rotate(vec2 p, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

void fragment() {
	CLOUD_FRAGMENT(0.35)
	vec2 p = world_pos.xz;
	vec2 texel = vec2(samples);
	vec4 macro_far = texture(macro_tex, p / 420.0);
	vec4 macro_near = texture(macro_tex, p / 47.0 + 0.37);
	// Data lookup, warped a little so borders do not follow the 2 m grid.
	vec2 warp = (vec2(macro_near.a, macro_far.g) - 0.5) * 1.6;
	vec2 data_uv = ((p + half_size) / cell + 0.5 + warp) / texel;
	vec2 base_uv = ((p + half_size) / cell + 0.5) / texel;

	vec2 nxz = texture(normal_tex, base_uv).rg * 2.0 - 1.0;
	vec3 base_n = normalize(vec3(nxz.x, sqrt(max(1.0 - dot(nxz, nxz), 0.0)), nxz.y));
	float grade = length(nxz) / max(base_n.y, 0.05);

	float w[LAYER_COUNT];
	vec4 s0 = texture(splat0_tex, data_uv);
	vec4 s1 = texture(splat1_tex, data_uv);
	w[0] = s0.r; w[1] = s0.g; w[2] = s0.b; w[3] = s0.a;
	w[4] = s1.r; w[5] = s1.g; w[6] = s1.b; w[7] = s1.a;
	// Optical layers: bare earth on slopes and in a few patches, rock on
	// the steepest slopes. They replace soft ground (grass, sand, snow).
	float soft = w[0] + w[3] + w[5];
	// Worn patches: where two noises of different size are both high.
	float patch = smoothstep(0.6, 0.72, macro_far.b) * smoothstep(0.45, 0.65, macro_near.r);
	// Mountains: grass holds on steeper slopes (alpine meadows), rock only
	// on the walls.
	vec2 earth_g = earth_grade + mountain * vec2(0.22, 0.26);
	vec2 rock_g = rock_grade + mountain * vec2(0.17, 0.2);
	float earth = max(smoothstep(earth_g.x, earth_g.y, grade), patch * 0.8);
	float rock = smoothstep(rock_g.x, rock_g.y, grade + (macro_near.r - 0.5) * 0.1);
	w[ROCK] = soft * rock;
	w[EARTH] = soft * earth * (1.0 - rock);
	// Scree (mountains): patchy, between earth and rock.
	float scree = mountain * soft * (1.0 - rock)
		* smoothstep(scree_grade.x, scree_grade.y, grade + (macro_near.g - 0.5) * 0.2);
	float keep = (1.0 - earth) * (1.0 - rock);
	w[0] *= keep; w[3] *= keep; w[5] *= keep;

	// Track direction: doubled angle -> angle along the track.
	vec4 track = texture(track_tex, base_uv);
	vec2 d2 = track.rg * 2.0 - 1.0;
	float track_angle = 0.5 * atan(d2.y, d2.x);
	vec2 along = vec2(cos(track_angle), sin(track_angle));
	vec2 across = vec2(-along.y, along.x);

	// Sample the layers that matter here.
	float blend = smoothstep(0.38, 0.62, macro_near.b);
	float far = smoothstep(40.0, 250.0, length(world_pos - CAMERA_POSITION_WORLD));
	vec2 dpdx = dFdx(p);
	vec2 dpdy = dFdy(p);
	vec3 albedo = vec3(0.0);
	vec3 detail = vec3(0.0);
	float rough = 0.0;
	float ao = 0.0;
	float total = 0.0;
	float best = 0.0;
	float bias[LAYER_COUNT];
	vec4 col[LAYER_COUNT];
	vec4 nrm[LAYER_COUNT];
	for (int i = 0; i < LAYER_COUNT; i++) {
		bias[i] = -1.0;
		if (w[i] < 0.02) {
			continue;
		}
		float tile = layer_tile[i];
		vec2 q = p;
		vec2 gx = dpdx;
		vec2 gy = dpdy;
		if (i == TRACK) {
			q = vec2(dot(p, across), dot(p, along));
			gx = vec2(dot(dpdx, across), dot(dpdx, along));
			gy = vec2(dot(dpdy, across), dot(dpdy, along));
		}
		// Two de-correlated samples: rotated, scaled and (far away) much
		// larger, blended by noise, so the repeat is hard to see. The b
		// sample has two fixed sizes, cross-faded by `far` (see the rule at
		// the top: a size that followed the camera slid the texture).
		float angle_b = i == TRACK ? 0.0 : 1.1 + float(i);
		vec2 uv_a = q / tile;
		vec2 gax = gx / tile;
		vec2 gay = gy / tile;
		float src = use_biome ? float(layer_source[i]) : float(i);
		vec4 ca = textureGrad(albedo_array, vec3(uv_a, src), gax, gay);
		vec4 na = textureGrad(normal_array, vec3(uv_a, src), gax, gay);
		vec2 qb = rotate(q, angle_b) / tile;
		vec2 gbx = rotate(gx, angle_b) / tile;
		vec2 gby = rotate(gy, angle_b) / tile;
		vec2 offset_b = vec2(0.31, 0.71) * float(i + 1);
		vec4 cb = vec4(0.0);
		vec4 nb = vec4(0.0);
		if (far < 1.0) {
			float s = 1.0 / B_SCALE_NEAR;
			cb += (1.0 - far) * textureGrad(albedo_array, vec3(qb * s + offset_b, src), gbx * s, gby * s);
			nb += (1.0 - far) * textureGrad(normal_array, vec3(qb * s + offset_b, src), gbx * s, gby * s);
		}
		if (far > 0.0) {
			float s = 1.0 / B_SCALE_FAR;
			cb += far * textureGrad(albedo_array, vec3(qb * s + offset_b, src), gbx * s, gby * s);
			nb += far * textureGrad(normal_array, vec3(qb * s + offset_b, src), gbx * s, gby * s);
		}
		// The b normal is rotated with its uv; turn it back.
		vec2 nbxy = rotate(nb.xy * 2.0 - 1.0, -angle_b) * 0.5 + 0.5;
		nb = vec4(nbxy, nb.zw);
		col[i] = mix(ca, cb, blend);
		if (use_biome) {
			col[i].rgb *= layer_tint[i].rgb;
		}
		nrm[i] = mix(na, nb, blend);
		bias[i] = w[i] + col[i].a * blend_contrast;
		best = max(best, bias[i]);
	}
	for (int i = 0; i < LAYER_COUNT; i++) {
		if (bias[i] < 0.0) {
			continue;
		}
		float f = max(bias[i] - (best - 0.2), 0.0);
		if (f <= 0.0) {
			continue;
		}
		vec2 dn = (nrm[i].xy * 2.0 - 1.0) * detail_normal_strength;
		vec2 dworld = dn;
		if (i == TRACK) {
			dworld = across * dn.x + along * dn.y;
		}
		vec3 c = col[i].rgb;
		float r = nrm[i].b;
		if (i == SHALLOW || i == DEEP) {
			// River bed: wet and silty under the water surface.
			float wet = i == DEEP ? 0.3 : 0.15;
			c *= mix(vec3(1.0), vec3(0.35, 0.42, 0.45), wet);
			r *= 1.0 - wet * 0.8;
			dworld *= 1.0 - wet;
		}
		albedo += c * f;
		detail += vec3(dworld.x, 0.0, dworld.y) * f;
		rough += r * f;
		ao += nrm[i].a * f;
		total += f;
	}
	total = max(total, 1e-4);
	albedo /= total;
	detail /= total;
	rough /= total;
	ao /= total;

	// Large-scale variation: brightness and a little hue (dry/lush).
	albedo *= mix(0.8, 1.15, macro_far.r) * mix(0.92, 1.06, macro_near.g);
	// Dry and lush areas: shift towards straw or towards deep green, mostly
	// on grass (green-dominant albedo).
	float greenness = clamp((albedo.g - max(albedo.r, albedo.b)) * 25.0, 0.0, 1.0);
	vec3 dry = albedo * vec3(1.35, 1.08, 0.7);
	vec3 lush = albedo * vec3(0.8, 0.95, 0.75);
	albedo = mix(albedo, dry, smoothstep(0.5, 0.8, macro_far.g) * 0.7 * greenness * grass_variation);
	albedo = mix(albedo, lush, smoothstep(0.45, 0.2, macro_far.g) * 0.6 * greenness * grass_variation);

	// Fields (prompt 088): masks (they count as grass in the weights).
	vec4 fields = texture(field_tex, data_uv);
	float field_mask = clamp(fields.r + fields.g + fields.b, 0.0, 1.0);

	// Meadow (prompt 057): palette in four scales, blades, soil, flowers.
	{
		float src0 = use_biome ? float(layer_source[0]) : 0.0;
		vec3 tint0 = use_biome ? layer_tint[0].rgb : vec3(1.0);
		vec3 grass_mean = textureLod(albedo_array, vec3(0.5, 0.5, src0), 12.0).rgb * tint0;
		vec3 grass_tex = bias[0] >= 0.0 ? col[0].rgb : grass_mean;
		float px = max(length(dpdx), length(dpdy));
		meadow_apply(p, smoothstep(0.2, 0.6, w[0]) * (1.0 - field_mask), grass_tex, grass_mean, w[TRACK], along,
			macro_far, macro_near, px, albedo, detail, rough);
	}

	// Tall grass (prompt 073).
	float tall_raw = texture(track_tex, data_uv).a;
	float tall = 0.0;
	if (tall_raw > 0.01) {
		float px = max(length(dpdx), length(dpdy));
		tall = tall_mask(p, tall_raw, macro_near);
		tall_apply(p, tall, tall_seam(tall_raw), length(world_pos - CAMERA_POSITION_WORLD), px, TIME,
			albedo, detail, rough);
	}

	// Fields (prompt 088): rows, tramlines and furrows along each field.
	float field_rows = 0.0;
	if (field_mask > 0.01) {
		float px = max(length(dpdx), length(dpdy));
		float fine = texture(macro_tex, p / 7.0 + 0.53).b;
		field_rows = field_apply(p, fields.rgb, texture(field_tex, base_uv).a, macro_far, macro_near, fine, px,
			albedo, detail, rough);
	}

	// Rock walls (mountains): strata, bands of lighter and darker rock.
	if (mountain > 0.0 && w[ROCK] > 0.02) {
		float band = sin(world_pos.y * 0.9 + macro_near.r * 4.0 + macro_far.g * 9.0);
		float band2 = sin(world_pos.y * 0.23 + macro_far.b * 6.0);
		albedo *= mix(1.0, 0.86 + 0.1 * band + 0.12 * band2, w[ROCK] * mountain);
	}

	// Scree (mountains): grey stones from the pebble texture, larger.
	if (scree > 0.01) {
		float src_scree = use_biome ? float(layer_source[SHALLOW]) : float(SHALLOW);
		float tile = layer_tile[SHALLOW] * 2.2;
		vec4 sc = textureGrad(albedo_array, vec3(p / tile, src_scree), dpdx / tile, dpdy / tile);
		vec4 sn = textureGrad(normal_array, vec3(p / tile, src_scree), dpdx / tile, dpdy / tile);
		float grey = dot(sc.rgb, vec3(0.333));
		vec3 stones = mix(vec3(grey), sc.rgb, 0.35) * vec3(1.02, 1.0, 0.96) * 1.05;
		albedo = mix(albedo, stones, scree * 0.9);
		rough = mix(rough, max(sn.b, 0.75), scree);
		vec2 sdn = (sn.xy * 2.0 - 1.0) * detail_normal_strength;
		detail = mix(detail, vec3(sdn.x, 0.0, sdn.y), scree * 0.8);
	}

	// Vehicle tracks (prompt 104).
	float rut = 0.0;
	float churned = 0.0;
	float worn = 0.0;
	vec3 track_detail = vec3(0.0);
	if (has_ground_tracks) {
		vec2 tuv = (p - ground_track_origin) / ground_track_size;
		vec2 edge = min(tuv, 1.0 - tuv);
		float inside = smoothstep(0.0, 0.08, min(edge.x, edge.y));
		if (inside > 0.0) {
			float soft_track = clamp(1.0 - w[ROAD] - w[ROCK] - w[SHALLOW] - w[DEEP], 0.0, 1.0) * inside;
			vec3 t = texture(ground_track_tex, tuv).rgb;
			// Ruts fade out where a texel shrinks below a few pixels (no
			// shimmer from far away); churned and worn ground are larger.
			float texel_size = 1.0 / float(textureSize(ground_track_tex, 0).x);
			float texel_m = ground_track_size.x * texel_size;
			float px = max(length(dpdx), length(dpdy));
			float sharp = smoothstep(texel_m * 4.0, texel_m * 1.5, px);
			rut = clamp(t.r, 0.0, 1.0) * soft_track * sharp;
			churned = clamp(t.g, 0.0, 1.0) * soft_track;
			worn = clamp(t.b, 0.0, 1.0) * soft_track;
			if (rut + churned + worn > 0.003) {
				float lum = dot(albedo, vec3(0.3, 0.55, 0.15));
				float k = mix(track_dark.x, track_dark.y, smoothstep(0.15, 0.6, lum));
				// Pressed meadow turns brown.
				vec3 crushed = albedo * mix(vec3(1.0), vec3(1.05, 0.82, 0.62), greenness);
				albedo = mix(albedo, crushed * mix(1.0, k, 0.55), worn * 0.7);
				albedo = mix(albedo, crushed * k, rut);
				albedo = mix(albedo, crushed * k * 0.85 * mix(0.8, 1.15, macro_near.r), churned);
				float rx = texture(ground_track_tex, tuv + vec2(texel_size, 0.0)).r - t.r;
				float rz = texture(ground_track_tex, tuv + vec2(0.0, texel_size)).r - t.r;
				track_detail = vec3(rx, 0.0, rz) * track_relief * soft_track * sharp;
				detail *= 1.0 - 0.5 * clamp(churned + rut * 0.5, 0.0, 1.0);
			}
		}
	}

	// Wet band above the waterline (and wet ground under the water).
	if (has_water) {
		float level = texture(water_tex, base_uv).r;
		if (level > -9000.0) {
			float above = world_pos.y - level;
			// Wet band: darkest at the waterline; under water only a little
			// darker (the water surface tints it).
			float wet = smoothstep(0.75, 0.04, above + (macro_near.g - 0.5) * 0.25);
			float dry_above = step(0.0, above);
			albedo *= mix(1.0, mix(0.82, 0.58, dry_above), wet);
			rough = mix(rough, 0.25, wet * dry_above);
			detail *= 1.0 - 0.4 * wet;
		}
	}

	// Rain (prompt 040): darker and glossier everywhere, puddles on flat
	// soft ground (driven tracks, mud and bare earth more than grass).
	if (wetness > 0.0) {
		float flat_wet = smoothstep(0.3, 0.08, grade);
		float soil = clamp(w[TRACK] + w[EARTH] + w[4] + 0.35 * w[0] + rut + churned, 0.0, 1.0);
		float puddle = smoothstep(0.62, 0.7, macro_near.r + 0.25 * soil + 0.15 * wetness - 0.2)
			* flat_wet * soil * wetness * (1.0 - far * 0.5);
		// Water stands in deep ruts; churned mud glistens.
		puddle = max(puddle, smoothstep(0.55, 0.95, rut) * smoothstep(0.4, 0.75, macro_near.g) * flat_wet * wetness);
		rough = mix(rough, 0.18, clamp(rut + churned, 0.0, 1.0) * wetness * 0.6);
		albedo *= mix(1.0, 0.62, wetness);
		rough = mix(rough, 0.32, wetness * 0.6);
		albedo = mix(albedo, albedo * 0.45 + vec3(0.02, 0.025, 0.03), puddle);
		rough = mix(rough, 0.04, puddle);
		detail *= 1.0 - 0.8 * puddle;
	}

	// Snow cover (winter): on flat ground, patchy, thinner on roads and
	// tracks (driven on) and none on water.
	// Mountains: snow above the snow line, also on steep slopes but not
	// on the steepest rock.
	float alpine = smoothstep(snow_line - snow_line_fade, snow_line + snow_line_fade,
		world_pos.y + (macro_far.b - 0.5) * snow_line_fade * 2.0);
	if (snow_cover > 0.0 || alpine > 0.0) {
		float flat_ground = smoothstep(0.55, 0.2, grade);
		float worn = clamp(w[TRACK] + w[ROAD], 0.0, 1.0);
		float water = clamp(w[SHALLOW] + w[DEEP], 0.0, 1.0);
		float cover = max(snow_cover * flat_ground, alpine * smoothstep(1.1, 0.6, grade));
		// Stalks of tall grass poke through the snow.
		float s = cover * (1.0 - 0.55 * worn) * (1.0 - water) * (1.0 - 0.45 * tall);
		s = smoothstep(0.25, 0.65, s + (macro_near.r - 0.5) * 0.45 + (macro_far.b - 0.5) * 0.25);
		float tile = layer_tile[SNOW];
		vec4 snow_col = textureGrad(albedo_array, vec3(p / tile, float(SNOW)), dpdx / tile, dpdy / tile);
		vec4 snow_nrm = textureGrad(normal_array, vec3(p / tile, float(SNOW)), dpdx / tile, dpdy / tile);
		albedo = mix(albedo, snow_col.rgb, s);
		// Pressed snow in tracks: greyer and a little blue.
		float pressed = clamp(rut + churned * 0.8 + worn * 0.3, 0.0, 1.0) * s;
		albedo = mix(albedo, albedo * vec3(0.72, 0.76, 0.82), pressed);
		// Rows and furrows of snowed fields show faintly (prompt 088).
		albedo *= 1.0 - 0.1 * field_rows * s;
		rough = mix(rough, snow_nrm.b, s);
		ao = mix(ao, snow_nrm.a, s);
		detail *= 1.0 - 0.6 * s;
	}

	if (show_lod) {
		float level = log2(lod_stride);
		vec3 tint = vec3[](vec3(1, 0.3, 0.3), vec3(1, 0.8, 0.2), vec3(0.3, 1, 0.3),
			vec3(0.2, 0.7, 1), vec3(0.8, 0.4, 1))[clamp(int(level + 0.5), 0, 4)];
		albedo = mix(albedo, tint, 0.45);
	}

	detail += track_detail;
	vec3 n = normalize(base_n + detail * base_n.y);
	ALBEDO = albedo;
	ROUGHNESS = rough;
	SPECULAR = 0.35;
	AO = ao;
	AO_LIGHT_AFFECT = 0.4;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
