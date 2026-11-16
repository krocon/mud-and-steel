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
## 4. Hill: one tank is right-clicked onto the `hill_top` marker of the map
##    (mouse ray against the height field), drives up and must stand at the
##    ground height of the simulation, well above its start.
## 5. Terrain: chunks are visible at every zoom, the displayed heights match
##    the simulation's samples, and a debug crater rebuilds only its region
##    (displayed ground lowered, simulation unchanged).
## 6. Replay: the recorded match is saved, read back and re-simulated to the
##    same end hash.
## 8. Water and bridges (prompt 014, `tools/water_self_test.gd`): water
##    mesh and level, bridge models at the deck height, a tank drives over
##    the stone bridge at the deck height.
## 7. Map objects (prompt 013, runs before the terrain test's crater): every
##    object of the map is drawn once, round objects stand on the
##    simulation's ground, decoration appears near the camera and is the same
##    when scattered twice.
## 8. Combat (prompt 017, every unit starts holding fire): right click on an
##    enemy tank orders the attack; the own tanks drive into range, fire and
##    destroy it (shots, hits, a destroyed event, a darkened wreck, health
##    bars); the enemy never fires back; F switches the selection to fire at
##    will.
## 9. Destruction (prompt 020, `tools/destruction_self_test.gd`): the own
##    tanks shell a building into a ruin and fell a tree; variants and
##    effects follow the simulation, the replay re-simulates it.
## 10. Craters (prompt 021, `tools/crater_self_test.gd`): the own tanks
##    fire at open grass until a crater appears; drawn terrain, mud and
##    decoration follow the simulation, the replay re-simulates it.
## 11. Sound (prompt 029, `tools/sound_self_test.gd`): library, buses,
##    fog, engines and the voice limit under 200 events per frame.
##    Then music and ambience (prompt 030, `tools/music_self_test.gd`).
## 12. HUD (prompt 031, `tools/hud_self_test.gd`): minimap click, drag and
##    right click, selection through the unit panel, a waypoint chain by
##    Shift + right click, orders from the command bar, event jump, health
##    bar setting.
## 13. Transport (prompt 034, `tools/transport_self_test.gd`): a squad
##    mounts an own tank by right click and dismounts with J.

## Panzer IV H: 16 km/h cross-country (catalog) × time factor 2 = 8.889 m/s,
## i.e. 0.4444 m per tick at 20 Hz. Terrain scales it (data/terrain.toml).
const EXPECTED_STEP := 16.0 * 2.0 / 3.6 / 20.0
## Panzer IV H on roads: 38 km/h × 2 = 1.0556 m per tick.
const ROAD_STEP := 38.0 * 2.0 / 3.6 / 20.0
const OVERVIEW_DISTANCE := 350.0
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
## The hill drive runs at this time scale to keep the test short.
const HILL_TIME_SCALE := 4.0
## The tank must end at least this many meters above its start.
const HILL_MIN_CLIMB := 10.0

const WaterSelfTest := preload("res://scenes/tools/water_self_test.gd")
const InfantrySelfTest := preload("res://scenes/tools/infantry_self_test.gd")
const DestructionSelfTest := preload("res://scenes/tools/destruction_self_test.gd")
const CraterSelfTest := preload("res://scenes/tools/crater_self_test.gd")
const SoundSelfTest := preload("res://scenes/tools/sound_self_test.gd")
const MusicSelfTest := preload("res://scenes/tools/music_self_test.gd")
const HudSelfTest := preload("res://scenes/tools/hud_self_test.gd")
const TransportSelfTest := preload("res://scenes/tools/transport_self_test.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")

## Screen size for the test (see `_ready`).
const SCREEN := Vector2i(1600, 900)

var main: Node3D
var _errors: PackedStringArray = []


