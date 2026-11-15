extends Node
## Self-test of the whole game flow (prompt 027), started by the main menu
## with `-- --self-test` (`just selftest`). Lives on the scene tree root, so
## it survives the scene changes it drives, and runs while the game is paused.
## Quits with exit code 0 (pass) or 1 (fail). Frame counts assume
## `--fixed-fps 60`.
##
## 1. Menu: all main buttons exist.
## 2. Settings: change quality, pan speed and two key bindings (the second
##    one takes the key of the first, which gets the second's old key), save,
##    load again, then restore the defaults. Uses its own settings file.
## 3. LAN: the lobby opens from the menu and its Back button returns; the
##    same for the online lobby (prompt 026), without initializing Steam.
## 4. Match 1 (test_small, Germany vs. Soviet Union, from the setup screen):
##    loading screen steps, then all match tests of `self_test.gd`; Esc
##    pauses (no ticks while paused) and resumes; surrender from the pause
##    menu gives a defeat; "Back to main menu" returns.
## 5. Match 2 (United States vs. Germany, winter): units and biome as
##    chosen; everyone starts with hold fire, the own units attack one enemy
##    after the other until the last is destroyed: victory by elimination in
##    real combat; back to the menu.
## 6. Match 3 (prompt 024): against the hard computer opponent while the own
##    tanks hold fire: the AI must give orders, open fire and destroy them
##    (defeat by elimination); back to the menu.
##
## 7. Match 4 (prompt 023): base building against the normal computer
##    opponent at eight times the speed: the build menu offers buildings,
##    a barracks is placed through the placement preview, completed,
##    selected, and produces a squad that appears and costs resources; the
##    computer opponent builds too; supply points are drawn; surrender and
##    back to the menu.
##
## Matches 1 and 2 choose the passive opponent, which gives no orders, so
## they play as before the computer opponent existed. Matches 1 to 3 play
## the "battle" mode (the fixed armies of the demo scenario).

signal match_tested

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
## Wall-clock limit for the whole flow (`--fixed-fps` decouples frame time).
const TIMEOUT_MS := 600000

var _errors: PackedStringArray = []
var _match_done := false
var _match_ok := false
var _started_ms := Time.get_ticks_msec()


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	_run()


func _process(_delta: float) -> void:
	if Time.get_ticks_msec() - _started_ms > TIMEOUT_MS:
		_check(false, "flow self-test timed out")
		_finish()


## Called by `self_test.gd` when the match tests of the first match are done.
func match_test_finished(ok: bool, message: String) -> void:
	_match_ok = ok
	_match_done = true
	if not ok:
		_check(false, "match tests failed: %s" % message)
	match_tested.emit()


func _run() -> void:
	var menu: Control = await _wait_scene("MainMenu")
	if menu == null:
		_finish()
		return
	for button: Button in [menu.single_player_button, menu.lan_button, menu.online_button, menu.settings_button, menu.quit_button]:
		_check(button != null and button.is_visible_in_tree(), "main menu button missing")
	await _settings_test(menu)
	menu = await _lan_test(menu)
	if menu != null:
		menu = await _online_test(menu)
	if menu == null:
		_finish()
		return

	# --- Match 1 -------------------------------------------------------
	var main: Node3D = await _start_match(menu, 0, 0, 1, "test_small")
	if main == null:
		_finish()
		return
	_check(MatchSetup.last_loading_steps.size() >= 5, "loading screen showed %s" % MatchSetup.last_loading_steps)
	if not _match_done:
		await match_tested
	await _pause_test(main)
	await _surrender_test(main)
	menu = await _back_to_menu(main)
	if menu == null:
		_finish()
		return

	# --- Match 2 -------------------------------------------------------
	main = await _start_match(menu, 3, 2, 0, "test_small")
	if main == null:
		_finish()
		return
	await _combat_victory_test(main)
	menu = await _back_to_menu(main)
	if menu == null:
		_finish()
		return

	# --- Match 3: computer opponent -----------------------------------
	main = await _start_match(menu, 2, 0, 1, "test_small", "hard")
	if main == null:
		_finish()
		return
	await _ai_opponent_test(main)
	menu = await _back_to_menu(main)
	if menu == null:
		_finish()
		return

	# --- Match 4: base building ----------------------------------------
	main = await _start_match(menu, 0, 0, 1, "test_small", "normal", "base")
	if main == null:
		_finish()
		return
	await _base_building_test(main)
	await _surrender_test(main)
	menu = await _back_to_menu(main)
	_finish()


func _finish() -> void:
	set_process(false)
	var ok := _errors.is_empty()
	var message := "menu, settings, LAN lobby, online lobby, loading, match tests, pause, surrender, combat victory, computer opponent, base building, result and back ok" if ok else " ".join(_errors)
	print("flow self-test %s: %s" % ["PASSED" if ok else "FAILED", message])
	get_tree().paused = false
	get_tree().quit(0 if ok else 1)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_errors.append(message + ".")
		push_error("flow self-test: " + message)


