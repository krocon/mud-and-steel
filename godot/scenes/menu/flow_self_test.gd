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
##    Language and accessibility (prompt 043, `l10n_self_test.gd`): every
##    menu screen and settings tab in German without a missing translation,
##    the new settings saved and read back; in match 1 the HUD and the pause
##    menu in German, subtitles.
##    Unit reference (prompt 066, `tools/units_self_test.gd`): every entry
##    opens its unit card.
## 3. LAN: the lobby opens from the menu and its Back button returns; the
##    same for the online lobby (prompt 026), without initializing Steam.
## 4. Match 1 (test_small, Germany vs. Soviet Union, from the setup screen):
##    loading screen steps, then all match tests of `self_test.gd`; Esc
##    pauses (no ticks while paused) and resumes; surrender from the pause
##    menu gives a defeat; "Back to main menu" returns.
## 5. Match 2 (United States vs. Germany, winter): units and biome as
##    chosen; everyone starts with hold fire, the own units attack one enemy
##    after the other until the last is destroyed: victory by elimination in
##    real combat (the music switches to combat and plays the victory
##    piece, prompt 030); back to the menu, which plays the menu music.
## 6. Match 3 (prompt 024): against the hard computer opponent while the own
##    tanks hold fire: the AI must give orders, open fire and destroy them
##    (defeat by elimination); back to the menu.
##
## 7. Match 4 (prompt 023): base building against the normal computer
##    opponent at eight times the speed: the build menu offers buildings,
##    a barracks is placed through the placement preview, completed,
##    selected, and produces a squad that appears and costs resources; the
##    computer opponent builds too; supply points are drawn; surrender and
##    back to the menu. Before that, readable supply points (prompt 107,
##    `tools/supply_point_self_test.gd`): minimap marks neutral, then own
##    after the own units took one; border radius from the data; the
##    border setting; the ground tooltip.
##
## 8. Saved game and replay viewer (prompt 038): match 4 is saved from the
##    pause menu before the surrender; "Load game" continues it at the saved
##    tick and state hash with its computer opponent; leaving writes the
##    replay of the whole match. "Replays" lists it (and greys out a file of
##    an older format); watching it: 8× speed, pause, a jump (from a
##    snapshot), the perspective of player 1 and of everything, and the end
##    of the replay reaches its recorded end hash; back to the menu.
##
## 9. Team match (prompt 037, after match 3): 2 against 2 on twin_meadow
##    from the setup screen, with a passive computer ally and two passive
##    computer enemies: four players, the ally's units tinted and counted
##    as allies, a ping reaches the minimap; surrendering does not decide
##    the match while the ally stands; leave to the menu.
##
## 10. Tutorial and missions (prompt 039, `mission_self_test.gd`, after
##	match 3): the menu entries, the tutorial and every mission played to
##	victory, the wins in the (self-test's) progress file.
##
## Matches 1 and 2 choose the passive opponent, which gives no orders, so
## they play as before the computer opponent existed. Matches 1 to 3 play
## the "battle" mode (the fixed armies of the demo scenario).

