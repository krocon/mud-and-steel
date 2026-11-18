extends Resource
## Look of one biome (prompt 015): sun, sky, haze, color grading, ground
## colors, the camouflage and environment looks of models, wind and weather.
##
## Files: res://assets/biomes/<id>.tres (forest, desert, winter). The map
## names its biome in map.toml (`SimHost.map_biome()`); `-- --biome=<id>`
## overrides it. Applied by res://scenes/world/atmosphere.gd. Presentation
## only: nothing here reaches the simulation.

## Terrain material layers (see terrain.gdshader / gen_textures.rs).
const LAYER_COUNT := 10

@export var id := "forest"
## Palette key for UnitCamo.palette(faction, camo_biome).
@export var camo_biome := "forest"
## Biome key for EnvLook.apply(model, env_biome).
@export var env_biome := "forest"

@export_group("Sun")
## Elevation above the horizon in degrees.
@export_range(5.0, 85.0) var sun_elevation_deg := 42.0
## Compass bearing the sunlight comes from (0 = north, 90 = east).
@export_range(-180.0, 180.0) var sun_azimuth_deg := 150.0
@export var sun_color := Color(1.0, 0.95, 0.86)
@export_range(0.0, 8.0) var sun_energy := 2.0
## Shadow opacity: below 1 lets some sky light into the shadows.
@export_range(0.0, 1.0) var shadow_opacity := 1.0

@export_group("Sky and ambient light")
@export var sky_top := Color(0.30, 0.47, 0.72)
@export var sky_horizon := Color(0.68, 0.74, 0.80)
@export var ground_horizon := Color(0.50, 0.50, 0.46)
@export var ground_bottom := Color(0.22, 0.22, 0.18)
## Light from the sky on everything (ambient), relative to the sky colors.
@export_range(0.0, 4.0) var ambient_energy := 0.9
## Share of sky (vs. a flat color) in the ambient light.
@export_range(0.0, 1.0) var ambient_sky_contribution := 1.0

@export_group("Haze")
@export var haze_color := Color(0.70, 0.76, 0.82)
## Haze at the far end of the view, 0..1.
@export_range(0.0, 1.0) var haze_amount := 0.35
## Haze starts and is full at these multiples of the camera distance.
@export var haze_begin := 1.0
@export var haze_end := 4.0
@export_range(0.5, 4.0) var haze_curve := 1.6
## How much the haze takes the sky color instead of haze_color.
@export_range(0.0, 1.0) var aerial_perspective := 0.5

@export_group("Grading")
@export_range(0.1, 4.0) var exposure := 1.0
@export_range(1.0, 32.0) var white := 8.0
@export_range(0.5, 1.5) var contrast := 1.05
@export_range(0.0, 2.0) var saturation := 1.0
## Per-channel tone curve after tonemapping (black to white with tints:
## e.g. cool shadows and warm highlights). Null: none.
@export var color_correction: Gradient
## Darkening of the screen corners, 0..1.
@export_range(0.0, 1.0) var vignette := 0.25

@export_group("Ground")
## Texture layer drawn for every terrain material layer (identity: 0..9).
@export var layer_source := PackedInt32Array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
## Color multiplier per terrain material layer (linear).
@export var layer_tint := PackedColorArray([
	Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE,
	Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE,
])
## Snow over all ground on flat parts (roads and tracks keep less), 0..1.
@export_range(0.0, 1.0) var snow_cover := 0.0
## Strength of the dry/lush color variation of grass, 0..1.
@export_range(0.0, 1.0) var grass_variation := 1.0
## Meadow look of the grass layer (prompt 057, terrain_meadow.gdshaderinc):
## palette, blades, soil patches, 0..1. 0 where grass is drawn as sand or snow.
@export_range(0.0, 1.0) var meadow_strength := 1.0
## Flowers in the meadow (daisies, dandelions), 0..1.
@export_range(0.0, 1.0) var flower_density := 1.0
## Top line of the map edge cut (sod, sand crust, snow).
@export var edge_top := Color(0.045, 0.04, 0.025)
## Color multiplier of the soil layers in the map edge cut.
@export var edge_tint := Color.WHITE

@export_group("Weather")
@export_range(0.0, 2.0) var wind_strength := 0.25
@export var wind_direction := Vector2(1.0, 0.3)
@export_range(0.0, 4.0) var wind_speed := 1.2
## Trees in the wind (prompt 086), factors on top of `wind_strength` (which
## the weather scales): bend of the whole tree (at 1 and wind 0.25 the tip
## of a 20 m tree swings 0.2-0.7 m, in gusts up to 1.2 m) and flutter of
## the leaves (0: bare winter trees only swing their branches).
@export_range(0.0, 4.0) var tree_sway := 1.0
@export_range(0.0, 2.0) var leaf_flutter := 1.0
## Shadows of passing clouds (prompt 087, `cloud_shadow.gdshaderinc`) on a
## clear day: share of the direct sunlight a cloud takes in its core, share
## of the ground under clouds, size factor of the patches (1: about
## 100-400 m) and softness of their edge (noise units: 0.07 gives edges of
## some tens of meters). `WeatherLook.adjust` changes them per weather and
## time of day.
@export_range(0.0, 1.0) var cloud_shadow_strength := 0.6
@export_range(0.0, 1.0) var cloud_shadow_cover := 0.3
@export_range(0.25, 4.0) var cloud_shadow_scale := 1.0
@export_range(0.005, 0.3) var cloud_shadow_softness := 0.07
## Falling snow, 0..1 (particles around the camera focus).
@export_range(0.0, 1.0) var snowfall := 0.0

@export_group("Water")
## Water color over shallow and over deep beds (prompt 014, water.gdshader).
@export var water_shallow := Color(0.24, 0.26, 0.14)
@export var water_deep := Color(0.07, 0.10, 0.06)
@export var water_foam := Color(0.85, 0.88, 0.84)
## Meters of water after which 63 % of the green light is absorbed.
@export_range(0.1, 5.0) var water_clarity := 1.2
