extends Node
## Screenshots of the unit values in the interface (prompt 066), to check by
## eye that everything is readable and nothing overflows:
## `just unit-card-shots` (into `build/unit-cards/`, German and English at
## two interface scales and at 1280 × 720). Started by `main_menu.gd` for
## `--unit-card-shots=<dir>`; needs a window.
##
## Shots: the unit reference of the main menu (a tank with both tabs, a
## squad, the general page), then a base-building match on test_small with
## a barracks, a depot and a vehicle works built at eight times the speed: the
## production bar with the ratings on the tiles, the tooltip of a tile, the
## unit card with both tabs next to the build menu, and a selected unit
## with its ratings in the unit panel and its card.
## Arguments: `--language=<code>`, `--ui-scale=<f>`.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const Picker := preload("res://scenes/hud/picker.gd")

var dir := ""
var shots: PackedStringArray = []


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	var scale := 1.0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--unit-card-shots="):
			dir = arg.trim_prefix("--unit-card-shots=")
		elif arg.begins_with("--ui-scale="):
			scale = float(arg.trim_prefix("--ui-scale="))
	Settings.set_value("ui_scale", scale)
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var menu: Control = await _wait_scene("MainMenu")
	await _frames(10)
	await _shot("menu_main")
	menu.units_button.pressed.emit()
	await _frames(20)
	var screen: Control = menu.units_screen
	screen.show_unit("de_pz4_h")
	await _frames(20)
	await _shot("reference_tank_game")
	screen.card.open_tab("tech")
	await _frames(5)
	await _shot("reference_tank_tech")
	screen.card.open_tab("game")
	screen.show_unit("de_inf_rifle_squad")
	await _frames(20)
	await _shot("reference_squad")
	screen.open_faction("neutral")
	await _frames(20)
	await _shot("reference_general")
	screen.back_button.pressed.emit()
	await _frames(3)

	menu.single_player_button.pressed.emit()
	await _frames(3)
	menu.select_map("test_small")
	menu.mode_option.select(MatchSetup.MODES.keys().find("base"))
	menu.opponent_option.select(MatchSetup.OPPONENTS.keys().find("passive"))
	menu.start_button.pressed.emit()
	var main: Node3D = await _wait_scene("Main")
	while not main._loaded:
		await _frames(1)
	await _frames(30)
	var host: SimHost = main.sim_host
	var hud: Control = main.hud
	var base_hud: Control = main.base_hud
	var barracks := await _build(main, "base_barracks")
	await _build(main, "base_depot")
	var works := await _build(main, "base_factory")
	host.speed = 1.0
	await _frames(20)

	# The production bar of the barracks: infantry tiles with ratings.
	main.selection._set_selection([barracks])
	await _frames(30)
	await _shot("picker_infantry")
	main.selection._set_selection([works])
	await _frames(30)
	var picker: Picker = base_hud.production_picker
	await _shot("picker_vehicles")
	# A vehicle with an entry in the vehicle database, for the second tab.
	var shown: int = picker.tiles.keys()[0]
	for o: Dictionary in host.production_options(works):
		if picker.tiles.has(int(o["type"])) and not SimHost.vehicle_record(str(o["id"])).is_empty():
			shown = int(o["type"])
			break
	var tile: Button = picker.tiles[shown]
	await _tooltip_shot(tile, "tooltip_vehicle")
	# Right click on the tile: the unit card next to the production bar.
	_right_click(tile.get_global_rect().get_center())
	await _frames(10)
	await _shot("card_game")
	hud.unit_card.open_tab("tech")
	await _frames(5)
	await _shot("card_tech")
	hud.unit_card.close()
	# The build menu's defense tab: fortifications show what they have.
	main.selection._set_selection([])
	hud.command_bar.press("build")
	await _frames(20)
	base_hud.build_picker.open_tab("defense")
	await _frames(10)
	await _shot("picker_defense")
	hud.command_bar.press("build")
	base_hud.cancel_placing()
	# A selected unit: ratings in the unit panel, the card by its button.
	var unit := -1
	for id: int in host.unit_ids():
		var info: Dictionary = host.unit_info(id)
		if main.selection._own_index(id) >= 0 and not str(info.get("type_id", "")).begins_with("base_"):
			unit = id
			if int(info.get("squad_size", 0)) == 0:
				break
	if unit >= 0:
		main.selection.set_selection([unit])
		await _frames(30)
		await _shot("panel_unit")
		hud.unit_panel.card_button.pressed.emit()
		await _frames(10)
		await _shot("panel_unit_card")
	print("unit card shots: %d pictures in %s: %s" % [shots.size(), dir, ", ".join(shots)])
	get_tree().quit(0)