signal match_tested

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")
const MusicSelfTest := preload("res://scenes/tools/music_self_test.gd")
const MissionSelfTest := preload("res://scenes/menu/mission_self_test.gd")
const L10nSelfTest := preload("res://scenes/menu/l10n_self_test.gd")
## Unit reference (prompt 066).
const UnitsSelfTest := preload("res://scenes/tools/units_self_test.gd")
const SupplyPointSelfTest := preload("res://scenes/tools/supply_point_self_test.gd")
## Wall-clock limit for the whole flow (`--fixed-fps` decouples frame time);
## raised from 10 to 20 minutes for the tutorial and missions (prompt 039).
const TIMEOUT_MS := 1200000

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
	await L10nSelfTest.new(self).menu_test(menu)
	await _credits_test(menu)
	await UnitsSelfTest.new().run(self, menu)
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
	await L10nSelfTest.new(self).match_test(main)
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
	await MusicSelfTest.flow_result(self, main, "victory")
	menu = await _back_to_menu(main)
	if menu == null:
		_finish()
		return
	MusicSelfTest.check_menu(self)

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

	# --- Tutorial and missions (prompt 039) -----------------------------
	menu = await MissionSelfTest.new(self).run(menu)
	if menu == null:
		_finish()
		return

	# --- Team match (prompt 037) -----------------------------------------
	main = await _team_test(menu)
	if main == null:
		_finish()
		return
	menu = await _quit_to_menu(main)
	if menu == null:
		_finish()
		return

	# --- Weather match (prompt 040): rain at night --------------------
	main = await _start_match(menu, 0, 1, 0, "test_small", "passive", "battle", "rain", "night")
	if main == null:
		_finish()
		return
	await _weather_test(main)
	await _surrender_test(main)
	menu = await _back_to_menu(main)
	if menu == null:
		_finish()
		return

	# --- Match 4: base building ----------------------------------------
	main = await _start_match(menu, 0, 0, 1, "test_small", "normal", "base")
	if main == null:
		_finish()
		return
	await SupplyPointSelfTest.new().run(self, main)
	await _base_building_test(main)
	var saved := await _save_test(main)
	await _surrender_test(main)
	menu = await _back_to_menu(main)
	if menu == null or saved.is_empty():
		_finish()
		return

	# --- Saved game and replay viewer (prompt 038) ---------------------
	main = await _load_test(menu, saved)
	if main == null:
		_finish()
		return
	menu = await _quit_to_menu(main)
	if menu != null:
		await _replay_viewer_test(menu, saved)
	DirAccess.remove_absolute(saved["path"])
	_finish()


func _finish() -> void:
	set_process(false)
	var ok := _errors.is_empty()
	var message := "menu, settings, language and accessibility, credits, LAN lobby, online lobby, loading, match tests, pause, surrender, combat victory, computer opponent, tutorial and missions, rain at night, base building, saving, loading, replay viewer, result and back ok" if ok else " ".join(_errors)
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
	panel.dof_option.item_selected.emit(Settings.DOF_MODES.find("miniature"))
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
	# The follow mode of the camera (prompt 094) is kept for the next match.
	_check(not Settings.get_value("camera_follow_active"), "follow mode is on by default")
	Settings.set_value("camera_follow_active", true)
	panel.save_button.pressed.emit()
	await _frames(2)
	_check(menu.screens["main"].visible, "saving the settings did not return to the main menu")
	_check(FileAccess.file_exists(Settings.path()), "settings file not written")

	Settings.reset()
	_check(not Settings.get_value("camera_follow_active"), "the defaults leave follow mode on")
	Settings.load_file()
	_check(Settings.get_value("camera_follow_active"), "follow mode not loaded")
	_check(Settings.get_value("quality") == "low", "quality not loaded")
	_check(Settings.get_value("depth_of_field") == "miniature", "depth of field mode not loaded")
	_check(is_equal_approx(Settings.get_value("pan_sensitivity"), 1.5), "pan speed not loaded")
	_check(_action_key("camera_rotate_right") == KEY_R, "key binding not loaded")
	# Older files stored the depth of field as a switch.
	var old_file := ConfigFile.new()
	old_file.set_value("game", "depth_of_field", false)
	old_file.save(Settings.path())
	Settings.load_file()
	_check(Settings.get_value("depth_of_field") == "off", "old depth of field switch not converted")
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


## Credits and licenses (prompt 044): every tab has text, Back returns.
func _credits_test(menu: Control) -> void:
	_check(menu.credits_button != null and menu.credits_button.is_visible_in_tree(), "main menu has no Credits button")
	menu.credits_button.pressed.emit()
	await _frames(2)
	var panel: PanelContainer = menu.credits_panel
	_check(panel.is_visible_in_tree(), "credits screen not shown")
	_check(panel.tabs.get_tab_count() == 4, "credits screen has %d tabs" % panel.tabs.get_tab_count())
	for view: TextEdit in panel.tabs.get_children():
		# The third-party file only exists after a release build.
		var minimum := 10 if view.name == "CREDITS_TAB_THIRD_PARTY" else 100
		_check(view.text.length() > minimum, "credits tab '%s' is (nearly) empty" % view.name)
	_check(panel.tabs.get_child(3).text.contains("Godot Engine"), "Godot license missing in the credits")
	panel.back_button.pressed.emit()
	await _frames(2)
	_check(menu.screens["main"].visible, "Back on the credits screen did not return to the main menu")
	print("flow self-test: credits opened and closed")


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
func _start_match(menu: Control, biome: int, own: int, enemy: int, map_name: String, opponent := "passive", mode := "battle", weather := "", daytime := "") -> Node3D:
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
	menu.weather_option.select(MatchSetup.WEATHERS.keys().find(weather))
	menu.daytime_option.select(MatchSetup.DAYTIMES.keys().find(daytime))
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


