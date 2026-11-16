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

// Soot over the albedo of a charred object; a little texture stays visible.
vec3 charred(vec3 albedo, float amount) {
	float l = dot(albedo, vec3(0.3, 0.59, 0.11));
	return mix(albedo, vec3(0.035, 0.032, 0.03) + l * 0.12, clamp(amount, 0.0, 1.0) * 0.92);
}

// Wind, prepared as material parameters (prompt 015 drives them globally).
uniform float wind_strength = 0.25;
uniform vec2 wind_direction = vec2(1.0, 0.3);
uniform float wind_speed = 1.2;

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

// Sway of plant vertices. `weight` = wind weight, `phase` = per branch/card.
vec3 wind_offset(vec3 vertex, float weight, float phase, vec3 instance_pos, float time) {
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	float t = time * wind_speed + dot(instance_pos.xz, vec2(0.13, 0.07));
	float sway = sin(t) * 0.6 + sin(t * 2.3 + 1.1) * 0.25;
	float flutter = sin(time * 7.0 + phase * 6.2831 + vertex.y * 1.5) * 0.12;
	float h = weight * weight;
	vec3 off = vec3(dir.x, 0.0, dir.y) * (sway * h + flutter * weight) * wind_strength;
	return off;
}

// `world_normal`: the normal the object had standing (its model normal;
// objects only turn about the vertical), so snow stays where it lay when a
// tree falls (prompt 056).
vec3 snow_cover(vec3 albedo, inout float rough, vec3 world_normal, vec3 p, float amount) {
	float s = amount * smoothstep(0.45, 0.8, world_normal.y + (fbm3(p * 2.0) - 0.5) * 0.5);
	rough = mix(rough, 0.75, s);
	return mix(albedo, vec3(0.9, 0.92, 0.95), s);
}
