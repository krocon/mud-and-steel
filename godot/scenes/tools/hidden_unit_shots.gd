extends Node
## Pictures of hidden own units as silhouettes (prompt 165) as PNG files,
## started by `main.gd` with `-- --hidden-unit-shots=<dir>`
## (`just hidden-unit-shots`; every unit holds fire): an own squad walks
## behind the house nearest to it, another one into the nearest tree group,
## an own vehicle behind a tree; each scene is saved with the setting
## `hidden_units` off and on (`<prefix>_<n>_<scene>_off.png` / `_on.png`).
## Extra arguments (`--map=`, `--daytime=`, `--weather=`) pick the scene;
## `--prefix=` names the files. Needs a window. Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")

## Camera distance and yaw of the pictures; how far behind the cover the
## units are sent (meters beyond its center).
const DISTANCE := 70.0
const YAW := 0.6
const BEHIND_HOUSE_M := 8.0
## Elevation of the view in degrees (a player's flatter tilt: the high
## default view looks over most houses).
const PITCH_DEG := 40.0
const BEHIND_TREE_M := 2.5

var main: Node3D
var dir := ""
var prefix := "day"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--hidden-unit-shots="):
			dir = arg.trim_prefix("--hidden-unit-shots=")
		if arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var squads := _own(host, true)
	var vehicles := _own(host, false)
	if squads.size() < 2 or vehicles.is_empty():
		push_error("hidden unit shots: need two own squads and a vehicle")
		get_tree().quit(1)
		return
	var house: Variant = _nearest_object(host, _position(squads[0]), ["env_house"])
	var trees: Variant = _nearest_object(host, _position(squads[1]), ["tree"])
	var tree: Variant = _nearest_object(host, _position(vehicles[0]), ["tree"], trees)
	print("hidden unit shots: house %s, trees %s, tree %s" % [house, trees, tree])
	# The view looks along `forward` (ground plane); units go beyond the cover.
	_view(Vector3.ZERO)
	await _frames(2)
	var forward: Vector3 = -main.camera.global_basis.z
	forward.y = 0.0
	forward = forward.normalized()
	var scenes := []
	if house != null:
		scenes.append(["house", squads[0], house as Vector3, BEHIND_HOUSE_M])
	if trees != null:
		scenes.append(["trees", squads[1], trees as Vector3, BEHIND_TREE_M])
	if tree != null:
		scenes.append(["vehicle_tree", vehicles[0], tree as Vector3, BEHIND_TREE_M])
	for scene: Array in scenes:
		var target: Vector3 = scene[2] + forward * float(scene[3])
		host.move_units(PackedInt64Array([scene[1]]), target.x, target.z)
	host.speed = 4.0
	await _frames(1800)
	host.speed = 1.0
	await _frames(20)
	var n := 1
	for scene: Array in scenes:
		print("hidden unit shots: %s: unit %d at %.1f m from the cover" % [scene[0], scene[1],
			Vector2(_position(scene[1]).x - scene[2].x, _position(scene[1]).z - scene[2].z).length()])
		var at: Vector3 = scene[2]
		at.y = host.height_at(at.x, at.z)
		_view(at)
		Settings.set_value("hidden_units", "off")
		await _frames(30)
		await _save("%s_%d_%s_off.png" % [prefix, n, scene[0]])
		Settings.set_value("hidden_units", "own_allies")
		await _frames(5)
		await _save("%s_%d_%s_on.png" % [prefix, n, scene[0]])
		n += 1
	get_tree().quit()


func _view(at: Vector3) -> void:
	var rig: Node3D = main.camera_rig
	rig.snap_to(at, DISTANCE, YAW)
	rig.target_tilt = deg_to_rad(PITCH_DEG) - rig.base_pitch()
	rig.snap_to(at, DISTANCE, YAW)


## Living own squads (`squads`) or vehicles, not base buildings.
func _own(host: SimHost, squads: bool) -> Array[int]:
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var stances := host.unit_stances()
	var alive := host.unit_alive()
	var result: Array[int] = []
	var base_view: Node = main.get("base_view")
	for i in ids.size():
		if owners[i] != host.local_player() or alive[i] == 0 or host.unit_is_crew(ids[i]):
			continue
		if base_view != null and base_view.structures.has(ids[i]):
			continue
		if (stances[i] >= 0) == squads:
			result.append(ids[i])
	return result


func _position(id: int) -> Vector3:
	var host: SimHost = main.sim_host
	var k := Array(host.unit_ids()).find(id)
	return host.current_positions()[k] if k >= 0 else Vector3.ZERO


## Position of the map object nearest to `from` whose catalog id contains
## one of `words`, at least 30 m from `away` (null: none).
func _nearest_object(host: SimHost, from: Vector3, words: Array, away: Variant = null) -> Variant:
	var kinds: PackedStringArray = host.object_kinds()
	var best: Variant = null
	var best_d := INF
	for group: Dictionary in host.object_groups(64.0):
		var id := kinds[int(group.kind)].get_slice(":", 0)
		if not words.any(func(w: String) -> bool: return id.contains(w)):
			continue
		var buffer: PackedFloat32Array = group.buffer
		for o in range(0, buffer.size(), 12):
			var p := Vector3(buffer[o + 3], buffer[o + 7], buffer[o + 11])
			if away != null and p.distance_to(away) < 30.0:
				continue
			var d := Vector2(p.x - from.x, p.z - from.z).length()
			if d < best_d:
				best_d = d
				best = p
	return best


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("hidden unit shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