## 2 against 2 with computers (prompt 037).
func _team_test(menu: Control) -> Node3D:
	menu.single_player_button.pressed.emit()
	await _frames(1)
	var map_index := Array(menu._maps).find("twin_meadow")
	_check(map_index >= 0, "map twin_meadow not offered")
	menu.map_option.select(maxi(map_index, 0))
	menu.map_option.item_selected.emit(maxi(map_index, 0))
	menu.biome_option.select(0)
	menu.faction_option.select(0)
	menu.enemy_option.select(1)
	menu.opponent_option.select(MatchSetup.OPPONENTS.keys().find("passive"))
	menu.mode_option.select(MatchSetup.MODES.keys().find("battle"))
	_check(menu.players_option.item_count >= 2, "twin_meadow offers no 2 vs 2")
	menu.players_option.select(1)
	menu.players_option.item_selected.emit(1)
	var slots: Array = menu.slot_list.slots()
	_check(slots.size() == 4 and menu.slot_list.visible, "2 vs 2 shows %d slots" % slots.size())
	MatchSetup.run_match_test = false
	menu.start_button.pressed.emit()
	var main: Node3D = await _wait_scene("Main")
	if main == null:
		return null
	var frames := 600
	while not main._loaded and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(main._loaded, "team match did not finish loading")
	var host: SimHost = main.sim_host
	await _frames(30)
	var players: Array = host.match_players()
	_check(players.size() == 4, "team match has %d players" % players.size())
	_check(host.relation(0) == 0 and host.relation(2) == 1 and host.relation(1) == 2 and host.relation(3) == 2,
		"relations %s" % [players])
	var relations := host.unit_relations()
	_check(Array(relations).has(1), "no allied units in the snapshot")
	var ally_tinted := false
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	for i in ids.size():
		if owners[i] == 2 and main._unit_nodes.has(ids[i]):
			ally_tinted = true
	_check(ally_tinted, "allied units are not drawn")
	var pings_before: int = main.hud.minimap.ping_count
	host.ping(0.0, 0.0)
	await _frames(40)
	_check(main.hud.minimap.ping_count > pings_before, "the ping did not reach the minimap")
	# Surrender: the passive ally still stands, so the match goes on.
	host.surrender()
	await _frames(30)
	var result: Dictionary = host.match_result()
	_check(not result["decided"], "the surrender decided the team match: %s" % result)
	_check(result["players"][0]["defeated"], "player 0 not defeated after surrendering")
	print("flow self-test: team match ok (%d players, ping, surrender keeps the ally in)" % players.size())
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


