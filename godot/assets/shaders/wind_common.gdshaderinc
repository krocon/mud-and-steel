// Gusts shared by the tall grass, the meadow and the trees (prompt 086), so
// one gust runs through the grass and the crowns above it. The including
// shader declares `wind_strength`, `wind_direction` and `wind_speed`
// (atmosphere.gd sets them on every wind material from the biome preset,
// scaled by the weather). Presentation only.

// Gusts: bands across the wind, about 13 m apart, running downwind.
// 0 (calm trough) .. 1 (crest). Depends on the world position and time only:
// a handful of sines, cheap enough per vertex in every shadow cascade.
float wind_gust(vec2 p, float t) {
	vec2 dir = normalize(wind_direction + vec2(1e-4));
	float along = dot(p, dir);
	float across = dot(p, vec2(-dir.y, dir.x));
	float phase = along / 13.0 - t * (0.15 + wind_speed * 0.22)
		+ sin(across / 23.0) * 0.55 + sin(across / 8.7 + along / 31.0) * 0.2;
	float g = 0.5 + 0.5 * sin(phase * 6.2831853);
	// Gusts come and go: a slow change along the band (tens of meters)
	// weakens some of them, so they do not read as regular stripes.
	float come = 0.5 + 0.5 * sin(across / 37.0 + along / 53.0 - t * 0.21) * sin(along / 29.0 + t * 0.13 + 1.3);
	// Narrow crests, wide troughs; stronger with the wind.
	return g * g * g * mix(0.25, 1.0, come) * clamp(wind_strength * 3.2, 0.0, 1.2);
}
