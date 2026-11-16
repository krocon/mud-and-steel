extends Control
## Main menu (prompt 027), the start scene of the game.
##
## Screens: main (single player, LAN, settings, quit), single-player setup
## (map, biome, own and enemy faction) and settings; "Map editor" opens
## `editor/map_editor.tscn` (prompt 028). A started match is
## handed to `main.tscn` through `match_setup.gd`; LAN opens
## `lan/lan_lobby.tscn`, "Online (Steam)" `online/steam_lobby.tscn`
## (prompt 026), also right away when Steam started the game with
## `+connect_lobby <code>` to join a friend. "Load game" and "Replays" list
## saved games and replays (prompt 038, `files_screen.gd`). "Units" opens
## the unit reference (prompt 066, `units_screen.gd`).
##
## Direct starts skip the menu: `-- --skip-menu` and the arguments that used
## to start the match right away (`--map=…`, `--biome=…`, `--demo`,
## `--camera-shots`, `--path-debug`, the benchmarks) go straight to
## `main.tscn`. `-- --self-test` plays the whole flow
## (`menu/flow_self_test.gd`).

const Settings := preload("res://scenes/menu/settings.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const SettingsPanel := preload("res://scenes/menu/settings_panel.gd")
const CreditsPanel := preload("res://scenes/menu/credits_panel.gd")
const FlowSelfTest := preload("res://scenes/menu/flow_self_test.gd")
const SlotList := preload("res://scenes/menu/slot_list.gd")
const Music := preload("res://scenes/audio/music.gd")
const LAN_SCENE := "res://scenes/lan/lan_lobby.tscn"
const ONLINE_SCENE := "res://scenes/online/steam_lobby.tscn"
const EDITOR_SCENE := "res://scenes/editor/map_editor.tscn"
## Sound gallery (prompt 029): `-- --sound-gallery` / `--sound-gallery-once`.
const SOUND_GALLERY_SCENE := "res://scenes/tools/sound_gallery.tscn"
const EditorSelfTest := preload("res://scenes/editor/editor_self_test.gd")
const FilesScreen := preload("res://scenes/menu/files_screen.gd")
## Tutorial and missions (prompt 039).
const MissionsScreen := preload("res://scenes/menu/missions_screen.gd")
const Missions := preload("res://scenes/menu/missions.gd")
## Unit reference (prompt 066).
const UnitsScreen := preload("res://scenes/menu/units_screen.gd")
const DIRECT_FLAGS: Array[String] = [
	"--skip-menu", "--demo", "--camera-shots", "--path-debug",
	"--terrain-bench", "--terrain-shots", "--objects-bench", "--objects-shots",
]
const DIRECT_PREFIXES: Array[String] = ["--map=", "--biome=", "--effects-"]
## Biome ids with the translation keys of their names.
const BIOMES: Dictionary[String, String] = {
	"": "MENU_BIOME_MAP", "forest": "MENU_BIOME_FOREST", "desert": "COMMON_BIOME_DESERT",
	"winter": "COMMON_BIOME_WINTER",
}

## Screens, by name: "main", "skirmish", "settings", "credits", "load",
## "replays", "missions", "units".
var screens: Dictionary[String, Control] = {}
var single_player_button: Button
var lan_button: Button
var online_button: Button
var editor_button: Button
var settings_button: Button
var credits_button: Button
var quit_button: Button
## Saved games and replays (prompt 038).
var load_button: Button
var replays_button: Button
## Tutorial and missions (prompt 039).
var tutorial_button: Button
var missions_button: Button
var missions_screen: MissionsScreen
## Unit reference (prompt 066).
var units_button: Button
var units_screen: UnitsScreen
var map_option: OptionButton
var biome_option: OptionButton
## Weather and time of day (prompt 040).
var weather_option: OptionButton
var daytime_option: OptionButton
var faction_option: OptionButton
var enemy_option: OptionButton
var opponent_option: OptionButton
var mode_option: OptionButton
## Teams (prompt 037): 1 vs 1 (the options above), 2 vs 2 or 3 vs 3 with
## computer allies and enemies in `slot_list`.
var players_option: OptionButton
var shared_vision_box: CheckBox
var slot_list: SlotList
var start_button: Button
var settings_panel: PanelContainer
var credits_panel: PanelContainer

var _maps: PackedStringArray = []
## `+connect_lobby` is followed once per process, not on every menu visit.
static var _launch_handled := false
var _map_info: Label


func _ready() -> void:
	if _is_direct_start():
		get_tree().change_scene_to_file.call_deferred(MatchSetup.MAIN_SCENE)
		return
	if Array(OS.get_cmdline_user_args()).any(func(arg: String) -> bool: return arg.begins_with("--sound-gallery")):
		get_tree().change_scene_to_file.call_deferred(SOUND_GALLERY_SCENE)
		return
	Settings.ensure_loaded()
	Music.play("menu")
	theme = MenuTheme.get_theme()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(MenuTheme.background())
	screens["main"] = _main_screen()
	screens["skirmish"] = _skirmish_screen()
	screens["settings"] = _settings_screen()
	screens["credits"] = _credits_screen()
	for kind in ["load", "replays"]:
		var files := FilesScreen.new()
		files.kind = kind
		files.visible = false
		files.back_pressed.connect(func() -> void: show_screen("main"))
		add_child(files)
		screens[kind] = files
	missions_screen = MissionsScreen.new()
	missions_screen.visible = false
	missions_screen.back_pressed.connect(func() -> void: show_screen("main"))
	add_child(missions_screen)
	screens["missions"] = missions_screen
	units_screen = UnitsScreen.new()
	units_screen.visible = false
	units_screen.back_pressed.connect(func() -> void: show_screen("main"))
	add_child(units_screen)
	screens["units"] = units_screen
	show_screen("main")
	# Steam starts the game like this when the player joins a friend.
	if SteamLobby.launch_lobby_code() != "" and not _launch_handled:
		_launch_handled = true
		open_online.call_deferred()
		return
	if "--self-test" in OS.get_cmdline_user_args() and MatchSetup.flow_test == null:
		var test := FlowSelfTest.new()
		test.name = "FlowSelfTest"
		MatchSetup.flow_test = test
		# On the root, so it survives the scene changes it drives.
		get_tree().root.add_child.call_deferred(test)
	# Screenshots of menus and HUD (prompt 043, `just l10n-shots`).
	if Array(OS.get_cmdline_user_args()).any(func(a: String) -> bool: return a.begins_with("--l10n-shots=")) \
			and not get_tree().root.has_node("L10nShots"):
		var shots: Node = load("res://scenes/tools/l10n_shots.gd").new()
		shots.name = "L10nShots"
		get_tree().root.add_child.call_deferred(shots)
	# Screenshots of the unit values (prompt 066, `just unit-card-shots`).
	if Array(OS.get_cmdline_user_args()).any(func(a: String) -> bool: return a.begins_with("--unit-card-shots=")) \
			and not get_tree().root.has_node("UnitCardShots"):
		var card_shots: Node = load("res://scenes/tools/unit_card_shots.gd").new()
		card_shots.name = "UnitCardShots"
		get_tree().root.add_child.call_deferred(card_shots)
	# Map editor self-test (prompt 028, `just editor-selftest`).
	if "--editor-self-test" in OS.get_cmdline_user_args() and not get_tree().root.has_node("EditorSelfTest"):
		var editor_test := EditorSelfTest.new()
		editor_test.name = "EditorSelfTest"
		get_tree().root.add_child.call_deferred(editor_test)


## Texts built at run time follow a language change (prompt 043).
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and _map_info != null and slot_list != null:
		_update_map_info()
		_update_team_sizes()


func show_screen(screen_name: String) -> void:
	for key in screens:
		screens[key].visible = key == screen_name


## Starts the match chosen on the setup screen.
func start_skirmish() -> void:
	var own: String = MatchSetup.FACTIONS.keys()[faction_option.selected]
	var enemy: String = MatchSetup.FACTIONS.keys()[enemy_option.selected]
	MatchSetup.pending = {
		"map": _maps[map_option.selected],
		"biome": BIOMES.keys()[biome_option.selected],
		"weather": MatchSetup.WEATHERS.keys()[weather_option.selected],
		"daytime": MatchSetup.DAYTIMES.keys()[daytime_option.selected],
		"factions": PackedStringArray([own, enemy]),
		"ai": MatchSetup.OPPONENTS.keys()[opponent_option.selected],
		"mode": MatchSetup.MODES.keys()[mode_option.selected],
	}
	if team_size() > 1:
		MatchSetup.pending["slots"] = slot_list.slots()
		MatchSetup.pending["shared_vision"] = shared_vision_box.button_pressed
	print("Menu: starting %s" % MatchSetup.pending)
	get_tree().change_scene_to_file(MatchSetup.MAIN_SCENE)


## Starts the tutorial (prompt 039).
func start_tutorial() -> void:
	var tutorial := Missions.tutorial()
	if tutorial.is_empty():
		push_error("tutorial mission not found")
		return
	Missions.start(get_tree(), tutorial)


func open_lan() -> void:
	get_tree().change_scene_to_file(LAN_SCENE)


## Opens the online lobby over Steam (prompt 026).
func open_online() -> void:
	get_tree().change_scene_to_file(ONLINE_SCENE)


## Opens the map editor (prompt 028).
func open_editor() -> void:
	get_tree().change_scene_to_file(EDITOR_SCENE)


func _is_direct_start() -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg in DIRECT_FLAGS:
			return true
		for prefix in DIRECT_PREFIXES:
			if arg.begins_with(prefix):
				return true
	return false


func _main_screen() -> Control:
	var box := _centered_column(360)
	box.add_child(MenuTheme.title("MUD AND STEEL"))  # l10n-ignore (game name)
	var subtitle := MenuTheme.hint("MENU_SUBTITLE")
	subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(subtitle)
	box.add_child(_spacer(4))
	tutorial_button = _button(box, "MENU_TUTORIAL", start_tutorial)
	missions_button = _button(box, "MENU_MISSIONS", func() -> void: show_screen("missions"))
	single_player_button = _button(box, "MENU_SINGLE_PLAYER", func() -> void: show_screen("skirmish"))
	load_button = _button(box, "MENU_LOAD_GAME", func() -> void: show_screen("load"))
	replays_button = _button(box, "MENU_REPLAYS", func() -> void: show_screen("replays"))
	lan_button = _button(box, "MENU_LAN_GAME", open_lan)
	online_button = _button(box, "MENU_ONLINE", open_online)
	editor_button = _button(box, "MENU_MAP_EDITOR", open_editor)
	units_button = _button(box, "MENU_UNITS", func() -> void: show_screen("units"))
	settings_button = _button(box, "COMMON_SETTINGS", func() -> void: show_screen("settings"))
	credits_button = _button(box, "MENU_CREDITS", func() -> void: show_screen("credits"))
	quit_button = _button(box, "COMMON_QUIT", func() -> void: get_tree().quit())
	# Twelve entries fit a window 720 pixels high (prompt 066).
	box.add_theme_constant_override(&"separation", 8)
	for button in box.get_children():
		if button is Button:
			(button as Button).custom_minimum_size.y = 40
	single_player_button.grab_focus.call_deferred()
	# Prompt 044: a crash report of the last session, kept locally.
	if not CrashGuard.last_report.is_empty():
		var crash := MenuTheme.hint(tr("CRASH_LAST_SESSION") % CrashGuard.last_report)
		crash.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		box.add_child(crash)
	return box.get_meta(&"screen")


func _skirmish_screen() -> Control:
	var box := _centered_column(560, true)
	box.add_child(MenuTheme.title("MENU_SINGLE_PLAYER", MenuTheme.HEADING_SIZE))

	map_option = OptionButton.new()
	_maps = MatchSetup.available_maps()
	for map_name in _maps:
		map_option.add_item(map_name)
	map_option.select(MatchSetup.default_map_index(_maps))
	map_option.item_selected.connect(func(_index: int) -> void:
		_update_map_info()
		_update_team_sizes())
	box.add_child(_row("MATCH_MAP", map_option))
	_map_info = MenuTheme.hint("")
	box.add_child(_map_info)

	biome_option = OptionButton.new()
	for id in BIOMES:
		biome_option.add_item(BIOMES[id])
	box.add_child(_row("MATCH_SEASON", biome_option))
	weather_option = OptionButton.new()
	for id in MatchSetup.WEATHERS:
		weather_option.add_item(MatchSetup.WEATHERS[id])
	box.add_child(_row("MATCH_WEATHER", weather_option))
	daytime_option = OptionButton.new()
	for id in MatchSetup.DAYTIMES:
		daytime_option.add_item(MatchSetup.DAYTIMES[id])
	box.add_child(_row("MATCH_DAYTIME", daytime_option))

	faction_option = OptionButton.new()
	enemy_option = OptionButton.new()
	for id in MatchSetup.FACTIONS:
		faction_option.add_item(MatchSetup.FACTIONS[id])
		enemy_option.add_item(MatchSetup.FACTIONS[id])
	faction_option.select(0)
	enemy_option.select(1)
	box.add_child(_row("MATCH_YOUR_SIDE", faction_option))
	box.add_child(_row("MATCH_ENEMY", enemy_option))
	opponent_option = OptionButton.new()
	for id in MatchSetup.OPPONENTS:
		opponent_option.add_item(MatchSetup.OPPONENTS[id])
	opponent_option.select(MatchSetup.OPPONENTS.keys().find(MatchSetup.DEFAULT_OPPONENT))
	box.add_child(_row("MATCH_OPPONENT", opponent_option))
	players_option = OptionButton.new()
	box.add_child(_row("MATCH_PLAYERS", players_option))
	shared_vision_box = CheckBox.new()
	shared_vision_box.text = "MATCH_SHARED_VISION"
	shared_vision_box.button_pressed = true
	box.add_child(shared_vision_box)
	slot_list = SlotList.new()
	slot_list.side_changed.connect(_on_slot_side)
	slot_list.kind_changed.connect(_on_slot_kind)
	box.add_child(slot_list)
	for option in [faction_option, enemy_option, opponent_option, players_option]:
		(option as OptionButton).item_selected.connect(func(_index: int) -> void: _build_slots())
	mode_option = OptionButton.new()
	for id in MatchSetup.MODES:
		mode_option.add_item(MatchSetup.MODES[id])
	mode_option.select(MatchSetup.MODES.keys().find(MatchSetup.DEFAULT_MODE))
	box.add_child(_row("MATCH_MODE", mode_option))
	box.add_child(MenuTheme.hint("MATCH_MODE_HINT"))

	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	_button(buttons, "COMMON_BACK", func() -> void: show_screen("main"))
	start_button = _button(buttons, "COMMON_START", start_skirmish)
	_update_map_info()
	_update_team_sizes()
	return box.get_meta(&"screen")


## Players per team chosen (1 = one against one).
func team_size() -> int:
	return players_option.selected + 1 if players_option.selected >= 0 else 1


## The sizes the map's start positions allow (prompt 037).
func _update_team_sizes() -> void:
	var starts := MatchSetup.map_starts(_maps[map_option.selected]).size()
	var keep := players_option.selected
	players_option.clear()
	for n in range(1, maxi(starts / 2, 1) + 1):
		players_option.add_item(tr("MATCH_TEAM_SIZE") % [n, n])
	players_option.select(clampi(keep, 0, players_option.item_count - 1))
	_build_slots()


## Slots for the chosen size: you in slot 0, computers of the opponent's
## level everywhere else, teams as the map suggests, your faction for
## your team and the enemy's for the other.
func _build_slots() -> void:
	var size := team_size()
	slot_list.visible = size > 1
	shared_vision_box.visible = size > 1
	if size <= 1:
		return
	var own: String = MatchSetup.FACTIONS.keys()[faction_option.selected]
	var enemy: String = MatchSetup.FACTIONS.keys()[enemy_option.selected]
	var level: String = MatchSetup.OPPONENTS.keys()[opponent_option.selected]
	var starts := MatchSetup.map_starts(_maps[map_option.selected])
	var own_team: int = starts[0]["team"]
	var slots: Array = []
	for s in starts.slice(0, size * 2):
		var i: int = s["player"]
		var team: int = s["team"]
		slots.append({
			"index": i, "kind": "human" if i == 0 else "computer", "level": level,
			"team": team, "faction": own if team == own_team else enemy,
			"name": tr("MATCH_YOU") if i == 0 else "", "local": i == 0, "ready": false,
		})
	slot_list.show_slots(slots)


func _on_slot_kind(index: int, kind: String, level: String) -> void:
	var slots := slot_list.slots()
	slots[index]["kind"] = kind
	slots[index]["level"] = level
	slot_list.show_slots(slots)


func _on_slot_side(index: int, team: int, faction: String) -> void:
	var slots := slot_list.slots()
	slots[index]["team"] = team
	slots[index]["faction"] = faction
	slot_list.show_slots(slots)


func _settings_screen() -> Control:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	settings_panel = SettingsPanel.new()
	settings_panel.closed.connect(func() -> void: show_screen("main"))
	center.add_child(settings_panel)
	return center


## Credits and licenses (prompt 044).
func _credits_screen() -> Control:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	credits_panel = CreditsPanel.new()
	credits_panel.closed.connect(func() -> void: show_screen("main"))
	center.add_child(credits_panel)
	return center


func _update_map_info() -> void:
	var map_name := _maps[map_option.selected]
	var info := MatchSetup.describe_map(map_name)
	if map_name == MatchSetup.BUILTIN_MAP:
		info = tr("MATCH_BUILTIN_MAP") + " " + info
	if MatchSetup.is_large(map_name):
		info += ". " + tr("MATCH_LARGE_MAP")
	_map_info.text = info


## A centered column; the screen root is stored as meta "screen".
func _centered_column(width: float, panel := false) -> VBoxContainer:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(width, 0)
	if panel:
		var frame := PanelContainer.new()
		center.add_child(frame)
		frame.add_child(box)
	else:
		center.add_child(box)
	box.set_meta(&"screen", center)
	return box


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button


func _row(text: String, control: Control) -> Control:
	var row := HBoxContainer.new()
	var label := Label.new()
	label.text = text
	label.custom_minimum_size = Vector2(170, 0)
	row.add_child(label)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(control)
	return row


func _spacer(height: float) -> Control:
	var spacer := Control.new()
	spacer.custom_minimum_size = Vector2(0, height)
	return spacer
