shader_type canvas_item;

uniform sampler2D ground_map : filter_linear, source_color;
uniform sampler2D fog_map : filter_linear;
uniform bool use_fog = false;
// Map rectangle within the vision grid's texture (UV).
uniform vec2 uv_scale = vec2(1.0);
// Vision grid uv to uv in the padded fog mask (fog_of_war.fog_uv_transform).
uniform vec4 fog_uv = vec4(1.0, 1.0, 0.0, 0.0);
// The main view's layer (fog_of_war.gd): same tint and desaturation.
uniform vec3 dim_tint = vec3(0.6, 0.63, 0.7);
uniform float desaturation = 0.6;

void fragment() {
	vec2 uv = UV * uv_scale;
	vec3 ground = texture(ground_map, uv).rgb;
	vec3 color = ground;
	if (use_fog) {
		// The whole map is drawn; out of sight it is dimmed (prompt 098).
		float seen = smoothstep(0.3, 0.7, texture(fog_map, uv * fog_uv.xy + fog_uv.zw).r);
		vec3 gray = vec3(dot(ground, vec3(0.2126, 0.7152, 0.0722)));
		vec3 dimmed = mix(ground, gray, desaturation) * dim_tint;
		color = mix(dimmed, ground, seen);
	}
	COLOR = vec4(color, 1.0);
}