func _ready() -> void:
	# The headless window is 64 × 64 pixels, which the match HUD (prompt
	# 031) would cover completely: test on the project's window size.
	if get_tree().root.size.x < SCREEN.x:
		get_tree().root.size = SCREEN
	print("self-test: screen %s" % get_viewport().get_visible_rect().size)
	_run()


func _run() -> void:
	var hud_test := HudSelfTest.new()
	hud_test.remember(self)
	await _camera_test()
	await _selection_test()
	await _controls_test()
	await _order_test()
	await _hill_test()
	await _objects_test()
	await _terrain_test()
	await _biome_test()
	await WaterSelfTest.new().run(self)
	await _fog_test()
	await _combat_test()
	await InfantrySelfTest.new().run(self)
	await DestructionSelfTest.new().run(self)
	await CraterSelfTest.new().run(self)
	await SoundSelfTest.new().run(self)
	await MusicSelfTest.new().run(self)
	await hud_test.run(self)
	await TransportSelfTest.new().run(self)
	_replay_test()
	var ok := _errors.is_empty()
	var message := "camera, selection, controls, orders, hill, objects, terrain, biomes, fog, combat, infantry, destruction, craters, sound, music, water, hud, transport and replay ok" if ok else " ".join(_errors)
	print("self-test %s: %s" % ["PASSED" if ok else "FAILED", message])
	# Started from the menu flow test (prompt 027): it continues the flow.
	var flow: Node = MatchSetup.flow_test
	if flow != null:
		flow.match_test_finished(ok, message)
		queue_free()
		return
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
	var corner := Vector2(rig.bounds.end.x, rig.bounds.position.y)
	var focus := Vector2(rig.focus.x, rig.focus.z)
	_check(focus.distance_to(corner) < 0.01, "pan did not stop at the map corner %s" % rig.focus)
	var ground: float = main.sim_host.height_at(rig.focus.x, rig.focus.z)
	_check(absf(rig.focus.y - ground) < 0.001, "camera focus %s not on the ground (%.3f)" % [rig.focus, ground])
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
	# Rifle squads in the box are taken too (prompt 022).
	_check(_tanks(selection.selected) == own, "box selected %s, expected own tanks %s" % [selection.selected, own])

	_key(KEY_1, true)
	_left_click(_screen_of_point(Vector3.ZERO))
	await _frames(1)
	_check(selection.selected.is_empty(), "click on empty ground kept %s" % [selection.selected])

	_key(KEY_1)
	await _frames(1)
	_check(_tanks(selection.selected) == own, "group 1 recalled %s" % [selection.selected])
	_key(KEY_1)
	await _frames(1)
	var centroid := _centroid(selection.selected)
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


# --- 2b. Map controls (trackpad, Option drag, double click, follow) ---------


