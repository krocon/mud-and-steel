class_name GamePaths
## Where the game finds the files the simulation reads from disk (prompt 044).
##
## In the repository (`just run`, the editor binary) the maps and scenarios
## are read from `data/` next to `godot/`, as before. An exported game has no
## repository: the release script (`tools/release/export.sh`) ships `data/maps`
## and `data/scenarios` next to the executable (macOS: in
## `Contents/Resources/data` of the app bundle). `res://../data` does not work
## there: Godot resolves it relative to the working directory.
##
## Maps the player makes in the map editor go to `user://maps` in an exported
## game, because the install folder may be read-only (and the macOS bundle is
## signed). In the repository both are `data/maps`.


## True in an exported game (export template), false when run from the repository.
static func is_exported() -> bool:
	return OS.has_feature("template")


## Absolute path of the shipped `data/` folder.
static func data_dir() -> String:
	if not is_exported():
		return ProjectSettings.globalize_path("res://../data").simplify_path()
	var exe_dir := OS.get_executable_path().get_base_dir()
	if OS.get_name() == "macOS":
		return exe_dir.path_join("../Resources/data").simplify_path()
	return exe_dir.path_join("data")


## Absolute path of the shipped maps (read-only in an exported game).
static func maps_dir() -> String:
	return data_dir().path_join("maps")


## Absolute path of the player's own maps (written by the map editor).
static func user_maps_dir() -> String:
	if not is_exported():
		return maps_dir()
	return ProjectSettings.globalize_path("user://maps")


## Absolute folder of the map `map_name`: the player's own map of that name
## if there is one, else the shipped one.
static func map_dir(map_name: String) -> String:
	var own := user_maps_dir().path_join(map_name)
	if FileAccess.file_exists(own.path_join("map.toml")):
		return own
	return maps_dir().path_join(map_name)


## Names of all map folders with a `map.toml`, shipped and own, sorted, without duplicates.
static func map_names() -> PackedStringArray:
	var names: PackedStringArray = []
	for folder in [maps_dir(), user_maps_dir()]:
		var dir := DirAccess.open(folder)
		if dir == null:
			continue
		for map_name in dir.get_directories():
			if not names.has(map_name) and FileAccess.file_exists(folder.path_join(map_name).path_join("map.toml")):
				names.append(map_name)
	names.sort()
	return names