## Rain at night (prompt 040): the simulation has the weather from the
## setup screen, the ground is soft, sight is short; rain falls, the ground
## is wet, the light is the night's, the ambience plays rain. Measures what
## the weather costs per frame on the CPU.
func _weather_test(main: Node3D) -> void:
	var host: SimHost = main.sim_host
	var info: Dictionary = host.weather_info()
	_check(info.get("weather") == "rain" and info.get("daytime") == "night", "match weather %s, chosen rain at night" % info)
	_check(info.get("ground") == "wet" and int(info.get("ground_pct", 0)) >= 50, "ground %s" % info)
	_check(int(info.get("sight_pct", 100)) < 50, "sight at a rainy night %s %%" % info.get("sight_pct"))
	var look: Node = main.weather_look
	_check(look != null, "no weather look")
	if look == null:
		return
	var quality: Dictionary = preload("res://scenes/world/graphics_quality.gd").settings(main.atmosphere.quality)
	_check(look.rain != null or int(quality.snow_particles) == 0, "no rain particles")
	var wet: Variant = main.terrain.shader_parameter("wetness")
	_check(wet != null and float(wet) > 0.5, "terrain wetness %s" % wet)
	var base_energy: float = main.atmosphere._biome_preset.sun_energy
	_check(main.atmosphere.preset.sun_energy < base_energy * 0.2, "night light %.2f of %.2f" % [main.atmosphere.preset.sun_energy, base_energy])
	if main.battle_sound:
		_check(main.battle_sound.ambience.rain > 0.9, "ambience without rain")
	var tick := host.current_tick()
	await _frames(120)
	_check(host.current_tick() > tick + 30, "the rainy match does not run (tick %d)" % host.current_tick())
	# CPU cost of the weather look per frame (rain moves on the GPU).
	var started := Time.get_ticks_usec()
	for i in 200:
		look._process(1.0 / 60.0)
	var usec := float(Time.get_ticks_usec() - started) / 200.0
	_check(usec < 2000.0, "weather costs %.0f µs per frame" % usec)
	print("flow self-test: rain at night ok (ground %d %%, sight %d %%, wetness %.2f, rain %s, weather %.1f µs per frame)" % [
		int(info.get("ground_pct", 0)), int(info.get("sight_pct", 0)), float(wet), "on" if look.rain else "off", usec
	])


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
	# Two rifle squads per side (prompt 022) and the tanks of the faction
	# (prompt 045: 9 M4A3 against 8 Panzer IV).
	_check(own.size() == 11 and enemies.size() == 10, "expected 11 own and 10 enemy units, got %d/%d" % [own.size(), enemies.size()])
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
	# Prompt 045: statistics on the result screen and as JSON.
	_check(game_menu.result_stats.text.contains("enemies destroyed"), "result statistics: %s" % game_menu.result_stats.text)
	_check(FileAccess.file_exists(game_menu.stats_path), "statistics JSON not written: '%s'" % game_menu.stats_path)
	var stats_json: Variant = JSON.parse_string(FileAccess.get_file_as_string(game_menu.stats_path))
	_check(stats_json is Dictionary and (stats_json["players"] as Array).size() == 2, "statistics JSON does not parse")
	var own_stats: Dictionary = (host.match_stats() as Array)[host.local_player()]
	_check(int(own_stats["kills"]) > 0, "no kills counted for the local player: %s" % own_stats)
	print("flow self-test: statistics ok (%s)" % game_menu.result_stats.text.replace("\n", "; "))
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
	_check(main.base_view.supply.flags.size() == 3, "supply points not drawn")
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
	# Through the build menu's picture tile (prompt 058).
	# The build button of the top bar (prompt 095) opens the menu above the
	# order grid's corner.
	var match_hud: Control = main.hud
	match_hud.top_bar._refresh()
	_check(match_hud.top_bar.build_button.is_visible_in_tree() and match_hud.top_bar.amounts.get("fuel", -1) == int(host.economy_state()["fuel"]),
		"the top bar shows no build button or not the stock of fuel (%s)" % [match_hud.top_bar.amounts])
	match_hud.top_bar.build_button.pressed.emit()
	await _frames(20)
	_check(hud._build_panel.visible and hud._build_panel.is_visible_in_tree(), "build menu not shown")
	_check(match_hud.covers(hud._build_panel.get_global_rect().get_center()), "the HUD does not count the build menu as a panel")
	_check_hud_layout(match_hud, "build menu open")
	hud.build_picker.open_tab(hud.build_picker.tab_of({"id": "base_barracks", "structure": true}))
	_check_tiles(hud.build_picker)
	_check(hud.build_picker.press_type(barracks) and hud.placing == barracks, "barracks tile does not start placing")
	var placed := false
	for r: float in [20.0, 30.0, 40.0, 50.0]:
		for k in 16:
			var angle := TAU * k / 16.0
			var p := hq + Vector3(cos(angle), 0, sin(angle)) * r
			if not placed and host.placement_check(barracks, p.x, p.z).is_empty():
				placed = hud.place_at(Vector3(p.x, 0, p.z))
	hud.cancel_placing()
	main.hud.command_bar.press("build")
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
	_check(hud._production_panel.visible and hud._production_panel.is_visible_in_tree(), "production bar not shown for the barracks")
	# Prompt 036: Repair (disabled while whole) and Sell for the building;
	# prompt 095: as tiles of the order grid, which shows for the building.
	var grid: Control = match_hud.command_bar
	var sell: Button = grid.buttons["sell"]
	_check(grid.visible and grid.page == "building" and sell.is_visible_in_tree() and not sell.disabled and str(sell.get_meta(&"label")) == "Sell",
		"no Sell tile for the barracks (page '%s')" % grid.page)
	_check(grid.buttons["repair"].disabled and grid.buttons["repair"].is_visible_in_tree(), "Repair enabled for an undamaged building")
	_check(not grid.buttons["move"].is_visible_in_tree(), "the order grid offers 'move' for a building")
	_check(match_hud.panels().has("info field"), "no info field for the barracks")
	_check_hud_layout(match_hud, "barracks selected")
	# The same in the smallest window the game supports.
	var window := get_tree().root.size
	get_tree().root.size = Vector2i(1280, 720)
	await _frames(20)
	_check_hud_layout(match_hud, "barracks selected, 1280 x 720")
	main.selection._set_selection([])
	main.hud.command_bar.press("build")
	await _frames(20)
	_check(hud._build_panel.is_visible_in_tree(), "build menu not shown at 1280 x 720")
	_check_hud_layout(match_hud, "build menu open, 1280 x 720")
	main.hud.command_bar.press("build")
	main.selection._set_selection([site["id"]])
	get_tree().root.size = window
	await _frames(20)
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
	# Through the production bar's picture tiles (prompt 058).
	var picker: Control = hud.production_picker
	_check_tiles(picker)
	var produced := -1
	for t: int in picker.tiles:
		if produced < 0 and picker.press_type(t):
			produced = t
	_check(produced >= 0, "no production tile enabled")
	await _frames(20)
	if produced >= 0:
		var count := picker.tiles[produced].get_node("Count") as Label
		_check(count.visible or own_units.call() > units_before, "queued order not shown on its tile")
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
	print("flow self-test: base building ok (barracks built and unit produced via picture tiles, computer opponent has %d buildings, tick %d)" % [ai_buildings, host.current_tick()])


