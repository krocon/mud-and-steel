extends Control
## Supply and repair radii (prompt 036): for every selected unit that
## resupplies or repairs (headquarters, depots, vehicle works, trucks,
## engineers of the local player or an ally) a ring on the ground, drawn on
## the HUD like the waypoint lines: amber for resupply, blue for repair.
## The ground height is the unit's own, so on slopes the ring floats or
## sinks a little.

## Selected units looked at, at most.
const MAX_UNITS := 24
const SEGMENTS := 48
## Meters above the ground.
const LIFT := 0.5
const SUPPLY_COLOR := Color(1.0, 0.78, 0.3, 0.7)
const REPAIR_COLOR := Color(0.45, 0.75, 1.0, 0.7)

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_draw` in microseconds (hud.gd).
var usec := 0
## Rings drawn in the last frame (self-test).
var rings := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "SupplyRings"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	var began := Time.get_ticks_usec()
	rings = 0
	var selected: Array[int] = main.selection.selected
	var camera: Camera3D = main.camera
	for id: int in selected.slice(0, MAX_UNITS):
		var radii := sim_host.supply_radius(id)
		if radii.size() != 2:
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var center := node.global_position + Vector3.UP * LIFT
		if radii[0] > 0.0:
			_ring(camera, center, radii[0], SUPPLY_COLOR)
		if radii[1] > 0.0:
			_ring(camera, center, radii[1], REPAIR_COLOR)
	usec += Time.get_ticks_usec() - began


func _ring(camera: Camera3D, center: Vector3, radius: float, color: Color) -> void:
	var points := PackedVector2Array()
	for k in SEGMENTS + 1:
		var angle := TAU * k / SEGMENTS
		var p := center + Vector3(cos(angle), 0.0, sin(angle)) * radius
		if camera.is_position_behind(p):
			return
		points.append(camera.unproject_position(p))
	draw_polyline(points, color, 2.0, true)
	rings += 1
