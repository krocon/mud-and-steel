extends Control
## Routes and waypoint chains of the selected units (prompts 031, 166),
## drawn on the HUD. Patrols are blue with the closing leg back to the first
## waypoint, routes of an attack-move orange, queued waypoints yellow;
## secured areas (prompt 046) show as a red circle.
##
## Prompt 166: one picture per order instead of one line per unit. Units
## that got the same order (`selection.gd`'s `formation_ordered` names
## them; units without such an order are grouped by a target within
## `GROUP_M`) share one dashed line from the middle of those of them on the
## screen to the order's target, one marker there, small dots at their
## formation places and the waypoint chain once. A unit's full path with
## every bend is drawn only when it is the only one selected or under the
## mouse pointer. No line starts from units off the screen.
##
## Setting `route_lines`: `off` (nothing), `targets` (markers only, no
## lines), `brief` (default: lines strong for `BRIEF_S` after an order, then
## faint; strong again while Shift (waypoints) or the show-all key
## `show_units` is held), `always` (one full route per unit, as before
## prompt 166).
##
## The points are projected to the screen; a straight segment stays straight
## under the perspective projection, so this is exact for segments in front
## of the camera.

const Settings := preload("res://scenes/menu/settings.gd")

## Selected units drawn at most.
const MAX_UNITS := 48
## Meters above the ground.
const LIFT := 0.6
const ROUTE_COLOR := Color(0.5, 1.0, 0.5, 0.55)
const QUEUE_COLOR := Color(1.0, 0.9, 0.35, 0.85)
const PATROL_COLOR := Color(0.45, 0.75, 1.0, 0.85)
const ATTACK_MOVE_COLOR := Color(1.0, 0.6, 0.3, 0.7)
const SECURE_COLOR := Color(1.0, 0.4, 0.3, 0.8)
## Seconds lines stay strong after an order, and the fade to `FAINT`
## (share of their opacity) after that.
const BRIEF_S := 2.5
const FADE_S := 0.6
const FAINT := 0.18
## Units without a shared order whose targets are this close (meters)
## count as one group.
const GROUP_M := 40.0
## A unit's formation place this close to its order's point (meters) still
## belongs to that order's first leg.
const ORDER_REACH_M := 150.0
## Line width (pixels) and dash length of the group lines.
const LINE_WIDTH := 1.5
const DASH := 9.0
## Half the side of a formation place dot (pixels).
const DOT_RADIUS := 2.0

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_draw` in microseconds (hud.gd sums and resets
## it every frame); the last `_draw` alone (route shots).
var usec := 0
var draw_usec := 0
## Queued waypoint markers drawn in the last frame (self-test).
var markers := 0
## Secured areas drawn in the last frame (self-test).
var secured := 0
## Groups (one per order) drawn in the last frame, and lines from units to
## a target (group lines and single full paths) (self-test).
var groups := 0
var lines := 0
## Opacity share of the lines in the last frame (1 strong, FAINT faint).
var strength := 1.0

## Order id per unit id, the target of every order id, when the last order
## was given (msec).
var _order_of: Dictionary[int, int] = {}
var _order_point: Dictionary[int, Vector3] = {}
var _next_order := 1
var _ordered_at := -100000
## Groups and secured areas of the selection, and what they were built for
## (tick, selection, orders given).
var _cache_key := []
var _cache_groups: Array[Dictionary] = []
var _cache_areas: Array[Vector4] = []


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "Waypoints"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var selection: Control = main.selection
	selection.formation_ordered.connect(_on_formation_ordered)
	selection.move_ordered.connect(func(_point: Vector3) -> void: _ordered_at = Time.get_ticks_msec())
	selection.attack_ordered.connect(func(_target: int) -> void: _ordered_at = Time.get_ticks_msec())


func _on_formation_ordered(point: Vector3, _facing: Vector3, ids: Array) -> void:
	var order := _next_order
	_next_order += 1
	_order_point[order] = point
	for id: int in ids:
		_order_of[id] = order
	_ordered_at = Time.get_ticks_msec()
	# Forget orders no unit carries any more.
	if _order_point.size() > 64:
		var used := {}
		for o: int in _order_of.values():
			used[o] = true
		for o: int in _order_point.keys():
			if not used.has(o):
				_order_point.erase(o)


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	var began := Time.get_ticks_usec()
	markers = 0
	secured = 0
	groups = 0
	lines = 0
	var selected: Array[int] = main.selection.selected
	var mode: String = Settings.get_value("route_lines")
	if selected.is_empty() or mode == "off":
		draw_usec = Time.get_ticks_usec() - began
		usec += draw_usec
		return
	var camera: Camera3D = main.camera
	var units := selected.slice(0, MAX_UNITS)
	# Route data changes only with a tick (or a new selection or order).
	var key := [sim_host.current_tick(), units.hash(), _next_order]
	if key != _cache_key:
		_cache_key = key
		_cache_areas.clear()
		for id: int in units:
			var drill := sim_host.unit_drill(id)
			if drill.x == 3.0 and not _cache_areas.has(drill):
				_cache_areas.append(drill)
		_cache_groups = _build_groups(units)
	for drill: Vector4 in _cache_areas:
		_circle(camera, Vector3(drill.y, 0.0, drill.z), drill.w)
	if mode == "always":
		for id: int in units:
			_full_route(camera, id, 1.0, true)
	else:
		strength = _strength() if mode == "brief" else 0.0
		_groups(camera, units, mode)
	draw_usec = Time.get_ticks_usec() - began
	usec += draw_usec


## 1 for BRIEF_S after an order or while Shift or the show-all key is
## held, then fading to FAINT.
func _strength() -> float:
	if Input.is_key_pressed(KEY_SHIFT) \
			or (InputMap.has_action("show_units") and Input.is_action_pressed("show_units")):
		return 1.0
	var since := (Time.get_ticks_msec() - _ordered_at) / 1000.0
	if since <= BRIEF_S:
		return 1.0
	return lerpf(1.0, FAINT, clampf((since - BRIEF_S) / FADE_S, 0.0, 1.0))


## Groups of the selected units by order (see the header), from the
## simulation's routes: {order, kind, patrol, queued, target, units,
## places, chain}.
func _build_groups(units: Array[int]) -> Array[Dictionary]:
	var found: Array[Dictionary] = []
	for id: int in units:
		var points := sim_host.unit_waypoints(id)
		if points.is_empty():
			continue
		var queued := sim_host.unit_queue_length(id)
		var patrol := sim_host.unit_patrols(id)
		var kind := sim_host.unit_drill(id).x
		var first_queued := points.size() - queued
		var place := points[maxi(first_queued - 1, 0)]
		var order: int = _order_of.get(id, 0)
		var group: Dictionary = {}
		for g: Dictionary in found:
			if g["kind"] != kind or g["patrol"] != patrol or g["queued"] != queued:
				continue
			if order > 0 and g["order"] == order \
					or order == 0 and g["order"] == 0 and (g["places"][0] as Vector3).distance_to(place) <= GROUP_M:
				group = g
				break
		if group.is_empty():
			# The order's point while the units drive their first leg to it
			# (their formation places lie around it).
			var target := place
			if order > 0 and first_queued > 0 and _order_point.has(order) \
					and (_order_point[order] as Vector3).distance_to(place) <= ORDER_REACH_M:
				target = _order_point[order]
			target.y = sim_host.height_at(target.x, target.z)
			group = {"order": order, "kind": kind, "patrol": patrol, "queued": queued,
				"target": target, "units": [], "places": [], "chain": []}
			for k in range(first_queued, points.size()):
				(group["chain"] as Array).append(points[k])
			found.append(group)
		(group["units"] as Array).append(id)
		(group["places"] as Array).append(place)
	return found


## Draws the cached groups: their lines start from where their units on
## the screen are drawn now.
func _groups(camera: Camera3D, units: Array[int], mode: String) -> void:
	var hovered: int = -1
	var highlight: Node = main.get("unit_highlight")
	if highlight != null and units.has(highlight.nearest):
		hovered = highlight.nearest
	var single := units.size() == 1
	for g: Dictionary in _cache_groups:
		var starts: Array = []
		for id: int in g["units"]:
			var node: Node3D = main._unit_nodes.get(id)
			if node != null and node.visible and _on_screen(camera, node.global_position):
				starts.append(node.global_position)
		g["starts"] = starts
		g["starts"] = starts
		groups += 1
		_group(camera, g, mode, single)
	if mode != "brief":
		return
	# Full path with every bend: the only selected unit, or the one under
	# the pointer (strong).
	for id: int in units:
		if single or id == hovered:
			_full_route(camera, id, 1.0 if id == hovered else maxf(strength, FAINT), false)


func _group(camera: Camera3D, g: Dictionary, mode: String, single: bool) -> void:
	var patrol: bool = g["patrol"]
	var route := ATTACK_MOVE_COLOR if g["kind"] == 1.0 else ROUTE_COLOR
	var chain_color := PATROL_COLOR if patrol else QUEUE_COLOR
	var target: Vector3 = g["target"]
	var chain: Array = g["chain"]
	var starts: Array = g["starts"]
	var places: Array = g["places"]
	var line_alpha := strength if mode == "brief" else 0.0
	# Line from the middle of the group's units on the screen to the target.
	if line_alpha > 0.0 and not starts.is_empty():
		var middle := Vector3.ZERO
		for p: Vector3 in starts:
			middle += p
		middle /= starts.size()
		# The only selected unit gets its full path instead.
		if not single:
			_dashed(camera, middle, target, Color(route, route.a * line_alpha))
			lines += 1
	# The waypoint chain once, from the target on.
	if not chain.is_empty():
		var previous: Vector3 = target
		for p: Vector3 in chain:
			if line_alpha > 0.0:
				_segment(camera, previous, p, Color(chain_color, chain_color.a * maxf(line_alpha, FAINT)))
			previous = p
		if patrol and chain.size() > 1 and line_alpha > 0.0:
			_segment(camera, chain[chain.size() - 1], chain[0], Color(chain_color, chain_color.a * maxf(line_alpha, FAINT)))
		# Markers: the target (end of the current leg) and every queued point.
		_marker(camera, target, chain_color)
		for p: Vector3 in chain:
			_marker(camera, p, chain_color)
	else:
		_target_mark(camera, target, route)
	# Formation places as dots (when the units do not share one point).
	if places.size() > 1:
		for p: Vector3 in places:
			_dot(camera, p, Color(route, 0.8))


## The full remaining route of one unit with every bend (and its chain).
func _full_route(camera: Camera3D, id: int, alpha: float, old_markers: bool) -> void:
	var points := sim_host.unit_waypoints(id)
	if points.is_empty():
		return
	var node: Node3D = main._unit_nodes.get(id)
	if node == null or not node.visible or (not old_markers and not _on_screen(camera, node.global_position)):
		return
	var drill := sim_host.unit_drill(id)
	var queued := sim_host.unit_queue_length(id)
	var patrol := sim_host.unit_patrols(id)
	var first_queued := points.size() - queued
	var previous: Vector3 = node.global_position
	var route := ATTACK_MOVE_COLOR if drill.x == 1.0 else ROUTE_COLOR
	for k in points.size():
		var point := points[k]
		if k >= first_queued and not old_markers:
			break
		var color := route if k < first_queued else (PATROL_COLOR if patrol else QUEUE_COLOR)
		if old_markers:
			_segment(camera, previous, point, Color(color, color.a * alpha), 2.0)
		else:
			_segment(camera, previous, point, Color(color, color.a * alpha), LINE_WIDTH)
		previous = point
	lines += 1
	if not old_markers:
		return
	for k in range(maxi(first_queued - 1, 0), points.size()):
		# The current target (end of the route) gets a marker too.
		if k == first_queued - 1 and queued == 0:
			continue
		_marker(camera, points[k], PATROL_COLOR if patrol else QUEUE_COLOR)
	if patrol and queued > 0:
		_segment(camera, points[points.size() - 1], points[first_queued], PATROL_COLOR, 2.0)


func _on_screen(camera: Camera3D, p: Vector3) -> bool:
	var at := p + Vector3.UP * LIFT
	return not camera.is_position_behind(at) and get_viewport_rect().has_point(camera.unproject_position(at))


func _segment(camera: Camera3D, a: Vector3, b: Vector3, color: Color, width := LINE_WIDTH) -> void:
	var lift := Vector3.UP * LIFT
	if camera.is_position_behind(a + lift) or camera.is_position_behind(b + lift):
		return
	draw_line(camera.unproject_position(a + lift), camera.unproject_position(b + lift), color, width, true)


func _dashed(camera: Camera3D, a: Vector3, b: Vector3, color: Color) -> void:
	var lift := Vector3.UP * LIFT
	if camera.is_position_behind(a + lift) or camera.is_position_behind(b + lift):
		return
	# Cut to the screen: a far target projects thousands of pixels away,
	# and every dash is drawn.
	var clipped := _clip(camera.unproject_position(a + lift), camera.unproject_position(b + lift),
		get_viewport_rect().grow(DASH))
	if not clipped.is_empty():
		draw_dashed_line(clipped[0], clipped[1], color, LINE_WIDTH, DASH, true, true)


## The part of the segment `p`-`q` inside `rect` (empty: none), by
## Liang-Barsky.
static func _clip(p: Vector2, q: Vector2, rect: Rect2) -> PackedVector2Array:
	var d := q - p
	var t0 := 0.0
	var t1 := 1.0
	var checks := [[-d.x, p.x - rect.position.x], [d.x, rect.end.x - p.x],
		[-d.y, p.y - rect.position.y], [d.y, rect.end.y - p.y]]
	for c: Array in checks:
		var pk: float = c[0]
		var qk: float = c[1]
		if is_zero_approx(pk):
			if qk < 0.0:
				return PackedVector2Array()
			continue
		var t := qk / pk
		if pk < 0.0:
			t0 = maxf(t0, t)
		else:
			t1 = minf(t1, t)
		if t0 > t1:
			return PackedVector2Array()
	return PackedVector2Array([p + d * t0, p + d * t1])


func _marker(camera: Camera3D, p: Vector3, color: Color) -> void:
	var at := p + Vector3.UP * LIFT
	if camera.is_position_behind(at):
		return
	var screen := camera.unproject_position(at)
	draw_circle(screen, 5.0, Color(color, 0.35))
	draw_arc(screen, 5.0, 0.0, TAU, 16, color, 1.5, true)
	markers += 1


## The target of a plain move: a small ring with a cross (not counted as a
## queued waypoint marker).
func _target_mark(camera: Camera3D, p: Vector3, color: Color) -> void:
	var at := p + Vector3.UP * LIFT
	if camera.is_position_behind(at):
		return
	var screen := camera.unproject_position(at)
	var c := Color(color, 0.9)
	draw_arc(screen, 6.0, 0.0, TAU, 20, c, 1.5, true)
	draw_line(screen - Vector2(3, 0), screen + Vector2(3, 0), c, 1.0, true)
	draw_line(screen - Vector2(0, 3), screen + Vector2(0, 3), c, 1.0, true)


func _dot(camera: Camera3D, p: Vector3, color: Color) -> void:
	var at := p + Vector3.UP * LIFT
	if camera.is_position_behind(at):
		return
	# A small square: `draw_circle` cost about 13 µs per dot.
	draw_rect(Rect2(camera.unproject_position(at) - Vector2.ONE * DOT_RADIUS, Vector2.ONE * DOT_RADIUS * 2.0), color)


## Outline of a secured area on the ground (height of the center's
## terrain is close enough for a hint).
func _circle(camera: Camera3D, center: Vector3, radius: float) -> void:
	center.y = sim_host.height_at(center.x, center.z)
	var previous := Vector3.ZERO
	for k in 33:
		var angle := TAU * k / 32.0
		var p := center + Vector3(cos(angle), 0.0, sin(angle)) * radius
		if k > 0:
			_segment(camera, previous, p, SECURE_COLOR, 2.0)
		previous = p
	secured += 1
