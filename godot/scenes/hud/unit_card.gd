extends PanelContainer
## Unit card (prompt 066): everything the game knows about a unit type, in
## two tabs.
##
## - Head: picture (`thumbnail.gd`) or, in the reference of the main menu
##   (`with_model`), the 3D model to turn with a camouflage choice; name,
##   designation, category, faction, service years, price and build time,
##   the five ratings with their numbers. For an own unit with a rank the
##   bonuses of the rank as their own line (never mixed into the values).
## - "Game values": the groups of `SimHost.unit_stats` (movement,
##   vulnerability with the comparison weapons, weapons with penetration
##   over distance, against vehicles with the reference tanks, against
##   infantry, against structures, indirect fire, other).
## - "Technical data": the entry of the vehicle database
##   (`SimHost.vehicle_record`: general, firepower, mobility, armor per
##   assembly) with its source, if the type has one.
##
## Display only: every number is taken from the card as it is and given a
## unit and the number format of the interface language (`unit_stats.gd`);
## the layout follows the unit page (`web/units/stats.js`). The match keeps
## running while the card is open; it takes no keyboard focus, so order
## keys reach the game. `hud.gd` places it and closes it on Escape.

signal closed

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
const RatingBars := preload("res://scenes/hud/rating_bars.gd")
const UnitModelView := preload("res://scenes/menu/unit_model_view.gd")
const PICTURE := Vector2(150, 100)
const MODEL := Vector2(300, 200)
const TABS: Array[String] = ["game", "tech"]
const TAB_NAMES: Dictionary[String, String] = {"game": "UNIT_CARD_TAB_GAME", "tech": "UNIT_CARD_TAB_TECH"}
const SECTIONS: Array[String] = ["general", "firepower", "mobility"]
const ARMOR_FACES: Array[String] = ["front", "side", "rear", "top_bottom"]
const HULL_FACES: Array[String] = ["hull_front", "hull_side", "hull_rear"]
const TURRET_FACES: Array[String] = ["turret_front", "turret_side", "turret_rear"]
const BIOME_NAMES: Dictionary[String, String] = {
	"forest": "MENU_BIOME_FOREST", "desert": "COMMON_BIOME_DESERT", "winter": "COMMON_BIOME_WINTER"}
const LITERS_PER_100_KM := "l/100 km"

## Shows the 3D model instead of the picture (set before the card enters
## the tree).
var with_model := false
## Without the close button (the reference keeps the card open).
var closable := true
## Catalog id shown ("" = none) and the open tab.
var type_id := ""
var tab := "game"
## The 3D model view (`with_model`), else null.
var model_view: UnitModelView
## Ratings drawn in the head.
var rating_bars: RatingBars

var _built := false
var _picture: TextureRect
var _backdrop: ColorRect
var _title: Label
var _subtitle: Label
var _cost: Label
var _mark: Label
var _rank: Label
var _camo: OptionButton
var _tab_buttons: Dictionary[String, Button] = {}
var _pages: Dictionary[String, ScrollContainer] = {}
var _boxes: Dictionary[String, VBoxContainer] = {}
var _unit := -1
var _sim_host: SimHost


func _ready() -> void:
	_build()


func _notification(what: int) -> void:
	# A closed card stays closed; it is filled again when it opens.
	if what == NOTIFICATION_TRANSLATION_CHANGED and _built and visible and not type_id.is_empty():
		show_type(type_id, _unit, _sim_host)


## Shows the card of catalog id `id`. With `unit` (an own unit of
## `sim_host`) the bonuses of its rank get their own line.
func show_type(id: String, unit := -1, sim_host: SimHost = null) -> void:
	_build()
	type_id = id
	_unit = unit
	_sim_host = sim_host
	_fill_head()
	for key in TABS:
		for child in _boxes[key].get_children():
			_boxes[key].remove_child(child)
			child.queue_free()
		_pages[key].scroll_vertical = 0
	_fill_game(_boxes["game"])
	_fill_tech(_boxes["tech"])
	visible = true


func close() -> void:
	if not visible:
		return
	visible = false
	closed.emit()


## Opens tab `key` (`game` or `tech`).
func open_tab(key: String) -> void:
	tab = key
	for other in TABS:
		_pages[other].visible = other == key
		_tab_buttons[other].set_pressed_no_signal(other == key)


## Every text the card shows now, the head first (self-test).
func texts() -> PackedStringArray:
	var out: PackedStringArray = []
	_collect(self, out)
	return out


## The texts of tab `key` (self-test).
func tab_texts(key: String) -> PackedStringArray:
	var out: PackedStringArray = []
	_collect(_boxes[key], out)
	return out


