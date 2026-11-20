extends Node
## Vehicle tracks on every soft ground (prompt 104). Presentation only:
## nothing here is read back by the simulation, it is not part of the game
## state and may differ between computers; saved games and replays start
## without tracks.
##
## Why a second system next to the tall-grass raster (`grass_trails.gd`,
## 1 m per texel over the whole map): two ruts of a track are 0.3-0.7 m wide
## and need texels of 0.25-0.5 m. Over a 4 km map that would be 16 000
## texels per side. Instead every driven segment is kept as one stamp (a
## ring of CAPACITY instances of a 2D MultiMesh in world meters) and the
## stamps are drawn into a raster window around the camera focus
## (`fx_track_texels` × `fx_track_m` of the graphics quality, 512 m at
## high). The window follows the camera by changing the canvas transform and
## drawing again; tracks outside it are kept and come back when the camera
## returns. A stamp fades out over its lifetime in the stamp shader
## (`ground_track_stamp.gdshader`), so redrawing every FADE_REFRESH_S is
## enough between new stamps.
##
## Raster channels: R ruts (two bands per vehicle: tracks or wheels, gauge
## from the catalog width), G churned ground (turning, wending on the spot),
## B wear (adds up over passes: a column's way turns into a worn, darker
## path). The terrain shader (`terrain.gdshader`) draws them by the ground
## under them: dark ruts in mud, pressed snow, soft-edged ruts in sand, a
## dark streak in meadow; roads, rock and water show none.
##
## Fog of war (prompt 098): a vehicle stamps only while its position is
## visible to the local player (`effects.visible_at`), so the tracks never
## show the way of an enemy that drove unseen. Infantry leaves no tracks.

const StampShader := preload("res://assets/shaders/ground_track_stamp.gdshader")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

## Stamps kept (the oldest is overwritten); about 30 vehicles driving for
## five minutes.
const CAPACITY := 32768
## A vehicle stamps a segment after driving this far (m).
const STEP_M := 1.0
## ... or after turning this far on the spot (degrees).
const TURN_STEP_DEG := 12.0
## A longer jump between two frames is no drive (a unit came into view,
## was placed); it stamps nothing.
const MAX_SEGMENT_M := 6.0
## Per ground: band strength (0..1 before mass) and lifetime (game seconds).
## Mud and snow keep tracks longest, sand loses them first, a firm dirt
## track shows them faintly; roads, rock and water none.
const GROUNDS: Dictionary[String, Array] = {
	"grass": [0.55, 240.0],
	"track": [0.3, 240.0],
	"sand": [0.8, 150.0],
	"mud": [1.0, 480.0],
	"snow": [0.95, 600.0],
}
## Rain (terrain wetness at least WET): sand loses tracks faster, meadow and
## bare earth turn to mud under the tracks.
const WET := 0.3
const RAIN_SAND_LIFE := 0.35
const RAIN_GRASS: Array = [0.8, 360.0]
## Wear per stamp relative to the band strength (a worn path after about
## 1 / WEAR_SHARE passes).
const WEAR_SHARE := 0.1
## Churn: turning in a curve beyond CHURN_FROM_DEG_PER_M churns the ground;
## a turn on the spot churns SPOT_CHURN per TURN_STEP_DEG.
const CHURN_FROM_DEG_PER_M := 5.0
const CHURN_PER_DEG_PER_M := 0.04
const SPOT_CHURN := 0.3
## Band width of tracks relative to the vehicle width (clamped to
## TRACK_BAND_M), and of wheels (m); wheels sit inside WHEEL_GAUGE of the width.
const TRACK_BAND_SHARE := 0.17
const TRACK_BAND_M := Vector2(0.3, 0.75)
const WHEEL_BAND_M := 0.3
const WHEEL_GAUGE := 0.9
## Mass factor on the band strength: 0.55 + t / MASS_T, clamped.
const MASS_T := 45.0
const MASS_RANGE := Vector2(0.6, 1.15)
## The window is drawn around a new center once the focus is this share of
## its side away.
const RECENTER_SHARE := 0.2
## Seconds between redraws for the fading alone; new stamps redraw every
## REDRAW_FRAMES frames.
const FADE_REFRESH_S := 1.0
const REDRAW_FRAMES := 2
## Catalog categories driven on wheels.
const WHEELED: Array[String] = ["truck", "recon_vehicle"]

