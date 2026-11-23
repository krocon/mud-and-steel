extends Node
## Screenshots of the map picker and the map previews (prompt 201):
## `just map-picker-shots` (into `build/map_picker/`). Started by
## `main_menu.gd` for `--map-picker-shots=<dir>`; needs a window.
##
## Shots at several window sizes: the single-player setup with the
## preview, the picker (top view and 3D relief), the missions screen, the
## LAN lobby and the map editor's "Open".

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MapPreview := preload("res://scenes/menu/map_preview.gd")

const SIZES: Array[Vector2i] = [Vector2i(1600, 900), Vector2i(1024, 600), Vector2i(2560, 1440)]

var dir := ""


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--map-picker-shots="):
			dir = arg.trim_prefix("--map-picker-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var menu: Control = await _wait_scene("MainMenu")
	await _frames(10)
	for size in SIZES:
		get_window().size = size
		await _frames(6)
		var tag := "%dx%d" % [get_window().size.x, get_window().size.y]
		menu.show_screen("skirmish")
		menu.select_map("coastal_heights")
		await _ready_preview(menu.map_preview)
		await _shot("setup_coastal_heights_" + tag)
		var picker: Control = menu.open_map_picker()
		await _frames(3)
		picker.select("river_city")
		await _ready_preview(picker.preview)
		await _shot("picker_river_city_" + tag)
		if size == SIZES[0]:
			picker.select("twin_meadow")
			await _ready_preview(picker.preview)
			picker.layout_option.select(1)
			picker.layout_option.item_selected.emit(1)
			await _frames(4)
			await _shot("picker_twin_meadow_variant")
			picker.select("coastal_heights")
			await _ready_preview(picker.preview)
			picker.set_relief(true)
			await _frames(40)
			await _shot("picker_coastal_heights_3d")
			picker.exaggerate_box.button_pressed = false
			await _frames(10)
			await _shot("picker_coastal_heights_3d_true_heights")
			picker.set_relief(false)
		picker.cancel()
		await _frames(3)
	get_window().size = SIZES[0]
	await _frames(6)
	menu.show_screen("skirmish")
	menu.select_map("skirmish")
	await _ready_preview(menu.map_preview)
	await _shot("setup_skirmish")
	menu.show_screen("missions")
	await _frames(4)
	await _ready_preview(menu.missions_screen.map_preview)
	await _shot("missions")
	menu.show_screen("main")
	menu.lan_button.pressed.emit()
	var lobby: Control = await _wait_scene("LanLobby")
	await _frames(4)
	await _ready_preview(lobby._map_preview)
	await _shot("lan_lobby")
	lobby.back_button.pressed.emit()
	menu = await _wait_scene("MainMenu")
	await _frames(4)
	menu.editor_button.pressed.emit()
	var editor: Node3D = await _wait_scene("MapEditorScene")
	await _frames(30)
	editor.panel.show_dialog("open")
	await _frames(3)
	editor.panel.picker.select("brick_quarter")
	await _ready_preview(editor.panel.picker.preview)
	await _shot("editor_open")
	print("map picker shots: %s" % dir)
	get_tree().quit(0)


func _ready_preview(preview: Control) -> void:
	for i in 600:
		if preview.is_ready():
			break
		await _frames(1)
	await _frames(4)


func _shot(shot_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(shot_name + ".png"))
	print("map picker shots: %s" % shot_name)


func _wait_scene(scene_name: String) -> Node:
	for i in 600:
		var scene := get_tree().current_scene
		if scene != null and scene.name == scene_name and scene.is_node_ready():
			return scene
		await get_tree().process_frame
	push_error("map picker shots: scene %s did not appear" % scene_name)
	get_tree().quit(1)
	return null


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