## Expects the selection test's end state (all own units selected) and leaves
## it unchanged; issues no orders, so the simulation is not affected.
func _controls_test() -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	var own := _ids_of(true)
	var center := get_viewport().get_visible_rect().size * 0.5

	# Two-finger trackpad pan: fingers move down, the map follows them, so
	# the camera looks further north (-z).
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	var pan := InputEventPanGesture.new()
	pan.position = center
	pan.delta = Vector2(0.0, -2.0)
	get_viewport().push_input(pan)
	await _frames(1)
	_check(rig.focus.z < -5.0, "trackpad pan did not move the map north (focus %s)" % rig.focus)

	# Option/Alt held while moving the mouse grabs the map: moving right
	# looks further west (-x).
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	var motion := InputEventMouseMotion.new()
	motion.position = center
	motion.global_position = center
	motion.relative = Vector2(80.0, 0.0)
	motion.alt_pressed = true
	get_viewport().push_input(motion)
	await _frames(1)
	_check(rig.focus.x < -5.0, "Option drag did not move the map (focus %s)" % rig.focus)

	# Double click on empty ground: the camera glides there and the
	# selection survives the first click of the double click.
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	var spot := center + Vector2(0.0, -center.y * 0.8)
	var ground: Variant = GroundPick.from_screen(main.camera, spot)
	_check(ground != null, "no ground under the double click point")
	_left_click(spot)
	_left_click(spot, false, true)
	await _frames(1)
	_check(selection.selected == own, "double click on ground changed the selection to %s" % [selection.selected])
	await _frames(90)
	if ground != null:
		var target := ground as Vector3
		var off := Vector2(rig.focus.x - target.x, rig.focus.z - target.z).length()
		_check(off < 1.0, "double click did not center the camera (%.2f m off)" % off)

	# Follow mode (C): the camera moves to the selection's center and stops
	# following as soon as the map is moved by hand.
	rig.snap_to(Vector3(200.0, 0.0, 200.0), OVERVIEW_DISTANCE, 0.0)
	_key(KEY_C)
	await _frames(90)
	_check(selection.follow, "follow mode did not turn on")
	var centroid := _centroid(own)
	var follow_off := Vector2(rig.focus.x - centroid.x, rig.focus.z - centroid.z).length()
	_check(follow_off < 1.0, "follow mode did not center the selection (%.2f m off)" % follow_off)
	get_viewport().push_input(pan)
	await _frames(1)
	_check(not selection.follow, "a hand pan did not end follow mode")
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	print("self-test: map controls ok (trackpad, Option drag, double click, follow)")


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

	# Follow the group tick by tick until it stands. The start meadow is
	# flat, so every step is the terrain speed under the tank (shorter only
	# on arrival); the steepest grass step must be the plain catalog speed.
	var max_step := 0.0
	var too_fast := ""
	var max_grass_step := 0.0
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
			var step := previous[i].distance_to(current[i])
			max_step = maxf(max_step, step)
			var terrain := sim_host.terrain_at(previous[i].x, previous[i].z)
			var basis := ROAD_STEP if sim_host.terrain_uses_road(terrain) else EXPECTED_STEP
			var allowed := minf(basis * sim_host.terrain_speed_pct(terrain) / 100.0, ROAD_STEP)
			if step > allowed + 0.002:
				too_fast = "step %.4f m on terrain %d, allowed %.4f m" % [step, terrain, allowed]
			if terrain == 0:
				max_grass_step = maxf(max_grass_step, step)
		var moving := _moving(own)
		started = started or moving
		if started and not moving:
			break
	_check(started and not _moving(own), "group did not arrive (started %s)" % started)
	_check(too_fast.is_empty(), too_fast)
	_check(absf(max_grass_step - EXPECTED_STEP) < 0.002,
		"max step per tick on grass %.4f m, expected %.4f m" % [max_grass_step, EXPECTED_STEP])

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


# --- 4. Hill --------------------------------------------------------------


func _hill_test() -> void:
	var sim_host: SimHost = main.sim_host
	var rig: Node3D = main.camera_rig
	var marker: Variant = sim_host.map_marker("hill_top")
	_check(marker != null, "map '%s' has no hill_top marker" % sim_host.map_name())
	if marker == null:
		return
	var top := marker as Vector3
	var tank: int = _ids_of(true)[0]
	var start := _positions_of([tank])[0]
	rig.snap_to((start + top) * 0.5, 450.0, 0.0)
	await _frames(2)

	_left_click(_screen_of_unit(tank))
	await _frames(1)
	_check(main.selection.selected == [tank], "hill test selected %s" % [main.selection.selected])
	_right_click(_screen_of_point(top))
	await _frames(1)
	var aimed: Vector3 = main.target_marker.position - Vector3(0, 0.05, 0)
	_check(aimed.distance_to(top) < 0.5, "mouse ray hit %s, expected the hill top %s" % [aimed, top])

	Engine.time_scale = HILL_TIME_SCALE
	var deadline := sim_host.current_tick() + 20 * 180
	var started := false
	while sim_host.current_tick() < deadline:
		await get_tree().process_frame
		var moving := _moving([tank])
		started = started or moving
		if started and not moving:
			break
	Engine.time_scale = 1.0

	var end := _positions_of([tank])[0]
	var ground := sim_host.height_at(end.x, end.z)
	_check(started and not _moving([tank]), "tank did not finish the hill drive (started %s)" % started)
	_check(Vector2(end.x, end.z).distance_to(Vector2(top.x, top.z)) < 0.5,
		"tank stopped at %s, hill top is %s" % [end, top])
	_check(absf(end.y - ground) < 0.01, "tank height %.3f, ground %.3f" % [end.y, ground])
	_check(end.y > start.y + HILL_MIN_CLIMB, "tank climbed only from %.2f to %.2f m" % [start.y, end.y])
	print("self-test: hill drive from %s to %s (ground %.3f m) at tick %d" % [
		start, end, ground, sim_host.current_tick()
	])


