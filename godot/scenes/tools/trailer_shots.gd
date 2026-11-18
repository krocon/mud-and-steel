extends Node
## Scripted recordings for the trailer (prompt 070), started by `main.gd`
## with `-- --trailer=<shot>`; `just trailer-shots` records every shot.
##
## A shot plays a scene of `data/trailer/<scene>.toml` (a mission script,
## docs/MISSIONS.md, which is no mission of the menu) on its map with a fixed
## seed, runs the simulation ahead to the shot's start without anybody
## watching, then moves the camera along the shot's keys for its length and
## quits. Everything that happens comes from the script's `order` and `fire`
## actions, i.e. normal commands; the simulation is not touched. Several
## shots can show the same scene (same script and seed) from other angles.
##
##     --trailer=<shot>          play the shot (with `--write-movie` and
##                               `--fixed-fps 60`: record it; the line
##                               "trailer: record …" names the first frame)
##     --trailer-stills=<dir>    save the shot's `stills` as PNG files instead
##     --trailer-scout[=<s>]     no camera: run the scene for <s> game seconds
##                               (default: to the shot's end) at high speed and
##                               print who is destroyed when and where
##     --trailer-seed=<n>        scouting: another seed than the shot's
##     --trailer-off=<a,b,..>    analysis only (prompt 068): turn parts of the
##                               picture off, see `OFF_PARTS`
##
## The camera does more than in the game (lower, closer, flat angles, chase
## rides); the camera rig stands still meanwhile and only receives focus and
## distance, which haze and sound follow. A shot with `game_camera` shows
## what a player can see instead (prompt 082): the rig places the camera.
## Presentation only.

const FPS := 60
const SIZE := Vector2i(1920, 1080)
## Folder of the scene scripts below `data/`.
const SCENE_DIR := "trailer"
## Game speed of the run-up (the bridge runs at most 5 ticks per frame).
const RUN_UP_SPEED := 15.0
## The last seconds before the shot run at normal speed, so smoke, dust and
## sound loops are in place when the first frame is written. Effects live in
## real time: what starts during the fast run-up is seen too long (a rocket
## trail of 9 s stands for 135 game seconds). A shot after long-lived effects
## sets `pre_roll_s` to more than their life.
const PRE_ROLL_S := 2.0
## The camera stays this far above the ground below it, in meters.
const CLEARANCE := 0.45
const TICK_HZ := 20
## Bus volumes of a recording in dB (-80: muted).
const BUS_DB: Dictionary[String, float] = {
	"Master": -14.0, "Effects": 0.0, "Ambience": -2.0, "UI": -2.0, "Music": -80.0,
}

## The shots. Per shot:
##   map, scene (file in data/trailer, default: the shot's name), seed,
##   start_s (game seconds before the first frame), seconds (length),
##   pre_roll_s (seconds before `start_s` played at normal speed, default 2),
##   hud (default false: no HUD, no fog of war, no relation tint),
##   weather, daytime (default: the map's), stills (seconds into the shot),
##   select ([owner, catalog id prefix]: units the player has selected),
##   camera: keys in time order, interpolated (Catmull-Rom):
##     t      seconds into the shot
##     at     Vector2 map position (x east, y south) the camera looks at, or
##     unit   [owner, catalog id prefix, index]: that unit (index -1: the
##            centre of all living ones), sorted by unit id
##     lift   meters above the ground the camera looks at (default 1)
##     dist   meters between camera and that point
##     yaw    degrees, 0 looks north, 90 looks west (as the game's camera)
##     heading  true: `yaw` is relative to the unit's driving direction
##     pitch  degrees above the horizon
##     fov    field of view in degrees (default: the game's 30)
##     haze   distance the haze and the sound are set for (default: `dist`
##            for steep views, farther for flat ones)
##   game_camera (default false): the camera of the game. The keys give only
##     what a player controls, `at` or `unit` (the focus on the ground),
##     `dist` and `yaw`; the camera rig places the camera as in a match: pitch
##     from the zoom level (45 to 65 degrees), distance within its limits,
##     field of view, clearance over ground and roofs, planes, shadow range.
##     Shots whose keys carry `pitch`, `lift`, `fov` or `haze` (the measuring
##     pans of prompts 068 to 080) keep the free placement with the game's
##     shadow range.
## Parts `--trailer-off=` can turn off (prompt 068, analysis of flicker, never
## a game setting): meadow tufts, procedural meadow of the ground shader,
## sun shadows, wind of the tufts, depth of field, SSAO, MSAA; "fxaa" and
## "taa" turn that antialiasing on instead; "snowfall" (prompt 080) hides
## the falling snow of a winter map; "clouds" (prompt 087) the cloud
## shadows (they are on otherwise and lie the same for the same scene and
## seed: they follow the game time).
const OFF_PARTS: Array[String] = ["tufts", "ground", "shadows", "wind", "dof", "ssao", "msaa", "fxaa", "taa", "snowfall", "clouds"]
const MeadowTufts := preload("res://scenes/world/meadow_tufts.gd")
const TallGrass := preload("res://scenes/world/tall_grass.gd")
const Settings := preload("res://scenes/menu/settings.gd")

