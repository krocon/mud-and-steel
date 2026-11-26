shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled, unshaded;

#define ATLAS
#include "res://assets/shaders/particle_common.gdshaderinc"

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = particle_soft_alpha(t.a * c.a, SCREEN_UV, VERTEX.z, INV_PROJECTION_MATRIX);
	float age = smoothstep(0.15, 0.65, v_age);
	vec3 crease = mix(c.rgb * vec3(1.0, 0.5, 0.2) * 0.75, c.rgb * 0.25, age);
	ALBEDO = mix(crease, c.rgb * 1.05, smoothstep(0.35, 0.9, t.b)) * (1.0 + emission);
	ALPHA = alpha;
}
