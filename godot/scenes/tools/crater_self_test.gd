extends RefCounted
## Self-test part for craters (prompt 021), run by `self_test.gd` after the
## destruction test with its helpers (`_check`, `_frames`, `_ticks`,
## `_ids_of`, `_positions_of`, `_indices`).
##
## The own tanks fire at open grass near them until the simulation reports
## a new crater (real ground hits, so the replay test that follows
## re-simulates it to the same hash). Then: the ground there is lower in
## the simulation, the drawn terrain shows the same height at the crater's
## sample, the bowl is mud, and no decoration grows on deformed samples.

const TIME_SCALE := 4.0
const SECONDS := 90
const GRASS := 0
const MUD := 4


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node = main.crater_view
	var tanks: Array[int] = []
	var alive: PackedByteArray = sim_host.unit_alive()
	var own: Array[int] = test._ids_of(true)
	var indices: Array[int] = test._indices(own)
	for k in own.size():
		if alive[indices[k]] != 0:
			tanks.append(own[k])
	test._check(not tanks.is_empty(), "no own tank left for the crater test")
	if tanks.is_empty():
		return
	var center: Vector3 = test._positions_of(tanks)[0]
	var target := _open_grass(sim_host, center)
	test._check(target != Vector3.INF, "no open grass near %s" % center)
	if target == Vector3.INF:
		return
	main.camera_rig.focus = target
	await test._frames(10)

	var before: int = sim_host.crater_count()
	var seen_before: int = view.craters
	var ids := PackedInt64Array(tanks)
	sim_host.attack_ground(ids, target.x, target.z)
	Engine.time_scale = TIME_SCALE
	var deadline := sim_host.current_tick() + SECONDS * sim_host.tick_rate()
	while sim_host.crater_count() == before and sim_host.current_tick() < deadline:
		await test.get_tree().process_frame
	sim_host.stop_units(ids)
	Engine.time_scale = 1.0
	await test._ticks(2)
	test._check(sim_host.crater_count() > before, "no crater after %d s of fire" % SECONDS)
	if sim_host.crater_count() == before:
		return
	# The view takes the event and uploads within half a second.
	await test._frames(40)
	test._check(view.craters > seen_before and view.uploads > 0,
		"crater view saw %d craters, %d uploads" % [view.craters - seen_before, view.uploads])
	if view.recent.is_empty():
		return
	var event: Dictionary = view.recent[-1]
	var at: Vector3 = event.position
	var s: Vector2i = main.terrain.sample_of(at)
	var deform: float = sim_host.deformation_at(s.x, s.y)
	test._check(deform < -0.05, "sample %s under the crater only %.3f m deep" % [s, deform])
	var half: Vector2 = sim_host.map_size() * 0.5
	var cell: float = sim_host.cell_size()
	var p := Vector3(s.x * cell - half.x, 0.0, s.y * cell - half.y)
	var drawn: float = main.terrain.displayed_height(s.x, s.y)
	var simulated: float = sim_host.height_at(p.x, p.z)
	test._check(absf(drawn - simulated) < 0.01,
		"drawn height %.3f differs from the simulation %.3f in the crater" % [drawn, simulated])
	test._check(sim_host.terrain_at(p.x, p.z) == MUD, "crater bowl is terrain %d, not mud" % sim_host.terrain_at(p.x, p.z))
	# Decoration: no grass instance stands on a deformed sample. The buffers
	# come from `scatter_deco` as `map_objects.gd` builds them (MultiMesh
	# instances cannot be read back headless).
	var objects: Node3D = main.map_objects
	var layer: Dictionary = objects.DECO_LAYERS[0]
	var chunk: float = objects.DECO_CHUNK_M
	var square := Vector2i(floori((at.x + half.x) / chunk), floori((at.z + half.y) / chunk))
	var on_crater := 0
	var checked := 0
	for dj: int in [-1, 0, 1]:
		for di: int in [-1, 0, 1]:
			var x0 := -half.x + (square.x + di) * chunk
			var z0 := -half.y + (square.y + dj) * chunk
			var buffers: Array = sim_host.scatter_deco(
				x0, z0, chunk, layer.spacing, layer.seed, PackedFloat32Array(layer.density),
				layer.wood, layer.patchiness, PackedFloat32Array(layer.weights), layer.scale, layer.sink)
			for buffer: PackedFloat32Array in buffers:
				for k in buffer.size() / 12:
					var o := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
					if absf(o.x - at.x) > 12.0 or absf(o.z - at.z) > 12.0:
						continue
					checked += 1
					var q: Vector2i = main.terrain.sample_of(o)
					if sim_host.deformation_at(q.x, q.y) != 0.0:
						on_crater += 1
	test._check(checked > 0, "no grass scattered around the crater")
	test._check(on_crater == 0, "%d grass instances stand in the crater" % on_crater)
	print("self-test: craters ok (%d craters, %.2f m deep at %s, %d uploads, last %.1f ms, %d grass near it, %d deco squares rebuilt)" % [
		sim_host.crater_count(), -deform, s, view.uploads, view.last_upload_ms, checked, main.map_objects.crater_rebuilds,
	])


## A grass point 25-60 m from `center` with grass all around (no road or
## water nearby), or Vector3.INF.
func _open_grass(sim_host: SimHost, center: Vector3) -> Vector3:
	for r: float in [30.0, 45.0, 60.0, 25.0]:
		for k in 16:
			var a := TAU * k / 16.0
			var p := center + Vector3(cos(a), 0.0, sin(a)) * r
			var ok := true
			for dx: float in [-6.0, 0.0, 6.0]:
				for dz: float in [-6.0, 0.0, 6.0]:
					if sim_host.terrain_at(p.x + dx, p.z + dz) != GRASS:
						ok = false
			if ok:
				return Vector3(p.x, sim_host.height_at(p.x, p.z), p.z)
	return Vector3.INF