## Every tile of the picker's open tab has its picture, name and costs.
func _check_tiles(picker: Control) -> void:
	_check(not picker.tiles.is_empty(), "picker '%s' shows no tiles" % picker.tab)
	for t: int in picker.tiles:
		var tile: Button = picker.tiles[t]
		_check(not tile.find_children("*", "TextureRect", false, false).is_empty(), "tile %s has no picture" % tile.name)
		_check(not (tile.get_node("Cost") as Label).text.is_empty(), "tile %s shows no costs" % tile.name)
		_check(not tile.tooltip_text.is_empty(), "tile %s has no tooltip" % tile.name)
	var key := InputEventKey.new()
	key.pressed = true
	key.alt_pressed = true
	key.physical_keycode = KEY_2
	_check(picker.hotkey_index(key) == 1, "Alt + 2 does not pick the second tile")


# --- Saved games and replays (prompt 038) ------------------------------


## Saves the running match from the pause menu; returns `path`, `name`,
## `tick` and `hash` of the saved state, or {} on failure.
func _save_test(main: Node3D) -> Dictionary:
	var game_menu: CanvasLayer = main.game_menu
	var host: SimHost = main.sim_host
	game_menu.open_pause()
	await _frames(2)
	_check(game_menu.save_button.is_visible_in_tree(), "no Save game button in the single-player pause menu")
	var tick := host.current_tick()
	var hash := host.state_hash()
	game_menu.save_button.pressed.emit()
	var path: String = game_menu.last_saved_path
	_check(not path.is_empty() and FileAccess.file_exists(path), "saved game not written")
	_check(game_menu.save_label.text.begins_with("Saved"), "save message: %s" % game_menu.save_label.text)
	_check(host.current_tick() == tick, "the game ran while saving")
	game_menu.close_pause()
	if path.is_empty():
		return {}
	print("flow self-test: saved at tick %d (%s, %d bytes)" % [tick, path, FileAccess.get_file_as_bytes(path).size()])
	return {"path": path, "name": path.get_file(), "tick": tick, "hash": hash}


