extends RefCounted
## Which sound class a weapon, an engine or an impact uses (prompt 029).
##
## Derived from the catalog facts of a unit type (`SimHost.unit_type_sound`:
## category, mass, weapons with caliber and rate of fire), never per unit:
##
## | Weapon | Class |
## |---|---|
## | caliber < 20 mm, rate < 300 rpm | `shot_rifle` (bolt-action and semi-automatic rifles, pistols) |
## | caliber < 20 mm, rate ≥ 300 rpm | `shot_mg` (machine guns, submachine guns) |
## | from 20 mm in an anti-tank squad (`inf_at`: Panzerschreck, Panzerfaust, Bazooka) | `shot_rocket` |
## | mortar (category `inf_mortar_*` / `mortar_heavy`, or "mortar" / "GrW" in the name) | `shot_mortar` |
## | howitzer (category `gun_howitzer`, or `self_propelled` from 100 mm) | `shot_howitzer` |
## | indirect fire of a `rocket_launcher` (prompt 062) | `shot_rocket` |
## | 20–36 mm | `shot_cannon_small` |
## | 37–74 mm | `shot_cannon_medium` |
## | from 75 mm | `shot_cannon_large` |
##
## Engines: `truck` and `recon_vehicle` drive on wheels (`engine_wheel`),
## `halftrack` → `engine_halftrack`, every other vehicle has tracks:
## `engine_track_light` below `HEAVY_TRACK_KG`, else `engine_track_heavy`;
## tracked vehicles add the `tracks` loop. Infantry, towed guns, buildings
## and fortifications have no engine.

const SMALL_ARMS_MM := 20.0
const AUTOMATIC_RPM := 300
const MEDIUM_CANNON_MM := 37.0
const LARGE_CANNON_MM := 75.0
const SP_HOWITZER_MM := 100.0
## Tracked vehicles from this combat mass sound heavy (Panzer IV, T-34,
## Sherman and up).
const HEAVY_TRACK_KG := 20000
const WHEELED: Array[String] = ["truck", "recon_vehicle"]
const TRACKED: Array[String] = [
	"tank_light", "tank_medium", "tank_heavy", "tank_destroyer", "assault_gun", "self_propelled",
]
const MORTAR_WORDS: Array[String] = ["mortar", "granatwerfer", "grw"]
## Signals (prompt 192): sirens, alarms, horns, signal whistles. They play
## through the one signal player of `ui_sound.gd`, at most one at a time in
## the whole game; the rocket strike's generated siren (`strike_view.gd`)
## joins them through `UiSound.hold_signal`.
const SIGNAL_CLASSES: Array[String] = ["ui_under_attack"]


## Whether `sound_class` is a signal (see SIGNAL_CLASSES).
static func is_signal(sound_class: String) -> bool:
	return SIGNAL_CLASSES.has(sound_class)


## Sound class of weapon `weapon` (`{name, caliber_mm, rate_rpm}`) of a
## unit type with catalog `category`.
static func shot_class(category: String, weapon: Dictionary) -> String:
	var caliber: float = weapon.get("caliber_mm", 0.0)
	var rate: int = weapon.get("rate_rpm", 0)
	var name := str(weapon.get("name", "")).to_lower()
	if caliber < SMALL_ARMS_MM:
		return "shot_mg" if rate >= AUTOMATIC_RPM else "shot_rifle"
	if category == "inf_at":
		return "shot_rocket"
	if category.begins_with("inf_mortar") or category == "mortar_heavy" or _has_any(name, MORTAR_WORDS):
		return "shot_mortar"
	if category == "gun_howitzer" or (category == "self_propelled" and caliber >= SP_HOWITZER_MM):
		return "shot_howitzer"
	return shot_class_by_caliber(caliber)


## Shells of indirect fire (prompt 032: shot events with weapon 255);
## rocket launchers (prompt 062) launch with `shot_rocket`.
static func indirect_shot_class(category: String) -> String:
	if category == "rocket_launcher":
		return "shot_rocket"
	return "shot_mortar" if category.begins_with("inf_mortar") or category == "mortar_heavy" else "shot_howitzer"


## Fallback without catalog facts: by caliber only.
static func shot_class_by_caliber(caliber: float) -> String:
	if caliber < SMALL_ARMS_MM:
		return "shot_rifle"
	if caliber < MEDIUM_CANNON_MM:
		return "shot_cannon_small"
	if caliber < LARGE_CANNON_MM:
		return "shot_cannon_medium"
	return "shot_cannon_large"


