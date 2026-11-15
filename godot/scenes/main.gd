extends Node3D
## Main scene: draws the simulation and hosts selection and orders.
##
## Everything game-relevant happens in SimHost (Rust). This script only reads
## snapshots, interpolates between the last two ticks, and draws selection and
## player colors. Input is handled by `selection/selection.gd`.
##
## On exit the match is written as a replay to `user://replays/`.
##
## Start with `-- --self-test` to drive camera, selection and orders
## automatically and quit with exit code 0 (pass) or 1 (fail); see
## `self_test.gd`. Start with `-- --camera-shots` (together with
## `--write-movie`) to record three fixed camera views.

const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const SelfTest := preload("res://scenes/self_test.gd")
const SEED := 42
## Size of the test ground in meters, centered on the origin. Prompt 010
## replaces this with the size of the loaded map.
const MAP_SIZE := Vector2(2000, 2000)
const REPLAY_DIR := "user://replays"
## Camera distances (meters) recorded by --camera-shots.
const SHOT_DISTANCES: Array[float] = [15.0, 120.0, 600.0]
const FRAMES_PER_SHOT := 30
## How long the target marker stays visible after an order, in seconds.
const MARKER_SECONDS := 1.0
## Selection ring radius relative to the unit footprint.
const RING_SCALE := 0.65
## Tint over the models of other players (until real markings exist).
const ENEMY_TINT := Color(0.85, 0.2, 0.15, 0.45)

@onready var sim_host: SimHost = $SimHost
@onready var camera_rig: Node3D = $RtsCamera
@onready var camera: Camera3D = $RtsCamera/Camera3D
@onready var sun: DirectionalLight3D = $Sun
@onready var units_root: Node3D = $Units
@onready var target_marker: MeshInstance3D = $TargetMarker
@onready var selection: Control = $Hud/Selection

var _unit_nodes: Dictionary[int, Node3D] = {}
var _camera_shots := "--camera-shots" in OS.get_cmdline_user_args()
var _frame := 0
var _marker_time_left := 0.0
var _enemy_material: StandardMaterial3D


func _ready() -> void:
	camera_rig.bounds = Rect2(-MAP_SIZE * 0.5, MAP_SIZE)
	sun.look_at_from_position(Vector3(10, 20, 5), Vector3.ZERO, Vector3.UP)
	camera_rig.snap_to(Vector3(0, 0, 40), 140.0)

	_enemy_material = StandardMaterial3D.new()
	_enemy_material.albedo_color = ENEMY_TINT
	_enemy_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_enemy_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED

	if not sim_host.start(SEED):
		push_error("SimHost failed to start")
		get_tree().quit(1)
		return
	print("Mud and Steel: simulation running at %d Hz, input delay %d tick(s)" % [
		sim_host.tick_rate(), sim_host.input_delay
	])

	selection.sim_host = sim_host
	selection.camera = camera
	selection.camera_rig = camera_rig
	selection.move_ordered.connect(_show_target_marker)

	if "--self-test" in OS.get_cmdline_user_args():
		camera_rig.edge_scroll_enabled = false
		var test := SelfTest.new()
		test.main = self
		add_child(test)
	if _camera_shots:
		camera_rig.edge_scroll_enabled = false


func _process(delta: float) -> void:
	_frame += 1
	_draw_units()
	if target_marker.visible:
		_marker_time_left -= delta
		target_marker.visible = _marker_time_left > 0.0
	if _camera_shots:
		_record_camera_shots()


func _exit_tree() -> void:
	save_replay()


## Writes the match so far to `user://replays/<date-time>.msreplay` and
## returns the path, or "" on failure.
func save_replay() -> String:
	var bytes := sim_host.replay_bytes()
	if bytes.is_empty():
		return ""
	DirAccess.make_dir_recursive_absolute(REPLAY_DIR)
	var stamp := Time.get_datetime_string_from_system().replace(":", "-")
	var path := "%s/%s.msreplay" % [REPLAY_DIR, stamp]
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("cannot write replay %s: %s" % [path, error_string(FileAccess.get_open_error())])
		return ""
	file.store_buffer(bytes)
	file.close()
	var absolute := ProjectSettings.globalize_path(path)
	print("replay written: %s (%d ticks)" % [absolute, sim_host.current_tick()])
	return absolute


func _show_target_marker(point: Vector3) -> void:
	target_marker.position = Vector3(point.x, 0.02, point.z)
	target_marker.visible = true
	_marker_time_left = MARKER_SECONDS


## Places every unit between its previous and current tick position.
func _draw_units() -> void:
	var ids := sim_host.unit_ids()
	var previous := sim_host.previous_positions()
	var current := sim_host.current_positions()
	var alpha := sim_host.interpolation_alpha()
	for i in ids.size():
		var node := _unit_node(ids[i], i)
		var from: Vector3 = previous[i] if i < previous.size() else current[i]
		var to: Vector3 = current[i]
		node.position = from.lerp(to, alpha)
		var heading := to - from
		if heading.length_squared() > 0.000001:
			# Models face -Z; turn them towards the driving direction.
			node.rotation.y = atan2(-heading.x, -heading.z)
		(node.get_node("SelectionRing") as Node3D).visible = selection.is_selected(ids[i])


func _unit_node(id: int, index: int) -> Node3D:
	if not _unit_nodes.has(id):
		var node := UNIT_SCENE.instantiate() as Node3D
		node.name = "Unit%d" % id
		units_root.add_child(node)
		_unit_nodes[id] = node
		var radius := sim_host.unit_footprints()[index] * RING_SCALE
		(node.get_node("SelectionRing") as Node3D).scale = Vector3(radius, 0.3, radius)
		if sim_host.unit_owners()[index] != sim_host.local_player():
			_tint(node, _enemy_material)
	return _unit_nodes[id]


func _tint(node: Node, material: Material) -> void:
	var mesh := node as MeshInstance3D
	if mesh != null and node.name != "SelectionRing":
		mesh.material_overlay = material
	for child in node.get_children():
		_tint(child, material)


## Holds each view of SHOT_DISTANCES for FRAMES_PER_SHOT frames, then quits.
func _record_camera_shots() -> void:
	var shot := (_frame - 1) / FRAMES_PER_SHOT
	if shot >= SHOT_DISTANCES.size():
		get_tree().quit()
	elif (_frame - 1) % FRAMES_PER_SHOT == 0:
		camera_rig.snap_to(Vector3.ZERO, SHOT_DISTANCES[shot], deg_to_rad(30.0))
