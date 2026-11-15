extends Control
## Unit selection and orders (presentation only).
##
## Selection lives entirely in Godot; only the resulting orders go to SimHost.
## Only units of the local player can be selected.
##
## - Left click selects a unit, Shift adds or removes it, a click on empty
##   ground clears the selection. Dragging draws a selection box.
## - Double click selects all visible own units of the same type.
## - Ctrl+1…9 (or Cmd on macOS) stores the selection as a group, 1…9 recalls
##   it, pressing the same number again quickly centers the camera on it.
## - Right click sends the selection to the clicked ground point, H stops it.
## - Right click on a unit of another player attacks it, Ctrl+right click
##   (Cmd on macOS) fires at the ground point, F switches between hold fire
##   and fire at will (prompt 017). Destroyed units cannot be selected.
##
## Sits full-screen in a CanvasLayer and draws the selection box itself. It
## ignores the mouse for GUI purposes and works from `_unhandled_input`.

signal move_ordered(point: Vector3)
signal attack_ordered(target: int)

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
## Pointer travel in pixels before a left press becomes a box drag.
const DRAG_THRESHOLD := 6.0
## Minimum click radius around a unit in pixels (for far zoom levels).
const MIN_PICK_RADIUS := 14.0
## A unit's pick point sits this high above the ground (hull center, meters).
const PICK_HEIGHT := 1.2
## Two presses of the same group key within this time center the camera.
const DOUBLE_PRESS_MS := 400
const BOX_FILL := Color(0.45, 0.85, 0.45, 0.12)
const BOX_LINE := Color(0.55, 0.95, 0.55, 0.9)

var sim_host: SimHost
var camera: Camera3D
var camera_rig: Node3D
## Selected unit ids, ascending.
var selected: Array[int] = []
## Control groups 1..9: number -> unit ids.
var groups: Dictionary[int, Array] = {}

var _pressing := false
var _dragging := false
var _double_click := false
var _press_position := Vector2.ZERO
var _drag_position := Vector2.ZERO
var _last_group_key := -1
var _last_group_time := -DOUBLE_PRESS_MS


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func is_selected(id: int) -> bool:
	return selected.has(id)


func _unhandled_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT:
			_on_left_button(button)
			get_viewport().set_input_as_handled()
		elif button.button_index == MOUSE_BUTTON_RIGHT and button.pressed:
			if button.ctrl_pressed or button.meta_pressed:
				_order_attack_ground(button.position)
			elif not _order_attack(button.position):
				_order_move(button.position)
			get_viewport().set_input_as_handled()
		return
	var motion := event as InputEventMouseMotion
	if motion != null and _pressing:
		_drag_position = motion.position
		if not _dragging and _drag_position.distance_to(_press_position) > DRAG_THRESHOLD:
			_dragging = true
		if _dragging:
			queue_redraw()
		return
	var key := event as InputEventKey
	if key != null and key.pressed and not key.echo:
		if _on_key(key):
			get_viewport().set_input_as_handled()


func _on_left_button(button: InputEventMouseButton) -> void:
	if button.pressed:
		_pressing = true
		_dragging = false
		_double_click = button.double_click
		_press_position = button.position
		_drag_position = button.position
		return
	if not _pressing:
		return
	_pressing = false
	if _dragging:
		_dragging = false
		queue_redraw()
		_select_in_box(Rect2(_press_position, Vector2.ZERO).expand(button.position), button.shift_pressed)
	elif _double_click:
		_select_same_type(_unit_at(button.position))
	else:
		_click(_unit_at(button.position), button.shift_pressed)


func _on_key(key: InputEventKey) -> bool:
	if key.keycode == KEY_H and not key.ctrl_pressed and not key.meta_pressed:
		stop_selected()
		return true
	if key.keycode == KEY_F and not key.ctrl_pressed and not key.meta_pressed:
		toggle_hold_fire()
		return true
	if key.keycode < KEY_1 or key.keycode > KEY_9:
		return false
	var number := int(key.keycode - KEY_0)
	if key.ctrl_pressed or key.meta_pressed:
		groups[number] = selected.duplicate()
		_last_group_key = -1
	else:
		recall_group(number)
	return true


## Selects group `number`; a second recall within DOUBLE_PRESS_MS also centers
## the camera on it.
func recall_group(number: int) -> void:
	var now := Time.get_ticks_msec()
	var again := number == _last_group_key and now - _last_group_time <= DOUBLE_PRESS_MS
	_last_group_key = number
	_last_group_time = now
	var members: Array = groups.get(number, [])
	_set_selection(members.filter(func(id: int) -> bool: return _own_index(id) >= 0))
	if again and not selected.is_empty():
		_center_camera_on_selection()


func stop_selected() -> void:
	if not selected.is_empty():
		sim_host.stop_units(PackedInt64Array(selected))


