extends "res://scenes/menu/dialog_frame.gd"
## "Campaign" screen of the main menu (prompts 189, 237): the campaigns of
## `data/campaigns/` with a horizontal interactive campaign timeline above
## their mission list. A mission is locked until the one before it was won
## (on any level); won ones show their stars per level. The text below is
## the campaign's briefing, tagline, selected mission's intro (and its outro
## once it was won). Start plays it like the missions screen, at the
## difficulty chosen here.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const Language := preload("res://scenes/menu/language.gd")

signal back_pressed

var campaign_button: OptionButton
var progress: Label
var timeline: Control
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


class CampaignTimeline extends Control:
	signal chapter_clicked(index: int)

	## Array of `{index, label, won, open, selected, stars}` (self-test).
	var nodes: Array[Dictionary] = []
	var _centers: Array[Vector2] = []

	func _ready() -> void:
		custom_minimum_size = Vector2(0, 76)
		mouse_filter = Control.MOUSE_FILTER_STOP

	func set_nodes(items: Array[Dictionary]) -> void:
		nodes = items
		queue_redraw()

	func _gui_input(event: InputEvent) -> void:
		var btn := event as InputEventMouseButton
		if btn == null or not btn.pressed or btn.button_index != MOUSE_BUTTON_LEFT:
			return
		for i in mini(_centers.size(), nodes.size()):
			if btn.position.distance_to(_centers[i]) <= 22.0 and bool(nodes[i].get("open", false)):
				chapter_clicked.emit(i)
				accept_event()
				return

	func _draw() -> void:
		_centers.clear()
		var n := nodes.size()
		if n == 0:
			return
		var pad_x := 64.0
		var y := size.y * 0.44
		var span := maxf(size.x - pad_x * 2.0, 1.0)
		for i in n:
			var t := 0.5 if n == 1 else float(i) / float(n - 1)
			_centers.append(Vector2(pad_x + span * t, y))
		# Connecting segments.
		for i in n - 1:
			var done_seg := bool(nodes[i].get("won", false))
			var col := Color(0.92, 0.78, 0.34, 0.85) if done_seg else Color(0.55, 0.56, 0.58, 0.45)
			draw_line(_centers[i], _centers[i + 1], col, 3.0 if done_seg else 2.0, true)
		var font := get_theme_default_font()
		var num_sz := MenuTheme.font(12)
		var lbl_sz := MenuTheme.font(12)
		for i in n:
			var item: Dictionary = nodes[i]
			var c := _centers[i]
			var won := bool(item.get("won", false))
			var open := bool(item.get("open", false))
			var selected := bool(item.get("selected", false))
			var r := 11.0 if selected else 8.5
			if selected:
				draw_arc(c, r + 4.5, 0.0, TAU, 24, Color(1.0, 0.86, 0.38, 0.95), 2.0, true)
			var fill := MenuTheme.VICTORY if won \
					else (Color(0.94, 0.80, 0.34) if selected \
					else (Color(0.75, 0.78, 0.82) if open else Color(0.28, 0.30, 0.32)))
			draw_circle(c, r + 1.5, Color(0.05, 0.06, 0.07, 0.9))
			draw_circle(c, r, fill)
			var top_text := str(item.get("stars", "")) if won and not str(item.get("stars", "")).is_empty() else str(i + 1)
			var tw := font.get_string_size(top_text, HORIZONTAL_ALIGNMENT_LEFT, -1, num_sz).x
			draw_string(font, Vector2(c.x - tw * 0.5, c.y - r - 6.0), top_text,
					HORIZONTAL_ALIGNMENT_LEFT, -1, num_sz, Color(1.0, 0.88, 0.42) if won else Color(0.84, 0.84, 0.80))
			var label: String = str(item.get("label", ""))
			if not label.is_empty():
				var lw := font.get_string_size(label, HORIZONTAL_ALIGNMENT_LEFT, -1, lbl_sz).x
				var lx := clampf(c.x - lw * 0.5, 4.0, maxf(4.0, size.x - lw - 4.0))
				var lc := Color(0.98, 0.94, 0.82) if selected else (Color(0.84, 0.84, 0.82) if open else Color(0.55, 0.56, 0.58))
				draw_string(font, Vector2(lx, c.y + r + float(lbl_sz) + 6.0), label,
						HORIZONTAL_ALIGNMENT_LEFT, -1, lbl_sz, lc)


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var frame := PanelContainer.new()
	add_child(frame)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(780, 0)
	frame.add_child(box)
	box.add_child(MenuTheme.title("MENU_CAMPAIGN_TITLE", MenuTheme.HEADING_SIZE))
	campaign_button = OptionButton.new()
	campaign_button.custom_minimum_size = Vector2(0, 40)
	campaign_button.item_selected.connect(func(_index: int) -> void: _fill())
	box.add_child(campaign_button)
	progress = MenuTheme.hint("")
	box.add_child(progress)
	timeline = CampaignTimeline.new()
	timeline.name = "CampaignTimeline"
	timeline.chapter_clicked.connect(_on_timeline_clicked)
	box.add_child(timeline)
	list = ItemList.new()
	# Grows with the window (dialog_frame.gd).
	list.custom_minimum_size = Vector2(0, 140)
	list.size_flags_vertical = Control.SIZE_EXPAND_FILL
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
	var level_label := MenuTheme.hint("MENU_LEVEL")
	# One line beside the buttons (see missions_screen.gd).
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


