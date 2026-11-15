class_name CamoPalette
extends Resource
## Camouflage palette of one faction in one biome (forest, desert, winter).
##
## Files: res://assets/materials/camo/<faction>_<biome>.tres. Applied to a
## unit model with UnitCamo.apply(); the values become instance uniforms of
## res://assets/shaders/unit_camo.gdshader. Colors are sRGB approximations of
## the historical paints named in `note`.

enum Pattern { PLAIN, SOFT, HARD, AMBUSH }

@export var faction := "de"
@export var biome := "forest"
## Base coat (factory color).
@export var base := Color(0.45, 0.43, 0.30)
## Camouflage colors over the base coat.
@export var color_1 := Color(0.30, 0.34, 0.22)
@export var color_2 := Color(0.38, 0.25, 0.17)
@export var pattern: Pattern = Pattern.PLAIN
## Pattern size in meters.
@export_range(0.2, 6.0) var scale := 1.5
## Share of the surface covered by the camouflage colors.
@export_range(0.0, 1.0) var coverage := 0.4
## Whitewash over the paint (winter), 0..1.
@export_range(0.0, 1.0) var winter_wash := 0.0
## Snow on upward faces, 0..1.
@export_range(0.0, 1.0) var snow := 0.0
## Mud thrown up from the ground, 0..1, and its color (sand in the desert).
@export_range(0.0, 1.0) var mud := 0.5
@export var mud_color := Color(0.25, 0.20, 0.14)
## Dust layer, 0..1.
@export_range(0.0, 1.0) var dust := 0.0
## Chipped paint on edges, 0..1.
@export_range(0.0, 1.0) var wear := 0.5
## Where the colors come from.
@export_multiline var note := ""