func _collect(node: Node, out: PackedStringArray) -> void:
	if node is Label and not (node as Label).text.is_empty():
		out.append(tr((node as Label).text))
	elif node is Button and not (node as Button).text.is_empty():
		out.append(tr((node as Button).text))
	for child in node.get_children():
		_collect(child, out)


# --- frame -------------------------------------------------------------------

func _build() -> void:
	if _built:
		return
	_built = true
	theme = MenuTheme.get_theme()
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override(&"separation", 6)
	add_child(box)
	var head := HBoxContainer.new()
	box.add_child(head)
	var picture_side := VBoxContainer.new()
	picture_side.add_theme_constant_override(&"separation", 4)
	head.add_child(picture_side)
	var frame := Control.new()
	frame.custom_minimum_size = MODEL if with_model else PICTURE
	frame.clip_contents = true
	picture_side.add_child(frame)
	_backdrop = ColorRect.new()
	_backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	frame.add_child(_backdrop)
	_picture = TextureRect.new()
	_picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_picture.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_picture.mouse_filter = Control.MOUSE_FILTER_IGNORE
	frame.add_child(_picture)
	if with_model:
		model_view = UnitModelView.new()
		model_view.name = "ModelView"
		model_view.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		model_view.tooltip_text = "UNIT_REF_MODEL_HINT"
		frame.add_child(model_view)
		var camo_row := HBoxContainer.new()
		picture_side.add_child(camo_row)
		camo_row.add_child(_text("UNIT_REF_CAMO", 13, MenuTheme.TEXT_DIM))
		_camo = OptionButton.new()
		_camo.focus_mode = Control.FOCUS_NONE
		_camo.add_theme_font_size_override(&"font_size", MenuTheme.font(13))
		for biome in UnitModelView.BIOMES:
			_camo.add_item(BIOME_NAMES[biome])
		_camo.item_selected.connect(func(index: int) -> void:
			model_view.show_model(type_id, UnitModelView.BIOMES[index]))
		_camo.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		camo_row.add_child(_camo)
	var side := VBoxContainer.new()
	side.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	side.add_theme_constant_override(&"separation", 2)
	head.add_child(side)
	var title_row := HBoxContainer.new()
	side.add_child(title_row)
	_title = _text("", 17, MenuTheme.ACCENT)
	_title.name = "Title"
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	title_row.add_child(_title)
	if closable:
		var close_button := Button.new()
		close_button.name = "Close"
		close_button.text = "✕"
		close_button.tooltip_text = "UNIT_CARD_CLOSE"
		close_button.focus_mode = Control.FOCUS_NONE
		close_button.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		close_button.add_theme_font_size_override(&"font_size", MenuTheme.font(13))
		close_button.pressed.connect(close)
		title_row.add_child(close_button)
	_subtitle = _text("", 12, MenuTheme.TEXT_DIM)
	_subtitle.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	side.add_child(_subtitle)
	_cost = _text("", 12, MenuTheme.TEXT)
	_cost.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	side.add_child(_cost)
	_mark = _text("UNIT_CARD_NOT_BUILDABLE", 12, MenuTheme.DEFEAT)
	side.add_child(_mark)
	rating_bars = RatingBars.new(RatingBars.Layout.FULL)
	rating_bars.name = "Ratings"
	rating_bars.bar_width = 90.0
	side.add_child(rating_bars)
	_rank = _text("", 12, MenuTheme.VICTORY)
	_rank.name = "Rank"
	_rank.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_rank)
	var tab_row := HBoxContainer.new()
	tab_row.add_theme_constant_override(&"separation", 2)
	box.add_child(tab_row)
	for key in TABS:
		var button := Button.new()
		button.name = "Tab_" + key
		button.text = TAB_NAMES[key]
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.add_theme_font_size_override(&"font_size", MenuTheme.font(13))
		button.pressed.connect(open_tab.bind(key))
		tab_row.add_child(button)
		_tab_buttons[key] = button
		var page := ScrollContainer.new()
		page.name = "Page_" + key
		page.size_flags_vertical = Control.SIZE_EXPAND_FILL
		page.custom_minimum_size = Vector2(0, 60)
		box.add_child(page)
		_pages[key] = page
		var content := VBoxContainer.new()
		content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		content.add_theme_constant_override(&"separation", 4)
		page.add_child(content)
		_boxes[key] = content
	open_tab(tab)


