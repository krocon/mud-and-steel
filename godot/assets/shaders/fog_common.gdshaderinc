// Shared fog-of-war mask sampling (prompts 018, 098, 208).

uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;
uniform sampler2D fog_map : filter_linear, repeat_disable;
/// World x/z of the padded mask's corner and its size in meters.
uniform vec2 fog_origin;
uniform vec2 fog_size = vec2(1.0);
/// Texels of the padded mask.
uniform vec2 fog_texels = vec2(1.0);
uniform float edge_softness = 0.15;

void vertex() {
	POSITION = vec4(VERTEX.xy, 1.0, 1.0);
}

vec4 bspline_weights(float t) {
	vec4 n = vec4(1.0, 2.0, 3.0, 4.0) - t;
	vec4 s = n * n * n;
	float x = s.x;
	float y = s.y - 4.0 * s.x;
	float z = s.z - 4.0 * s.y + 6.0 * s.x;
	return vec4(x, y, z, 6.0 - x - y - z) * (1.0 / 6.0);
}

/// Cubic B-spline of the mask from four bilinear taps: a smooth field
/// whose 0.5 line is a round curve through the 8 m cells.
float mask_at(vec2 uv) {
	vec2 p = uv * fog_texels - 0.5;
	vec2 f = fract(p);
	p -= f;
	vec4 wx = bspline_weights(f.x);
	vec4 wy = bspline_weights(f.y);
	vec4 c = p.xxyy + vec2(-0.5, 1.5).xyxy;
	vec4 s = vec4(wx.xz + wx.yw, wy.xz + wy.yw);
	vec4 o = (c + vec4(wx.yw, wy.yw) / s) / fog_texels.xxyy;
	float a = texture(fog_map, o.xz).r;
	float b = texture(fog_map, o.yz).r;
	float d = texture(fog_map, o.xw).r;
	float e = texture(fog_map, o.yw).r;
	float sx = s.x / (s.x + s.y);
	float sy = s.z / (s.z + s.w);
	return mix(mix(e, d, sx), mix(b, a, sx), sy);
}

/// 1 where the local player sees the pixel, 0 under the layer; -1 for sky.
float seen(vec2 screen_uv, mat4 inv_view, mat4 inv_projection) {
	float depth = texture(depth_texture, screen_uv).x;
	if (depth <= 0.0) {
		return -1.0; // sky (reverse z: the far plane is 0)
	}
	vec4 world = inv_view * inv_projection * vec4(screen_uv * 2.0 - 1.0, depth, 1.0);
	vec2 uv = (world.xz / world.w - fog_origin) / fog_size;
	// Surround outside the playable map (prompt 222) always counts as never seen
	// without sampling outside the padded mask.
	if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
		return 0.0;
	}
	return smoothstep(0.5 - edge_softness, 0.5 + edge_softness, mask_at(uv));
}
