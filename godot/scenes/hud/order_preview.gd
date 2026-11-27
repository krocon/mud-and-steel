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
## Unreachable slots (prompt 175): where the unit cannot get to its slot
## (across a river without a bridge it may use, into a lake), the mark
## stands at the point it really drives to (`REACH_COLOR`), a thin line
## leads from the clicked slot to it, and a short note says "Target not
## reachable – drives up to here". `SimHost.formation_preview` gives that
## point with the slot (the same rule as the path search).
##
## Drawn on the HUD like `waypoints.gd`, so above the fog layer.

const RangeRings := preload("res://scenes/hud/range_rings.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const GRENADE_FRAG_COLOR := Color(1.0, 0.56, 0.16, 0.92)
const GRENADE_FRAG_FAR_COLOR := Color(1.0, 0.78, 0.34, 0.78)
const GRENADE_SMOKE_COLOR := Color(0.94, 0.96, 1.0, 0.92)
const GRENADE_SMOKE_FAR_COLOR := Color(0.76, 0.84, 0.94, 0.78)
const GRENADE_RANGE_COLOR := Color(1.0, 0.82, 0.35, 0.72)

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
const REACH_COLOR := Color(1.0, 0.68, 0.25)
## Floats per unit in `slots` (id, slot x, z, blocked, reach x, z).
const STRIDE := 6
## A reach point farther than this (m) from its slot counts as a
## substitute.
const SUBSTITUTE_M := 6.0
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
## Marks moved to a substitute point in the last frame (prompt 175;
## self-test).
var substitutes := 0
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
## Cover preview cache (prompt 233): 5 floats per soldier spot [x, z, fx, fz, grade].
var _cover_spots := PackedFloat32Array()
var _cover_key := []
var _cover_cursor := Vector3.INF


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
	var mode: String = main.selection.command_mode
	var grenade_mode: bool = mode in ["grenade", "smoke_grenade"]
	var cover_mode: bool = mode == "cover"
	if cover_mode:
		var hit: Variant = GroundPick.from_screen(main.camera, get_viewport().get_mouse_position())
		if hit != null:
			_cover_cursor = hit as Vector3
			_fetch_cover(_cover_cursor, Vector3.ZERO, main.selection.selected)
		else:
			_cover_cursor = Vector3.INF
			_cover_spots = PackedFloat32Array()
	elif _held and _all_infantry(main.selection.selected) and not pending.is_empty():
		_cover_cursor = pending[0]
		_fetch_cover(pending[0], pending[1], main.selection.selected)
	elif not _cover_spots.is_empty():
		_cover_spots = PackedFloat32Array()
		_cover_key = []
		_cover_cursor = Vector3.INF
	if not slots.is_empty() or marks > 0 or grenade_mode or cover_mode:
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
	var mode: String = main.selection.command_mode
	if mode == "grenade" or mode == "smoke_grenade":
		_draw_grenade_preview(mode == "smoke_grenade")
	elif mode == "cover":
		_draw_cover_spots(true, 1.0)
	if slots.is_empty():
		usec += Time.get_ticks_usec() - began
		return
	var alpha := 1.0
	if _age > HOLD_SECONDS - FADE_SECONDS:
		alpha = clampf((HOLD_SECONDS - _age) / FADE_SECONDS, 0.0, 1.0)
	if _held and not _cover_spots.is_empty() and _has_any_cover(_cover_spots):
		_draw_cover_spots(false, alpha)
	var camera: Camera3D = main.camera
	var own := RelationColors.own()
	var forward := _forward()
	var ids := sim_host.unit_ids()
	var towing := sim_host.unit_towing()
	var count := mini(slots.size() / STRIDE, MAX_MARKS)
	substitutes = 0
	var note_at := Vector3.INF
	for k in count:
		var id := int(slots[STRIDE * k])
		var at := Vector3(slots[STRIDE * k + 1], 0.0, slots[STRIDE * k + 2])
		at.y = sim_host.height_at(at.x, at.z) + LIFT
		var color := BLOCKED_COLOR if slots[STRIDE * k + 3] > 0.5 else own
		var reach := Vector3(slots[STRIDE * k + 4], 0.0, slots[STRIDE * k + 5])
		if Vector2(reach.x - at.x, reach.z - at.z).length() > SUBSTITUTE_M:
			# Not reachable: the mark stands where the unit stops.
			reach.y = sim_host.height_at(reach.x, reach.z) + LIFT
			if not camera.is_position_behind(at) and not camera.is_position_behind(reach):
				draw_dashed_line(camera.unproject_position(at), camera.unproject_position(reach),
					Color(REACH_COLOR, 0.7 * alpha), 1.5, 6.0)
			substitutes += 1
			at = reach
			color = REACH_COLOR
			if note_at == Vector3.INF:
				note_at = reach
		if _is_vehicle(id):
			_rect_mark(camera, at, forward, color, alpha)
		else:
			_round_mark(camera, at, color, alpha)
		var index := ids.find(id)
		if index >= 0 and index < towing.size() and towing[index] >= 0:
			gun_rings += RangeRings.draw_unit_rings(self, camera, sim_host, towing[index], at,
				RangeRings.STYLE_TOWED, alpha)
	if note_at != Vector3.INF and not camera.is_position_behind(note_at):
		var font := get_theme_default_font()
		var text := tr("HUD_TARGET_UNREACHABLE")
		var size := 15
		var p := camera.unproject_position(note_at) + Vector2(12, -12)
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		draw_rect(Rect2(p - Vector2(5, size), Vector2(width + 10, size + 8)), Color(0, 0, 0, 0.55 * alpha))
		draw_string(font, p, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, Color(REACH_COLOR, alpha))
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


## Grenade / smoke grenade targeting overlay (prompt 232): throw-range rings
## around eligible selected squads, effect-radius circle at the cursor, and an
## "out of range – squad advances" note when the cursor lies beyond throw range.
func _draw_grenade_preview(smoke: bool) -> void:
	var params := sim_host.grenade_params()
	if params.x <= 0.0:
		return
	var throw_range := params.x
	var radius := params.z if smoke else params.y
	var camera: Camera3D = main.camera
	var drawn_rings := 0
	var min_dist := INF
	var cursor_hit: Variant = GroundPick.from_screen(camera, get_viewport().get_mouse_position())
	var cursor := cursor_hit as Vector3 if cursor_hit != null else Vector3.INF
	for id: int in main.selection.selected:
		var g := sim_host.unit_grenades(id)
		if g.size() < 6 or (g[2] if smoke else g[0]) <= 0:
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var at := node.global_position
		if drawn_rings < 8:
			RangeRings.draw_ring(self, camera, sim_host, at, throw_range,
					GRENADE_RANGE_COLOR, RangeRings.STYLE_DOTTED, 0.9)
			drawn_rings += 1
		if cursor != Vector3.INF:
			var d := Vector2(cursor.x - at.x, cursor.z - at.z).length()
			if d < min_dist:
				min_dist = d
	if cursor == Vector3.INF or drawn_rings == 0:
		return
	var in_range := min_dist <= throw_range + 0.5
	var color := (GRENADE_SMOKE_COLOR if smoke else GRENADE_FRAG_COLOR) if in_range \
			else (GRENADE_SMOKE_FAR_COLOR if smoke else GRENADE_FRAG_FAR_COLOR)
	var style := RangeRings.STYLE_SOLID if in_range else RangeRings.STYLE_DOTTED
	RangeRings.draw_ring(self, camera, sim_host, cursor, radius, color, style, 1.0)
	if not in_range and not camera.is_position_behind(cursor):
		var font := get_theme_default_font()
		var text := tr("HUD_GRENADE_ADVANCE")
		var size := 14
		var p := camera.unproject_position(cursor) + Vector2(14, -14)
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		draw_rect(Rect2(p - Vector2(5, size), Vector2(width + 10, size + 8)), Color(0, 0, 0, 0.6))
		draw_string(font, p, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)


func _all_infantry(ids: Array) -> bool:
	if ids.is_empty():
		return false
	for id: int in ids:
		if _is_vehicle(id):
			return false
	return true


func _fetch_cover(point: Vector3, facing: Vector3, ids: Array) -> void:
	var key := [snappedf(point.x, REFETCH_M), snappedf(point.z, REFETCH_M),
		snappedf(facing.x, REFETCH_M), snappedf(facing.z, REFETCH_M), ids.size(), ids.hash()]
	if key == _cover_key:
		return
	_cover_key = key
	_cover_spots = sim_host.cover_preview(PackedInt64Array(ids), point.x, point.z, facing.x, facing.z)


static func _has_any_cover(spots: PackedFloat32Array) -> bool:
	for k in spots.size() / 5:
		if spots[5 * k + 4] > 0.5:
			return true
	return false


## Draws per-soldier cover dots with facing ticks from `SimHost.cover_preview`
## (prompt 233: green = heavy cover, amber/yellow = light cover, grey = open).
func _draw_cover_spots(show_label: bool, alpha: float) -> void:
	if _cover_spots.is_empty():
		return
	var camera: Camera3D = main.camera
	var count := mini(_cover_spots.size() / 5, MAX_MARKS)
	var best_grade := 0
	for k in count:
		var sx := _cover_spots[5 * k]
		var sz := _cover_spots[5 * k + 1]
		var fx := _cover_spots[5 * k + 2]
		var fz := _cover_spots[5 * k + 3]
		var grade := int(round(_cover_spots[5 * k + 4]))
		best_grade = maxi(best_grade, grade)
		var at := Vector3(sx, sim_host.height_at(sx, sz) + LIFT, sz)
		if camera.is_position_behind(at):
			continue
		var col := RelationColors.cover_heavy(alpha) if grade >= 2 \
				else (RelationColors.cover_light(alpha) if grade == 1 else Color(BLOCKED_COLOR, alpha * 0.75))
		var p := camera.unproject_position(at)
		draw_circle(p, 6.2, Color(0, 0, 0, 0.75 * alpha))
		draw_circle(p, 4.6, col)
		var tip_w := Vector3(sx + fx * 1.1, sim_host.height_at(sx + fx * 1.1, sz + fz * 1.1) + LIFT, sz + fz * 1.1)
		if not camera.is_position_behind(tip_w):
			var tip := camera.unproject_position(tip_w)
			draw_line(p, tip, Color(0, 0, 0, 0.75 * alpha), 3.6, true)
			draw_line(p, tip, col, 2.0, true)
	if show_label and _cover_cursor != Vector3.INF and not camera.is_position_behind(_cover_cursor):
		var font := get_theme_default_font()
		var key := "HUD_COVER_HEAVY" if best_grade >= 2 else ("HUD_COVER_LIGHT" if best_grade == 1 else "HUD_COVER_OPEN")
		var col := RelationColors.cover_heavy(alpha) if best_grade >= 2 \
				else (RelationColors.cover_light(alpha) if best_grade == 1 else Color(BLOCKED_COLOR, alpha))
		var text := tr(key)
		var size := 14
		var pos := camera.unproject_position(_cover_cursor) + Vector2(14, -14)
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		draw_rect(Rect2(pos - Vector2(5, size), Vector2(width + 10, size + 8)), Color(0, 0, 0, 0.62 * alpha))
		draw_string(font, pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, col)