func _fill_head() -> void:
	var info := UnitStats.info(type_id)
	var stats := UnitStats.card(type_id)
	_backdrop.color = Thumbnail.backdrop(type_id)
	_picture.texture = Thumbnail.texture(type_id)
	if model_view != null:
		model_view.show_model(type_id, UnitModelView.BIOMES[maxi(_camo.selected, 0)])
		# The picture stays as the stand-in for a type without a model.
		model_view.visible = model_view.has_model
		_picture.visible = not model_view.has_model
	_title.text = str(info.get("name", type_id))
	var sub: Array[String] = []
	var designation := str(info.get("designation", ""))
	if not designation.is_empty() and not _title.text.contains(designation):
		sub.append(designation)
	var category := str(info.get("category", "base" if info.get("group", "") == "building" else ""))
	if not category.is_empty():
		sub.append(Language.code_text("UNIT_CAT_", category))
	var faction := str(info.get("faction", ""))
	if Language.has("COMMON_FACTION_" + faction.to_upper()):
		sub.append(tr("COMMON_FACTION_" + faction.to_upper()))
	var years := UnitStats.years(type_id)
	if not years.is_empty():
		sub.append(tr("UNIT_CARD_SERVICE") % years)
	_subtitle.text = " · ".join(sub)
	var cost: Array = (stats.get("other", {}) as Dictionary).get("cost", [])
	var seconds := -1.0
	if cost.size() == 3:
		seconds = int(stats.other.get("build_game_s_x10", 0)) / 10.0
	elif info.has("cost"):
		cost = Array(info.cost)
		seconds = float(info.get("build_s", 0))
	_cost.visible = cost.size() == 3
	if cost.size() == 3:
		_cost.text = tr("HUD_COST_LINE") % [cost[0], cost[1], cost[2], roundi(seconds)]
	_mark.visible = not UnitStats.buildable(type_id)
	rating_bars.ratings = stats.get("ratings", {})
	rating_bars.visible = not rating_bars.ratings.is_empty()
	_rank.visible = false
	if _unit >= 0 and _sim_host != null:
		var xp := _sim_host.unit_experience(_unit)
		if int(xp.get("rank", 0)) > 0:
			_rank.visible = true
			_rank.text = tr("UNIT_CARD_RANK") % tr("HUD_XP_BONUS").format({"rank": xp.rank_name,
					"dispersion": xp.dispersion_pct, "reload": xp.reload_pct, "pinned": xp.suppression_pct})


# --- game values -------------------------------------------------------------

func _fill_game(box: VBoxContainer) -> void:
	var stats := UnitStats.card(type_id)
	if stats.is_empty():
		_fill_building(box)
		return
	_movement(box, stats.get("movement", {}))
	_protection(box, stats.get("protection", {}))
	_weapons(box, stats)
	_against_vehicles(box, stats.get("against_vehicles", {}))
	_against_infantry(box, stats.get("against_infantry", {}))
	_against_structures(box, stats.get("against_structures", {}))
	_indirect(box, stats.get("indirect", {}))
	_other(box, stats.get("other", {}))
	_object(box, stats.get("object", {}))
	if box.get_child_count() > 0:
		_note(box, tr("UNIT_NOTE_BASE"))


## Base buildings are no catalog entries: their values come from the
## economy rules (`SimHost.catalog_card`).
func _fill_building(box: VBoxContainer) -> void:
	var info := UnitStats.info(type_id)
	if info.is_empty():
		return
	var rows: Array = [["UNIT_L_HIT_POINTS", UnitStats.num(int(info.get("hp", 0)))]]
	rows.append(["UNIT_L_ARMOR", _mm(int(info.get("armor_mm", 0)))])
	var income: PackedInt64Array = info.get("income", PackedInt64Array())
	if income.size() == 3 and income[0] + income[1] + income[2] > 0:
		rows.append(["UNIT_L_INCOME", tr("UNIT_V_COST").format({"fuel": income[0], "ammo": income[1], "manpower": income[2]})])
	if not str(info.get("produces", "")).is_empty():
		rows.append(["UNIT_L_PRODUCES", Language.code_text("HUD_PRODUCES_", str(info.produces))])
	var requires: PackedStringArray = info.get("requires", PackedStringArray())
	if not requires.is_empty():
		rows.append(["UNIT_L_REQUIRES", ", ".join(requires)])
	_group(box, "UNIT_GROUP_BUILDING", rows)