## Loads the saved game from the menu; it continues at the saved state with
## its computer opponent.
func _load_test(menu: Control, saved: Dictionary) -> Node3D:
	menu.load_button.pressed.emit()
	await _frames(2)
	var screen: Control = menu.screens["load"]
	_check(screen.visible, "load screen not shown")
	var index: int = screen.find(saved["name"])
	_check(index >= 0, "saved game %s not listed" % saved["name"])
	if index < 0:
		return null
	_check(screen.list.get_item_text(index).contains("test_small"), "list line: %s" % screen.list.get_item_text(index))
	screen.list.select(index)
	screen.activate()
	var main: Node3D = await _wait_scene("Main")
	if main == null:
		return null
	var frames := 600
	while not main._loaded and frames > 0:
		frames -= 1
		await get_tree().process_frame
	var host: SimHost = main.sim_host
	var state: Dictionary = host.loaded_state()
	_check(state.get("tick", -1) == saved["tick"], "loaded at tick %s, saved at %d" % [state.get("tick"), saved["tick"]])
	_check(state.get("hash", "") == saved["hash"], "loaded state %s, saved %s" % [state.get("hash"), saved["hash"]])
	_check(host.ai_status()["difficulty"] == "normal", "computer opponent not restored: %s" % host.ai_status())
	_check(host.economy_state()["enabled"], "economy not restored")
	_check(main.game_menu.save_button != null, "pause menu missing")
	await _frames(60)
	_check(host.current_tick() > saved["tick"], "the loaded game does not run")
	print("flow self-test: saved game loaded at tick %d, running at tick %d" % [saved["tick"], host.current_tick()])
	return main


## Leaves the match through the pause menu (writes its replay).
func _quit_to_menu(main: Node3D) -> Control:
	main.game_menu.open_pause()
	await _frames(1)
	main.game_menu.leave_button.pressed.emit()
	var menu: Control = await _wait_scene("MainMenu")
	_check(not get_tree().paused, "tree still paused in the menu")
	return menu


