// Storey-by-storey collapse of a building (prompt 197, building_collapse.gd).
// Presentation only: the simulation has already made the object a ruin.
//
// A height band in world space: the storey (or the roof) from `bottom` to
// `bottom + height` collapses with `progress` 0..1. Its walls tip about
// their foot (the side facing the shot inward, the others outward; per
// side a little different), then the band drops into the storey below,
// shrinks towards the middle and crumbles. Everything above the band fell
// in an earlier stage and is folded into a point (degenerate triangles draw
// nothing, cheaper than discarding). Below the band nothing moves.
// Only the few nodes of a collapsing building set these; every other
// object keeps height 0 and skips the branch.

// x = band bottom (world y), y = band height (m, 0 = off), z = progress,
// w = tip angle (radians; negative: every side falls inward, the roof).
instance uniform vec4 env_collapse = vec4(0.0);
// xy = middle of the building (world xz), zw = direction the shot flew
// (world xz, unit).
instance uniform vec4 env_collapse_at = vec4(0.0);

// `w`: rest position in world space; returns the collapsed one.
vec3 collapse_world(vec3 w) {
	float bottom = env_collapse.x;
	float h = env_collapse.y;
	float p = clamp(env_collapse.z, 0.0, 1.0);
	vec2 c = env_collapse_at.xy;
	if (w.y >= bottom + h + 0.02) {
		return vec3(c.x, bottom - 1.0, c.y);
	}
	if (w.y < bottom) {
		return w;
	}
	float f = clamp((w.y - bottom) / h, 0.0, 1.0);
	vec2 r = w.xz - c;
	float rl = length(r);
	vec2 rn = rl > 1e-3 ? r / rl : vec2(0.0);
	// Tip: the wall turns about its foot.
	float sector = floor((atan(rn.y, rn.x) + 3.14159) / 0.7854);
	float j = hash2(vec2(sector, floor(bottom * 3.0)) + c * 0.013);
	float side = env_collapse.w < 0.0 ? -1.0 : (dot(rn, -env_collapse_at.zw) > 0.25 ? -1.0 : 1.0);
	float tip = abs(env_collapse.w) * smoothstep(0.0, 0.65, p) * (0.6 + 0.6 * j);
	float up = f * h;
	vec3 o = vec3(w.x, bottom, w.z);
	o.xz += rn * side * sin(tip) * up;
	o.y += cos(tip) * up;
	// Drop into the storey below, shrink, crumble.
	float drop = pow(clamp((p - 0.3) / 0.7, 0.0, 1.0), 2.0);
	o.xz = c + (o.xz - c) * (1.0 - 0.14 * drop);
	o.y -= drop * (h + 0.6);
	vec3 k = floor(w * 1.7);
	o += (vec3(hash1(k), hash1(k + vec3(5.1)), hash1(k + vec3(9.7))) - 0.5) * 0.5 * drop;
	return o;
}

// Model-space vertex for vertex(): unchanged unless this node collapses.
vec3 collapse_vertex(vec3 vertex, mat4 model) {
	if (env_collapse.y <= 0.0) {
		return vertex;
	}
	vec3 w = (model * vec4(vertex, 1.0)).xyz;
	return (inverse(model) * vec4(collapse_world(w), 1.0)).xyz;
}