const SHOTS: Dictionary[String, Dictionary] = {
	# --- Scene "column" on farms: march and first contact ---------------------
	"column_pass": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 6.0, "seconds": 8.0,
		"stills": [1.0, 4.0, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(5, 262), "dist": 24.0, "yaw": 196.0, "pitch": 3.0, "lift": 1.2},
			{"t": 8.0, "at": Vector2(3, 258), "dist": 22.0, "yaw": 204.0, "pitch": 3.5, "lift": 1.2},
		],
	},
	"column_high": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 18.0, "seconds": 6.0,
		"stills": [1.0, 4.0],
		"camera": [
			{"t": 0.0, "unit": [0, "de_sdkfz251_1", -1], "dist": 120.0, "yaw": 150.0, "pitch": 48.0},
			{"t": 6.0, "unit": [0, "de_sdkfz251_1", -1], "dist": 95.0, "yaw": 172.0, "pitch": 42.0},
		],
	},
	"column_chase": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 7.0, "seconds": 7.0,
		"stills": [1.0, 3.5, 6.0],
		"camera": [
			{"t": 0.0, "unit": [0, "de_pz4_h", 3], "heading": true, "dist": 13.0, "yaw": 150.0, "pitch": 6.0, "lift": 1.5},
			{"t": 7.0, "unit": [0, "de_pz4_h", 3], "heading": true, "dist": 10.0, "yaw": 112.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"contact_line": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 36.5, "seconds": 8.0,
		"stills": [1.0, 1.9, 3.4, 5.9, 7.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-2, 100), "dist": 30.0, "yaw": -18.0, "pitch": 6.0, "lift": 1.5},
			{"t": 8.0, "at": Vector2(-2, 98), "dist": 24.0, "yaw": -30.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"contact_t34": {
		"map": "farms", "scene": "column", "seed": 7, "daytime": "morning", "start_s": 36.2, "seconds": 6.0,
		"stills": [1.0, 2.2, 3.7, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-22, -9), "dist": 34.0, "yaw": 196.0, "pitch": 9.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(-22, -9), "dist": 24.0, "yaw": 226.0, "pitch": 7.0, "lift": 1.5},
		],
	},
	# --- Scene "bridge" on river_land: the main battle ------------------------
	"bridge_player": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 15.0, "seconds": 8.0, "hud": true, "select": [0, "de_pz4_h"], "stills": [1.0, 4.3, 6.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-12, 2), "dist": 205.0, "yaw": 0.0, "pitch": 56.0, "lift": 0.0},
			{"t": 8.0, "at": Vector2(-4, 2), "dist": 190.0, "yaw": 0.0, "pitch": 55.0, "lift": 0.0},
		],
	},
	"bridge_pak": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 9.5, "seconds": 5.0, "stills": [1.0, 3.0, 3.3, 4.5],
		"camera": [
			{"t": 0.0, "unit": [0, "de_pak40", 1], "dist": 12.0, "yaw": 42.0, "pitch": 9.0, "lift": 1.0},
			{"t": 5.0, "unit": [0, "de_pak40", 1], "dist": 9.5, "yaw": 62.0, "pitch": 7.0, "lift": 1.0},
		],
	},
	"bridge_low": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 15.5, "seconds": 7.0, "stills": [1.0, 3.6, 4.2, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(-62, 2), "dist": 38.0, "yaw": -98.0, "pitch": 6.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(-58, 2), "dist": 30.0, "yaw": -82.0, "pitch": 5.0, "lift": 1.5},
		],
	},
	"bridge_east": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 27.0, "seconds": 7.0, "stills": [1.0, 3.5, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(80, 4), "dist": 34.0, "yaw": -98.0, "pitch": 8.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(80, 4), "dist": 28.0, "yaw": -114.0, "pitch": 6.0, "lift": 1.5},
		],
	},
	"bridge_top": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 14.0, "seconds": 7.0, "stills": [1.0, 5.2, 6.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-5, 2), "dist": 215.0, "yaw": 0.0, "pitch": 74.0, "lift": 0.0},
			{"t": 7.0, "at": Vector2(-5, 2), "dist": 195.0, "yaw": 12.0, "pitch": 70.0, "lift": 0.0},
		],
	},
	"bridge_after": {
		"map": "river_land", "scene": "bridge", "seed": 7, "weather": "clear", "daytime": "day",
		"start_s": 46.0, "seconds": 15.0, "stills": [2.0, 8.0, 13.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 2), "dist": 250.0, "yaw": 18.0, "pitch": 60.0, "lift": 0.0},
			{"t": 15.0, "at": Vector2(-8, 2), "dist": 205.0, "yaw": 48.0, "pitch": 56.0, "lift": 0.0},
		],
	},
	# --- Variety: desert, winter, town ----------------------------------------
	"desert_road": {
		"map": "desert_village", "scene": "desert", "seed": 7, "start_s": 6.0, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "unit": [0, "us_m4a3_75", 0], "dist": 30.0, "yaw": 205.0, "pitch": 5.0, "lift": 1.5},
			{"t": 6.0, "unit": [0, "us_m4a3_75", 0], "dist": 20.0, "yaw": 230.0, "pitch": 6.0, "lift": 1.5},
		],
	},
	"desert_square": {
		"map": "desert_village", "scene": "desert", "seed": 7, "start_s": 47.0, "seconds": 7.0,
		"stills": [1.2, 4.6, 6.0],
		"camera": [
			{"t": 0.0, "at": Vector2(8, 26), "dist": 38.0, "yaw": 140.0, "pitch": 13.0, "lift": 1.5},
			{"t": 7.0, "at": Vector2(8, 26), "dist": 30.0, "yaw": 185.0, "pitch": 10.0, "lift": 1.5},
		],
	},
	"winter_is2": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 5.0, "seconds": 7.0,
		"stills": [1.0, 3.4, 6.0],
		"camera": [
			{"t": 0.0, "unit": [0, "su_is2", 0], "heading": true, "dist": 17.0, "yaw": 65.0, "pitch": 6.0, "lift": 1.6},
			{"t": 7.0, "unit": [0, "su_is2", 0], "heading": true, "dist": 13.0, "yaw": 110.0, "pitch": 5.0, "lift": 1.6},
		],
	},
	"winter_village": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 6.5, "seconds": 6.0,
		"stills": [1.0, 1.9, 3.4, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(40, 72), "dist": 36.0, "yaw": 150.0, "pitch": 10.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(40, 72), "dist": 28.0, "yaw": 175.0, "pitch": 8.0, "lift": 1.5},
		],
	},
	"winter_infantry": {
		"map": "winter_village", "scene": "winter", "seed": 7, "start_s": 1.0, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "unit": [0, "su_inf_rifle_squad", -1], "dist": 20.0, "yaw": 205.0, "pitch": 8.0, "lift": 1.0},
			{"t": 6.0, "unit": [0, "su_inf_rifle_squad", -1], "dist": 15.0, "yaw": 225.0, "pitch": 7.0, "lift": 1.0},
		],
	},
	"town_street": {
		"map": "town_small", "scene": "town", "seed": 7, "start_s": 1.5, "seconds": 6.0,
		"stills": [1.0, 3.0, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 128), "dist": 30.0, "yaw": 180.0, "pitch": 8.0, "lift": 1.5},
			{"t": 6.0, "at": Vector2(0, 122), "dist": 24.0, "yaw": 180.0, "pitch": 10.0, "lift": 1.5},
		],
	},
	# --- Maps from above, for planning shots ----------------------------------
	# Impact markers of the own shells (prompt 084): the launchers' salvo
	# into the wood's edge seen with the player's interface, by day and in
	# the evening, at a middle and a far zoom level. Only when named, like
	# every `survey_` shot: `just trailer-shots --stills --only=...`.
	"survey_markers_day_mid": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "day", "game_camera": true, "hud": true,
		"start_s": 68.0, "seconds": 3.0, "stills": [0.5, 1.6, 2.8],
		"camera": [{"t": 0.0, "at": Vector2(0, -105), "dist": 110.0, "yaw": 0.0}],
	},
	"survey_markers_day_far": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "day", "game_camera": true, "hud": true,
		"start_s": 68.0, "seconds": 3.0, "stills": [0.5, 1.6, 2.8],
		"camera": [{"t": 0.0, "at": Vector2(0, -105), "dist": 260.0, "yaw": 0.0}],
	},
	"survey_markers_dusk_mid": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "evening", "game_camera": true, "hud": true,
		"start_s": 68.0, "seconds": 3.0, "stills": [0.5, 1.6, 2.8],
		"camera": [{"t": 0.0, "at": Vector2(0, -105), "dist": 110.0, "yaw": 0.0}],
	},
	"survey_markers_dusk_far": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "evening", "game_camera": true, "hud": true,
		"start_s": 68.0, "seconds": 3.0, "stills": [0.5, 1.6, 2.8],
		"camera": [{"t": 0.0, "at": Vector2(0, -105), "dist": 260.0, "yaw": 0.0}],
	},
	# Trees under heavy fire (prompt 085): the launchers' salvo into the
	# wood, before it lands and ten seconds after the last rocket, from the
	# side and from above. Only when named.
	"survey_forest_blast_side": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "day", "game_camera": true,
		"start_s": 66.0, "seconds": 19.0, "stills": [0.5, 18.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -125), "dist": 120.0, "yaw": 0.0}],
	},
	"survey_forest_blast_top": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "day",
		"start_s": 66.0, "seconds": 19.0, "stills": [0.5, 18.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -125), "dist": 170.0, "yaw": 0.0, "pitch": 80.0}],
	},
	# Trees in the wind (prompt 086, `just wind-shots`): the wood's edge
	# with the meadow in front, two stills one second apart per view.
	"survey_wind_mid": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "day", "game_camera": true,
		"start_s": 2.0, "seconds": 2.2, "stills": [0.5, 1.5],
		"camera": [{"t": 0.0, "at": Vector2(-70, -110), "dist": 110.0, "yaw": 0.0}],
	},
	"survey_wind_near": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "day", "game_camera": true,
		"start_s": 2.0, "seconds": 2.2, "stills": [0.5, 1.5],
		"camera": [{"t": 0.0, "at": Vector2(-70, -108), "dist": 45.0, "yaw": 0.0}],
	},
	# Cloud shadows (prompt 087): the meadow and the wood's edge of
	# ambush_meadow, a village of test_large; near, middle and far, clear by
	# day, overcast and evening once; a series every 2 s at a fixed camera.
	"survey_clouds_meadow": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 2.0, "seconds": 1.5, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -40), "dist": 300.0, "yaw": 0.0}],
	},
	"survey_clouds_near": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 30.0, "seconds": 18.0, "stills": [0.5, 3.5, 6.5, 9.5, 12.5, 15.5],
		"camera": [{"t": 0.0, "unit": [0, "su_", -1], "dist": 110.0, "yaw": 0.0}],
	},
	"survey_clouds_far": {
		"map": "test_large", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 2.0, "seconds": 1.5, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(-200, 0), "dist": 700.0, "yaw": 0.0}],
	},
	"survey_clouds_overcast": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "day", "weather": "overcast", "game_camera": true,
		"start_s": 2.0, "seconds": 1.5, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -40), "dist": 300.0, "yaw": 0.0}],
	},
	"survey_clouds_evening": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "evening", "weather": "clear", "game_camera": true,
		"start_s": 2.0, "seconds": 1.5, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -40), "dist": 300.0, "yaw": 0.0}],
	},
	"survey_clouds_series": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 2.0, "seconds": 25.0, "stills": [0.5, 2.5, 4.5, 6.5, 8.5, 10.5, 12.5, 14.5, 16.5, 18.5, 20.5, 22.5, 24.5],
		"camera": [{"t": 0.0, "at": Vector2(0, -40), "dist": 300.0, "yaw": 0.0}],
	},
	# Maps from picture sketches (prompt 088): the whole map from above,
	# for the comparison with the template (`build/sketch-compare/`).
	"survey_river_fields": {
		"map": "river_fields", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_river_fields_winter": {
		"map": "river_fields_winter", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_river_fork": {
		"map": "river_fork", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_island_chain": {
		"map": "island_chain", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_desert_lagoon": {
		"map": "desert_lagoon", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_winter_town": {
		"map": "winter_town", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 3000.0, "yaw": 0.0, "pitch": 89.0, "haze": 6000.0}],
	},
	"survey_fields_near": {
		"map": "river_fields", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(150, -330), "dist": 60.0, "yaw": 30.0}],
	},
	"survey_fields_mid": {
		"map": "river_fields", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(150, -330), "dist": 180.0, "yaw": 30.0}],
	},
	"survey_fields_far": {
		"map": "river_fields", "scene": "survey", "seed": 1, "daytime": "day", "weather": "clear", "game_camera": true,
		"start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(100, 300), "dist": 480.0, "yaw": 0.0}],
	},
	"survey_fields_winter": {
		"map": "river_fields_winter", "scene": "survey", "seed": 1, "daytime": "day", "game_camera": true,
		"start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(150, -330), "dist": 180.0, "yaw": 30.0}],
	},
	"survey_fields_rain": {
		"map": "river_fields", "scene": "survey", "seed": 1, "daytime": "day", "weather": "rain", "game_camera": true,
		"start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(150, -330), "dist": 180.0, "yaw": 30.0}],
	},
	"survey_farms": {
		"map": "farms", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_desert_village": {
		"map": "desert_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1522.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_winter_village": {
		"map": "winter_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_river_land": {
		"map": "river_land", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_town_small": {
		"map": "town_small", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1141.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	# Prompt 071: meadow and road near a corner of the 4 km map, panned, zoomed
	# and turned at once, to measure whether the ground texture sticks to the
	# ground (`just ground-motion`, tools/ground_flicker.py).
	"survey_ground": {
		"map": "test_large", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 6.0,
		"stills": [0.5, 3.0, 5.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-1720, 1180), "dist": 150.0, "yaw": 0.0, "pitch": 52.0},
			{"t": 6.0, "at": Vector2(-1680, 1220), "dist": 80.0, "yaw": 40.0, "pitch": 46.0},
		],
	},
	# Prompt 068: open meadow with tufts at the game's near zoom levels, panned
	# at about 6 px/frame like a player's pan; camera as in the game (pitch of
	# the rig, `haze` = distance, so depth of field and shadow range match).
	# `just ground-motion --near` measures them, `--trailer-off=` turns parts off.
	"meadow_near_15": {
		"map": "test_large", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-30, 70), "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
			{"t": 3.0, "at": Vector2(-22.5, 70), "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
		],
	},
	"meadow_near_35": {
		"map": "test_large", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-40, 75), "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
			{"t": 3.0, "at": Vector2(-22, 75), "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
		],
	},
	"meadow_near_55": {
		"map": "test_large", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-50, 85), "dist": 55.0, "yaw": 23.0, "pitch": 50.6, "haze": 55.0},
			{"t": 3.0, "at": Vector2(-20, 85), "dist": 55.0, "yaw": 23.0, "pitch": 50.6, "haze": 55.0},
		],
	},
	# Prompt 073: the same pans over tall grass (`tall_grass_test`), plus one
	# at 110 m where the terrain draws the stand. `just ground-motion --tall`.
	"tall_near_15": {
		"map": "tall_grass_test", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(104, -60), "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
			{"t": 3.0, "at": Vector2(111.5, -60), "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
		],
	},
	"tall_near_35": {
		"map": "tall_grass_test", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(96, -60), "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
			{"t": 3.0, "at": Vector2(114, -60), "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
		],
	},
	"tall_near_55": {
		"map": "tall_grass_test", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(90, -60), "dist": 55.0, "yaw": 23.0, "pitch": 50.6, "haze": 55.0},
			{"t": 3.0, "at": Vector2(120, -60), "dist": 55.0, "yaw": 23.0, "pitch": 50.6, "haze": 55.0},
		],
	},
	"tall_near_110": {
		"map": "tall_grass_test", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(80, -60), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
			{"t": 3.0, "at": Vector2(140, -60), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
		],
	},
	# Prompt 075: pans over the town houses of town_small at the game's zoom
	# levels, to measure whether the roofs flicker (`just ground-motion --roofs`).
	"roof_near_15": {
		"map": "town_small", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(40, -12), "lift": 12.0, "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
			{"t": 3.0, "at": Vector2(47.5, -12), "lift": 12.0, "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
		],
	},
	"roof_near_35": {
		"map": "town_small", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(30, -12), "lift": 12.0, "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
			{"t": 3.0, "at": Vector2(48, -12), "lift": 12.0, "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
		],
	},
	"roof_near_110": {
		"map": "town_small", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(10, 0), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
			{"t": 3.0, "at": Vector2(70, 0), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
		],
	},
	# Prompt 080: the same pans over the houses east of the winter village's
	# center (log house, stone house, farm house in a row), for the roof snow
	# (`just ground-motion --snow-roofs`).
	"roof_snow_15": {
		"map": "winter_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"weather": "clear", "daytime": "day", "stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(50, -11.5), "lift": 6.0, "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
			{"t": 3.0, "at": Vector2(57.5, -11.5), "lift": 6.0, "dist": 15.0, "yaw": 23.0, "pitch": 45.0, "haze": 15.0},
		],
	},
	"roof_snow_35": {
		"map": "winter_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"weather": "clear", "daytime": "day", "stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(40, -11.5), "lift": 6.0, "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
			{"t": 3.0, "at": Vector2(58, -11.5), "lift": 6.0, "dist": 35.0, "yaw": 23.0, "pitch": 47.8, "haze": 35.0},
		],
	},
	"roof_snow_110": {
		"map": "winter_village", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 3.0, "game_camera": true,
		"weather": "clear", "daytime": "day", "stills": [1.5],
		"camera": [
			{"t": 0.0, "at": Vector2(20, 0), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
			{"t": 3.0, "at": Vector2(80, 0), "dist": 110.0, "yaw": 23.0, "pitch": 56.0, "haze": 110.0},
		],
	},
	# --- Scene "ambush" on ambush_meadow (prompt 082) ---------------------------
	# One battle told in seventeen shots, all with the player's camera
	# (`game_camera`: focus, distance and yaw only). Seed 14: the point tank
	# blows up at 50.0 s and loses its turret, a Tiger at 57.75 s, the last
	# T-34 at 74.5 s; rockets leave from 56.5 s and land from 69.25 s; the
	# bombs fall from 82.25 s. In the order of the edit. Shots after the
	# first explosions have a long pre-roll (see `PRE_ROLL_S`).
	# Act 1, march: the column as a line in the grass, the west wood behind it.
	"ambush_march_wide": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 34.3, "seconds": 5.0, "stills": [1.0, 4.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 325), "dist": 300.0, "yaw": 65.0},
			{"t": 5.0, "at": Vector2(0, 255), "dist": 280.0, "yaw": 72.0},
		],
	},
	# Act 1: beside the tanks, the grass parts.
	"ambush_march_side": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 37.5, "seconds": 4.5, "stills": [1.0, 3.5],
		"camera": [
			{"t": 0.0, "unit": [0, "su_t34", -1], "dist": 70.0, "yaw": 95.0},
			{"t": 4.5, "unit": [0, "su_t34", -1], "dist": 55.0, "yaw": 80.0},
		],
	},
	# Act 1: the silent wood, the point tank drives into the picture.
	"ambush_march_wood": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 46.8, "seconds": 3.0, "stills": [0.5, 2.6],
		"camera": [
			{"t": 0.0, "at": Vector2(0, -48), "dist": 235.0, "yaw": 10.0},
			{"t": 3.0, "at": Vector2(0, -50), "dist": 225.0, "yaw": 10.0},
		],
	},
	# Act 2, 50.0 s: the point tank, the shot comes from outside the picture.
	"ambush_shot_near": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 48.6, "seconds": 4.2, "stills": [1.2, 1.5, 2.2, 3.6],
		"camera": [
			{"t": 0.0, "unit": [0, "su_t34", 0], "dist": 32.0, "yaw": 100.0},
			{"t": 1.4, "unit": [0, "su_t34", 0], "dist": 32.0, "yaw": 100.0},
			{"t": 4.2, "unit": [0, "su_t34", 0], "dist": 42.0, "yaw": 92.0},
		],
	},
	# Act 2: the same second wide, the muzzle flashes at the wood's edge.
	"ambush_shot_wide": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 49.2, "seconds": 4.3, "stills": [0.85, 1.3, 2.1, 3.5],
		"camera": [
			{"t": 0.0, "at": Vector2(0, -68), "dist": 200.0, "yaw": 8.0},
			{"t": 4.3, "at": Vector2(0, -68), "dist": 200.0, "yaw": 8.0},
		],
	},
	# Act 3, 50.0 to 51.0 s: machine guns from the west wood cut a rifle squad down.
	"ambush_mg": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 49.8, "seconds": 3.6, "stills": [0.5, 1.3, 2.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-50, 376), "dist": 60.0, "yaw": 70.0},
			{"t": 3.6, "at": Vector2(-50, 376), "dist": 55.0, "yaw": 70.0},
		],
	},
	# Act 3: the three tanks spread and fire back; 55.0 s the Tigers' second salvo.
	"ambush_spread": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 52.0, "seconds": 4.5, "pre_roll_s": 3, "stills": [1.0, 3.05, 4.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, -35), "dist": 175.0, "yaw": 70.0},
			{"t": 4.5, "at": Vector2(0, -40), "dist": 160.0, "yaw": 55.0},
		],
	},
	# Act 4: the German side, along the wood's edge: both Tigers, the meadow before them.
	"ambush_reveal": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 53.4, "seconds": 4.0, "pre_roll_s": 4.5, "stills": [0.5, 1.65, 3.5],
		"camera": [
			{"t": 0.0, "at": Vector2(0, -98), "dist": 150.0, "yaw": 80.0},
			{"t": 4.0, "at": Vector2(0, -98), "dist": 140.0, "yaw": 80.0},
		],
	},
	# Act 5, 56.5 s: the launchers fire.
	"ambush_rockets_near": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 56.2, "seconds": 4.0, "pre_roll_s": 7.5, "stills": [0.5, 1.5, 3.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 300), "dist": 50.0, "yaw": 95.0},
			{"t": 4.0, "at": Vector2(0, 296), "dist": 58.0, "yaw": 95.0},
		],
	},
	# Act 6, 57.75 s: a Tiger blows up.
	"ambush_tiger_hit": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 56.9, "seconds": 2.6, "pre_roll_s": 8, "stills": [0.6, 0.95, 1.8],
		"camera": [
			{"t": 0.0, "at": Vector2(-36, -112), "dist": 45.0, "yaw": 20.0},
			{"t": 2.6, "at": Vector2(-36, -112), "dist": 45.0, "yaw": 20.0},
		],
	},
	# Act 5: from behind the launchers, the smoke trails towards the wood.
	"ambush_rockets_trails": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 57.2, "seconds": 4.4, "pre_roll_s": 8.5, "stills": [1.0, 2.5, 4.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 255), "dist": 200.0, "yaw": 0.0},
			{"t": 4.4, "at": Vector2(0, 250), "dist": 200.0, "yaw": 8.0},
		],
	},
	# Act 6, 60.0 s: the tank on the east flank is hit.
	"ambush_t34_east": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 58.9, "seconds": 2.6, "pre_roll_s": 10, "stills": [0.8, 1.2, 2.0],
		"camera": [
			{"t": 0.0, "unit": [0, "su_t34", 2], "dist": 55.0, "yaw": 60.0},
			{"t": 2.6, "unit": [0, "su_t34", 2], "dist": 55.0, "yaw": 60.0},
		],
	},
	# Act 6, 63.75 and 64.75 s: tank against tank across the edge.
	"ambush_duel": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 63.2, "seconds": 3.2, "pre_roll_s": 12, "stills": [0.6, 1.6, 2.5],
		"camera": [
			{"t": 0.0, "at": Vector2(-16, -74), "dist": 185.0, "yaw": 39.0},
			{"t": 3.2, "at": Vector2(-16, -74), "dist": 185.0, "yaw": 39.0},
		],
	},
	# Act 5, 69.25 to 74.25 s: the rockets come down in the wood.
	"ambush_impacts": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 69.0, "seconds": 5.2, "pre_roll_s": 12, "stills": [0.6, 2.0, 3.5, 5.0],
		"camera": [
			{"t": 0.0, "at": Vector2(5, -128), "dist": 170.0, "yaw": 25.0},
			{"t": 5.2, "at": Vector2(5, -128), "dist": 160.0, "yaw": 12.0},
		],
	},
	# Act 6, 74.5 s: the last tank.
	"ambush_last_t34": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 73.6, "seconds": 2.4, "pre_roll_s": 12, "stills": [0.8, 1.0, 1.9],
		"camera": [
			{"t": 0.0, "unit": [0, "su_t34", 3], "dist": 45.0, "yaw": 110.0},
			{"t": 2.4, "unit": [0, "su_t34", 3], "dist": 45.0, "yaw": 110.0},
		],
	},
	# Act 7, 82.25 to 86.0 s: the bombers' shadows, then the sticks along the column.
	"ambush_bombers": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 80.0, "seconds": 6.6, "pre_roll_s": 12, "stills": [1.0, 2.4, 3.6, 5.1, 6.2],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 235), "dist": 270.0, "yaw": 90.0},
			{"t": 6.6, "at": Vector2(0, 250), "dist": 270.0, "yaw": 90.0},
		],
	},
	# Act 8, from 110 s: the meadow burns; out to the widest view, and stay.
	"ambush_end": {
		"map": "ambush_meadow", "scene": "ambush", "seed": 14, "daytime": "morning", "game_camera": true,
		"start_s": 110.0, "seconds": 12.0, "pre_roll_s": 12, "stills": [1.0, 5.0, 11.0],
		"camera": [
			{"t": 0.0, "at": Vector2(0, 150), "dist": 230.0, "yaw": 25.0},
			{"t": 9.0, "at": Vector2(0, 110), "dist": 600.0, "yaw": 40.0},
			{"t": 12.0, "at": Vector2(0, 110), "dist": 600.0, "yaw": 40.0},
		],
	},
	"survey_ambush_meadow": {
		"map": "ambush_meadow", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
	"survey_twin_meadow": {
		"map": "twin_meadow", "scene": "survey", "seed": 1, "start_s": 0.5, "seconds": 1.0, "stills": [0.5],
		"camera": [{"t": 0.0, "at": Vector2(0, 0), "dist": 1902.0, "yaw": 0.0, "pitch": 89.0, "haze": 4000.0}],
	},
}

