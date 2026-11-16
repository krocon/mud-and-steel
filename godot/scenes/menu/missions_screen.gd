extends CenterContainer
## "Missions" screen of the main menu (prompt 039): the tutorial and the
## missions with their faction, map and whether they were won; the
## briefing of the selected one; Start plays it in `main.tscn`.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Missions := preload("res://scenes/menu/missions.gd")

signal back_pressed

var list: ItemList
var details: Label
var start_button: Button
var back_button: Button
## Headers of `Missions.list`, in list order.
var entries: Array[Dictionary] = []


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var frame := PanelContainer.new()
	add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(720, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title("Missions", MenuTheme.HEADING_SIZE))
	list = ItemList.new()
	list.custom_minimum_size = Vector2(0, 240)
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
	back_button = _button(buttons, "Back", func() -> void: back_pressed.emit())
	start_button = _button(buttons, "Start", start_selected)
	visibility_changed.connect(func() -> void:
		if visible:
			refresh())


## Reads the mission files again.
func refresh() -> void:
	entries = Missions.list()
	list.clear()
	for m in entries:
		var index := list.add_item(describe(m))
		if not m.get("ok", false):
			list.set_item_custom_fg_color(index, MenuTheme.DEFEAT.darkened(0.2))
	if entries.is_empty():
		details.text = "No missions found."
	else:
		list.select(0)
	_update_details()


## One list line: title, side, map, and "won" once it was won.
static func describe(m: Dictionary) -> String:
	if not m.get("ok", false):
		return "%s (%s): cannot be read" % [m["id"], m["map"]]
	var side: String = MatchSetup.FACTIONS.get(m.get("faction", ""), "")
	var kind := "Tutorial" if m["kind"] == "tutorial" else side
	return "%s   –   %s, %s%s" % [m["title"], kind, m["map"], "   (won)" if Missions.is_done(m["id"]) else ""]


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
		Missions.start(get_tree(), m)


func _update_details() -> void:
	var selected := list.get_selected_items()
	if selected.is_empty():
		start_button.disabled = true
		return
	var m := entries[selected[0]]
	start_button.disabled = not m.get("ok", false)
	details.text = m.get("briefing", "") if m.get("ok", false) else m.get("error", "")


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
