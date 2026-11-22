// Light streak (prompt 199): a shell or tracer in flight drawn as light, not
// as a body. The quad of `light_streak.gd` runs from the tail (local y =
// -0.5) to the head (y = +0.5) of the streak; its x axis carries the real
// width in meters. On screen the streak is a soft, additive line with round
// ends whose width follows the real width but stays between `min_px` and
// `max_px` (pixels at 1080 lines), so it reads from far away and never
// grows into a ball near the camera. A streak shorter than its width is a
// point. `glow` (per instance) scales the brightness; above 1.1 the
// environment's glow picks it up.
shader_type spatial;
render_mode blend_add, unshaded, cull_disabled, depth_draw_never, skip_vertex_transform, fog_disabled, shadows_disabled;

uniform vec4 color : source_color = vec4(1.0, 0.75, 0.35, 1.0);
uniform float energy = 3.0;
uniform float min_px = 1.5;
uniform float max_px = 4.0;
// Brightness of the tail end relative to the head (1: even).
uniform float tail_share = 0.3;
instance uniform float glow = 1.0;

varying float v_across;
varying float v_along;
varying float v_len;
varying float v_r;

void vertex() {
	vec3 head = (MODEL_MATRIX * vec4(0.0, 0.5, 0.0, 1.0)).xyz;
	vec3 tail = (MODEL_MATRIX * vec4(0.0, -0.5, 0.0, 1.0)).xyz;
	float width_m = length(MODEL_MATRIX[0].xyz);
	mat4 view_projection = PROJECTION_MATRIX * VIEW_MATRIX;
	vec4 h = view_projection * vec4(head, 1.0);
	vec4 t = view_projection * vec4(tail, 1.0);
	vec2 half_size = VIEWPORT_SIZE * 0.5;
	vec2 hs = h.xy / max(h.w, 0.001) * half_size;
	vec2 ts = t.xy / max(t.w, 0.001) * half_size;
	vec2 d = hs - ts;
	float len = length(d);
	vec2 dir = len > 0.001 ? d / len : vec2(0.0, 1.0);
	vec2 side = vec2(-dir.y, dir.x);
	float lines = VIEWPORT_SIZE.y / 1080.0;
	float real_px = width_m * PROJECTION_MATRIX[1][1] * half_size.y / max(h.w, 0.001);
	float r = 0.5 * clamp(real_px, min_px * lines, max_px * lines);
	float along = VERTEX.y + 0.5;
	vec4 at = mix(t, h, along);
	vec2 offset = side * VERTEX.x * 2.0 * r + dir * (along * 2.0 - 1.0) * r;
	at.xy += offset / half_size * at.w;
	POSITION = at;
	v_across = VERTEX.x * 2.0;
	v_along = along;
	v_len = len;
	v_r = r;
}

void fragment() {
	// Pixels from the tail point along the streak, and the distance to the
	// core line in units of the half width.
	float s = v_along * (v_len + 2.0 * v_r) - v_r;
	float beyond = s < 0.0 ? -s : max(s - v_len, 0.0);
	float dist = length(vec2(beyond, abs(v_across) * v_r)) / max(v_r, 0.001);
	// Soft round profile: still bright at a pixel center half a pixel off
	// the line when the streak is only two pixels wide.
	float core = clamp(1.0 - dist * dist, 0.0, 1.0);
	float toward_head = v_len > 0.001 ? clamp(s / v_len, 0.0, 1.0) : 1.0;
	ALBEDO = color.rgb * energy * glow * core * mix(tail_share, 1.0, toward_head);
}
