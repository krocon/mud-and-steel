extends Node3D
## Main scene: draws the simulation and turns right-clicks into move commands.
##
## Everything game-relevant happens in SimHost (Rust). This script only reads
## snapshots, interpolates between the last two ticks, and forwards input.
##
## Start with `-- --self-test` to drive the unit and the camera automatically
## and quit with exit code 0 (pass) or 1 (fail). Used for headless checks.
##
## Start with `-- --camera-shots` (together with `--write-movie`) to record
## three fixed camera views: close-up, medium and overview.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const SEED := 42
## Size of the test ground in meters, centered on the origin. Prompt 010
## replaces this with the size of the loaded map.
const MAP_SIZE := Vector2(2000, 2000)
## Camera distances (meters) recorded by --camera-shots.
const SHOT_DISTANCES: Array[float] = [15.0, 120.0, 600.0]
const FRAMES_PER_SHOT := 30

@onready var sim_host: SimHost = $SimHost
@onready var camera_rig: Node3D = $RtsCamera
@onready var camera: Camera3D = $RtsCamera/Camera3D
@onready var sun: DirectionalLight3D = $Sun
@onready var units_root: Node3D = $Units
@onready var target_marker: MeshInstance3D = $TargetMarker

var _unit_nodes: Dictionary[int, Node3D] = {}
var _self_test := "--self-test" in OS.get_cmdline_user_args()
var _camera_shots := "--camera-shots" in OS.get_cmdline_user_args()
var _frame := 0
## Self-test results: null while running, otherwise an error message ("" = ok).
var _sim_error: Variant = null
var _camera_error: Variant = null


func _ready() -> void:
	camera_rig.bounds = Rect2(-MAP_SIZE * 0.5, MAP_SIZE)
	sun.look_at_from_position(Vector3(10, 20, 5), Vector3.ZERO, Vector3.UP)

	if not sim_host.start(SEED):
		push_error("SimHost failed to start")
		get_tree().quit(1)
		return
	print("Mud and Steel: simulation running at %d Hz" % sim_host.tick_rate())

	if _self_test:
		camera_rig.edge_scroll_enabled = false
		_send_first_unit_to(Vector3(24, 0, 32))
	if _camera_shots:
		camera_rig.edge_scroll_enabled = false


func _process(_delta: float) -> void:
	_frame += 1
	_draw_units()
	if _self_test:
		_check_self_test()
		_check_camera_self_test()
		if _sim_error != null and _camera_error != null:
			var error := "%s%s" % [_sim_error, _camera_error]
			_finish_self_test(error.is_empty(), error if error else "unit and camera ok")
	if _camera_shots:
		_record_camera_shots()


func _unhandled_input(event: InputEvent) -> void:
	var click := event as InputEventMouseButton
	if click == null or not click.pressed or click.button_index != MOUSE_BUTTON_RIGHT:
		return
	var hit: Variant = GroundPick.from_screen(camera, click.position)
	if hit == null:
		return
	_send_first_unit_to(hit as Vector3)


func _send_first_unit_to(point: Vector3) -> void:
	var ids := sim_host.unit_ids()
	if ids.is_empty():
		return
	sim_host.move_to(ids[0], point.x, point.z)
	target_marker.position = Vector3(point.x, 0.02, point.z)
	target_marker.visible = true


## Places every unit between its previous and current tick position.
func _draw_units() -> void:
	var ids := sim_host.unit_ids()
	var previous := sim_host.previous_positions()
	var current := sim_host.current_positions()
	var alpha := sim_host.interpolation_alpha()
	for i in ids.size():
		var node := _unit_node(ids[i])
		var from: Vector3 = previous[i] if i < previous.size() else current[i]
		var to: Vector3 = current[i]
		node.position = from.lerp(to, alpha)
		var heading := to - from
		if heading.length_squared() > 0.000001:
			# Models face -Z; turn them towards the driving direction.
			node.rotation.y = atan2(-heading.x, -heading.z)


func _unit_node(id: int) -> Node3D:
	if not _unit_nodes.has(id):
		var node := UNIT_SCENE.instantiate() as Node3D
		node.name = "Unit%d" % id
		units_root.add_child(node)
		_unit_nodes[id] = node
	return _unit_nodes[id]


