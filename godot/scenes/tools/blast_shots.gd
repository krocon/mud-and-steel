extends Node
## Big blasts and the heavy strike in pictures (prompt 054), started by
## `main.gd`:
##
##     godot --path godot --fixed-fps 30 -- --blast-shots=<dir>
##
## On the demo map with every unit holding fire and the fog of war off, it
## triggers the effects directly (the functions the events call) and saves
## PNG screenshots at fixed frames: a big blast in the open by day (shock
## dome, pressure and dust ring, flash), one next to the village, the same
## by night (the flash lights up ground and houses), a tank whose
## ammunition explodes (the turret flies), and a heavy strike: marker smoke
## and area ring, then the salvo, by day and by night. `just blast-shots`.
##
## With `--mode=base --strike-live` it plays a real strike instead: on the
## base skirmish (enemy passive) at high speed it builds depot, works and
## rocket battery position, waits for the unlock, calls the strike on the
## enemy headquarters through `SimHost.call_strike` and records the marked
## target with the countdown and the salvo; it fails (exit code 1) if the
## strike, the siren or the launch do not show. `just strike-shots`.

const SHOT_SIZE := Vector2i(1600, 900)
const CALIBER := 150.0
const SALVO := 24
const SALVO_RADIUS := 60.0
const SALVO_GAP_FRAMES := 7

var main: Node3D
var _dir := ""
var _shot_count := 0


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--blast-shots="):
			_dir = arg.trim_prefix("--blast-shots=")
	if "--strike-live" in OS.get_cmdline_user_args():
		var ok: bool = await _live()
		get_tree().quit(0 if ok else 1)
		return
	await _shots()
	get_tree().quit()


func _shots() -> void:
	if _dir.is_empty():
		_dir = "user://blast-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.camera_rig.edge_scroll_enabled = false
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	await _frames(10)
	var fx: Node3D = main.effects
	var blast: Node3D = main.blast_effects
	var own := _ids_of(true)
	var enemies := _ids_of(false)
	var tank_pos := _position_of(own[0])

	# A big blast in the open by day.
	var field := _ground(tank_pos + _forward(own[0]) * 45.0)
	await _view(field + Vector3(0, 0, 0), 75.0, 0.6)
	_big(field)
	await _capture("blast_day", [1, 2, 4, 7, 12, 24, 45])

	# Next to the village.
	var house := _house()
	var near := _ground(house + Vector3(14, 0, 10))
	await _view((house + near) * 0.5, 60.0, 0.9)
	await _frames(60)
	_big(near)
	await _capture("blast_village_day", [1, 3, 8])

	# A tank's ammunition explodes: the turret flies off.
	var victim := enemies[1]
	var victim_pos := _position_of(victim)
	await _view(victim_pos, 32.0, (main._unit_nodes[victim] as Node3D).rotation.y + PI * 0.5)
	fx.destroyed(victim, victim_pos)
	blast.ammo_explosion(victim, victim_pos)
	await _capture("ammo_explosion", [2, 4, 8, 14, 22, 32, 60, 90])

	# A heavy strike by day: marked target, then the salvo.
	var target := _ground((tank_pos + _position_of(enemies[0])) * 0.5 + Vector3(-40, 0, 0))
	await _strike(target, "strike_day")

	# Night (prompt 040): the flash lights up ground and houses.
	main.atmosphere.set_weather("clear", "night")
	await _frames(30)
	await _view((house + near) * 0.5, 60.0, 0.9)
	await _frames(240)
	await _capture("blast_village_night", [0])
	_big(near)
	await _capture("blast_village_night", [1, 2, 4, 8])
	await _view(field, 75.0, 0.6)
	_big(field)
	await _capture("blast_night", [1, 3])
	await _strike(target + Vector3(30, 0, 50), "strike_night")
	print("blast-shots: %d screenshots in %s; counts %s, strike %s" % [
		_shot_count, ProjectSettings.globalize_path(_dir), blast.counts, main.strike_view.counts])


# --- A real strike -------------------------------------------------------------


func _live() -> bool:
	if _dir.is_empty():
		_dir = "user://blast-shots"
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	var host: SimHost = main.sim_host
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	main.effects.visible_at = func(_point: Vector3) -> bool: return true
	await _frames(10)
	var me := host.local_player()
	var hq := _structure("base_hq", true)
	if hq.is_empty():
		printerr("strike-live: no headquarters (start with --mode=base)")
		return false
	host.speed = 30.0
	for id in ["base_depot", "base_factory", "base_rocket_battery"]:
		if not await _build(id, hq["position"]):
			printerr("strike-live: could not build %s" % id)
			host.speed = 1.0
			return false
	while not str(host.strike_option().get("blocked", "?")).is_empty():
		await _frames(10)
	host.speed = 1.0
	var target: Vector3 = _structure("base_hq", false).get("position", Vector3.ZERO)
	var view: Node3D = main.strike_view
	host.call_strike(target.x, target.z)
	var waited := 0
	while int(view.counts.get("called", 0)) == 0 and waited < 300:
		await _frames(1)
		waited += 1
	await _view(target, 170.0, 0.5)
	await _frames(240)
	var siren: bool = (view._siren as AudioStreamPlayer).playing
	var counting: bool = view._countdown.visible
	await _capture("strike_live_marked", [0])
	host.speed = 4.0
	waited = 0
	while int(view.counts.get("launched", 0)) == 0 and waited < 600:
		await _frames(1)
		waited += 1
	host.speed = 1.0
	# The first rockets land after their flight (at least 3 s).
	waited = 0
	while main.blast_effects.counts.get("big_blast", 0) < 3 and waited < 900:
		await _frames(1)
		waited += 1
	await _capture("strike_live_salvo", [10, 60, 140])
	await _view(_structure("base_rocket_battery", true).get("position", target), 45.0, 0.8)
	await _capture("strike_live_battery", [2])
	var ok: bool = int(view.counts.get("called", 0)) == 1 and int(view.counts.get("launched", 0)) == 1 \
		and siren and counting and main.blast_effects.counts.get("big_blast", 0) >= 10
	print("strike-live: %s; strike %s, blasts %s, siren %s, countdown %s, player %d" % [
		"PASSED" if ok else "FAILED", view.counts, main.blast_effects.counts, siren, counting, me])
	return ok


