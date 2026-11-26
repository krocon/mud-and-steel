// One stamp of a vehicle track (prompts 104, 239), drawn by
// scenes/world/ground_tracks.gd as an instance of a 2D MultiMesh into the
// track raster around the camera (additive, cleared on every redraw).
//
// The quad covers the vehicle's width (y of UV, across) and the driven
// segment (x of UV, along). Instance data:
//   COLOR: r band strength (the two ruts), g churn (turning: whole patch),
//          b wear (the whole width, worn path after many passes),
//          a segment length in meters for tracked vehicles (grouser ribs),
//            or 0.0 for wheeled vehicles (smooth tire ruts).
//   INSTANCE_CUSTOM: x birth (game seconds), y lifetime (s), z cumulative
//          distance along the path (m) for continuous grouser phase,
//          w half width of a band (band center = 1 - w).
// Output: R ruts, G churned, B worn, each faded out over the stamp's life.
shader_type canvas_item;
render_mode blend_add, unshaded;

// Game seconds now (set by ground_tracks.gd on every redraw).
uniform float now = 0.0;
// Share of the lifetime that a stamp stays at full strength.
uniform float hold_share = 0.4;
// Track grouser rib spacing in meters (>= 2.2 texels at 0.25 m/texel -> no alias).
const float GROUSER_PERIOD_M = 0.58;

varying vec4 v_data;
varying vec4 v_strength;

void vertex() {
	v_data = INSTANCE_CUSTOM;
	v_strength = COLOR;
}

void fragment() {
	float age = now - v_data.x;
	float life = max(v_data.y, 0.001);
	float fade = 1.0 - smoothstep(life * hold_share, life, age);
	if (age < -1.0 || fade <= 0.0) {
		discard;
	}
	float across = abs(UV.y - 0.5) * 2.0;
	// Ruts: crisp-edged bands with slight softness at the rim.
	float band_w = max(v_data.w, 0.01);
	float band_center = 1.0 - band_w;
	float d = abs(across - band_center);
	float rut = 1.0 - smoothstep(band_w * 0.55, band_w * 1.15, d);
	// Prompt 239: periodic grouser/cleat ribs on tracked vehicles (smooth on wheels).
	if (v_strength.a > 0.01) {
		float along_m = v_data.z + UV.x * v_strength.a;
		float cleat = 0.80 + 0.20 * sin(along_m * (6.2831853 / GROUSER_PERIOD_M));
		rut *= cleat;
	}
	// Churned patch: an ellipse over the quad.
	float r = length(UV * 2.0 - 1.0);
	float churn = 1.0 - smoothstep(0.55, 1.0, r);
	// Worn strip: the whole width with soft edges.
	float wear = 1.0 - smoothstep(0.75, 1.0, across);
	COLOR = vec4(vec3(rut * v_strength.r, churn * v_strength.g, wear * v_strength.b) * fade, 1.0);
}
