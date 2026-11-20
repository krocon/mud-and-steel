extends Control
## Target preview of a formation move (prompt 099): while the right button
## is held for a move, every selected unit gets a flat mark on the ground at
## the slot it will take, turned to the facing of the drag. After the order
## went out (release, click, M / Y / I) the marks of that order stay for
## `HOLD_SECONDS` and fade. A slot the unit cannot stand on (water, house,
## rock, off the map) is drawn in `BLOCKED_COLOR`; the simulation then stops
## the unit at the nearest reachable spot.
##
## The slots come from `SimHost.formation_preview`, the same simulation code
## the order uses: no second layout in GDScript. They are fetched at most
## once per frame and only when the target or the facing changed.
##
## Gun teams: a tractor towing a gun shows the gun's range circle at its
## slot (dashed and pale: limbered, the gun cannot fire).
##
## Drawn on the HUD like `waypoints.gd`, so above the fog layer.

const RangeRings := preload("res://scenes/hud/range_rings.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")

## Marks drawn at most (the selection can hold more units).
const MAX_MARKS := 128
## Seconds the marks of a sent order stay, and of them the fade at the end.
const HOLD_SECONDS := 1.0
const FADE_SECONDS := 0.4
## Meters above the ground.
const LIFT := 0.4
## Half size of a vehicle mark (across, along the facing) and the radius of
## a squad mark, in meters.
const VEHICLE_HALF := Vector2(1.6, 3.0)
const SQUAD_RADIUS := 2.4
const FILL_ALPHA := 0.28
const LINE_ALPHA := 0.9
const LINE_WIDTH := 2.0
const BLOCKED_COLOR := Color(0.62, 0.62, 0.62)
## Target or facing must move this far (m) before the slots are fetched again.
const REFETCH_M := 0.25

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_draw` in microseconds (hud.gd).
var usec := 0
## Marks drawn in the last frame (self-test).
var marks := 0
## Gun circles drawn at slots in the last frame (self-test).
var gun_rings := 0
## Bridge calls for slots so far (self-test: at most one per change).
var fetches := 0
## Slots shown: four floats per unit (id, x, z, blocked).
var slots := PackedFloat32Array()
var _facing := Vector3.ZERO
var _key := []
## Seconds since the order went out; < 0 while the button is held.
var _age := -1.0
var _held := false
## Per unit id: true for vehicles (bigger rectangular mark).
var _vehicle := {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "OrderPreview"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	main.selection.formation_ordered.connect(_on_ordered)


func _process(delta: float) -> void:
	var pending: Array = main.selection.pending_move()
	if not pending.is_empty():
		_held = true
		_age = -1.0
		_fetch(pending[0], pending[1], main.selection.selected)
	elif _held:
		# Released without an order (the selection's click went elsewhere).
		_held = false
		if _age < 0.0:
			slots = PackedFloat32Array()
	elif _age >= 0.0:
		# During the tactical pause (prompt 102) the order waits: so do its marks.
		if not get_tree().paused:
			_age += delta
		if _age > HOLD_SECONDS:
			_age = -1.0
			slots = PackedFloat32Array()
			_key = []
	if not slots.is_empty() or marks > 0:
		queue_redraw()


## Whether marks are shown now (self-test).
func showing() -> bool:
	return not slots.is_empty()


func _on_ordered(point: Vector3, facing: Vector3, ids: Array) -> void:
	_key = []
	_fetch(point, facing, ids)
	_held = false
	_age = 0.0


func _fetch(point: Vector3, facing: Vector3, ids: Array) -> void:
	var key := [snappedf(point.x, REFETCH_M), snappedf(point.z, REFETCH_M),
		snappedf(facing.x, REFETCH_M), snappedf(facing.z, REFETCH_M), ids.size(),
		ids.hash(), main.selection.formation]
	if key == _key:
		return
	_key = key
	_facing = facing
	slots = sim_host.formation_preview(PackedInt64Array(ids), point.x, point.z,
		main.selection.formation, facing.x, facing.z)
	fetches += 1


func _draw() -> void:
	var began := Time.get_ticks_usec()
	marks = 0
	gun_rings = 0
	if slots.is_empty():
		usec += Time.get_ticks_usec() - began
		return
	var alpha := 1.0
	if _age > HOLD_SECONDS - FADE_SECONDS:
		alpha = clampf((HOLD_SECONDS - _age) / FADE_SECONDS, 0.0, 1.0)
	var camera: Camera3D = main.camera
	var own := RelationColors.own()
	var forward := _forward()
	var ids := sim_host.unit_ids()
	var towing := sim_host.unit_towing()
	var count := mini(slots.size() / 4, MAX_MARKS)
	for k in count:
		var id := int(slots[4 * k])
		var at := Vector3(slots[4 * k + 1], 0.0, slots[4 * k + 2])
		at.y = sim_host.height_at(at.x, at.z) + LIFT
		var color := BLOCKED_COLOR if slots[4 * k + 3] > 0.5 else own
		if _is_vehicle(id):
			_rect_mark(camera, at, forward, color, alpha)
		else:
			_round_mark(camera, at, color, alpha)
		var index := ids.find(id)
		if index >= 0 and index < towing.size() and towing[index] >= 0:
			gun_rings += RangeRings.draw_unit_rings(self, camera, sim_host, towing[index], at,
				RangeRings.STYLE_TOWED, alpha)
	usec += Time.get_ticks_usec() - began


## Facing of the marks on the ground: the drag, else from the group's
## center to the target (the simulation's default), else north.
func _forward() -> Vector3:
	var f := Vector3(_facing.x, 0.0, _facing.z)
	if f.length() > 0.01:
		return f.normalized()
	var center := Vector3.ZERO
	var target := Vector3.ZERO
	var n := 0
	for k in slots.size() / 4:
		var node: Node3D = main._unit_nodes.get(int(slots[4 * k]))
		if node != null:
			center += node.global_position
			n += 1
		target += Vector3(slots[4 * k + 1], 0.0, slots[4 * k + 2])
	if n > 0:
		f = target / (slots.size() / 4) - center / n
		f.y = 0.0
	return f.normalized() if f.length() > 0.01 else Vector3(0, 0, -1)


func _is_vehicle(id: int) -> bool:
	if not _vehicle.has(id):
		var info := sim_host.unit_info(id)
		_vehicle[id] = int(info.get("squad_size", 0)) == 0
	return _vehicle[id]


func _rect_mark(camera: Camera3D, at: Vector3, forward: Vector3, color: Color, alpha: float) -> void:
	var right := Vector3(-forward.z, 0.0, forward.x)
	var corners := [
		at + right * VEHICLE_HALF.x + forward * VEHICLE_HALF.y,
		at - right * VEHICLE_HALF.x + forward * VEHICLE_HALF.y,
		at - right * VEHICLE_HALF.x - forward * VEHICLE_HALF.y,
		at + right * VEHICLE_HALF.x - forward * VEHICLE_HALF.y,
	]
	var points := PackedVector2Array()
	for c: Vector3 in corners:
		if camera.is_position_behind(c):
			return
		points.append(camera.unproject_position(c))
	draw_colored_polygon(points, Color(color, FILL_ALPHA * alpha))
	points.append(points[0])
	draw_polyline(points, Color(color, LINE_ALPHA * alpha), LINE_WIDTH, true)
	# The front edge marks the facing.
	var nose := camera.unproject_position(at + forward * (VEHICLE_HALF.y + 1.2))
	draw_line((points[0] + points[1]) * 0.5, nose, Color(color, LINE_ALPHA * alpha), LINE_WIDTH, true)
	marks += 1


func _round_mark(camera: Camera3D, at: Vector3, color: Color, alpha: float) -> void:
	var points := PackedVector2Array()
	for k in 13:
		var angle := TAU * k / 12.0
		var p := at + Vector3(cos(angle), 0.0, sin(angle)) * SQUAD_RADIUS
		if camera.is_position_behind(p):
			return
		points.append(camera.unproject_position(p))
	draw_colored_polygon(points.slice(0, 12), Color(color, FILL_ALPHA * alpha))
	draw_polyline(points, Color(color, LINE_ALPHA * alpha), LINE_WIDTH, true)
	marks += 1
