extends Node3D
## A building that collapses storey by storey (prompt 197). Presentation
## only: the simulation made the object a ruin at one tick; this draws the
## fall in slow motion from the top down, with the ruin already standing
## inside (map_objects.gd draws it next to this node).
##
## The model is the building as it stood (intact or damaged variant), a
## little wider than the ruin so their walls do not flicker. Its height is
## cut into bands (`bands_of`): the roof from the eaves up, then the
## storeys from the top down. Each band collapses in turn in the shader
## (`env_collapse.gdshaderinc`, instance uniforms `env_collapse` /
## `env_collapse_at`): walls tip in or out, sag, drop into the storey below
## and crumble; a short hesitation follows. Chunks (a MultiMesh of boxes on
## precomputed arcs, no physics) fall off the outside of every storey, and
## `on_stage` lets effects.gd raise dust and debris at that height, so the
## cloud grows from the top down. Durations, tip angles and chunks come
## from the object index and the shot direction: a replay looks the same.
##
## Quality (graphics_quality.gd): `fx_collapse_chunks` chunks per storey
## (low 0), `fx_collapse_tip` (low: the bands only sink).

## Storey height (m) the bands are cut to: the blueprints use 2.6-3.2 m
## (`storey_h`), barns 4.5 m; the band count is rounded, so the bands
## match the storeys of the model.
const STOREY_M := 3.0
## A roof band lower than this (flat roofs, parapets) falls with the top
## storey.
const MIN_ROOF_M := 0.35
## Seconds per band (multi-storey: roof, storey), single-storey buildings,
## hesitation between bands, and the moment of the hit before the first.
const ROOF_SECONDS := Vector2(0.75, 1.0)
const STOREY_SECONDS := Vector2(0.85, 1.2)
const SINGLE_SECONDS := Vector2(0.6, 0.8)
const PAUSE_SECONDS := Vector2(0.25, 0.5)
const HIT_SECONDS := 0.12
## Tip angle of the walls (radians) and of the roof (always inward).
const WALL_TIP := 0.75
const ROOF_TIP := 0.3
## Chunks lie this long after the last band, then sink into the rubble.
const CHUNK_REST_SECONDS := 4.0
const CHUNK_SINK_SECONDS := 1.5
const GRAVITY := 9.81

## Called when a band starts: (stage: Dictionary, last: bool), stage has
## name, bottom (world y), height, center (world), half (half extents xz).
var on_stage: Callable
## Names of the bands started so far, in order ("roof", "storey_3" ...).
var started: Array[String] = []
## All bands done, the model gone (chunks may still lie).
var finished := false
## Bands: [{name, bottom, height, start, seconds, tip}] in world meters.
var stages: Array[Dictionary] = []

## Microseconds spent in `_process` of all collapses since the start
## (collapse-shots bench).
static var usec_total := 0

var _model: Node3D
var _meshes: Array[GeometryInstance3D] = []
var _time := 0.0
var _current := -1
var _center := Vector3.ZERO
## Turn of the building (its box of walls turns with it).
var _basis := Basis.IDENTITY
var _half := Vector2.ONE
var _direction := Vector2(0.0, -1.0)
var _seed := 0
var _chunks_per_band := 0
var _chunk_node: MultiMeshInstance3D
## Per chunk: {from, velocity, spin_axis, spin, size, start, land_y, color}.
var _chunks: Array[Dictionary] = []
var _height_at: Callable
var _end := 0.0


## Bands of a model (local meters, model space y): `parts` as
## map_objects.gd `_parts` gives them. Floor top = foot of the walls,
## lowest roof point = eaves; [{name, bottom, height}] from the top down.
static func bands_of(parts: Array) -> Array[Dictionary]:
	var floor_top := INF
	var walls_low := INF
	var eave := INF
	var top := -INF
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for part: Dictionary in parts:
		if part.lod == "far" or part.lod == "shadow" or part.mesh == null:
			continue
		var box: AABB = (part.transform as Transform3D) * (part.mesh as Mesh).get_aabb()
		var name := str(part.name)
		top = maxf(top, box.end.y)
		if name.ends_with("Floor"):
			floor_top = minf(floor_top, box.end.y)
		elif name.ends_with("Roof"):
			eave = minf(eave, box.position.y)
		elif name.ends_with("Walls"):
			walls_low = minf(walls_low, box.position.y)
			lo = Vector2(minf(lo.x, box.position.x), minf(lo.y, box.position.z))
			hi = Vector2(maxf(hi.x, box.end.x), maxf(hi.y, box.end.z))
	if top == -INF:
		return []
	var base := floor_top if floor_top < INF else (walls_low if walls_low < INF else 0.0)
	# A floor slab that reaches high (a damaged interior) is no foot.
	if base > 1.5:
		base = walls_low if walls_low < INF else 0.0
	var eaves := eave if eave < INF else top
	var bands: Array[Dictionary] = []
	var roof_h := top - eaves
	if roof_h < MIN_ROOF_M:
		eaves = top
	var count := maxi(1, roundi((eaves - base) / STOREY_M))
	var h := (eaves - base) / count
	if eaves < top:
		bands.append({"name": "roof", "bottom": eaves, "height": top - eaves})
	for k in range(count - 1, -1, -1):
		bands.append({"name": "storey_%d" % (k + 1), "bottom": base + k * h, "height": h})
	for band in bands:
		band["half"] = (hi - lo) * 0.5 if lo.x < INF else Vector2(4.0, 4.0)
	return bands