## Engine loop of a unit type, "" for none.
static func engine_class(category: String, mass_kg: int) -> String:
	if category in WHEELED:
		return "engine_wheel"
	if category == "halftrack":
		return "engine_halftrack"
	if category in TRACKED:
		return "engine_track_heavy" if mass_kg >= HEAVY_TRACK_KG else "engine_track_light"
	return ""


## Whether a unit type runs on tracks (adds the `tracks` loop).
static func has_tracks(category: String) -> bool:
	return category in TRACKED


## Sound class and extra gain (dB) of a hit event (`SimHost.take_combat_events`).
## `target_infantry`: the hit unit is a squad; `water`: the ground hit is in
## water. Returns ["", 0] for no sound.
static func hit_class(event: Dictionary, target_infantry: bool, water: bool) -> Array:
	var caliber: float = event.get("caliber_mm", 0.0)
	var gain := caliber_gain_db(caliber)
	match str(event.get("surface", "")):
		"ground":
			if caliber < SMALL_ARMS_MM:
				return ["hit_bullet_ground", 0.0]
			if water:
				return ["hit_shell_water", gain]
			return ["hit_shell_small" if caliber < MEDIUM_CANNON_MM else "hit_shell", gain]
		"unit":
			if target_infantry:
				if caliber < SMALL_ARMS_MM:
					return ["", 0.0]
				return ["hit_shell_small" if caliber < MEDIUM_CANNON_MM else "hit_shell", gain]
			if caliber < SMALL_ARMS_MM:
				return ["hit_bullet_armor", 0.0]
			return ["hit_penetration" if event.get("penetrated", false) else "hit_ricochet", gain]
		"object":
			if caliber < SMALL_ARMS_MM:
				return ["hit_bullet_ground", -3.0]
			return ["hit_object", gain]
	return ["", 0.0]


## Sound class of a map object stage event (`SimHost.take_object_events`).
static func object_class(event: Dictionary) -> String:
	if event.get("stage", "") == "damaged":
		# A tree stripped by a big blast (prompt 085) is heard in the blast.
		if str(event.get("remains", "")) == "fallen":
			return ""
		return "building_damage"
	match str(event.get("remains", "")):
		"ruin":
			return "building_collapse"
		"fallen":
			return "tree_fall"
		"gone":
			return "wall_break"
		"flat":
			return "fence_crush"
	return ""


## Louder for larger shells: ±6 dB from 37 to 150 mm around 75 mm.
static func caliber_gain_db(caliber: float) -> float:
	if caliber < SMALL_ARMS_MM:
		return 0.0
	return clampf(20.0 * log(clampf(caliber / 75.0, 0.5, 2.0)) / log(10.0), -6.0, 6.0)


## Folder of real recordings of a catalog weapon (prompt 053,
## `assets/audio/weapon/<slug>/`): the name without parentheses, lower
## case, every run of other characters one `_` ("MG 42" → `mg_42`,
## "Browning M2HB (AA)" → `browning_m2hb`). Same rule as
## `tools/audio/synth.py --weapons`.
static func weapon_slug(weapon_name: String) -> String:
	var text := weapon_name
	while text.contains("(") and text.find(")", text.find("(")) > 0:
		var open := text.find("(")
		text = text.substr(0, open) + " " + text.substr(text.find(")", open) + 1)
	var out := ""
	var gap := false
	for c in text.to_lower():
		var code := c.unicode_at(0)
		if (code >= 48 and code <= 57) or (code >= 97 and code <= 122):
			if gap and not out.is_empty():
				out += "_"
			out += c
			gap = false
		else:
			gap = true
	return out


## Pitch of a blast by caliber (prompt 053): larger charges sound lower,
## 75 mm = 1.0, 150 mm ≈ 0.87, 37 mm ≈ 1.12 (0.75 .. 1.2).
static func blast_pitch(caliber: float) -> float:
	if caliber < SMALL_ARMS_MM:
		return 1.0
	return clampf(pow(75.0 / caliber, 0.2), 0.75, 1.2)


## Level of the long `explosion_tail` of a blast (dB), or NAN for none:
## shells from `TAIL_MM`, louder with caliber.
const TAIL_MM := 100.0
static func blast_tail_db(caliber: float) -> float:
	if caliber < TAIL_MM:
		return NAN
	return clampf(20.0 * log(caliber / 150.0) / log(10.0), -6.0, 9.0)


static func _has_any(text: String, words: Array[String]) -> bool:
	for word in words:
		if text.contains(word):
			return true
	return false
