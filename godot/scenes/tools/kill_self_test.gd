extends RefCounted
## Self-test part for kill sequences, state marks and ricochets (prompt
## 105), run by `self_test.gd` with its helpers (`_check`, `_frames`,
## `_ids_of`, `_indices`, `_positions_of`).
##
## `watch` (before the combat test) notes every destroyed event: the unit
## must be dead in that very snapshot, no click may select it, and it must
## have left the selection in the same frame.
##
## `run_kills` (after the combat test):
## - The tank destroyed in the combat test got a kill sequence of its
##   class; it starts with the flash, and played on (the effects' clock run
##   forward without the simulation) it plays every planned stage in order
##   and ends with a burning wreck; no sequence is left running.
## - The table: the same unit and tick give the same stages, a truck burns
##   out without a cook-off, a heavy hit skips the smoke, two units differ.
## - A round that bounces off an own tank leaves a ricochet tracer that
##   flies away from the face it hit; out of sight none.
##
## `run_marks` (after the works test, whose tank lost its track):
## - Marks by rule: an enemy never shows out of ammunition, out of fuel or
##   being serviced; at most two marks, one from far away.
## - The tank without its track shows the track mark; every immobilized
##   unit shows the immobilized mark.
## - An enemy whose state says out of ammunition and immobilized shows only
##   the immobilized mark; the setting "never" draws none.

const StateMarks := preload("res://scenes/hud/state_marks.gd")
const KillSequence := preload("res://scenes/world/kill_sequence.gd")
const CombatView := preload("res://scenes/world/combat_view.gd")
const Settings := preload("res://scenes/menu/settings.gd")

## Destroyed events seen: {unit, alive, selectable, frame}.
var _seen: Array[Dictionary] = []
var _test: Node


func watch(test: Node) -> void:
	_test = test
	var main: Node3D = test.main
	main.combat_view.combat_event.connect(_on_event)


func _on_event(event: Dictionary) -> void:
	if event.get("type", "") != "destroyed":
		return
	var main: Node3D = _test.main
	var sim_host: SimHost = main.sim_host
	var id := int(event["unit"])
	var index := sim_host.unit_ids().find(id)
	_seen.append({"unit": id, "alive": sim_host.unit_alive()[index] if index >= 0 else 0,
		"selectable": main.selection._selectable_index(id), "frame": Engine.get_process_frames()})


