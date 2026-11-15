extends Node
## Headless self-test, started by `main.gd` with `-- --self-test`.
##
## Drives camera, selection and orders through real input events (pushed into
## the viewport), checks the results and quits with exit code 0 (pass) or 1
## (fail). Frame counts assume `--fixed-fps 60`.
##
## 1. Camera: zoom limits, pan to the map corner, 45° rotation.
## 2. Selection: box over the whole screen selects exactly the 8 own tanks,
##    Ctrl+1 / 1 / 1-1 (center), click, Shift-click, enemy click, double click.
## 3. Orders: right click sends the group into a grid formation (speed,
##    spacing, center checked), enemy tanks stay put, H stops a moving group.
## 4. Replay: the recorded match is saved, read back and re-simulated to the
##    same end hash.

## Panzer IV H: 16 km/h cross-country (catalog) × time factor 2 = 8.889 m/s,
## i.e. 0.4444 m per tick at 20 Hz.
const EXPECTED_STEP := 16.0 * 2.0 / 3.6 / 20.0
const OVERVIEW_DISTANCE := 350.0

var main: Node3D
var _errors: PackedStringArray = []


func _ready() -> void:
	_run()


func _run() -> void:
	await _camera_test()
	await _selection_test()
	await _order_test()
	_replay_test()
	var ok := _errors.is_empty()
	var message := "camera, selection, orders and replay ok" if ok else " ".join(_errors)
	print("self-test %s: %s" % ["PASSED" if ok else "FAILED", message])
	get_tree().quit(0 if ok else 1)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_errors.append(message + ".")
		push_error("self-test: " + message)


# --- 1. Camera ------------------------------------------------------------


func _camera_test() -> void:
	var rig: Node3D = main.camera_rig
	await get_tree().process_frame
	_push_wheel(MOUSE_BUTTON_WHEEL_UP, 40, Vector2(0.1, 0.1))
	await _frames(89)
	_camera_limits(rig.min_distance)
	_push_wheel(MOUSE_BUTTON_WHEEL_DOWN, 80, Vector2(0.9, 0.8))
	await _frames(90)
	_camera_limits(rig.max_distance)
	Input.action_press("camera_pan_right")
	Input.action_press("camera_pan_forward")
	await _frames(300)
	Input.action_release("camera_pan_right")
	Input.action_release("camera_pan_forward")
	_camera_limits(rig.max_distance)
	var corner := Vector3(rig.bounds.end.x, 0.0, rig.bounds.position.y)
	_check(rig.focus.distance_to(corner) < 0.01, "pan did not stop at the map corner %s" % rig.focus)
	var rotate := InputEventAction.new()
	rotate.action = "camera_rotate_left"
	rotate.pressed = true
	get_viewport().push_input(rotate)
	await _frames(60)
	_check(absf(rig.yaw - PI / 4.0) < 0.001, "rotation did not reach 45 degrees (%.4f)" % rig.yaw)
	print("self-test: camera distance %.2f, focus %s, yaw %.3f" % [rig.distance, rig.focus, rig.yaw])


func _camera_limits(expected_distance: float) -> void:
	var rig: Node3D = main.camera_rig
	_check(absf(rig.distance - expected_distance) < 0.01,
		"camera distance %.3f, expected %.3f" % [rig.distance, expected_distance])
	var focus := Vector2(rig.focus.x, rig.focus.z)
	_check(rig.bounds.grow(0.01).has_point(focus), "camera focus %s outside the map" % focus)


# --- 2. Selection ---------------------------------------------------------