func _movement(box: VBoxContainer, mv: Dictionary) -> void:
	if mv.is_empty():
		return
	var rows: Array = []
	var road_real: Variant = mv.get("road_real_kmh_x10", mv.get("max_real_kmh_x10"))
	rows.append(["UNIT_L_ROAD", _speed(road_real, int(mv.get("road_game_mps_x10", 0)))])
	if mv.has("offroad_real_kmh_x10"):
		rows.append(["UNIT_L_OFFROAD", _speed(mv.offroad_real_kmh_x10, int(mv.get("offroad_game_mps_x10", 0)))])
	if mv.has("cruise_real_kmh_x10"):
		rows.append(["UNIT_L_CRUISE", tr("UNIT_V_REAL").format({"real": _kmh(int(mv.cruise_real_kmh_x10))})])
	if mv.has("hull_turn_game_deg_per_s"):
		rows.append(["UNIT_L_HULL_TURN", _per_second(int(mv.hull_turn_game_deg_per_s))])
	if mv.has("turret_turn_game_deg_per_s"):
		rows.append(["UNIT_L_TURRET_TURN", _per_second(int(mv.turret_turn_game_deg_per_s))])
	if mv.has("gun_traverse_deg"):
		rows.append(["UNIT_L_TRAVERSE", tr("UNIT_V_CASEMATE").format({"deg": int(mv.gun_traverse_deg)})])
	if mv.has("mud_speed_pct"):
		rows.append(["UNIT_L_MUD", tr("UNIT_V_OF_SPEED").format({"pct": int(mv.mud_speed_pct)})])
	if mv.has("snow_speed_pct"):
		rows.append(["UNIT_L_SNOW", tr("UNIT_V_OF_SPEED").format({"pct": int(mv.snow_speed_pct)})])
	if mv.has("max_grade_pct"):
		rows.append(["UNIT_L_GRADE", tr("UNIT_V_GRADE").format({
			"pct": int(mv.max_grade_pct), "speed": int(mv.get("steepest_grade_speed_pct", 0))})])
	if mv.has("crushes"):
		var crushed: Array[String] = []
		for id: String in mv.crushes:
			crushed.append(UnitStats.name_of(id))
		rows.append(["UNIT_L_CRUSHES", ", ".join(crushed) if not crushed.is_empty() else tr("UNIT_V_NOTHING")])
	_group(box, "UNIT_GROUP_MOVEMENT", rows)


func _protection(box: VBoxContainer, pr: Dictionary) -> void:
	if pr.is_empty():
		return
	var armored := pr.has("armor_hull_mm")
	var turret := pr.has("armor_turret_mm")
	var rows: Array = []
	if pr.has("soldiers"):
		rows.append(["UNIT_L_HIT_POINTS", tr("UNIT_V_SOLDIERS").format({
			"hp": UnitStats.num(int(pr.hp)), "n": int(pr.soldiers), "each": int(pr.soldier_hp)})])
	else:
		rows.append(["UNIT_L_HIT_POINTS", UnitStats.num(int(pr.get("hp", 0)))])
	if armored:
		rows.append(["UNIT_L_ARMOR_HULL", _faces(pr.armor_hull_mm) + " " + tr("HUD_ARMOR_SIDES")])
	if turret:
		rows.append(["UNIT_L_ARMOR_TURRET", _faces(pr.armor_turret_mm)])
	if armored:
		rows.append(["UNIT_L_FRONT_EFFECTIVE", tr("UNIT_V_EFFECTIVE").format({
			"mm": int(pr.get("hull_front_effective_mm", 0)), "deg": int(pr.get("hull_front_slope_deg", 0))})])
	if pr.get("unarmored", false):
		rows.append(["UNIT_L_ARMOR", tr("UNIT_V_NO_ARMOR")])
	if pr.has("hurt_by_fragments"):
		rows.append(["UNIT_L_FRAGMENTS", tr("UNIT_V_FRAGMENTS_YES" if pr.hurt_by_fragments else "UNIT_V_FRAGMENTS_NO")])
	_group(box, "UNIT_GROUP_PROTECTION", rows)
	var faces: Array[String] = []
	var head: Array = ["UNIT_T_REFERENCE_WEAPON"]
	if armored:
		faces.append_array(HULL_FACES)
		head.append_array(["UNIT_T_HULL_FRONT", "UNIT_T_SIDE", "UNIT_T_REAR"] if turret else ["UNIT_T_FRONT", "UNIT_T_SIDE", "UNIT_T_REAR"])
		if turret:
			faces.append_array(TURRET_FACES)
			head.append_array(["UNIT_T_TURRET_FRONT", "UNIT_T_SIDE", "UNIT_T_REAR"])
	head.append("UNIT_T_HITS")
	var table: Array = []
	for versus: Dictionary in pr.get("versus", []):
		var row: Array = [UnitStats.weapon_class(str(versus.weapon))]
		for face in faces:
			row.append(_reach(int(versus.get(face + "_real_m", -1)), int(versus.range_real_m)))
		if versus.has("hits_per_soldier"):
			row.append(tr("UNIT_V_PER_SOLDIER").format({"n": int(versus.hits_per_soldier)}))
		else:
			row.append(str(int(versus.get("hits_to_destroy", 0))))
		table.append(row)
	_table(box, head, table)
	if armored and not table.is_empty():
		_note(box, tr("UNIT_NOTE_VERSUS"))


