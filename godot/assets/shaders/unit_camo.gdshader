// Shared unit material: faction base color, camouflage pattern, edge wear,
// mud and dust from below, winter whitewash and snow on top faces.
//
// One ShaderMaterial per material slot (godot/assets/materials/unit/*.tres,
// assigned on import by assets/import/unit_post_import.gd). Everything that
// differs per unit (palette of faction x biome, mud, snow, wear) is an
// *instance* uniform, set per mesh by UnitCamo.apply(), so hundreds of units
// share the same few materials.
//
// Vertex colors baked by tools/blender/kit/bake.py:
//   R = ambient occlusion, G = edge wear mask, B = height above ground / 4 m,
//   A = 1 - track flag (0 on track link surfaces).
// Presentation only: nothing here feeds back into the simulation.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// 0 Paint, 1 Metal, 2 Rubber, 3 Wood, 4 Canvas, 5 Glass, 6 Skin, 7 Cloth
uniform int slot = 0;

instance uniform vec4 camo_base : source_color = vec4(0.45, 0.43, 0.30, 1.0);
instance uniform vec4 camo_color_1 : source_color = vec4(0.30, 0.34, 0.22, 1.0);
instance uniform vec4 camo_color_2 : source_color = vec4(0.38, 0.25, 0.17, 1.0);
// 0 plain, 1 soft sprayed blotches, 2 hard-edged patches, 3 ambush dots
instance uniform float camo_pattern = 0.0;
// pattern size in meters and share of the surface covered by color 1 and 2
instance uniform float camo_scale = 1.5;
instance uniform float camo_coverage = 0.4;
instance uniform float winter_wash = 0.0;
instance uniform float snow = 0.0;
instance uniform float mud = 0.5;
instance uniform vec4 mud_color : source_color = vec4(0.25, 0.20, 0.14, 1.0);
instance uniform float dust = 0.0;
instance uniform float wear = 0.5;
// track link scroll (links), animated by the unit when it drives
instance uniform float track_offset = 0.0;

varying vec3 v_local;
varying vec3 v_world_normal;

float hash(vec3 p) {
	p = fract(p * 0.3183099 + vec3(0.11, 0.17, 0.13));
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise(vec3 x) {
	vec3 i = floor(x);
	vec3 f = fract(x);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x),
				   mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
			   mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x),
				   mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}

float fbm(vec3 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * noise(p);
		p = p * 2.03 + vec3(1.7, 9.2, 3.1);
		a *= 0.5;
	}
	return v;
}

// Mask that covers `coverage` of the surface; `edge` = softness of the border.
float blotch(float n, float coverage, float edge) {
	float t = 1.0 - coverage;
	return smoothstep(t * 0.5 + 0.25 - edge, t * 0.5 + 0.25 + edge, n);
}

