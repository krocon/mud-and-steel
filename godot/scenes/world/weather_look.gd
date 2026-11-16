extends Node
## Weather and time of day in the picture (prompt 040).
##
## The simulation decides the weather of a match (`SimHost.weather_info()`:
## weather, time of day, soft ground, wind); this node only draws it:
##
## - `adjust()` turns the biome preset (prompt 015) into the look of the
##   weather and time of day: sun height, direction, color and strength,
##   sky and ambient light, haze (fog), grading, snowfall and wind. Used by
##   `atmosphere.gd` (`set_weather`). Clear weather by day returns the
##   preset unchanged.
## - Rain: falling streaks in a box around the camera focus, scaled with the
##   camera distance like the snowfall of `atmosphere.gd`.
## - Wet ground: the terrain shader's `wetness` follows the softness of the
##   ground (rain soaks it over the match).
## - Ambience (prompt 030): its `rain` control.
##
## Presentation only: nothing here reaches the simulation.

const BiomePreset := preload("res://assets/biomes/biome_preset.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

## Rain streaks per quality level (the snow budget of graphics_quality.gd
## times this).
const RAIN_PER_SNOW := 1.5
## Seconds between reads of the ground state.
const POLL_INTERVAL := 1.0

var main: Node3D
## The weather of the match (names as in data/weather.toml).
var weather := "clear"
var daytime := "day"
## Ground softness 0..100 and its kind (dry, wet, snow).
var ground_pct := 0
var ground := "dry"
## Current wetness of the terrain shader (0..1).
var wetness := 0.0
## Rain particles (null without rain).
var rain: GPUParticles3D

var _rain_root: Node3D
var _poll := 0.0


## The preset `base` as it looks in `weather` at `daytime`. Returns `base`
## itself for clear weather by day.
static func adjust(base: BiomePreset, weather_name: String, daytime_name: String) -> BiomePreset:
	if base == null or (weather_name in ["", "clear"] and daytime_name in ["", "day"]):
		return base
	var p: BiomePreset = base.duplicate()
	match weather_name:
		"overcast":
			_cloud_cover(p, 0.65)
		"rain":
			_cloud_cover(p, 0.85)
			p.haze_amount = maxf(p.haze_amount, 0.55)
			p.haze_begin *= 0.7
			p.haze_end *= 0.8
			p.wind_strength *= 2.0
			p.wind_speed *= 1.5
		"snow":
			_cloud_cover(p, 0.7)
			p.snowfall = maxf(p.snowfall, 0.9)
			p.snow_cover = maxf(p.snow_cover, 0.85)
			p.haze_amount = maxf(p.haze_amount, 0.5)
			p.haze_color = p.haze_color.lerp(Color(0.86, 0.88, 0.9), 0.6)
			p.wind_strength *= 1.5
		"fog":
			_cloud_cover(p, 0.6)
			p.haze_amount = 0.97
			p.haze_begin = 0.12
			p.haze_end = 1.5
			p.haze_curve = 1.0
			p.aerial_perspective = 0.1
			p.haze_color = p.haze_color.lerp(Color(0.72, 0.74, 0.75), 0.7)
			p.wind_strength *= 0.3
	match daytime_name:
		"morning":
			_low_sun(p, 105.0, Color(1.0, 0.78, 0.56), Color(0.98, 0.78, 0.62))
		"evening":
			_low_sun(p, 255.0, Color(1.0, 0.62, 0.38), Color(0.95, 0.6, 0.45))
		"night":
			# Moonlight: weak, cold, from high up; dark blue sky and haze. A
			# game night: darker and bluer, but the ground stays readable.
			p.sun_elevation_deg = 38.0
			p.sun_azimuth_deg = 200.0
			p.sun_color = Color(0.62, 0.72, 1.0)
			p.sun_energy *= 0.35
			p.shadow_opacity *= 0.7
			var night := Color(0.10, 0.13, 0.22)
			p.sky_top = p.sky_top.lerp(Color(0.03, 0.05, 0.12), 0.85)
			p.sky_horizon = p.sky_horizon.lerp(night, 0.75)
			p.ground_horizon = p.ground_horizon.lerp(night, 0.75)
			p.ground_bottom = p.ground_bottom.lerp(Color(0.02, 0.02, 0.04), 0.8)
			p.haze_color = p.haze_color.lerp(night, 0.75)
			p.ambient_energy *= 0.75
			p.exposure *= 1.5
			p.saturation *= 0.6
	return p


## Clouds: weaker sun, softer shadows, grey sky, flatter colors.
static func _cloud_cover(p: BiomePreset, amount: float) -> void:
	var grey := Color(0.55, 0.57, 0.6)
	p.sun_energy *= lerpf(1.0, 0.3, amount)
	p.shadow_opacity *= lerpf(1.0, 0.35, amount)
	p.sky_top = p.sky_top.lerp(grey, amount)
	p.sky_horizon = p.sky_horizon.lerp(grey.lightened(0.15), amount)
	p.haze_color = p.haze_color.lerp(grey.lightened(0.1), amount)
	p.haze_amount = maxf(p.haze_amount, lerpf(p.haze_amount, 0.5, amount))
	p.ambient_energy *= lerpf(1.0, 1.15, amount)
	p.saturation *= lerpf(1.0, 0.75, amount)
	p.contrast *= lerpf(1.0, 0.94, amount)


## Morning or evening: low sun from the east or west, warm light and sky.
static func _low_sun(p: BiomePreset, azimuth: float, light: Color, sky: Color) -> void:
	p.sun_elevation_deg = 11.0
	p.sun_azimuth_deg = azimuth
	p.sun_color = p.sun_color * light
	p.sun_energy *= 0.8
	p.sky_horizon = p.sky_horizon.lerp(sky, 0.5)
	p.haze_color = p.haze_color.lerp(sky, 0.45)
	p.haze_amount = maxf(p.haze_amount, 0.42)
	p.ambient_energy *= 0.8


func setup(main_node: Node3D) -> void:
	main = main_node
	var info: Dictionary = main.sim_host.weather_info()
	weather = info.get("weather", "clear")
	daytime = info.get("daytime", "day")
	_read_ground(info)
	_apply_wetness(true)
	_build_rain()


func _process(delta: float) -> void:
	if main == null:
		return
	_poll -= delta
	if _poll <= 0.0:
		_poll = POLL_INTERVAL
		_read_ground(main.sim_host.weather_info())
	_apply_wetness(false)
	if _rain_root and main.camera_rig:
		var rig: Node3D = main.camera_rig
		_rain_root.global_position = rig.focus
		_rain_root.scale = Vector3.ONE * clampf(rig.distance / 60.0, 0.4, 10.0)


func _read_ground(info: Dictionary) -> void:
	if info.is_empty():
		return
	ground = info.get("ground", "dry")
	ground_pct = int(info.get("ground_pct", 0))


## Terrain wetness: rain soaks the look over the match (half wet at once,
## more as the ground softens); fog leaves a damp sheen. Eased towards the
## target so a ground step does not jump.
func _apply_wetness(at_once: bool) -> void:
	var target := 0.0
	if ground == "wet":
		target = 0.35 + 0.65 * ground_pct / 100.0 if weather == "rain" else 0.25 * ground_pct / 100.0
	wetness = target if at_once else move_toward(wetness, target, 0.002)
	if main.terrain and main.terrain.has_method("set_wetness"):
		main.terrain.set_wetness(wetness)
	var sound: Node = main.get("battle_sound")
	if sound and "ambience" in sound and sound.ambience:
		sound.ambience.rain = 1.0 if weather == "rain" else 0.0


func _build_rain() -> void:
	if weather != "rain":
		return
	var q := GraphicsQuality.settings(main.atmosphere.quality if main.atmosphere else GraphicsQuality.DEFAULT)
	var amount := int(float(q.snow_particles) * RAIN_PER_SNOW)
	if amount <= 0:
		return
	_rain_root = Node3D.new()
	_rain_root.name = "Rainfall"
	add_child(_rain_root)
	rain = GPUParticles3D.new()
	rain.amount = amount
	rain.lifetime = 2.2
	rain.preprocess = 2.2
	rain.local_coords = true
	rain.position = Vector3(0, 30, 0)
	rain.visibility_aabb = AABB(Vector3(-70, -60, -70), Vector3(140, 100, 140))
	rain.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var wind: Vector2 = main.sim_host.weather_info().get("wind", Vector2.ZERO)
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(60, 2, 60)
	pm.direction = Vector3(wind.x * 0.05, -1, wind.y * 0.05).normalized()
	pm.spread = 3.0
	pm.initial_velocity_min = 16.0
	pm.initial_velocity_max = 20.0
	pm.gravity = Vector3(0, -4.0, 0)
	pm.scale_min = 0.7
	pm.scale_max = 1.2
	rain.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(0.018, 0.6)
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	mat.billboard_keep_scale = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(0.75, 0.8, 0.88, 0.22)
	# Streaks right in front of the camera would be bars: fade them in.
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_ALPHA
	mat.distance_fade_min_distance = 3.0
	mat.distance_fade_max_distance = 12.0
	quad.material = mat
	rain.draw_pass_1 = quad
	_rain_root.add_child(rain)