func _weapons(box: VBoxContainer, stats: Dictionary) -> void:
	var weapons: Array = stats.get("weapons", [])
	if weapons.is_empty():
		return
	var distances: Array = UnitStats.comparisons().get("distances_real_m", [])
	var head: Array = ["UNIT_T_WEAPON", "UNIT_T_RANGE", "UNIT_T_ROUNDS_MIN"]
	for distance: int in distances:
		head.append(_m(distance))
	var table: Array = []
	for w: Dictionary in weapons:
		var row: Array = [("%d× " % w.count if int(w.count) > 1 else "") + str(w.name)]
		row.append(tr("UNIT_V_RANGE").format({"game": UnitStats.num(int(w.range_game_m)), "real": UnitStats.num(int(w.range_real_m))}))
		row.append(UnitStats.tenths(int(w.shots_per_game_min_x10)))
		var penetration: Array = w.get("penetration_mm", [])
		for i in mini(penetration.size(), distances.size()):
			row.append([_mm(int(penetration[i])), "far" if int(distances[i]) > int(w.range_real_m) else ""])
		table.append(row)
	_group(box, "UNIT_GROUP_WEAPONS", [])
	_table(box, head, table)
	_note(box, tr("UNIT_NOTE_PENETRATION"))


func _against_vehicles(box: VBoxContainer, av: Dictionary) -> void:
	if av.is_empty():
		return
	_group(box, "UNIT_GROUP_VS_VEHICLES", [
		["UNIT_L_STRONGEST", str(av.weapon)],
		["UNIT_L_PENETRATION", tr("UNIT_V_PEN_AT").format({"mm": int(av.penetration_mm), "m": UnitStats.num(int(av.distance_real_m))})],
		["UNIT_L_HITS_TO_DESTROY", str(int(av.hits_to_destroy))],
	])
	var table: Array = []
	for target: Dictionary in av.get("targets", []):
		var row: Array = [UnitStats.name_of(str(target.id))]
		for face in HULL_FACES:
			var within := int(target.get(face + "_real_m", -1))
			if target.get(face, false):
				row.append([tr("COMMON_YES"), "yes"])
			elif within >= 0:
				row.append([tr("UNIT_V_ONLY_WITHIN").format({"m": UnitStats.num(within)}), "no"])
			else:
				row.append([tr("COMMON_NO"), "no"])
		table.append(row)
	_table(box, ["UNIT_T_TARGET", "UNIT_T_FRONT", "UNIT_T_SIDE", "UNIT_T_REAR"], table)
	_note(box, tr("UNIT_NOTE_DUEL").format({"m": UnitStats.num(int(av.distance_real_m))}))


func _against_infantry(box: VBoxContainer, ai: Dictionary) -> void:
	if ai.is_empty():
		return
	var rows: Array = []
	if ai.has("damage_per_hit"):
		rows.append(["UNIT_L_DAMAGE_PER_HIT", tr("UNIT_V_DAMAGE_SOLDIER").format({
			"damage": UnitStats.num(int(ai.damage_per_hit)), "hp": int(ai.get("soldier_hp", 0))})])
	if ai.has("hits_per_soldier"):
		rows.append(["UNIT_L_HITS_PER_SOLDIER", str(int(ai.hits_per_soldier))])
	if ai.has("automatic_weapons"):
		rows.append(["UNIT_L_AUTOMATIC", tr("UNIT_V_AUTOMATIC").format({"n": int(ai.automatic_weapons),
				"rounds": UnitStats.tenths(int(ai.get("automatic_shots_per_game_min_x10", 0)), 0)})])
	if ai.has("blast_radius_game_m_x10"):
		rows.append(["UNIT_L_BLAST_RADIUS", _m_tenths(int(ai.blast_radius_game_m_x10))])
	rows.append(["UNIT_L_SOLDIERS_PER_MIN", tr("UNIT_V_IF_ALL_HIT").format({
		"n": UnitStats.tenths(int(ai.get("soldiers_per_game_min_x10", 0)), 0)})])
	_group(box, "UNIT_GROUP_VS_INFANTRY", rows)


func _against_structures(box: VBoxContainer, structures: Dictionary) -> void:
	if structures.is_empty():
		return
	if not structures.get("effective", false):
		_group(box, "UNIT_GROUP_VS_STRUCTURES", [])
		_note(box, tr("UNIT_V_NO_EFFECT").format({"mm": int(structures.get("min_caliber_mm", 0))}))
		return
	_group(box, "UNIT_GROUP_VS_STRUCTURES", [["UNIT_L_DAMAGE_PER_HIT", UnitStats.num(int(structures.damage_per_hit))]])
	var table: Array = []
	for object: Dictionary in structures.get("objects", []):
		table.append([UnitStats.name_of(str(object.id)), UnitStats.num(int(object.hp)), str(int(object.hits))])
	_table(box, ["UNIT_T_OBJECT", "UNIT_T_HIT_POINTS", "UNIT_T_HITS_TO_COLLAPSE"], table)