var main: Node3D
var sim_host: Object
## Stamps since the start (self-test, tools).
var stamps := 0
## Microseconds of the last frame (benchmark).
var process_usec := 0
## Window side (m) and meters per texel; 0: tracks off.
var window_m := 0.0
var texel_m := 0.0

var _viewport: SubViewport
var _multimesh: MultiMesh
var _material: ShaderMaterial
var _center := Vector2.INF
var _level := ""
var _next := 0
## CPU copy of the stamps for `amount_at`: center, angle, half size, birth,
## life, band strength.
var _centers := PackedVector2Array()
var _angles := PackedFloat32Array()
var _halves := PackedVector2Array()
var _births := PackedFloat32Array()
var _lives := PackedFloat32Array()
var _strengths := PackedFloat32Array()
## Unit id -> [last position, last heading (rad)].
var _last: Dictionary[int, Array] = {}
## Unit id -> vehicle info (see `_info`), {} for units without tracks.
var _infos: Dictionary[int, Dictionary] = {}
var _dirty := false
var _since_redraw := 0.0
var _frames_since_redraw := 0
var _frame := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_centers.resize(CAPACITY)
	_angles.resize(CAPACITY)
	_halves.resize(CAPACITY)
	_births.resize(CAPACITY)
	_lives.resize(CAPACITY)
	_strengths.resize(CAPACITY)
	_viewport = SubViewport.new()
	_viewport.name = "TrackRaster"
	_viewport.disable_3d = true
	_viewport.transparent_bg = true
	_viewport.render_target_clear_mode = SubViewport.CLEAR_MODE_ALWAYS
	_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	add_child(_viewport)
	_multimesh = MultiMesh.new()
	_multimesh.transform_format = MultiMesh.TRANSFORM_2D
	_multimesh.use_colors = true
	_multimesh.use_custom_data = true
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	_multimesh.mesh = quad
	_multimesh.instance_count = CAPACITY
	_multimesh.visible_instance_count = 0
	var instance := MultiMeshInstance2D.new()
	instance.multimesh = _multimesh
	_material = ShaderMaterial.new()
	_material.shader = StampShader
	instance.material = _material
	_viewport.add_child(instance)
	_configure()


## Game seconds now.
func now() -> float:
	return float(sim_host.current_tick()) / 20.0


## Sum of the band strengths of the live stamps whose rectangle covers
## `point` (self-test, tools; ignores the band profile).
func amount_at(point: Vector3) -> float:
	var p := Vector2(point.x, point.z)
	var t := now()
	var sum := 0.0
	for i in mini(stamps, CAPACITY):
		if t - _births[i] > _lives[i]:
			continue
		var local := (p - _centers[i]).rotated(-_angles[i])
		if absf(local.x) <= _halves[i].x and absf(local.y) <= _halves[i].y:
			sum += _strengths[i]
	return sum


## Ground of the tracks at a terrain id in the current biome: "grass",
## "track", "sand", "mud", "snow", or "" (no tracks: road, water).
func track_ground(terrain: int) -> String:
	var biome: String = main.effects._biome() if main.get("effects") != null else "forest"
	match terrain:
		0, 8:
			return {"desert": "sand", "winter": "snow"}.get(biome, "grass")
		1:
			return "snow" if biome == "winter" else "track"
		3:
			return "snow" if biome == "winter" else "sand"
		4:
			return "sand" if biome == "desert" else "mud"
		5:
			return "sand" if biome == "desert" else "snow"
	return ""