var main: Node3D
var _name := ""
var _shot: Dictionary = {}
var _stills_dir := ""
var _scout := false
var _scout_seconds := -1.0
## -1: run-up, else the frame of the shot.
var _frame := -1
## Master bus muted (the fast part of the run-up).
var _silent := true
var _stills_left: Array = []
## Parts turned off with `--trailer-off=` (prompt 068).
var _off: PackedStringArray = []
## Smoothed heading per camera key (chase rides must not jitter).
var _headings: Dictionary[int, float] = {}
## Combat events by type since the last scout report.
var _scout_counts: Dictionary[String, int] = {}
var _scout_first_shot := false
## Turrets blown off so far (ammunition explosions).
var _scout_turrets := 0


## The shot named on the command line, or "".
static func requested() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--trailer="):
			return arg.trim_prefix("--trailer=")
	return ""


## Starts the scene of the requested shot on `sim_host` (called by `main.gd`
## in place of the normal start).
static func start(sim_host: SimHost) -> bool:
	var shot_name := requested()
	if not SHOTS.has(shot_name):
		push_error("unknown trailer shot '%s'; known: %s" % [shot_name, ", ".join(SHOTS.keys())])
		return false
	var shot: Dictionary = SHOTS[shot_name]
	var scene: String = shot.get("scene", shot_name)
	var file := GamePaths.data_dir().path_join(SCENE_DIR).path_join(scene + ".toml")
	sim_host.hold_fire_at_start = false
	sim_host.weather = shot.get("weather", "")
	sim_host.daytime = shot.get("daytime", "")
	var match_seed := int(shot.get("seed", 1))
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--trailer-seed="):
			match_seed = int(arg.trim_prefix("--trailer-seed="))
	return sim_host.start_mission_on(match_seed, file.simplify_path(),
		GamePaths.map_dir(shot["map"]).simplify_path())


