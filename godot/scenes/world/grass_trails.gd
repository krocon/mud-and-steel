extends Node
## Tracks in tall grass (prompt 073). Presentation only: nothing here is
## read back or reaches the simulation.
##
## A raster over the whole map (1 m per texel, coarser on maps larger than
## 2 km) in which vehicles stamp their footprint as they drive (R: pressed
## flat) and scorch marks of blasts their size (G: burnt). It is a 2D
## SubViewport that is never cleared: every frame with new stamps draws only
## those, additively, onto what is there, so the tracks stay. The tall
## grass plants (tall_grass.gdshader) and the terrain's look of tall grass
## read it through `tall_grass_common.gdshaderinc`.
##
## Built only on maps with tall grass. Like the tracks on other ground
## (`ground_tracks.gd`, prompt 104) only vehicles the local player sees
## press the grass, so it never shows where an enemy drove unseen.

const TallGrass := preload("res://scenes/world/tall_grass.gd")
## Terrain id of tall grass (data/terrain.toml).
const TALL_GRASS := 8
## Largest raster side in texels.
const MAX_TEXELS := 2048
## A vehicle stamps again after moving this far (m).
const STAMP_STEP_M := 0.4
## Strength of one stamp (additive; a vehicle overlaps many).
const STAMP_STRENGTH := 0.2
## Width of a vehicle's footprint relative to its length.
const WIDTH_SHARE := 0.48

var main: Node3D
var sim_host: Object
## Stamps drawn since the start (self-test, tools).
var stamps := 0

var _viewport: SubViewport
var _painter: Node2D
var _origin := Vector2.ZERO
var _texel_m := 1.0
## Unit id -> last stamped position.
var _last: Dictionary[int, Vector3] = {}
var _infantry: Dictionary[int, bool] = {}
## Pending stamps: [kind ("track" or "burn"), center (texels), angle, size (texels)].
var _pending: Array = []
var _frame := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	if int(sim_host.terrain_count(TALL_GRASS)) == 0:
		set_process(false)
		return
	var size: Vector2 = sim_host.map_size()
	_texel_m = maxf(1.0, maxf(size.x, size.y) / MAX_TEXELS)
	_origin = -size * 0.5
	_viewport = SubViewport.new()
	_viewport.name = "TrailRaster"
	_viewport.size = Vector2i(ceili(size.x / _texel_m), ceili(size.y / _texel_m))
	_viewport.disable_3d = true
	_viewport.transparent_bg = true
	_viewport.render_target_clear_mode = SubViewport.CLEAR_MODE_ONCE
	_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(_viewport)
	_painter = Node2D.new()
	var additive := CanvasItemMaterial.new()
	additive.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD
	_painter.material = additive
	_painter.draw.connect(_draw_stamps)
	_viewport.add_child(_painter)
	var texture := _viewport.get_texture()
	TallGrass.MATERIAL.set_shader_parameter("trail_tex", texture)
	TallGrass.MATERIAL.set_shader_parameter("trail_origin", _origin)
	TallGrass.MATERIAL.set_shader_parameter("trail_size", size)
	if main.terrain.has_method("set_trails"):
		main.terrain.set_trails(texture, _origin, size)


func _exit_tree() -> void:
	# The plants' material is shared: no tracks of this match in the next.
	if _viewport != null:
		TallGrass.MATERIAL.set_shader_parameter("trail_tex", null)


## Burns tall grass in a circle (scorch mark of a blast).
func burn(at: Vector3, radius: float) -> void:
	if _viewport == null:
		return
	_pending.append(["burn", _texel(at), 0.0, Vector2.ONE * radius / _texel_m])


## Presses tall grass flat in a rectangle (`length` along `heading`, the
## direction in the x-z plane), for tools and tests.
func press(at: Vector3, heading: Vector2, length: float, width: float) -> void:
	if _viewport == null:
		return
	_pending.append(["track", _texel(at), heading.angle(), Vector2(length, width) / _texel_m])


func _texel(at: Vector3) -> Vector2:
	return (Vector2(at.x, at.z) - _origin) / _texel_m


func _process(_delta: float) -> void:
	_frame += 1
	# Vehicles: every other frame is enough at 0.4 m steps.
	if _frame % 2 == 0:
		_collect_vehicles()
	if _pending.is_empty():
		return
	_painter.queue_redraw()
	_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE


func _collect_vehicles() -> void:
	var ids: PackedInt64Array = sim_host.unit_ids()
	var alive: PackedByteArray = sim_host.unit_alive()
	var footprints := PackedFloat32Array()
	var types := PackedStringArray()
	for i in ids.size():
		var id := ids[i]
		if not _infantry.has(id):
			if types.is_empty():
				types = sim_host.unit_type_ids()
			_infantry[id] = types[i].contains("_inf_")
		if _infantry[id] or i >= alive.size() or alive[i] == 0:
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var at := node.global_position
		if not main.effects.visible_at.call(at):
			_last[id] = Vector3.INF
			continue
		var last: Vector3 = _last.get(id, Vector3.INF)
		if last != Vector3.INF and Vector2(at.x - last.x, at.z - last.z).length() < STAMP_STEP_M:
			continue
		_last[id] = at
		if last == Vector3.INF:
			# Where a vehicle stands at the start it has not driven yet.
			continue
		if footprints.is_empty():
			footprints = sim_host.unit_footprints()
		var length: float = footprints[i] if i < footprints.size() else 5.0
		# Models face -Z.
		var forward := -node.global_transform.basis.z
		press(at, Vector2(forward.x, forward.z), length, length * WIDTH_SHARE)


func _draw_stamps() -> void:
	for stamp: Array in _pending:
		var center: Vector2 = stamp[1]
		var size: Vector2 = stamp[3]
		if stamp[0] == "burn":
			_painter.draw_set_transform(center, 0.0, Vector2.ONE)
			_painter.draw_circle(Vector2.ZERO, size.x, Color(0.0, 1.0, 0.0, 1.0))
		else:
			_painter.draw_set_transform(center, stamp[2], Vector2.ONE)
			_painter.draw_rect(Rect2(-size * 0.5, size), Color(1.0, 0.0, 0.0, STAMP_STRENGTH))
		stamps += 1
	_pending.clear()