void vertex() {
	v_local = VERTEX;
	v_world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

vec3 camo(vec3 p) {
	vec3 q = p / max(camo_scale, 0.05);
	vec3 col = camo_base.rgb;
	if (camo_pattern < 0.5) {
		return col;
	}
	float edge = camo_pattern < 1.5 ? 0.06 : 0.01;
	float n1 = fbm(q * 0.6);
	float n2 = fbm(q * 0.6 + vec3(23.1, 7.7, 11.3));
	float m1 = blotch(n1, camo_coverage, edge);
	float m2 = blotch(n2, camo_coverage * 0.6, edge) * (1.0 - m1);
	col = mix(col, camo_color_1.rgb, m1);
	col = mix(col, camo_color_2.rgb, m2);
	if (camo_pattern > 2.5) {
		// ambush scheme: small base-colored dots over the patches
		float dots = step(0.78, noise(q * 9.0));
		col = mix(col, camo_base.rgb, dots * max(m1, m2));
	}
	return col;
}

void fragment() {
	vec4 vc = COLOR;
	float ao = vc.r;
	float edge = vc.g;
	float height = vc.b * 4.0;
	float is_track = 1.0 - vc.a;
	vec3 p = v_local;
	vec3 wn = normalize(v_world_normal);
	float grain = noise(p * 6.0);

	vec3 albedo;
	float rough = 0.8;
	float metal = 0.0;
	if (slot == 0) {
		albedo = camo(p) * (0.94 + 0.12 * grain);
		rough = 0.82;
	} else if (slot == 1) {
		albedo = vec3(0.27, 0.26, 0.25) * (0.85 + 0.3 * grain);
		rough = 0.55;
		metal = 0.55;
		if (is_track > 0.5) {
			// links: v along the track (one unit per link), u across
			float v = fract(UV.y + track_offset);
			float grouser = smoothstep(0.08, 0.14, v) * (1.0 - smoothstep(0.86, 0.92, v));
			float guide = 1.0 - smoothstep(0.42, 0.46, abs(UV.x - 0.5) + 0.38);
			albedo = mix(vec3(0.10, 0.09, 0.085), vec3(0.30, 0.28, 0.26), grouser);
			albedo = mix(albedo, vec3(0.34, 0.32, 0.30), guide * 0.4);
			rough = mix(0.9, 0.45, grouser);
		}
	} else if (slot == 2) {
		albedo = vec3(0.055, 0.055, 0.05);
		rough = 0.92;
	} else if (slot == 3) {
		albedo = vec3(0.40, 0.29, 0.17) * (0.8 + 0.4 * noise(p * vec3(2.0, 2.0, 30.0)));
	} else if (slot == 4) {
		albedo = mix(camo_base.rgb, vec3(0.45, 0.42, 0.32), 0.6) * (0.9 + 0.2 * grain);
		rough = 0.95;
	} else if (slot == 5) {
		albedo = vec3(0.05, 0.07, 0.08);
		rough = 0.08;
		metal = 0.4;
	} else if (slot == 6) {
		albedo = vec3(0.72, 0.55, 0.44);
		rough = 0.6;
	} else {
		albedo = camo_color_1.rgb;
		rough = 0.95;
	}

	bool painted = slot == 0 || slot == 4 || slot == 7;
	if (painted && winter_wash > 0.0) {
		// brushed whitewash, worn through at edges and in occluded corners
		float coverage = fbm(p * 1.3 + vec3(5.0)) + 0.25 - edge * 0.35 - (1.0 - ao) * 0.4;
		float wash = winter_wash * smoothstep(0.5, 0.62, coverage);
		albedo = mix(albedo, vec3(0.78, 0.79, 0.76) * (0.95 + 0.1 * grain), wash);
	}
	if (slot == 0) {
		// chipped paint on edges and a few flecks elsewhere
		float chips = edge * smoothstep(0.45, 0.6, noise(p * 9.0)) + step(0.9, noise(p * 17.0)) * 0.6;
		float w = clamp(chips * wear, 0.0, 1.0);
		albedo = mix(albedo, vec3(0.22, 0.21, 0.20), w);
		rough = mix(rough, 0.5, w);
		metal = mix(metal, 0.4, w);
	}
	if (slot != 5) {
		// mud and dust thrown up from the ground
		float line = 0.55 + 0.45 * fbm(p * vec3(0.8, 2.0, 0.8)) + is_track * 1.5;
		float mud_mask = mud * (1.0 - smoothstep(line - 0.25, line + 0.1, height));
		mud_mask *= 0.55 + 0.45 * noise(p * 4.0);
		albedo = mix(albedo, mud_color.rgb * (0.85 + 0.3 * grain), clamp(mud_mask * 1.3, 0.0, 1.0));
		rough = mix(rough, 0.95, mud_mask);
		metal *= 1.0 - mud_mask;
		float dust_mask = dust * (0.25 + 0.5 * (1.0 - smoothstep(0.0, 2.5, height)) + 0.3 * max(wn.y, 0.0));
		albedo = mix(albedo, mud_color.rgb * 1.15, clamp(dust_mask * (0.6 + 0.4 * grain), 0.0, 0.85));
		// snow settles on upward faces
		float s = snow * smoothstep(0.5, 0.75, wn.y + (fbm(p * 2.5) - 0.5) * 0.5);
		albedo = mix(albedo, vec3(0.92, 0.93, 0.95), s);
		rough = mix(rough, 0.7, s);
		metal *= 1.0 - s;
	}

	ALBEDO = albedo * mix(0.5, 1.0, ao);
	AO = ao;
	AO_LIGHT_AFFECT = 0.25;
	ROUGHNESS = rough;
	METALLIC = metal;
}