func _ready() -> void:
	_name = requested()
	_shot = SHOTS[_name]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--trailer-stills="):
			_stills_dir = arg.trim_prefix("--trailer-stills=")
		elif arg == "--trailer-scout":
			_scout = true
		elif arg.begins_with("--trailer-scout="):
			_scout = true
			_scout_seconds = float(arg.trim_prefix("--trailer-scout="))
		elif arg.begins_with("--trailer-off="):
			_off = arg.trim_prefix("--trailer-off=").split(",", false)
			for part in _off:
				if part not in OFF_PARTS:
					push_error("unknown --trailer-off part '%s'; known: %s" % [part, ", ".join(OFF_PARTS)])
	_stills_left = (_shot.get("stills", [float(_shot["seconds"]) * 0.5]) as Array).duplicate()
	# Effects roll with Godot's generator (where a turret lands, how a wreck
	# smokes): the same numbers every time the shot is recorded.
	seed(int(_shot.get("seed", 1)))
	if not _stills_dir.is_empty():
		DirAccess.make_dir_recursive_absolute(_stills_dir)

	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	rig.set_process(false)
	rig.set_process_unhandled_input(false)
	# Nobody plays: no pause menu, no result screen, no clicks.
	main.game_menu.visible = false
	main.game_menu.process_mode = Node.PROCESS_MODE_DISABLED
	get_viewport().gui_disable_input = true
	main.selection.set_process_input(false)
	main.selection.set_process_unhandled_input(false)
	main.atmosphere.set_quality("high")
	if not _shot.get("hud", false):
		(main.get_node("Hud") as CanvasLayer).visible = false
		main.fog_of_war.set_fog_enabled(false)
		main.effects.visible_at = func(_point: Vector3) -> bool: return true
		if main.battle_sound != null:
			main.battle_sound.visible_at = main.effects.visible_at
		# The rings where the own shells will land are interface too.
		main.artillery_view.markers_enabled = false
		# The relation tint is part of the interface, not of the world.
		main._enemy_material.albedo_color.a = 0.0
		main._ally_material.albedo_color.a = 0.0
		# Flags and capture rings of the supply points.
		for marker: Node3D in main.base_view._supply_nodes:
			marker.visible = false
	else:
		# The player's view with a large interface (a game setting), so it
		# can be read in a video.
		get_window().content_scale_factor = 1.25
	get_window().scaling_3d_scale = 1.0
	if _scout:
		main.combat_view.combat_event.connect(_on_scout_event)
	elif DisplayServer.get_name() != "headless":
		get_window().size = SIZE
	_set_volumes()
	main.sim_host.speed = RUN_UP_SPEED
	_place_camera(0.0)