## Starts the collapse of `model` (a variant node placed at `transform`)
## with the local `bands` of `bands_of`. `index`, `fall_deg`: object index
## and shot heading (simulation degrees, 0 = north = -Z). `height_at`:
## ground height (x, z) for the chunks.
func setup(model: Node3D, transform: Transform3D, bands: Array, index: int, fall_deg: int,
		quality: Dictionary, height_at: Callable) -> void:
	_model = model
	model.name = "Model"
	_height_at = height_at
	_seed = index * 7919 + fall_deg * 31
	var heading := deg_to_rad(float(fall_deg))
	_direction = Vector2(sin(heading), -cos(heading))
	_center = transform.origin
	_basis = transform.basis.orthonormalized()
	_chunks_per_band = int(quality.get("fx_collapse_chunks", 0))
	var tip_on: bool = quality.get("fx_collapse_tip", true)
	# A little wider than the ruin standing inside it.
	model.transform = Transform3D(transform.basis * Basis.from_scale(Vector3(1.012, 1.0, 1.012)), transform.origin)
	add_child(model)
	for node in model.find_children("*", "GeometryInstance3D", true, false):
		_meshes.append(node as GeometryInstance3D)
	var scale_y := transform.basis.y.length()
	var single := bands.size() <= 2
	var t := HIT_SECONDS
	for i in bands.size():
		var band: Dictionary = bands[i]
		var roof: bool = band.name == "roof"
		var range_s: Vector2 = SINGLE_SECONDS if single else (ROOF_SECONDS if roof else STOREY_SECONDS)
		var seconds := lerpf(range_s.x, range_s.y, _hash(i, 1))
		var tip := 0.0
		if tip_on:
			tip = -ROOF_TIP if roof else WALL_TIP * (0.8 + 0.4 * _hash(i, 2))
		stages.append({
			"name": band.name, "bottom": _center.y + float(band.bottom) * scale_y,
			"height": float(band.height) * scale_y, "start": t, "seconds": seconds, "tip": tip,
			"half": band.half,
		})
		t += seconds + lerpf(PAUSE_SECONDS.x, PAUSE_SECONDS.y, _hash(i, 3))
	_end = stages[-1].start + stages[-1].seconds if not stages.is_empty() else 0.0
	if _chunks_per_band > 0:
		_build_chunk_node()
	_apply(0)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_time += delta
	if not finished:
		var next := _current + 1
		if next < stages.size() and _time >= float(stages[next].start):
			_begin(next)
		if _current >= 0:
			var stage: Dictionary = stages[_current]
			_apply(_current, clampf((_time - float(stage.start)) / float(stage.seconds), 0.0, 1.0))
		if _time >= _end:
			_finish()
	_move_chunks()
	usec_total += Time.get_ticks_usec() - began
	if finished and _time >= _end + CHUNK_REST_SECONDS + CHUNK_SINK_SECONDS:
		queue_free()


func _begin(i: int) -> void:
	_current = i
	var stage: Dictionary = stages[i]
	started.append(stage.name)
	if on_stage.is_valid():
		var top := Vector3(_center.x, float(stage.bottom) + float(stage.height), _center.z)
		on_stage.call({"name": stage.name, "bottom": stage.bottom, "height": stage.height,
			"center": top, "half": stage.half}, i == stages.size() - 1)
	for k in _chunks_per_band:
		_chunks.append(_chunk(i, k, stage))


func _finish() -> void:
	if finished:
		return
	finished = true
	if _model != null:
		_model.queue_free()
		_model = null
	_meshes.clear()