## Headless check with the demo Panzer IV: 16 km/h cross-country (catalog)
## times the time factor 2 is 8.889 m/s, so 40 ticks at 20 Hz give 17.78 m, and
## the unit must stop exactly on the 40 m distant target (reached at tick 90).
func _check_self_test() -> void:
	if _sim_error != null:
		return
	var tick := sim_host.current_tick()
	var unit_position := sim_host.current_positions()[0]
	if tick == 40:
		var driven := unit_position.length()
		print("self-test: tick 40, driven %.3f m, alpha %.2f" % [driven, sim_host.interpolation_alpha()])
		if absf(driven - 17.78) > 0.25:
			_sim_error = "expected ~17.78 m after 40 ticks. "
	elif tick >= 100:
		print("self-test: tick %d, position %s, hash %s" % [tick, unit_position, sim_host.state_hash()])
		var ok := unit_position.distance_to(Vector3(24, 0, 32)) < 0.001
		_sim_error = "" if ok else "unit missed target. "


## Headless check of the camera: wheel input towards screen corners and a long
## diagonal pan must keep distance and focus inside their limits. Frame numbers
## assume `--fixed-fps 60`.
func _check_camera_self_test() -> void:
	if _camera_error != null:
		return
	var error := ""
	match _frame:
		1:
			_push_wheel(MOUSE_BUTTON_WHEEL_UP, 40, Vector2(0.1, 0.1))
		90:
			error = _camera_limit_error(camera_rig.min_distance)
			_push_wheel(MOUSE_BUTTON_WHEEL_DOWN, 80, Vector2(0.9, 0.8))
		180:
			error = _camera_limit_error(camera_rig.max_distance)
			Input.action_press("camera_pan_right")
			Input.action_press("camera_pan_forward")
		480:
			Input.action_release("camera_pan_right")
			Input.action_release("camera_pan_forward")
			error = _camera_limit_error(camera_rig.max_distance)
			var corner := Vector3(camera_rig.bounds.end.x, 0.0, camera_rig.bounds.position.y)
			if camera_rig.focus.distance_to(corner) > 0.01:
				error += "pan did not stop at the map corner %s. " % camera_rig.focus
			var rotate := InputEventAction.new()
			rotate.action = "camera_rotate_left"
			rotate.pressed = true
			get_viewport().push_input(rotate)
		540:
			if absf(camera_rig.yaw - PI / 4.0) > 0.001:
				error = "rotation did not reach 45 degrees (%.4f). " % camera_rig.yaw
			_camera_error = ""
	if _frame in [90, 180, 480, 540]:
		print("self-test: camera frame %d, distance %.2f, focus %s, yaw %.3f" % [
			_frame, camera_rig.distance, camera_rig.focus, camera_rig.yaw
		])
	if error:
		_camera_error = error


func _camera_limit_error(expected_distance: float) -> String:
	var error := ""
	if absf(camera_rig.distance - expected_distance) > 0.01:
		error += "distance %.3f, expected %.3f. " % [camera_rig.distance, expected_distance]
	var focus := Vector2(camera_rig.focus.x, camera_rig.focus.z)
	if not camera_rig.bounds.grow(0.01).has_point(focus):
		error += "focus %s outside the map. " % focus
	return error


## `at` is relative to the viewport size (0..1).
func _push_wheel(button: MouseButton, notches: int, at: Vector2) -> void:
	for i in notches:
		var wheel := InputEventMouseButton.new()
		wheel.button_index = button
		wheel.position = at * get_viewport().get_visible_rect().size
		wheel.pressed = true
		get_viewport().push_input(wheel)


## Holds each view of SHOT_DISTANCES for FRAMES_PER_SHOT frames, then quits.
func _record_camera_shots() -> void:
	var shot := (_frame - 1) / FRAMES_PER_SHOT
	if shot >= SHOT_DISTANCES.size():
		get_tree().quit()
	elif (_frame - 1) % FRAMES_PER_SHOT == 0:
		camera_rig.snap_to(Vector3.ZERO, SHOT_DISTANCES[shot], deg_to_rad(30.0))


func _finish_self_test(ok: bool, message: String) -> void:
	_self_test = false
	print("self-test %s: %s" % ["PASSED" if ok else "FAILED", message])
	get_tree().quit(0 if ok else 1)
