extends Node
## Scripted demo, started by `main.gd` with `-- --demo`. Meant to be recorded:
##
##     godot --path godot --write-movie build/demo/demo.avi --fixed-fps 30 -- --demo
##
## Flies the camera over the map, selects the own tanks with a selection box,
## sends them across the field, rotates the view, sends one tank up the hill
## and zooms out to the overview. Uses the same input events as a player.
## The tanks of both sides fight on their own (prompt 017), so the hill
## drive takes the first own tank that is still alive.
## Quits when done.

## Frame rate the demo is timed for; record with `--fixed-fps 30`.
const FPS := 30

var main: Node3D


func _ready() -> void:
	_run()


func _run() -> void:
	var rig: Node3D = main.camera_rig
	rig.edge_scroll_enabled = false
	var own := _ids_of(true)
	var start := _centroid(own)

	# 1. From the overview down to the own tanks.
	rig.snap_to(start + Vector3(0, 0, 60), 560.0, 0.0)
	await _seconds(1.0)
	await _fly(start, 90.0, 0.0, 4.0)
	await _seconds(0.5)

	# 2. Box-select the own tanks (slow drag, so the box is visible).
	var box := _screen_box(own, 40.0)
	await _drag(box.position, box.end, 1.0)
	await _seconds(0.8)

	# 3. Send them across the field and follow them.
	var goal := start + Vector3(70, 0, -90)
	goal.y = main.sim_host.height_at(goal.x, goal.z)
	_right_click(_screen_of_point(goal))
	print("demo: group order to %s, marker at %s" % [goal, main.target_marker.position])
	await _follow(own, 110.0, 5.0)
	print("demo: group centroid after 5 s %s, moving %s" % [_centroid(own), main.sim_host.unit_moving()])

	# 4. Rotate the view while they drive.
	await _fly_follow(own, 110.0, -PI / 4.0, 3.0)
	await _follow(own, 110.0, 3.0, -PI / 4.0)

	# 5. One tank drives up the hill, if the map has one.
	var marker: Variant = main.sim_host.map_marker("hill_top")
	if marker != null:
		var top := marker as Vector3
		var survivors := _ids_of(true, true)
		var tank: int = survivors[0] if not survivors.is_empty() else own[0]
		# Center the tank first: the battle decides where it stands.
		await _fly(_positions_of([tank])[0], 110.0, -PI / 4.0, 1.0)
		survivors = _ids_of(true, true)
		if not survivors.is_empty() and not survivors.has(tank):
			tank = survivors[0]
		_left_click(_screen_of_unit(tank))
		await _seconds(0.3)
		var mid := (_positions_of([tank])[0] + top) * 0.5
		await _fly(mid, 160.0, -PI / 4.0, 2.0)
		print("demo: selected %s for the hill" % [main.selection.selected])
		_right_click(_screen_of_point(top))
		await _follow([tank], 120.0, 15.0, -PI / 4.0)
		print("demo: hill tank at %s, hill top %s" % [_positions_of([tank])[0], top])

	# 6. Back up to the overview.
	await _fly(rig.focus, 600.0, -PI / 2.0, 4.0)
	await _seconds(1.5)
	get_tree().quit()


# --- Camera moves (eased, frame by frame) ---------------------------------


func _fly(to_focus: Vector3, to_distance: float, to_yaw: float, seconds: float) -> void:
	var rig: Node3D = main.camera_rig
	var from_focus: Vector3 = rig.focus
	var from_distance: float = rig.distance
	var from_yaw: float = rig.yaw
	var frames := _frame_count(seconds)
	for i in range(1, frames + 1):
		var t := smoothstep(0.0, 1.0, float(i) / frames)
		var distance := exp(lerpf(log(from_distance), log(to_distance), t))
		rig.snap_to(from_focus.lerp(to_focus, t), distance, lerp_angle(from_yaw, to_yaw, t))
		await get_tree().process_frame


## Keeps the camera on the centroid of `ids` for `seconds`.
func _follow(ids: Array[int], distance: float, seconds: float, yaw := 0.0) -> void:
	var rig: Node3D = main.camera_rig
	for i in _frame_count(seconds):
		rig.snap_to(rig.focus.lerp(_centroid(ids), 0.08), lerpf(rig.distance, distance, 0.05), yaw)
		await get_tree().process_frame


func _fly_follow(ids: Array[int], distance: float, to_yaw: float, seconds: float) -> void:
	var rig: Node3D = main.camera_rig
	var from_yaw: float = rig.yaw
	var frames := _frame_count(seconds)
	for i in range(1, frames + 1):
		var t := smoothstep(0.0, 1.0, float(i) / frames)
		rig.snap_to(rig.focus.lerp(_centroid(ids), 0.08), lerpf(rig.distance, distance, 0.05),
			lerp_angle(from_yaw, to_yaw, t))
		await get_tree().process_frame


# --- Input (same events as a player) ---------------------------------------


func _mouse(button: MouseButton, at: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.position = at
	event.global_position = at
	event.pressed = pressed
	get_viewport().push_input(event)


func _left_click(at: Vector2) -> void:
	_mouse(MOUSE_BUTTON_LEFT, at, true)
	_mouse(MOUSE_BUTTON_LEFT, at, false)


func _right_click(at: Vector2) -> void:
	_mouse(MOUSE_BUTTON_RIGHT, at, true)
	_mouse(MOUSE_BUTTON_RIGHT, at, false)


func _drag(from: Vector2, to: Vector2, seconds: float) -> void:
	_mouse(MOUSE_BUTTON_LEFT, from, true)
	var frames := _frame_count(seconds)
	for i in range(1, frames + 1):
		var motion := InputEventMouseMotion.new()
		motion.position = from.lerp(to, float(i) / frames)
		motion.global_position = motion.position
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		get_viewport().push_input(motion)
		await get_tree().process_frame
	_mouse(MOUSE_BUTTON_LEFT, to, false)


# --- Queries ----------------------------------------------------------------


func _ids_of(own: bool, alive_only := false) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var result: Array[int] = []
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own and (alive[i] != 0 or not alive_only):
			result.append(ids[i])
	result.sort()
	return result


func _positions_of(ids: Array[int]) -> Array[Vector3]:
	var all: PackedInt64Array = main.sim_host.unit_ids()
	var current: PackedVector3Array = main.sim_host.current_positions()
	var result: Array[Vector3] = []
	for id in ids:
		result.append(current[all.find(id)])
	return result


func _centroid(ids: Array[int]) -> Vector3:
	var sum := Vector3.ZERO
	for p in _positions_of(ids):
		sum += p
	return sum / ids.size()


func _screen_of_point(point: Vector3) -> Vector2:
	return (main.camera as Camera3D).unproject_position(point)


func _screen_of_unit(id: int) -> Vector2:
	return _screen_of_point(_positions_of([id])[0] + Vector3.UP * 1.2)


## Screen rectangle around `ids`, grown by `margin` pixels.
func _screen_box(ids: Array[int], margin: float) -> Rect2:
	var box := Rect2(_screen_of_unit(ids[0]), Vector2.ZERO)
	for id in ids:
		box = box.expand(_screen_of_unit(id))
	return box.grow(margin)


func _frame_count(seconds: float) -> int:
	return maxi(1, roundi(seconds * FPS))


func _seconds(seconds: float) -> void:
	for i in _frame_count(seconds):
		await get_tree().process_frame