func _apply(i: int, progress := 0.0) -> void:
	var stage: Dictionary = stages[i]
	var band := Vector4(stage.bottom, stage.height, progress, stage.tip)
	var at := Vector4(_center.x, _center.z, _direction.x, _direction.y)
	for mesh in _meshes:
		mesh.set_instance_shader_parameter(&"env_collapse", band)
		mesh.set_instance_shader_parameter(&"env_collapse_at", at)


# --- Chunks -------------------------------------------------------------------

## Chunk colors: plaster, brick, stone, dark rubble.
const CHUNK_COLORS: Array[Color] = [
	Color(0.46, 0.43, 0.37), Color(0.36, 0.19, 0.13), Color(0.34, 0.32, 0.29), Color(0.20, 0.18, 0.16),
]


func _build_chunk_node() -> void:
	var box := BoxMesh.new()
	box.size = Vector3.ONE
	var material := StandardMaterial3D.new()
	material.vertex_color_use_as_albedo = true
	material.roughness = 0.95
	box.material = material
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = box
	mm.instance_count = _chunks_per_band * stages.size()
	mm.visible_instance_count = 0
	_chunk_node = MultiMeshInstance3D.new()
	_chunk_node.name = "Chunks"
	_chunk_node.multimesh = mm
	_chunk_node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	add_child(_chunk_node)


## Chunk `k` of band `i`: leaves a point on the outside of the wall at the
## band's height, flies outward and lands on the ground beside the house.
func _chunk(i: int, k: int, stage: Dictionary) -> Dictionary:
	var half: Vector2 = stage.half
	var a := _hash(i * 37 + k, 11) * TAU
	var out := Vector2(cos(a), sin(a))
	# A point on the box of the walls in that direction.
	var reach := minf(half.x / maxf(absf(out.x), 1e-3), half.y / maxf(absf(out.y), 1e-3))
	var side := _basis * Vector3(out.x * reach, 0.0, out.y * reach)
	var dir := Vector3(side.x, 0.0, side.z).normalized()
	var from := Vector3(_center.x, float(stage.bottom), _center.z) + side
	from.y += float(stage.height) * (0.3 + 0.7 * _hash(i * 37 + k, 12))
	var speed := 1.2 + 2.8 * _hash(i * 37 + k, 13)
	var velocity := dir * speed + Vector3.UP * (0.5 + 1.5 * _hash(i * 37 + k, 14))
	var land_y := from.y - 30.0
	if _height_at.is_valid():
		var flat := from + dir * speed * 1.2
		land_y = float(_height_at.call(flat.x, flat.z))
	var size := 0.25 + 0.6 * _hash(i * 37 + k, 15)
	return {
		"from": from, "velocity": velocity, "size": Vector3(size, size * (0.5 + 0.5 * _hash(k, 16)), size * 0.8),
		"axis": Vector3(_hash(k, 17) - 0.5, 0.4, _hash(k, 18) - 0.5).normalized(),
		"spin": 2.0 + 6.0 * _hash(k, 19),
		"start": float(stage.start) + float(stage.seconds) * 0.25 * _hash(k, 20),
		"land_y": land_y + size * 0.3, "color": CHUNK_COLORS[(k + i) % CHUNK_COLORS.size()],
	}


func _move_chunks() -> void:
	if _chunk_node == null:
		return
	var mm := _chunk_node.multimesh
	var shown := 0
	var sink := clampf((_time - _end - CHUNK_REST_SECONDS) / CHUNK_SINK_SECONDS, 0.0, 1.0)
	for c: Dictionary in _chunks:
		var t := _time - float(c.start)
		if t < 0.0:
			continue
		var from: Vector3 = c.from
		var v: Vector3 = c.velocity
		# The landing time on the ground (precomputed arc, no physics).
		var drop := from.y - float(c.land_y)
		var land := (v.y + sqrt(maxf(v.y * v.y + 2.0 * GRAVITY * drop, 0.0))) / GRAVITY
		var tt := minf(t, land)
		var p := from + v * tt + Vector3.DOWN * 0.5 * GRAVITY * tt * tt
		p.y -= sink * float(c.size.y) * 1.2
		var spin := Basis(c.axis, float(c.spin) * tt)
		mm.set_instance_transform(shown, Transform3D(spin * Basis.from_scale(c.size), p))
		mm.set_instance_color(shown, c.color)
		shown += 1
	mm.visible_instance_count = shown


## A value 0..1 from the object, the shot direction, `i` and `salt`
## (deterministic: the same building falls the same way in a replay).
func _hash(i: int, salt: int) -> float:
	var h := (_seed * 73856093) ^ (i * 19349663) ^ (salt * 83492791)
	h = (h ^ (h >> 13)) * 1274126177
	h = h ^ (h >> 16)
	return float(h & 0xffff) / 65535.0