# --- Settings ---------------------------------------------------------


func _settings_test(menu: Control) -> void:
	Settings.reset()
	Settings.save_file()
	menu.settings_button.pressed.emit()
	await _frames(2)
	var panel: PanelContainer = menu.settings_panel
	_check(panel.is_visible_in_tree(), "settings screen not shown")
	panel.quality_option.item_selected.emit(0)
	panel.pan_slider.value = 1.5
	var rotate_right_key: int = Settings.bindings["camera_rotate_right"]
	panel.capture("camera_rotate_left")
	_push_key(KEY_R)
	await _frames(2)
	_check(_action_key("camera_rotate_left") == KEY_R, "rotate left not bound to R")
	panel.capture("camera_rotate_right")
	_push_key(KEY_R)
	await _frames(2)
	_check(_action_key("camera_rotate_right") == KEY_R, "rotate right not bound to R")
	_check(_action_key("camera_rotate_left") == rotate_right_key, "rotate left did not get the key of rotate right")
	panel.save_button.pressed.emit()
	await _frames(2)
	_check(menu.screens["main"].visible, "saving the settings did not return to the main menu")
	_check(FileAccess.file_exists(Settings.path()), "settings file not written")

	Settings.reset()
	Settings.load_file()
	_check(Settings.get_value("quality") == "low", "quality not loaded")
	_check(is_equal_approx(Settings.get_value("pan_sensitivity"), 1.5), "pan speed not loaded")
	_check(_action_key("camera_rotate_right") == KEY_R, "key binding not loaded")
	# The match tests expect the defaults.
	Settings.reset()
	Settings.save_file()
	_check(_action_key("camera_rotate_right") == KEY_E, "defaults not restored")
	print("flow self-test: settings ok (%s)" % ProjectSettings.globalize_path(Settings.path()))


func _action_key(action: String) -> int:
	for event in InputMap.action_get_events(action):
		if event is InputEventKey:
			return (event as InputEventKey).physical_keycode
	return 0


# --- LAN ---------------------------------------------------------------


func _lan_test(menu: Control) -> Control:
	menu.lan_button.pressed.emit()
	var lobby: Control = await _wait_scene("LanLobby")
	if lobby == null:
		return null
	_check(lobby.back_button != null and lobby.back_button.is_visible_in_tree(), "LAN lobby has no Back button")
	lobby.back_button.pressed.emit()
	menu = await _wait_scene("MainMenu")
	print("flow self-test: LAN lobby opened and closed")
	return menu


# --- Online (prompt 026) --------------------------------------------------


## Opens the online lobby without initializing Steam (`--self-test` counts
## as `--no-steam` there) and goes back.
func _online_test(menu: Control) -> Control:
	menu.online_button.pressed.emit()
	var lobby: Control = await _wait_scene("OnlineLobby")
	if lobby == null:
		return null
	_check(lobby.back_button != null and lobby.back_button.is_visible_in_tree(), "online lobby has no Back button")
	_check(not lobby._connected, "the flow self-test must not initialize Steam")
	lobby.back_button.pressed.emit()
	menu = await _wait_scene("MainMenu")
	print("flow self-test: online lobby opened and closed")
	return menu


# --- Matches ------------------------------------------------------------


## Chooses biome, own and enemy faction (indices in the setup screen) and
## map, starts and waits until the match is loaded.
func _start_match(menu: Control, biome: int, own: int, enemy: int, map_name: String, opponent := "passive", mode := "battle") -> Node3D:
	menu.single_player_button.pressed.emit()
	await _frames(1)
	_check(menu.screens["skirmish"].visible, "setup screen not shown")
	var map_index := Array(menu._maps).find(map_name)
	_check(map_index >= 0, "map %s not offered" % map_name)
	menu.map_option.select(maxi(map_index, 0))
	menu.biome_option.select(biome)
	menu.faction_option.select(own)
	menu.enemy_option.select(enemy)
	menu.opponent_option.select(MatchSetup.OPPONENTS.keys().find(opponent))
	menu.mode_option.select(MatchSetup.MODES.keys().find(mode))
	MatchSetup.run_match_test = biome == 0 and own == 0 and mode == "battle"
	menu.start_button.pressed.emit()
	var main: Node3D = await _wait_scene("Main")
	if main == null:
		return null
	var frames := 600
	while not main._loaded and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(main._loaded, "match did not finish loading")
	print("flow self-test: match loaded (%s), loading steps %s" % [map_name, MatchSetup.last_loading_steps])
	return main


