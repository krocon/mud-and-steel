extends RefCounted
## Self-test part for water and bridges (prompt 014), run by `self_test.gd`
## with its helpers (`_check`, `_frames`, `_ids_of`, `_positions_of`,
## `_moving`).
##
## - The water mesh exists, its level lies between the river bed and the
##   bridge decks.
## - Every bridge model stands with its deck (glTF extra `deck_height_m`) at
##   the simulation's deck height.
## - A tank drives over the stone bridge to the other bank and stands on the
##   deck at the deck height of the simulation while it crosses.

const MapObjects := preload("res://scenes/world/map_objects.gd")
const TIME_SCALE := 4.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var bridges: Array[Dictionary] = sim_host.bridges()
	test._check(bridges.size() >= 2, "expected bridges on '%s', got %d" % [sim_host.map_name(), bridges.size()])
	test._check(main.water.triangle_count > 0, "no water mesh")
	var stone: Dictionary = {}
	for b in bridges:
		if b.kind == "env_bridge_stone" and stone.is_empty():
			stone = b
		var c: Vector3 = b.center
		var level := sim_host.water_level_at(c.x, c.z)
		var ground := _ground(sim_host, c)
		test._check(level > ground and level < c.y - 0.5,
			"bridge %s: water level %.2f not between bed %.2f and deck %.2f" % [b.variant, level, ground, c.y])
		_check_model(test, main, b)
	if stone.is_empty():
		return

	# Drive a tank to the west end, then over the bridge to the east.
	var tank: int = test._ids_of(true)[1]
	var axis := _axis(stone)
	var center: Vector3 = stone.center
	var west := center - axis * (stone.half_length + 24.0)
	var east := center + axis * (stone.half_length + 20.0)
	Engine.time_scale = TIME_SCALE
	await _drive(test, tank, west)
	var on_deck := 0
	var worst := 0.0
	sim_host.move_units(PackedInt64Array([tank]), east.x, east.z)
	var deadline := sim_host.current_tick() + 20 * 120
	var started := false
	while sim_host.current_tick() < deadline:
		await test.get_tree().process_frame
		var p: Vector3 = test._positions_of([tank])[0]
		var local := _local(stone, p)
		if absf(local.x) < stone.half_length - 1.0 and absf(local.y) < stone.half_width:
			on_deck += 1
			var t := (local.x + stone.half_length) / (2.0 * stone.half_length)
			var deck := lerpf(stone.height_start, stone.height_end, t)
			worst = maxf(worst, absf(p.y - deck))
		var moving: bool = test._moving([tank])
		started = started or moving
		if started and not moving:
			break
	Engine.time_scale = 1.0
	var end: Vector3 = test._positions_of([tank])[0]
	test._check(on_deck > 0, "tank never drove on the stone bridge deck")
	test._check(worst < 0.01, "tank %.3f m off the deck height" % worst)
	test._check(Vector2(end.x, end.z).distance_to(Vector2(east.x, east.z)) < 3.0,
		"tank stopped at %s, expected %s beyond the bridge" % [end, east])
	print("self-test: water and bridges ok (%d bridges, %d water triangles, %d frames on the deck, %s)" % [
		bridges.size(), main.water.triangle_count, on_deck, end
	])


func _drive(test: Node, tank: int, target: Vector3) -> void:
	var sim_host: SimHost = test.main.sim_host
	sim_host.move_units(PackedInt64Array([tank]), target.x, target.z)
	var deadline := sim_host.current_tick() + 20 * 120
	var started := false
	while sim_host.current_tick() < deadline:
		await test.get_tree().process_frame
		var moving: bool = test._moving([tank])
		started = started or moving
		if started and not moving:
			return


## Unit vector along the deck (sim heading: clockwise from north = -Z).
func _axis(bridge: Dictionary) -> Vector3:
	var h := deg_to_rad(float(bridge.heading_deg))
	return Vector3(sin(h), 0.0, -cos(h))


## Position along (x) and across (y) the deck, relative to its center.
func _local(bridge: Dictionary, p: Vector3) -> Vector2:
	var axis := _axis(bridge)
	var d: Vector3 = p - bridge.center
	return Vector2(d.x * axis.x + d.z * axis.z, d.x * -axis.z + d.z * axis.x)


func _ground(sim_host: SimHost, p: Vector3) -> float:
	return sim_host.ground_height_at(p.x, p.z)


## The model of `bridge` stands with its deck on the simulation's deck.
func _check_model(test: Node, main: Node3D, bridge: Dictionary) -> void:
	var objects: Node3D = main.map_objects
	var prefix := "%s_%s_" % [bridge.kind, bridge.variant]
	var model := (load("res://assets/models/neutral/%s.glb" % bridge.kind) as PackedScene).instantiate()
	var deck_height := MapObjects._deck_height(EnvLook.variant_node(model, bridge.variant))
	model.free()
	var found := false
	for node in objects.find_children(prefix + "*", "MultiMeshInstance3D", true, false):
		var mm := (node as MultiMeshInstance3D).multimesh
		for i in mm.instance_count:
			var t := mm.get_instance_transform(i)
			var c: Vector3 = bridge.center
			if Vector2(t.origin.x, t.origin.z).distance_to(Vector2(c.x, c.z)) < 0.5:
				found = true
				test._check(absf(t.origin.y + deck_height - c.y) < 0.02,
					"%s: model deck at %.3f, simulation deck at %.3f" % [node.name, t.origin.y + deck_height, c.y])
	test._check(found and deck_height > 0.0, "no model for bridge %s (deck height %.2f)" % [prefix, deck_height])
