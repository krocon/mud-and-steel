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
## saved games and replays (prompt 038, `files_screen.gd`).
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
const Music := preload("res://scenes/audio/music.gd")
const LAN_SCENE := "res://scenes/lan/lan_lobby.tscn"
const ONLINE_SCENE := "res://scenes/online/steam_lobby.tscn"
const EDITOR_SCENE := "res://scenes/editor/map_editor.tscn"
## Sound gallery (prompt 029): `-- --sound-gallery` / `--sound-gallery-once`.
const SOUND_GALLERY_SCENE := "res://scenes/tools/sound_gallery.tscn"
const EditorSelfTest := preload("res://scenes/editor/editor_self_test.gd")
const FilesScreen := preload("res://scenes/menu/files_screen.gd")
const DIRECT_FLAGS: Array[String] = [
	"--skip-menu", "--demo", "--camera-shots", "--path-debug",
	"--terrain-bench", "--terrain-shots", "--objects-bench", "--objects-shots",
]
const DIRECT_PREFIXES: Array[String] = ["--map=", "--biome=", "--effects-"]
const BIOMES: Dictionary[String, String] = {
	"": "As the map says", "forest": "Forest (summer)", "desert": "Desert", "winter": "Winter",
}

## Screens, by name: "main", "skirmish", "settings", "credits", "load", "replays".
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
var map_option: OptionButton
var biome_option: OptionButton
var faction_option: OptionButton
var enemy_option: OptionButton
var opponent_option: OptionButton
var mode_option: OptionButton
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
	# Map editor self-test (prompt 028, `just editor-selftest`).
	if "--editor-self-test" in OS.get_cmdline_user_args() and not get_tree().root.has_node("EditorSelfTest"):
		var editor_test := EditorSelfTest.new()
		editor_test.name = "EditorSelfTest"
		get_tree().root.add_child.call_deferred(editor_test)


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
		"factions": PackedStringArray([own, enemy]),
		"ai": MatchSetup.OPPONENTS.keys()[opponent_option.selected],
		"mode": MatchSetup.MODES.keys()[mode_option.selected],
	}
	print("Menu: starting %s" % MatchSetup.pending)
	get_tree().change_scene_to_file(MatchSetup.MAIN_SCENE)


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
	box.add_child(MenuTheme.title("MUD AND STEEL"))
	var subtitle := MenuTheme.hint("Real-time tactics, 1939–1945")
	subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(subtitle)
	box.add_child(_spacer(24))
	single_player_button = _button(box, "Single player", func() -> void: show_screen("skirmish"))
	load_button = _button(box, "Load game", func() -> void: show_screen("load"))
	replays_button = _button(box, "Replays", func() -> void: show_screen("replays"))
	lan_button = _button(box, "LAN game", open_lan)
	online_button = _button(box, "Online (Steam)", open_online)
	editor_button = _button(box, "Map editor", open_editor)
	settings_button = _button(box, "Settings", func() -> void: show_screen("settings"))
	credits_button = _button(box, "Credits", func() -> void: show_screen("credits"))
	quit_button = _button(box, "Quit", func() -> void: get_tree().quit())
	single_player_button.grab_focus.call_deferred()
	# Prompt 044: a crash report of the last session, kept locally.
	if not CrashGuard.last_report.is_empty():
		var crash := MenuTheme.hint("The game ended unexpectedly last time. A report was saved (not sent anywhere):\n%s" % CrashGuard.last_report)
		crash.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		box.add_child(crash)
	return box.get_meta(&"screen")


func _skirmish_screen() -> Control:
	var box := _centered_column(560, true)
	box.add_child(MenuTheme.title("Single player", MenuTheme.HEADING_SIZE))

	map_option = OptionButton.new()
	_maps = MatchSetup.available_maps()
	for map_name in _maps:
		map_option.add_item(map_name)
	map_option.select(MatchSetup.default_map_index(_maps))
	map_option.item_selected.connect(func(_index: int) -> void: _update_map_info())
	box.add_child(_row("Map", map_option))
	_map_info = MenuTheme.hint("")
	box.add_child(_map_info)

	biome_option = OptionButton.new()
	for id in BIOMES:
		biome_option.add_item(BIOMES[id])
	box.add_child(_row("Season", biome_option))

	faction_option = OptionButton.new()
	enemy_option = OptionButton.new()
	for id in MatchSetup.FACTIONS:
		faction_option.add_item(MatchSetup.FACTIONS[id])
		enemy_option.add_item(MatchSetup.FACTIONS[id])
	faction_option.select(0)
	enemy_option.select(1)
	box.add_child(_row("Your side", faction_option))
	box.add_child(_row("Enemy", enemy_option))
	opponent_option = OptionButton.new()
	for id in MatchSetup.OPPONENTS:
		opponent_option.add_item(MatchSetup.OPPONENTS[id])
	opponent_option.select(MatchSetup.OPPONENTS.keys().find(MatchSetup.DEFAULT_OPPONENT))
	box.add_child(_row("Opponent", opponent_option))
	mode_option = OptionButton.new()
	for id in MatchSetup.MODES:
		mode_option.add_item(MatchSetup.MODES[id])
	mode_option.select(MatchSetup.MODES.keys().find(MatchSetup.DEFAULT_MODE))
	box.add_child(_row("Mode", mode_option))
	box.add_child(MenuTheme.hint("Base building: build a base, hold supply points, produce an army. Battle: eight tanks per side. Win when the enemy surrenders or has no units left. The computer opponent sees only what its units see."))

	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	_button(buttons, "Back", func() -> void: show_screen("main"))
	start_button = _button(buttons, "Start", start_skirmish)
	_update_map_info()
	return box.get_meta(&"screen")


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
		info = "Built-in test map. " + info
	if MatchSetup.is_large(map_name):
		info += ". Large map: loading takes a few seconds."
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
	label.custom_minimum_size = Vector2(150, 0)
	row.add_child(label)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(control)
	return row


func _spacer(height: float) -> Control:
	var spacer := Control.new()
	spacer.custom_minimum_size = Vector2(0, height)
	return spacer