func _process(_delta: float) -> void:
	var host: SimHost = main.sim_host
	var tick := host.current_tick()
	_set_volumes()
	_apply_off()
	if _scout:
		_scout_step(tick)
		return
	var start_tick := roundi(float(_shot.get("start_s", 0.0)) * TICK_HZ)
	if _frame < 0:
		# Run-up: fast and silent, then the pre-roll at normal speed.
		var pre_roll := float(_shot.get("pre_roll_s", PRE_ROLL_S))
		if tick >= start_tick - roundi(pre_roll * TICK_HZ) and host.speed != 1.0:
			host.speed = 1.0
			_silent = false
		_place_camera(0.0)
		if tick < start_tick:
			return
		_frame = 0
		if _shot.has("select"):
			var ids: Array = []
			for node in _unit_nodes(_shot["select"]):
				ids.append(int(str(node.name).trim_prefix("Unit")))
			main.selection.set_selection(ids)
		print("trailer: record %s start_frame=%d frames=%d fps=%d tick=%d" % [
			_name, Engine.get_frames_drawn(), _frame_count(), FPS, tick])
	var t := float(_frame) / FPS
	_place_camera(t)
	if not _stills_dir.is_empty():
		_save_stills(t)
	_frame += 1
	if _frame >= _frame_count() or (not _stills_dir.is_empty() and _stills_left.is_empty()):
		print("trailer: %s done at tick %d" % [_name, host.current_tick()])
		set_process(false)
		# The frame in work is still drawn (and written) before the quit.
		get_tree().quit.call_deferred()


