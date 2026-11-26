// Shared uniforms and billboard vertex shader for battle effect particles (prompt 208).
// Optional defines before including: ATLAS (2x2 cell atlas), CLOUD (no spin),
// STREAK (align quad up-axis to particle velocity).

uniform sampler2D tex : source_color, filter_linear_mipmap, repeat_disable;
uniform float emission = 0.0;
uniform float fire_glow = 0.0;
uniform float soft_distance = 0.0;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
instance uniform vec4 tint : source_color = vec4(1.0);
// Prompt 200: the cloud a puff belongs to: xyz its center, w its radius
// (m); w < 0: a column, only the horizontal offset counts; w = 0: unknown.
instance uniform vec4 cloud_center = vec4(0.0);

varying float v_angle;
varying float v_age;
varying vec2 v_quad_uv;
varying vec3 v_center;

void vertex() {
	// Face the camera like BILLBOARD_PARTICLES: keep the particle's position
	// and scale, spin it around the view axis by its angle (CUSTOM.x).
	mat4 world = mat4(normalize(INV_VIEW_MATRIX[0]), normalize(INV_VIEW_MATRIX[1]),
		normalize(INV_VIEW_MATRIX[2]), MODEL_MATRIX[3]);
	float a = INSTANCE_CUSTOM.x;
	v_quad_uv = UV;
	v_center = MODEL_MATRIX[3].xyz;
#ifdef CLOUD
	// Clouds do not spin (prompt 200).
	a = 0.0;
#endif
#ifdef STREAK
	// Earth streaks (prompt 072): the quad's up axis follows the particle's
	// Y axis, which ParticleProcessMaterial aligns to the velocity.
	vec3 axis = (VIEW_MATRIX * vec4(MODEL_MATRIX[1].xyz, 0.0)).xyz;
	a = atan(axis.x, axis.y);
#endif
	v_angle = a;
	v_age = INSTANCE_CUSTOM.y;
#ifdef ATLAS
	// One of the four cells of a 2 x 2 atlas, chosen by the particle's
	// random animation offset (CUSTOM.z).
	float cell = min(floor(INSTANCE_CUSTOM.z * 4.0), 3.0);
	UV = UV * 0.5 + vec2(mod(cell, 2.0), floor(cell / 2.0)) * 0.5;
#endif
	world = world * mat4(vec4(cos(a), -sin(a), 0.0, 0.0), vec4(sin(a), cos(a), 0.0, 0.0),
		vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_MATRIX = VIEW_MATRIX * world * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
}

float particle_soft_alpha(float alpha, vec2 screen_uv, float vertex_z, mat4 inv_proj) {
	if (soft_distance > 0.0) {
		float depth = textureLod(depth_tex, screen_uv, 0.0).r;
		vec4 scene = inv_proj * vec4(screen_uv * 2.0 - 1.0, depth, 1.0);
		alpha *= clamp((vertex_z - scene.z / scene.w) / soft_distance, 0.0, 1.0);
	}
	return alpha;
}
