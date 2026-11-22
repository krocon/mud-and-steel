extends RefCounted
## Graphics quality levels (prompt 015): low, medium, high.
##
## Chosen in the settings (`menu/settings.gd`, prompt 027), overridden by
## `-- --quality=<level>`, or at run time in the debug menu (F3). `atmosphere.gd` applies the
## level to the sun, the environment and the viewport. Presentation only.

const LEVELS: Array[String] = ["low", "medium", "high"]
const DEFAULT := "high"

## Settings per level.
## shadow_size: directional shadow atlas (pixels per side);
## cascades: PSSM splits (2 or 4); shadow_filter: RenderingServer
## ShadowQuality; ssao: -1 off, else RenderingServer EnvironmentSSAOQuality;
## ssao_half: half resolution; ssil: screen-space indirect light;
## glow; msaa: Viewport.MSAA; fxaa;
## snow_particles: particle count of the snowfall (0 = off).
## Effects (prompt 019, `effects.gd`): fx_particles: particle count factor;
## fx_pools: factor on the pooled emitters per effect kind; fx_lights: light
## flashes at once; fx_decals: scorch marks kept; fx_fires: burning wrecks
## (older ones only smolder); fx_fire_lights: wreck fires with a light;
## fx_soft: soft particles; fx_vehicle_m: camera range of dust trails and
## exhaust smoke in meters.
## Rocket salvo (prompt 076, `rocket_effects.gd`): fx_rocket_trails: smoke
## trails at once (the oldest goes); fx_trail_life: seconds a trail stays;
## fx_trail_width: factor on its width; fx_salvo_smoke: haze over a firing
## position; fx_rocket_lights: firing positions with a light.
## Muzzle blast (prompt 077, `effects.gd`): fx_muzzle_detail: the glowing
## gas cloud and the side blasts of muzzle brakes (off: core, tongue,
## powder smoke, dust and light only).
## Tracks and ground effects (prompt 104): fx_track_texels: side of the track
## raster around the camera (0 = no tracks); fx_track_m: meters per texel;
## fx_ground_fx: mud clods, snow dust, bow spray and wake (off: only the dust
## trail, tinted, and nothing on mud and water).
## Meadow relief (prompt 191, `terrain_meadow.gdshaderinc`): ground_relief:
## grass clumps beyond the blades; ground_shadows: their fake shadows
## (about 3 and 1 ms per frame on an M1 Max at the middle zoom).
## Shells in flight (prompt 199, `artillery_view.gd`): fx_shell_trails: the
## faint vapour trail behind a shell's light.
const SETTINGS: Dictionary[String, Dictionary] = {
	"low": {
		"shadow_size": 2048, "cascades": 2, "shadow_filter": RenderingServer.SHADOW_QUALITY_HARD,
		"ssao": -1, "ssao_half": true, "ssil": false, "glow": false,
		"msaa": Viewport.MSAA_DISABLED, "fxaa": true, "snow_particles": 0,
		"fx_particles": 0.4, "fx_pools": 0.35, "fx_lights": 2, "fx_decals": 32, "fx_fires": 4,
		"fx_fire_lights": 0, "fx_soft": false, "fx_vehicle_m": 150.0,
		"fx_rocket_trails": 24, "fx_trail_life": 4.5, "fx_trail_width": 0.7, "fx_salvo_smoke": false,
		"fx_rocket_lights": 1, "fx_muzzle_detail": false, "fx_shell_trails": false,
		"fx_track_texels": 1024, "fx_track_m": 0.5, "fx_ground_fx": false,
		"ground_relief": false, "ground_shadows": false,
	},
	"medium": {
		"shadow_size": 4096, "cascades": 4, "shadow_filter": RenderingServer.SHADOW_QUALITY_SOFT_LOW,
		"ssao": RenderingServer.ENV_SSAO_QUALITY_LOW, "ssao_half": true, "ssil": false, "glow": true,
		"msaa": Viewport.MSAA_DISABLED, "fxaa": true, "snow_particles": 6000,
		"fx_particles": 0.7, "fx_pools": 0.65, "fx_lights": 6, "fx_decals": 96, "fx_fires": 10,
		"fx_fire_lights": 3, "fx_soft": false, "fx_vehicle_m": 250.0,
		"fx_rocket_trails": 48, "fx_trail_life": 7.0, "fx_trail_width": 1.0, "fx_salvo_smoke": true,
		"fx_rocket_lights": 2, "fx_muzzle_detail": true, "fx_shell_trails": true,
		"fx_track_texels": 1536, "fx_track_m": 0.3, "fx_ground_fx": true,
		"ground_relief": true, "ground_shadows": false,
	},
	"high": {
		"shadow_size": 8192, "cascades": 4, "shadow_filter": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
		"ssao": RenderingServer.ENV_SSAO_QUALITY_MEDIUM, "ssao_half": false, "ssil": false, "glow": true,
		"msaa": Viewport.MSAA_4X, "fxaa": false, "snow_particles": 16000,
		"fx_particles": 1.0, "fx_pools": 1.0, "fx_lights": 12, "fx_decals": 192, "fx_fires": 24,
		"fx_fire_lights": 8, "fx_soft": true, "fx_vehicle_m": 400.0,
		"fx_rocket_trails": 64, "fx_trail_life": 9.0, "fx_trail_width": 1.0, "fx_salvo_smoke": true,
		"fx_rocket_lights": 2, "fx_muzzle_detail": true, "fx_shell_trails": true,
		"fx_track_texels": 2048, "fx_track_m": 0.25, "fx_ground_fx": true,
		"ground_relief": true, "ground_shadows": true,
	},
}


## The level named by `--quality=<level>`, or DEFAULT.
static func from_cmdline() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--quality="):
			var level := arg.trim_prefix("--quality=")
			if level in LEVELS:
				return level
			push_warning("unknown quality '%s', using %s" % [level, DEFAULT])
	return DEFAULT


static func settings(level: String) -> Dictionary:
	return SETTINGS.get(level, SETTINGS[DEFAULT])
