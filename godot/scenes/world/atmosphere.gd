extends Node
## Lighting, atmosphere and biome look of the main scene (prompt 015).
##
## `main.gd` creates this node, calls `setup()` with the scene's
## WorldEnvironment, sun, camera rig and terrain, then `set_biome()` and
## `set_quality()`. It builds a fresh Environment from the BiomePreset
## (res://assets/biomes/<id>.tres) and the quality level (graphics_quality.gd)
## and every frame follows the camera distance:
##
## - haze (depth fog) starts and ends at multiples of the camera distance, so
##   the foreground stays clear at every zoom and the upper screen edge lies
##   in light haze;
## - SSAO radius grows with the distance: contact shadows up close, shading of
##   hollows from far away;
## - the snowfall volume (winter) is scaled around the camera focus.
## - tilt-shift depth of field (prompt 057): the ground in front of and behind
##   the focus blurs, so the upper and lower screen edges are soft like a
##   photo of a model. Setting "depth_of_field" (menu, `DOF_MODES`): "subtle"
##   only at close zoom, "miniature" strong, up to middle zoom and with
##   stronger colours; never on quality low.
##
## Shadows: the rig sets `directional_shadow_max_distance` from the camera
## distance (`shadow_distance_factor`), in steps (prompt 090: the next step
## above it, up to 1.6 times as far); the cascade splits here are
## fractions of it and put the cascades where the ground is seen (about 0.8
## to 1.5 camera distances), not near the camera where there is only air.
##
## Cloud shadows (prompt 087): large soft patches drift with the wind of the
## simulation over the map and dim the direct sunlight of every world shader
## (`cloud_shadow.gdshaderinc`, global shader uniforms). Their position runs
## with the game time (`cloud_clock`, set by main.gd: they stand in the pause
## and follow replays and recordings); strength, cover, size and softness
## come from the preset (`WeatherLook.adjust`), the setting "cloud_shadows"
## turns them off. Without `cloud_clock` (map editor, tools) there are none.
##
## Presentation only: nothing here reaches the simulation.

