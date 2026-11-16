extends RefCounted
## Game values of unit types for the interface (prompt 066): the picker
## tiles, their tooltip, the unit panel and the unit card.
##
## Display only. The numbers come from `SimHost.unit_stats` (the card of
## `sim::unit_stats`, the same one `cli unit-stats` prints for the unit
## page); nothing here computes a game value. The bridge computes a card on
## every call, so each type is fetched once and kept (`fetches` counts the
## calls for the self-test). Texts are built in the interface language:
## numbers get its decimal and thousands signs (`num`).
##
## The words of the tooltip ("strong", "medium", "weak", "none") and its
## "good against … / vulnerable to …" line are read off the card: the
## ratings, which reference tanks the strongest gun penetrates from the
## front or the side (`against_vehicles.targets`) and the weakest comparison weapon
## that gets through the hull (`protection.versus`).

const Language := preload("res://scenes/menu/language.gd")

## The five ratings in display order.
const RATINGS: Array[String] = ["speed", "protection", "anti_vehicle", "anti_infantry", "anti_structure"]
const RATING_NAMES: Dictionary[String, String] = {
	"speed": "UNIT_RATING_SPEED",
	"protection": "UNIT_RATING_PROTECTION",
	"anti_vehicle": "UNIT_RATING_ANTI_VEHICLE",
	"anti_infantry": "UNIT_RATING_ANTI_INFANTRY",
	"anti_structure": "UNIT_RATING_ANTI_STRUCTURE",
}
## A rating below `WEAK_BELOW` reads "weak", from `STRONG_FROM` "strong",
## 0 or absent "none".
const WEAK_BELOW := 25
const STRONG_FROM := 60
## Lines of values in the tooltip at most (prompt 066).
const MAX_TIP_LINES := 8

## Bridge calls of `unit_stats` so far (once per type).
static var fetches := 0
static var _cards: Dictionary[String, Dictionary] = {}
static var _records: Dictionary[String, Dictionary] = {}
static var _infos: Dictionary[String, Dictionary] = {}
static var _reference: Dictionary = {}
static var _buildable: Dictionary[String, bool] = {}


## The card of game values of `id` (empty for base buildings and unknown
## ids).
static func card(id: String) -> Dictionary:
	if not _cards.has(id):
		fetches += 1
		_cards[id] = SimHost.unit_stats(id)
	return _cards[id]


## The ratings of `id`: rating name -> 0…100, only those that apply.
static func ratings(id: String) -> Dictionary:
	return card(id).get("ratings", {})


## What the cards compare with (`SimHost.unit_stats_reference`).
static func comparisons() -> Dictionary:
	if _reference.is_empty():
		_reference = SimHost.unit_stats_reference()
	return _reference


## The entry of the vehicle database of `id` (empty without one).
static func record(id: String) -> Dictionary:
	if not _records.has(id):
		_records[id] = SimHost.vehicle_record(id)
	return _records[id]


## Catalog values of `id` (`SimHost.catalog_card`) with the name in the
## interface language.
static func info(id: String) -> Dictionary:
	var key := TranslationServer.get_locale() + ":" + id
	if not _infos.has(key):
		_infos[key] = SimHost.catalog_card(id)
	return _infos[key]


## The name of `id` in the interface language (the id if unknown).
static func name_of(id: String) -> String:
	return str(info(id).get("name", id))


## Whether a player can build or produce `id`.
static func buildable(id: String) -> bool:
	if _buildable.is_empty():
		for other in SimHost.buildable_ids():
			_buildable[other] = true
	return _buildable.has(id)


## `value` with `digits` decimals in the interface language: German
## "1.234,5", English "1,234.5".
static func num(value: float, digits := 0) -> String:
	var german := TranslationServer.get_locale().begins_with("de")
	var text := "%.*f" % [digits, absf(value)]
	var whole := text.get_slice(".", 0)
	var grouped := ""
	while whole.length() > 3:
		grouped = ("." if german else ",") + whole.right(3) + grouped
		whole = whole.left(whole.length() - 3)
	grouped = whole + grouped
	if digits > 0:
		grouped += ("," if german else ".") + text.get_slice(".", 1)
	return ("-" if value < 0.0 and float(text) != 0.0 else "") + grouped


## A value in tenths (`_x10`): one decimal only where there is one, or
## exactly `digits` decimals.
static func tenths(value: int, digits := -1) -> String:
	return num(value / 10.0, digits if digits >= 0 else (1 if value % 10 != 0 else 0))


## `value` with its unit sign.
static func with_unit(value: String, sign: String) -> String:
	return value + (sign if sign in ["°", "%"] else " " + sign)


