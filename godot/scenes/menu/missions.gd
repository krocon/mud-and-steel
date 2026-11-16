extends RefCounted
## Missions and the tutorial (prompt 039): finding the mission files,
## starting one, and the player's progress in `user://missions.cfg`.
##
## Mission scripts lie next to their map: `<maps>/<map>/missions/<id>.toml`
## (shipped maps and the player's own maps, see `GamePaths`). Their headers
## come from `SimHost.mission_header`; the format is in `docs/MISSIONS.md`.
## The self-test keeps its progress in its own file.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const PROGRESS_PATH := "user://missions.cfg"
const SELF_TEST_PROGRESS_PATH := "user://missions-selftest.cfg"
## Id of the tutorial's mission file.
const TUTORIAL_ID := "tutorial"


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


## The tutorial's header, or {} if it is missing.
static func tutorial() -> Dictionary:
	for m in list():
		if m.get("kind", "") == "tutorial" and m.get("ok", false):
			return m
	return {}


## Starts the mission of `header` (from `list`) through `main.tscn`.
static func start(tree: SceneTree, header: Dictionary) -> void:
	MatchSetup.pending = {
		"map": header["map"],
		"mission": header["path"],
		"biome": header.get("biome", ""),
	}
	print("Menu: starting mission %s (%s)" % [header["id"], header["path"]])
	tree.change_scene_to_file(MatchSetup.MAIN_SCENE)


static func _progress_path() -> String:
	return SELF_TEST_PROGRESS_PATH if MatchSetup.flow_test != null else PROGRESS_PATH


static func _load() -> ConfigFile:
	var file := ConfigFile.new()
	file.load(_progress_path())
	return file


## Whether the mission `id` was won once.
static func is_done(id: String) -> bool:
	return bool(_load().get_value("done", id, false))


## Records a won mission.
static func mark_done(id: String) -> void:
	var file := _load()
	file.set_value("done", id, true)
	var error := file.save(_progress_path())
	if error != OK:
		push_error("cannot save mission progress: %s" % error_string(error))


## Forgets all progress (self-test).
static func reset() -> void:
	DirAccess.remove_absolute(ProjectSettings.globalize_path(_progress_path()))