func _pause_test(main: Node3D) -> void:
	var game_menu: CanvasLayer = main.game_menu
	_push_key(KEY_ESCAPE, true)
	await _frames(2)
	_check(game_menu.is_paused() and get_tree().paused, "Esc did not pause the game")
	var tick: int = main.sim_host.current_tick()
	await _frames(30)
	_check(main.sim_host.current_tick() == tick, "simulation ran while paused")
	_push_key(KEY_ESCAPE, true)
	await _frames(30)
	_check(not get_tree().paused and not game_menu.is_paused(), "Esc did not resume the game")
	_check(main.sim_host.current_tick() > tick, "simulation did not resume")
	print("flow self-test: pause ok")


func _surrender_test(main: Node3D) -> void:
	var game_menu: CanvasLayer = main.game_menu
	game_menu.open_pause()
	await _frames(1)
	game_menu.surrender_button.pressed.emit()
	_check(game_menu.confirm_box.visible, "surrender asks no confirmation")
	game_menu.confirm_surrender_button.pressed.emit()
	await _wait_result(game_menu)
	var result: Dictionary = main.sim_host.match_result()
	_check(result["decided"] and result["winner"] == 1, "surrender did not decide the match for player 1: %s" % result)
	_check(result["players"][0]["reason"] == "surrendered", "player 0 not marked as surrendered")
	_check(game_menu.result_title.text == "DEFEAT", "result shows '%s' after surrendering" % game_menu.result_title.text)
	_check(game_menu.result_reason.text == "You surrendered.", "result reason '%s'" % game_menu.result_reason.text)
	print("flow self-test: surrender ok (%s)" % game_menu.result_details.text.replace("\n", "; "))


func _combat_victory_test(main: Node3D) -> void:
	var host: SimHost = main.sim_host
	var own := PackedInt64Array()
	var enemies := PackedInt64Array()
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	for i in ids.size():
		var mine := owners[i] == host.local_player()
		var expected := "us_" if mine else "de_"
		_check(types[i].begins_with(expected), "unit %d is %s, expected %s…" % [ids[i], types[i], expected])
		if mine:
			own.append(ids[i])
		else:
			enemies.append(ids[i])
	_check(main.atmosphere.preset.id == "winter", "biome is %s, not winter" % main.atmosphere.preset.id)
	# Eight tanks and two rifle squads per side (prompt 022).
	_check(own.size() == 10 and enemies.size() == 10, "expected 10 units per side")
	# Everyone starts with hold fire in the self-test: the own units attack
	# one enemy after the other until none is left.
	var game_menu: CanvasLayer = main.game_menu
	var frames := 12000
	var last_order_tick := -1000
	while not game_menu.result_panel.visible and frames > 0:
		frames -= 1
		var tick := host.current_tick()
		if tick - last_order_tick >= 40:
			last_order_tick = tick
			var alive := host.unit_alive()
			for i in ids.size():
				if owners[i] != host.local_player() and alive[i] != 0:
					host.attack_unit(own, ids[i])
					break
		await get_tree().process_frame
	_check(game_menu.result_panel.visible, "the attack did not decide the match (%d enemies left)" % host.alive_unit_count(1))
	var result: Dictionary = host.match_result()
	_check(result["decided"] and result["winner"] == host.local_player(), "combat did not decide the match for us: %s" % result)
	_check(result["players"][1]["reason"] == "eliminated", "player 1 not marked as eliminated")
	_check(host.alive_unit_count(1) == 0, "enemy units left")
	_check(game_menu.result_title.text == "VICTORY", "result shows '%s'" % game_menu.result_title.text)
	_check(game_menu.result_reason.text == "All enemy units were destroyed.", "result reason '%s'" % game_menu.result_reason.text)
	_check(game_menu.result_details.text.contains("Germany: 0 units left – eliminated"), "result details: %s" % game_menu.result_details.text)
	print("flow self-test: combat victory ok at tick %d (%s)" % [result["tick"], game_menu.result_details.text.replace("\n", "; ")])


## The hard computer opponent beats tanks that hold fire.
func _ai_opponent_test(main: Node3D) -> void:
	var host: SimHost = main.sim_host
	var status: Dictionary = host.ai_status()
	_check(status["difficulty"] == "hard" and status["players"] == 1, "computer opponent not running: %s" % status)
	_check(not host.is_networked(), "a match against the computer counts as networked")
	var game_menu: CanvasLayer = main.game_menu
	var frames := 9000
	while not game_menu.result_panel.visible and frames > 0:
		frames -= 1
		await get_tree().process_frame
	status = host.ai_status()
	_check(status["decisions"] > 0, "the computer opponent never decided anything")
	_check(game_menu.result_panel.visible, "the computer opponent did not decide the match (%d own tanks left)" % host.alive_unit_count(0))
	var result: Dictionary = host.match_result()
	_check(result["decided"] and result["winner"] == 1, "the computer opponent did not win: %s" % result)
	_check(result["players"][0]["reason"] == "eliminated", "player 0 not eliminated: %s" % result)
	_check(game_menu.result_title.text == "DEFEAT", "result shows '%s'" % game_menu.result_title.text)
	print("flow self-test: computer opponent won at tick %d after %d decisions (%s)" % [result["tick"], status["decisions"], game_menu.result_details.text.replace("\n", "; ")])


