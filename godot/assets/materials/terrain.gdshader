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
// Layer order: see LAYERS in crates/cli/src/gen_textures.rs.
shader_type spatial;
render_mode cull_back;

const int LAYER_COUNT = 10;
const int TRACK = 1;
const int ROAD = 2;
const int SNOW = 5;
const int SHALLOW = 6;
const int DEEP = 7;
const int ROCK = 8;
const int EARTH = 9;

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
// 1: tint every chunk by its LOD level (debug menu).
uniform bool show_lod = false;
uniform sampler2D water_tex : filter_linear, repeat_disable;
uniform bool has_water = false;

varying vec3 world_pos;
varying float lod_stride;

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
	float earth = max(smoothstep(earth_grade.x, earth_grade.y, grade), patch * 0.8);
	float rock = smoothstep(rock_grade.x, rock_grade.y, grade + (macro_near.r - 0.5) * 0.1);
	w[ROCK] = soft * rock;
	w[EARTH] = soft * earth * (1.0 - rock);
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
		// larger, blended by noise, so the repeat is hard to see.
		float scale_b = 1.37 * mix(1.0, 4.0, far);
		float angle_b = i == TRACK ? 0.0 : 1.1 + float(i);
		vec2 uv_a = q / tile;
		vec2 uv_b = rotate(q, angle_b) / (tile * scale_b) + vec2(0.31, 0.71) * float(i + 1);
		vec2 gax = gx / tile;
		vec2 gay = gy / tile;
		vec2 gbx = rotate(gx, angle_b) / (tile * scale_b);
		vec2 gby = rotate(gy, angle_b) / (tile * scale_b);
		float src = use_biome ? float(layer_source[i]) : float(i);
		vec4 ca = textureGrad(albedo_array, vec3(uv_a, src), gax, gay);
		vec4 cb = textureGrad(albedo_array, vec3(uv_b, src), gbx, gby);
		vec4 na = textureGrad(normal_array, vec3(uv_a, src), gax, gay);
		vec4 nb = textureGrad(normal_array, vec3(uv_b, src), gbx, gby);
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

	// Snow cover (winter): on flat ground, patchy, thinner on roads and
	// tracks (driven on) and none on water.
	if (snow_cover > 0.0) {
		float flat_ground = smoothstep(0.55, 0.2, grade);
		float worn = clamp(w[TRACK] + w[ROAD], 0.0, 1.0);
		float water = clamp(w[SHALLOW] + w[DEEP], 0.0, 1.0);
		float s = snow_cover * flat_ground * (1.0 - 0.55 * worn) * (1.0 - water);
		s = smoothstep(0.25, 0.65, s + (macro_near.r - 0.5) * 0.45 + (macro_far.b - 0.5) * 0.25);
		float tile = layer_tile[SNOW];
		vec4 snow_col = textureGrad(albedo_array, vec3(p / tile, float(SNOW)), dpdx / tile, dpdy / tile);
		vec4 snow_nrm = textureGrad(normal_array, vec3(p / tile, float(SNOW)), dpdx / tile, dpdy / tile);
		albedo = mix(albedo, snow_col.rgb, s);
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

	vec3 n = normalize(base_n + detail * base_n.y);
	ALBEDO = albedo;
	ROUGHNESS = rough;
	SPECULAR = 0.35;
	AO = ao;
	AO_LIGHT_AFFECT = 0.4;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
