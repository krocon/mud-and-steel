extends Node
## Pictures of the ground tooltip (prompt 100) as PNG files, started by
## `main.gd` with `-- --ground-tooltip-shots=<dir>` (`just
## ground-tooltip-shots`): the box over the first road, woods, house, deep
## or shallow water, meadow and field the map has. Extra arguments
## (`--map=`, `--biome=`, `--language=`) pick the scene. Needs a window.

const KINDS := ["road", "woods", "house", "water", "meadow", "field", "mud"]

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--ground-tooltip-shots="):
			dir = arg.trim_prefix("--ground-tooltip-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _kind(info: Dictionary) -> String:
	var terrain: String = info.get("terrain_name", "")
	var object: String = info.get("object", "")
	if info.get("building", false):
		return "house"
	if info.get("sight", "") == "foliage" and object.begins_with("env_tree"):
		return "woods"
	if not object.is_empty():
		return ""
	match terrain:
		"road":
			return "road"
		"deep_water", "shallow_water":
			return "water"
		"grass":
			return "meadow"
		"field_grain", "field_green", "field_ploughed":
			return "field"
		"mud":
			return "mud"
	return ""


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var size: Vector2 = host.map_size()
	var found := {}
	var x := -size.x * 0.45
	while x <= size.x * 0.45:
		var z := -size.y * 0.45
		while z <= size.y * 0.45:
			var kind := _kind(host.ground_info(x, z))
			if not kind.is_empty() and not found.has(kind):
				found[kind] = Vector3(x, host.height_at(x, z), z)
			z += 6.0
		x += 6.0
	var tip: Control = main.hud.ground_tooltip
	# The box shows what this tool asks for, not the ground under the mouse.
	tip.set_process(false)
	for kind: String in KINDS:
		if not found.has(kind):
			continue
		var at: Vector3 = found[kind]
		main.camera_rig.snap_to(at, 70.0, deg_to_rad(20.0))
		await _frames(15)
		tip.show_at(at, main.camera.unproject_position(at))
		await _frames(2)
		await _save("tooltip_%s.png" % kind)
	get_tree().quit()


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("ground tooltip shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
