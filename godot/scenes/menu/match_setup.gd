extends RefCounted
## Hands the single-player setup from the menu to `main.tscn` (prompt 027).
##
## Static, so it survives the scene change, like `lan/lan_match.gd`.
## `main.gd` takes `pending` and clears it.

const MENU_SCENE := "res://scenes/menu/main_menu.tscn"
const MAIN_SCENE := "res://scenes/main.tscn"
## Map folders, relative to the Godot project (repository checkout only).
const MAPS_DIR := "res://../data/maps"
## Name of the map compiled into the simulation.
const BUILTIN_MAP := "test_small"

## Faction ids of the catalog with their display names. Names only: no
## insignia anywhere in the menus.
const FACTIONS: Dictionary[String, String] = {
	"de": "Germany",
	"su": "Soviet Union",
	"us": "United States",
}

## Computer opponents (prompt 024): difficulty id (`SimHost.ai_opponent`)
## with its display name. "passive" gives no orders: its tanks only fire.
const OPPONENTS: Dictionary[String, String] = {
	"passive": "Passive (holds its ground)",
	"easy": "Easy",
	"normal": "Normal",
	"hard": "Hard",
}
const DEFAULT_OPPONENT := "normal"

## The match to start: `map` (folder name in `data/maps/`, or BUILTIN_MAP for
## the compiled-in map), `biome` ("" = the map's), `factions` (faction id per
## player, local player first), `ai` (difficulty of the computer opponent,
## see OPPONENTS). Empty when nothing is pending.
static var pending := {}
## The menu self-test that drives the whole flow, or null. While it runs,
## `main.gd` adds the match self-test only when `run_match_test` is set.
static var flow_test: Node = null
static var run_match_test := false
## Steps the last loading screen showed, for the self-test.
static var last_loading_steps: PackedStringArray = []


## Map folders found in `data/maps/` (sorted), the built-in map first.
static func available_maps() -> PackedStringArray:
	var maps: PackedStringArray = [BUILTIN_MAP]
	var dir := DirAccess.open(ProjectSettings.globalize_path(MAPS_DIR))
	if dir == null:
		return maps
	var names := dir.get_directories()
	names.sort()
	for map_name in names:
		if map_name != BUILTIN_MAP and FileAccess.file_exists(
			ProjectSettings.globalize_path(MAPS_DIR.path_join(map_name).path_join("map.toml"))
		):
			maps.append(map_name)
	return maps


## Absolute folder of `map`, or "" for the built-in map.
static func map_dir(map_name: String) -> String:
	if map_name == BUILTIN_MAP or map_name.is_empty():
		return ""
	return ProjectSettings.globalize_path(MAPS_DIR.path_join(map_name)).simplify_path()


## Size and biome of a map from its `map.toml`, for the setup screen.
static func describe_map(map_name: String) -> String:
	var path := ProjectSettings.globalize_path(MAPS_DIR.path_join(map_name).path_join("map.toml"))
	var text := FileAccess.get_file_as_string(path)
	if text.is_empty():
		return ""
	var samples := 0
	var cell_mm := 0
	var biome := ""
	for line in text.split("\n"):
		var parts := line.split("=", true, 1)
		if parts.size() != 2:
			continue
		if line.begins_with("["):
			break
		var key := parts[0].strip_edges()
		var value := parts[1].strip_edges().trim_prefix("\"").trim_suffix("\"")
		match key:
			"samples":
				samples = int(value.trim_prefix("[").split(",")[0])
			"cell_size_mm":
				cell_mm = int(value)
			"biome":
				biome = value
	var size := ""
	if samples > 1 and cell_mm > 0:
		var meters := (samples - 1) * cell_mm / 1000
		size = "%d m × %d m" % [meters, meters] if meters < 1000 else "%.1f km × %.1f km" % [meters / 1000.0, meters / 1000.0]
	return "%s%s" % [size, (", " + biome) if not biome.is_empty() else ""]


## Whether `map` is large enough that loading takes a while.
static func is_large(map_name: String) -> bool:
	return map_name != BUILTIN_MAP and describe_map(map_name).contains("km")
