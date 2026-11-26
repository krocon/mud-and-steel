shader_type spatial;
render_mode unshaded, depth_test_disabled, depth_draw_never, cull_disabled, blend_mul, fog_disabled, shadows_disabled;

uniform vec4 dim_tint : source_color = vec4(0.6, 0.63, 0.7, 1.0);

#include "res://assets/shaders/fog_common.gdshaderinc"

void fragment() {
	float v = seen(SCREEN_UV, INV_VIEW_MATRIX, INV_PROJECTION_MATRIX);
	// Prompt 208: blend_mul with vec3(1.0) is a no-op, avoiding discard on TBDR GPUs.
	float factor = (v < 0.0) ? 1.0 : v;
	ALBEDO = mix(dim_tint.rgb, vec3(1.0), factor);
}
