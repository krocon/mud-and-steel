// Debug look of the height field until the real terrain (prompt 011):
// terrain type as vertex color, grid lines every 10 m / 100 m and contour
// lines every 1 m / 5 m of height (at x.5 m). Lines are anti-aliased with screen-space
// derivatives and fade when they get too dense on screen.
shader_type spatial;

uniform vec4 line_color : source_color = vec4(0.1, 0.1, 0.08, 1.0);
uniform vec4 contour_color : source_color = vec4(0.95, 0.9, 0.7, 1.0);

varying vec3 world_position;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

// Coverage (0..1) of lines of `width` meters every `cell` meters along `v`.
float lines(float v, float cell, float width) {
	float pixel = max(fwidth(v), 1e-5);
	float to_line = abs(fract(v / cell + 0.5) - 0.5) * cell;
	float drawn = max(width, pixel);
	float line = clamp((drawn * 0.5 - to_line) / pixel + 0.5, 0.0, 1.0);
	line *= clamp(width / pixel, 0.0, 1.0);
	return line * smoothstep(3.0, 10.0, cell / pixel);
}

void fragment() {
	vec2 p = world_position.xz;
	// Vertex colors come from the 0xRRGGBB values of data/terrain.toml (sRGB).
	vec3 color = pow(COLOR.rgb, vec3(2.2));
	float grid = max(max(lines(p.x, 10.0, 0.1), lines(p.y, 10.0, 0.1)) * 0.25,
		max(lines(p.x, 100.0, 0.6), lines(p.y, 100.0, 0.6)) * 0.5);
	color = mix(color, line_color.rgb, grid);
	// Contours sit half a meter off the whole meters, so flattened areas at
	// whole-meter heights do not light up, and fade out on flat ground.
	float h = world_position.y + 0.5;
	float grade = length(vec2(dFdx(world_position.y), dFdy(world_position.y)))
		/ max(length(vec2(length(dFdx(p)), length(dFdy(p)))), 1e-5);
	float contour = max(lines(h, 1.0, 0.04) * 0.25, lines(h, 5.0, 0.08) * 0.5);
	contour *= smoothstep(0.003, 0.02, grade);
	color = mix(color, contour_color.rgb, contour);
	ALBEDO = color;
	ROUGHNESS = 1.0;
}
