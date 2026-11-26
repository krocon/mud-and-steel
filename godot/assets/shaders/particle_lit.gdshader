shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled;

#include "res://assets/shaders/particle_common.gdshaderinc"

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = particle_soft_alpha(t.a * c.a, SCREEN_UV, VERTEX.z, INV_PROJECTION_MATRIX);
	vec2 p = UV * 2.0 - 1.0;
	NORMAL = normalize(vec3(p.x, -p.y, 1.1));
	ALBEDO = t.rgb * c.rgb;
	EMISSION = ALBEDO * emission;
	ALPHA = alpha;
}