## Places building `id` near the headquarters and waits until it stands;
## returns its unit id.
func _build(main: Node3D, id: String) -> int:
	var host: SimHost = main.sim_host
	var base_hud: Control = main.base_hud
	var hq := Vector3.ZERO
	for s: Dictionary in host.structures():
		if s["owner"] == 0 and s["type_id"] == "base_hq":
			hq = host.current_positions()[Array(host.unit_ids()).find(s["id"])]
	main.camera_rig.snap_to(hq, 90.0, 0.0)
	var type := -1
	for o: Dictionary in host.build_options():
		if o["id"] == id:
			type = o["type"]
	base_hud.start_placing(type)
	var placed := false
	for r: float in [22.0, 32.0, 42.0, 52.0]:
		for k in 16:
			var p := hq + Vector3(cos(TAU * k / 16.0), 0, sin(TAU * k / 16.0)) * r
			if not placed and host.placement_check(type, p.x, p.z).is_empty():
				placed = base_hud.place_at(p)
	base_hud.cancel_placing()
	if not placed:
		push_error("unit card shots: no place for %s" % id)
		get_tree().quit(1)
		return -1
	host.speed = 8.0
	for i in 600:
		for s: Dictionary in host.structures():
			if s["owner"] == 0 and s["type_id"] == id and float(s["built"]) >= 1.0:
				return s["id"]
		await _frames(5)
	push_error("unit card shots: %s was not built" % id)
	return -1


## A picture of the tooltip of `tile`: the mouse rests on it until the
## engine shows the real tooltip (embedded in the window, so it is in the
## picture); if that does not come, its panel is put next to the tile.
func _tooltip_shot(tile: Button, shot_name: String) -> void:
	var root := get_viewport()
	root.gui_embed_subwindows = true
	var at := tile.get_global_rect().get_center()
	for offset: Vector2 in [Vector2(-6, 0), Vector2(-3, 0), Vector2.ZERO]:
		_motion(at + offset)
		await _frames(2)
	await get_tree().create_timer(1.5).timeout
	var real := not root.get_embedded_subwindows().is_empty()
	var stand_in: Control = null if real else _tooltip(tile)
	print("unit card shots: %s tooltip" % ("the engine's" if real else "stand-in"))
	await _frames(5)
	await _shot(shot_name)
	if stand_in != null:
		stand_in.queue_free()
	_motion(Vector2(root.get_visible_rect().size.x * 0.6, 200))
	await _frames(10)
	root.gui_embed_subwindows = false


func _motion(at: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = at
	motion.global_position = at
	get_viewport().push_input(motion)


## The tooltip of `tile` as a panel next to it.
func _tooltip(tile: Button) -> Control:
	var panel := PanelContainer.new()
	panel.theme_type_variation = &"TooltipPanel"
	panel.add_child(tile.call("_make_custom_tooltip", tile.tooltip_text))
	tile.get_tree().current_scene.get_node("Hud").add_child(panel)
	panel.reset_size()
	var at := tile.get_global_rect()
	panel.global_position = Vector2(at.end.x + 8.0, at.position.y - 40.0)
	return panel


func _right_click(at: Vector2) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_RIGHT
		event.position = at
		event.global_position = at
		event.pressed = pressed
		get_viewport().push_input(event)


func _shot(shot_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(shot_name + ".png"))
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
	push_error("unit card shots: scene %s did not appear" % scene_name)
	get_tree().quit(1)
	return null