const BiomePreset := preload("res://assets/biomes/biome_preset.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const WeatherLook := preload("res://scenes/world/weather_look.gd")
const Settings := preload("res://scenes/menu/settings.gd")

const BIOMES_DIR := "res://assets/biomes/"
const BIOMES: Array[String] = ["forest", "desert", "winter"]
## Shadow distance as a multiple of the camera distance (the visible ground
## ends at about 1.5 distances; the rest keeps shadows of hills off screen).
const SHADOW_DISTANCE_FACTOR := 1.8
## Cascade splits as fractions of the shadow distance (4 and 2 cascades).
const SPLITS_4 := Vector3(0.45, 0.58, 0.75)
const SPLIT_2 := 0.55
## Tilt-shift depth of field per setting "depth_of_field" ("off" has no
## entry). zoom: full below the first camera distance (m), none above the
## second; sharp band from `near` to `far` camera distances along the view,
## blurred over `transition` distances beyond; amount: blur size; shape and
## blur_quality: RenderingServer DOFBokehShape / DOFBlurQuality; saturation
## and contrast: factors on the biome's, fading out with the blur.
## "subtle" (prompt 057) only softens the screen edges at close zoom.
## "miniature" is the look of a photo of a model railway: a narrow sharp band
## across the screen up to middle zoom, and stronger colours.
const DOF_MODES: Dictionary[String, Dictionary] = {
	"subtle": {
		"zoom": Vector2(30.0, 60.0), "near": 0.86, "far": 1.1, "transition": 0.22, "amount": 0.09,
		# Cheapest blur: the blurred band is ground texture, no bokeh highlights.
		"shape": RenderingServer.DOF_BOKEH_BOX, "blur_quality": RenderingServer.DOF_BLUR_QUALITY_VERY_LOW,
		"saturation": 1.0, "contrast": 1.0,
	},
	"miniature": {
		"zoom": Vector2(150.0, 350.0), "near": 0.95, "far": 1.05, "transition": 0.2, "amount": 0.22,
		"shape": RenderingServer.DOF_BOKEH_HEXAGON, "blur_quality": RenderingServer.DOF_BLUR_QUALITY_MEDIUM,
		"saturation": 1.18, "contrast": 1.05,
	},
}
## Cloud shadows (prompt 087): meters of ground per repeat of the noise
## texture at `cloud_shadow_scale` 1, its size in pixels and the noise
## frequency per pixel (patches of about 100-400 m), and the drift speed in
## m/s of game time: 10 m/s plus 4 s per (m/s) of simulation wind, at most
## 20 (calm but visible; real low clouds move 5-15 m/s, at the time factor 2
## that would be 10-30).
const CLOUD_REPEAT_M := 1600.0
const CLOUD_TEX_SIZE := 256
const CLOUD_FREQUENCY := 0.022
const CLOUD_SPEED := Vector2(10.0, 20.0)
const CLOUD_SPEED_PER_WIND := 4.0
## The second noise layer drifts this much slower and turned by this angle,
## so the patches change their shape as they go.
const CLOUD_SECOND_SPEED := 0.7
const CLOUD_SECOND_TURN_DEG := 25.0
## SSAO radius in meters: distance * factor, clamped.
const SSAO_RADIUS_FACTOR := 0.025
const SSAO_RADIUS_RANGE := Vector2(0.6, 10.0)
## Environment materials whose wind parameters follow the preset.
const WIND_MATERIALS: Array[String] = [
	"res://assets/materials/env/bark.tres",
	"res://assets/materials/env/foliage.tres",
	# the roof snow lies thicker on the lee side (prompt 080)
	"res://assets/materials/env/roof.tres",
	"res://assets/materials/env/metal.tres",
	"res://assets/materials/env/plaster.tres",
	"res://assets/materials/env/meadow_tuft.tres",
	"res://assets/materials/env/tall_grass.tres",
]
const VIGNETTE_SHADER := """
shader_type canvas_item;
uniform float amount = 0.25;
void fragment() {
	vec2 d = (UV - 0.5) * vec2(1.25, 1.0);
	float v = smoothstep(0.35, 0.85, length(d));
	COLOR = vec4(0.0, 0.0, 0.0, v * amount);
}
"""

var preset: BiomePreset
## Shadow blur of the sun (prompt 057: 2 for softer edges, was 1).
var shadow_blur := 2.0
var quality := GraphicsQuality.DEFAULT
## Weather and time of day of the match (prompt 040, `set_weather`): the
## preset is the biome's as it looks then (`WeatherLook.adjust`).
var weather := "clear"
var daytime := "day"
var _biome_preset: BiomePreset

var _world_env: WorldEnvironment
var _env: Environment
var _sun: DirectionalLight3D
var _rig: Node3D
var _terrain: Node
var _vignette: ShaderMaterial
var _snow_root: Node3D
var _snow: GPUParticles3D
var _dof: CameraAttributesPractical
## Mode whose bokeh shape and blur quality are set (they are global).
var _dof_mode := ""
## Game time in seconds and simulation wind (m/s, x east, y south) for the
## cloud shadows: `func() -> Vector3(seconds, wind.x, wind.y)`; empty: none.
var cloud_clock := Callable()
## Values of the last frame (self-test): offsets and shape as set.
var cloud_offset := Vector4.ZERO
var cloud_shape := Vector4.ZERO
## Noise levels (sorted samples of the texture) to turn a cover share into
## a threshold.
static var _cloud_levels := PackedFloat32Array()
static var _cloud_texture: ImageTexture
## `--no-cloud-shadows` (measurements, prompt 087): off whatever the setting.
static var _no_clouds_arg := "--no-cloud-shadows" in OS.get_cmdline_user_args()


func setup(world_env: WorldEnvironment, sun: DirectionalLight3D, camera_rig: Node3D, terrain: Node) -> void:
	# No cloud shadows until a match sets them (a previous match may have).
	RenderingServer.global_shader_parameter_set(&"cloud_shadow_shape", Vector4(0.0, 0.0, 0.6, 0.08))
	_world_env = world_env
	_sun = sun
	_rig = camera_rig
	_terrain = terrain
	if "shadow_distance_factor" in _rig:
		_rig.shadow_distance_factor = SHADOW_DISTANCE_FACTOR

	var layer := CanvasLayer.new()
	layer.name = "Vignette"
	layer.layer = 0
	var rect := ColorRect.new()
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var shader := Shader.new()
	shader.code = VIGNETTE_SHADER
	_vignette = ShaderMaterial.new()
	_vignette.shader = shader
	rect.material = _vignette
	layer.add_child(rect)
	add_child(layer)


## The biome named by `--biome=<id>`, else `map_biome` (from map.toml), else
## forest.
static func biome_from(map_biome: String) -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--biome="):
			var id := arg.trim_prefix("--biome=")
			if id in BIOMES:
				return id
			push_warning("unknown biome '%s', using the map's" % id)
	return map_biome if map_biome in BIOMES else "forest"


static func load_preset(id: String) -> BiomePreset:
	var path := BIOMES_DIR + id + ".tres"
	if ResourceLoader.exists(path):
		return load(path) as BiomePreset
	push_warning("no biome preset %s" % path)
	return BiomePreset.new()


func set_biome(id: String) -> void:
	_biome_preset = load_preset(id)
	preset = WeatherLook.adjust(_biome_preset, weather, daytime)
	_apply()


## Weather and time of day (prompt 040); applies at once if a biome is set.
func set_weather(weather_name: String, daytime_name: String) -> void:
	weather = weather_name
	daytime = daytime_name
	if _biome_preset:
		preset = WeatherLook.adjust(_biome_preset, weather, daytime)
		_apply()


func set_quality(level: String) -> void:
	quality = level if level in GraphicsQuality.LEVELS else GraphicsQuality.DEFAULT
	_apply()


func _apply() -> void:
	if preset == null or _world_env == null:
		return
	var q := GraphicsQuality.settings(quality)
	_apply_sun(q)
	_env = _build_environment(q)
	_world_env.environment = _env
	_apply_viewport(q)
	_vignette.set_shader_parameter("amount", preset.vignette)
	for path in WIND_MATERIALS:
		var mat := load(path) as ShaderMaterial
		if mat:
			mat.set_shader_parameter("wind_strength", preset.wind_strength)
			mat.set_shader_parameter("wind_direction", preset.wind_direction)
			mat.set_shader_parameter("wind_speed", preset.wind_speed)
			mat.set_shader_parameter("tree_sway", preset.tree_sway)
			mat.set_shader_parameter("leaf_flutter", preset.leaf_flutter)
	if _terrain and _terrain.has_method("apply_biome"):
		_terrain.apply_biome(preset)
	_build_snow(int(q.snow_particles) if preset.snowfall > 0.0 else 0)
	_follow_camera()


func _apply_sun(q: Dictionary) -> void:
	var el := deg_to_rad(preset.sun_elevation_deg)
	var az := deg_to_rad(preset.sun_azimuth_deg)
	# Direction towards the sun: bearing 0 = north (-Z), 90 = east (+X).
	var to_sun := Vector3(sin(az) * cos(el), sin(el), -cos(az) * cos(el))
	_sun.basis = Basis.looking_at(-to_sun, Vector3.UP)
	_sun.light_color = preset.sun_color
	_sun.light_energy = preset.sun_energy
	_sun.light_angular_distance = 0.0
	_sun.shadow_enabled = true
	_sun.shadow_opacity = preset.shadow_opacity
	# Softer shadow edges (prompt 057): the PCF kernel of the soft filters
	# grows with the blur; hard shadows (low) stay hard.
	_sun.shadow_blur = shadow_blur
	_sun.shadow_bias = 0.05
	_sun.shadow_normal_bias = 1.0
	_sun.directional_shadow_blend_splits = int(q.cascades) == 4
	_sun.directional_shadow_fade_start = 0.9
	if int(q.cascades) == 4:
		_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
		_sun.directional_shadow_split_1 = SPLITS_4.x
		_sun.directional_shadow_split_2 = SPLITS_4.y
		_sun.directional_shadow_split_3 = SPLITS_4.z
	else:
		_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
		_sun.directional_shadow_split_1 = SPLIT_2
	RenderingServer.directional_shadow_atlas_set_size(int(q.shadow_size), true)
	RenderingServer.directional_soft_shadow_filter_set_quality(int(q.shadow_filter))


func _build_environment(q: Dictionary) -> Environment:
	var e := Environment.new()
	var sky_mat := ProceduralSkyMaterial.new()
	sky_mat.sky_top_color = preset.sky_top
	sky_mat.sky_horizon_color = preset.sky_horizon
	sky_mat.ground_horizon_color = preset.ground_horizon
	sky_mat.ground_bottom_color = preset.ground_bottom
	sky_mat.sun_angle_max = 8.0
	var sky := Sky.new()
	sky.sky_material = sky_mat
	e.background_mode = Environment.BG_SKY
	e.sky = sky
	e.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	e.ambient_light_sky_contribution = preset.ambient_sky_contribution
	e.ambient_light_color = preset.sky_horizon
	e.ambient_light_energy = preset.ambient_energy
	e.reflected_light_source = Environment.REFLECTION_SOURCE_SKY

	e.tonemap_mode = Environment.TONE_MAPPER_AGX
	e.tonemap_exposure = preset.exposure
	e.tonemap_white = preset.white

	e.adjustment_enabled = true
	e.adjustment_contrast = preset.contrast
	e.adjustment_saturation = preset.saturation
	if preset.color_correction:
		var tex := GradientTexture1D.new()
		tex.gradient = preset.color_correction
		tex.width = 256
		e.adjustment_color_correction = tex

	e.fog_enabled = preset.haze_amount > 0.0
	e.fog_mode = Environment.FOG_MODE_DEPTH
	e.fog_density = preset.haze_amount
	e.fog_depth_curve = preset.haze_curve
	e.fog_light_color = preset.haze_color
	e.fog_light_energy = 1.0
	e.fog_sun_scatter = 0.08
	e.fog_aerial_perspective = preset.aerial_perspective
	e.fog_sky_affect = 0.4

	e.ssao_enabled = int(q.ssao) >= 0
	e.ssao_intensity = 1.6
	e.ssao_power = 1.4
	e.ssao_detail = 0.6
	e.ssao_horizon = 0.06
	e.ssao_sharpness = 0.98
	e.ssao_light_affect = 0.15
	e.ssao_ao_channel_affect = 0.3
	if e.ssao_enabled:
		RenderingServer.environment_set_ssao_quality(int(q.ssao), bool(q.ssao_half), 0.5, 2, 50.0, 300.0)
	e.ssil_enabled = bool(q.ssil)

	e.glow_enabled = bool(q.glow)
	e.glow_intensity = 0.35
	e.glow_strength = 0.9
	e.glow_bloom = 0.0
	e.glow_hdr_threshold = 1.1
	e.glow_blend_mode = Environment.GLOW_BLEND_MODE_SOFTLIGHT
	return e


func _apply_viewport(q: Dictionary) -> void:
	var vp := get_viewport()
	vp.msaa_3d = int(q.msaa) as Viewport.MSAA
	vp.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA if bool(q.fxaa) else Viewport.SCREEN_SPACE_AA_DISABLED


func _process(_delta: float) -> void:
	_follow_camera()
	_update_clouds()


## Sets the cloud shadow uniforms for this frame (prompt 087).
func _update_clouds() -> void:
	var strength := 0.0
	if preset != null and cloud_clock.is_valid() and Settings.get_value("cloud_shadows") \
			and not _no_clouds_arg:
		strength = preset.cloud_shadow_strength
	if strength <= 0.0:
		if cloud_shape.y != 0.0:
			cloud_shape = Vector4.ZERO
			RenderingServer.global_shader_parameter_set(&"cloud_shadow_shape", Vector4(0.0, 0.0, 0.6, 0.08))
		return
	_make_cloud_texture()
	var clock: Vector3 = cloud_clock.call()
	var wind := Vector2(clock.y, clock.z)
	var dir := wind.normalized() if wind.length() > 0.01 else preset.wind_direction.normalized()
	var speed := clampf(CLOUD_SPEED.x + wind.length() * CLOUD_SPEED_PER_WIND, CLOUD_SPEED.x, CLOUD_SPEED.y)
	var repeat := CLOUD_REPEAT_M * preset.cloud_shadow_scale
	var t := clock.x
	# The pattern moves downwind: its texture coordinate moves upwind.
	var first := -dir * speed * t / repeat
	var second := -dir.rotated(deg_to_rad(CLOUD_SECOND_TURN_DEG)) * speed * CLOUD_SECOND_SPEED * t * 1.7 / repeat
	cloud_offset = Vector4(fposmod(first.x, 1.0), fposmod(first.y, 1.0), fposmod(second.x, 1.0), fposmod(second.y, 1.0))
	cloud_shape = Vector4(1.0 / repeat, strength, cloud_threshold(preset.cloud_shadow_cover), preset.cloud_shadow_softness)
	RenderingServer.global_shader_parameter_set(&"cloud_shadow_offset", cloud_offset)
	RenderingServer.global_shader_parameter_set(&"cloud_shadow_shape", cloud_shape)


## Noise level above which a share `cover` of the ground lies in cloud
## shadow (the two layers mixed as in the shader, from the texture).
static func cloud_threshold(cover: float) -> float:
	_make_cloud_texture()
	var n := _cloud_levels.size()
	return _cloud_levels[clampi(int((1.0 - clampf(cover, 0.0, 1.0)) * (n - 1)), 0, n - 1)]


## The tileable noise of the cloud shadows, made once per run (the same
## every time: fixed seed); also measures its levels for `cloud_threshold`.
static func _make_cloud_texture() -> void:
	if _cloud_texture != null:
		return
	var noise := FastNoiseLite.new()
	noise.seed = 87
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = CLOUD_FREQUENCY
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 4
	var image := noise.get_seamless_image(CLOUD_TEX_SIZE, CLOUD_TEX_SIZE, false, false, 0.1, true)
	image.convert(Image.FORMAT_L8)
	image.generate_mipmaps()
	_cloud_texture = ImageTexture.create_from_image(image)
	RenderingServer.global_shader_parameter_set(&"cloud_shadow_tex", _cloud_texture)
	# Levels of the mix the shader samples (a: uv, b: uv × 1.7), every
	# second pixel.
	var levels := PackedFloat32Array()
	var size := CLOUD_TEX_SIZE
	for y in range(0, size, 2):
		for x in range(0, size, 2):
			var a := image.get_pixel(x, y).r
			var bx := int(fposmod(x * 1.7 + 0.37 * size, size))
			var by := int(fposmod(y * 1.7 + 0.61 * size, size))
			var b := image.get_pixel(bx, by).r
			levels.append(a * 0.62 + b * 0.38)
	levels.sort()
	_cloud_levels = levels


## Zoom-dependent values: haze range, SSAO radius, snow volume.
func _follow_camera() -> void:
	if _env == null or _rig == null:
		return
	var d: float = _rig.distance
	_env.fog_depth_begin = d * preset.haze_begin
	_env.fog_depth_end = d * preset.haze_end
	_env.ssao_radius = clampf(d * SSAO_RADIUS_FACTOR, SSAO_RADIUS_RANGE.x, SSAO_RADIUS_RANGE.y)
	if _snow_root:
		_snow_root.global_position = _rig.focus
		_snow_root.scale = Vector3.ONE * clampf(d / 60.0, 0.4, 10.0)
	_follow_dof(d)


## Tilt-shift depth of field around the focus distance (`DOF_MODES`), with
## the mode's stronger colours as far as the blur is on.
func _follow_dof(d: float) -> void:
	var camera: Camera3D = _rig.get("camera")
	if camera == null:
		return
	var mode_name: String = Settings.get_value("depth_of_field")
	var mode: Dictionary = DOF_MODES.get(mode_name, {})
	var amount := 0.0
	if not mode.is_empty() and quality != "low":
		var zoom: Vector2 = mode.zoom
		amount = 1.0 - smoothstep(zoom.x, zoom.y, d)
	_env.adjustment_saturation = preset.saturation * lerpf(1.0, float(mode.get("saturation", 1.0)), amount)
	_env.adjustment_contrast = preset.contrast * lerpf(1.0, float(mode.get("contrast", 1.0)), amount)
	if amount <= 0.0:
		if camera.attributes == _dof and _dof != null:
			camera.attributes = null
		return
	if _dof == null:
		_dof = CameraAttributesPractical.new()
		_dof.dof_blur_near_enabled = true
		_dof.dof_blur_far_enabled = true
	if _dof_mode != mode_name:
		_dof_mode = mode_name
		RenderingServer.camera_attributes_set_dof_blur_bokeh_shape(int(mode.shape) as RenderingServer.DOFBokehShape)
		RenderingServer.camera_attributes_set_dof_blur_quality(int(mode.blur_quality) as RenderingServer.DOFBlurQuality, false)
	if camera.attributes != _dof:
		camera.attributes = _dof
	var transition := d * float(mode.transition)
	_dof.dof_blur_amount = float(mode.amount) * amount
	_dof.dof_blur_near_distance = d * float(mode.near)
	_dof.dof_blur_near_transition = transition
	_dof.dof_blur_far_distance = d * float(mode.far)
	_dof.dof_blur_far_transition = transition


## Snowfall: a box of falling flakes around the camera focus, in local
## coordinates of a node that is scaled with the camera distance, so the
## flakes stay visible at every zoom.
func _build_snow(amount: int) -> void:
	if _snow_root:
		_snow_root.queue_free()
		_snow_root = null
		_snow = null
	if amount <= 0:
		return
	_snow_root = Node3D.new()
	_snow_root.name = "Snowfall"
	add_child(_snow_root)
	_snow = GPUParticles3D.new()
	_snow.amount = int(amount * preset.snowfall)
	_snow.lifetime = 12.0
	_snow.preprocess = 12.0
	_snow.local_coords = true
	_snow.position = Vector3(0, 32, 0)
	_snow.visibility_aabb = AABB(Vector3(-70, -60, -70), Vector3(140, 100, 140))
	_snow.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(60, 2, 60)
	pm.direction = Vector3(0, -1, 0)
	pm.spread = 15.0
	pm.initial_velocity_min = 2.0
	pm.initial_velocity_max = 3.0
	pm.gravity = Vector3(preset.wind_direction.x, -0.6, preset.wind_direction.y) * 0.8
	pm.turbulence_enabled = true
	pm.turbulence_noise_strength = 1.5
	pm.turbulence_noise_scale = 6.0
	pm.scale_min = 0.6
	pm.scale_max = 1.4
	_snow.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(0.09, 0.09)
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(1, 1, 1, 0.85)
	quad.material = mat
	_snow.draw_pass_1 = quad
	_snow_root.add_child(_snow)
