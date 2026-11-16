extends RefCounted
## Self-test part for wounds and buddy aid (prompt 049), run by
## `self_test.gd` with its helpers (`_check`, `_frames`, `_ticks`,
## `_ids_of`, `_positions_of`). It runs after the replay test:
## `SimHost.debug_wound_soldier` is no command and would make the replay
## differ.
##
## - Two men of an own squad are wounded by debug, one heavily, one lightly:
##   the event feed reports it, the unit panel lists them, the heavily
##   wounded man plays `wounded`, the lightly wounded one `bandage`.
## - The squad is sent 25 m away: a comrade carries the heavily wounded man
##   along (`drag` / `dragged`).


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var infantry: Node = main.infantry_view
	var hud: Control = main.get("hud")
	var care: Node = infantry.care
	# An own squad with at least three men alive.
	var squad := -1
	var ids := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	for id: int in test._ids_of(true, true):
		if alive[ids.find(id)] != 0 and _living(sim_host, id).size() >= 3:
			squad = id
			break
	if squad < 0:
		print("self-test: care skipped (no own squad with three men left)")
		return
	var men := _living(sim_host, squad)
	var squad_only: Array[int] = [squad]
	var at: Vector3 = test._positions_of(squad_only)[0]
	main.camera_rig.snap_to(at, 40.0, 0.0)
	await test._frames(5)
	var drawn: Dictionary = care.counts.duplicate()
	sim_host.debug_wound_soldier(squad, men[0], true)
	sim_host.debug_wound_soldier(squad, men[1], false)
	await test._ticks(2)
	await test._frames(10)
	var counts: Dictionary = sim_host.unit_wounded_counts()
	test._check(counts.get(squad, Vector2i.ZERO) == Vector2i(1, 1),
		"care: wounded counts %s" % counts.get(squad, Vector2i.ZERO))
	test._check(care.counts["wounded"] > drawn["wounded"], "care: the heavily wounded man plays no wounded clip")
	test._check(care.counts["bandage"] > drawn["bandage"], "care: the lightly wounded man does not bandage")
	if hud != null:
		await test._frames(int(hud.event_feed.POLL * 60.0) + 5)
		var reported := false
		for m: Dictionary in hud.event_feed.history:
			reported = reported or str(m.text).begins_with("Man wounded")
		test._check(reported, "care: the event feed reports no wounded man")
		main.selection._set_selection(squad_only)
		await test._frames(20)
		var panel: Control = hud.unit_panel
		test._check(panel._details.text.contains("wounded 1 light, 1 down"),
			"care: unit panel '%s'" % panel._details.text)
		main.selection._set_selection([])
	# Moving off: a comrade carries him along.
	sim_host.move_units(PackedInt64Array(squad_only), at.x + 25.0, at.z)
	for i in 30:
		await test._ticks(10)
		if care.counts["dragged"] > drawn["dragged"] and care.counts["drag"] > drawn["drag"]:
			break
	test._check(care.counts["dragged"] > drawn["dragged"], "care: nobody is carried along")
	test._check(care.counts["drag"] > drawn["drag"], "care: nobody drags a comrade")
	print("self-test: care ok (squad %d: %s)" % [squad, care.counts])


## Indices of the living men of unit `id`.
static func _living(sim_host: SimHost, id: int) -> Array[int]:
	var units := sim_host.soldier_units()
	var indices := sim_host.soldier_indices()
	var hp := sim_host.soldier_hp()
	var out: Array[int] = []
	for k in units.size():
		if units[k] == id and hp[k] > 0:
			out.append(indices[k])
	return out