func _indirect(box: VBoxContainer, ind: Dictionary) -> void:
	if ind.is_empty():
		return
	_group(box, "UNIT_GROUP_INDIRECT", [
		["UNIT_L_WEAPON", str(ind.name)],
		["UNIT_L_RANGE", tr("UNIT_V_RANGE_INDIRECT").format({"min": UnitStats.num(int(ind.min_range_game_m)),
				"max": UnitStats.num(int(ind.range_game_m)), "real": UnitStats.num(int(ind.range_real_m))})],
		["UNIT_L_SETUP", _seconds_tenths(int(ind.setup_game_s_x10))],
		["UNIT_L_ROUNDS_PER_MIN", UnitStats.tenths(int(ind.shots_per_game_min_x10))],
		["UNIT_L_BLAST_RADIUS", _m_tenths(int(ind.blast_radius_game_m_x10))],
		["UNIT_L_DIRECT_HIT", tr("UNIT_V_DIRECT_HIT").format({"damage": UnitStats.num(int(ind.direct_hit_damage)), "mm": int(ind.direct_hit_roof_mm)})],
		["UNIT_L_SMOKE", tr("COMMON_YES" if ind.get("smoke", false) else "COMMON_NO")],
	])


func _other(box: VBoxContainer, other: Dictionary) -> void:
	if other.is_empty():
		return
	var rows: Array = []
	if other.has("sight_game_m"):
		rows.append(["UNIT_L_SIGHT", tr("UNIT_V_RANGE").format({
			"game": UnitStats.num(int(other.sight_game_m)), "real": UnitStats.num(int(other.get("sight_real_m", 0)))})])
	if other.has("ammo_salvos"):
		if other.has("ammo_game_min_x10"):
			rows.append(["UNIT_L_AMMO", tr("UNIT_V_SALVOS_MIN").format({
				"n": UnitStats.num(int(other.ammo_salvos)), "min": UnitStats.tenths(int(other.ammo_game_min_x10), 1)})])
		else:
			rows.append(["UNIT_L_AMMO", tr("UNIT_V_SALVOS").format({"n": UnitStats.num(int(other.ammo_salvos))})])
	if other.has("fuel_game_m"):
		rows.append(["UNIT_L_FUEL", tr("UNIT_V_FUEL").format({
			"game": UnitStats.num(int(other.fuel_game_m)), "real": UnitStats.num(int(other.get("fuel_real_km", 0)))})])
	if other.has("squad_size"):
		rows.append(["UNIT_L_SQUAD", str(int(other.squad_size))])
	if other.has("crew"):
		rows.append(["UNIT_L_CREW", str(int(other.crew))])
	if other.has("passengers"):
		rows.append(["UNIT_L_SEATS", str(int(other.passengers))])
	_group(box, "UNIT_GROUP_OTHER", rows)


func _object(box: VBoxContainer, object: Dictionary) -> void:
	if object.is_empty():
		return
	var hp := int(object.get("hp", 0))
	var rows: Array = [["UNIT_L_HIT_POINTS", UnitStats.num(hp) if hp > 0 else tr("UNIT_V_INDESTRUCTIBLE")]]
	if hp > 0:
		rows.append(["UNIT_L_MIN_CALIBER", _mm(int(object.get("min_caliber_mm", 0)))])
	if object.has("crush_min_mass_kg"):
		rows.append(["UNIT_L_CRUSHABLE", tr("UNIT_V_CRUSH_FROM").format({"t": UnitStats.num(int(object.crush_min_mass_kg) / 1000.0, 1)})])
	_group(box, "UNIT_GROUP_OBJECT", rows)


# --- technical data ----------------------------------------------------------

func _fill_tech(box: VBoxContainer) -> void:
	var record := UnitStats.record(type_id)
	if record.is_empty():
		_note(box, tr("UNIT_NOTE_NO_RECORD"))
		return
	for section in SECTIONS:
		var data: Dictionary = record.get(section, {})
		var originals: Dictionary = data.get("text", {})
		var keys: Array = data.keys().filter(func(key: String) -> bool: return key != "text")
		for key: String in originals:
			if not data.has(key):
				keys.append(key)
		var rows: Array = []
		for key: String in keys:
			var shown := ""
			if data.has(key):
				shown = field_text(key, data[key])
				if originals.has(key):
					shown += " (%s)" % (tr("UNIT_V_QUOTE") % str(originals[key]))
			else:
				shown = tr("UNIT_V_QUOTE") % str(originals[key])
			rows.append([Language.code_text("UNIT_FIELD_", key), shown])
		_group(box, "UNIT_SECTION_" + section.to_upper(), rows)
	_armor(box, record.get("armor", {}))
	var notes: Array = record.get("notes", [])
	if not notes.is_empty():
		_note(box, tr("UNIT_NOTE_SOURCE_NOTES") % " ".join(notes))
	_note(box, tr("UNIT_NOTE_CATALOG_RULES"))
	var source: Dictionary = record.get("source", {})
	var cite := tr("UNIT_NOTE_SOURCE").format({"name": source.get("name", source.get("url", "")), "date": _date(str(source.get("fetched", "")))})
	var references: Array = source.get("references", [])
	if not references.is_empty():
		cite += tr("UNIT_NOTE_SOURCE_REFS") % ", ".join(references)
	_note(box, cite + ".")