func _on_timeline_clicked(index: int) -> void:
	if index >= 0 and index < list.item_count:
		list.select(index)
		_update_details()


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
		timeline.set_nodes([])
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
		timeline.set_nodes([])
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


## Updates the horizontal timeline nodes to reflect current selection & progress.
func _update_timeline(selected_index: int) -> void:
	var c := campaign()
	if not c.get("ok", false):
		timeline.set_nodes([])
		return
	var missions: Array = c.get("missions", [])
	var items: Array[Dictionary] = []
	for k in missions.size():
		var m: Dictionary = missions[k]
		var id: String = m["id"]
		var header: Dictionary = entries[k] if k < entries.size() else {}
		var open: bool = Missions.unlocked(c, k) and bool(header.get("ok", false))
		var won := Missions.is_done(id)
		var date_raw := String(m.get("date_label", ""))
		var label := Missions.text(c["path"], date_raw) if not date_raw.is_empty() else "#%d" % (k + 1)
		var best_stars := 0
		for lvl in Missions.LEVELS:
			best_stars = maxi(best_stars, Missions.stars(id, lvl))
		items.append({
			"index": k,
			"label": label,
			"won": won,
			"open": open,
			"selected": k == selected_index,
			"stars": "★".repeat(best_stars) if best_stars > 0 else "",
		})
	timeline.set_nodes(items)


## One list line: number, date/season if set, title, side, won with stars or locked.
func describe(header: Dictionary, id: String, k: int, open: bool) -> String:
	var c := campaign()
	var m: Dictionary = c["missions"][k] if k < (c.get("missions", []) as Array).size() else {}
	if header.is_empty():
		return "%d. %s" % [k + 1, Language.t("MENU_MISSION_BROKEN").format({"id": id, "map": m.get("map", "")})]
	var side := Language.t(MatchSetup.FACTIONS.get(header.get("faction", ""), ""))
	var date_raw := String(m.get("date_label", ""))
	var date_part := ("[%s] " % Missions.text(c["path"], date_raw)) if not date_raw.is_empty() else ""
	var line := "%d. %s%s   –   %s" % [k + 1, date_part, Missions.text(header["path"], header["title"]), side]
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
		_update_timeline(-1)
		return
	var k := selected[0]
	_update_timeline(k)
	var m: Dictionary = c["missions"][k]
	var open: bool = Missions.unlocked(c, k) and bool(entries[k].get("ok", false))
	start_button.disabled = not open
	var parts: PackedStringArray = []
	if k == 0:
		parts.append(Missions.text(c["path"], c.get("briefing", "")))
	var tag_raw := String(m.get("tagline", ""))
	if not tag_raw.is_empty():
		parts.append("„%s“" % Missions.text(c["path"], tag_raw))
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