## Band strength and lifetime of the ground under `at`, [0, 0] for none;
## the weather counts (rain: shorter in sand, muddier meadow).
func ground_params(at: Vector3) -> Array:
	var ground := track_ground(int(sim_host.terrain_at(at.x, at.z)))
	if ground.is_empty():
		return [0.0, 0.0]
	var params: Array = GROUNDS[ground].duplicate()
	if _wet():
		if ground == "sand":
			params[1] *= RAIN_SAND_LIFE
		elif ground == "grass":
			params = RAIN_GRASS.duplicate()
	return params


func _wet() -> bool:
	var look: Node = main.get("weather_look")
	return look != null and look.wetness >= WET


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_process_tracks(delta)
	process_usec = Time.get_ticks_usec() - began


func _process_tracks(delta: float) -> void:
	_frame += 1
	_since_redraw += delta
	_frames_since_redraw += 1
	if _frame % 30 == 0:
		_configure()
	if window_m <= 0.0:
		return
	# Every other frame is enough for 1 m segments.
	if _frame % 2 == 0:
		_collect()
	var focus: Vector3 = main.camera_rig.focus if main.get("camera_rig") != null else Vector3.ZERO
	var f := Vector2(focus.x, focus.z)
	if _center == Vector2.INF or f.distance_to(_center) > window_m * RECENTER_SHARE:
		_place_window(f)
	if (_dirty and _frames_since_redraw >= REDRAW_FRAMES) or _since_redraw >= FADE_REFRESH_S:
		_redraw()


## Applies the graphics quality (size of the window; off at 0 texels).
func _configure() -> void:
	var quality: String = main.atmosphere.quality if main.get("atmosphere") != null else GraphicsQuality.DEFAULT
	if quality == _level:
		return
	_level = quality
	var settings := GraphicsQuality.settings(quality)
	var texels: int = settings.get("fx_track_texels", 0)
	texel_m = settings.get("fx_track_m", 0.5)
	window_m = texels * texel_m
	if texels <= 0:
		_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
		_set_terrain(null, Vector2.ZERO, Vector2.ONE)
		return
	_viewport.size = Vector2i(texels, texels)
	_set_terrain(_viewport.get_texture(), Vector2.ZERO, Vector2.ONE)
	_center = Vector2.INF


func _set_terrain(texture: Texture2D, origin: Vector2, size: Vector2) -> void:
	var terrain: Node = main.get("terrain")
	if terrain != null and terrain.has_method("set_ground_tracks"):
		terrain.set_ground_tracks(texture, origin, size)


## Centers the window on `focus` (snapped to 16 texels so the picture does
## not crawl) and draws it again.
func _place_window(focus: Vector2) -> void:
	var snap := texel_m * 16.0
	_center = (focus / snap).round() * snap
	var origin := _center - Vector2.ONE * window_m * 0.5
	_viewport.canvas_transform = Transform2D(0.0, Vector2.ONE / texel_m, 0.0, -origin / texel_m)
	_set_terrain(_viewport.get_texture(), origin, Vector2.ONE * window_m)
	_redraw()


func _redraw() -> void:
	_material.set_shader_parameter("now", now())
	_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	_dirty = false
	_since_redraw = 0.0
	_frames_since_redraw = 0


func _collect() -> void:
	var ids: PackedInt64Array = sim_host.unit_ids()
	var alive: PackedByteArray = sim_host.unit_alive()
	var types := PackedStringArray()
	for i in ids.size():
		var id := ids[i]
		if not _infos.has(id):
			if types.is_empty():
				types = sim_host.unit_type_ids()
			_infos[id] = _info(types[i])
		var info: Dictionary = _infos[id]
		if info.is_empty() or i >= alive.size() or alive[i] == 0:
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		# Models face -Z.
		var forward := -node.global_transform.basis.z
		var at := node.global_position
		step(id, at, atan2(forward.x, forward.z), info, main.effects.visible_at.call(at))


