extends Node
## Screenshots of menus and HUD in one language (prompt 043), to check by eye
## that no text overflows: `just l10n-shots` (German into `build/l10n/`).
## Started by `main_menu.gd` for `--l10n-shots=<dir>`; needs a window.
##
## Shots: every main menu screen, every settings tab, a base-building match
## on test_small with a selected tank group, the build menu, a selected
## infantry squad, the pause menu, the result after surrendering. Extra
## arguments: `--language=<code>` (the recipe passes `de`),
## `--ui-scale=<f>` for a second set at another interface scale.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")

var dir := ""
var shots: PackedStringArray = []


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--l10n-shots="):
			dir = arg.trim_prefix("--l10n-shots=")
		elif arg.begins_with("--ui-scale="):
			Settings.set_value("ui_scale", float(arg.trim_prefix("--ui-scale=")))
			Settings.save_file()
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var menu: Control = await _wait_scene("MainMenu")
	await _frames(10)
	for screen: String in menu.screens:
		menu.show_screen(screen)
		await _frames(4)
		await _shot("menu_" + screen)
	menu.show_screen("main")
	menu.settings_button.pressed.emit()
	await _frames(3)
	var tabs: TabContainer = menu.settings_panel.find_children("", "TabContainer", true, false)[0]
	for i in tabs.get_tab_count():
		tabs.current_tab = i
		await _frames(3)
		await _shot("settings_%d_%s" % [i, tabs.get_child(i).name.to_lower()])
	menu.settings_panel.cancel()
	await _frames(2)

	menu.single_player_button.pressed.emit()
	await _frames(3)
	menu.map_option.select(maxi(Array(menu._maps).find("test_small"), 0))
	menu.mode_option.select(MatchSetup.MODES.keys().find("base"))
	await _shot("menu_setup_base")
	menu.start_button.pressed.emit()
	var main: Node3D = await _wait_scene("Main")
	while not main._loaded:
		await _frames(1)
	await _frames(60)
	var own: Array = []
	var squads: Array = []
	for id: int in main.sim_host.unit_ids():
		var info: Dictionary = main.sim_host.unit_info(id)
		if main.selection._own_index(id) >= 0:
			own.append(id)
			if info.get("squad_size", 0) > 0:
				squads.append(id)
	var tanks := own.filter(func(id: int) -> bool: return not squads.has(id))
	main.selection.set_selection(tanks.slice(0, 4))
	await _frames(20)
	await _shot("hud_tanks")
	if not squads.is_empty():
		main.selection.set_selection([squads[0]])
		await _frames(20)
		await _shot("hud_squad")
	main.selection.set_selection([])
	main.hud.command_bar.press("build")
	await _frames(20)
	await _shot("hud_build_menu")
	main.base_hud.cancel_placing()
	main.game_menu.open_pause()
	await _frames(4)
	await _shot("pause")
	main.game_menu.surrender_button.pressed.emit()
	main.game_menu.confirm_surrender_button.pressed.emit()
	for i in 600:
		if main.game_menu.result_panel.visible:
			break
		await _frames(1)
	await _frames(10)
	await _shot("result")
	print("l10n shots: %d pictures in %s: %s" % [shots.size(), dir, ", ".join(shots)])
	get_tree().quit(0)


func _shot(shot_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var path := dir.path_join(shot_name + ".png")
	image.save_png(path)
	shots.append(shot_name)


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame


func _wait_scene(scene_name: String) -> Node:
	for i in 1200:
		var scene := get_tree().current_scene
		if scene != null and scene.name == scene_name and scene.is_node_ready():
			return scene
		await get_tree().process_frame
	push_error("l10n shots: scene %s did not appear" % scene_name)
	get_tree().quit(1)
	return null
