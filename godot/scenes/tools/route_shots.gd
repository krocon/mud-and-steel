extends Node
## Pictures of the route lines (prompt 166) as PNG files, started by
## `main.gd` with `-- --route-shots=<dir>` (`just route-shots`; every unit
## holds fire): twelve own vehicles get one move order; another squad a
## waypoint chain, one a patrol, two vehicles an attack-move. Each scene is
## saved with the setting `route_lines` `always` (as before prompt 166),
## `brief` right after the order, `brief` after the fade, and `targets`.
## Then the time of `waypoints.gd`'s `_draw` with every own unit selected
## (up to 48) is printed for `always` and `brief`. Extra arguments
## (`--map=`, `--daytime=`) pick the scene; `--prefix=` names the files.
## Needs a window. Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")
const FormationOrders := preload("res://scenes/selection/formation_orders.gd")

var main: Node3D
var dir := ""
var prefix := "meadow"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--route-shots="):
			dir = arg.trim_prefix("--route-shots=")
		if arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(20)
	var host: SimHost = main.sim_host
	var selection: Control = main.selection
	var vehicles := _own(host, false)
	var squads := _own(host, true)
	if vehicles.size() < 4 or squads.size() < 2:
		push_error("route shots: need four own vehicles and two squads")
		get_tree().quit(1)
		return
	var start := _middle(vehicles)
	# Away from the enemy (the map's middle).
	var away := Vector3(start.x, 0.0, start.z).normalized()
	if away.length() < 0.5:
		away = Vector3(1, 0, 0)
	var side := away.cross(Vector3.UP)
	# Scene 1: the twelve vehicles nearest to one in the middle of the
	# army (the first ones may stand alone at the base), one order.
	var twelve := _nearest(vehicles, _middle([vehicles[vehicles.size() / 2]]), 12)
	var from := _middle(twelve)
	var target := from + away * 160.0 + side * 50.0
	selection.set_selection(twelve)
	selection.order_move_to(target)
	await _scene("1_twelve", [from, target])
	# Scene 2: near one squad, a waypoint chain, a patrol (the next squad)
	# and an attack-move (the two nearest vehicles), all selected.
	var chain_squad: int = squads[squads.size() / 2]
	var at_chain := _middle([chain_squad])
	var others := squads.duplicate()
	others.erase(chain_squad)
	var patrol_squad: int = _nearest(others, at_chain, 1)[0]
	var pair := _nearest(vehicles, at_chain, 2)
	var points: Array = [at_chain]
	var c1 := at_chain + side * 50.0
	var c2 := c1 + away * 50.0
	var c3 := at_chain + away * 90.0
	selection.set_selection([chain_squad])
	selection.order_move_to(c1)
	selection.order_move_to(c2, true)
	selection.order_move_to(c3, true)
	points.append_array([c1, c2, c3])
	var at_patrol := _middle([patrol_squad])
	var p1 := at_patrol - side * 30.0
	var p2 := p1 + away * 60.0
	host.move_units(PackedInt64Array([patrol_squad]), p1.x, p1.z)
	host.patrol_units(PackedInt64Array([patrol_squad]), p2.x, p2.z)
	points.append_array([at_patrol, p1, p2])
	var at_pair := _middle(pair)
	var attack := at_pair + away * 110.0 - side * 20.0
	selection.set_selection(pair)
	selection.order_move_to(attack, false, Vector3.ZERO, FormationOrders.ENGAGE_ALL)
	points.append_array([at_pair, attack])
	selection.set_selection([chain_squad, patrol_squad] + pair)
	await _scene("2_chain_patrol_attack", points)
	# Drawing time with every own unit selected.
	selection.set_selection((vehicles + squads).slice(0, 48))
	selection.order_move_to(start + away * 260.0)
	main.camera_rig.snap_to(start, 260.0, main.camera_rig.yaw)
	for mode: String in ["always", "brief"]:
		Settings.set_value("route_lines", mode)
		await _frames(10)
		var waypoints: Node = main.hud.waypoints
		var total := 0
		for k in 120:
			await RenderingServer.frame_post_draw
			total += waypoints.draw_usec
		print("route shots: %d selected, route_lines %s: %.3f ms per frame" % [
			mini(48, vehicles.size() + squads.size()), mode, total / 120.0 / 1000.0])
	Settings.set_value("route_lines", "brief")
	get_tree().quit()


## One scene in four pictures (see the header), the camera over `points`.
func _scene(name: String, points: Array) -> void:
	var look_at := Vector3.ZERO
	for p: Vector3 in points:
		look_at += p
	look_at /= points.size()
	var spread := 0.0
	for p: Vector3 in points:
		spread = maxf(spread, Vector2(p.x - look_at.x, p.z - look_at.z).length())
	look_at.y = main.sim_host.height_at(look_at.x, look_at.z)
	main.camera_rig.snap_to(look_at, clampf(spread * 2.4, 120.0, 420.0), main.camera_rig.yaw)
	var waypoints: Node = main.hud.waypoints
	Settings.set_value("route_lines", "always")
	await _frames(60)
	await _save("%s_%s_a_always.png" % [prefix, name])
	Settings.set_value("route_lines", "brief")
	waypoints._ordered_at = Time.get_ticks_msec()
	await _frames(3)
	await _save("%s_%s_b_brief_strong.png" % [prefix, name])
	print("route shots: %s: %d groups, %d lines (brief)" % [name, waypoints.groups, waypoints.lines])
	waypoints._ordered_at = -100000
	await _frames(3)
	await _save("%s_%s_c_brief_faint.png" % [prefix, name])
	Settings.set_value("route_lines", "targets")
	await _frames(3)
	await _save("%s_%s_d_targets.png" % [prefix, name])


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


## The `count` units of `units` nearest to `at`.
func _nearest(units: Array, at: Vector3, count: int) -> Array:
	var host: SimHost = main.sim_host
	var ids := Array(host.unit_ids())
	var positions := host.current_positions()
	var sorted := units.duplicate()
	sorted.sort_custom(func(a: int, b: int) -> bool:
		return positions[ids.find(a)].distance_to(at) < positions[ids.find(b)].distance_to(at))
	return sorted.slice(0, count)


func _middle(units: Array) -> Vector3:
	var host: SimHost = main.sim_host
	var ids := Array(host.unit_ids())
	var positions := host.current_positions()
	var sum := Vector3.ZERO
	for id: int in units:
		sum += positions[ids.find(id)]
	return sum / maxf(1.0, units.size())


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("route shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
