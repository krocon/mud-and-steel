extends "res://scenes/menu/dialog_frame.gd"
## "Missions" screen of the main menu (prompt 039): the tutorial and the
## missions with their faction, map and whether they were won; the
## briefing of the selected one; Start plays it in `main.tscn`.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const Language := preload("res://scenes/menu/language.gd")

signal back_pressed

var list: ItemList
var details: Label
var start_button: Button
var back_button: Button
## Difficulty (prompt 181), in the order of `Missions.LEVELS`.
var level_button: OptionButton
## Headers of `Missions.list`, in list order.
var entries: Array[Dictionary] = []


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var frame := PanelContainer.new()
	add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(720, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title("MENU_MISSIONS_TITLE", MenuTheme.HEADING_SIZE))
	list = ItemList.new()
	# Grows with the window (dialog_frame.gd).
	list.custom_minimum_size = Vector2(0, 160)
	list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	list.item_selected.connect(func(_index: int) -> void: _update_details())
	list.item_activated.connect(func(_index: int) -> void: start_selected())
	box.add_child(list)
	details = MenuTheme.hint("")
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	details.custom_minimum_size = Vector2(0, 96)
	box.add_child(details)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	var level_label := MenuTheme.hint("MENU_LEVEL")
	# One line beside the buttons: wrapping would squeeze it to one letter
	# per line and stretch the row.
	level_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	level_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	buttons.add_child(level_label)
	level_button = OptionButton.new()
	level_button.custom_minimum_size = Vector2(140, 44)
	for level: String in Missions.LEVELS:
		level_button.add_item(Missions.LEVEL_KEYS[level])
	level_button.item_selected.connect(func(index: int) -> void:
		Missions.set_level(Missions.LEVELS[index]))
	buttons.add_child(level_button)
	back_button = _button(buttons, "COMMON_BACK", func() -> void: back_pressed.emit())
	start_button = _button(buttons, "COMMON_START", start_selected)
	visibility_changed.connect(func() -> void:
		if visible:
			refresh())


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and list != null and visible:
		refresh()


## Reads the mission files again.
func refresh() -> void:
	entries = Missions.list()
	list.clear()
	for m in entries:
		var index := list.add_item(describe(m))
		if not m.get("ok", false):
			list.set_item_custom_fg_color(index, MenuTheme.DEFEAT.darkened(0.2))
	level_button.select(Missions.LEVELS.find(Missions.level()))
	if entries.is_empty():
		details.text = "MENU_MISSIONS_NONE"
	else:
		list.select(0)
	_update_details()


## One list line: title, side, map, and "won" once it was won.
static func describe(m: Dictionary) -> String:
	if not m.get("ok", false):
		return Language.t("MENU_MISSION_BROKEN").format({"id": m["id"], "map": m["map"]})
	var side := Language.t(MatchSetup.FACTIONS.get(m.get("faction", ""), ""))
	var kind := Language.t("MENU_MISSION_TUTORIAL") if m["kind"] == "tutorial" else side
	var won := ""
	if Missions.is_done(m["id"]):
		won = "   %s · %s" % [Language.t("MENU_MISSION_WON"), Language.t(Missions.LEVEL_KEYS.get(Missions.best_level(m["id"]), "MENU_LEVEL_NORMAL"))]
	return "%s   –   %s, %s%s" % [Missions.text(m["path"], m["title"]), kind, m["map"], won]


## Index of the mission with this id, or -1.
func find(id: String) -> int:
	for i in entries.size():
		if entries[i]["id"] == id:
			return i
	return -1


func start_selected() -> void:
	var selected := list.get_selected_items()
	if selected.is_empty():
		return
	var m := entries[selected[0]]
	if m.get("ok", false):
		Missions.start(get_tree(), m, Missions.LEVELS[maxi(level_button.selected, 0)])


func _update_details() -> void:
	var selected := list.get_selected_items()
	if selected.is_empty():
		start_button.disabled = true
		return
	var m := entries[selected[0]]
	start_button.disabled = not m.get("ok", false)
	details.text = Missions.text(m["path"], m.get("briefing", "")) if m.get("ok", false) else m.get("error", "")


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