func _base_building_test(main: Node3D) -> void:
	var host: SimHost = main.sim_host
	var hud: Control = main.base_hud
	host.speed = 8.0
	await _frames(30)
	var state: Dictionary = host.economy_state()
	_check(state["enabled"] and hud.visible, "base building HUD not shown: %s" % state)
	_check(main.base_view._supply_nodes.size() == 3, "supply points not drawn")
	var barracks := -1
	for o: Dictionary in host.build_options():
		if o["id"] == "base_barracks":
			barracks = o["type"]
	_check(barracks >= 0, "barracks not in the build menu")
	# Place it through the preview on the first valid spot around the HQ.
	var hq := Vector3.ZERO
	for s: Dictionary in host.structures():
		if s["owner"] == 0 and s["type_id"] == "base_hq":
			var index := Array(host.unit_ids()).find(s["id"])
			hq = host.current_positions()[index]
	hud.start_placing(barracks)
	var placed := false
	for r: float in [20.0, 30.0, 40.0, 50.0]:
		for k in 16:
			var angle := TAU * k / 16.0
			var p := hq + Vector3(cos(angle), 0, sin(angle)) * r
			if not placed and host.placement_check(barracks, p.x, p.z).is_empty():
				placed = hud.place_at(Vector3(p.x, 0, p.z))
	hud.cancel_placing()
	_check(placed, "no valid spot for a barracks near the headquarters")
	var before: Dictionary = host.economy_state()
	# Wait until it is completed (25 s game time).
	var site := {}
	var frames := 1200
	while frames > 0:
		frames -= 1
		await get_tree().process_frame
		for s: Dictionary in host.structures():
			if s["owner"] == 0 and s["type_id"] == "base_barracks":
				site = s
		if not site.is_empty() and float(site["built"]) >= 1.0:
			break
	_check(not site.is_empty() and float(site["built"]) >= 1.0, "barracks not completed: %s" % site)
	if site.is_empty():
		return
	main.selection._set_selection([site["id"]])
	await _frames(20)
	_check(hud._production_panel.visible, "production bar not shown for the barracks")
	var options: Array = host.production_options(site["id"])
	_check(not options.is_empty(), "the barracks offers nothing")
	var own_units := func() -> int:
		var n := 0
		var ids := host.unit_ids()
		var owners := host.unit_owners()
		for i in ids.size():
			if owners[i] == 0 and not main.base_view.structures.has(ids[i]) and not host.structures().any(func(s: Dictionary) -> bool: return s["id"] == ids[i]):
				n += 1
		return n
	var units_before: int = own_units.call()
	var buttons: Array = hud._production_options.get_children().filter(func(c: Node) -> bool: return c is Button and not (c as Button).disabled)
	_check(not buttons.is_empty(), "no production button enabled")
	if not buttons.is_empty():
		(buttons[0] as Button).pressed.emit()
	frames = 1200
	while own_units.call() == units_before and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(own_units.call() > units_before, "no unit produced")
	var after: Dictionary = host.economy_state()
	_check(int(after["manpower"]) < int(before["manpower"]) + 60, "production cost nothing: %s -> %s" % [before, after])
	var ai_buildings := 0
	for s: Dictionary in host.structures():
		if s["owner"] == 1:
			ai_buildings += 1
	_check(ai_buildings >= 2, "the computer opponent built nothing")
	host.speed = 1.0
	print("flow self-test: base building ok (barracks built, unit produced, computer opponent has %d buildings, tick %d)" % [ai_buildings, host.current_tick()])


func _wait_result(game_menu: CanvasLayer) -> void:
	var frames := 300
	while not game_menu.result_panel.visible and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(game_menu.result_panel.visible, "result screen not shown")


func _back_to_menu(main: Node3D) -> Control:
	main.game_menu.menu_button.pressed.emit()
	var menu: Control = await _wait_scene("MainMenu")
	_check(not get_tree().paused, "tree still paused in the menu")
	return menu


# --- Helpers -----------------------------------------------------------


func _wait_scene(scene_name: String) -> Node:
	for i in 600:
		var scene := get_tree().current_scene
		if scene != null and scene.name == scene_name and scene.is_node_ready():
			await _frames(2)
			return scene
		await get_tree().process_frame
	_check(false, "scene %s did not appear" % scene_name)
	return null


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame


func _push_key(keycode: Key, logical := false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.pressed = pressed
		if logical:
			event.keycode = keycode
		event.physical_keycode = keycode
		get_tree().root.push_input(event)
