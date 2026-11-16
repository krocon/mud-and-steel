extends Node
## Records a trench strongpoint (prompt 052) as PNG files, started by
## `main.gd` with `-- --position-shots=<dir>` (`just position-shots <dir>`;
## the test match, every unit holding fire):
## a neutral strongpoint (`SimHost.debug_strongpoint`) beside the own
## squads, the squads walk in and take the firing positions, a vehicle
## pit under a camouflage net in front of it with an own tank dug in,
## a debug crater tears the ring's parapet; then an overview, a firing bay,
## the dugouts, the ring with its wire and the dug-in vehicle.
## Needs a window (not headless). Quits when done.

var main: Node3D
var dir := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--position-shots="):
			dir = arg.trim_prefix("--position-shots=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	var host: SimHost = main.sim_host
	await _frames(20)
	var squads := _own_squads(host)
	var positions := host.current_positions()
	var ids := Array(host.unit_ids())
	var start := Vector3.ZERO
	for id: int in squads:
		start += positions[ids.find(id)]
	start /= maxf(1.0, float(squads.size()))
	# Behind the own start, away from the enemy.
	var dir_out := Vector3(start.x, 0.0, start.z).normalized()
	if dir_out.length() < 0.5:
		dir_out = Vector3(1, 0, 0)
	var center := start + dir_out * 35.0
	center.y = host.height_at(center.x, center.z)
	var placed := host.debug_strongpoint(center.x, center.z)
	var kinds := {}
	for w: Dictionary in host.field_works():
		kinds[w.kind] = int(kinds.get(w.kind, 0)) + 1
	print("position shots: strongpoint with %d works at %s: %s" % [placed, center, kinds])
	# The squads man the side of the ring facing them.
	var front := center - dir_out * 14.0
	if not squads.is_empty():
		host.move_units(PackedInt64Array(squads), front.x, front.z)
	# A vehicle pit under a camouflage net in front of the ring; an own tank
	# drives in and digs in.
	var pit := center - dir_out * 42.0
	host.debug_work("vehicle_pit", Vector2(pit.x, pit.z), Vector2(pit.x, pit.z))
	host.debug_work("camo_net", Vector2(pit.x, pit.z), Vector2(pit.x, pit.z))
	var tank := _own_vehicle(host)
	if tank >= 0:
		host.move_units(PackedInt64Array([tank]), pit.x, pit.z)
	host.speed = 4.0
	await _frames(1600)
	host.speed = 1.0
	# A shell struck the ring: its parapet is torn.
	var hit := center + Vector3(0, 0, 1) * 16.0
	host.debug_crater(hit.x, hit.z, 105.0)
	await _frames(20)
	print("position shots: dug in %s, camouflaged %s, damaged %s" % [
		host.dug_in_units(), host.camouflaged_units(), host.damaged_works()])
	main.works_view.refresh()
	await _frames(30)
	main.camera_rig.snap_to(center, 70.0, deg_to_rad(30.0))
	await _frames(40)
	await _save("1_strongpoint_overview.png")
	front.y = host.height_at(front.x, front.z)
	main.camera_rig.snap_to(front, 26.0, deg_to_rad(200.0))
	await _frames(40)
	await _save("2_firing_bay_closeup.png")
	main.camera_rig.snap_to(center + Vector3(0, 0, -6), 30.0, deg_to_rad(120.0))
	await _frames(40)
	await _save("3_dugouts_and_comm_trench.png")
	main.camera_rig.snap_to(center, 130.0, deg_to_rad(0.0))
	await _frames(40)
	await _save("4_ring_and_wire.png")
	pit.y = host.height_at(pit.x, pit.z)
	main.camera_rig.snap_to(pit, 26.0, deg_to_rad(160.0))
	await _frames(40)
	await _save("5_dug_in_tank_under_net.png")
	get_tree().quit()


func _own_vehicle(host: SimHost) -> int:
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var stances := host.unit_stances()
	for i in ids.size():
		var base_view: Node = main.get("base_view")
		var structure: bool = base_view != null and base_view.structures.has(ids[i])
		if owners[i] == host.local_player() and stances[i] < 0 and host.unit_alive()[i] != 0 \
				and not structure:
			return ids[i]
	return -1


func _own_squads(host: SimHost) -> Array[int]:
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var stances := host.unit_stances()
	var result: Array[int] = []
	for i in ids.size():
		if owners[i] == host.local_player() and stances[i] >= 0 and host.unit_alive()[i] != 0:
			result.append(ids[i])
	return result


func _save(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("position shots: %s" % dir.path_join(file))


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