## The armor of every assembly and face, as the source gives it.
func _armor(box: VBoxContainer, armor: Dictionary) -> void:
	if armor.is_empty():
		return
	var head: Array = ["UNIT_T_ASSEMBLY"]
	for face in ARMOR_FACES:
		head.append("UNIT_FACE_" + face.to_upper())
	var table: Array = []
	var footnotes: Array[String] = []
	for part: String in armor:
		var cells: Dictionary = armor[part]
		var label := Language.code_text("UNIT_PART_", part)
		var row: Array = [label]
		for face in ARMOR_FACES:
			var cell: Dictionary = cells.get(face, {})
			var text := _armor_cell(cell)
			if cell.has("text"):
				# The original text says more than the numbers (curved plates).
				footnotes.append("%s %s: %s" % [label, tr("UNIT_FACE_" + face.to_upper()), cell.text])
				text = (text + " *") if not text.is_empty() else "*"
			row.append(text if not text.is_empty() else "–")
		if cells.has("text"):
			footnotes.append("%s: %s" % [label, cells.text])
		table.append(row)
	_group(box, "UNIT_SECTION_ARMOR", [])
	_table(box, head, table)
	var fine := tr("UNIT_NOTE_TECH_ARMOR")
	if not footnotes.is_empty():
		fine += " " + tr("UNIT_NOTE_SOURCE_WORDING") % "; ".join(footnotes)
	_note(box, fine)


func _armor_cell(cell: Dictionary) -> String:
	var parts: Array[String] = []
	if cell.has("mm"):
		parts.append(_mm(int(cell.mm)))
	elif cell.has("mm_min"):
		parts.append(_mm_range(int(cell.mm_min), int(cell.get("mm_max", 0))))
	if cell.has("deg"):
		parts.append(UnitStats.with_unit(str(int(cell.deg)), "°"))
	return " · ".join(parts)


## A database value with its unit, by the suffix of the field name
## (`data/vehicles/SCHEMA.md`). Texts of the source stay as they are.
static func field_text(key: String, value: Variant) -> String:
	if typeof(value) == TYPE_STRING:
		return str(value)
	var v := int(value)
	if key == "turning_radius_mm" and v == 0:
		return Language.t("UNIT_V_TURNS_ON_SPOT")
	if key.ends_with("_kmh_x10"):
		return UnitStats.with_unit(UnitStats.tenths(v), "km/h")
	if key.ends_with("_deg_x10"):
		return UnitStats.with_unit(UnitStats.tenths(v), "°")
	if key == "main_gun_caliber_mm_x10":
		return UnitStats.with_unit(UnitStats.tenths(v), "mm")
	if key == "main_gun_length_cal_x10":
		return "L/" + UnitStats.tenths(v)
	if key == "ground_pressure_kpa_x10":
		return UnitStats.with_unit(UnitStats.tenths(v, 1), "kPa")
	if key == "ground_pressure_psi_x10":
		return UnitStats.with_unit(UnitStats.tenths(v, 1), "psi")
	if key.ends_with("_hp_per_t_x10"):
		return UnitStats.with_unit(UnitStats.tenths(v, 1), Language.t("UNIT_U_HP") + "/t")
	if key.ends_with("_x10"):
		return UnitStats.tenths(v, 1)
	if key.ends_with("_l_per_100km"):
		return UnitStats.with_unit(UnitStats.num(v), LITERS_PER_100_KM)
	if key.ends_with("_mm"):
		return UnitStats.with_unit(UnitStats.num(v / 1000.0, 2), "m") if v >= 1000 else UnitStats.with_unit(UnitStats.num(v), "mm")
	if key.ends_with("_kg"):
		return UnitStats.with_unit(UnitStats.num(v / 1000.0, 1), "t") if v >= 1000 else UnitStats.with_unit(UnitStats.num(v), "kg")
	if key.ends_with("_km"):
		return UnitStats.with_unit(UnitStats.num(v), "km")
	if key.ends_with("_deg"):
		return UnitStats.with_unit(str(v), "°")
	if key.ends_with("_cc"):
		return UnitStats.with_unit(UnitStats.num(v / 1000.0, 1), "l")
	if key.ends_with("_hp"):
		return UnitStats.with_unit(UnitStats.num(v), Language.t("UNIT_U_HP"))
	if key.ends_with("_rpm"):
		return UnitStats.num(v) + "/min"
	if key.ends_with("_l"):
		return UnitStats.with_unit(UnitStats.num(v), "l")
	if key.ends_with("_s"):
		return UnitStats.with_unit(UnitStats.num(v), "s")
	return UnitStats.num(v)