## The first structure of `type_id` (own or enemy): its dictionary with
## `position`.
func _structure(type_id: String, own: bool) -> Dictionary:
	var host: SimHost = main.sim_host
	for s: Dictionary in host.structures():
		if s["type_id"] == type_id and (int(s["owner"]) == host.local_player()) == own:
			var node: Node3D = main._unit_nodes.get(int(s["id"]))
			if node != null:
				s["position"] = node.global_position
			return s
	return {}


## Places `type_id` at the first free spot around `near` and waits until
## it is completed.
func _build(type_id: String, near: Vector3) -> bool:
	var host: SimHost = main.sim_host
	var unit_type := -1
	for o: Dictionary in host.build_options():
		if o["id"] == type_id:
			unit_type = int(o["type"])
	if unit_type < 0:
		return false
	var placed := false
	for r in range(20, 80, 6):
		for k in 16:
			var a := k * TAU / 16.0
			var p := near + Vector3(cos(a), 0.0, sin(a)) * r
			if placed or not host.placement_check(unit_type, p.x, p.z).is_empty():
				continue
			host.place_building(unit_type, p.x, p.z)
			placed = true
	if not placed:
		return false
	for i in 3000:
		await _frames(1)
		var s := _structure(type_id, true)
		if not s.is_empty() and float(s["built"]) >= 1.0:
			return true
	return false


func _big(at: Vector3) -> void:
	main.effects.ground_hit(at, CALIBER)
	main.blast_effects.big_blast(at, CALIBER)


## Marker smoke and ring for a few seconds, then the rockets.
func _strike(target: Vector3, name: String) -> void:
	var view: Node3D = main.strike_view
	var marker: Dictionary = view._make_marker({"id": 9000 + _shot_count, "owner": 1, "target": target,
		"radius": SALVO_RADIUS, "from": target + Vector3(0, 0, -300)}, false)
	await _view(target, 190.0, 0.5)
	await _frames(150)
	await _capture(name + "_marked", [0])
	for k in SALVO:
		var angle := k * 2.399963
		var r := SALVO_RADIUS * sqrt((k + 0.5) / SALVO)
		_big_rocket(_ground(target + Vector3(cos(angle) * r, 0, sin(angle) * r)))
		await _frames(SALVO_GAP_FRAMES)
		if k == 6 or k == 15:
			await _capture(name + "_salvo_%02d" % k, [0])
	await _capture(name + "_after", [20])
	view._retire(marker)


func _big_rocket(at: Vector3) -> void:
	main.effects.ground_hit(at, 132.0)
	main.blast_effects.big_blast(at, 132.0)


## A farm house of the village (the map's "village" marker).
func _house() -> Vector3:
	var found: PackedInt64Array = main.sim_host.destructible_objects_near("env_house_farm", -138.0, 138.0, 120.0)
	if found.is_empty():
		return _ground(Vector3(-138, 0, 138))
	return _ground(main.sim_host.object_position(found[0]))


func _view(focus: Vector3, distance: float, yaw: float) -> void:
	main.camera_rig.snap_to(focus, distance, yaw)
	await _frames(3)


## Saves screenshots `offsets` frames after now (frame 0 = the next one).
func _capture(name: String, offsets: Array) -> void:
	var frame := 0
	for offset: int in offsets:
		while frame <= offset:
			await _frame()
			frame += 1
		var image := get_viewport().get_texture().get_image()
		var path := "%s/%s_%02d.png" % [_dir, name, offset]
		image.save_png(path)
		_shot_count += 1
		print("blast-shots: %s" % path.get_file())


func _ids_of(own: bool) -> Array[int]:
	var sim_host: SimHost = main.sim_host
	var result: Array[int] = []
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	for i in ids.size():
		if (owners[i] == sim_host.local_player()) == own:
			result.append(ids[i])
	return result


func _position_of(id: int) -> Vector3:
	return (main._unit_nodes[id] as Node3D).global_position


func _forward(id: int) -> Vector3:
	return -(main._unit_nodes[id] as Node3D).global_basis.z


func _ground(point: Vector3) -> Vector3:
	return Vector3(point.x, main.sim_host.height_at(point.x, point.z), point.z)


func _frames(count: int) -> void:
	for i in count:
		await _frame()


func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
	await get_tree().process_frame
