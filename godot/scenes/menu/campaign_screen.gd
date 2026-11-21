extends CenterContainer
## "Campaign" screen of the main menu (prompt 189): the campaigns of
## `data/campaigns/` with their missions in play order. A mission is
## locked until the one before it was won (on any level); won ones show
## their stars per level. The text below is the campaign's briefing, then
## the selected mission's intro (and its outro once it was won). Start
## plays it like the missions screen, at the difficulty chosen here (the
## same setting). Progress is the one of `menu/missions.gd`; nothing
## carries over between missions.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const Language := preload("res://scenes/menu/language.gd")

signal back_pressed

var campaign_button: OptionButton
var progress: Label
var list: ItemList
var details: Label
var start_button: Button
var back_button: Button
var level_button: OptionButton
## Headers of `Missions.campaigns`.
var campaigns: Array[Dictionary] = []
## The missions of the chosen campaign: their mission headers (from
## `Missions.list`), `{}` where a file is missing.
var entries: Array[Dictionary] = []


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var frame := PanelContainer.new()
	add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(760, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title("MENU_CAMPAIGN_TITLE", MenuTheme.HEADING_SIZE))
	campaign_button = OptionButton.new()
	campaign_button.custom_minimum_size = Vector2(0, 40)
	campaign_button.item_selected.connect(func(_index: int) -> void: _fill())
	box.add_child(campaign_button)
	progress = MenuTheme.hint("")
	box.add_child(progress)
	list = ItemList.new()
	list.custom_minimum_size = Vector2(0, 200)
	list.item_selected.connect(func(_index: int) -> void: _update_details())
	list.item_activated.connect(func(_index: int) -> void: start_selected())
	box.add_child(list)
	details = MenuTheme.hint("")
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	details.custom_minimum_size = Vector2(0, 120)
	box.add_child(details)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	buttons.add_child(MenuTheme.hint("MENU_LEVEL"))
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


## Reads the campaign files and the progress again.
func refresh() -> void:
	var chosen := maxi(campaign_button.selected, 0)
	campaigns = Missions.campaigns()
	campaign_button.clear()
	for c in campaigns:
		var title: String = Missions.text(c["path"], c.get("title", c["id"])) if c.get("ok", false) else String(c["id"])
		campaign_button.add_item(title)
	level_button.select(Missions.LEVELS.find(Missions.level()))
	if campaigns.is_empty():
		list.clear()
		progress.text = ""
		details.text = "MENU_CAMPAIGN_NONE"
		start_button.disabled = true
		return
	campaign_button.select(mini(chosen, campaigns.size() - 1))
	_fill()


## The chosen campaign, {} without one.
func campaign() -> Dictionary:
	if campaigns.is_empty():
		return {}
	return campaigns[maxi(campaign_button.selected, 0)]


func _fill() -> void:
	var c := campaign()
	list.clear()
	entries.clear()
	if not c.get("ok", false):
		details.text = c.get("error", "")
		start_button.disabled = true
		return
	var by_id := {}
	for m in Missions.list():
		by_id[m["id"]] = m
	var missions: Array = c.get("missions", [])
	var won := 0
	var first_open := -1
	for k in missions.size():
		var id: String = missions[k]["id"]
		var header: Dictionary = by_id.get(id, {})
		entries.append(header)
		var open: bool = Missions.unlocked(c, k) and bool(header.get("ok", false))
		if Missions.is_done(id):
			won += 1
		elif open and first_open < 0:
			first_open = k
		var index := list.add_item(describe(header, id, k, open))
		if not open:
			list.set_item_custom_fg_color(index, MenuTheme.DEFEAT.darkened(0.2) if header.is_empty() else Color(0.6, 0.6, 0.6))
	progress.text = Language.t("MENU_CAMPAIGN_PROGRESS").format({"won": won, "total": missions.size()})
	list.select(first_open if first_open >= 0 else 0)
	_update_details()


## One list line: number, title, side, won with its stars or locked.
func describe(header: Dictionary, id: String, k: int, open: bool) -> String:
	if header.is_empty():
		return "%d. %s" % [k + 1, Language.t("MENU_MISSION_BROKEN").format({"id": id, "map": campaign()["missions"][k]["map"]})]
	var side := Language.t(MatchSetup.FACTIONS.get(header.get("faction", ""), ""))
	var line := "%d. %s   –   %s" % [k + 1, Missions.text(header["path"], header["title"]), side]
	if Missions.is_done(id):
		line += "   " + Missions.stars_text(id)
	elif not open:
		line += "   " + Language.t("MENU_CAMPAIGN_LOCKED")
	return line


## Index of the campaign mission with this id, or -1.
func find(id: String) -> int:
	for i in entries.size():
		if entries[i].get("id", "") == id:
			return i
	return -1


func start_selected() -> void:
	var selected := list.get_selected_items()
	if selected.is_empty():
		return
	var k := selected[0]
	var m := entries[k]
	if m.get("ok", false) and Missions.unlocked(campaign(), k):
		Missions.start(get_tree(), m, Missions.LEVELS[maxi(level_button.selected, 0)])


func _update_details() -> void:
	var c := campaign()
	var selected := list.get_selected_items()
	if selected.is_empty() or not c.get("ok", false):
		start_button.disabled = true
		return
	var k := selected[0]
	var m: Dictionary = c["missions"][k]
	var open: bool = Missions.unlocked(c, k) and bool(entries[k].get("ok", false))
	start_button.disabled = not open
	var parts: PackedStringArray = []
	if k == 0:
		parts.append(Missions.text(c["path"], c.get("briefing", "")))
	parts.append(Missions.text(c["path"], m.get("intro", "")))
	if open and not entries[k].is_empty():
		parts.append(Missions.text(entries[k]["path"], entries[k].get("briefing", "")))
	elif not open:
		parts.append(Language.t("MENU_CAMPAIGN_LOCKED_HINT"))
	if Missions.is_done(m["id"]) and not String(m.get("outro", "")).is_empty():
		parts.append(Language.t("MENU_CAMPAIGN_AFTER") + " " + Missions.text(c["path"], m["outro"]))
	var shown: PackedStringArray = []
	for part in parts:
		if not part.is_empty():
			shown.append(part)
	details.text = "\n\n".join(shown)


func _button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 44)
	button.pressed.connect(action)
	parent.add_child(button)
	return button
