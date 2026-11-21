extends RefCounted
## Tutorial and missions in the flow self-test (prompt 039), driven by
## `flow_self_test.gd`: the menu entries, then the tutorial from its button
## and every mission from the "Missions" screen, each played to victory
## through the game's own paths: the camera is moved and units selected
## (the mission HUD sends the signals the tutorial waits for), buildings are
## placed with the base HUD's placement, and the units follow the active
## objective like `crates/ai/tests/missions.rs` does: they attack the
## nearest enemy they see near its area, else move there. Wins must land in
## the progress file (the self-test's own).

const Missions := preload("res://scenes/menu/missions.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
## Game time per real time while a mission plays.
const SPEED := 12.0
## Frames a mission may take at most.
const MAX_FRAMES := 30000
## Ticks between two orders of the autopilot.
const ORDER_TICKS := 100
## Enemies this far beyond an objective's area are attacked first (m).
const REACH_M := 300.0

## The flow self-test (for `_check`, `_frames`, `_wait_scene`).
var flow: Node
var _last_order_tick := -ORDER_TICKS


func _init(flow_test: Node) -> void:
	flow = flow_test


## Runs everything from the main menu; returns the main menu at the end
## (null if a scene did not appear).
func run(menu: Control) -> Control:
	Missions.reset()
	flow._check(menu.tutorial_button != null and menu.tutorial_button.is_visible_in_tree(), "main menu has no Tutorial button")
	flow._check(menu.missions_button != null and menu.missions_button.is_visible_in_tree(), "main menu has no Missions button")
	menu.missions_button.pressed.emit()
	await flow._frames(2)
	var screen: Control = menu.screens["missions"]
	flow._check(screen.visible, "missions screen not shown")
	var entries: Array[Dictionary] = screen.entries.duplicate()
	flow._check(entries.size() >= 4, "missions screen lists %d entries" % entries.size())
	for m in entries:
		flow._check(m.get("ok", false), "mission %s cannot be read: %s" % [m["id"], m.get("error", "")])
	flow._check(not entries.is_empty() and entries[0]["kind"] == "tutorial", "the tutorial is not listed first")
	var factions := {}
	for m in entries:
		if m["kind"] == "mission":
			factions[m["faction"]] = true
	flow._check(factions.size() >= 3, "missions cover the factions %s" % [factions.keys()])
	screen.back_button.pressed.emit()
	await flow._frames(2)

	# The tutorial from its own button.
	menu.tutorial_button.pressed.emit()
	var main := await _wait_loaded("test_small")
	if main == null:
		return null
	await _play(main, true)
	menu = await flow._back_to_menu(main)
	if menu == null:
		return null
	flow._check(Missions.is_done(Missions.TUTORIAL_ID), "tutorial not recorded as won")

	# Every mission from the missions screen.
	for m in entries:
		if m["kind"] != "mission":
			continue
		menu.missions_button.pressed.emit()
		await flow._frames(2)
		screen = menu.screens["missions"]
		var index: int = screen.find(m["id"])
		flow._check(index >= 0, "mission %s not listed" % m["id"])
		if index < 0:
			continue
		screen.list.select(index)
		screen.start_selected()
		main = await _wait_loaded(m["map"])
		if main == null:
			return null
		if not m.get("biome", "").is_empty():
			flow._check(main.atmosphere.preset.id == m["biome"], "mission %s plays in %s, not %s" % [m["id"], main.atmosphere.preset.id, m["biome"]])
		await _play(main, false)
		menu = await flow._back_to_menu(main)
		if menu == null:
			return null
		flow._check(Missions.is_done(m["id"]), "mission %s not recorded as won" % m["id"])
	menu.missions_button.pressed.emit()
	await flow._frames(2)
	screen = menu.screens["missions"]
	flow._check(screen.list.get_item_text(0).contains("(won)"), "missions screen does not show the won tutorial: %s" % screen.list.get_item_text(0))
	screen.back_button.pressed.emit()
	await flow._frames(2)
	Missions.reset()
	return menu


func _wait_loaded(map_name: String) -> Node3D:
	var main: Node3D = await flow._wait_scene("Main")
	if main == null:
		return null
	var frames := 900
	while not main._loaded and frames > 0:
		frames -= 1
		await flow.get_tree().process_frame
	flow._check(main._loaded, "mission on %s did not finish loading" % map_name)
	flow._check(main.sim_host.map_name() == map_name, "mission plays on %s, not %s" % [main.sim_host.map_name(), map_name])
	return main


## Plays the running mission until the result screen shows a victory.
func _play(main: Node3D, tutorial: bool) -> void:
	var host: SimHost = main.sim_host
	var info: Dictionary = host.mission_info()
	flow._check(info.get("active", false), "no mission running")
	flow._check(main.mission_hud != null and main.mission_hud.visible, "mission panel not shown")
	flow._check(info.get("tutorial", false) == tutorial, "tutorial flag %s" % info.get("tutorial"))
	var game_menu: CanvasLayer = main.game_menu
	host.speed = SPEED
	_last_order_tick = -ORDER_TICKS
	var signals := {}
	var rings := 0
	var messages := {}
	var frames := MAX_FRAMES
	var started := Time.get_ticks_msec()
	while not game_menu.result_panel.visible and frames > 0:
		frames -= 1
		await flow.get_tree().process_frame
		if frames % 10 != 0:
			continue
		info = host.mission_info()
		for s: String in info["wanted"]:
			signals[s] = true
		rings = maxi(rings, main.mission_hud.rings)
		messages[info["message"]] = true
		_autopilot(main, info)
	host.speed = 1.0
	flow._check(game_menu.result_panel.visible, "mission %s not decided after %d ticks: %s" % [info.get("id"), host.current_tick(), info.get("objectives")])
	var result: Dictionary = host.match_result()
	flow._check(result["decided"] and result["winner"] == host.local_player(), "mission %s lost: %s" % [info.get("id"), result])
	flow._check(game_menu.result_title.text == "VICTORY", "mission result shows '%s'" % game_menu.result_title.text)
	flow._check(game_menu.result_reason.text == "Mission accomplished.", "mission result reason '%s'" % game_menu.result_reason.text)
	info = host.mission_info()
	for o: Dictionary in info["objectives"]:
		flow._check(o["state"] == "done", "objective %s of %s is %s" % [o["id"], info["id"], o["state"]])
	flow._check(rings > 0, "no objective ring drawn in %s" % info["id"])
	flow._check(messages.size() >= 2, "mission %s showed %d messages" % [info["id"], messages.size()])
	# Prompt 108: every column that drove in reached the event feed.
	var arrivals: Array = info.get("arrivals", [])
	flow._check(main.mission_hud.arrivals_reported == arrivals.size(), "mission %s: %d columns drove in, %d reported" % [info["id"], arrivals.size(), main.mission_hud.arrivals_reported])
	if tutorial:
		flow._check(signals.has("camera") and signals.has("select"), "the tutorial did not wait for camera and selection: %s" % [signals.keys()])
		flow._check(main.mission_hud.title.text.begins_with("Training"), "tutorial title '%s'" % main.mission_hud.title.text)
	# The HUD records the victory (every 30 frames).
	await flow._frames(40)
	flow._check(main.mission_hud.recorded, "the mission HUD did not record the victory")
	print("flow self-test: %s '%s' decided at tick %d (%d s game time, %.1f s real time, %d messages, signals %s, %d columns drove in)" % [
		"tutorial" if tutorial else "mission", info["id"], result["tick"], result["tick"] / 20,
		(Time.get_ticks_msec() - started) / 1000.0, messages.size(), signals.keys(), arrivals.size()])


## What a player following the objectives does (see the file comment).
func _autopilot(main: Node3D, info: Dictionary) -> void:
	var host: SimHost = main.sim_host
	var wanted: PackedStringArray = info["wanted"]
	var own := _own_units(main)
	if wanted.has("camera") and not main.camera_rig.is_gliding():
		var rig: Node3D = main.camera_rig
		rig.snap_to(rig.focus + Vector3(25.0, 0.0, 0.0), rig.distance, rig.yaw)
	if wanted.has("select") and not own.is_empty():
		main.selection.set_selection(Array(own))
	var tick := host.current_tick()
	if tick - _last_order_tick < ORDER_TICKS:
		return
	var objective: Dictionary = {}
	for o: Dictionary in info["objectives"]:
		if o["state"] == "active":
			objective = o
			break
	if objective.is_empty():
		return
	_last_order_tick = tick
	if not String(objective["unit"]).is_empty():
		_build_or_produce(main, objective["unit"])
	if objective["position"] == null or own.is_empty():
		return
	var center: Vector3 = objective["position"]
	var reach: float = float(objective["radius"]) + REACH_M
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var alive := host.unit_alive()
	var positions := host.current_positions()
	var best := -1
	var best_d := INF
	for i in ids.size():
		if owners[i] == host.local_player() or alive[i] == 0:
			continue
		var d := Vector2(positions[i].x - center.x, positions[i].z - center.z).length()
		if d <= reach and d < best_d:
			best_d = d
			best = ids[i]
	if best >= 0:
		host.attack_unit(own, best)
	else:
		host.formation_move(own, center.x, center.z, 0, 0.0, 0.0, 1)


## Own living units that can take orders (no buildings).
func _own_units(main: Node3D) -> PackedInt64Array:
	var host: SimHost = main.sim_host
	var out := PackedInt64Array()
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var alive := host.unit_alive()
	var structures := {}
	for s: Dictionary in host.structures():
		structures[s["id"]] = true
	for i in ids.size():
		if owners[i] == host.local_player() and alive[i] != 0 and not structures.has(ids[i]):
			out.append(ids[i])
	return out


## Builds the structure `unit` near the headquarters with the base HUD's
## placement, or produces the unit in a building that offers it.
func _build_or_produce(main: Node3D, unit: String) -> void:
	var host: SimHost = main.sim_host
	var me := host.local_player()
	for o: Dictionary in host.build_options():
		if o["id"] != unit:
			continue
		var hq := Vector3.INF
		for s: Dictionary in host.structures():
			if s["owner"] != me:
				continue
			if s["type_id"] == unit:
				return
			if hq == Vector3.INF:
				var index := Array(host.unit_ids()).find(s["id"])
				if index >= 0:
					hq = host.current_positions()[index]
		if hq == Vector3.INF:
			return
		var hud: Node = main.base_hud
		hud.start_placing(o["type"])
		var placed := false
		for r: float in [25.0, 35.0, 45.0, 55.0]:
			for k in 16:
				var angle := TAU * k / 16.0
				var p := hq + Vector3(cos(angle), 0, sin(angle)) * r
				if not placed and host.placement_check(o["type"], p.x, p.z).is_empty():
					placed = hud.place_at(Vector3(p.x, 0, p.z))
		hud.cancel_placing()
		flow._check(placed, "no spot for %s near the headquarters" % unit)
		return
	for s: Dictionary in host.structures():
		if s["owner"] != me or float(s["built"]) < 1.0 or not (s["queue"] as PackedInt32Array).is_empty():
			continue
		for o: Dictionary in host.production_options(s["id"]):
			if o["id"] == unit and String(o["blocked"]).is_empty():
				host.produce(s["id"], o["type"])
				return
