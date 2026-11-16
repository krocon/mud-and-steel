extends Node
## Self-test of the map editor (prompt 028), started by the main menu with
## `-- --editor-self-test` (`just editor-selftest`). Lives on the scene tree
## root, so it survives the scene changes it drives. Quits with exit code 0
## (pass) or 1 (fail). Frame counts assume `--fixed-fps 60`.
##
## 1. Opens the editor from the main menu and creates a 256 m map.
## 2. Uses every tool through the same stroke functions as the mouse:
##    raise, lower, smooth, flatten, paint (road, ford), river, objects
##    brush, place (bridge, house), erase, both start positions; checks that
##    the terrain is redrawn per dab and objects and water after strokes.
## 3. Undo and redo (objects and heights), the check (playable), saving to
##    data/maps/editor_selftest/.
## 4. Back in the menu the map is in the single-player list; the match on it
##    starts with the units at the editor's start positions and the map hash
##    of the check; own tanks drive. Then the test map is deleted again.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MAP_NAME := "editor_selftest"
const TIMEOUT_MS := 300000

var _errors: PackedStringArray = []
var _started_ms := Time.get_ticks_msec()


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	_run()


func _process(_delta: float) -> void:
	if Time.get_ticks_msec() - _started_ms > TIMEOUT_MS:
		_check(false, "editor self-test timed out")
		_finish()


func _run() -> void:
	var menu: Control = await _wait_scene("MainMenu")
	if menu == null:
		_finish()
		return
	_check(menu.editor_button != null and menu.editor_button.is_visible_in_tree(), "no map editor button")
	menu.editor_button.pressed.emit()
	var scene: Node3D = await _wait_scene("MapEditorScene")
	if scene == null:
		_finish()
		return
	scene.open_new(MAP_NAME, 256, "forest")
	scene = await _wait_scene("MapEditorScene", scene)
	if scene == null:
		_finish()
		return
	var editor: MapEditor = scene.editor
	_check(editor.map_name() == MAP_NAME and editor.map_size() == Vector2(256, 256), "new map is %s %s" % [editor.map_name(), editor.map_size()])
	var hash_before: String = editor.validate()["map_hash"]
	await _tools_test(scene, editor)
	await _history_test(scene, editor)

	var report: Dictionary = scene.validate_map()
	_check(report["ok"], "the built map is not playable: %s" % report.get("errors"))
	_check(report["map_hash"] != hash_before, "the map hash did not change")
	# Default weather of the map (prompt 040): the match below plays in it.
	_check(editor.set_map_weather("fog", "evening"), "setting the map weather failed")
	_check(editor.map_weather() == PackedStringArray(["fog", "evening"]), "map weather %s" % editor.map_weather())
	_check(scene.save_as(MAP_NAME), "saving failed")
	var dir: String = scene.map_dir(MAP_NAME)
	_check(FileAccess.get_file_as_string(dir.path_join("map.toml")).contains("weather = \"fog\""), "map.toml without the weather")
	for file in ["map.toml", "height.png", "terrain.png", "objects.csv"]:
		_check(FileAccess.file_exists(dir.path_join(file)), "%s not written" % file)
	_check(scene.save_as("test_small") == false, "the editor overwrote test_small")

	scene.back_to_menu()
	menu = await _wait_scene("MainMenu")
	if menu == null:
		_finish()
		return
	await _play_test(menu, report["map_hash"])
	_finish()


