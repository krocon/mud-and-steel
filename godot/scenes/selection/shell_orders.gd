extends RefCounted
## Shelling a building, wall or tree (prompt 093): the targeted order
## `attack_object` of the selection.
##
## While the order waits for its click, the map object under the cursor that
## can be shelled (`SimHost.shell_target_at`: it has hit points and is not
## destroyed yet) is outlined, and the cursor shows whether there is one.
## The click sends `SimHost.attack_object`; the units drive into range and
## fire with the guns that damage objects until the object is a ruin or
## gone. Presentation only: the rules are the simulation's.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const MODE := "attack_object"
## Meters around an object's ground shape that still count as on it.
const MARGIN := 1.5
## The view ray is searched this far in front of its ground point, in these
## steps (meters): the cursor on a roof or wall picks that building, not the
## ground behind it.
const RAY_REACH := 60.0
const RAY_STEP := 1.5
const OUTLINE := Color(1.0, 0.5, 0.25, 0.95)
const FILL := Color(1.0, 0.5, 0.25, 0.16)
## Edges of the box of `SimHost.object_outline` (bottom ring, top ring,
## uprights).
const EDGES: Array[Vector2i] = [
	Vector2i(0, 1), Vector2i(1, 2), Vector2i(2, 3), Vector2i(3, 0),
	Vector2i(4, 5), Vector2i(5, 6), Vector2i(6, 7), Vector2i(7, 4),
	Vector2i(0, 4), Vector2i(1, 5), Vector2i(2, 6), Vector2i(3, 7),
]


## The object under `screen_position` that can be shelled, or -1.
static func target_at(selection: Control, screen_position: Vector2) -> int:
	var camera: Camera3D = selection.camera
	var sim_host: SimHost = selection.sim_host
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit == null:
		return -1
	var ground := hit as Vector3
	var toward := -camera.project_ray_normal(screen_position)
	# From the camera's side towards the ground point: the first object
	# whose box the ray passes through.
	var steps := int(RAY_REACH / RAY_STEP)
	for k in range(steps, 0, -1):
		var p := ground + toward * (k * RAY_STEP)
		var object := sim_host.shell_target_at(p.x, p.z, 0.0)
		if object < 0:
			continue
		var box := sim_host.object_outline(object)
		if box.size() == 8 and p.y <= box[4].y:
			return object
	return sim_host.shell_target_at(ground.x, ground.z, MARGIN)


## Updates the hovered target and the cursor while the order waits.
static func hover(selection: Control) -> void:
	var target := target_at(selection, selection.get_viewport().get_mouse_position())
	Input.set_default_cursor_shape(Input.CURSOR_CROSS if target >= 0 else Input.CURSOR_FORBIDDEN)
	if target != selection.shell_target:
		selection.shell_target = target
	# The outline follows the camera.
	selection.queue_redraw()


## Carries out the order at `screen_position` (false without a target or
## without a unit that can shell objects).
static func click(selection: Control, screen_position: Vector2) -> bool:
	var target := target_at(selection, screen_position)
	var units := PackedInt64Array(selection.selected)
	if target < 0 or not selection.sim_host.can_shell_objects(units):
		return false
	selection.sim_host.attack_object(units, target)
	selection.object_attack_ordered.emit(target)
	return true


## Draws the outline of the hovered target on the selection's canvas.
static func draw(selection: Control) -> void:
	if selection.shell_target < 0:
		return
	var camera: Camera3D = selection.camera
	var box: PackedVector3Array = selection.sim_host.object_outline(selection.shell_target)
	if box.size() != 8:
		return
	var points := PackedVector2Array()
	for corner in box:
		if camera.is_position_behind(corner):
			return
		points.append(camera.unproject_position(corner))
	var hull := Geometry2D.convex_hull(points)
	if hull.size() >= 3:
		selection.draw_colored_polygon(hull, FILL)
	for edge in EDGES:
		selection.draw_line(points[edge.x], points[edge.y], OUTLINE, 2.0, true)
