shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled, unshaded;

#include "res://assets/shaders/particle_common.gdshaderinc"

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = particle_soft_alpha(t.a * c.a, SCREEN_UV, VERTEX.z, INV_PROJECTION_MATRIX);
	ALBEDO = t.rgb * c.rgb * (1.0 + emission);
	ALPHA = alpha;
}