func _selection_test() -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(2)
	var own := _ids_of(true)
	var enemy := _ids_of(false)
	_check(own.size() == 8 and enemy.size() == 8, "expected 8 own and 8 enemy units, got %d/%d" % [own.size(), enemy.size()])

	var size := get_viewport().get_visible_rect().size
	_drag(Vector2(4, 4), size - Vector2(4, 4))
	await _frames(1)
	_check(selection.selected == own, "box selected %s, expected own units %s" % [selection.selected, own])

	_key(KEY_1, true)
	_left_click(_screen_of_point(Vector3.ZERO))
	await _frames(1)
	_check(selection.selected.is_empty(), "click on empty ground kept %s" % [selection.selected])

	_key(KEY_1)
	await _frames(1)
	_check(selection.selected == own, "group 1 recalled %s" % [selection.selected])
	_key(KEY_1)
	await _frames(1)
	var centroid := _centroid(own)
	_check(rig.focus.distance_to(centroid) < 0.5, "double recall focused %s, expected %s" % [rig.focus, centroid])
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)

	_left_click(_screen_of_unit(own[0]))
	await _frames(1)
	_check(selection.selected == [own[0]], "click selected %s" % [selection.selected])
	_left_click(_screen_of_unit(own[1]), true)
	await _frames(1)
	_check(selection.selected == [own[0], own[1]], "shift click selected %s" % [selection.selected])
	_left_click(_screen_of_unit(enemy[0]))
	await _frames(1)
	_check(selection.selected.is_empty(), "enemy click selected %s" % [selection.selected])
	_left_click(_screen_of_unit(own[2]))
	_left_click(_screen_of_unit(own[2]), false, true)
	await _frames(1)
	_check(selection.selected == own, "double click selected %s" % [selection.selected])
	print("self-test: selection ok, group of %d" % selection.selected.size())


# --- 3. Orders ------------------------------------------------------------


func _order_test() -> void:
	var sim_host: SimHost = main.sim_host
	var own := _ids_of(true)
	var enemy_start := _positions_of(_ids_of(false))
	var target := Vector3(40, 0, -10)

	_right_click(_screen_of_point(target))
	await _frames(1)
	_check(main.target_marker.visible, "no target marker after the order")
	var aimed := Vector3(main.target_marker.position.x, 0.0, main.target_marker.position.z)

	# Follow the group tick by tick until it stands.
	var max_step := 0.0
	var last_tick := sim_host.current_tick()
	var deadline := last_tick + 20 * 30
	var started := false
	while sim_host.current_tick() < deadline:
		await get_tree().process_frame
		if sim_host.current_tick() == last_tick:
			continue
		last_tick = sim_host.current_tick()
		var previous := sim_host.previous_positions()
		var current := sim_host.current_positions()
		for i in _indices(own):
			max_step = maxf(max_step, previous[i].distance_to(current[i]))
		var moving := _moving(own)
		started = started or moving
		if started and not moving:
			break
	_check(started and not _moving(own), "group did not arrive (started %s)" % started)
	_check(absf(max_step - EXPECTED_STEP) < 0.002,
		"max step per tick %.4f m, expected %.4f m" % [max_step, EXPECTED_STEP])

	var positions := _positions_of(own)
	var low := positions[0]
	var high := positions[0]
	for p in positions:
		low = low.min(p)
		high = high.max(p)
	var center := (low + high) * 0.5
	_check(center.distance_to(aimed) < 0.01, "formation centered at %s, ordered %s" % [center, aimed])
	var spacing := sim_host.unit_footprints()[_indices(own)[0]] * 1.5
	var closest := INF
	for a in positions.size():
		for b in range(a + 1, positions.size()):
			closest = minf(closest, positions[a].distance_to(positions[b]))
	_check(closest > spacing - 0.01, "closest units %.3f m apart, expected %.3f m" % [closest, spacing])
	_check(_positions_of(_ids_of(false)) == enemy_start, "enemy units moved")
	print("self-test: group arrived at tick %d, center %s, spacing %.2f m, step %.4f m" % [
		last_tick, center, closest, max_step
	])

	# Stop a moving group with H.
	var far := Vector3(-80, 0, 40)
	_right_click(_screen_of_point(far))
	await _ticks(15)
	_check(_moving(own), "group did not start towards the second target")
	_key(KEY_H)
	await _ticks(3)
	_check(not _moving(own), "H did not stop the group")
	var stopped := _positions_of(own)
	await _ticks(5)
	_check(_positions_of(own) == stopped, "group kept driving after stop")
	_check(_centroid(own).distance_to(far) > 5.0, "group reached the target despite stop")
	print("self-test: stop ok at tick %d, hash %s" % [sim_host.current_tick(), sim_host.state_hash()])