## Tracks of one vehicle at `at` with heading `heading` (rad, of its -Z):
## stamps the segment since its last stamp, or the churn of a turn on the
## spot. `visible`: whether the local player sees it now; unseen vehicles
## stamp nothing and start afresh when seen again.
func step(id: int, at: Vector3, heading: float, info: Dictionary, visible: bool) -> void:
	var last: Array = _last.get(id, [])
	if last.is_empty() or not visible:
		_last[id] = [at, heading]
		return
	var from: Vector3 = last[0]
	var segment := Vector2(at.x - from.x, at.z - from.z)
	var length := segment.length()
	var turn := rad_to_deg(absf(angle_difference(heading, last[1])))
	if length > MAX_SEGMENT_M:
		_last[id] = [at, heading]
		return
	if length < STEP_M and turn < TURN_STEP_DEG:
		return
	_last[id] = [at, heading]
	var params := ground_params(at)
	if params[0] <= 0.0:
		return
	var strength: float = minf(params[0] * info.mass, 1.0)
	if length < STEP_M:
		# Turning on the spot: a churned patch the size of the vehicle.
		var spot := Color(0.0, SPOT_CHURN * params[0] * turn / TURN_STEP_DEG, strength * WEAR_SHARE)
		_stamp(Vector2(at.x, at.z), heading, Vector2(info.length, info.width), spot, params[1], info)
		return
	var churn: float = clampf((turn / length - CHURN_FROM_DEG_PER_M) * CHURN_PER_DEG_PER_M, 0.0, 0.6) * params[0]
	var center := Vector2(from.x + at.x, from.z + at.z) * 0.5
	_stamp(center, segment.angle(), Vector2(length, info.width), Color(strength, churn, strength * WEAR_SHARE),
		params[1], info)


func _stamp(center: Vector2, angle: float, size: Vector2, strength: Color, life: float, info: Dictionary) -> void:
	var i := _next
	_next = (_next + 1) % CAPACITY
	var birth := now()
	_multimesh.set_instance_transform_2d(i, Transform2D(angle, size, 0.0, center))
	_multimesh.set_instance_color(i, Color(strength.r, strength.g, strength.b, 1.0))
	_multimesh.set_instance_custom_data(i, Color(birth, life, info.band_center, info.band_half))
	_centers[i] = center
	_angles[i] = angle
	_halves[i] = size * 0.5
	_births[i] = birth
	_lives[i] = life
	_strengths[i] = strength.r
	stamps += 1
	_multimesh.visible_instance_count = mini(stamps, CAPACITY)
	_dirty = true


## Width, length, mass factor and band layout of a unit type from the
## catalog; {} for units without tracks (infantry, aircraft, buildings).
func _info(type_id: String) -> Dictionary:
	var card: Dictionary = SimHost.catalog_card(type_id)
	if not card.get("group", "") in ["vehicle", "gun"]:
		return {}
	var width: float = card.get("width_m", 0.0)
	var length: float = card.get("length_m", 0.0)
	if width <= 0.0:
		width = 2.5
	if length <= 0.0:
		length = 5.0
	var wheels: bool = card.get("group", "") == "gun" or card.get("category", "") in WHEELED
	var outer := width * WHEEL_GAUGE if wheels else width
	var band := WHEEL_BAND_M if wheels else clampf(width * TRACK_BAND_SHARE, TRACK_BAND_M.x, TRACK_BAND_M.y)
	# Across the quad: 0 is the middle, 1 the outer edge.
	var half := band / outer
	return {
		"width": outer, "length": length,
		"mass": clampf(0.55 + float(card.get("mass_t", 10.0)) / MASS_T, MASS_RANGE.x, MASS_RANGE.y),
		"band_center": 1.0 - half, "band_half": half,
	}
