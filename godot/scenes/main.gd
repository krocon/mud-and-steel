extends Node3D
## Main scene: draws the simulation and hosts selection and orders.
##
## Everything game-relevant happens in SimHost (Rust). This script only reads
## snapshots, interpolates between the last two ticks, and draws selection and
## player colors. Input is handled by `selection/selection.gd`.
##
## On exit the match is written as a replay to `user://replays/`.
##
## The match runs on the map compiled into the simulation (`test_small`);
## `-- --map=<name>` loads `data/maps/<name>/` from the repository instead
## (e.g. `--map=test_large`). The height field is drawn by
## `terrain/terrain.gd` (chunks with LOD and a splatting shader), the map
## objects and decoration by `world/map_objects.gd`.
##
## Start with `-- --self-test` to drive camera, selection and orders
## automatically and quit with exit code 0 (pass) or 1 (fail); see
## `self_test.gd`. Start with `-- --camera-shots` (together with
## `--write-movie`) to record three fixed camera views, or with `-- --demo` to
## play the scripted demo in `tools/demo_reel.gd`. `-- --terrain-bench`
## measures frame rate and draw calls (`tools/terrain_bench.gd`). F3 opens the
## debug menu (`debug/debug_menu.gd`); `-- --objects-bench` / `--objects-shots`
## measure and record the map objects (`tools/objects_bench.gd`); F4 / F5 show the path cost grid and the
## unit paths (`debug/path_debug.gd`; `-- --path-debug` starts with both on).

