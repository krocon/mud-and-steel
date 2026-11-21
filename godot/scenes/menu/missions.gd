extends RefCounted
## Missions and the tutorial (prompt 039): finding the mission files,
## starting one, and the player's progress in `user://missions.cfg`.
##
## Mission scripts lie next to their map: `<maps>/<map>/missions/<id>.toml`
## (shipped maps and the player's own maps, see `GamePaths`). Their headers
## come from `SimHost.mission_header`; the format is in `docs/MISSIONS.md`.
## The self-test keeps its progress in its own file.
##
## Texts (prompt 043): mission files are English; `<id>.<lang>.po` next to
## the file translates them (msgid = English text, `just mission-po`).
## `text(path, english)` gives the text in the interface language.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Language := preload("res://scenes/menu/language.gd")
const PROGRESS_PATH := "user://missions.cfg"
const SELF_TEST_PROGRESS_PATH := "user://missions-selftest.cfg"
## Id of the tutorial's mission file.
const TUTORIAL_ID := "tutorial"
## Difficulty levels (prompt 181), easiest first; names of
## `SimHost.start_mission_level`.
const LEVELS: Array[String] = ["easy", "normal", "hard"]
const LEVEL_KEYS := {"easy": "MENU_LEVEL_EASY", "normal": "MENU_LEVEL_NORMAL", "hard": "MENU_LEVEL_HARD"}


## Loaded translations by PO path (null: no file).
static var _translations := {}


## `english` (a text of the mission file at `mission_path`) in the interface
## language; unchanged without a translation.
static func text(mission_path: String, english: String) -> String:
	if Language.current == Language.FALLBACK or english.is_empty() or mission_path.is_empty():
		return english
	var po := "%s.%s.po" % [mission_path.get_basename(), Language.current]
	if not _translations.has(po):
		_translations[po] = load(po) if FileAccess.file_exists(po) else null
	var translation: Translation = _translations[po]
	if translation == null:
		return english
	var translated := String(translation.get_message(english))
	return translated if not translated.is_empty() else english


## Path of the mission file with this id (from `list`), "" if unknown.
static func path_of(id: String) -> String:
	var pending: String = MatchSetup.pending.get("mission", "")
	if pending.get_file().get_basename() == id:
		return pending
	for m in list():
		if m["id"] == id:
			return m["path"]
	return ""


## Every mission header (dictionaries of `SimHost.mission_header` plus
## `path`), tutorial first, then by `order` and title. Broken files are
## included with `ok` false.
static func list() -> Array[Dictionary]:
	var found: Array[Dictionary] = []
	var seen := {}
	for root: String in [GamePaths.user_maps_dir(), GamePaths.maps_dir()]:
		var maps := DirAccess.open(root)
		if maps == null:
			continue
		for map_name in maps.get_directories():
			var folder := root.path_join(map_name).path_join("missions")
			var dir := DirAccess.open(folder)
			if dir == null:
				continue
			for file in dir.get_files():
				if not file.ends_with(".toml"):
					continue
				var key := map_name + "/" + file
				if seen.has(key):
					continue
				seen[key] = true
				var header: Dictionary = SimHost.mission_header(folder.path_join(file))
				header["path"] = folder.path_join(file)
				found.append(header)
	found.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var ta := 0 if a.get("kind", "") == "tutorial" else 1
		var tb := 0 if b.get("kind", "") == "tutorial" else 1
		if ta != tb:
			return ta < tb
		if int(a.get("order", 0)) != int(b.get("order", 0)):
			return int(a.get("order", 0)) < int(b.get("order", 0))
		return String(a.get("title", a["id"])) < String(b.get("title", b["id"])))
	return found


## The tutorial chapters (prompt 180) in their order (`order`).
static func chapters() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for m in list():
		if m.get("kind", "") == "tutorial" and m.get("ok", false):
			out.append(m)
	return out


## The first chapter not won yet, else the first chapter ({} without any):
## what the main menu's Tutorial button starts.
static func next_chapter() -> Dictionary:
	var all := chapters()
	for m in all:
		if not is_done(m["id"]):
			return m
	return all[0] if not all.is_empty() else {}


## The tutorial's header (chapter 1), or {} if it is missing.
static func tutorial() -> Dictionary:
	for m in list():
		if m.get("kind", "") == "tutorial" and m.get("ok", false):
			return m
	return {}