# --- 5. Terrain -----------------------------------------------------------


func _terrain_test() -> void:
	var sim_host: SimHost = main.sim_host
	var rig: Node3D = main.camera_rig
	var terrain: Node3D = main.terrain
	for distance: float in [rig.min_distance, 120.0, rig.max_distance]:
		rig.snap_to(Vector3(0, 0, 40), distance, 0.0)
		await _frames(2)
		var visible := 0
		for count in terrain.visible_per_lod:
			visible += count
		_check(visible > 0, "no terrain chunk visible at %.0f m" % distance)
		# Close up the chunk under the camera uses full detail.
		if distance == rig.min_distance:
			_check(terrain.visible_per_lod[0] > 0, "no full-detail chunk at %.0f m" % distance)
	var marker: Variant = sim_host.map_marker("hill_top")
	var point: Vector3 = marker if marker != null else Vector3(30, 0, 30)
	var s: Vector2i = terrain.sample_of(point)
	# Compare on the sample itself: between samples the mesh and the
	# simulation interpolate differently (triangles versus bilinear).
	var half: Vector2 = sim_host.map_size() * 0.5
	var cell: float = sim_host.cell_size()
	point = Vector3(s.x * cell - half.x, 0.0, s.y * cell - half.y)
	var before: float = terrain.displayed_height(s.x, s.y)
	_check(absf(before - sim_host.height_at(point.x, point.z)) < 0.01,
		"displayed height %.3f differs from the simulation %.3f" % [before, sim_host.height_at(point.x, point.z)])
	var updates: int = terrain.region_updates
	terrain.debug_crater(point, 6.0, 1.5)
	await _frames(1)
	var after: float = terrain.displayed_height(s.x, s.y)
	_check(terrain.region_updates == updates + 1, "debug crater did not rebuild its region")
	_check(absf(before - 1.5 - after) < 0.05, "crater center %.3f m, expected %.3f m" % [after, before - 1.5])
	var far: Vector2i = s + Vector2i(40, 0)
	_check(absf(terrain.displayed_height(far.x, far.y) - sim_host.height_at(point.x + 80.0, point.z)) < 0.01,
		"crater changed the ground 80 m away")
	_check(absf(sim_host.height_at(point.x, point.z) - before) < 0.01, "debug crater changed the simulation")
	print("self-test: terrain ok (crater %.2f -> %.2f m, chunks per LOD %s)" % [before, after, terrain.visible_per_lod])


# --- 6. Biomes (prompt 015) -------------------------------------------------