func _tools_test(scene: Node3D, editor: MapEditor) -> void:
	var updates: int = scene.terrain.region_updates
	scene.radius = 25.0
	scene.strength = 0.5
	scene.tool = "raise"
	scene.stroke(_points([Vector2(-70, -70), Vector2(-68, -70), Vector2(-70, -68), Vector2(-72, -70)], editor), 0.5)
	var hill := editor.height_at(-70, -70)
	_check(hill > 1.0, "raise: hill only %.2f m" % hill)
	_check(scene.terrain.region_updates >= updates + 4, "terrain not redrawn per dab")
	var sample: Vector2i = scene.terrain.sample_of(Vector3(-70, 0, -70))
	_check(absf(scene.terrain.displayed_height(sample.x, sample.y) - hill) < 0.01, "terrain shows %.2f m instead of %.2f m" % [scene.terrain.displayed_height(sample.x, sample.y), hill])

	scene.radius = 15.0
	scene.tool = "lower"
	scene.stroke(_points([Vector2(-70, 70), Vector2(-70, 72)], editor), 0.5)
	_check(editor.height_at(-70, 70) < -0.5, "lower: %.2f m" % editor.height_at(-70, 70))
	scene.tool = "smooth"
	var peak := editor.height_at(-70, -70)
	scene.stroke(_points([Vector2(-70, -70), Vector2(-70, -70)], editor), 0.2)
	_check(editor.height_at(-70, -70) < peak, "smooth did not lower the peak")
	scene.tool = "flatten"
	scene.radius = 12.0
	var level := editor.height_at(-70, -58)
	scene.stroke(_points([Vector2(-70, -58), Vector2(-66, -58), Vector2(-74, -58)], editor), 1.0)
	_check(absf(editor.height_at(-66, -58) - level) < 0.05, "flatten: %.2f m, wanted %.2f m" % [editor.height_at(-66, -58), level])

	# Road from the house to the meadow, river with a ford and a bridge.
	scene.tool = "paint"
	scene.terrain_id = 2
	scene.radius = 4.0
	scene.stroke(_line(Vector2(60, 80), Vector2(10, 70), 2.0, editor))
	_check(editor.terrain_at(35, 75) == 2, "no road painted")
	scene.tool = "river"
	scene.radius = 10.0
	scene.strength = 0.5
	scene.stroke(_line(Vector2(90, -128), Vector2(90, 128), 3.0, editor))
	_check(editor.terrain_at(90, 20) == 7 and editor.height_at(90, 20) < -1.0, "river: terrain %d at %.2f m" % [editor.terrain_at(90, 20), editor.height_at(90, 20)])
	_check(scene.water.triangle_count > 0, "no water surface after the river")
	scene.tool = "paint"
	scene.terrain_id = 6
	scene.radius = 6.0
	scene.stroke(_line(Vector2(76, 0), Vector2(104, 0), 2.0, editor))
	_check(editor.terrain_at(90, 0) == 6, "no ford")
	scene.tool = "place"
	scene.place_kind = "env_bridge_stone:Arch2"
	scene.place_heading = 90
	scene.stroke(_points([Vector2(90, -62)], editor))

	# A wood, then thin it out.
	scene.tool = "scatter"
	scene.scatter_preset = "Deciduous wood"
	scene.density = 400.0
	scene.radius = 15.0
	scene.stroke(_line(Vector2(-70, 0), Vector2(-40, 0), 4.0, editor))
	var trees := editor.object_count()
	_check(trees > 15, "objects brush placed only %d trees" % trees)
	scene.tool = "erase"
	scene.radius = 5.0
	scene.stroke(_points([Vector2(-55, 0)], editor))
	_check(editor.object_count() < trees, "erase removed nothing")
	_check(scene.map_objects.object_instances == editor.object_count(), "objects drawn: %d of %d" % [scene.map_objects.object_instances, editor.object_count()])

	for player in 2:
		scene.tool = "start%d" % player
		scene.stroke(_points([Vector2(20, 70) if player == 0 else Vector2(-20, -70)], editor))
	var starts := editor.start_positions()
	_check(Vector2(starts[0].x, starts[0].z) == Vector2(20, 70) and Vector2(starts[1].x, starts[1].z) == Vector2(-20, -70), "start positions %s" % starts)
	_check(scene.has_node("Start0") and scene.has_node("Start1"), "start markers missing")
	# A third start gets the usual team (prompt 037); the panel changes it.
	scene.tool = "start2"
	scene.stroke(_points([Vector2(60, 70)], editor))
	_check(editor.start_teams() == {2: 0}, "start teams %s" % [editor.start_teams()])
	scene.set_start_team(1)
	_check(editor.start_teams() == {2: 1} and scene.has_node("Start2"), "start team not changed: %s" % [editor.start_teams()])
	await _frames(5)


func _history_test(scene: Node3D, editor: MapEditor) -> void:
	scene.tool = "place"
	scene.place_kind = "env_house_farm:Intact"
	scene.place_heading = 180
	scene.stroke(_points([Vector2(60, 86)], editor))
	var count := editor.object_count()
	var rebuilds: int = scene.world_rebuilds
	scene.undo()
	_check(editor.object_count() == count - 1, "undo kept the house")
	_check(scene.world_rebuilds > rebuilds and scene.map_objects.object_instances == count - 1, "objects not redrawn after undo")
	scene.redo()
	_check(editor.object_count() == count, "redo did not bring the house back")
	# Prompt 059: a compound and the village brush, each one undo step.
	scene.tool = "compound"
	scene.compound_name = "merchant_yard"
	scene.place_heading = 90
	scene.stroke(_points([Vector2(-60, 60)], editor))
	_check(editor.object_count() > count + 15, "compound placed %d objects" % (editor.object_count() - count))
	scene.undo()
	_check(editor.object_count() == count, "undo kept the compound")
	scene.tool = "village"
	scene.radius = 40.0
	scene.stroke(_points([Vector2(-60, 60)], editor))
	_check(editor.object_count() > count + 30, "village brush placed %d objects" % (editor.object_count() - count))
	scene.undo()
	_check(editor.object_count() == count, "undo kept the village")
	# Heights: undo a raise, the terrain shows the old height again.
	var before := editor.height_at(0, -20)
	scene.tool = "raise"
	scene.radius = 8.0
	scene.stroke(_points([Vector2(0, -20), Vector2(0, -20)], editor), 1.0)
	_check(editor.height_at(0, -20) > before + 0.5, "raise for undo failed")
	scene.undo()
	var sample: Vector2i = scene.terrain.sample_of(Vector3(0, 0, -20))
	_check(is_equal_approx(editor.height_at(0, -20), before), "undo did not restore the height")
	_check(absf(scene.terrain.displayed_height(sample.x, sample.y) - before) < 0.01, "terrain not redrawn after undo")
	_check(editor.redo_steps() == 1, "redo steps %d" % editor.redo_steps())
	await _frames(2)