func _frame_count() -> int:
	return roundi(float(_shot["seconds"]) * FPS)


## Turns the parts of `--trailer-off=` off, every frame: decoration is built
## while the camera moves, and the game re-applies quality and settings.
func _apply_off() -> void:
	if _off.is_empty():
		return
	if "tufts" in _off:
		for node: Node in main.map_objects._deco_root.get_children():
			if node.name.begins_with("grass_"):
				(node as Node3D).visible = false
	if "ground" in _off:
		main.terrain._material.set_shader_parameter("meadow_strength", 0.0)
		main.terrain._material.set_shader_parameter("flower_density", 0.0)
	if "shadows" in _off:
		main.sun.shadow_enabled = false
	if "wind" in _off:
		MeadowTufts.MATERIAL.set_shader_parameter("wind_strength", 0.0)
		TallGrass.MATERIAL.set_shader_parameter("wind_strength", 0.0)
		main.terrain._material.set_shader_parameter("wind_strength", 0.0)
		# The trees too (prompt 086).
		EnvLook.still_trees()
	# Prompt 073: tall grass plants, or its look on the ground.
	if "tall" in _off:
		for node: Node in main.map_objects._deco_root.get_children():
			if node.name.begins_with("tall_"):
				(node as Node3D).visible = false
	if "tall_ground" in _off:
		main.terrain.set_tall_strength(0.0)
	if "dof" in _off and Settings.get_value("depth_of_field") != "off":
		Settings.set_value("depth_of_field", "off")
	if "ssao" in _off:
		main.atmosphere._env.ssao_enabled = false
	if "snowfall" in _off and main.atmosphere._snow_root != null:
		main.atmosphere._snow_root.visible = false
	if "clouds" in _off:
		main.atmosphere.cloud_clock = Callable()
	var vp := get_viewport()
	if "msaa" in _off:
		vp.msaa_3d = Viewport.MSAA_DISABLED
	if "fxaa" in _off:
		vp.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA
	if "taa" in _off:
		vp.use_taa = true


