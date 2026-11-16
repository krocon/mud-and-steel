extends CanvasLayer
## In-match menu (prompt 027): pause menu on Esc and the result screen.
##
## Single player: Esc pauses the game (the scene tree, so the simulation and
## the camera stop) and offers Resume, Settings, Surrender and Quit to the
## main menu. Multiplayer: the lockstep match cannot stop for one player, so
## the menu only overlays the running game and offers Surrender and Leave.
##
## As soon as `SimHost.match_result()` is decided the result screen shows
## victory, defeat or draw, why, the match time and the units left per
## player. Added by `main.gd`; runs while the tree is paused.
##
## Prompt 038: "Save game" in the single-player pause menu writes a saved
## game (`game_files.gd`). While a replay plays there is no surrender, no
## saving and no result screen.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const SettingsPanel := preload("res://scenes/menu/settings_panel.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")
const Language := preload("res://scenes/menu/language.gd")
const StatsChart := preload("res://scenes/menu/stats_chart.gd")

var main: Node3D

var pause_panel: PanelContainer
var confirm_box: Control
var result_panel: PanelContainer
var result_title: Label
var result_reason: Label
var result_details: Label
## Prompt 045: losses, kills and points per player, resource curves.
var result_stats: Label
var result_chart: Control
## Absolute path of the statistics JSON of this match ("" if not written).
var stats_path := ""
var settings_panel: PanelContainer
var resume_button: Button
var surrender_button: Button
var confirm_surrender_button: Button
var leave_button: Button
var menu_button: Button
var watch_button: Button
var save_button: Button
var save_label: Label
## Absolute path of the last saved game of this match ("" if none).
var last_saved_path := ""

var _root: Control
var _shade: ColorRect
var _result_shown := false
## Faction id per player, from the units at the start.
var _factions: Dictionary[int, String] = {}


func _ready() -> void:
	layer = 50
	process_mode = Node.PROCESS_MODE_ALWAYS
	_root = Control.new()
	_root.theme = MenuTheme.get_theme()
	_root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_root)
	_shade = ColorRect.new()
	_shade.color = Color(0, 0, 0, 0.45)
	_shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_shade.visible = false
	_root.add_child(_shade)
	_build_pause()
	_build_result()
	_build_settings()
	var owners: PackedInt32Array = main.sim_host.unit_owners()
	var types: PackedStringArray = main.sim_host.unit_type_ids()
	for i in owners.size():
		if not _factions.has(owners[i]):
			_factions[owners[i]] = types[i].get_slice("_", 0)


func is_networked() -> bool:
	return main.sim_host.is_networked()


func is_paused() -> bool:
	return pause_panel.visible


func open_pause() -> void:
	if _result_shown and result_panel.visible:
		return
	save_button.visible = main.sim_host.can_save()
	save_label.visible = false
	pause_panel.visible = true
	confirm_box.visible = false
	_shade.visible = true
	if not is_networked():
		get_tree().paused = true
	resume_button.grab_focus()


func close_pause() -> void:
	pause_panel.visible = false
	settings_panel.visible = false
	_shade.visible = result_panel.visible
	get_tree().paused = false


## Writes the match as a saved game (prompt 038).
func save_game() -> void:
	var settings := {"biome": main.atmosphere.preset.id if main.atmosphere != null else ""}
	last_saved_path = GameFiles.save_game(main.sim_host, settings)
	save_label.visible = true
	if last_saved_path.is_empty():
		save_label.text = "MENU_SAVE_FAILED"
	else:
		save_label.text = tr("MENU_SAVED_AT") % GameFiles.now_text()


func surrender() -> void:
	main.sim_host.surrender()
	close_pause()


func back_to_menu() -> void:
	get_tree().paused = false
	get_tree().change_scene_to_file(MatchSetup.MENU_SCENE)


func _unhandled_input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo or key.keycode != KEY_ESCAPE:
		return
	get_viewport().set_input_as_handled()
	if settings_panel.visible:
		settings_panel.cancel()
	elif pause_panel.visible:
		close_pause()
	elif not result_panel.visible:
		open_pause()


