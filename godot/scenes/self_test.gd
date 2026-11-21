extends Node
## Headless self-test, started by `main.gd` with `-- --self-test`.
##
## Drives camera, selection and orders through real input events (pushed into
## the viewport), checks the results and quits with exit code 0 (pass) or 1
## (fail). Frame counts assume `--fixed-fps 60`.
##
## 1. Camera: zoom limits, pan to the map corner, 45° rotation.
## 2. Selection: box over the whole screen selects exactly the own tanks,
##    Ctrl+1 / 1 / 1-1 (center), click, Shift-click, enemy click, double click.
## 2b. Map controls: trackpad pan, Option drag, double click on the ground;
##    prompt 094: the active unit (click, box, deselecting, tile frame), the
##    panel's center button and Shift+C, the follow toggle and C (same
##    state, zoom and rotation keep it, a hand pan ends it, a new selection
##    is followed, the choice is remembered). The hill drive (4) checks that
##    a driving tank stays in the screen center while followed.
##    Prompt 096 (`_tilt_test`): the camera control (drag, clicks on the
##    directions, home, wheel), the tilt keys and the orbit drag on the map,
##    the tilt limits, ground picking and the minimap's view turned and
##    tilted, shadows and far plane, clearance flat and close.
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
##    will. Prompt 105 (`tools/kill_self_test.gd`): the destroyed tank plays
##    its kill sequence to a burning wreck, dead units are never selectable,
##    ricochets fly off; after the works test the state marks follow the
##    rules (track off, immobilized, nothing seen only from inside over
##    enemies).
## 9. Destruction (prompt 020, `tools/destruction_self_test.gd`): the own
##    tanks shell a building into a ruin and fell a tree; variants and
##    effects follow the simulation, the replay re-simulates it.
##    Prompt 106 (`tools/tree_crush_self_test.gd`, after the track test): a
##    tree pushed over by a vehicle falls with a bounce, throws leaves and
##    is heard in sight, and stays silent out of sight.
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
## 13. Fire (prompt 041, `tools/fire_self_test.gd`, after 9): the own tanks
##    shell the ground next to hay or a barn until it burns; the flames are
##    drawn, the replay re-simulates the fire.
## 14. Transport (prompt 034, `tools/transport_self_test.gd`): a squad
##    mounts an own tank by right click and dismounts with J.
## 15. Supply (prompt 036, `tools/supply_self_test.gd`): the own tanks used
##    fuel, the unit panel shows their stock bars, tanks draw no supply ring.
## 16. Works (prompt 035, `tools/works_self_test.gd`, after the replay
##    test): a hidden debug minefield, an own tank throws its track in it.
## 17. Positions (prompt 052, `tools/positions_self_test.gd`): a debug
##    strongpoint is drawn as trenches, a squad walks in, a line shows its price.
## 18. Experience (prompt 050, `tools/experience_self_test.gd`, after the
##    replay test): kill rings, rank in the unit panel, an elite squad's kit.
## 19. Crossings (prompt 055, `tools/crossing_self_test.gd`, before the
##    replay test): camouflage from the command bar, paratroopers called by
##    a drag, a collapsing deck and a laid bridge drawn.
## 20. Wounds (prompt 049, `tools/care_self_test.gd`, after the replay
##    test): debug-wounded men of an own squad in the feed, the unit panel and
##    their clips; a comrade carries the heavily wounded man along.