## The same volumes whatever the player's settings say, set every frame:
## the game applies the settings again (and mutes) when its window loses
## or gains the focus. Headroom on the master bus, because layered
## explosions clip a 16-bit recording otherwise (the edit sets the
## loudness); no music, which a cut would chop; silent during the run-up.
func _set_volumes() -> void:
	for bus: String in BUS_DB:
		var index := AudioServer.get_bus_index(bus)
		if index >= 0:
			AudioServer.set_bus_volume_db(index, BUS_DB[bus])
			AudioServer.set_bus_mute(index, BUS_DB[bus] <= -80.0 or (index == 0 and _silent))


# --- Camera ---------------------------------------------------------------------


## Places the camera for `t` seconds into the shot.
func _place_camera(t: float) -> void:
	var keys: Array = _shot.get("camera", [])
	if keys.is_empty():
		return
	# The segment of `t` and the position in it.
	var i := 0
	while i < keys.size() - 2 and t >= float(keys[i + 1]["t"]):
		i += 1
	var a: Dictionary = keys[i]
	var b: Dictionary = keys[mini(i + 1, keys.size() - 1)]
	var span := float(b["t"]) - float(a["t"])
	var u := clampf((t - float(a["t"])) / span, 0.0, 1.0) if span > 0.0 else 0.0
	var before: Dictionary = keys[maxi(i - 1, 0)]
	var after: Dictionary = keys[mini(i + 2, keys.size() - 1)]
	var indices := [maxi(i - 1, 0), i, mini(i + 1, keys.size() - 1), mini(i + 2, keys.size() - 1)]
	var four := [before, a, b, after]

	var value := func(key: String, fallback: float, logarithmic := false) -> float:
		var v: Array[float] = []
		for k: Dictionary in four:
			var x := float(k.get(key, fallback))
			v.append(log(x) if logarithmic else x)
		var r := _catmull(v[0], v[1], v[2], v[3], u)
		return exp(r) if logarithmic else r

	var points: Array[Vector3] = []
	var yaws: Array[float] = []
	for n in 4:
		var k: Dictionary = four[n]
		var point := _look_point(k)
		points.append(point)
		var yaw := deg_to_rad(float(k.get("yaw", 0.0)))
		if k.get("heading", false):
			yaw += _heading(indices[n], k)
		yaws.append(yaw)
	var target := points[1].cubic_interpolate(points[2], points[0], points[3], u)
	# Angles turn the short way between neighbouring keys.
	for n in range(1, 4):
		yaws[n] = yaws[n - 1] + angle_difference(yaws[n - 1], yaws[n]) if four[n].get("heading", false) \
			or four[n - 1].get("heading", false) else yaws[n]
	var yaw := _catmull(yaws[0], yaws[1], yaws[2], yaws[3], u)
	var distance: float = value.call("dist", 80.0, true)
	var pitch := deg_to_rad(value.call("pitch", 45.0))
	var fov: float = value.call("fov", main.camera_rig.fov_deg)
	# Flat views look far: the haze of a steep view would hide the distance.
	var haze: float = value.call("haze", -1.0)
	if haze <= 0.0:
		haze = maxf(distance / maxf(sin(pitch), 0.2), 100.0)

	var rig: Node3D = main.camera_rig
	var camera: Camera3D = main.camera
	if _player_view():
		# The rig does what it does in a match; nothing is placed by hand.
		camera.fov = rig.fov_deg
		rig.snap_to(Vector3(target.x, 0.0, target.z), distance, yaw)
		return
	var ground := Vector3(target.x, main.sim_host.height_at(target.x, target.z), target.z)
	rig.focus = ground
	rig.distance = haze
	rig.target_distance = haze
	rig.yaw = yaw
	rig.target_yaw = yaw
	rig.global_position = ground
	var back := Vector3(0.0, sin(pitch), cos(pitch)).rotated(Vector3.UP, yaw)
	var eye := target + back * distance
	eye.y = maxf(eye.y, main.sim_host.height_at(eye.x, eye.z) + CLEARANCE)
	camera.global_position = eye
	camera.look_at(target, Vector3.UP)
	camera.fov = fov
	camera.near = clampf(distance * 0.02, 0.05, 2.0)
	camera.far = maxf(distance * 4.0, 2500.0)
	if _shot.get("game_camera", false):
		main.sun.directional_shadow_max_distance = haze * main.camera_rig.shadow_distance_factor
	else:
		main.sun.directional_shadow_max_distance = clampf(haze * 2.5, 60.0, 500.0)


## Whether the shot is taken with the game's camera alone: `game_camera`
## and no key that places the camera freely.
func _player_view() -> bool:
	if not _shot.get("game_camera", false):
		return false
	for key: Dictionary in _shot.get("camera", []):
		for free: String in ["pitch", "lift", "fov", "haze"]:
			if key.has(free):
				return false
	return true


## The point a camera key looks at, right now.
func _look_point(key: Dictionary) -> Vector3:
	var lift := float(key.get("lift", 1.0))
	if key.has("unit"):
		var nodes := _unit_nodes(key["unit"])
		if not nodes.is_empty():
			var sum := Vector3.ZERO
			for node in nodes:
				sum += node.global_position
			return sum / nodes.size() + Vector3.UP * lift
	var at: Vector2 = key.get("at", Vector2.ZERO)
	return Vector3(at.x, main.sim_host.height_at(at.x, at.y) + lift, at.y)