## The map's biome is applied at start; switching it changes lighting,
## ground and unit palettes (the simulation does not know the biome beyond
## map.toml; its hash is checked by the replay test below).
func _biome_test() -> void:
	var sim_host: SimHost = main.sim_host
	var atmosphere: Node = main.atmosphere
	var start_biome: String = atmosphere.preset.id
	var expected: String = atmosphere.biome_from(sim_host.map_biome())
	_check(start_biome == expected, "biome '%s' at start, map says '%s'" % [start_biome, sim_host.map_biome()])
	var unit: Node3D = main.units_root.get_child(0)
	var model := unit.get_node_or_null("Model")
	var faction: String = model.get_meta(&"faction", "") if model else ""
	for id: String in ["desert", "winter", "forest"]:
		main.set_biome(id)
		await _frames(2)
		var preset: Resource = atmosphere.preset
		_check(preset.id == id, "preset '%s' after switching to '%s'" % [preset.id, id])
		_check(main.terrain.shader_parameter(&"layer_source") == preset.layer_source,
			"terrain layer sources do not follow the '%s' preset" % id)
		_check(is_equal_approx(main.terrain.shader_parameter(&"snow_cover"), preset.snow_cover),
			"terrain snow cover does not follow the '%s' preset" % id)
		var env: Environment = main.get_node("WorldEnvironment").environment
		_check(env.fog_enabled and env.fog_depth_end > env.fog_depth_begin, "no haze in '%s'" % id)
		_check(main.sun.shadow_enabled, "sun casts no shadows in '%s'" % id)
		_check(main.map_objects.biome == preset.env_biome, "map objects do not follow the '%s' biome" % id)
		# First child of MapObjects holds the object MultiMeshes.
		var objects_root: Node = main.map_objects.get_child(0)
		if objects_root.get_child_count() > 0:
			var obj := objects_root.get_child(0) as GeometryInstance3D
			_check(obj.get_instance_shader_parameter(&"env_snow") == EnvLook.LOOK[preset.env_biome].snow,
				"map object look does not follow the '%s' biome" % id)
		if not faction.is_empty():
			var mesh: GeometryInstance3D = UnitCamo.meshes(model)[0]
			var base: Color = mesh.get_instance_shader_parameter(&"camo_base")
			_check(base == UnitCamo.palette(faction, preset.camo_biome).base,
				"unit palette does not follow the '%s' biome" % id)
	main.set_biome(start_biome)
	_check(faction != "", "first unit has no catalog model")
	print("self-test: biomes ok (start '%s', switched desert, winter, forest)" % start_biome)


# --- 7. Map objects ---------------------------------------------------------


func _objects_test() -> void:
	var sim_host: SimHost = main.sim_host
	var objects: Node3D = main.map_objects
	var count := sim_host.object_count()
	_check(count > 0, "map '%s' has no objects" % sim_host.map_name())
	_check(objects.object_instances == count,
		"%d object instances drawn, the map has %d" % [objects.object_instances, count])
	# Trees are sunk 0.15 m, other round objects stand on the ground; boxes
	# stand at the lowest ground under them (not checked here).
	var kinds := sim_host.object_kinds()
	var checked := 0
	for group: Dictionary in sim_host.object_groups(128.0):
		var kind: String = kinds[group.kind]
		var sink := 0.15 if kind.begins_with("env_tree_") else 0.0
		if not (kind.begins_with("env_tree_") or kind.begins_with("env_pole") or kind.begins_with("env_rock")):
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
			var ground := sim_host.height_at(p.x, p.z)
			if absf(p.y + sink - ground) > 0.01:
				_check(false, "%s at %s stands at %.3f m, ground %.3f m" % [kind, p, p.y, ground])
				return
			checked += 1
	_check(checked > 0, "no round objects checked")
	# Decoration near the camera, the same when scattered again.
	main.camera_rig.snap_to(Vector3(-6, 0, 44), main.camera_rig.min_distance, 0.0)
	await _frames(30)
	_check(objects.deco_instances > 0, "no decoration near the camera")
	var a := sim_host.scatter_deco(0.0, 0.0, 64.0, 1.0, 5, PackedFloat32Array([0.8]), 1.0, 0.5,
		PackedFloat32Array([1.0]), Vector2(1, 1), 0.0)
	var b := sim_host.scatter_deco(0.0, 0.0, 64.0, 1.0, 5, PackedFloat32Array([0.8]), 1.0, 0.5,
		PackedFloat32Array([1.0]), Vector2(1, 1), 0.0)
	_check(a == b and not (a[0] as PackedFloat32Array).is_empty(), "decoration differs between two scatters")
	print("self-test: objects ok (%d objects, %d on the ground checked, %d decoration instances)" % [
		count, checked, objects.deco_instances
	])


