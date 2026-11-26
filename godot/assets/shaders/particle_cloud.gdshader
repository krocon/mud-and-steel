shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled;

#define ATLAS
#define CLOUD
#include "res://assets/shaders/particle_common.gdshaderinc"

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = particle_soft_alpha(t.a * c.a, SCREEN_UV, VERTEX.z, INV_PROJECTION_MATRIX);
	vec2 p = v_quad_uv * 2.0 - 1.0;
	vec3 dome = vec3(p.x, -p.y, 0.9);
	vec3 place = vec3(0.0, 0.45, 0.0);
	if (cloud_center.w != 0.0) {
		vec3 offset = v_center - cloud_center.xyz;
		if (cloud_center.w < 0.0) {
			offset.y = 0.0;
		}
		place += offset / abs(cloud_center.w);
	}
	vec2 n = t.rg * 2.0 - 1.0;
	NORMAL = normalize(mat3(VIEW_MATRIX) * place + dome * 0.55 + vec3(n * 0.2, 0.0));
	ALBEDO = c.rgb * mix(0.7, 1.0, t.b);
	EMISSION = ALBEDO * emission;
	// Prompt 220: warm orange underlight from the fire core on lower smoke collars.
	if (fire_glow > 0.0) {
		float h = max(v_center.y - cloud_center.y, 0.0);
		float low = exp(-h * 0.24) * (1.0 - smoothstep(0.12, 0.78, v_age));
		float under = clamp(0.5 + p.y * 0.45, 0.15, 1.0);
		EMISSION += vec3(1.0, 0.44, 0.10) * fire_glow * low * under * mix(0.65, 1.2, t.b);
	}
	ALPHA = alpha;
}