func _play_test(menu: Control, map_hash: String) -> void:
	var maps := MatchSetup.available_maps()
	var index := maps.find(MAP_NAME)
	_check(index >= 0, "the saved map is not in the single-player list %s" % maps)
	if index < 0:
		return
	menu.show_screen("skirmish")
	await _frames(2)
	menu.map_option.select(index)
	menu.map_option.item_selected.emit(index)
	# The fixed armies of the demo (prompt 023 made base building the default).
	menu.mode_option.select(MatchSetup.MODES.keys().find("battle"))
	menu.start_button.pressed.emit()
	var main: Node3D = await _wait_scene("Main")
	if main == null:
		return
	for i in 600:
		if main._loaded:
			break
		await get_tree().process_frame
	var host: SimHost = main.sim_host
	_check(host.map_name() == MAP_NAME, "match runs on '%s'" % host.map_name())
	_check(host.map_hash() == map_hash, "match map hash %s, editor check %s" % [host.map_hash(), map_hash])
	var weather: Dictionary = host.weather_info()
	_check(weather.get("weather") == "fog" and weather.get("daytime") == "evening", "match weather %s, map default fog in the evening" % weather)
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var own := PackedInt64Array()
	for i in ids.size():
		if owners[i] == host.local_player():
			own.append(ids[i])
	var positions := host.current_positions()
	_check(not own.is_empty() and Vector2(positions[0].x - 20, positions[0].z - 70).length() < 30.0, "first unit at %s, start at (20, 70)" % positions[0])
	host.move_units(own, 20.0, 20.0)
	var tick := host.current_tick()
	await _frames(240)
	_check(host.current_tick() > tick + 60, "the match does not run (tick %d)" % host.current_tick())
	var moved := 0
	var now := host.current_positions()
	for i in ids.size():
		if owners[i] == host.local_player() and now[i].distance_to(positions[i]) > 2.0:
			moved += 1
	_check(moved >= 4, "only %d tanks drove" % moved)
	print("editor self-test: match on '%s' ok (map hash %s, %s in the %s, %d tanks drove, tick %d)" % [MAP_NAME, map_hash, weather.get("weather"), weather.get("daytime"), moved, host.current_tick()])


func _finish() -> void:
	set_process(false)
	_delete_map()
	var ok := _errors.is_empty()
	print("editor self-test %s: %s" % ["PASSED" if ok else "FAILED", "menu, tools, undo/redo, check, save, single-player match ok" if ok else " ".join(_errors)])
	get_tree().quit(0 if ok else 1)


func _delete_map() -> void:
	var dir := GamePaths.user_maps_dir().path_join(MAP_NAME)
	if not DirAccess.dir_exists_absolute(dir):
		return
	for file in DirAccess.get_files_at(dir):
		DirAccess.remove_absolute(dir.path_join(file))
	DirAccess.remove_absolute(dir)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_errors.append(message + ".")
		push_error("editor self-test: " + message)


## Ground points (with height) of map points.
func _points(points: Array[Vector2], editor: MapEditor) -> Array:
	var out := []
	for p in points:
		out.append(Vector3(p.x, editor.height_at(p.x, p.y), p.y))
	return out


func _line(a: Vector2, b: Vector2, step: float, editor: MapEditor) -> Array:
	var points: Array[Vector2] = []
	var n := maxi(ceili(a.distance_to(b) / step), 1)
	for k in n + 1:
		points.append(a.lerp(b, float(k) / n))
	return _points(points, editor)


## Waits for the scene `scene_name` (a new instance, not `old`).
func _wait_scene(scene_name: String, old: Node = null) -> Node:
	for i in 600:
		var scene := get_tree().current_scene
		if scene != null and scene != old and is_instance_valid(scene) and scene.name == scene_name and scene.is_node_ready():
			await _frames(2)
			return scene
		await get_tree().process_frame
	_check(false, "scene %s did not appear" % scene_name)
	return null


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