# --- 8. Combat -------------------------------------------------------------


## Fog of war (prompt 018): the overlay is drawn, the local player's units
## stand in visible cells, units out of sight are hidden, and revealing
## (debug) brings every unit into the snapshot.
func _fog_test() -> void:
	var sim_host: SimHost = main.sim_host
	var fog: Node3D = main.fog_of_war
	var overlay := main.camera.get_node_or_null("FogOverlay") as MeshInstance3D
	_check(overlay != null and overlay.visible, "fog overlay missing or hidden")
	_check(fog.updates > 0, "fog texture never uploaded")
	var size := sim_host.vision_grid_size()
	_check(size.x > 0 and size.y > 0 and sim_host.vision_image().size() == size.x * size.y,
		"vision image does not match the grid %s" % size)
	var own := _ids_of(true)
	for p in _positions_of(own):
		if fog.state_at(p) != 255:
			_check(false, "own unit at %s stands in a fogged cell (%d)" % [p, fog.state_at(p)])
			break
	var seen := sim_host.unit_ids().size()
	await _frames(1)
	for id: int in main._unit_nodes:
		var node: Node3D = main._unit_nodes[id]
		if not (id in sim_host.unit_ids()):
			_check(not node.visible, "unit %d is out of sight but drawn" % id)
	fog.set_fog_enabled(false)
	await _frames(2)
	var all := sim_host.unit_ids().size()
	_check(not overlay.visible, "fog overlay drawn while revealed")
	_check(all >= seen and sim_host.vision_image().count(255) == size.x * size.y,
		"reveal all shows %d of %d units" % [all, seen])
	fog.set_fog_enabled(true)
	await _frames(2)
	_check(overlay.visible and sim_host.unit_ids().size() <= all, "fog not back after revealing")
	print("self-test: fog ok (%d x %d cells, %.0f %% visible, %.0f %% explored, %d of %d units in sight)" % [
		size.x, size.y, fog.share_of(255) * 100.0, fog.share_of(128) * 100.0, seen, all
	])


