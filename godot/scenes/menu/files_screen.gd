extends CenterContainer
## A screen of the main menu that lists saved games ("Load game") or
## replays ("Replays", prompt 038): newest first, one line each with date,
## map, players, duration and (replays) the result. Files this build cannot
## use are greyed out with the reason. The action button loads the saved
## game or plays the replay in `main.tscn`.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")

signal back_pressed

## "load" or "replays".
var kind := "load"
var list: ItemList
var details: Label
var action_button: Button
var back_button: Button
## Entries of `GameFiles.list`, in list order.
var entries: Array[Dictionary] = []


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var frame := PanelContainer.new()
	add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(820, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title("MENU_LOAD_GAME" if kind == "load" else "MENU_REPLAYS", MenuTheme.HEADING_SIZE))
	list = ItemList.new()
	list.custom_minimum_size = Vector2(0, 340)
	list.item_selected.connect(func(_index: int) -> void: _update_details())
	list.item_activated.connect(func(_index: int) -> void: activate())
	box.add_child(list)
	details = MenuTheme.hint("")
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(details)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	back_button = _button(buttons, "COMMON_BACK", func() -> void: back_pressed.emit())
	action_button = _button(buttons, "COMMON_LOAD" if kind == "load" else "FILES_WATCH", activate)
	visibility_changed.connect(func() -> void:
		if visible:
			refresh())


## The list lines are built with `tr`: read them again when the language
## changes (prompt 043).
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and list != null and visible:
		refresh()


## Reads the folder again.
func refresh() -> void:
	var saved_games := kind == "load"
	entries = GameFiles.list(
		GameFiles.SAVE_DIR if saved_games else GameFiles.REPLAY_DIR,
		GameFiles.SAVE_EXTENSION if saved_games else GameFiles.REPLAY_EXTENSION,
	)
	list.clear()
	for entry in entries:
		var index := list.add_item(GameFiles.describe(entry, saved_games))
		if not entry["info"].get("playable", false):
			list.set_item_custom_fg_color(index, MenuTheme.DEFEAT.darkened(0.2))
	if entries.is_empty():
		details.text = "FILES_NO_SAVES" if saved_games else "FILES_NO_REPLAYS"
	else:
		list.select(0)
	_update_details()


## Index of the first entry whose file name is `file_name`, or -1.
func find(file_name: String) -> int:
	for i in entries.size():
		if entries[i]["name"] == file_name:
			return i
	return -1


## Loads or plays the selected entry.
func activate() -> void:
	var selected := list.get_selected_items()
	if selected.is_empty():
		return
	var entry: Dictionary = entries[selected[0]]
	var info: Dictionary = entry["info"]
	if not info.get("playable", false):
		details.text = tr("FILES_INCOMPATIBLE") % info.get("error", "")
		return
	if kind == "load":
		MatchSetup.pending = {"load": entry["path"], "map": info["map"]}
		print("Menu: loading %s" % entry["path"])
	else:
		var error := GameFiles.open_replay(entry["path"])
		if not error.is_empty():
			details.text = tr("FILES_CANNOT_PLAY") % error
			return
		GameFiles.watch_replay(0, -1)
		print("Menu: watching %s" % entry["path"])
	get_tree().change_scene_to_file(MatchSetup.MAIN_SCENE)


func _update_details() -> void:
	var selected := list.get_selected_items()
	action_button.disabled = selected.is_empty()
	if selected.is_empty():
		return
	var entry: Dictionary = entries[selected[0]]
	var info: Dictionary = entry["info"]
	if not info.get("ok", false) or not info.get("playable", false):
		details.text = "%s\n%s" % [entry["name"], info.get("error", "")]
		action_button.disabled = true
		return
	details.text = "%s\n%s" % [
		entry["name"],
		(tr("FILES_VERSION_MULTIPLAYER") if info["multiplayer"] else tr("FILES_VERSION")) % info["game_version"],
	]


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
