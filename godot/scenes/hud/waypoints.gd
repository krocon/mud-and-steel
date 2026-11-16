extends Control
## Routes and waypoint chains of the selected units (prompt 031), drawn on
## the HUD: a line from the unit through its remaining route to every
## queued waypoint, a marker at each queued waypoint, patrols in blue with
## the closing leg back to the first waypoint. Secured areas (prompt 046)
## show as a red circle, routes of an attack-move in orange.
##
## The points are projected to the screen; a straight segment stays straight
## under the perspective projection, so this is exact for segments in front
## of the camera.

## Selected units drawn at most.
const MAX_UNITS := 48
## Meters above the ground.
const LIFT := 0.6
const ROUTE_COLOR := Color(0.5, 1.0, 0.5, 0.55)
const QUEUE_COLOR := Color(1.0, 0.9, 0.35, 0.85)
const PATROL_COLOR := Color(0.45, 0.75, 1.0, 0.85)
const ATTACK_MOVE_COLOR := Color(1.0, 0.6, 0.3, 0.7)
const SECURE_COLOR := Color(1.0, 0.4, 0.3, 0.8)

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_draw` in microseconds (hud.gd).
var usec := 0
## Queued waypoint markers drawn in the last frame (self-test).
var markers := 0
## Secured areas drawn in the last frame (self-test).
var secured := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "Waypoints"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	var began := Time.get_ticks_usec()
	markers = 0
	secured = 0
	var selected: Array[int] = main.selection.selected
	if selected.is_empty():
		usec += Time.get_ticks_usec() - began
		return
	var camera: Camera3D = main.camera
	var areas: Array[Vector4] = []
	for id: int in selected.slice(0, MAX_UNITS):
		var drill := sim_host.unit_drill(id)
		if drill.x == 3.0 and not areas.has(drill):
			areas.append(drill)
			_circle(camera, Vector3(drill.y, 0.0, drill.z), drill.w)
		var points := sim_host.unit_waypoints(id)
		if points.is_empty():
			continue
		var node: Node3D = main._unit_nodes.get(id)
		var start: Vector3 = node.global_position if node != null else points[0]
		var queued := sim_host.unit_queue_length(id)
		var patrol := sim_host.unit_patrols(id)
		var first_queued := points.size() - queued
		var previous := start
		for k in points.size():
			var point := points[k]
			var route := ATTACK_MOVE_COLOR if drill.x == 1.0 else ROUTE_COLOR
			var color := route if k < first_queued else (PATROL_COLOR if patrol else QUEUE_COLOR)
			_segment(camera, previous, point, color)
			previous = point
		for k in range(maxi(first_queued - 1, 0), points.size()):
			# The current target (end of the route) gets a marker too.
			if k == first_queued - 1 and queued == 0:
				continue
			_marker(camera, points[k], PATROL_COLOR if patrol else QUEUE_COLOR)
		if patrol and queued > 0:
			_segment(camera, points[points.size() - 1], points[first_queued], PATROL_COLOR)
	usec += Time.get_ticks_usec() - began


func _segment(camera: Camera3D, a: Vector3, b: Vector3, color: Color) -> void:
	var lift := Vector3.UP * LIFT
	if camera.is_position_behind(a + lift) or camera.is_position_behind(b + lift):
		return
	draw_line(camera.unproject_position(a + lift), camera.unproject_position(b + lift), color, 2.0, true)


func _marker(camera: Camera3D, p: Vector3, color: Color) -> void:
	var at := p + Vector3.UP * LIFT
	if camera.is_position_behind(at):
		return
	var screen := camera.unproject_position(at)
	draw_circle(screen, 5.0, Color(color, 0.35))
	draw_arc(screen, 5.0, 0.0, TAU, 16, color, 1.5, true)
	markers += 1


## Outline of a secured area on the ground (height of the center's
## terrain is close enough for a hint).
func _circle(camera: Camera3D, center: Vector3, radius: float) -> void:
	center.y = sim_host.height_at(center.x, center.z)
	var previous := Vector3.ZERO
	for k in 33:
		var angle := TAU * k / 32.0
		var p := center + Vector3(cos(angle), 0.0, sin(angle)) * radius
		if k > 0:
			_segment(camera, previous, p, SECURE_COLOR)
		previous = p
	secured += 1
