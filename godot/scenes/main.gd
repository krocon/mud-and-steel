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
##
## Lighting, haze and the biome look come from `world/atmosphere.gd`
## (prompt 015): the biome of the map (`map.toml`), overridden by
## `-- --biome=<forest|desert|winter>`; graphics quality by
## `-- --quality=<low|medium|high>`.
## The game starts in the main menu (`menu/main_menu.tscn`, prompt 027),
## which hands the chosen match over through `menu/match_setup.gd` and
## skips itself for the direct-start arguments above; `menu/game_menu.gd`
## adds the pause menu (Esc) and the result screen.
## LAN games start in `lan/lan_lobby.tscn` (`just lan`), which hands the
## started game over through `lan/lan_match.gd`; `lan/net_status.gd` then
## shows the connection state.
## Combat (prompt 017) is drawn by `world/combat_view.gd`: tracers, health
## bars, wrecks, turrets. The self-test starts with every unit holding fire.
## Fog of war (prompt 018): `world/fog_of_war.gd` darkens what the local
## player does not see; enemy units out of sight are not in the snapshot and
## their nodes are hidden here.
## Effects (prompt 019): `world/effects.gd` (smoke, fire, impacts, dust);
## `-- --effects-shots` / `--effects-bench` run `tools/effects_bench.gd`.