## Translation key of the word for rating `rating` of `card`.
static func strength_key(stats: Dictionary, rating: String) -> String:
	var value := int((stats.get("ratings", {}) as Dictionary).get(rating, 0))
	if value <= 0:
		return "UNIT_WORD_NONE"
	if value < WEAK_BELOW:
		return "UNIT_WORD_WEAK"
	return "UNIT_WORD_MEDIUM" if value < STRONG_FROM else "UNIT_WORD_STRONG"


## The ratings as one line of text ("Speed 50 · Protection 37 …").
static func ratings_line(id: String) -> String:
	var values := ratings(id)
	var parts: Array[String] = []
	for rating in RATINGS:
		if values.has(rating):
			parts.append("%s %d" % [Language.t(RATING_NAMES[rating]), int(values[rating])])
	return " · ".join(parts)


## Name, designation and first year of `id` ("Panzer IV H (Pz.Kpfw. IV
## Ausf. H), from 1943").
static func title_line(id: String, shown_name := "") -> String:
	var data := info(id)
	var title := shown_name if not shown_name.is_empty() else str(data.get("name", id))
	var designation := str(data.get("designation", ""))
	if not designation.is_empty() and not title.contains(designation):
		title += "  (%s)" % designation
	if int(data.get("year", 0)) > 0:
		title += ", " + Language.t("HUD_FROM_YEAR") % int(data.year)
	return title


## The values a tooltip lists of `id`, at most `MAX_TIP_LINES` lines:
## speed, armor, main weapon, effect against infantry and structures in a
## word, men and seats.
static func summary_lines(id: String) -> Array[String]:
	var stats := card(id)
	var lines: Array[String] = []
	var movement: Dictionary = stats.get("movement", {})
	if movement.has("road_real_kmh_x10"):
		lines.append(Language.t("UNIT_TIP_SPEED").format({
			"road": tenths(int(movement.road_real_kmh_x10)), "offroad": tenths(int(movement.get("offroad_real_kmh_x10", 0)))}))
	elif movement.has("max_real_kmh_x10"):
		lines.append(Language.t("UNIT_TIP_SPEED_AIR").format({"speed": tenths(int(movement.max_real_kmh_x10))}))
	elif movement.has("road_game_mps_x10"):
		lines.append(Language.t("UNIT_TIP_SPEED_GAME").format({"speed": tenths(int(movement.road_game_mps_x10), 1)}))
	var protection: Dictionary = stats.get("protection", {})
	if protection.has("armor_hull_mm"):
		var hull: Array = protection.armor_hull_mm
		var armor := Language.t("HUD_ARMOR_HULL") % [hull[0], hull[1], hull[2]]
		if protection.has("armor_turret_mm"):
			var turret: Array = protection.armor_turret_mm
			armor += ", " + Language.t("HUD_ARMOR_TURRET") % [turret[0], turret[1], turret[2]]
		lines.append(armor + " " + Language.t("HUD_ARMOR_SIDES"))
	elif protection.get("unarmored", false):
		lines.append(Language.t("UNIT_TIP_UNARMORED").format({"hp": num(int(protection.get("hp", 0)))}))
	var weapon := _weapon_line(stats)
	if not weapon.is_empty():
		lines.append(weapon)
	if stats.has("indirect"):
		var indirect: Dictionary = stats.indirect
		lines.append(Language.t("UNIT_TIP_INDIRECT").format({"weapon": indirect.name,
				"min": num(int(indirect.min_range_game_m)), "max": num(int(indirect.range_game_m))}))
	var values: Dictionary = stats.get("ratings", {})
	if values.has("anti_infantry") or values.has("anti_structure"):
		lines.append(Language.t("UNIT_TIP_EFFECT").format({
			"infantry": Language.t(strength_key(stats, "anti_infantry")),
			"structures": Language.t(strength_key(stats, "anti_structure"))}))
	var other: Dictionary = stats.get("other", {})
	var men: Array[String] = []
	if other.has("squad_size"):
		men.append(Language.t("HUD_SQUAD_OF") % int(other.squad_size))
	elif other.has("crew"):
		men.append(Language.t("HUD_CREW") % int(other.crew))
	if other.has("passengers"):
		men.append(Language.t("HUD_SEATS") % int(other.passengers))
	if not men.is_empty():
		lines.append(" · ".join(men))
	return lines.slice(0, MAX_TIP_LINES)


