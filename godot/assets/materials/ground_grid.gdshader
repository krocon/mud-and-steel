// Flat ground with a 5 m grid, so movement is easy to judge.
shader_type spatial;

uniform vec4 base_color : source_color = vec4(0.42, 0.40, 0.30, 1.0);
uniform vec4 line_color : source_color = vec4(0.31, 0.30, 0.22, 1.0);
uniform float cell_size = 5.0;
uniform float line_width = 0.06;

varying vec3 world_position;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 to_line = abs(fract(world_position.xz / cell_size + 0.5) - 0.5) * cell_size;
	float on_line = step(min(to_line.x, to_line.y), line_width);
	ALBEDO = mix(base_color.rgb, line_color.rgb, on_line);
	ROUGHNESS = 1.0;
}
