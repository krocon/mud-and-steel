// Thin outline around the unit under the mouse pointer (prompt 167), set by
// scenes/world/unit_highlight.gd as the last next pass of a mesh's material
// overlay, only on the hovered unit. Presentation only.
//
// The mesh is drawn again, every vertex pushed outward along its normal by
// `width_px` screen pixels (the same width at every zoom). The stencil
// keeps only the rim: every unit model writes 1 where it is visible
// (unit_camo.gdshader, soldier.gdshader; see unit_xray.gdshader), and this
// pass draws only where the stencil is still 0 (compare_greater: 1 > 0),
// writing 1 itself so overlapping meshes add no opacity. The normal depth
// test hides the rim behind houses and hills.
// The color is the instance parameter `outline_color` (its alpha fades the
// outline in and out). Its index is fixed above unit_camo.gdshader's 13
// instance parameters and unit_xray.gdshader's `xray_color` (14).
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_back, shadows_disabled, fog_disabled,
		skip_vertex_transform;
stencil_mode read, write, compare_greater, 1;

instance uniform vec4 outline_color : source_color, instance_index(15) = vec4(0.3, 0.55, 1.0, 0.0);
// Rim width in screen pixels (unit_highlight.gd scales it with the HUD).
uniform float width_px = 2.5;

void vertex() {
	VERTEX = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	NORMAL = normalize((MODELVIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	vec4 clip = PROJECTION_MATRIX * vec4(VERTEX, 1.0);
	vec2 dir = (PROJECTION_MATRIX * vec4(NORMAL, 0.0)).xy;
	float len = length(dir);
	dir = len > 1e-6 ? dir / len : vec2(0.0);
	clip.xy += dir * width_px * 2.0 / VIEWPORT_SIZE * clip.w;
	POSITION = clip;
}

void fragment() {
	ALBEDO = outline_color.rgb;
	ALPHA = outline_color.a;
}
