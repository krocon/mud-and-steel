extends MarginContainer
## "Units" screen of the main menu (prompt 066): a reference of every unit,
## fortification and base building of the catalog.
##
## Left: the factions (and "General" for what all of them share: base
## buildings and field fortifications) and under the chosen one its entries
## by kind (infantry, vehicles, guns, aircraft, defense, base) in catalog
## order, so the categories stay together. Environment objects are not
## listed. Entries that are in no build or production menu (vehicles
## without rules yet, crews, aircraft of the air support, what engineers
## dig) follow under "Not in the build menus" within their kind, dimmed. Right: the unit
## card (`hud/unit_card.gd`, the same as in a match) with the 3D model to
## turn instead of the picture and a camouflage choice.
##
## The list is `SimHost.catalog_ids()`, so new catalog entries appear by
## themselves. Display only.

signal back_pressed

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
const UnitCard := preload("res://scenes/hud/unit_card.gd")
## Faction codes in tab order with the keys of their names.
const FACTIONS: Dictionary[String, String] = {
	"de": "COMMON_FACTION_DE", "su": "COMMON_FACTION_SU", "us": "COMMON_FACTION_US",
	"neutral": "UNIT_REF_GENERAL",
}
## Kinds in list order: catalog group -> heading.
const KINDS: Dictionary[String, String] = {
	"infantry": "HUD_TAB_INFANTRY", "vehicle": "HUD_TAB_VEHICLES", "gun": "HUD_TAB_GUNS",
	"aircraft": "HUD_TAB_AIR", "fortification": "HUD_TAB_DEFENSE", "building": "HUD_TAB_BASE",
}
const LIST_WIDTH := 360.0

## The unit card on the right.
var card: UnitCard
var back_button: Button
## Open faction and the entry shown.
var faction := "de"
var shown_id := ""
## Entry button per catalog id of the open faction.
var entries: Dictionary[String, Button] = {}

var _built := false
var _faction_buttons: Dictionary[String, Button] = {}
var _list: VBoxContainer
## Faction -> catalog ids in list order.
var _ids: Dictionary[String, Array] = {}


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in [&"margin_left", &"margin_right", &"margin_top", &"margin_bottom"]:
		add_theme_constant_override(side, 16)
	visibility_changed.connect(func() -> void:
		if visible:
			_build())


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and _built:
		open_faction(faction)


## The factions that have entries, in tab order.
func faction_ids() -> Array[String]:
	_build()
	var out: Array[String] = []
	for code in FACTIONS:
		if not (_ids.get(code, []) as Array).is_empty():
			out.append(code)
	return out


## The catalog ids of faction `code`, in list order.
func ids_of(code: String) -> Array:
	_build()
	return _ids.get(code, [])


## Shows the entries of faction `code` and its first unit.
func open_faction(code: String) -> void:
	_build()
	faction = code
	for key in _faction_buttons:
		_faction_buttons[key].set_pressed_no_signal(key == code)
	for child in _list.get_children():
		_list.remove_child(child)
		child.queue_free()
	entries.clear()
	var kind := ""
	var buildable := true
	for id: String in _ids.get(code, []):
		var info := UnitStats.info(id)
		var group := str(info.get("group", ""))
		if group != kind:
			kind = group
			buildable = true
			var heading := Label.new()
			heading.text = KINDS.get(group, "HUD_TAB_OTHER")
			heading.add_theme_color_override(&"font_color", MenuTheme.ACCENT)
			heading.add_theme_font_size_override(&"font_size", MenuTheme.font(15))
			_list.add_child(heading)
		if buildable and not UnitStats.buildable(id):
			buildable = false
			var mark := Label.new()
			mark.text = "UNIT_REF_NOT_BUILDABLE"
			mark.add_theme_color_override(&"font_color", MenuTheme.TEXT_DIM)
			mark.add_theme_font_size_override(&"font_size", MenuTheme.font(13))
			_list.add_child(mark)
		var button := Button.new()
		button.name = "Entry_" + id
		button.toggle_mode = true
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.icon = Thumbnail.texture(id)
		button.expand_icon = true
		button.custom_minimum_size = Vector2(0, 38)
		button.add_theme_constant_override(&"icon_max_width", 48)
		button.add_theme_font_size_override(&"font_size", MenuTheme.font(14))
		button.clip_text = true
		button.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
		var category := str(info.get("category", ""))
		button.text = str(info.get("name", id))
		if not UnitStats.buildable(id):
			button.add_theme_color_override(&"font_color", MenuTheme.TEXT_DIM)
		if not category.is_empty():
			button.tooltip_text = Language.code_text("UNIT_CAT_", category)
		button.pressed.connect(show_unit.bind(id))
		_list.add_child(button)
		entries[id] = button
	var ids: Array = _ids.get(code, [])
	if not ids.is_empty():
		show_unit(shown_id if ids.has(shown_id) else str(ids[0]))


## Shows the unit card of catalog id `id` (of the open faction).
func show_unit(id: String) -> void:
	shown_id = id
	for key in entries:
		entries[key].set_pressed_no_signal(key == id)
	card.show_type(id)


func _build() -> void:
	if _built:
		return
	_built = true
	for id in SimHost.catalog_ids():
		var info := UnitStats.info(id)
		var code := str(info.get("faction", "neutral"))
		if not _ids.has(code):
			_ids[code] = []
		_ids[code].append(id)
	# By kind in the order of KINDS, what can be built first; the catalog
	# order stays within both.
	var order := KINDS.keys()
	for code: String in _ids:
		var sorted: Array = []
		for group: String in order:
			for buildable: bool in [true, false]:
				for id: String in _ids[code]:
					if str(UnitStats.info(id).get("group", "")) == group and UnitStats.buildable(id) == buildable:
						sorted.append(id)
		_ids[code] = sorted

	var columns := HBoxContainer.new()
	columns.add_theme_constant_override(&"separation", 16)
	add_child(columns)
	var left := VBoxContainer.new()
	left.custom_minimum_size = Vector2(LIST_WIDTH, 0)
	left.add_theme_constant_override(&"separation", 6)
	columns.add_child(left)
	var title := MenuTheme.title("UNIT_REF_TITLE", MenuTheme.HEADING_SIZE)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	left.add_child(title)
	var tabs := GridContainer.new()
	tabs.columns = 2
	tabs.add_theme_constant_override(&"h_separation", 4)
	tabs.add_theme_constant_override(&"v_separation", 4)
	left.add_child(tabs)
	for code in FACTIONS:
		if (_ids.get(code, []) as Array).is_empty():
			continue
		var button := Button.new()
		button.name = "Faction_" + code
		button.text = FACTIONS[code]
		button.toggle_mode = true
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.add_theme_font_size_override(&"font_size", MenuTheme.font(14))
		button.pressed.connect(open_faction.bind(code))
		tabs.add_child(button)
		_faction_buttons[code] = button
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	left.add_child(scroll)
	_list = VBoxContainer.new()
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_list.add_theme_constant_override(&"separation", 3)
	scroll.add_child(_list)
	back_button = Button.new()
	back_button.text = "COMMON_BACK"
	back_button.custom_minimum_size = Vector2(140, 40)
	back_button.pressed.connect(func() -> void: back_pressed.emit())
	left.add_child(back_button)

	card = UnitCard.new()
	card.name = "UnitCard"
	card.with_model = true
	card.closable = false
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	columns.add_child(card)
	open_faction(faction if _ids.has(faction) else str(_ids.keys()[0]))