## Starts the mission of `header` (from `list`) through `main.tscn`, at
## `level` (default: the level chosen last, `level()`).
static func start(tree: SceneTree, header: Dictionary, level := "") -> void:
	if not LEVELS.has(level):
		level = level()
	MatchSetup.pending = {
		"map": header["map"],
		"mission": header["path"],
		"biome": header.get("biome", ""),
		"level": level,
	}
	print("Menu: starting mission %s (%s) on %s" % [header["id"], header["path"], level])
	tree.change_scene_to_file(MatchSetup.MAIN_SCENE)


## Starts the mission `id` again from the beginning at `level` (the
## "hopeless" warning, prompt 181). False if it is not found.
static func restart(tree: SceneTree, id: String, level: String) -> bool:
	for m in list():
		if m["id"] == id and m.get("ok", false):
			start(tree, m, level)
			return true
	return false


## The difficulty chosen last (prompt 181), "normal" at first.
static func level() -> String:
	var value := String(_load().get_value("settings", "level", "normal"))
	return value if LEVELS.has(value) else "normal"


## Remembers the chosen difficulty.
static func set_level(level: String) -> void:
	if not LEVELS.has(level):
		return
	var file := _load()
	file.set_value("settings", "level", level)
	_save(file)


## The hardest level the mission `id` was won on, "" if never.
static func best_level(id: String) -> String:
	return String(_load().get_value("level", id, "normal" if is_done(id) else ""))


static func _progress_path() -> String:
	return SELF_TEST_PROGRESS_PATH if MatchSetup.flow_test != null else PROGRESS_PATH


static func _load() -> ConfigFile:
	var file := ConfigFile.new()
	file.load(_progress_path())
	return file


## Whether the mission `id` was won once.
static func is_done(id: String) -> bool:
	return bool(_load().get_value("done", id, false))


## Records a won mission, the hardest level it was won on and the most
## stars per level (prompt 189, section `stars`, key `<id>.<level>`).
static func mark_done(id: String, level := "normal", stars := 0) -> void:
	var file := _load()
	file.set_value("done", id, true)
	var best := String(file.get_value("level", id, ""))
	if LEVELS.find(level) > LEVELS.find(best):
		file.set_value("level", id, level)
	var key := "%s.%s" % [id, level]
	if stars > int(file.get_value("stars", key, -1)):
		file.set_value("stars", key, stars)
	_save(file)


## The most stars the mission `id` was won with at `level`, -1 if never.
static func stars(id: String, level: String) -> int:
	return int(_load().get_value("stars", "%s.%s" % [id, level], -1))


## "★★☆" per level the mission was won on, e.g. "Normal ★★☆", "" if none.
static func stars_text(id: String) -> String:
	var parts: PackedStringArray = []
	for level in LEVELS:
		var n := stars(id, level)
		if n >= 0:
			parts.append("%s %s%s" % [TranslationServer.translate(LEVEL_KEYS[level]), "★".repeat(n), "☆".repeat(3 - n)])
	return "  ".join(parts)


## Every campaign file (`data/campaigns/<id>.toml`, prompt 189) as
## `SimHost.campaign_header` plus `path`, by `order` and title.
static func campaigns() -> Array[Dictionary]:
	var found: Array[Dictionary] = []
	var folder := GamePaths.data_dir().path_join("campaigns")
	var dir := DirAccess.open(folder)
	if dir == null:
		return found
	for file in dir.get_files():
		if not file.ends_with(".toml"):
			continue
		var header: Dictionary = SimHost.campaign_header(folder.path_join(file))
		header["path"] = folder.path_join(file)
		found.append(header)
	found.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		if int(a.get("order", 0)) != int(b.get("order", 0)):
			return int(a.get("order", 0)) < int(b.get("order", 0))
		return String(a.get("title", a["id"])) < String(b.get("title", b["id"])))
	return found


## Whether mission `k` of a campaign (from `campaigns`) can be played: the
## first always, every other once the one before it was won.
static func unlocked(campaign: Dictionary, k: int) -> bool:
	var missions: Array = campaign.get("missions", [])
	return k == 0 or (k > 0 and k < missions.size() + 1 and is_done(missions[k - 1]["id"]))


static func _save(file: ConfigFile) -> void:
	var error := file.save(_progress_path())
	if error != OK:
		push_error("cannot save mission progress: %s" % error_string(error))


## Forgets all progress (self-test).
static func reset() -> void:
	DirAccess.remove_absolute(ProjectSettings.globalize_path(_progress_path()))