## Hold fire for the selection, or fire at will if all of it holds fire.
func toggle_hold_fire() -> void:
	if selected.is_empty():
		return
	var hold := sim_host.unit_hold_fire()
	var all_hold := true
	for id: int in selected:
		var index := _index_of(id)
		all_hold = all_hold and index >= 0 and hold[index] != 0
	sim_host.set_hold_fire(PackedInt64Array(selected), not all_hold)


## Attacks the living unit of another player under the cursor; false if
## there is none (or nothing is selected).
func _order_attack(screen_position: Vector2) -> bool:
	if selected.is_empty():
		return false
	var target := _unit_at(screen_position, false)
	if target < 0:
		return false
	sim_host.attack_unit(PackedInt64Array(selected), target)
	attack_ordered.emit(target)
	return true


func _order_attack_ground(screen_position: Vector2) -> void:
	if selected.is_empty():
		return
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit != null:
		var point := hit as Vector3
		sim_host.attack_ground(PackedInt64Array(selected), point.x, point.z)


func _order_move(screen_position: Vector2) -> void:
	if selected.is_empty():
		return
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit == null:
		return
	var point := hit as Vector3
	sim_host.move_units(PackedInt64Array(selected), point.x, point.z)
	move_ordered.emit(point)


func _click(id: int, additive: bool) -> void:
	if not additive:
		_set_selection([] if id < 0 else [id])
	elif id >= 0:
		var next := selected.duplicate()
		if next.has(id):
			next.erase(id)
		else:
			next.append(id)
		_set_selection(next)


func _select_same_type(id: int) -> void:
	if id < 0:
		_set_selection([])
		return
	var ids := sim_host.unit_ids()
	var types := sim_host.unit_types()
	var wanted := types[_index_of(id)]
	var visible_rect := get_viewport().get_visible_rect()
	var result: Array[int] = []
	for i in _own_indices():
		if types[i] == wanted and _on_screen(i, visible_rect):
			result.append(ids[i])
	_set_selection(result)


func _select_in_box(box: Rect2, additive: bool) -> void:
	var ids := sim_host.unit_ids()
	var result: Array[int] = []
	if additive:
		result = selected.duplicate()
	for i in _own_indices():
		if _on_screen(i, box) and not result.has(ids[i]):
			result.append(ids[i])
	_set_selection(result)


## Id of the own (or, with `own` false, another player's) living unit under
## `screen_position`, or -1. Picks the closest one whose screen radius
## contains the point.
func _unit_at(screen_position: Vector2, own := true) -> int:
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var footprints := sim_host.unit_footprints()
	var best := -1
	var best_distance := INF
	for i in (_own_indices() if own else _other_indices()):
		var world := positions[i] + Vector3.UP * PICK_HEIGHT
		if camera.is_position_behind(world):
			continue
		var center := camera.unproject_position(world)
		var edge := camera.unproject_position(world + camera.global_basis.x * footprints[i] * 0.5)
		var radius := maxf(MIN_PICK_RADIUS, center.distance_to(edge))
		var distance := center.distance_to(screen_position)
		if distance <= radius and distance < best_distance:
			best = ids[i]
			best_distance = distance
	return best


func _on_screen(index: int, rect: Rect2) -> bool:
	var world := sim_host.current_positions()[index] + Vector3.UP * PICK_HEIGHT
	return not camera.is_position_behind(world) and rect.has_point(camera.unproject_position(world))


func _center_camera_on_selection() -> void:
	var positions := sim_host.current_positions()
	var sum := Vector3.ZERO
	for id: int in selected:
		sum += positions[_index_of(id)]
	camera_rig.snap_to(sum / selected.size(), camera_rig.distance, camera_rig.yaw)


func _set_selection(ids: Array) -> void:
	var next: Array[int] = []
	for id: int in ids:
		if not next.has(id):
			next.append(id)
	next.sort()
	selected = next


## Indices (into the SimHost arrays) of the local player's living units.
func _own_indices() -> Array[int]:
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var me := sim_host.local_player()
	var result: Array[int] = []
	for i in owners.size():
		if owners[i] == me and alive[i] != 0:
			result.append(i)
	return result


## Indices of the other players' living units.
func _other_indices() -> Array[int]:
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var me := sim_host.local_player()
	var result: Array[int] = []
	for i in owners.size():
		if owners[i] != me and alive[i] != 0:
			result.append(i)
	return result


func _index_of(id: int) -> int:
	return sim_host.unit_ids().find(id)


## Index of `id` if it is an existing living own unit, else -1.
func _own_index(id: int) -> int:
	var index := _index_of(id)
	if index < 0 or sim_host.unit_owners()[index] != sim_host.local_player():
		return -1
	if sim_host.unit_alive()[index] == 0:
		return -1
	return index


func _draw() -> void:
	if not _dragging:
		return
	var box := Rect2(_press_position, Vector2.ZERO).expand(_drag_position)
	draw_rect(box, BOX_FILL, true)
	draw_rect(box, BOX_LINE, false, 1.0)
