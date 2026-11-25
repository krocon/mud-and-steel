// Pattern filtering and relief shared by the unit and environment shaders
// (prompt 203; the relief itself since prompt 186). Procedural patterns are
// drawn per pixel without textures, so nothing filters them for us: a hard
// threshold or a thin line flickers as soon as it gets close to a pixel in
// size. These helpers widen such edges to the pixel and fade a pattern into
// its mean before it gets smaller than a pixel.
// They use screen derivatives: call them where every pixel of a triangle
// runs the same code (not inside a branch on a noise value).

// 1 while a pattern of period `size` (m) covers at least ~5 pixels of `fw`
// meters, 0 below ~2 pixels.
float pattern_fine(float fw, float size) {
	return 1.0 - smoothstep(0.2 * size, 0.45 * size, fw);
}

// step(edge, v), its edge one pixel wide.
float aa_step(float edge, float v) {
	float w = max(fwidth(v) * 0.7, 1e-5);
	return smoothstep(edge - w, edge + w, v);
}

// A line where `d` (a distance, in any unit) is below `half_w`: with a
// growing pixel it gets wider and fainter, so its total darkness stays.
float aa_line(float d, float half_w) {
	float w = fwidth(d);
	return (1.0 - smoothstep(0.0, half_w + w, d)) * half_w / (half_w + w);
}

// Bump mapping from the screen-space derivatives of a height `h` (meters,
// outward) on a surface at view-space position `pos` with view-space normal
// `n` (Mikkelsen, "Bump Mapping Unparametrized Surfaces on the GPU"). Call
// it at the top level of fragment(), so the derivatives are defined.
vec3 surface_bump(vec3 n, vec3 pos, float h) {
	vec3 dpdx = dFdx(pos);
	vec3 dpdy = dFdy(pos);
	float dhx = dFdx(h);
	float dhy = dFdy(h);
	// flat height (most pixels): the normal as it is
	if (dhx == 0.0 && dhy == 0.0) {
		return n;
	}
	vec3 r1 = cross(dpdy, n);
	vec3 r2 = cross(n, dpdx);
	float det = dot(dpdx, r1);
	vec3 grad = sign(det) * (dhx * r1 + dhy * r2);
	vec3 bumped = abs(det) * n - grad;
	// a degenerate pixel: the normal as it is
	return dot(bumped, bumped) > 1e-20 ? normalize(bumped) : n;
}
