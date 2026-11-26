shader_type canvas_item;

uniform float amount = 0.25;

void fragment() {
	vec2 d = (UV - 0.5) * vec2(1.25, 1.0);
	float v = smoothstep(0.35, 0.85, length(d));
	COLOR = vec4(0.0, 0.0, 0.0, v * amount);
}
