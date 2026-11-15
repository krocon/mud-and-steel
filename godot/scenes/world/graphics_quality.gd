extends RefCounted
## Graphics quality levels (prompt 015): low, medium, high.
##
## Chosen with `-- --quality=<level>` (default high) or in the debug menu (F3);
## a settings menu follows with prompt 027. `atmosphere.gd` applies the
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
const SETTINGS: Dictionary[String, Dictionary] = {
	"low": {
		"shadow_size": 2048, "cascades": 2, "shadow_filter": RenderingServer.SHADOW_QUALITY_HARD,
		"ssao": -1, "ssao_half": true, "ssil": false, "glow": false,
		"msaa": Viewport.MSAA_DISABLED, "fxaa": true, "snow_particles": 0,
	},
	"medium": {
		"shadow_size": 4096, "cascades": 4, "shadow_filter": RenderingServer.SHADOW_QUALITY_SOFT_LOW,
		"ssao": RenderingServer.ENV_SSAO_QUALITY_LOW, "ssao_half": true, "ssil": false, "glow": true,
		"msaa": Viewport.MSAA_DISABLED, "fxaa": true, "snow_particles": 6000,
	},
	"high": {
		"shadow_size": 8192, "cascades": 4, "shadow_filter": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
		"ssao": RenderingServer.ENV_SSAO_QUALITY_MEDIUM, "ssao_half": false, "ssil": false, "glow": true,
		"msaa": Viewport.MSAA_4X, "fxaa": false, "snow_particles": 16000,
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