## The main weapon: the strongest gun with its penetration at the duel
## distance and its range in the game, else the weapon that reaches
## furthest.
static func _weapon_line(stats: Dictionary) -> String:
	var weapons: Array = stats.get("weapons", [])
	if stats.has("against_vehicles"):
		var duel: Dictionary = stats.against_vehicles
		var reach := 0
		for w: Dictionary in weapons:
			if w.name == duel.weapon:
				reach = int(w.range_game_m)
		return Language.t("UNIT_TIP_GUN").format({"weapon": duel.weapon, "mm": int(duel.penetration_mm),
				"distance": num(int(duel.distance_real_m)), "range": num(reach)})
	var best := {}
	for w: Dictionary in weapons:
		if best.is_empty() or int(w.range_game_m) > int(best.range_game_m):
			best = w
	if best.is_empty():
		return ""
	return Language.t("UNIT_TIP_WEAPON").format({"weapon": best.name, "range": num(int(best.range_game_m))})


## "Good against … / vulnerable to …" of `id`, from the card's comparison
## data ("" if it says neither).
static func matchup_line(id: String) -> String:
	var parts: Array[String] = []
	var good := good_against(id)
	if not good.is_empty():
		parts.append(Language.t("UNIT_TIP_GOOD") % ", ".join(good))
	var weak := vulnerable_to(id)
	if not weak.is_empty():
		parts.append(Language.t("UNIT_TIP_VULNERABLE") % ", ".join(weak))
	return " / ".join(parts)


## What `id` is good against: the classes of reference tanks whose hull
## front its strongest gun penetrates at the duel distance (all tanks of
## the class), those it only gets through from the side, infantry and
## structures where the rating is strong.
static func good_against(id: String) -> Array[String]:
	var stats := card(id)
	var out: Array[String] = []
	if stats.has("against_vehicles"):
		# Class -> [targets, penetrated from the front, from the side].
		var classes := {}
		for target: Dictionary in stats.against_vehicles.targets:
			var category := str(card(str(target.id)).get("category", ""))
			var count: Array = classes.get(category, [0, 0, 0])
			count[0] += 1
			count[1] += 1 if target.hull_front else 0
			count[2] += 1 if target.hull_side else 0
			classes[category] = count
		var frontal: Array[String] = []
		var flanks: Array[String] = []
		for category: String in classes:
			if classes[category][1] == classes[category][0]:
				frontal.append(Language.code_text("UNIT_VS_", category))
			elif classes[category][2] == classes[category][0]:
				flanks.append(Language.code_text("UNIT_VS_", category))
		if frontal.size() == classes.size() and not classes.is_empty():
			out.append(Language.t("UNIT_VS_ALL_TANKS"))
		else:
			out.append_array(frontal)
			if flanks.size() == classes.size():
				out.append(Language.t("UNIT_VS_SIDE") % Language.t("UNIT_VS_ALL_TANKS"))
			elif not flanks.is_empty():
				out.append(Language.t("UNIT_VS_SIDE") % ", ".join(flanks))
	var values: Dictionary = stats.get("ratings", {})
	if int(values.get("anti_infantry", 0)) >= STRONG_FROM:
		out.append(Language.t("UNIT_VS_INFANTRY"))
	if int(values.get("anti_structure", 0)) >= STRONG_FROM:
		out.append(Language.t("UNIT_VS_STRUCTURES"))
	return out


## What `id` is vulnerable to: the weakest comparison weapon that gets
## through its hull front and its side, small arms and shell fragments.
static func vulnerable_to(id: String) -> Array[String]:
	var protection: Dictionary = card(id).get("protection", {})
	var out: Array[String] = []
	if protection.is_empty():
		return out
	if protection.has("soldiers"):
		out.append(Language.t("UNIT_BY_SMALL_ARMS"))
	elif protection.get("unarmored", false):
		out.append(Language.t("UNIT_BY_ANY_WEAPON"))
	else:
		var front := ""
		var side := ""
		for versus: Dictionary in protection.get("versus", []):
			if front.is_empty() and int(versus.get("hull_front_real_m", -1)) >= 0:
				front = str(versus.weapon)
			if side.is_empty() and int(versus.get("hull_side_real_m", -1)) >= 0:
				side = str(versus.weapon)
		if not front.is_empty():
			out.append(Language.t("UNIT_BY_FRONT" if side != front else "UNIT_BY_FROM") % weapon_class(front))
		if not side.is_empty() and side != front:
			out.append(Language.t("UNIT_BY_SIDE") % weapon_class(side))
	if protection.get("hurt_by_fragments", false) or protection.has("soldiers"):
		out.append(Language.t("UNIT_BY_FRAGMENTS"))
	return out


## Name of a comparison weapon class (`rifle`, `gun_75mm`, …).
static func weapon_class(code: String) -> String:
	return Language.code_text("UNIT_WEAPON_", code)


## Service years of `id` ("1943–1945"), "" if unknown.
static func years(id: String) -> String:
	var data := info(id)
	var from := int(data.get("year", 0))
	var to := int(data.get("year_to", 0))
	if from <= 0:
		return ""
	return str(from) if to <= from else "%d–%d" % [from, to]