func _process(_delta: float) -> void:
	if _result_shown or main.sim_host.is_replay():
		return
	var result: Dictionary = main.sim_host.match_result()
	if result["decided"]:
		_show_result(result)


func _build_pause() -> void:
	pause_panel = _panel(360)
	var box: VBoxContainer = pause_panel.get_child(0)
	box.add_child(MenuTheme.title("MENU_MULTIPLAYER_MENU" if is_networked() else "MENU_PAUSED", MenuTheme.HEADING_SIZE))
	if is_networked():
		box.add_child(MenuTheme.hint("MENU_NO_PAUSE_HINT"))
	resume_button = _button(box, "MENU_RESUME", close_pause)
	_button(box, "COMMON_SETTINGS", func() -> void:
		pause_panel.visible = false
		settings_panel.visible = true)
	save_button = _button(box, "MENU_SAVE_GAME", save_game)
	save_button.visible = main.sim_host.can_save()
	save_label = MenuTheme.hint("")
	save_label.visible = false
	box.add_child(save_label)
	surrender_button = _button(box, "MENU_SURRENDER", func() -> void:
		confirm_box.visible = not confirm_box.visible)
	surrender_button.visible = not main.sim_host.is_replay()
	confirm_box = VBoxContainer.new()
	confirm_box.visible = false
	confirm_box.add_child(MenuTheme.hint("MENU_SURRENDER_CONFIRM"))
	confirm_surrender_button = _button(confirm_box, "MENU_SURRENDER_YES", surrender)
	box.add_child(confirm_box)
	leave_button = _button(box, "MENU_LEAVE_MATCH" if is_networked() else "MENU_QUIT_TO_MENU", back_to_menu)


func _build_result() -> void:
	result_panel = _panel(460)
	var box: VBoxContainer = result_panel.get_child(0)
	result_title = MenuTheme.title("")
	box.add_child(result_title)
	result_reason = Label.new()
	result_reason.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	result_reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(result_reason)
	result_details = MenuTheme.hint("")
	box.add_child(result_details)
	result_stats = MenuTheme.hint("")
	box.add_child(result_stats)
	result_chart = StatsChart.new()
	box.add_child(result_chart)
	box.add_child(MenuTheme.hint("RESULT_STATS_LEGEND"))
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	box.add_child(buttons)
	watch_button = _button(buttons, "RESULT_KEEP_WATCHING", func() -> void:
		result_panel.visible = false
		_shade.visible = false)
	menu_button = _button(buttons, "RESULT_BACK_TO_MENU", back_to_menu)


func _build_settings() -> void:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.add_child(center)
	settings_panel = SettingsPanel.new()
	settings_panel.visible = false
	settings_panel.changed.connect(_apply_settings)
	settings_panel.closed.connect(func() -> void:
		settings_panel.visible = false
		if not result_panel.visible:
			pause_panel.visible = true)
	center.add_child(settings_panel)


func _apply_settings() -> void:
	Settings.apply_camera(main.camera_rig)
	main.atmosphere.set_quality(Settings.quality())


