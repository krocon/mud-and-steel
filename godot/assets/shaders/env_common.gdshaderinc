// Shared code of the environment shaders (env.gdshader, env_foliage.gdshader).
//
// Vertex colors baked by tools/blender/kit/env.py:
//   R = ambient occlusion
//   G = Bark/Foliage: wind weight, others: edge wear
//   B = Bark/Foliage: wind phase, others: height above ground / 4 m
//   A = material kind of the slot, A = 1 - kind / 4
// Presentation only: nothing here feeds back into the simulation.

// Biome look, set per mesh by EnvLook.apply() (instance uniforms, so all
// environment objects share the same few materials).
instance uniform vec4 env_foliage : source_color = vec4(0.27, 0.38, 0.14, 1.0);
instance uniform float env_dry = 0.0;
instance uniform float env_snow = 0.0;
instance uniform float env_dust = 0.0;
instance uniform vec4 env_paint : source_color = vec4(0.33, 0.35, 0.32, 1.0);
// Charred by fire (prompt 041, map_objects.gd set_charred): 0 = not, 1 = black.
instance uniform float env_char = 0.0;
// Fallen or flattened (prompt 056, map_objects.gd _falling): 1 = lying, no
// wind sway.
instance uniform float env_fallen = 0.0;
// Stripped of leaves by a big blast (prompt 085, map_objects.gd): 1 = the
// foliage cards fold away (env_foliage.gdshader), trunk and limbs stay.
instance uniform float env_bare = 0.0;

// Soot over the albedo of a charred object; a little texture stays visible.
vec3 charred(vec3 albedo, float amount) {
	float l = dot(albedo, vec3(0.3, 0.59, 0.11));
	return mix(albedo, vec3(0.035, 0.032, 0.03) + l * 0.12, clamp(amount, 0.0, 1.0) * 0.92);
}

// Wind, prepared as material parameters (prompt 015 drives them globally;
// atmosphere.gd from the biome preset, scaled by the weather).
uniform float wind_strength = 0.25;
uniform vec2 wind_direction = vec2(1.0, 0.3);
uniform float wind_speed = 1.2;
// Prompt 086, per biome (BiomePreset): bend of the whole tree and flutter
// of the leaves as factors. 0 / 0 = still (thumbnails, tools).
uniform float tree_sway = 1.0;
uniform float leaf_flutter = 1.0;
// Bend of the whole object per kind (EnvLook.BEND, set by map_objects.gd):
// 0 for everything that is not a tree (houses with bark logs, fences,
// grass and bushes keep only the branch sway), conifers stiffer, birches
// softer.
instance uniform float env_bend = 0.0;
#include "res://assets/shaders/wind_common.gdshaderinc"

float hash1(vec3 p) {
	p = fract(p * 0.3183099 + vec3(0.11, 0.17, 0.13));
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float hash2(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vnoise(vec3 x) {
	vec3 i = floor(x);
	vec3 f = fract(x);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(mix(hash1(i), hash1(i + vec3(1, 0, 0)), f.x),
				   mix(hash1(i + vec3(0, 1, 0)), hash1(i + vec3(1, 1, 0)), f.x), f.y),
			   mix(mix(hash1(i + vec3(0, 0, 1)), hash1(i + vec3(1, 0, 1)), f.x),
				   mix(hash1(i + vec3(0, 1, 1)), hash1(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}

float fbm3(vec3 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * vnoise(p);
		p = p * 2.03 + vec3(1.7, 9.2, 3.1);
		a *= 0.5;
	}
	return v;
}

int kind_of(float a) {
	return int(round((1.0 - a) * 4.0));
}

// Wind on plant vertices (prompt 086), in model space. Bark and foliage call
// it with the same arguments (`leaf` 0 for bark), so the crown never slides
// off the trunk. Three parts:
// 1. the whole plant bends downwind, growing with the height above its foot
//    ((h / 20 m)^2, the foot stays), slowly swinging about a slight lean,
//    pushed further by the gust wave (wind_gust, shared with the grass);
// 2. branches and crown parts (vertex color G = weight, B = phase) swing
//    faster, along and across the wind and a little up and down;
// 3. leaves flutter (`leaf` 1), stronger in gusts.
// Phase, tempo and size differ per plant (from its position); large plants
// swing slower (their scale). At the forest preset (wind_strength 0.25) the
// tip of a 20 m tree swings 0.2-0.7 m and up to about 1.2 m in a gust.
vec3 wind_offset(vec3 vertex, float weight, float phase, mat4 model, float time, float leaf) {
	vec3 inst = model[3].xyz;
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	// World directions in model space (plants only turn about the vertical;
	// normalize takes out their scale).
	vec3 down_l = normalize((vec4(dir.x, 0.0, dir.y, 0.0) * model).xyz);
	vec3 across_l = normalize((vec4(-dir.y, 0.0, dir.x, 0.0) * model).xyz);
	float seed = hash1(inst * 0.37 + vec3(0.5));
	float scale = length(model[1].xyz);
	float tempo = (0.8 + 0.25 * wind_speed) * (0.85 + 0.3 * seed) / sqrt(max(scale, 0.3));
	float gust = wind_gust(inst.xz, time);
	// 1. The whole plant.
	float h = max(vertex.y, 0.0);
	float bend = min(h * h / 400.0, 2.0);
	float t = time * tempo * 0.9 + seed * 6.2831;
	float swing = sin(t) * 0.75 + sin(t * 2.13 + 1.7) * 0.25;
	float push = wind_strength * tree_sway * env_bend * 3.6 * bend * (0.5 + 0.3 * swing + 0.6 * gust);
	// 2. Branches.
	float bt = time * tempo * 2.4 + phase * 6.2831;
	float branch = sin(bt) * weight * weight * wind_strength * tree_sway * 0.6 * (0.8 + 1.2 * gust);
	// 3. Leaves.
	float flutter = sin(time * 9.0 + phase * 6.2831 + vertex.y * 1.7) * weight
		* wind_strength * leaf_flutter * leaf * 0.3 * (1.0 + gust);
	vec3 off = down_l * (push + branch * 0.6) + across_l * (branch * 0.5 + flutter * 0.5);
	// The crown sinks a little as the plant bends (keeps its length).
	off.y += branch * 0.25 + flutter - push * push * 0.5 / max(h, 1.0);
	return off;
}

// `world_normal`: the normal the object had standing (its model normal;
// objects only turn about the vertical), so snow stays where it lay when a
// tree falls (prompt 056).
vec3 snow_cover(vec3 albedo, inout float rough, vec3 world_normal, vec3 p, float amount) {
	// no snow in this biome: skip the noise (32 hashes per pixel)
	if (amount <= 0.0) {
		return albedo;
	}
	float s = amount * smoothstep(0.45, 0.8, world_normal.y + (fbm3(p * 2.0) - 0.5) * 0.5);
	rough = mix(rough, 0.75, s);
	return mix(albedo, vec3(0.9, 0.92, 0.95), s);
}