## Driving direction of the key's unit as a camera yaw, smoothed.
func _heading(index: int, key: Dictionary) -> float:
	var nodes := _unit_nodes(key.get("unit", []))
	if nodes.is_empty():
		return _headings.get(index, 0.0)
	var heading: float = nodes[0].rotation.y
	if _headings.has(index):
		heading = lerp_angle(_headings[index], heading, 0.04)
	_headings[index] = heading
	return heading


## The drawn nodes of `[owner, catalog id prefix, index]`: the living units
## of that kind sorted by id, or only the one at `index` (also when it is
## a wreck by now).
func _unit_nodes(spec: Array) -> Array[Node3D]:
	var result: Array[Node3D] = []
	if spec.size() < 2:
		return result
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var alive := host.unit_alive()
	var matching: Array[int] = []
	var living: Dictionary[int, bool] = {}
	for i in ids.size():
		if owners[i] == int(spec[0]) and types[i].begins_with(str(spec[1])):
			matching.append(ids[i])
			living[ids[i]] = alive[i] != 0
	matching.sort()
	var index := int(spec[2]) if spec.size() > 2 else -1
	for n in matching.size():
		var id := matching[n]
		if (index < 0 and living[id]) or n == index:
			var node: Node3D = main._unit_nodes.get(id)
			if node != null:
				result.append(node)
	return result


static func _catmull(p0: float, p1: float, p2: float, p3: float, u: float) -> float:
	return 0.5 * ((2.0 * p1) + (-p0 + p2) * u + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
		+ (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u)


# --- Stills ---------------------------------------------------------------------


func _save_stills(t: float) -> void:
	if _stills_left.is_empty() or t < float(_stills_left[0]):
		return
	var at := float(_stills_left.pop_front())
	# The picture of this frame exists after it was drawn.
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var path := "%s/%s_%04.1f.png" % [_stills_dir, _name, at]
	image.save_png(path)
	print("trailer: still %s" % path)


# --- Scouting -------------------------------------------------------------------


func _scout_step(tick: int) -> void:
	var seconds := _scout_seconds
	if seconds < 0.0:
		seconds = float(_shot.get("start_s", 0.0)) + float(_shot["seconds"])
	var turrets := int(main.blast_effects.counts.get("turret", 0))
	if turrets > _scout_turrets:
		_scout_turrets = turrets
		print("trailer-scout: %6.2f s ammunition explosion, turret %d blown off" % [
			float(tick) / TICK_HZ, turrets])
	if tick % (5 * TICK_HZ) < 5 and tick > 0:
		_print_groups(tick)
	if tick >= roundi(seconds * TICK_HZ):
		_print_groups(tick)
		set_process(false)
		get_tree().quit()


func _on_scout_event(event: Dictionary) -> void:
	_scout_counts[event["type"]] = int(_scout_counts.get(event["type"], 0)) + 1
	if event["type"] == "shot" and not _scout_first_shot:
		_scout_first_shot = true
		print("trailer-scout: %6.2f s first shot by %s" % [
			float(main.sim_host.current_tick()) / TICK_HZ, _unit_text(event.get("unit", -1))])
	# Guns (not small arms): who fires when, and what a hit on a unit does.
	var now := float(main.sim_host.current_tick()) / TICK_HZ
	if event["type"] == "shot" and float(event["caliber_mm"]) >= 37.0:
		print("trailer-scout: %6.2f s shot %s %d mm%s" % [now, _unit_text(event["unit"]),
			roundi(event["caliber_mm"]), " (indirect)" if int(event["weapon"]) == 255 else ""])
	if event["type"] == "hit" and float(event["caliber_mm"]) >= 37.0 and event["surface"] == "unit":
		print("trailer-scout: %6.2f s hit %s by %s: %s, damage %d" % [now, _unit_text(event["target"]),
			_unit_text(event["shooter"]), "penetrated" if event["penetrated"] else "bounced",
			int(event["damage"])])
	# Where shells, rockets and bombs come down.
	if event["type"] == "hit" and float(event["caliber_mm"]) >= 100.0 and event["surface"] != "unit":
		var where: Vector3 = event["position"]
		print("trailer-scout: %6.2f s impact %d mm at (%d, %d) on %s" % [now,
			roundi(event["caliber_mm"]), roundi(where.x), roundi(where.z), event["surface"]])
	if event["type"] == "destroyed":
		var at: Vector3 = event["position"]
		print("trailer-scout: %6.2f s destroyed %s at (%d, %d) by %s" % [
			float(main.sim_host.current_tick()) / TICK_HZ, _unit_text(event["unit"]),
			roundi(at.x), roundi(at.z), _unit_text(event["by"])])


## "owner type #index" of a unit id, the index as camera keys count it.
func _unit_text(id: int) -> String:
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var at := ids.find(id)
	if at < 0:
		return "unit %d" % id
	var index := 0
	for i in ids.size():
		if owners[i] == owners[at] and types[i] == types[at] and ids[i] < id:
			index += 1
	return "%d %s #%d" % [owners[at], types[at], index]


## Where the living units of every owner and type stand.
func _print_groups(tick: int) -> void:
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	var alive := host.unit_alive()
	var health := host.unit_health()
	var positions := host.current_positions()
	var groups: Dictionary[String, Array] = {}
	for i in ids.size():
		if alive[i] != 0:
			var key := "%d %s" % [owners[i], types[i]]
			if not groups.has(key):
				groups[key] = []
			# Place and hit points left in percent.
			groups[key].append("(%d, %d) %d%%" % [
				roundi(positions[i].x), roundi(positions[i].z), roundi(health[i] * 100.0)])
	var keys := groups.keys()
	keys.sort()
	var text := ""
	for key: String in keys:
		text += "\n    %s: %s" % [key, groups[key]]
	# Presentation only: how far the grass fire has come (16 m² per cell).
	var fire: Node = main.grass_fire
	if fire != null and fire.ignited > 0:
		text += "\n    grass fire: %d cells burning, %.2f ha burning or burnt" % [
			fire.burning(), fire.ignited * 16.0 / 10000.0]
	print("trailer-scout: %6.2f s  %s%s" % [float(tick) / TICK_HZ, _scout_counts, text])
	_scout_counts.clear()
