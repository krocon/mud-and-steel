// Infantry material (prompt 042): procedural cloth, canvas, leather, metal, wood
// and skin detail, camouflage patterns and soiled winter whites. No texture files.
//
// One ShaderMaterial per faction x biome x slot x skin tone x camouflage, made and
// cached by SoldierLook (godot/scenes/tools/soldier_look.gd), so a squad shares a
// handful of materials and the far MultiMesh soldiers use the same ones.
//
// Pattern space: the soldier build (tools/blender/soldier/mesh.py) stores every
// vertex's REST-POSE position in meters in the UVs: UV = (x, 1 - z), UV2 = (y, 1)
// after glTF's V flip, with
// Blender axes (x right, y forward, z up; weapons: y along the barrel). Patterns
// drawn there stay on the cloth while the skinned body moves.
// Presentation only: nothing here feeds back into the simulation.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// 0 Cloth, 1 Canvas, 2 Leather, 3 Paint, 4 Skin, 5 Metal, 6 Wood, 7 Rubber, 8 Glass,
// 9 Insignia (same order as SoldierLook.SLOTS)
uniform int slot = 0;
uniform vec4 base_color : source_color = vec4(0.4, 0.4, 0.3, 1.0);
uniform float roughness = 0.9;
uniform float metallic = 0.0;
// 0 none, 1 splinter (Heer Splittertarn), 2 amoeba (Soviet camouflage suit),
// 3 frogskin (US M1942, green side)
uniform int camo = 0;
uniform vec4 camo_base : source_color = vec4(0.6, 0.58, 0.47, 1.0);
uniform vec4 camo_1 : source_color = vec4(0.31, 0.35, 0.23, 1.0);
uniform vec4 camo_2 : source_color = vec4(0.42, 0.29, 0.2, 1.0);
uniform vec4 camo_3 : source_color = vec4(0.18, 0.23, 0.15, 1.0);
uniform vec4 camo_4 : source_color = vec4(0.37, 0.29, 0.21, 1.0);
// pattern size in meters (splinter cell, amoeba blob, frogskin spot)
uniform float camo_scale = 0.11;
// the pattern covers rest-pose heights above this (smock hem); -1 = whole garment
uniform float camo_min_z = -1.0;
// helmet cover in the camouflage pattern (Paint above 1.55 m rest height)
uniform bool camo_helmet = false;
// winter: grime on white over-garments (0 = clean)
uniform float soil = 0.0;

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
	return 0.5 * noise(p) + 0.3 * noise(p * 2.1 + vec3(3.1)) + 0.2 * noise(p * 4.3 + vec3(7.7));
}

// Splittertarn: angular cells (nearest point of a stretched, rotated grid) in base,
// green and brown, overlaid with short dark-green rain strokes.
vec3 splinter(vec3 p, float detail) {
	// shear and stretch so the cells become long, slanted splinters
	vec3 q = vec3(p.x * 0.9 + p.z * 0.5, p.y, p.z * 0.9 - p.x * 0.5) / camo_scale;
	q.x *= 0.55;
	vec3 cell = floor(q);
	float best = 9.0;
	vec3 id = cell;
	for (int x = -1; x <= 1; x++) {
		for (int z = -1; z <= 1; z++) {
			vec3 c = cell + vec3(float(x), 0.0, float(z));
			vec3 pt = c + vec3(hash(c), 0.5, hash(c + vec3(4.2, 1.3, 7.1)));
			// Manhattan distance gives straight, angular borders
			vec3 d = abs(q - pt);
			float dist = d.x + d.z;
			if (dist < best) {
				best = dist;
				id = c;
			}
		}
	}
	float h = hash(id + vec3(2.7, 0.0, 5.3));
	vec3 col = h < 0.42 ? camo_base.rgb : (h < 0.74 ? camo_1.rgb : camo_2.rgb);
	// rain: thin strokes (4 mm wide, 2.5 cm apart, slanted), broken into dashes
	vec2 r = vec2(p.x + p.y * 0.4 + p.z * 0.18, p.z);
	float lane = floor(r.x * 40.0);
	float line = 1.0 - smoothstep(0.12, 0.2, abs(fract(r.x * 40.0) - 0.5) * 2.0 - 0.6);
	float dash = step(0.5, noise(vec3(lane * 1.7, r.y * 14.0, 0.0)));
	return mix(col, camo_3.rgb, line * dash * 0.85 * detail);
}

// Soviet camouflage suit: large dark blobs with ragged edges on olive khaki.
vec3 amoeba(vec3 p) {
	float n = fbm(p / camo_scale);
	float m = smoothstep(0.56, 0.6, n);
	float m2 = smoothstep(0.6, 0.64, fbm(p / camo_scale * 0.7 + vec3(9.1, 3.3, 1.7))) * (1.0 - m);
	return mix(mix(camo_base.rgb, camo_1.rgb, m), camo_2.rgb, m2 * 0.6);
}

