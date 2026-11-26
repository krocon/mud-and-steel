shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, shadows_disabled;

#define ATLAS
#include "res://assets/shaders/particle_common.gdshaderinc"

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = COLOR * tint;
	float alpha = particle_soft_alpha(t.a * c.a, SCREEN_UV, VERTEX.z, INV_PROJECTION_MATRIX);
	vec2 n = t.rg * 2.0 - 1.0;
	vec2 r = vec2(n.x * cos(v_angle) + n.y * sin(v_angle), -n.x * sin(v_angle) + n.y * cos(v_angle));
	vec2 p = fract(UV * 2.0) * 2.0 - 1.0;
	NORMAL = normalize(vec3(r * 0.55 + vec2(p.x, -p.y) * 0.25, 0.85));
	ALBEDO = c.rgb * mix(0.6, 1.0, t.b);
	EMISSION = ALBEDO * emission;
	ALPHA = alpha;
}