func _show_result(result: Dictionary) -> void:
	_result_shown = true
	close_pause()
	var me: int = main.sim_host.local_player()
	var winner: int = result["winner"]
	var players: Array = result["players"]
	var own: Dictionary = {}
	for player: Dictionary in players:
		if player["id"] == me:
			own = player
	# Teams (prompt 037): the whole team of the winner wins.
	var won: bool = winner == me or (winner >= 0 and main.sim_host.local_team_won())
	if won:
		result_title.text = tr("COMMON_VICTORY")
		result_title.add_theme_color_override("font_color", MenuTheme.VICTORY)
	elif winner < 0:
		result_title.text = tr("RESULT_DRAW")
	else:
		result_title.text = tr("COMMON_DEFEAT")
		result_title.add_theme_color_override("font_color", MenuTheme.DEFEAT)
	result_reason.text = _reason(winner, me, own, players, won)
	var seconds: int = result["tick"] / maxi(main.sim_host.tick_rate(), 1)
	var lines: PackedStringArray = [tr("RESULT_MATCH_TIME") % [seconds / 60, seconds % 60]]
	for player: Dictionary in players:
		var id: int = player["id"]
		var relation: int = main.sim_host.relation(id)
		var faction: String = _factions.get(id, "")
		var line := tr("RESULT_PLAYER_UNITS").format({
			"player": tr(MatchSetup.FACTIONS[faction]) if MatchSetup.FACTIONS.has(faction) else tr("COMMON_PLAYER_N") % id,
			"relation": (" " + tr("MATCH_SUFFIX_YOU")) if id == me else ((" " + tr("RESULT_SUFFIX_ALLY")) if relation == 1 else ""),
			"units": player["units"],
		})
		if player["defeated"]:
			line += " – " + Language.code_text("RESULT_DEFEAT_", player["reason"])
		lines.append(line)
	result_details.text = "\n".join(lines)
	_show_stats(me)
	result_panel.visible = true
	_shade.visible = true
	menu_button.grab_focus()
	print("Match decided at tick %d: %s (%s)" % [result["tick"], result_title.text, result_reason.text])


## Prompt 045: statistics of every player, the best unit types of the
## local player and its resource curves; writes them as JSON as well.
func _show_stats(me: int) -> void:
	var lines: PackedStringArray = []
	for player: Dictionary in main.sim_host.match_stats():
		var id: int = player["id"]
		var faction: String = _factions.get(id, player["faction"])
		lines.append(tr("RESULT_STATS_PLAYER").format({
			"player": tr(MatchSetup.FACTIONS[faction]) if MatchSetup.FACTIONS.has(faction) else tr("COMMON_PLAYER_N") % id,
			"lost": player["lost"],
			"kills": player["kills"],
			"points": player["points_captured"],
		}))
		if id != me:
			continue
		var best: PackedStringArray = []
		for row: Dictionary in player["types"]:
			if row["kills"] > 0 and best.size() < 3:
				best.append("%s %d" % [row["name"], row["kills"]])
		if not best.is_empty():
			lines.append(tr("RESULT_STATS_BEST") % ", ".join(best))
		result_chart.set_samples(player["samples"])
	stats_path = GameFiles.save_stats(main.sim_host)
	if not stats_path.is_empty():
		lines.append(tr("RESULT_STATS_SAVED") % stats_path.get_file())
	result_stats.text = "\n".join(lines)


func _reason(winner: int, me: int, own: Dictionary, players: Array, won: bool) -> String:
	if winner < 0:
		return tr("RESULT_REASON_DRAW")
	# Prompt 039: the mission script decided.
	if own.get("reason", "") == "mission" or (won and players.any(func(p: Dictionary) -> bool: return p["reason"] == "mission")):
		return tr("RESULT_REASON_MISSION_WON") if won else tr("RESULT_REASON_MISSION_LOST")
	if not won:
		if own.get("defeated", false):
			match own.get("reason", ""):
				"surrendered":
					return tr("RESULT_REASON_YOU_SURRENDERED")
				"base_lost":
					# Decision 6 of prompt 045: no headquarters and no production left.
					return tr("RESULT_REASON_YOU_BASE_LOST")
			return tr("RESULT_REASON_YOU_ELIMINATED")
		return tr("RESULT_REASON_TEAM_DEFEATED")
	for player: Dictionary in players:
		if main.sim_host.relation(player["id"]) == 2 and player["reason"] == "surrendered":
			return tr("RESULT_REASON_ENEMY_SURRENDERED")
	for player: Dictionary in players:
		if main.sim_host.relation(player["id"]) == 2 and player["reason"] == "base_lost":
			return tr("RESULT_REASON_ENEMY_BASE_LOST")
	return tr("RESULT_REASON_ENEMY_ELIMINATED")


func _panel(width: float) -> PanelContainer:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.add_child(center)
	var panel := PanelContainer.new()
	panel.visible = false
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(width, 0)
	panel.add_child(box)
	return panel


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(0, 40)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