// M1942 frogskin, green side: rounded spots of four colors on pale green.
vec3 frogskin(vec3 p) {
	vec3 q = p / camo_scale;
	vec3 col = camo_base.rgb;
	col = mix(col, camo_1.rgb, smoothstep(0.66, 0.7, noise(q)));
	col = mix(col, camo_2.rgb, smoothstep(0.68, 0.72, noise(q * 1.1 + vec3(11.3, 5.1, 2.9))));
	col = mix(col, camo_3.rgb, smoothstep(0.7, 0.74, noise(q * 0.9 + vec3(4.7, 17.9, 8.3))));
	col = mix(col, camo_4.rgb, smoothstep(0.72, 0.76, noise(q * 1.2 + vec3(23.1, 2.2, 13.7))));
	return col;
}

// Prompt 048: the helmet (or cap) flew off (presentation effect, set per soldier
// on his Gear mesh by infantry_view.gd). Paint and Cloth above HELMET_Z rest height
// are only headgear; those vertices collapse to a point, so the headgear vanishes.
instance uniform float helmet_off = 0.0;
const float HELMET_Z = 1.62;

void vertex() {
	if (helmet_off > 0.5 && (slot == 3 || slot == 0) && 1.0 - UV.y > HELMET_Z) {
		VERTEX = vec3(0.0);
	}
}

void fragment() {
	// glTF stores V flipped (v = 1 - v_blender)
	vec3 p = vec3(UV.x, UV2.x, 1.0 - UV.y);
	// fade fine detail when one pixel covers more than a few centimeters (no shimmer
	// at RTS distance)
	float px = length(fwidth(p));
	float detail = 1.0 - smoothstep(0.006, 0.03, px);
	float grain = noise(p * 40.0);
	float mottle = fbm(p * 6.0);

	vec3 albedo = base_color.rgb;
	float rough = roughness;
	float metal = metallic;
	bool garment = slot == 0;
	if (slot == 0) {
		// cloth: faded, mottled wool with fine weave and darker folds
		float fold = smoothstep(0.55, 0.85, noise(vec3(p.x * 9.0, p.y * 9.0, p.z * 3.0)));
		albedo *= 0.9 + 0.16 * mottle - 0.12 * fold + 0.1 * (grain - 0.5) * detail;
	} else if (slot == 1) {
		// canvas webbing, bags, gaiters: coarse weave, stains
		float weave = abs(sin(p.x * 260.0 + p.z * 3.0)) * abs(sin(p.z * 260.0));
		albedo *= 0.86 + 0.2 * mottle + 0.12 * (weave - 0.5) * detail;
	} else if (slot == 2) {
		// leather: creases, polished worn spots
		float crease = noise(p * vec3(30.0, 30.0, 70.0));
		albedo *= 0.8 + 0.35 * crease;
		rough = mix(0.35, 0.75, fbm(p * 12.0));
	} else if (slot == 3) {
		// helmet paint: rough, scratched to bare metal in places
		albedo *= 0.9 + 0.18 * (grain - 0.5) * detail + 0.1 * mottle;
		float chip = step(0.86, noise(p * 55.0)) * detail;
		albedo = mix(albedo, vec3(0.24, 0.23, 0.22), chip);
		rough = mix(rough, 0.45, chip);
	} else if (slot == 4) {
		// skin: slightly blotchy tone
		albedo *= 0.94 + 0.1 * mottle;
	} else if (slot == 5) {
		// blued steel, rubbed bright on edges and wear spots
		float wear = smoothstep(0.62, 0.8, noise(p * 25.0));
		albedo = mix(albedo * (0.8 + 0.3 * grain), vec3(0.42, 0.42, 0.43), wear * 0.6);
		rough = mix(0.5, 0.3, wear);
	} else if (slot == 6) {
		// stock wood: grain along the weapon (rest y)
		float g = noise(vec3(p.x * 90.0, p.y * 3.0, p.z * 90.0));
		float rings = sin((p.x + p.z) * 160.0 + g * 6.0) * 0.5 + 0.5;
		albedo *= 0.78 + 0.22 * g + 0.15 * rings * detail;
		rough = 0.55;
	}

	if (garment && camo > 0 && p.z > camo_min_z) {
		vec3 pattern = camo == 1 ? splinter(p, detail) : (camo == 2 ? amoeba(p) : frogskin(p));
		float shade = 0.9 + 0.16 * mottle;
		albedo = pattern * shade;
	}
	if (slot == 3 && camo > 0 && camo_helmet && p.z > 1.55) {
		vec3 pattern = camo == 1 ? splinter(p, detail) : (camo == 2 ? amoeba(p) : frogskin(p));
		albedo = pattern * (0.92 + 0.1 * mottle);
		rough = 0.95;
	}
	if (soil > 0.0 && (slot <= 1 || slot == 3)) {
		// winter whites: grey grime, dirt at knees, cuffs and hems
		float low = 1.0 - smoothstep(0.08, 0.55, p.z);
		float grime = soil * (0.35 * fbm(p * 3.0) + 0.5 * low * fbm(p * 9.0));
		albedo = mix(albedo, albedo * vec3(0.66, 0.63, 0.58), clamp(grime, 0.0, 0.8));
	}

	ALBEDO = albedo * COLOR.rgb;
	ROUGHNESS = rough;
	METALLIC = metal;
}
