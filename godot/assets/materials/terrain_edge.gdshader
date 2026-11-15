// Map edge (prompt 011): the vertical cut around the map, like a diorama
// block. A thin dark sod line at the top, then layers of earth, clay and
// gravel with noise, darker towards the bottom.
shader_type spatial;
// Both sides: the cut is seen from outside only, and this keeps it independent
// of the winding of the generated wall.
render_mode cull_disabled;

uniform sampler2D macro_tex : filter_linear_mipmap, repeat_enable;
uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float bottom = -20.0;

varying vec3 world_pos;
varying float depth_below_top;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	// UV.y: depth below the ground at this edge point, in meters.
	depth_below_top = UV.y;
}

void fragment() {
	// One of x and z is constant on each wall, so their sum runs along it.
	float along = world_pos.x + world_pos.z;
	vec4 noise = texture(macro_tex, vec2(along / 23.0, world_pos.y / 9.0));
	float d = depth_below_top + (noise.r - 0.5) * 0.6;
	vec3 sod = vec3(0.045, 0.04, 0.025);
	vec3 humus = vec3(0.07, 0.05, 0.03);
	vec3 loam = vec3(0.16, 0.11, 0.065);
	vec3 clay = vec3(0.22, 0.17, 0.11);
	vec3 gravel = textureLod(albedo_array, vec3(vec2(along, world_pos.y) / 3.0, 2.0), 0.0).rgb;
	vec3 c = mix(sod, humus, smoothstep(0.05, 0.25, d));
	c = mix(c, loam, smoothstep(0.4, 1.2, d));
	c = mix(c, clay, smoothstep(2.0, 3.5, d + noise.g * 2.0));
	c = mix(c, gravel * 0.8, smoothstep(5.0, 7.0, d + noise.b * 3.0) * 0.7);
	// Thin horizontal bands and some darkening towards the bottom.
	c *= 0.85 + 0.3 * noise.a + 0.08 * sin(world_pos.y * 4.0 + noise.g * 6.0);
	float to_bottom = clamp((world_pos.y - bottom) / 6.0, 0.0, 1.0);
	c *= mix(0.35, 1.0, to_bottom);
	ALBEDO = c;
	ROUGHNESS = 0.95;
}