const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const SelfTest := preload("res://scenes/self_test.gd")
const DemoReel := preload("res://scenes/tools/demo_reel.gd")
const TerrainBench := preload("res://scenes/tools/terrain_bench.gd")
const DebugMenu := preload("res://scenes/debug/debug_menu.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const PathDebug := preload("res://scenes/debug/path_debug.gd")
const Atmosphere := preload("res://scenes/world/atmosphere.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const MapObjects := preload("res://scenes/world/map_objects.gd")
const Water := preload("res://scenes/world/water.gd")
const ObjectsBench := preload("res://scenes/tools/objects_bench.gd")
const LanMatch := preload("res://scenes/lan/lan_match.gd")
const NetStatus := preload("res://scenes/lan/net_status.gd")
const CombatView := preload("res://scenes/world/combat_view.gd")
const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const Effects := preload("res://scenes/world/effects.gd")
const EffectsBench := preload("res://scenes/tools/effects_bench.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const LoadingScreen := preload("res://scenes/menu/loading_screen.gd")
const GameMenu := preload("res://scenes/menu/game_menu.gd")
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
## Water surfaces of rivers (prompt 014).
var water: Water
var _unit_nodes: Dictionary[int, Node3D] = {}
var _camera_shots := "--camera-shots" in OS.get_cmdline_user_args()
var _frame := 0
var _marker_time_left := 0.0
var _enemy_material: StandardMaterial3D
## Lighting, haze and biome look (prompt 015).
var atmosphere: Atmosphere
## Tracers, health bars, wrecks (prompt 017).
var combat_view: CombatView
## Fog overlay (prompt 018).
var fog_of_war: FogOfWar
## Smoke, fire, impacts, dust (prompt 019).
var effects: Effects
## Pause menu and result screen (prompt 027).
var game_menu: CanvasLayer
var _loading: CanvasLayer
var _loaded := false


func _ready() -> void:
	_enemy_material = StandardMaterial3D.new()
	_enemy_material.albedo_color = ENEMY_TINT
	_enemy_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_enemy_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED

	Settings.ensure_loaded()
	# Matches from the menu or the LAN lobby are built in steps behind a
	# loading screen (prompt 027); direct starts load in one frame as before.
	if not MatchSetup.pending.is_empty() or LanMatch.lobby != null:
		var map_name: String = MatchSetup.pending.get("map", "LAN match")
		_loading = LoadingScreen.new(map_name)
		add_child(_loading)
		sim_host.set_process(false)
		$Hud.process_mode = Node.PROCESS_MODE_DISABLED
		await _loading_step("Preparing the battlefield …", 0.05)
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
	await _loading_step("Building the terrain …", 0.35)
	terrain.build(sim_host)
	await _loading_step("Setting the weather …", 0.6)
	atmosphere = Atmosphere.new()
	atmosphere.name = "Atmosphere"
	add_child(atmosphere)
	atmosphere.setup($WorldEnvironment, sun, camera_rig, terrain)
	atmosphere.quality = Settings.quality()
	var biome: String = MatchSetup.pending.get("biome", "")
	atmosphere.set_biome(biome if not biome.is_empty() else Atmosphere.biome_from(sim_host.map_biome()))
	MatchSetup.pending = {}
	print("Mud and Steel: biome '%s', graphics quality '%s'" % [atmosphere.preset.id, atmosphere.quality])
	await _loading_step("Placing woods and villages …", 0.7)
	map_objects = MapObjects.new()
	map_objects.name = "MapObjects"
	add_child(map_objects)
	map_objects.biome = atmosphere.preset.env_biome
	map_objects.build(sim_host, camera)
	water = Water.new()
	water.name = "Water"
	add_child(water)
	water.build(sim_host)
	water.apply_biome(atmosphere.preset)
	fog_of_war = FogOfWar.new()
	fog_of_war.name = "FogOfWar"
	add_child(fog_of_war)
	fog_of_war.setup(sim_host, camera)

	combat_view = CombatView.new()
	combat_view.name = "CombatView"
	add_child(combat_view)
	combat_view.setup(self, $Hud)
	effects = Effects.new()
	effects.name = "Effects"
	add_child(effects)
	effects.setup(self)
	# Effects only appear where the local player currently sees (255 =
	# visible); without vision data everything is visible.
	effects.visible_at = func(point: Vector3) -> bool:
		var state := fog_of_war.state_at(point)
		return state < 0 or state == 255

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

	if sim_host.is_networked():
		var net_status := NetStatus.new()
		net_status.main = self
		$Hud.add_child(net_status)

	# Pause menu and result screen (prompt 027).
	Settings.apply_camera(camera_rig)
	game_menu = GameMenu.new()
	game_menu.name = "GameMenu"
	game_menu.main = self
	add_child(game_menu)
	if _loading != null:
		_loading.set_step("Ready.", 1.0)
		await get_tree().process_frame
		_loading.finish()
		_loading = null
		sim_host.set_process(true)
		$Hud.process_mode = Node.PROCESS_MODE_INHERIT
	_loaded = true

	if "--self-test" in OS.get_cmdline_user_args() and _wants_match_test():
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
	if _effects_tool():
		var effects_bench := EffectsBench.new()
		effects_bench.main = self
		add_child(effects_bench)
	if "--demo" in OS.get_cmdline_user_args():
		var demo := DemoReel.new()
		demo.main = self
		add_child(demo)


func _process(delta: float) -> void:
	if not _loaded:
		return
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


## Shows a loading step and lets a frame be drawn (only behind the loading
## screen).
func _loading_step(text: String, fraction: float) -> void:
	if _loading == null:
		return
	_loading.set_step(text, fraction)
	await get_tree().process_frame
	await get_tree().process_frame


## In the menu flow self-test only the first match runs the match tests.
func _wants_match_test() -> bool:
	if MatchSetup.flow_test == null:
		return true
	var wanted := MatchSetup.run_match_test
	MatchSetup.run_match_test = false
	return wanted


## Whether `--effects-bench` or `--effects-shots[=<dir>]` was given.
func _effects_tool() -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg == "--effects-bench" or arg.begins_with("--effects-shots"):
			return true
	return false


## Starts the built-in map, or the one named by `--map=<name>`, or the LAN
## match the lobby started, or the match chosen in the menu.
func _start_simulation() -> bool:
	# The self-test checks movement first; combat is tested at its end.
	sim_host.hold_fire_at_start = "--self-test" in OS.get_cmdline_user_args()
	# The effects tools trigger their own effects (prompt 019).
	sim_host.hold_fire_at_start = sim_host.hold_fire_at_start or _effects_tool()
	if not MatchSetup.pending.is_empty():
		var setup := MatchSetup.pending
		return sim_host.start_match(SEED, MatchSetup.map_dir(setup["map"]), setup["factions"])
	if LanMatch.lobby != null:
		var lobby := LanMatch.lobby
		LanMatch.lobby = null
		return sim_host.start_lan(lobby, ProjectSettings.globalize_path(MAPS_DIR).simplify_path())
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--map="):
			var dir := ProjectSettings.globalize_path(MAPS_DIR.path_join(arg.trim_prefix("--map=")))
			return sim_host.start_on_map(SEED, dir.simplify_path())
	return sim_host.start(SEED)


## Writes the match so far to `user://replays/<prefix><date-time>.msreplay`
## and returns the path, or "" on failure.
func save_replay(prefix := "") -> String:
	var bytes := sim_host.replay_bytes()
	if bytes.is_empty():
		return ""
	DirAccess.make_dir_recursive_absolute(REPLAY_DIR)
	var stamp := Time.get_datetime_string_from_system().replace(":", "-")
	var path := "%s/%s%s.msreplay" % [REPLAY_DIR, prefix, stamp]
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
	var facings := sim_host.unit_facing_yaws()
	var alive := sim_host.unit_alive()
	var drawn := {}
	for i in ids.size():
		drawn[ids[i]] = true
		var node := _unit_node(ids[i], i)
		if node.has_meta(&"fogged"):
			node.remove_meta(&"fogged")
			node.visible = true
		var from: Vector3 = previous[i] if i < previous.size() else current[i]
		var to: Vector3 = current[i]
		node.position = from.lerp(to, alpha)
		# Models face -Z; the hull follows the simulation's direction
		# (prompt 017), turned smoothly so small corrections do not jitter.
		var turn := clampf(get_process_delta_time() * TURN_RATE, 0.0, 1.0)
		if not node.has_meta(&"placed"):
			node.set_meta(&"placed", true)
			turn = 1.0
		node.rotation.y = lerp_angle(node.rotation.y, facings[i], turn)
		(node.get_node("SelectionRing") as Node3D).visible = selection.is_selected(ids[i]) and alive[i] != 0
	# Units out of the local player's sight are not in the snapshot (prompt 018).
	if drawn.size() < _unit_nodes.size():
		for id: int in _unit_nodes:
			if not drawn.has(id) and not _unit_nodes[id].has_meta(&"fogged"):
				_unit_nodes[id].set_meta(&"fogged", true)
				_unit_nodes[id].visible = false


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
## it with the faction palette of the biome (prompt 015).
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
	model.set_meta(&"faction", faction)
	node.add_child(model)
	UnitCamo.apply(model, UnitCamo.palette(faction, atmosphere.preset.camo_biome))
	UnitCamo.add_markings(model, faction, 100 + id)


## Switches the biome look at runtime (debug menu, self-test): lighting,
## ground and the palettes of all unit models. Presentation only.
func set_biome(id: String) -> void:
	atmosphere.set_biome(id)
	map_objects.set_biome(atmosphere.preset.env_biome)
	water.apply_biome(atmosphere.preset)
	for node: Node3D in _unit_nodes.values():
		var model := node.get_node_or_null("Model") as Node3D
		var faction: String = model.get_meta(&"faction", "") if model else ""
		if not faction.is_empty():
			UnitCamo.apply(model, UnitCamo.palette(faction, atmosphere.preset.camo_biome))


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
