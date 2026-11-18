// Shadows of passing clouds (prompt 087). Every world shader includes this
// file once and starts its fragment() with `CLOUD_FRAGMENT(...)`:
//
//     #include "res://assets/shaders/cloud_shadow.gdshaderinc"
//     ...
//     CLOUD_FRAGMENT(specular);  // first line of fragment()
//
// (`specular` = the shader's SPECULAR, 0.5 where it sets none). Shaders that
// write BACKLIGHT define CLOUD_BACKLIGHT before the include.
//
// The file brings the shader's light() function: Godot's own diffuse_burley
// and specular_schlick_ggx (the same terms as the built-in lighting, so a
// model looks as before), with the clouds dimming only the direct light of
// the sun (`ATTENUATION` of the directional light, which already holds the
// sun's shadow, so a tree's shadow does not turn doubly dark under a cloud).
// Ambient light, sky reflections and other lights stay as they are.
//
// One pattern in world coordinates for every material: two layers of the
// same tileable noise (cloud_shadow_tex), moved by atmosphere.gd with the
// wind of the simulation and the game time. Global shader uniforms
// (project.godot [shader_globals]); their defaults switch the clouds off,
// so thumbnails, the model gallery and the map editor show none.
// Presentation only.

global uniform sampler2D cloud_shadow_tex : filter_linear_mipmap, repeat_enable;
// xy, zw: offsets of the two layers (in texture repeats).
global uniform vec4 cloud_shadow_offset;
// x: texture repeats per meter, y: strength (share of the sun taken in the
// core of a cloud shadow, 0 = off), z: cover (noise level above which it is
// shadow), w: softness (half the noise range of the soft edge).
global uniform vec4 cloud_shadow_shape;

varying vec3 cloud_world;
varying float cloud_specular;

// Share of the direct sunlight that reaches the world point `p` (0..1).
float cloud_sun(vec3 p) {
	if (cloud_shadow_shape.y <= 0.0) {
		return 1.0;
	}
	vec2 uv = p.xz * cloud_shadow_shape.x;
	float a = texture(cloud_shadow_tex, uv + cloud_shadow_offset.xy).r;
	float b = texture(cloud_shadow_tex, uv * 1.7 + vec2(0.37, 0.61) + cloud_shadow_offset.zw).r;
	float n = a * 0.62 + b * 0.38;
	float shade = smoothstep(cloud_shadow_shape.z - cloud_shadow_shape.w, cloud_shadow_shape.z + cloud_shadow_shape.w, n);
	return 1.0 - cloud_shadow_shape.y * shade;
}

// Varyings can only be written in fragment() itself, hence a macro.
#define CLOUD_FRAGMENT(specular) cloud_world = (INV_VIEW_MATRIX * vec4(VERTEX, 1.0)).xyz; cloud_specular = specular;

float cloud_schlick(float u) {
	float m = 1.0 - u;
	float m2 = m * m;
	return m2 * m2 * m;
}

void light() {
	float attenuation = ATTENUATION;
	if (LIGHT_IS_DIRECTIONAL) {
		attenuation *= cloud_sun(cloud_world);
	}
	vec3 n = NORMAL;
	vec3 l = LIGHT;
	vec3 v = VIEW;
	float ndotl = min(dot(n, l), 1.0);
	float c_ndotl = max(ndotl, 0.0);
	float c_ndotv = max(dot(n, v), 1e-4);
	vec3 h = normalize(v + l);
	float c_ndoth = clamp(dot(n, h), 0.0, 1.0);
	float c_ldoth = clamp(dot(l, h), 0.0, 1.0);
	float roughness = ROUGHNESS;
	float metallic = METALLIC;
	if (metallic < 1.0) {
		// diffuse_burley
		float fd90_minus_1 = 2.0 * c_ldoth * c_ldoth * roughness - 0.5;
		float fdv = 1.0 + fd90_minus_1 * cloud_schlick(c_ndotv);
		float fdl = 1.0 + fd90_minus_1 * cloud_schlick(c_ndotl);
		float diffuse_brdf_nl = (1.0 / PI) * fdv * fdl * c_ndotl;
		DIFFUSE_LIGHT += LIGHT_COLOR * diffuse_brdf_nl * attenuation;
#ifdef CLOUD_BACKLIGHT
		DIFFUSE_LIGHT += LIGHT_COLOR * (vec3(1.0 / PI) - diffuse_brdf_nl) * BACKLIGHT * attenuation;
#endif
	}
	if (roughness > 0.0) {
		// specular_schlick_ggx
		float alpha_ggx = roughness * roughness;
		float a = c_ndoth * alpha_ggx;
		float k = alpha_ggx / (1.0 - c_ndoth * c_ndoth + a * a);
		float d = k * k * (1.0 / PI);
		float g = 0.5 / mix(2.0 * c_ndotl * c_ndotv, c_ndotl + c_ndotv, alpha_ggx);
		float dielectric = 0.16 * cloud_specular * cloud_specular;
		vec3 f0 = mix(vec3(dielectric), ALBEDO, vec3(metallic));
		float f90 = clamp(dot(f0, vec3(50.0 * 0.33)), metallic, 1.0);
		vec3 f = f0 + (f90 - f0) * cloud_schlick(c_ldoth);
		vec3 specular_brdf_nl = c_ndotl * d * f * g;
		SPECULAR_LIGHT += specular_brdf_nl * LIGHT_COLOR * attenuation * SPECULAR_AMOUNT;
	}
}
