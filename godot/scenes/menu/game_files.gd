extends RefCounted
## Saved games and replays on disk (prompt 038): listing, describing,
## writing, and starting them in `main.tscn`.
##
## Saved games live in `user://saves/<date-time>.mssave`, replays in
## `user://replays/<date-time>.msreplay`. The menu hands a chosen file over
## through `MatchSetup.pending` (`load` = path of a saved game, `replay` =
## an opened `ReplayPlayer` with `replay_tick`, `replay_view`,
## `replay_speed`); `main.gd` calls `start` instead of starting a new match.
## Static, so the open replay survives the scene rebuilds of a jump.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const SAVE_DIR := "user://saves"
const REPLAY_DIR := "user://replays"
const SAVE_EXTENSION := "mssave"
const REPLAY_EXTENSION := "msreplay"
## Playback speeds of the replay viewer.
const SPEEDS: Array[float] = [1.0, 2.0, 4.0, 8.0]

## The replay being watched, kept across the scene rebuilds of jumps.
static var replay_player: ReplayPlayer = null
## Path of that replay.
static var replay_path := ""
## Speed and pause state of the replay viewer, kept across rebuilds.
static var replay_speed := 1.0
static var replay_paused := false
## Absolute path of the last replay this process wrote ("" if none).
static var last_replay_path := ""


## Local date and time as stored in the files ("2026-11-15 21:30:05").
static func now_text() -> String:
	return Time.get_datetime_string_from_system(false, true)


## Files of `dir` with `extension`, newest first: dictionaries with `path`
## (absolute) and `info` (`ReplayPlayer.read_*_info`).
static func list(dir: String, extension: String) -> Array[Dictionary]:
	var entries: Array[Dictionary] = []
	var access := DirAccess.open(dir)
	if access == null:
		return entries
	for file_name in access.get_files():
		if file_name.get_extension() != extension:
			continue
		var path := ProjectSettings.globalize_path(dir.path_join(file_name))
		var bytes := FileAccess.get_file_as_bytes(path)
		var info: Dictionary = ReplayPlayer.read_save_info(bytes) if extension == SAVE_EXTENSION else ReplayPlayer.read_replay_info(bytes)
		entries.append({"path": path, "name": file_name, "info": info, "modified": FileAccess.get_modified_time(path)})
	entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return a["modified"] > b["modified"] if a["modified"] != b["modified"] else a["name"] > b["name"])
	return entries


## One line for a list: date, map, players, duration, result.
static func describe(entry: Dictionary, saved_game := false) -> String:
	var info: Dictionary = entry["info"]
	if not info.get("ok", false):
		return "%s – unreadable: %s" % [entry["name"], info.get("error", "")]
	var created: String = info["created"] if not String(info["created"]).is_empty() else String(entry["name"]).get_basename()
	var seconds: int = info["seconds"]
	var parts: PackedStringArray = [
		created,
		info["map"],
		players_text(info),
		"%d:%02d" % [seconds / 60, seconds % 60],
	]
	if not saved_game:
		parts.append(result_text(info))
	var line := " · ".join(parts)
	if not info["playable"]:
		line += "  [incompatible]"
	return line


## "Germany (you) vs Soviet Union (computer, hard)".
static func players_text(info: Dictionary) -> String:
	var names: PackedStringArray = []
	for player: Dictionary in info["players"]:
		var faction: String = MatchSetup.FACTIONS.get(player["faction"], "Player %d" % player["id"])
		var kind: String = player["kind"]
		if kind == "human":
			faction += " (player)" if info["multiplayer"] else " (you)"
		elif kind.begins_with("ai:"):
			faction += " (computer, %s)" % kind.trim_prefix("ai:")
		names.append(faction)
	return " vs ".join(names)


## Who won, from the replay info.
static func result_text(info: Dictionary) -> String:
	if not info["decided"]:
		return "not decided"
	var winner: int = info["winner"]
	if winner < 0:
		return "draw"
	for player: Dictionary in info["players"]:
		if player["id"] == winner:
			return "%s won" % MatchSetup.FACTIONS.get(player["faction"], "Player %d" % winner)
	return "player %d won" % winner


## Writes the running match as a saved game; returns the absolute path or
## "" (cannot be saved, or the file cannot be written).
static func save_game(sim_host: SimHost, settings: Dictionary) -> String:
	var bytes := sim_host.save_game_bytes(now_text(), settings)
	if bytes.is_empty():
		return ""
	DirAccess.make_dir_recursive_absolute(SAVE_DIR)
	var stamp := Time.get_datetime_string_from_system().replace(":", "-")
	var path := unique_path(SAVE_DIR, stamp, SAVE_EXTENSION)
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("cannot write saved game %s: %s" % [path, error_string(FileAccess.get_open_error())])
		return ""
	file.store_buffer(bytes)
	file.close()
	var absolute := ProjectSettings.globalize_path(path)
	print("game saved: %s (tick %d)" % [absolute, sim_host.current_tick()])
	return absolute


## `dir/base.extension`, or with a suffix if that exists: several games
## (also other running instances) may write in the same second.
static func unique_path(dir: String, base: String, extension: String) -> String:
	var path := "%s/%s.%s" % [dir, base, extension]
	if not FileAccess.file_exists(path):
		return path
	var suffix := 2
	while true:
		path = "%s/%s-%d-%d.%s" % [dir, base, OS.get_process_id(), suffix, extension]
		if not FileAccess.file_exists(path):
			return path
		suffix += 1
	return path


## Opens a replay for watching; returns "" or why it cannot be played.
static func open_replay(path: String) -> String:
	var player := ReplayPlayer.new()
	var bytes := FileAccess.get_file_as_bytes(path)
	var error := player.open(bytes, maps_dir(ReplayPlayer.read_replay_info(bytes).get("map", "")))
	if not error.is_empty():
		return error
	replay_player = player
	replay_path = path
	replay_speed = 1.0
	replay_paused = false
	return ""


## Hands the open replay to `main.tscn`, from `tick`, seen by `view` (-1:
## everything visible).
static func watch_replay(tick: int, view: int) -> void:
	MatchSetup.pending = {
		"replay": replay_player,
		"replay_tick": tick,
		"replay_view": view,
		"map": replay_player.info()["map"],
	}


## Absolute folder that holds the map `map_name` (shipped or the player's
## own, `GamePaths`); the bridge loads `<folder>/<map_name>/`.
static func maps_dir(map_name: String) -> String:
	if map_name.is_empty() or map_name == MatchSetup.BUILTIN_MAP:
		return GamePaths.maps_dir()
	return GamePaths.map_dir(map_name).get_base_dir()


## Whether `main.gd` should call `start` for the pending setup.
static func is_pending(setup: Dictionary) -> bool:
	return setup.has("load") or setup.has("replay")


## Starts the saved game or replay of `setup` on `sim_host`.
static func start(sim_host: SimHost, setup: Dictionary) -> bool:
	if setup.has("load"):
		var bytes := FileAccess.get_file_as_bytes(setup["load"])
		var ok := sim_host.load_game(bytes, maps_dir(ReplayPlayer.read_save_info(bytes).get("map", "")))
		if ok:
			# The biome of the saved match (main.gd reads it after the start).
			setup["biome"] = sim_host.loaded_settings().get("biome", "")
		return ok
	var ok := sim_host.start_replay(setup["replay"], setup["replay_tick"], setup["replay_view"])
	if ok:
		sim_host.speed = 0.0 if replay_paused else replay_speed
	return ok