func _combat_test() -> void:
	var sim_host: SimHost = main.sim_host
	var view: Node3D = main.combat_view
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var shots_by: Dictionary[int, int] = {}
	var on_event := func(event: Dictionary) -> void:
		if event["type"] == "shot":
			var owner: int = sim_host.unit_owners()[_indices([event["unit"]])[0]]
			shots_by[owner] = shots_by.get(owner, 0) + 1
	view.combat_event.connect(on_event)
	_check(view.shots == 0, "%d shots before the combat test (everyone holds fire)" % view.shots)

	var target := enemies[0]
	main.camera_rig.snap_to(_positions_of([target])[0], 160.0, 0.0)
	await _frames(2)
	main.selection._set_selection(own)
	_right_click(_screen_of_unit(target))
	var deadline := sim_host.current_tick() + 20 * 60
	var bars := 0
	while sim_host.current_tick() < deadline and sim_host.unit_alive()[_indices([target])[0]] != 0:
		await get_tree().process_frame
		bars = maxi(bars, view.health_bars.drawn)
	await _frames(2)
	var alive: PackedByteArray = sim_host.unit_alive()
	_check(alive[_indices([target])[0]] == 0, "attacked tank %d not destroyed by tick %d" % [target, sim_host.current_tick()])
	_check(view.shots > 0 and view.hits > 0 and view.penetrations > 0,
		"combat events missing: %d shots, %d hits, %d penetrations" % [view.shots, view.hits, view.penetrations])
	_check(view.destroyed >= 1, "no destroyed event")
	_check(view.wrecks.has(target), "tank %d is not drawn as a wreck" % target)
	_check(bars > 0, "no health bars drawn")
	_check(shots_by.get(1, 0) == 0, "the enemy fired %d rounds while holding fire" % shots_by.get(1, 0))
	# Rounds that miss the target may hit the tanks behind it. The enemy
	# also has two rifle squads (prompt 022).
	var tanks_alive := 0
	for i in _indices(enemies):
		tanks_alive += alive[i]
	var enemies_alive := tanks_alive
	for i in _indices(_ids_of(false, true)):
		enemies_alive += alive[i]
	_check(sim_host.alive_unit_count(1) == enemies_alive and tanks_alive < enemies.size(),
		"alive_unit_count says %d enemy units, %d drawn alive" % [sim_host.alive_unit_count(1), enemies_alive])
	_check(not main.selection.selected.is_empty() and main.selection.selected.size() <= own.size(), "selection lost")

	# F: fire at will for the selection.
	_key(KEY_F)
	await _ticks(3)
	var hold := sim_host.unit_hold_fire()
	var holding := 0
	for i in _indices(main.selection.selected):
		holding += hold[i]
	_check(holding == 0, "F left %d selected units holding fire" % holding)
	view.combat_event.disconnect(on_event)
	# Effects (prompt 019) follow the events: flashes, impacts, the explosion
	# and the burning wreck.
	var counts: Dictionary = main.effects.counts
	_check(counts.get("flash", 0) > 0 and counts.get("powder", 0) > 0, "no muzzle effects: %s" % counts)
	_check(counts.get("sparks", 0) + counts.get("fountain", 0) > 0, "no impact effects: %s" % counts)
	_check(counts.get("fireball", 0) > 0 and counts.get("wreck", 0) >= 1, "no explosion or wreck fire: %s" % counts)
	_check(main.effects.burning_wrecks() >= 1, "no burning wreck")
	print("self-test: combat ok at tick %d: %d shots, %d hits, %d penetrations, %d destroyed, %d enemies left" % [
		sim_host.current_tick(), view.shots, view.hits, view.penetrations, view.destroyed, sim_host.alive_unit_count(1)
	])


# --- 6. Replay ------------------------------------------------------------


func _replay_test() -> void:
	var sim_host: SimHost = main.sim_host
	var path: String = main.save_replay()
	_check(not path.is_empty(), "replay was not written")
	if path.is_empty():
		return
	var bytes := FileAccess.get_file_as_bytes(path)
	var created: String = ReplayPlayer.read_replay_info(bytes).get("created", "")
	_check(bytes == sim_host.replay_file_bytes(created), "replay file differs from the recorded match")
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


## Ids of the own (or the other players') tanks, ascending; with `squads`
## their infantry squads instead (prompt 022).
func _ids_of(own: bool, squads := false) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var stances := sim_host.unit_stances()
	var result: Array[int] = []
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own and (stances[i] >= 0) == squads:
			result.append(ids[i])
	result.sort()
	return result


## The tanks among `ids`, ascending.
func _tanks(ids: Array[int]) -> Array[int]:
	var stances: PackedInt32Array = main.sim_host.unit_stances()
	var all: PackedInt64Array = main.sim_host.unit_ids()
	var result: Array[int] = []
	for i in all.size():
		if stances[i] < 0 and all[i] in ids:
			result.append(all[i])
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
		# Order keys are input actions bound to physical keys (prompt 031).
		event.physical_keycode = keycode
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
		# Like a real wheel notch: press and release. A press without
		# release keeps the GUI's mouse focus on the control under it
		# (the minimap, prompt 031), which would then take every click.
		var release := wheel.duplicate() as InputEventMouseButton
		release.pressed = false
		get_viewport().push_input(release)
