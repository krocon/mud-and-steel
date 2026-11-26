shader_type spatial;
render_mode unshaded, depth_test_disabled, depth_draw_never, cull_disabled, blend_mix, fog_disabled, shadows_disabled;

uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
uniform float desaturation = 0.6;

#include "res://assets/shaders/fog_common.gdshaderinc"

void fragment() {
	float v = seen(SCREEN_UV, INV_VIEW_MATRIX, INV_PROJECTION_MATRIX);
	// Prompt 208: avoid discard (TBDR-friendly) and only sample screen_texture
	// where the pixel is actually under fog.
	if (v < 0.0 || v >= 1.0) {
		ALBEDO = vec3(0.0);
		ALPHA = 0.0;
	} else {
		vec3 color = texture(screen_texture, SCREEN_UV).rgb;
		ALBEDO = vec3(dot(color, vec3(0.2126, 0.7152, 0.0722)));
		ALPHA = desaturation * (1.0 - v);
	}
}