## Panzer IV H: 16 km/h cross-country (catalog) × the vehicle speed factor
## 1/1 (`data/scale.toml [speed]`, prompt 097; the time factor 2 before) =
## 4.444 m/s, i.e. 0.2222 m per tick at 20 Hz. Terrain scales it
## (data/terrain.toml).
const EXPECTED_STEP := 16.0 * 1.0 / 3.6 / 20.0
## Panzer IV H on roads: 38 km/h × 1 = 0.5278 m per tick.
const ROAD_STEP := 38.0 * 1.0 / 3.6 / 20.0
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
const FireSelfTest := preload("res://scenes/tools/fire_self_test.gd")
const SoundSelfTest := preload("res://scenes/tools/sound_self_test.gd")
const MusicSelfTest := preload("res://scenes/tools/music_self_test.gd")
const HudSelfTest := preload("res://scenes/tools/hud_self_test.gd")
const AirSelfTest := preload("res://scenes/tools/air_self_test.gd")
const TransportSelfTest := preload("res://scenes/tools/transport_self_test.gd")
const SupplySelfTest := preload("res://scenes/tools/supply_self_test.gd")
const WorksSelfTest := preload("res://scenes/tools/works_self_test.gd")
const PositionsSelfTest := preload("res://scenes/tools/positions_self_test.gd")
const FormationSelfTest := preload("res://scenes/tools/formation_self_test.gd")
const ExperienceSelfTest := preload("res://scenes/tools/experience_self_test.gd")
const CrossingSelfTest := preload("res://scenes/tools/crossing_self_test.gd")
const CareSelfTest := preload("res://scenes/tools/care_self_test.gd")
const TrackSelfTest := preload("res://scenes/tools/track_self_test.gd")
const KillSelfTest := preload("res://scenes/tools/kill_self_test.gd")
const TreeCrushSelfTest := preload("res://scenes/tools/tree_crush_self_test.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")

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
	var kill_test := KillSelfTest.new()
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
	kill_test.watch(self)
	await _combat_test()
	await kill_test.run_kills(self)
	await TrackSelfTest.new().run(self)
	await TreeCrushSelfTest.new().run(self)
	await _marker_test()
	await _cloud_test()
	await InfantrySelfTest.new().run(self)
	await DestructionSelfTest.new().run(self)
	await FireSelfTest.new().run(self)
	await CraterSelfTest.new().run(self)
	await SoundSelfTest.new().run(self)
	await MusicSelfTest.new().run(self)
	await hud_test.run(self)
	await AirSelfTest.new().run(self)
	await TransportSelfTest.new().run(self)
	await SupplySelfTest.new().run(self)
	await FormationSelfTest.new().run(self)
	await CrossingSelfTest.new().run(self)
	_replay_test()
	# After the replay test: its debug minefield is no command (prompt 035).
	await WorksSelfTest.new().run(self)
	await kill_test.run_marks(self)
	await PositionsSelfTest.new().run(self)
	await ExperienceSelfTest.new().run(self)
	await CareSelfTest.new().run(self)
	# Last: the extra frames shift the match, and the positions test above
	# depends on where the fight stands at its start (prompt 164).
	await _highlight_test()
	await _pause_test()
	await _view_test()
	var ok := _errors.is_empty()
	var message := "camera, selection, controls, orders, hill, objects, terrain, biomes, fog, combat, kill sequences, tracks, pushed trees, impact markers, cloud shadows, infantry, destruction, fire, craters, sound, music, water, hud, air, transport, supply, formations, crossings, replay, works, state marks, positions, experience, care, unit highlight, tactical pause, overview, view switches and photo mode ok" if ok else " ".join(_errors)
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
	# Tanks per faction in battle mode (prompt 045, `[faction_count_pct]` of
	# data/scenarios/demo.toml): 8 Panzer IV, 10 T-34, 9 M4A3.
	var own_tanks := _expected_tanks(own)
	var enemy_tanks := _expected_tanks(enemy)
	_check(own_tanks > 0 and enemy_tanks > 0 and own.size() == own_tanks and enemy.size() == enemy_tanks,
		"expected %d own and %d enemy tanks, got %d/%d" % [own_tanks, enemy_tanks, own.size(), enemy.size()])

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

	await _active_unit_test(own)
	await _center_test(own)

	# Follow mode (C): the camera moves to the active unit (prompt 094;
	# before: the selection's center) and stops following as soon as the
	# map is moved by hand.
	var panel: Control = main.hud.unit_panel
	_check(not selection.follow and not Settings.get_value("camera_follow_active"), "follow mode is on without the player's choice")
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	_left_click(_screen_of_unit(own[0]))
	_left_click(_screen_of_unit(own[1]), true)
	await _frames(2)
	rig.snap_to(Vector3(200.0, 0.0, 200.0), OVERVIEW_DISTANCE, 0.0)
	_key(KEY_C)
	await _frames(90)
	_check(selection.follow, "follow mode did not turn on")
	_check(selection.active_unit == own[1], "follow test: active unit %d, expected %d" % [selection.active_unit, own[1]])
	var follow_off := _off_center(own[1])
	_check(follow_off < 1.0, "follow mode did not center the selection (%.2f m off)" % follow_off)
	_check(panel.follow_button.button_pressed, "C turned follow mode on, the button does not show it")
	_check(Settings.get_value("camera_follow_active"), "follow mode chosen with C is not remembered")
	get_viewport().push_input(pan)
	await _frames(2)
	_check(not selection.follow, "a hand pan did not end follow mode")
	_check(not panel.follow_button.button_pressed, "a hand pan ended follow mode, the button still shows it")
	# The hand pan is no choice: the next match follows again.
	_check(Settings.get_value("camera_follow_active"), "a hand pan changed the remembered follow mode")
	selection.load_follow()
	await _frames(2)
	_check(selection.follow and panel.follow_button.button_pressed, "the remembered follow mode is not taken over")
	_key(KEY_C)
	await _frames(2)
	_check(not selection.follow and not panel.follow_button.button_pressed and not Settings.get_value("camera_follow_active"),
		"C did not turn follow mode off (follow %s, button %s)" % [selection.follow, panel.follow_button.button_pressed])

	# The toggle in the unit panel is the same mode. Zoom (with the cursor
	# off the center) and rotation keep it and keep the unit centered.
	rig.snap_to(Vector3(200.0, 0.0, 200.0), OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	_left_click(panel.follow_button.get_global_rect().get_center())
	await _frames(90)
	_check(selection.follow and panel.follow_button.button_pressed and Settings.get_value("camera_follow_active"),
		"the follow button did not turn follow mode on")
	_check(selection.selected == [own[0], own[1]] and selection.command_mode.is_empty(),
		"the follow button changed the selection to %s" % [selection.selected])
	_check(_off_center(own[1]) < 1.0, "follow button: active unit %.2f m off the center" % _off_center(own[1]))
	var distance_before: float = rig.distance
	_push_wheel(MOUSE_BUTTON_WHEEL_UP, 3, Vector2(0.25, 0.3))
	await _frames(60)
	_check(rig.distance < distance_before - 1.0, "the wheel did not zoom while following (%.1f m)" % rig.distance)
	_check(selection.follow, "zooming ended follow mode")
	_check(_off_center(own[1]) < 0.05, "zooming while following pushed the unit %.2f m off the center" % _off_center(own[1]))
	var rotate := InputEventAction.new()
	rotate.action = "camera_rotate_left"
	rotate.pressed = true
	get_viewport().push_input(rotate)
	await _frames(60)
	_check(absf(rig.yaw - PI / 4.0) < 0.001, "rotation while following reached %.4f" % rig.yaw)
	_check(selection.follow and _off_center(own[1]) < 0.05, "rotating ended follow mode or lost the unit (%.2f m off)" % _off_center(own[1]))
	# Tilting and a drag on the camera control keep it too (prompt 096).
	var gizmo: Control = main.hud.gizmo
	_key(KEY_PAGEDOWN)
	_gizmo_drag(gizmo.global_position + gizmo.dial_center(), Vector2(25.0, 10.0))
	await _frames(60)
	_check(rig.tilt < -0.1 and absf(rig.yaw - PI / 4.0) > 0.1, "tilt key and dial drag did nothing while following (tilt %.2f, yaw %.2f)" % [rig.tilt, rig.yaw])
	_check(selection.follow and _off_center(own[1]) < 0.05, "tilting or turning ended follow mode or lost the unit (%.2f m off)" % _off_center(own[1]))
	rig.snap_tilt(0.0)
	rig.snap_to(rig.focus, OVERVIEW_DISTANCE, 0.0)

	# Another active unit: the camera wanders there without a jump.
	var apart := _positions_of([own[0]])[0].distance_to(_positions_of([own[1]])[0])
	var tiles: Array = panel._grid.get_children()
	_check(tiles.size() == 2, "unit panel has %d tiles for two units" % tiles.size())
	if tiles.size() == 2:
		_left_click((tiles[0] as Control).get_global_rect().get_center())
		await _frames(3)
		_check(selection.selected == [own[0]] and selection.active_unit == own[0], "tile click: selected %s, active %d" % [selection.selected, selection.active_unit])
		_check(selection.follow, "a new selection ended follow mode")
		_check(_off_center(own[0]) > apart * 0.3, "the camera jumped to the new active unit (%.2f of %.2f m left)" % [_off_center(own[0]), apart])
		await _frames(90)
		_check(_off_center(own[0]) < 1.0, "the camera did not wander to the new active unit (%.2f m off)" % _off_center(own[0]))
	# Without a selection the mode waits where the camera is ...
	_left_click(_free_ground())
	await _frames(10)
	var waiting: Vector3 = rig.focus
	_check(selection.selected.is_empty() and selection.follow and panel.follow_button.button_pressed,
		"without a selection: selected %s, follow %s" % [selection.selected, selection.follow])
	_check(panel.center_button.disabled, "the center button is not disabled without an active unit")
	await _frames(10)
	_check(rig.focus.distance_to(waiting) < 0.01, "the waiting follow mode moved the camera")
	# ... and takes the next selection.
	_left_click(_screen_of_unit(own[1]))
	await _frames(90)
	_check(selection.active_unit == own[1] and not panel.center_button.disabled and _off_center(own[1]) < 1.0,
		"follow mode did not take the next selection (active %d, %.2f m off)" % [selection.active_unit, _off_center(own[1])])
	# A double click on the ground looks there and ends the mode.
	var away := _free_ground()
	_left_click(away)
	_left_click(away, false, true)
	await _frames(2)
	_check(not selection.follow and not panel.follow_button.button_pressed, "a double click on the ground did not end follow mode")
	_check(selection.selected == [own[1]] and selection.active_unit == own[1], "the double click on the ground lost the active unit")
	# Back to the state the next parts expect.
	Settings.set_value("camera_follow_active", false)
	Settings.save_file()
	selection.set_selection(own)
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	await _tilt_test(own)
	await _dof_tilt_test()
	_check(selection.selected == own, "map controls left the selection %s" % [selection.selected])
	print("self-test: map controls ok (trackpad, Option drag, double click, active unit, center, follow, turn and tilt)")


## Prompt 183: the depth of field follows the view angle. At the zoom's own
## angle the tilt-shift band is as before (near and far blur), tilted flat
## there is no near blur and the far blur starts up the screen; steep, the
## miniature look keeps its near blur. Checks the values of the camera's
## CameraAttributesPractical, no pictures. Leaves tilt 0 at the overview.
func _dof_tilt_test() -> void:
	var rig: Node3D = main.camera_rig
	var camera: Camera3D = main.camera
	var atmosphere: Node = main.atmosphere
	var saved_mode: Variant = Settings.get_value("depth_of_field")
	var saved_quality: String = atmosphere.quality
	if saved_quality == "low":
		atmosphere.quality = "medium"
	Settings.set_value("depth_of_field", "subtle")
	rig.snap_to(Vector3.ZERO, 40.0, 0.0)
	rig.snap_tilt(0.0)
	await _frames(2)
	var dof := camera.attributes as CameraAttributesPractical
	var d: float = rig.distance
	_check(dof != null and dof.dof_blur_amount > 0.0, "no depth of field at 40 m with the setting subtle")
	if dof:
		var mode: Dictionary = atmosphere.DOF_MODES.subtle
		_check(dof.dof_blur_near_enabled and absf(dof.dof_blur_near_distance - d * float(mode.near)) < 0.01 * d
			and absf(dof.dof_blur_far_distance - d * float(mode.far)) < 0.01 * d,
			"the tilt-shift band changed at the zoom's angle (near %s %.1f, far %.1f, distance %.1f)"
			% [dof.dof_blur_near_enabled, dof.dof_blur_near_distance, dof.dof_blur_far_distance, d])
	rig.snap_tilt(-PI)
	await _frames(2)
	dof = camera.attributes as CameraAttributesPractical
	d = rig.distance
	if dof:
		var row: float = atmosphere.row_depth(rig.pitch(), deg_to_rad(camera.fov), atmosphere.DOF_FLAT_ROWS.x)
		_check(not dof.dof_blur_near_enabled and is_zero_approx(dof.dof_blur_near_distance),
			"near blur at the flattest tilt (%.1f degrees): on %s, %.1f m"
			% [rad_to_deg(rig.pitch()), dof.dof_blur_near_enabled, dof.dof_blur_near_distance])
		_check(dof.dof_blur_far_distance >= d * row - 0.01 and dof.dof_blur_far_distance > d * 1.2,
			"the far blur starts at %.1f m at the flattest tilt, below the screen row at %.1f m" % [dof.dof_blur_far_distance, d * row])
	else:
		_check(false, "no depth of field at the flattest tilt")
	Settings.set_value("depth_of_field", "miniature")
	rig.snap_to(Vector3.ZERO, 100.0, 0.0)
	rig.snap_tilt(PI)
	await _frames(2)
	dof = camera.attributes as CameraAttributesPractical
	_check(dof != null and dof.dof_blur_near_enabled and dof.dof_blur_near_distance > 0.5 * rig.distance,
		"the miniature look lost its near blur looking steeply down")
	Settings.set_value("depth_of_field", saved_mode)
	atmosphere.quality = saved_quality
	rig.snap_tilt(0.0)
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	print("self-test: depth of field by view angle ok")


## Prompt 096: the camera control (drag, clicks on the directions, home,
## double click, wheel), the tilt keys, the orbit drag on the map, the
## limits, ground picking and the minimap's view under a turned and tilted
## camera, the camera's clearance at the flattest tilt. Expects all own
## units selected and leaves it so; issues no orders.
func _tilt_test(own: Array[int]) -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	var sim_host: SimHost = main.sim_host
	var hud: Control = main.hud
	var gizmo: Control = hud.gizmo
	var camera: Camera3D = main.camera
	var low := deg_to_rad(rig.tilt_min_deg)
	var high := deg_to_rad(rig.tilt_max_deg)
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(2)
	var dial: Vector2 = gizmo.global_position + gizmo.dial_center()
	_check(gizmo.is_visible_in_tree() and hud.panels().has("camera control") and hud.covers(dial)
		and hud.covers(gizmo.home_button.get_global_rect().get_center()), "the camera control does not show or is no HUD panel")
	_check(not hud.minimap.north_button.is_visible_in_tree(), "the north button shows beside the camera control")
	# Tilt 0 is the view of the zoom, as before the prompt.
	var base: float = rig.base_pitch()
	_check(is_zero_approx(rig.tilt) and is_zero_approx(rig.target_tilt) and is_equal_approx(rig.pitch(), base),
		"the match does not start with the tilt of the zoom (tilt %.3f)" % rig.tilt)
	_check(absf(gizmo.elevation - base) < 0.001 and absf(gizmo.turn) < 0.001, "the dial shows turn %.2f, elevation %.2f" % [gizmo.turn, gizmo.elevation])
	var north: Vector2 = gizmo.mark_position(0) - gizmo.dial_center()
	_check(absf(north.x) < 0.5 and north.y < 0.0, "facing north, the dial's N is at %s" % north)

	# A sideways drag turns by any angle, at once, and nothing moves on.
	var rate: float = deg_to_rad(rig.orbit_deg_per_pixel) * gizmo.DRAG_GAIN / gizmo.HudTheme.scale
	_gizmo_drag(dial, Vector2(30.0, 0.0))
	await _frames(1)
	var turned: float = rig.yaw
	_check(absf(turned + 30.0 * rate) < 0.005, "a 30 px drag on the dial turned to %.4f, expected %.4f" % [turned, -30.0 * rate])
	_check(absf(fposmod(turned, PI / 4.0)) > 0.01 and is_equal_approx(rig.target_yaw, turned), "the dial drag turns in steps or glides on")
	_check(is_equal_approx(rig.pitch(), base), "a sideways drag tilted the camera")
	await _frames(30)
	_check(is_equal_approx(rig.yaw, turned), "the camera turned on after the drag (%.4f, was %.4f)" % [rig.yaw, turned])
	north = gizmo.mark_position(0) - gizmo.dial_center()
	_check(absf(north.angle_to(Vector2.UP.rotated(turned))) < 0.35 and absf(gizmo.turn - turned) < 0.001,
		"the dial's N at %s does not follow the camera turned %.2f" % [north, turned])

	# Up is steeper, down flatter; the limits hold.
	_gizmo_drag(dial, Vector2(0.0, -15.0))
	await _frames(1)
	_check(absf(rig.pitch() - (base + 15.0 * rate)) < 0.005 and is_equal_approx(rig.yaw, turned),
		"a 15 px drag up tilted to %.3f, expected %.3f" % [rig.pitch(), base + 15.0 * rate])
	var squash_default := absf(gizmo.mark_position(0).y - gizmo.mark_position(2).y)
	_gizmo_drag(dial, Vector2(0.0, -300.0))
	await _frames(1)
	_check(absf(rig.pitch() - high) < 0.001, "dragging far up tilted to %.1f degrees, the limit is %.1f" % [rad_to_deg(rig.pitch()), rig.tilt_max_deg])
	_gizmo_drag(dial, Vector2(0.0, 600.0))
	await _frames(2)
	_check(absf(rig.pitch() - low) < 0.001, "dragging far down tilted to %.1f degrees, the limit is %.1f" % [rad_to_deg(rig.pitch()), rig.tilt_min_deg])
	_check(camera.global_position.y > rig.focus.y + 1.0, "the camera is not above its focus at the flattest tilt")
	# No dead way at the limit: a small drag back acts at once.
	_gizmo_drag(dial, Vector2(0.0, -10.0))
	await _frames(1)
	_check(absf(rig.pitch() - (low + 10.0 * rate)) < 0.005, "after the limit a 10 px drag up tilted to %.3f, expected %.3f" % [rig.pitch(), low + 10.0 * rate])
	# The dial is squashed by the tilt.
	gizmo.queue_redraw()
	await _frames(2)
	var squash_flat := absf(gizmo.mark_position(0).y - gizmo.mark_position(2).y)
	_check(absf(gizmo.elevation - rig.pitch()) < 0.001, "the dial shows elevation %.2f, the camera has %.2f" % [gizmo.elevation, rig.pitch()])

	# Home button: north and the tilt of the zoom, smoothly.
	_left_click(gizmo.home_button.get_global_rect().get_center())
	await _frames(2)
	_check(not is_zero_approx(rig.tilt) and not is_zero_approx(rig.yaw), "the home button jumps instead of gliding")
	await _frames(120)
	_check(absf(angle_difference(rig.yaw, 0.0)) < 0.001 and is_zero_approx(rig.tilt) and absf(rig.pitch() - base) < 0.001,
		"the home button left yaw %.3f, tilt %.3f" % [rig.yaw, rig.tilt])
	rig.snap_to(rig.focus, rig.distance, 0.0)

	# A click on E turns the camera to look east (+x).
	await _frames(2)
	_left_click(gizmo.global_position + gizmo.mark_position(1))
	await _frames(120)
	var forward := -camera.global_transform.basis.z
	forward.y = 0.0
	_check(forward.normalized().dot(Vector3.RIGHT) > 0.9999, "a click on E left the camera looking along %s" % forward.normalized())
	_check(is_equal_approx(rig.pitch(), base), "a click on E tilted the camera")
	# ... on S south, on N north again (each time the mark is where it is now).
	_left_click(gizmo.global_position + gizmo.mark_position(2))
	await _frames(120)
	forward = -camera.global_transform.basis.z
	forward.y = 0.0
	_check(forward.normalized().dot(Vector3.BACK) > 0.9999, "a click on S left the camera looking along %s" % forward.normalized())
	_left_click(gizmo.global_position + gizmo.mark_position(0))
	await _frames(120)
	_check(absf(angle_difference(rig.yaw, 0.0)) < 0.001, "a click on N left yaw %.3f" % rig.yaw)
	rig.snap_to(rig.focus, rig.distance, 0.0)
	await _frames(1)

	# Wheel over the dial: it tilts in steps, the zoom stays.
	var step: float = deg_to_rad(rig.tilt_step_deg)
	_push_wheel(MOUSE_BUTTON_WHEEL_UP, 1, dial / get_viewport().get_visible_rect().size)
	await _frames(60)
	_check(absf(rig.tilt - step) < 0.001 and is_equal_approx(rig.target_distance, OVERVIEW_DISTANCE),
		"the wheel over the dial: tilt %.3f (expected %.3f), distance %.1f" % [rig.tilt, step, rig.target_distance])
	_push_wheel(MOUSE_BUTTON_WHEEL_DOWN, 1, dial / get_viewport().get_visible_rect().size)
	await _frames(60)
	_check(is_zero_approx(rig.tilt), "the wheel down over the dial left tilt %.3f" % rig.tilt)

	# Keys: steeper / flatter in steps, within the limits.
	_key(KEY_PAGEDOWN)
	await _frames(1)
	_check(absf(rig.target_tilt + step) < 0.0001 and rig.tilt > rig.target_tilt, "Page down: target tilt %.3f, expected %.3f, gliding" % [rig.target_tilt, -step])
	await _frames(60)
	_check(absf(rig.pitch() - (base - step)) < 0.001, "Page down tilted to %.3f, expected %.3f" % [rig.pitch(), base - step])
	_key(KEY_PAGEUP)
	_key(KEY_PAGEUP)
	await _frames(60)
	_check(absf(rig.pitch() - (base + step)) < 0.001, "Page up twice tilted to %.3f, expected %.3f" % [rig.pitch(), base + step])
	for i in 12:
		_key(KEY_PAGEUP)
	await _frames(90)
	_check(absf(rig.pitch() - high) < 0.001, "Page up at the limit: %.1f degrees" % rad_to_deg(rig.pitch()))
	for i in 12:
		_key(KEY_PAGEDOWN)
	await _frames(90)
	_check(absf(rig.pitch() - low) < 0.001, "Page down at the limit: %.1f degrees" % rad_to_deg(rig.pitch()))
	# Zooming keeps the limits (the zoom's own angle changes).
	for distance: float in [rig.min_distance, 60.0, rig.max_distance]:
		for tilt: float in [-PI, PI]:
			rig.snap_to(rig.focus, distance, 0.0)
			rig.snap_tilt(tilt)
			_check(rig.pitch() >= low - 0.0001 and rig.pitch() <= high + 0.0001, "tilt %.1f at %.0f m: %.1f degrees" % [tilt, distance, rad_to_deg(rig.pitch())])
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	rig.snap_tilt(0.0)
	await _frames(1)

	# Orbit drag on the map: Shift + middle button (or Shift + Alt + move)
	# turns and tilts around the focus, it does not pan.
	var center := get_viewport().get_visible_rect().size * 0.5
	var spot := _free_ground()
	var map_rate: float = deg_to_rad(rig.orbit_deg_per_pixel)
	_mouse(MOUSE_BUTTON_MIDDLE, spot, true, true)
	for i in 4:
		var motion := InputEventMouseMotion.new()
		motion.position = spot + Vector2(20.0, -10.0) * (i + 1)
		motion.global_position = motion.position
		motion.relative = Vector2(20.0, -10.0)
		motion.shift_pressed = true
		motion.button_mask = MOUSE_BUTTON_MASK_MIDDLE
		get_viewport().push_input(motion)
	_mouse(MOUSE_BUTTON_MIDDLE, spot + Vector2(80.0, -40.0), false, true)
	await _frames(2)
	_check(absf(rig.yaw + 80.0 * map_rate) < 0.001 and absf(rig.tilt - 40.0 * map_rate) < 0.001,
		"Shift + middle drag: yaw %.4f (expected %.4f), tilt %.4f (expected %.4f)" % [rig.yaw, -80.0 * map_rate, rig.tilt, 40.0 * map_rate])
	_check(Vector2(rig.focus.x, rig.focus.z).length() < 0.01, "the orbit drag moved the map (focus %s)" % rig.focus)
	var alt_motion := InputEventMouseMotion.new()
	alt_motion.position = center
	alt_motion.global_position = center
	alt_motion.relative = Vector2(-80.0, 40.0)
	alt_motion.alt_pressed = true
	alt_motion.shift_pressed = true
	get_viewport().push_input(alt_motion)
	await _frames(2)
	_check(absf(rig.yaw) < 0.001 and absf(rig.tilt) < 0.001 and Vector2(rig.focus.x, rig.focus.z).length() < 0.01,
		"Shift + Alt + move: yaw %.4f, tilt %.4f, focus %s" % [rig.yaw, rig.tilt, rig.focus])
	# The setting turns the drag direction of the tilt around.
	rig.tilt_inverted = true
	_gizmo_drag(dial, Vector2(0.0, -15.0))
	await _frames(1)
	_check(absf(rig.pitch() - (base - 15.0 * rate)) < 0.005, "inverted, a drag up tilted to %.3f, expected %.3f" % [rig.pitch(), base - 15.0 * rate])
	Settings.apply_camera(rig)
	_check(not rig.tilt_inverted, "the tilt is inverted without the setting")

	# Ground picking and the minimap's view, turned and tilted to both limits.
	var top_edges: Array[float] = []
	for tilt: float in [0.0, -PI, PI]:
		rig.snap_to(Vector3.ZERO, 150.0, 1.1)
		rig.snap_tilt(tilt)
		await _frames(3)
		var worst := 0.0
		for offset: Vector2 in [Vector2.ZERO, Vector2(18, 7), Vector2(-25, 12), Vector2(6, -30), Vector2(-14, -22), Vector2(35, 30)]:
			var point := Vector3(offset.x, sim_host.height_at(offset.x, offset.y), offset.y)
			var hit: Variant = GroundPick.from_screen(camera, camera.unproject_position(point))
			_check(hit != null, "no ground under the pointer at %s (tilt %.1f)" % [offset, tilt])
			if hit != null:
				worst = maxf(worst, (hit as Vector3).distance_to(point))
		_check(worst < 1.0, "turned and tilted (%.0f degrees), the ground pick is %.2f m off" % [rad_to_deg(rig.pitch()), worst])
		var polygon: PackedVector2Array = hud.minimap._view_polygon
		_check(polygon.size() == 5, "the minimap's view has %d points" % polygon.size())
		var corner: Variant = GroundPick.from_screen(camera, Vector2.ZERO)
		if corner != null and polygon.size() == 5:
			_check(polygon[0].distance_to(hud.minimap.to_minimap(corner)) < 0.5, "the minimap's view corner is not under the screen corner (tilt %.1f)" % tilt)
		if polygon.size() == 5:
			top_edges.append(polygon[0].distance_to(polygon[1]))
	_check(top_edges.size() == 3 and top_edges[1] > top_edges[0] * 1.3 and top_edges[2] < top_edges[0],
		"the minimap's view does not widen with a flat tilt: far edges %s (default, flat, steep)" % [top_edges])
	# Shadows and far plane reach the ground at the upper screen edge.
	rig.snap_to(Vector3.ZERO, 40.0, 0.0)
	rig.snap_tilt(-PI)
	var far_ground: Variant = GroundPick.from_screen(camera, Vector2(center.x, 0.0))
	_check(far_ground != null, "the flattest view shows no ground at the upper screen edge")
	if far_ground != null:
		var depth := (far_ground as Vector3 - camera.global_position).dot(-camera.global_transform.basis.z)
		_check(main.sun.directional_shadow_max_distance > depth and camera.far > depth * 1.2,
			"flattest view: ground %.0f m deep, shadows to %.0f m, far plane %.0f m" % [depth, main.sun.directional_shadow_max_distance, camera.far])
	_check(squash_flat < squash_default * 0.75, "the dial is not squashed by a flat tilt (%.1f px, steeper %.1f px)" % [squash_flat, squash_default])

	# Flattest tilt at the closest zoom: above ground and roofs, all around,
	# on the hill and with the camera right over a building.
	var spots: Array[Vector3] = [Vector3.ZERO]
	var hill: Variant = sim_host.map_marker("hill_top") if sim_host.has_method(&"map_marker") else null
	if hill is Vector3:
		spots.append(hill)
	var building: Variant = null
	var half: Vector2 = sim_host.map_size() * 0.5
	var x := -half.x + 4.0
	while x < half.x and building == null:
		var z := -half.y + 4.0
		while z < half.y:
			if GroundPick.object_top_at(x, z, 0.0) > -1e8:
				building = Vector3(x, 0.0, z)
				break
			z += 6.0
		x += 6.0
	_check(building != null, "no building on the map for the clearance test")
	var checked := 0
	for k in 8:
		var yaw := TAU * k / 8.0
		var all := spots.duplicate()
		if building != null:
			# The focus from which the camera stands over the building.
			var reach: float = rig.min_distance * cos(low)
			all.append((building as Vector3) - Vector3(sin(yaw), 0.0, cos(yaw)) * reach)
		for at: Vector3 in all:
			rig.snap_to(at, rig.min_distance, yaw)
			rig.snap_tilt(-PI)
			var eye := camera.global_position
			var ground: float = sim_host.height_at(eye.x, eye.z)
			_check(eye.y >= ground + rig.min_clearance - 0.01, "flat and close at %s, yaw %.1f: camera %.2f m over the ground" % [at, yaw, eye.y - ground])
			var roof := GroundPick.object_top_at(eye.x, eye.z, 2.0)
			_check(eye.y >= roof + rig.building_clearance - 0.01, "flat and close at %s, yaw %.1f: camera %.2f m over the roof" % [at, yaw, eye.y - roof])
			var middle := eye.lerp(rig.focus, 0.5)
			_check(middle.y > sim_host.height_at(middle.x, middle.z), "flat and close at %s, yaw %.1f: ground between camera and focus" % [at, yaw])
			checked += 1
	if building != null:
		var over := camera.global_position
		_check(GroundPick.object_top_at(over.x, over.z, 2.0) > -1e8, "the clearance test never put the camera over a building")

	# None of this was an order or a selection.
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	rig.snap_tilt(0.0)
	await _frames(2)
	dial = gizmo.global_position + gizmo.dial_center()
	_right_click(dial)
	_left_click(dial + Vector2(0.0, gizmo.dial_radius() * 0.9))
	await _ticks(3)
	_check(selection.selected == own and selection.command_mode.is_empty() and not _moving(own) and is_zero_approx(rig.yaw),
		"clicks on the camera control gave an order or changed the selection (%s, moving %s)" % [selection.selected, _moving(own)])
	for id in own:
		_check(sim_host.unit_queue_length(id) == 0, "unit %d has orders after the camera control test" % id)
	print("self-test: turn and tilt ok (limits %.0f to %.0f degrees, %d clearance views)" % [rig.tilt_min_deg, rig.tilt_max_deg, checked])


## A left drag that starts on the camera control at `from` (screen pixels).
func _gizmo_drag(from: Vector2, by: Vector2, steps := 6) -> void:
	_mouse(MOUSE_BUTTON_LEFT, from, true)
	for step in steps:
		var motion := InputEventMouseMotion.new()
		motion.position = from + by * (step + 1.0) / steps
		motion.global_position = motion.position
		motion.relative = by / steps
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		get_viewport().push_input(motion)
	_mouse(MOUSE_BUTTON_LEFT, from + by, false)


## Active unit (prompt 094): the unit clicked last; a box keeps it, taking
## it out of the selection hands it to the nearest selected unit.
func _active_unit_test(own: Array[int]) -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	var panel: Control = main.hud.unit_panel
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	_left_click(_screen_of_unit(own[0]))
	await _frames(2)
	_check(selection.selected == [own[0]] and selection.active_unit == own[0],
		"click: selected %s, active %d" % [selection.selected, selection.active_unit])
	# Shift + click on a second unit: both selected, the clicked one active.
	_left_click(_screen_of_unit(own[1]), true)
	await _frames(2)
	_check(selection.selected == [own[0], own[1]] and selection.active_unit == own[1],
		"shift click: selected %s, active %d" % [selection.selected, selection.active_unit])
	_check(panel.active_tile() == own[1], "unit panel frames the tile of %d, active is %d" % [panel.active_tile(), own[1]])
	# A box over everything keeps the active unit.
	var size := get_viewport().get_visible_rect().size
	_drag(Vector2(4, 4), size - Vector2(4, 4))
	await _frames(2)
	_check(_tanks(selection.selected) == own, "box selected %s" % [selection.selected])
	_check(selection.active_unit == own[1], "the box made %d active, it was %d" % [selection.active_unit, own[1]])
	_check(panel.active_tile() == own[1], "after the box the panel frames %d" % panel.active_tile())
	# Taking the active unit out: the nearest selected unit takes over.
	var from := _positions_of([own[1]])[0]
	var nearest := -1
	var nearest_distance := INF
	for id: int in selection.selected:
		var d := _positions_of([id])[0].distance_to(from)
		if id != own[1] and d < nearest_distance:
			nearest = id
			nearest_distance = d
	_left_click(_screen_of_unit(own[1]), true)
	await _frames(2)
	_check(not selection.selected.has(own[1]) and selection.active_unit == nearest,
		"deselecting the active unit made %d active, the nearest is %d" % [selection.active_unit, nearest])
	_check(panel.active_tile() == nearest, "after deselecting the panel frames %d" % panel.active_tile())
	# A group recalled without the active unit in it: its first unit.
	selection.groups[9] = [own[2], own[3]]
	_left_click(_screen_of_unit(own[0]))
	await _frames(1)
	selection._last_group_key = -1
	selection.recall_group(9)
	_check(selection.selected == [own[2], own[3]] and selection.active_unit == own[2],
		"group: selected %s, active %d" % [selection.selected, selection.active_unit])
	selection.groups.erase(9)
	selection._last_group_key = -1


## The center button of the unit panel and its key (Shift+C, prompt 094):
## the camera glides to the active unit; zoom and rotation stay.
func _center_test(own: Array[int]) -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	var panel: Control = main.hud.unit_panel
	rig.snap_to(Vector3.ZERO, OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	_left_click(_screen_of_unit(own[0]))
	_left_click(_screen_of_unit(own[1]), true)
	await _frames(2)
	var two: Array[int] = [own[0], own[1]]
	_check(selection.selected == two and selection.active_unit == own[1], "center test: selected %s" % [selection.selected])
	_check(Settings.action_key_name("camera_center") != "" and Settings.bindings["camera_center"] != Settings.bindings["camera_follow"],
		"no own key for centering the active unit")
	_check(panel.center_button.is_visible_in_tree() and not panel.center_button.disabled
		and panel.follow_button.is_visible_in_tree(), "the camera buttons of the unit panel are not shown")
	rig.snap_to(Vector3(200.0, 0.0, 200.0), 200.0, PI / 4.0)
	await _frames(1)
	_left_click(panel.center_button.get_global_rect().get_center())
	await _frames(2)
	_check(rig.is_gliding(), "the center button did not start a glide")
	await _frames(90)
	_check(_off_center(own[1]) < 1.0, "center button: active unit %.2f m off the center" % _off_center(own[1]))
	_check(selection.selected == two and selection.active_unit == own[1] and selection.command_mode.is_empty(),
		"the center button changed the selection to %s" % [selection.selected])
	_check(is_equal_approx(rig.distance, 200.0) and is_equal_approx(rig.yaw, PI / 4.0),
		"centering changed zoom or rotation (%.1f m, %.3f)" % [rig.distance, rig.yaw])
	_check(not selection.follow, "centering turned follow mode on")
	rig.snap_to(Vector3(-200.0, 0.0, 150.0), OVERVIEW_DISTANCE, 0.0)
	await _frames(1)
	_key(KEY_C, false, true)
	await _frames(90)
	_check(_off_center(own[1]) < 1.0, "Shift+C: active unit %.2f m off the center" % _off_center(own[1]))
	_check(not selection.follow, "Shift+C turned follow mode on")


## A screen point on open ground: no unit under it, no HUD panel over it.
func _free_ground() -> Vector2:
	var selection: Control = main.selection
	var center := get_viewport().get_visible_rect().size * 0.5
	for offset: Vector2 in [Vector2(0, -200), Vector2(-300, -150), Vector2(300, -150), Vector2(-450, 0), Vector2(450, 0), Vector2(0, -330)]:
		var at := center + offset
		if selection._unit_at(at) < 0 and selection._unit_at(at, false) < 0 and not main.hud.covers(at) \
				and GroundPick.from_screen(main.camera, at) != null:
			return at
	_check(false, "no open ground on the screen")
	return center


## Meters (x/z) between the screen center and where unit `id` is drawn.
func _off_center(id: int) -> float:
	var sim_host: SimHost = main.sim_host
	var index := sim_host.unit_ids().find(id)
	var at := sim_host.previous_positions()[index].lerp(sim_host.current_positions()[index], sim_host.interpolation_alpha())
	var focus: Vector3 = main.camera_rig.focus
	return Vector2(focus.x - at.x, focus.z - at.z).length()


## Follow mode with a driving tank (prompt 094; called by the hill test once
## `tank`, selected alone, has its order): turned on with the panel's
## toggle, the camera glides to the tank and then holds it in the screen
## center for three seconds, through a zoom and a rotation; a hand pan ends
## it. Camera only, so the drive is the same; the camera is put back.
func _follow_drive_test(tank: int) -> void:
	var rig: Node3D = main.camera_rig
	var selection: Control = main.selection
	var panel: Control = main.hud.unit_panel
	var focus: Vector3 = rig.focus
	var distance: float = rig.distance
	_check(selection.active_unit == tank, "hill test: active unit %d, expected %d" % [selection.active_unit, tank])
	_left_click(panel.follow_button.get_global_rect().get_center())
	await _frames(2)
	_check(selection.follow and panel.follow_button.button_pressed, "the follow button did not turn follow mode on (hill)")
	_check(selection.selected == [tank], "the follow button changed the selection to %s" % [selection.selected])
	await _frames(90)
	var start := _positions_of([tank])[0]
	var worst := 0.0
	for frame in 180:
		await get_tree().process_frame
		if frame == 60:
			_push_wheel(MOUSE_BUTTON_WHEEL_UP, 4, Vector2(0.25, 0.3))
		elif frame == 120:
			var rotate := InputEventAction.new()
			rotate.action = "camera_rotate_right"
			rotate.pressed = true
			get_viewport().push_input(rotate)
		worst = maxf(worst, _off_center(tank))
	var driven := _positions_of([tank])[0].distance_to(start)
	_check(driven > 10.0, "the followed tank drove only %.1f m" % driven)
	_check(worst < 1.0, "the followed tank was up to %.2f m off the screen center" % worst)
	_check(selection.follow and panel.follow_button.button_pressed, "zoom or rotation ended follow mode on the drive")
	_check(rig.distance < distance - 1.0 and absf(rig.yaw) > 0.5, "zoom or rotation did not work while following (%.1f m, %.3f)" % [rig.distance, rig.yaw])
	var pan := InputEventPanGesture.new()
	pan.position = get_viewport().get_visible_rect().size * 0.5
	pan.delta = Vector2(0.0, -2.0)
	get_viewport().push_input(pan)
	await _frames(2)
	_check(not selection.follow and not panel.follow_button.button_pressed, "a hand pan did not end follow mode on the drive")
	Settings.set_value("camera_follow_active", false)
	Settings.save_file()
	rig.snap_to(focus, distance, 0.0)
	await _frames(1)
	print("self-test: followed tank %d over %.1f m, at most %.3f m off the screen center" % [tank, driven, worst])


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
	# Formations of prompt 046 (a wedge here) are centered by their mean.
	var center := Vector3.ZERO
	for p in positions:
		center += p
	center /= positions.size()
	center.y = 0.0
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
	await _follow_drive_test(tank)

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
	await _steady_zoom_test()
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


## Prompt 090: the shadow distance takes only step values and does not
## switch back and forth at a boundary; a small camera move changes no
## chunk's LOD level.
func _steady_zoom_test() -> void:
	var rig: Node3D = main.camera_rig
	var terrain: Node3D = main.terrain
	var factor: float = rig.shadow_distance_factor
	var step := 0.0
	var values := {}
	var distance: float = rig.min_distance
	while distance <= rig.max_distance:
		step = rig.shadow_step_for(distance * factor, step)
		values[step] = true
		var n := log(step / rig.SHADOW_STEP_BASE) / log(rig.SHADOW_STEP_RATIO)
		_check(absf(n - roundf(n)) < 0.001, "shadow distance %.2f m is no step" % step)
		_check(step >= distance * factor - 0.001, "shadows end at %.1f m, needed %.1f m" % [step, distance * factor])
		_check(step <= distance * factor * rig.SHADOW_STEP_RATIO * (1.0 + rig.SHADOW_STEP_HYSTERESIS) + 0.001,
			"shadow distance %.1f m is too far for %.1f m" % [step, distance * factor])
		_check(rig.near_for(step) <= maxf(0.05, distance * 0.02) + 0.0001, "near plane beyond 2 %% at %.1f m" % distance)
		distance *= 1.01
	var steps_out := values.size()
	_check(steps_out >= 3 and steps_out <= 16, "%d shadow distance steps over the zoom range" % steps_out)
	# Zooming back and forth across a boundary switches once, not every time.
	var boundary: float = rig.SHADOW_STEP_BASE * pow(rig.SHADOW_STEP_RATIO, 3)
	step = rig.shadow_step_for(boundary * 0.99, 0.0)
	var switches := 0
	for i in 20:
		var next: float = rig.shadow_step_for(boundary * (1.01 if i % 2 == 0 else 0.99), step)
		switches += int(next != step)
		step = next
	_check(switches == 1, "shadow distance switched %d times at a boundary" % switches)
	# The rig really uses it.
	rig.snap_to(Vector3(0, 0, 40), 120.0, 0.0)
	_check(is_equal_approx(main.sun.directional_shadow_max_distance, rig.shadow_step_above(120.0 * factor)),
		"sun shadow distance %.1f m is not the step for 120 m" % main.sun.directional_shadow_max_distance)

	# Terrain: jitter the camera by 2 m at a far zoom, where LOD boundaries
	# cross the view.
	var flips := [0, 0]
	for pass_index in 2:
		terrain.lod_hysteresis = 0.1 if pass_index == 0 else 0.0
		terrain.reset_lods()
		rig.snap_to(Vector3(0, 0, 40), 300.0, 0.0)
		await _frames(2)
		var before: PackedInt32Array = terrain.chunk_lods().duplicate()
		for i in 12:
			rig.snap_to(Vector3(2.0 if i % 2 == 0 else 0.0, 0, 40 + (1.5 if i % 3 == 0 else 0.0)), 300.0, 0.0)
			await _frames(1)
			var now: PackedInt32Array = terrain.chunk_lods()
			for k in now.size():
				flips[pass_index] += int(now[k] != before[k])
			before = now.duplicate()
	terrain.lod_hysteresis = 0.1
	terrain.reset_lods()
	_check(flips[0] == 0, "%d LOD switches while the camera moved 2 m" % flips[0])
	print("self-test: steady zoom ok (%d shadow steps, LOD switches %d with hysteresis, %d without)" % [steps_out, flips[0], flips[1]])


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
		# Prompt 057: meadow and flowers only where grass is drawn as grass.
		_check(is_equal_approx(main.terrain.shader_parameter(&"meadow_strength"), preset.meadow_strength)
			and is_equal_approx(main.terrain.shader_parameter(&"flower_density"), preset.flower_density),
			"terrain meadow look does not follow the '%s' preset" % id)
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


## The layer of prompt 098: the whole map stays visible under it. The
## mask is 0 at a never-seen cell (no separate black state any more) and
## 255 where an own unit stands; out of sight the picture keeps a good
## part of its brightness (the headless test has no pixels to read, so
## the layer's own values are checked) and stays darker than in sight.
func _fog_layer_test(fog: Node3D, sim_host: SimHost, own: Array[int]) -> void:
	_check(main.camera.get_node_or_null("FogDesaturate") != null, "fog desaturation quad missing")
	var tint: Color = fog.DIM_TINT
	var kept := tint.get_luminance()
	_check(kept > 0.5 and kept < 0.95 and fog.DESATURATION < 1.0,
		"fog layer too dark or too light: keeps %.2f of the brightness" % kept)
	var image: Image = fog._image
	var size := sim_host.vision_grid_size()
	_check(image != null and image.get_size() == fog.padded_size(), "fog mask missing")
	if image == null:
		return
	var states := sim_host.vision_image()
	var unseen := states.find(0)
	if unseen >= 0:
		var x: int = unseen % size.x + fog.BORDER
		var y: int = floori(float(unseen) / size.x) + fog.BORDER
		_check(image.get_pixel(x, y).r < 0.01, "never-seen cell not under the layer in the mask")
	_check(image.get_pixel(0, 0).r < 0.01, "map border not under the layer")
	var cell := sim_host.vision_cell_size()
	var origin := sim_host.vision_origin()
	var lit := 0
	for p in _positions_of(own):
		var x: int = int(floor((p.x - origin.x) / cell)) + fog.BORDER
		var y: int = int(floor((p.z - origin.y) / cell)) + fog.BORDER
		if image.get_pixel(x, y).r > 0.99:
			lit += 1
	_check(lit > 0, "no own unit stands in a clear cell of the fog mask")


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
	_fog_layer_test(fog, sim_host, own)
	var seen := sim_host.unit_ids().size()
	await _frames(1)
	var in_sight := sim_host.unit_ids()
	for id: int in main._unit_nodes:
		var node: Node3D = main._unit_nodes[id]
		if not (id in in_sight):
			_check(not node.visible, "unit %d is out of sight but drawn" % id)
		else:
			_check(not main._fogged.has(id), "unit %d is in sight but hidden by the fog" % id)
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
	# Quiet again (prompt 050): own tanks that gained a rank from the kill
	# shoot better, and firing at will at enemies that hold fire they would
	# finish them off and decide the match before the later parts.
	main.sim_host.set_hold_fire(PackedInt64Array(own), true)
	view.combat_event.disconnect(on_event)
	# Effects (prompt 019) follow the events: flashes, impacts, the explosion
	# and the burning wreck.
	var counts: Dictionary = main.effects.counts
	_check(counts.get("muzzle_core", 0) > 0 and counts.get("muzzle_tongue", 0) > 0 and counts.get("powder", 0) > 0,
			"no muzzle effects: %s" % counts)
	# Prompt 077: the guns ran back and the tracers started at the models'
	# muzzles.
	_check(main.effects.recoils > 0, "no gun recoil")
	_check(view.tracers_from_muzzle > 0, "no tracer started at a muzzle socket")
	_check(counts.get("sparks", 0) + counts.get("fountain", 0) > 0, "no impact effects: %s" % counts)
	_check(counts.get("fireball", 0) > 0 and counts.get("wreck", 0) >= 1, "no explosion or wreck fire: %s" % counts)
	_check(main.effects.burning_wrecks() >= 1, "no burning wreck")
	# Big blasts and the heavy strike (prompt 054): dome, rings and flash at
	# an own unit; the strike's call bar is there.
	var blasts: Dictionary = main.blast_effects.counts
	var big: int = blasts.get("big_blast", 0)
	main.blast_effects.big_blast((main._unit_nodes[own[0]] as Node3D).global_position, 150.0)
	_check(blasts.get("big_blast", 0) == big + 1 and blasts.get("big_flash", 0) > 0, "no big blast: %s" % blasts)
	_check(main.strike_view.bar != null, "no strike bar")
	print("self-test: combat ok at tick %d: %d shots, %d hits, %d penetrations, %d destroyed, %d enemies left" % [
		sim_host.current_tick(), view.shots, view.hits, view.penetrations, view.destroyed, sim_host.alive_unit_count(1)
	])


# --- 6. Replay ------------------------------------------------------------


## Impact markers of the own shells (prompt 084): hidden until the last
## MARKER_LEAD_SECONDS, then fading in; one ring for a salvo on one point;
## fading out after the impact instead of vanishing; none when disabled.
func _marker_test() -> void:
	var view: Node = main.artillery_view
	var lead: float = view.MARKER_LEAD_SECONDS
	_check(view.marker_alpha(lead + 0.5) == 0.0, "impact marker shown %.1f s before the impact" % (lead + 0.5))
	_check(view.marker_alpha(lead + 0.01) == 0.0, "impact marker shown before its window")
	var early: float = view.marker_alpha(lead - 0.1)
	_check(early > 0.0 and early < 1.0, "impact marker does not fade in (%.2f)" % early)
	_check(view.marker_alpha(0.0) == 1.0, "impact marker not fully shown at the impact")
	var at := Vector3(100, 0, 100)
	var salvo: Array[Array] = [[at, 8.0, false, 1.0], [at + Vector3(2, 0, 1), 8.0, false, 0.5],
			[at + Vector3(30, 0, 0), 8.0, false, 1.0], [at, 8.0, true, 1.0]]
	var merged: Array = view.merge_markers(salvo)
	_check(merged.size() == 3, "salvo on one point drawn as %d markers instead of 3" % merged.size())
	var shown: Dictionary[int, Array] = {1: [at, 8.0, false, 1.0]}
	view._draw_markers([shown[1]] as Array[Array], shown, 0.0)
	_check(view.markers_drawn == 1, "impact marker not drawn")
	var none: Dictionary[int, Array] = {}
	view._draw_markers([] as Array[Array], none, 0.1)
	_check(view.markers_drawn == 1, "impact marker vanished at the impact instead of fading out")
	for i in 20:
		view._draw_markers([] as Array[Array], none, 0.1)
	_check(view.markers_drawn == 0, "impact marker did not fade out")
	view.markers_enabled = false
	view._draw_markers([[at, 8.0, false, 1.0]] as Array[Array], shown, 0.0)
	view._draw_markers([] as Array[Array], none, 0.0)
	_check(view.markers_drawn == 0, "impact marker drawn while disabled")
	view.markers_enabled = true
	var radius: float = view._marker_radius(105.0)
	_check(radius > 1.0 and radius <= 10.5, "impact marker radius %.1f m for 10.5 cm" % radius)
	await get_tree().process_frame


## Own units easier to see (prompt 164): the cursor over an own squad
## highlights it (ring, lit models, bright markers), away from it nothing is
## highlighted and only the faint markers stay; the show-all key lights
## every own unit; the setting `off` draws nothing.
func _highlight_test() -> void:
	var highlight: Node = main.unit_highlight
	var squads := _ids_of(true, true)
	_check(not squads.is_empty(), "no own squad for the highlight test")
	if squads.is_empty():
		return
	var squad := squads[0]
	var at := _centroid([squad] as Array[int])
	highlight.forced_point = at
	await _frames(2)
	_check(highlight.hovered.has(squad) and highlight.nearest == squad,
		"squad %d under the cursor not highlighted (nearest %d)" % [squad, highlight.nearest])
	_check(highlight.lit.has(squad), "squad under the cursor not lit up")
	_check(highlight.markers_bright > 0, "no bright markers under the cursor")
	var ring: MeshInstance3D = main._rings[squad]
	_check(ring.visible and ring.material_override != null, "no hover ring on the squad")
	highlight.forced_point = Vector3(-10000, 0, -10000)
	await _frames(2)
	_check(highlight.hovered.is_empty() and highlight.lit.is_empty(), "highlight stays after the cursor left")
	_check(highlight.markers_drawn > 0 and highlight.markers_bright == 0,
		"faint markers: %d drawn, %d bright" % [highlight.markers_drawn, highlight.markers_bright])
	_check(ring.material_override == null, "hover ring look stays after the cursor left")
	Input.action_press("show_units")
	await _frames(2)
	var own := _ids_of(true, false) + squads
	var shown := 0
	for id in own:
		if highlight.hovered.has(id) or main.selection.is_selected(id):
			shown += 1
	_check(shown == own.size() and highlight.markers_bright == highlight.markers_drawn,
		"show-all: %d of %d own units shown, %d of %d markers bright" % [shown, own.size(),
			highlight.markers_bright, highlight.markers_drawn])
	Input.action_release("show_units")
	Settings.set_value("unit_markers", "off")
	highlight.forced_point = at
	await _frames(2)
	_check(highlight.markers_drawn == 0 and highlight.hovered.is_empty(), "unit markers drawn while off")
	Settings.set_value("unit_markers", "always")
	highlight.forced_point = null
	await _frames(1)


## Prompt 103: the overview goes out and back to the same view; every mark
## switch hides its mark; HUD off and on; the photo mode stops the ticks
## and restores the camera.
func _view_test() -> void:
	var rig: Node3D = main.camera_rig
	var sim_host: SimHost = main.sim_host
	var toggles: Node = main.view_toggles
	var hud: Control = main.hud
	rig.snap_to(_centroid(_ids_of(true)), 120.0, 0.3)
	await _frames(5)
	var before := [rig.target_distance, rig.target_tilt, rig.max_distance]
	main.overview.enter()
	await _frames(150)
	var far: float = main.overview.distance()
	_check(far >= before[2] and absf(rig.distance - far) < 2.0 and not main.sun.shadow_enabled
		and main.unit_highlight.force_show_all,
		"overview: distance %.0f m of %.0f m, shadows %s" % [rig.distance, far, main.sun.shadow_enabled])
	main.overview.leave()
	await _frames(150)
	_check(absf(rig.distance - before[0]) < 1.0 and absf(rig.target_tilt - before[1]) < 0.001
		and rig.max_distance == before[2] and main.sun.shadow_enabled and not main.unit_highlight.force_show_all,
		"overview did not restore the view: %.1f m (was %.1f), tilt %.3f (was %.3f)" % [
			rig.distance, before[0], rig.target_tilt, before[1]])
	# Mark switches.
	var tanks := _tanks(_ids_of(true))
	main.selection.set_selection(tanks.slice(0, 1))
	await _frames(3)
	var ring: Node3D = main._rings[tanks[0]]
	_check(ring.visible, "the selected tank shows no ring")
	var checks := {
		"show_selection_rings": func() -> bool: return not ring.visible,
		"show_routes": func() -> bool: return not hud.waypoints.visible and not hud.order_preview.visible,
		"show_supply_rings": func() -> bool: return not hud.supply_rings.visible,
		"show_impact_markers": func() -> bool: return not main.artillery_view.markers_enabled,
		"show_edge_arrows": func() -> bool: return not hud.alerts.visible,
		"show_relation_tint": func() -> bool: return main._enemy_material.albedo_color.a == 0.0,
	}
	for key: String in checks:
		Settings.set_value(key, false)
		await _frames(2)
		_check((checks[key] as Callable).call(), "switch %s off: its mark still shows" % key)
		Settings.set_value(key, true)
	await _frames(2)
	_check(ring.visible and hud.waypoints.visible and main._enemy_material.albedo_color.a > 0.0,
		"the marks did not come back after the switches")
	Settings.set_value("ground_lines", "subtle")
	await _frames(2)
	_check(hud.waypoints.modulate.a < 1.0, "subtle lines are drawn at full strength")
	Settings.set_value("ground_lines", "strong")
	# HUD off and on.
	toggles.set_hidden(true, true)
	await _frames(2)
	_check(not (main.get_node("Hud") as CanvasLayer).visible and not ring.visible, "HUD off left the HUD or a ring")
	toggles.set_hidden(false)
	await _frames(2)
	_check((main.get_node("Hud") as CanvasLayer).visible and ring.visible, "HUD on did not bring it back")
	# Photo mode.
	var view := [rig.focus, rig.target_distance, rig.target_yaw, rig.target_tilt, rig.fov_deg, rig.min_distance]
	var tick := sim_host.current_tick()
	main.photo_mode.enter()
	main.photo_mode._set_fov(50.0)
	rig.target_distance = 6.0
	await _frames(40)
	_check(get_tree().paused and sim_host.current_tick() == tick and toggles.hidden,
		"photo mode: paused %s, ticks %d -> %d" % [get_tree().paused, tick, sim_host.current_tick()])
	_check(rig.distance < 10.0, "photo mode: the camera does not go closer (%.1f m)" % rig.distance)
	# Headless nothing is drawn: no picture to save (the export self-test
	# and `just`-recipes with a window save one).
	if DisplayServer.get_name() != "headless":
		var path: String = await main.photo_mode.save_photo()
		_check(not path.is_empty() and FileAccess.file_exists(path), "photo mode saved no picture")
		if not path.is_empty():
			DirAccess.remove_absolute(path)
	main.photo_mode.leave()
	await _frames(3)
	_check(not get_tree().paused and not toggles.hidden and rig.focus == view[0] and rig.target_distance == view[1]
		and rig.target_yaw == view[2] and rig.target_tilt == view[3] and rig.fov_deg == view[4] and rig.min_distance == view[5],
		"photo mode did not restore the camera")
	main.selection.set_selection([])


## Tactical pause (prompt 102): the tick stands, the camera moves, an
## order given in the pause starts with the first tick after it; the Esc
## menu keeps the pause; the speed goes to 2x and back.
func _pause_test() -> void:
	var pause: Node = main.tactical_pause
	var sim_host: SimHost = main.sim_host
	_check(pause.available(), "no tactical pause in single player")
	# A squad: tanks may stand by now without fuel or with a thrown track.
	var units := _ids_of(true, true)
	var mover := -1
	var idle := sim_host.unit_idle()
	var immobile := sim_host.unit_immobile()
	for i in _indices(units):
		# Not a tank that lost its track to a mine in the works test.
		if idle[i] != 0 and immobile[i] == 0:
			mover = sim_host.unit_ids()[i]
			break
	_check(mover >= 0, "pause test: no idle own unit")
	pause.pause()
	var tick := sim_host.current_tick()
	await _frames(30)
	_check(get_tree().paused and sim_host.current_tick() == tick,
		"the tick ran on in the pause (%d -> %d)" % [tick, sim_host.current_tick()])
	var rig: Node3D = main.camera_rig
	var goal: Vector3 = rig.focus + Vector3(30.0, 0.0, 0.0)
	rig.center_on(goal)
	await _frames(90)
	_check(Vector2(rig.focus.x, rig.focus.z).distance_to(Vector2(goal.x, goal.z)) < 2.0,
		"the camera did not move in the pause (%s, wanted %s)" % [rig.focus, goal])
	var start: Vector3 = Vector3.ZERO
	if mover >= 0:
		start = sim_host.current_positions()[_indices([mover] as Array[int])[0]]
		# 25 m toward the map center: open ground in test_small.
		var way := Vector3(-start.x, 0.0, -start.z).normalized() * 25.0
		sim_host.move_units(PackedInt64Array([mover]), start.x + way.x, start.z + way.z)
		await _frames(10)
		_check(sim_host.current_tick() == tick and sim_host.unit_idle()[_indices([mover] as Array[int])[0]] != 0,
			"the order of the pause was carried out before the pause ended")
	main.game_menu.open_pause()
	await _frames(2)
	main.game_menu.close_pause()
	await _frames(2)
	_check(pause.active and get_tree().paused and sim_host.current_tick() == tick,
		"the Esc menu ended the tactical pause")
	pause.resume()
	await _ticks(4)
	_check(not get_tree().paused and sim_host.current_tick() > tick, "the game did not go on after the pause")
	if mover >= 0:
		await _ticks(30)
		var k: int = _indices([mover] as Array[int])[0]
		var now: Vector3 = sim_host.current_positions()[k]
		_check(sim_host.unit_idle()[k] == 0 or now.distance_to(start) > 0.5,
			"the order of the pause did not start after it (unit %d moved %.2f m, alive %d)" % [
				mover, now.distance_to(start), sim_host.unit_alive()[k]])
		sim_host.stop_units(PackedInt64Array([mover]))
	pause.set_speed(2.0)
	_check(is_equal_approx(sim_host.speed, 2.0), "speed 2x not set")
	pause.set_speed(1.0)


## Cloud shadows (prompt 087): set in clear weather by day, drifting
## downwind with the game time, standing when the game stands, off in rain,
## at night, with the setting off and by default (thumbnails, editor).
func _cloud_test() -> void:
	var atmosphere: Node = main.atmosphere
	var sim_host: SimHost = main.sim_host
	var default_shape: Vector4 = ProjectSettings.get_setting("shader_globals/cloud_shadow_shape")["value"]
	_check(default_shape.y == 0.0, "cloud shadows on by default (thumbnails would show them)")
	var base: Resource = atmosphere.load_preset(atmosphere.preset.id)
	var weather_look: GDScript = load("res://scenes/world/weather_look.gd")
	for case: Array in [["rain", "day"], ["snow", "day"], ["fog", "day"], ["clear", "night"]]:
		var p: Resource = weather_look.adjust(base, case[0], case[1])
		_check(p.cloud_shadow_strength == 0.0, "cloud shadows in %s / %s" % case)
	var overcast: Resource = weather_look.adjust(base, "overcast", "day")
	_check(overcast.cloud_shadow_cover > base.cloud_shadow_cover, "overcast has no closed cloud deck")
	if atmosphere.preset.cloud_shadow_strength <= 0.0:
		print("self-test: cloud shadows off in this weather, drift not checked")
		return
	await _frames(2)
	_check(atmosphere.cloud_shape.y > 0.0, "no cloud shadows in clear weather")
	var before: Vector4 = atmosphere.cloud_offset
	await _ticks(40)
	await _frames(1)
	var moved := Vector2(atmosphere.cloud_offset.x - before.x, atmosphere.cloud_offset.y - before.y)
	moved = Vector2(wrapf(moved.x, -0.5, 0.5), wrapf(moved.y, -0.5, 0.5))
	var wind: Vector2 = sim_host.weather_info().get("wind", Vector2.ZERO)
	var down: Vector2 = wind.normalized() if wind.length() > 0.01 else atmosphere.preset.wind_direction.normalized()
	# The texture coordinate moves upwind, so the pattern moves downwind.
	_check(moved.length() > 0.0 and moved.normalized().dot(-down) > 0.99,
		"cloud shadows do not drift downwind (%s, wind %s)" % [moved, down])
	var speed: float = sim_host.speed
	sim_host.speed = 0.0
	await _frames(2)
	var stood: Vector4 = atmosphere.cloud_offset
	await _frames(10)
	_check(atmosphere.cloud_offset == stood, "cloud shadows move while the game stands")
	sim_host.speed = speed
	Settings.values["cloud_shadows"] = false
	await _frames(2)
	_check(atmosphere.cloud_shape.y == 0.0, "the setting does not turn cloud shadows off")
	Settings.values["cloud_shadows"] = true
	await _frames(2)
	_check(atmosphere.cloud_shape.y > 0.0, "cloud shadows do not come back with the setting")


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
## their infantry squads instead (prompt 022), without crews that bailed
## out of destroyed tanks (prompt 049).
func _ids_of(own: bool, squads := false) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var stances := sim_host.unit_stances()
	var result: Array[int] = []
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own and (stances[i] >= 0) == squads \
				and not (squads and sim_host.unit_is_crew(ids[i])):
			result.append(ids[i])
	result.sort()
	return result


## Tanks of a faction in battle mode (prompt 045), from the type of the
## first of `ids`; 0 if unknown.
func _expected_tanks(ids: Array[int]) -> int:
	if ids.is_empty():
		return 0
	var counts := {"de": 8, "su": 10, "us": 9}
	var sim_host: SimHost = main.sim_host
	var types := sim_host.unit_type_ids()
	var index := Array(sim_host.unit_ids()).find(ids[0])
	return counts.get(types[index].get_slice("_", 0), 0)


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


func _key(keycode: Key, ctrl := false, shift := false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		# Order keys are input actions bound to physical keys (prompt 031).
		event.physical_keycode = keycode
		event.ctrl_pressed = ctrl
		event.shift_pressed = shift
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