func run_kills(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var effects: Node3D = main.effects
	await test._frames(1)
	test._check(not _seen.is_empty(), "no destroyed event seen")
	for seen in _seen:
		test._check(seen.alive == 0, "unit %d was still alive in the snapshot of its destroyed event" % seen.unit)
		test._check(seen.selectable < 0, "unit %d could be selected in the frame it was destroyed" % seen.unit)
		test._check(not main.selection.selected.has(seen.unit), "destroyed unit %d is still selected" % seen.unit)

	# The combat test's tank: its sequence, played to the end.
	var tank: int = _seen[0].unit
	var entry: Dictionary = {}
	for e: Dictionary in effects.kill_log:
		if e.unit == tank:
			entry = e
	test._check(not entry.is_empty(), "the tank %d destroyed in the combat test got no kill sequence: %s" % [tank, effects.kill_log])
	if not entry.is_empty():
		test._check(str(entry.profile).begins_with("armor"), "tank %d got the kill profile '%s'" % [tank, entry.profile])
		test._check(not entry.planned.is_empty() and entry.planned[0] == "flash" and entry.planned.has("cookoff"),
			"tank kill stages %s" % [entry.planned])
		test._check(not entry.done.is_empty() and entry.done[0] == "flash", "the tank's kill did not start with the flash: %s" % [entry.done])
		test._check(entry.planned.size() >= KillSequence.stages(entry.profile, tank, entry.tick).size() - 1,
			"the stages of tank %d are not the table's" % tank)
	for k in 40:
		effects._process_effects(0.2)
	test._check(effects.kills_running() == 0, "%d kill sequences still running after 8 s" % effects.kills_running())
	if not entry.is_empty():
		test._check(entry.done == entry.planned, "tank %d played %s of %s" % [tank, entry.done, entry.planned])
	var wreck: Dictionary = effects._wreck_of(tank)
	test._check(not wreck.is_empty() and wreck.lit and (wreck.fires.is_empty() or (wreck.fires[0] as GPUParticles3D).emitting),
		"the kill of tank %d did not end in a burning wreck: %s" % [tank, wreck])
	test._check(effects.counts.get("kill_stage_cookoff", 0) > 0, "no cook-off played: %s" % effects.counts)

	# The table.
	test._check(KillSequence.stages("armor", 7, 120) == KillSequence.stages("armor", 7, 120), "kill stages are not repeatable")
	test._check(KillSequence.stages("armor", 7, 120) != KillSequence.stages("armor", 8, 120), "two kills look the same")
	test._check(KillSequence.profile("truck", false, 3, 50) == "soft" and not _names("soft", 3, 50).has("cookoff"),
		"a truck cooks off: %s" % [_names("soft", 3, 50)])
	test._check(not _names(KillSequence.profile("tank_medium", true, 3, 50), 3, 50).has("smoke"), "a heavy hit does not skip the smoke")
	test._check(KillSequence.profile("env_building", false, 3, 50).is_empty(), "a building got a kill sequence")
	for stages: Array in KillSequence.STAGES.values():
		var last := 0.0
		for st: Array in stages:
			last = maxf(last, float(st[1]) + float(st[2]))
		test._check(last <= 5.0, "a kill sequence lasts %.1f s" % last)

	# A ricochet off an own tank (visible), flying away from its face.
	var own: Array[int] = []
	var alive := sim_host.unit_alive()
	for id in test._ids_of(true):
		var one: Array[int] = [id]
		if alive[test._indices(one)[0]] != 0:
			own.append(id)
	test._check(not own.is_empty(), "no own tank left for the ricochet test")
	if not own.is_empty():
		var at: Vector3 = test._positions_of(own.slice(0, 1))[0] + Vector3.UP
		var view: Node3D = main.combat_view
		var before: int = view.ricochets
		var event := {"type": "hit", "projectile": 4711, "shooter": -1, "position": at, "surface": "unit",
			"target": own[0], "face": "front", "turret": false, "penetrated": false, "damage": 0,
			"caliber_mm": 75.0, "from": at + Vector3(0, 0, -80), "tick": sim_host.current_tick()}
		var visible_at: Callable = effects.visible_at
		effects.visible_at = func(_point: Vector3) -> bool: return true
		view.ricochet(event)
		test._check(view.ricochets == before + 1, "a round bouncing off a tank left no ricochet")
		effects.visible_at = func(_point: Vector3) -> bool: return false
		view.ricochet(event)
		test._check(view.ricochets == before + 1, "a ricochet out of sight was drawn")
		effects.visible_at = visible_at
		# From the north onto a tank facing north: back north and up.
		var out := CombatView.ricochet_direction(Vector3(0, -0.1, 1), "front", 0.0, 0.5, 0.5)
		test._check(out.z < 0.0 and out.y > 0.1, "a ricochet off the front flies to %s" % out)
	print("self-test: kill sequences ok (%d destroyed seen, %s, %d ricochets)" % [
		_seen.size(), entry.get("done", []), main.combat_view.ricochets])


func _names(profile: String, unit: int, tick: int) -> Array:
	return KillSequence.stages(profile, unit, tick).map(func(st: Array) -> String: return st[1])


func run_marks(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var marks: Control = main.combat_view.state_marks
	test._check(marks != null, "no state marks")
	if marks == null:
		return
	var all: Array[String] = StateMarks.ORDER.duplicate()
	test._check(StateMarks.marks_of(["no_ammo", "no_fuel", "service"], false, false).is_empty(),
		"an enemy shows marks seen only from inside")
	test._check(StateMarks.marks_of(["immobile", "no_ammo"], false, false) == ["immobile"], "enemy marks wrong")
	test._check(StateMarks.marks_of(all, true, false).size() == 2, "more than two marks")
	test._check(StateMarks.marks_of(all, true, true) == ["burning"], "from far away not only the most important mark")
	test._check(StateMarks.marks_of(["immobile"], true, false) == ["immobile"], "an immobilized unit shows no mark")

	var saved_mode: String = Settings.get_value("state_marks")
	Settings.values["state_marks"] = "always"
	marks.scan()
	var immobile := sim_host.unit_immobile()
	var alive := sim_host.unit_alive()
	var ids := sim_host.unit_ids()
	var immobilized := 0
	for i in ids.size():
		if alive[i] != 0 and immobile[i] != 0:
			immobilized += 1
			test._check(marks.states.get(ids[i], []).has("immobile"), "immobilized unit %d has no mark" % ids[i])
	var tank := -1
	for id in sim_host.tracks_off():
		var index := ids.find(id)
		if index >= 0 and alive[index] != 0:
			tank = id
	test._check(tank >= 0, "no living vehicle without its track")
	if tank >= 0:
		test._check(marks.states.get(tank, []).has("track"), "the tank without its track has no state: %s" % [marks.states.get(tank, [])])
		var tank_only: Array[int] = [tank]
		main.camera_rig.snap_to(test._positions_of(tank_only)[0], 80.0, 0.0)
		marks._scan_left = 10.0
		await test._frames(2)
		test._check(marks.drawn_kinds.get("track", 0) > 0, "the tank without its track shows no mark (%s)" % marks.drawn_kinds)

	# An enemy said to be out of ammunition and immobilized, in plain sight.
	var enemy := -1
	var owners := sim_host.unit_owners()
	for i in ids.size():
		if alive[i] != 0 and owners[i] != sim_host.local_player() and sim_host.relation(owners[i]) != 1 \
				and main._unit_nodes.has(ids[i]):
			enemy = ids[i]
			break
	test._check(enemy >= 0, "no enemy left for the mark test")
	if enemy >= 0:
		var fog: Node3D = main.fog_of_war
		fog.set_fog_enabled(false)
		var visible_at: Callable = main.effects.visible_at
		main.effects.visible_at = func(_point: Vector3) -> bool: return true
		var enemy_only: Array[int] = [enemy]
		main.camera_rig.snap_to(test._positions_of(enemy_only)[0], 80.0, 0.0)
		await test._frames(2)
		marks.states[enemy] = ["immobile", "no_ammo", "no_fuel", "service"]
		marks._scan_left = 10.0
		await test._frames(2)
		test._check(marks.enemy_kinds.get("immobile", 0) > 0, "an immobilized enemy in sight shows no mark (%s)" % marks.enemy_kinds)
		for kind in ["no_ammo", "no_fuel", "service"]:
			test._check(not marks.enemy_kinds.has(kind), "an enemy shows '%s'" % kind)
		Settings.values["state_marks"] = "never"
		await test._frames(2)
		test._check(marks.drawn == 0, "the setting 'never' still draws %d marks" % marks.drawn)
		main.effects.visible_at = visible_at
		fog.set_fog_enabled(true)
		await test._frames(2)
	Settings.values["state_marks"] = saved_mode
	marks._scan_left = 0.0
	print("self-test: state marks ok (track mark on tank %d, %d immobilized units)" % [tank, immobilized])
