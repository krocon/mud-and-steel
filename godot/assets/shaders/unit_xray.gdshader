// Hidden parts of own and allied units as a flat silhouette (prompt 165),
// set by scenes/world/unit_highlight.gd as material overlay (or as the
// next pass of the ally tint and the hover light). Presentation only.
//
// Every unit model (unit_camo.gdshader, soldier.gdshader) and see-through
// cover (tall_grass.gdshader, meadow_tuft.gdshader) writes STENCIL_VISIBLE
// in its opaque pass. This pass draws only where
// - the unit's surface is behind what is in the depth buffer
//   (depth_test_inverted), by more than MIN_GAP_M (feet in the ground,
//   soldiers in a vehicle seat), and
// - the stencil is not STENCIL_VISIBLE: no unit and no tall grass is the
//   front-most thing there (so a unit never lights up through itself, another
//   unit or grass), and it writes STENCIL_VISIBLE itself, so overlapping
//   triangles and silhouettes are drawn once and the opacity stays even.
// Transparent things (smoke, dust, water) write no depth and hide nothing.
// The color is an instance parameter (`xray_color`, per mesh): one material
// for all units. Its index is fixed: instance parameters of all materials
// on a mesh share one slot list, and without it `xray_color` took the slot
// of `camo_base` in unit_camo.gdshader (13 instance parameters, 0-12) and
// painted the vehicles in the silhouette color.
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, depth_test_inverted, cull_back,
		shadows_disabled, fog_disabled;
// 1 = STENCIL_VISIBLE (unit_highlight.gd).
stencil_mode read, write, compare_not_equal, 1;

// Below this gap (meters along the view) between the scene in front and
// the unit's surface nothing is drawn.
const float MIN_GAP_M = 0.3;
// Edges of the silhouette (grazing angles) are drawn this much stronger.
const float RIM_GAIN = 0.45;

instance uniform vec4 xray_color : source_color, instance_index(14) = vec4(0.35, 0.95, 0.4, 0.6);
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;

void fragment() {
	float depth = texture(depth_tex, SCREEN_UV).r;
	vec4 scene = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	float scene_z = -scene.z / scene.w;
	if (-VERTEX.z - scene_z < MIN_GAP_M) {
		discard;
	}
	float rim = 1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0);
	ALBEDO = xray_color.rgb;
	ALPHA = clamp(xray_color.a * (1.0 + RIM_GAIN * rim * rim), 0.0, 1.0);
}
