// Flat test ground with two grid levels: thin lines every 5 m, thick lines
// every 100 m. Lines are anti-aliased with screen-space derivatives and the
// fine grid fades out when its cells get too small on screen, so the ground
// stays calm from close-up to overview.
shader_type spatial;

uniform vec4 base_color : source_color = vec4(0.42, 0.40, 0.30, 1.0);
uniform vec4 minor_color : source_color = vec4(0.34, 0.33, 0.24, 1.0);
uniform vec4 major_color : source_color = vec4(0.24, 0.23, 0.16, 1.0);
uniform float minor_cell = 5.0;
uniform float minor_width = 0.08;
uniform float major_cell = 100.0;
uniform float major_width = 0.6;

varying vec3 world_position;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

// Coverage (0..1) of grid lines of `width` meters every `cell` meters.
// Lines never get thinner than one pixel; thinner lines fade instead.
float grid(vec2 p, float cell, float width) {
	vec2 pixel = max(fwidth(p), vec2(1e-5));
	vec2 to_line = abs(fract(p / cell + 0.5) - 0.5) * cell;
	vec2 drawn = max(vec2(width), pixel);
	vec2 line = clamp((drawn * 0.5 - to_line) / pixel + 0.5, 0.0, 1.0);
	line *= clamp(vec2(width) / pixel, 0.0, 1.0);
	// Fade the whole grid once a cell covers only a few pixels.
	float fade = smoothstep(3.0, 10.0, cell / max(pixel.x, pixel.y));
	return max(line.x, line.y) * fade;
}

void fragment() {
	vec2 p = world_position.xz;
	vec3 color = mix(base_color.rgb, minor_color.rgb, grid(p, minor_cell, minor_width));
	color = mix(color, major_color.rgb, grid(p, major_cell, major_width));
	ALBEDO = color;
	ROUGHNESS = 1.0;
}