# --- formats -----------------------------------------------------------------

func _mm(value: int) -> String:
	return UnitStats.with_unit(str(value), "mm")


func _mm_range(low: int, high: int) -> String:
	return UnitStats.with_unit("%d–%d" % [low, high], "mm")


func _m(value: int) -> String:
	return UnitStats.with_unit(UnitStats.num(value), "m")


func _m_tenths(value: int) -> String:
	return UnitStats.with_unit(UnitStats.tenths(value, 1), "m")


func _seconds_tenths(value: int) -> String:
	return UnitStats.with_unit(UnitStats.tenths(value, 1), "s")


func _kmh(value: int) -> String:
	return UnitStats.with_unit(UnitStats.tenths(value), "km/h")


func _per_second(degrees: int) -> String:
	return "%d°/s" % degrees


func _faces(values: Array) -> String:
	return UnitStats.with_unit("%d / %d / %d" % [values[0], values[1], values[2]], "mm")


## Real and game speed; guns and aircraft have only one of them.
func _speed(real: Variant, game: int) -> String:
	var in_game := UnitStats.with_unit(UnitStats.tenths(game, 1), "m/s")
	if real == null:
		return tr("UNIT_V_GAME").format({"game": in_game})
	return tr("UNIT_V_REAL_GAME").format({"real": _kmh(int(real)), "game": in_game})


## Up to which real distance a comparison weapon gets through a face.
func _reach(meters: int, weapon_range: int) -> Array:
	if meters < 0:
		return ["–", "no"]
	if meters >= weapon_range:
		return [tr("UNIT_V_ALWAYS"), "yes"]
	return ["≤ " + _m(meters), "yes"]


## An ISO date in the order of the interface language.
func _date(iso: String) -> String:
	var parts := iso.split("-")
	if parts.size() != 3 or not TranslationServer.get_locale().begins_with("de"):
		return iso
	return "%s.%s.%s" % [parts[2], parts[1], parts[0]]


# --- building blocks ---------------------------------------------------------

func _text(text: String, size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override(&"font_size", MenuTheme.font(size))
	label.add_theme_color_override(&"font_color", color)
	return label


## A heading and rows of label (a key) and value.
func _group(box: VBoxContainer, title: String, rows: Array) -> void:
	var heading := _text(title, 14, MenuTheme.ACCENT)
	box.add_child(heading)
	if rows.is_empty():
		return
	var grid := GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override(&"h_separation", 12)
	grid.add_theme_constant_override(&"v_separation", 1)
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(grid)
	for row: Array in rows:
		var label := _text(str(row[0]), 12, MenuTheme.TEXT_DIM)
		label.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		grid.add_child(label)
		var value := _text(str(row[1]), 12, MenuTheme.TEXT)
		value.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
		value.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		value.custom_minimum_size = Vector2(120, 0)
		grid.add_child(value)


## A table: `head` are keys (or finished texts), a cell is a text or
## [text, tone] with the tone `yes`, `no` or `far` (beyond the range).
func _table(box: VBoxContainer, head: Array, rows: Array) -> void:
	if rows.is_empty():
		return
	var scroll := ScrollContainer.new()
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(scroll)
	var grid := GridContainer.new()
	grid.columns = head.size()
	grid.add_theme_constant_override(&"h_separation", 10)
	grid.add_theme_constant_override(&"v_separation", 1)
	scroll.add_child(grid)
	for cell: String in head:
		grid.add_child(_text(cell, 11, MenuTheme.TEXT_DIM))
	for row: Array in rows:
		for i in head.size():
			var cell: Variant = row[i] if i < row.size() else ""
			var tone := ""
			if cell is Array:
				tone = str(cell[1])
				cell = cell[0]
			var color := MenuTheme.TEXT
			if tone == "yes":
				color = MenuTheme.ACCENT
			elif tone == "no" or tone == "far":
				color = MenuTheme.TEXT_DIM
			var label := _text(str(cell), 11, color)
			label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
			grid.add_child(label)
	# The grid's height, plus room for the horizontal scroll bar; again once
	# the labels know their theme.
	var fit := func() -> void:
		if is_instance_valid(scroll) and is_instance_valid(grid):
			scroll.custom_minimum_size.y = grid.get_combined_minimum_size().y + 10.0
	fit.call()
	fit.call_deferred()


## Fine print.
func _note(box: VBoxContainer, text: String) -> void:
	var label := _text(text, 11, MenuTheme.TEXT_DIM)
	label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(200, 0)
	box.add_child(label)