func _replay_viewer_test(menu: Control, saved: Dictionary) -> void:
	# A replay of the format before prompt 038 is listed, but not playable.
	var old_path := ProjectSettings.globalize_path(GameFiles.REPLAY_DIR.path_join("zz-flow-self-test-old.msreplay"))
	var old := FileAccess.open(old_path, FileAccess.WRITE)
	old.store_buffer(PackedByteArray([0x4D, 0x53, 0x52, 0x50, 5, 0, 1, 2, 3, 4]))
	old.close()
	menu.replays_button.pressed.emit()
	await _frames(2)
	var screen: Control = menu.screens["replays"]
	_check(screen.visible, "replay screen not shown")
	var old_index: int = screen.find(old_path.get_file())
	_check(old_index >= 0 and not screen.entries[old_index]["info"]["playable"], "old replay not listed as unplayable")
	if old_index >= 0:
		_check(String(screen.entries[old_index]["info"]["error"]).contains("older game version"), "old replay: %s" % screen.entries[old_index]["info"])
	DirAccess.remove_absolute(old_path)
	# The replay written when the loaded game was left: the whole match.
	var index: int = screen.find(GameFiles.last_replay_path.get_file())
	_check(index >= 0, "replay of the loaded game not listed: %s" % GameFiles.last_replay_path)
	if index < 0:
		return
	var info: Dictionary = screen.entries[index]["info"]
	_check(info.get("playable", false), "replay of the loaded game not playable: %s" % info)
	if not info.get("playable", false):
		return
	_check(info["ticks"] > saved["tick"] + 10 and info["map"] == "test_small", "replay does not continue the loaded match: %s" % info)
	_check(GameFiles.players_text(info).to_lower().contains("computer, normal"), "players: %s" % GameFiles.players_text(info))
	# Only replays this process writes count: parallel self-tests in other
	# worktrees share the replay folder.
	GameFiles.last_replay_path = ""
	screen.list.select(index)
	screen.activate()
	var main := await _wait_replay()
	if main == null:
		return
	var host: SimHost = main.sim_host
	var bar: CanvasLayer = main.get_node("ReplayBar")
	_check(host.is_replay() and not host.can_save(), "replay mode not active")
	_check(not main.game_menu.surrender_button.visible, "surrender offered in a replay")
	var ticks: int = host.replay_status()["ticks"]
	_check(ticks == info["ticks"], "replay has %d ticks, list said %d" % [ticks, info["ticks"]])
	# Speed and pause.
	bar.set_speed(8.0)
	_check(is_equal_approx(host.speed, 8.0), "speed 8× not set")
	var tick := host.current_tick()
	await _frames(60)
	_check(host.current_tick() >= tick + 60, "8× played only %d ticks in 60 frames" % (host.current_tick() - tick))
	bar.toggle_pause()
	tick = host.current_tick()
	await _frames(20)
	_check(host.current_tick() == tick, "replay ran while paused")
	# Jump (paused, so the tick stays where it landed).
	bar.jump_to(700)
	main = await _wait_replay()
	if main == null:
		return
	host = main.sim_host
	_check(host.current_tick() == 700, "jump landed at tick %d" % host.current_tick())
	_check(Array(GameFiles.replay_player.keyframe_ticks()).has(600), "no snapshot at tick 600: %s" % GameFiles.replay_player.keyframe_ticks())
	# Perspectives.
	main.get_node("ReplayBar").set_view(1)
	main = await _wait_replay()
	if main == null:
		return
	host = main.sim_host
	_check(host.local_player() == 1 and not host.is_reveal_all(), "view of player 1 not shown")
	_check(host.current_tick() == 700, "perspective change moved to tick %d" % host.current_tick())
	main.get_node("ReplayBar").set_view(-1)
	main = await _wait_replay()
	if main == null:
		return
	host = main.sim_host
	_check(host.is_reveal_all() and host.replay_status()["view"] == -1, "everything-visible view not shown")
	# The end: the recorded end hash.
	main.get_node("ReplayBar").jump_to(ticks - 40)
	main = await _wait_replay()
	if main == null:
		return
	host = main.sim_host
	main.get_node("ReplayBar").set_speed(8.0)
	var frames := 600
	while not host.replay_status()["ended"] and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(host.replay_status()["ended"], "replay did not reach its end")
	_check(host.state_hash() == GameFiles.replay_player.end_hash(), "replay ended in %s, recorded %s" % [host.state_hash(), GameFiles.replay_player.end_hash()])
	print("flow self-test: replay viewer ok (%d ticks, snapshots at %s, end hash %s)" % [ticks, GameFiles.replay_player.keyframe_ticks(), host.state_hash()])
	main.get_node("ReplayBar").leave()
	menu = await _wait_scene("MainMenu")
	_check(GameFiles.last_replay_path.is_empty(), "watching a replay wrote a replay: %s" % GameFiles.last_replay_path)


## Waits for `main.tscn` with a replay, loaded.
func _wait_replay() -> Node3D:
	# The scene of the previous view goes first.
	await _frames(2)
	var main: Node3D = await _wait_scene("Main")
	if main == null:
		return null
	var frames := 600
	while not main._loaded and frames > 0:
		frames -= 1
		await get_tree().process_frame
	_check(main._loaded and main.sim_host.is_replay(), "replay did not load")
	return main


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


## No two parts of the match HUD overlap and all lie inside the window
## (prompt 095).
func _check_hud_layout(match_hud: Control, what: String) -> void:
	var parts: Dictionary = match_hud.panels()
	var screen := Rect2(Vector2.ZERO, match_hud.get_viewport_rect().size)
	var names := parts.keys()
	for i in names.size():
		var a: Control = parts[names[i]]
		_check(screen.grow(0.5).encloses(a.get_global_rect()), "%s sticks out of the window (%s): %s" % [names[i], what, a.get_global_rect()])
		for j in range(i + 1, names.size()):
			var b: Control = parts[names[j]]
			_check(not a.get_global_rect().intersects(b.get_global_rect()),
				"%s overlaps %s (%s): %s / %s" % [names[i], names[j], what, a.get_global_rect(), b.get_global_rect()])
