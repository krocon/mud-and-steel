extends Node3D
## Main scene: draws the simulation and turns right-clicks into move commands.
##
## Everything game-relevant happens in SimHost (Rust). This script only reads
## snapshots, interpolates between the last two ticks, and forwards input.
##
## Start with `-- --self-test` to drive the unit automatically and quit with
## exit code 0 (pass) or 1 (fail). Used for headless checks.

const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const SEED := 42
const CAMERA_OFFSET := Vector3(30, 30, 30)

@onready var sim_host: SimHost = $SimHost
@onready var camera: Camera3D = $Camera3D
@onready var sun: DirectionalLight3D = $Sun
@onready var units_root: Node3D = $Units
@onready var target_marker: MeshInstance3D = $TargetMarker

var _unit_nodes: Dictionary[int, Node3D] = {}
var _self_test := "--self-test" in OS.get_cmdline_user_args()


func _ready() -> void:
	# Equal offsets on all axes give the classic isometric angle
	# (35.26° elevation, 45° yaw).
	camera.look_at_from_position(CAMERA_OFFSET, Vector3.ZERO, Vector3.UP)
	sun.look_at_from_position(Vector3(10, 20, 5), Vector3.ZERO, Vector3.UP)

	if not sim_host.start(SEED):
		push_error("SimHost failed to start")
		get_tree().quit(1)
		return
	print("Mud and Steel: simulation running at %d Hz" % sim_host.tick_rate())

	if _self_test:
		_send_first_unit_to(Vector3(8, 0, 6))


func _process(_delta: float) -> void:
	_draw_units()
	if _self_test:
		_check_self_test()


func _unhandled_input(event: InputEvent) -> void:
	var click := event as InputEventMouseButton
	if click == null or not click.pressed or click.button_index != MOUSE_BUTTON_RIGHT:
		return
	var origin := camera.project_ray_origin(click.position)
	var direction := camera.project_ray_normal(click.position)
	var hit: Variant = Plane(Vector3.UP, 0.0).intersects_ray(origin, direction)
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


## Headless check: 4 m/s at 20 Hz must give 8 m after 40 ticks, and the unit
## must stop exactly on the 10 m distant target.
func _check_self_test() -> void:
	var tick := sim_host.current_tick()
	var unit_position := sim_host.current_positions()[0]
	if tick == 40:
		var driven := unit_position.length()
		print("self-test: tick 40, driven %.3f m, alpha %.2f" % [driven, sim_host.interpolation_alpha()])
		if absf(driven - 8.0) > 0.25:
			_finish_self_test(false, "expected ~8 m after 40 ticks")
	elif tick >= 80:
		print("self-test: tick %d, position %s, hash %s" % [tick, unit_position, sim_host.state_hash()])
		var ok := unit_position.distance_to(Vector3(8, 0, 6)) < 0.001
		_finish_self_test(ok, "unit stopped on target" if ok else "unit missed target")


func _finish_self_test(ok: bool, message: String) -> void:
	_self_test = false
	print("self-test %s: %s" % ["PASSED" if ok else "FAILED", message])
	get_tree().quit(0 if ok else 1)