# --- 4. Replay ------------------------------------------------------------


func _replay_test() -> void:
	var sim_host: SimHost = main.sim_host
	var path: String = main.save_replay()
	_check(not path.is_empty(), "replay was not written")
	if path.is_empty():
		return
	var bytes := FileAccess.get_file_as_bytes(path)
	_check(bytes == sim_host.replay_bytes(), "replay file differs from the recorded match")
	var problem := sim_host.verify_replay(bytes)
	_check(problem.is_empty(), "replay check failed: %s" % problem)
	print("self-test: replay ok (%d bytes, %d ticks)" % [bytes.size(), sim_host.current_tick()])


# --- Helpers --------------------------------------------------------------


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame


## Waits until `count` more ticks are simulated (at most 10 frames per tick).
func _ticks(count: int) -> void:
	var sim_host: SimHost = main.sim_host
	var until := sim_host.current_tick() + count
	var frames_left := count * 10
	while sim_host.current_tick() < until:
		if frames_left == 0:
			_check(false, "simulation stalled at tick %d" % sim_host.current_tick())
			return
		frames_left -= 1
		await get_tree().process_frame


## Ids of the own (or the other players') units, ascending.
func _ids_of(own: bool) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var result: Array[int] = []
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own:
			result.append(ids[i])
	result.sort()
	return result


func _indices(ids: Array[int]) -> Array[int]:
	var all: PackedInt64Array = main.sim_host.unit_ids()
	var result: Array[int] = []
	for id in ids:
		result.append(all.find(id))
	return result


func _positions_of(ids: Array[int]) -> Array[Vector3]:
	var current: PackedVector3Array = main.sim_host.current_positions()
	var result: Array[Vector3] = []
	for i in _indices(ids):
		result.append(current[i])
	return result


func _moving(ids: Array[int]) -> bool:
	var moving: PackedByteArray = main.sim_host.unit_moving()
	for i in _indices(ids):
		if moving[i] != 0:
			return true
	return false


func _centroid(ids: Array[int]) -> Vector3:
	var sum := Vector3.ZERO
	for p in _positions_of(ids):
		sum += p
	return sum / ids.size()


func _screen_of_point(point: Vector3) -> Vector2:
	return (main.camera as Camera3D).unproject_position(point)


func _screen_of_unit(id: int) -> Vector2:
	return _screen_of_point(_positions_of([id])[0] + Vector3.UP * 1.2)


func _mouse(button: MouseButton, at: Vector2, pressed: bool, shift := false, double := false) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.position = at
	event.global_position = at
	event.pressed = pressed
	event.shift_pressed = shift
	event.double_click = double
	get_viewport().push_input(event)


func _left_click(at: Vector2, shift := false, double := false) -> void:
	_mouse(MOUSE_BUTTON_LEFT, at, true, shift, double)
	_mouse(MOUSE_BUTTON_LEFT, at, false, shift)


func _right_click(at: Vector2) -> void:
	_mouse(MOUSE_BUTTON_RIGHT, at, true)
	_mouse(MOUSE_BUTTON_RIGHT, at, false)


func _drag(from: Vector2, to: Vector2) -> void:
	_mouse(MOUSE_BUTTON_LEFT, from, true)
	for step in range(1, 5):
		var motion := InputEventMouseMotion.new()
		motion.position = from.lerp(to, step / 4.0)
		motion.global_position = motion.position
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		get_viewport().push_input(motion)
	_mouse(MOUSE_BUTTON_LEFT, to, false)


func _key(keycode: Key, ctrl := false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		event.ctrl_pressed = ctrl
		event.pressed = pressed
		get_viewport().push_input(event)


## `at` is relative to the viewport size (0..1).
func _push_wheel(button: MouseButton, notches: int, at: Vector2) -> void:
	for i in notches:
		var wheel := InputEventMouseButton.new()
		wheel.button_index = button
		wheel.position = at * get_viewport().get_visible_rect().size
		wheel.pressed = true
		get_viewport().push_input(wheel)
