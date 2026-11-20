extends SceneTree
## Pictures of the fog of war (prompt 098) as PNG files: a match on the
## given map, the camera on the edge of the local player's sight (looking
## from the own units towards the map center), with the minimap. Needs a
## window; quits when done. Run it as a main loop script:
##
##     godot --path godot -s res://scenes/tools/fog_shots.gd -- \
##         --fog-shots=<absolute dir> --prefix=forest --map=river_fields [--daytime=night]
##
## The other arguments go to `main.gd` like for `just run`.

const MAIN_SCENE := preload("res://scenes/main.tscn")

var _main: Node3D
var _dir := ""
var _prefix := "fog"


func _initialize() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--fog-shots="):
			_dir = arg.trim_prefix("--fog-shots=")
		if arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(_dir)
	_main = MAIN_SCENE.instantiate() as Node3D
	root.add_child(_main)
	_run()


func _run() -> void:
	await _frames(30)
	var host: SimHost = _main.sim_host
	var fog: Node = _main.fog_of_war
	_main.camera_rig.edge_scroll_enabled = false
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var positions := host.current_positions()
	var own := Vector3.ZERO
	var count := 0
	for i in ids.size():
		if owners[i] == host.local_player():
			own += positions[i]
			count += 1
	if count == 0:
		push_error("fog shots: no own unit")
		quit(1)
		return
	own /= count
	# Walk from the own units towards the map center until out of sight.
	var towards := -Vector3(own.x, 0.0, own.z).normalized()
	var edge := own
	for step in 200:
		var p := own + towards * (step * 8.0)
		if fog.state_at(p) != 255:
			edge = p
			break
	_main.camera_rig.snap_to(edge, 260.0, atan2(towards.x, towards.z) + PI)
	await _frames(40)
	await RenderingServer.frame_post_draw
	var file := _dir.path_join("%s.png" % _prefix)
	root.get_viewport().get_texture().get_image().save_png(file)
	print("fog shots: %s (%.0f %% visible)" % [file, fog.share_of(255) * 100.0])
	quit()


func _frames(count: int) -> void:
	for i in count:
		await process_frame