const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const SelfTest := preload("res://scenes/self_test.gd")
const DemoReel := preload("res://scenes/tools/demo_reel.gd")
const TerrainBench := preload("res://scenes/tools/terrain_bench.gd")
const DebugMenu := preload("res://scenes/debug/debug_menu.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const PathDebug := preload("res://scenes/debug/path_debug.gd")
const MapObjects := preload("res://scenes/world/map_objects.gd")
const ObjectsBench := preload("res://scenes/tools/objects_bench.gd")
const SEED := 42
## Map folders, relative to the Godot project (repository checkout only).
const MAPS_DIR := "res://../data/maps"
const REPLAY_DIR := "user://replays"
## Unit models built by the Blender kit, one folder per faction.
const MODELS_DIR := "res://assets/models/"
## Camera distances (meters) recorded by --camera-shots.
const SHOT_DISTANCES: Array[float] = [15.0, 120.0, 600.0]
const FRAMES_PER_SHOT := 30
## How long the target marker stays visible after an order, in seconds.
const MARKER_SECONDS := 1.0
## How fast models turn towards their driving direction (presentation only).
const TURN_RATE := 6.0
## Selection ring radius relative to the unit footprint.
const RING_SCALE := 0.65
## Tint over the models of other players (until real markings exist).
const ENEMY_TINT := Color(0.85, 0.2, 0.15, 0.22)

@onready var sim_host: SimHost = $SimHost
@onready var camera_rig: Node3D = $RtsCamera
@onready var camera: Camera3D = $RtsCamera/Camera3D
@onready var sun: DirectionalLight3D = $Sun
@onready var units_root: Node3D = $Units
@onready var target_marker: MeshInstance3D = $TargetMarker
@onready var terrain: Node3D = $Terrain
@onready var selection: Control = $Hud/Selection

## Trees, buildings, hedges and decoration (prompt 013).
var map_objects: Node3D
var _unit_nodes: Dictionary[int, Node3D] = {}
var _camera_shots := "--camera-shots" in OS.get_cmdline_user_args()
var _frame := 0
var _marker_time_left := 0.0
var _enemy_material: StandardMaterial3D


func _ready() -> void:
	sun.look_at_from_position(Vector3(10, 20, 5), Vector3.ZERO, Vector3.UP)

	_enemy_material = StandardMaterial3D.new()
	_enemy_material.albedo_color = ENEMY_TINT
	_enemy_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_enemy_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED

	if not _start_simulation():
		push_error("SimHost failed to start")
		get_tree().quit(1)
		return
	print("Mud and Steel: map '%s' (%s m), simulation running at %d Hz, input delay %d tick(s)" % [
		sim_host.map_name(), sim_host.map_size(), sim_host.tick_rate(), sim_host.input_delay
	])
	GroundPick.height_source = sim_host
	var map_size := sim_host.map_size()
	camera_rig.bounds = Rect2(-map_size * 0.5, map_size)
	camera_rig.snap_to(Vector3(0, 0, 40), 140.0)
	terrain.build(sim_host)
	map_objects = MapObjects.new()
	map_objects.name = "MapObjects"
	add_child(map_objects)
	map_objects.build(sim_host, camera)

	selection.sim_host = sim_host
	selection.camera = camera
	selection.camera_rig = camera_rig
	selection.move_ordered.connect(_show_target_marker)
	var debug_menu := DebugMenu.new()
	debug_menu.main = self
	$Hud.add_child(debug_menu)

	# Pathfinding debug view: F4 cost grid, F5 paths (prompt 012).
	var path_debug := PathDebug.new()
	path_debug.name = "PathDebug"
	path_debug.sim_host = sim_host
	path_debug.selection = selection
	add_child(path_debug)

	if "--self-test" in OS.get_cmdline_user_args():
		camera_rig.edge_scroll_enabled = false
		var test := SelfTest.new()
		test.main = self
		add_child(test)
	if _camera_shots:
		camera_rig.edge_scroll_enabled = false
	if "--terrain-bench" in OS.get_cmdline_user_args() or "--terrain-shots" in OS.get_cmdline_user_args():
		var bench := TerrainBench.new()
		bench.main = self
		add_child(bench)
	if "--objects-bench" in OS.get_cmdline_user_args() or "--objects-shots" in OS.get_cmdline_user_args():
		camera_rig.edge_scroll_enabled = false
		var objects_bench := ObjectsBench.new()
		objects_bench.main = self
		add_child(objects_bench)
	if "--demo" in OS.get_cmdline_user_args():
		var demo := DemoReel.new()
		demo.main = self
		add_child(demo)


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
	GroundPick.height_source = null


## Starts the built-in map, or the one named by `--map=<name>`.
func _start_simulation() -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--map="):
			var dir := ProjectSettings.globalize_path(MAPS_DIR.path_join(arg.trim_prefix("--map=")))
			return sim_host.start_on_map(SEED, dir.simplify_path())
	return sim_host.start(SEED)


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
	target_marker.position = point + Vector3(0, 0.05, 0)
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
			# Turned smoothly, so small sideways corrections do not jitter.
			var turn := clampf(get_process_delta_time() * TURN_RATE, 0.0, 1.0)
			node.rotation.y = lerp_angle(node.rotation.y, atan2(-heading.x, -heading.z), turn)
		(node.get_node("SelectionRing") as Node3D).visible = selection.is_selected(ids[i])


func _unit_node(id: int, index: int) -> Node3D:
	if not _unit_nodes.has(id):
		var node := UNIT_SCENE.instantiate() as Node3D
		node.name = "Unit%d" % id
		_use_catalog_model(node, sim_host.unit_type_ids()[index], id)
		units_root.add_child(node)
		_unit_nodes[id] = node
		var radius := sim_host.unit_footprints()[index] * RING_SCALE
		(node.get_node("SelectionRing") as Node3D).scale = Vector3(radius, 0.3, radius)
		if sim_host.unit_owners()[index] != sim_host.local_player():
			_tint(node, _enemy_material)
	return _unit_nodes[id]


## Replaces the placeholder with the unit's model from
## `res://assets/models/<faction>/<catalog id>.glb`, if one exists, and paints
## it with the faction palette (forest until maps choose a biome, prompt 015).
func _use_catalog_model(node: Node3D, type_id: String, id: int) -> void:
	var faction := type_id.get_slice("_", 0)
	var path := "%s%s/%s.glb" % [MODELS_DIR, faction, type_id]
	if type_id.is_empty() or not ResourceLoader.exists(path):
		return
	var placeholder := node.get_node("Model")
	node.remove_child(placeholder)
	placeholder.free()
	var model := (load(path) as PackedScene).instantiate() as Node3D
	model.name = "Model"
	node.add_child(model)
	UnitCamo.apply(model, UnitCamo.palette(faction, "forest"))
	UnitCamo.add_markings(model, faction, 100 + id)


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
