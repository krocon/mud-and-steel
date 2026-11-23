extends Node
## Self-test of boats (prompt 119), started by `main.gd`:
##
##     godot --path godot --headless --fixed-fps 60 res://scenes/main.tscn -- --battle=dune_coast/boat_test --boat-test
##
## Part of `just selftest`. On `data/battles/dune_coast/boat_test.toml`: the LCVP at
## sea is selected; the command bar offers "Land", its click on the beach
## sends the boat there, the view draws a wake while it drives; it runs
## aground and lowers its ramp (the `Ramp` node turns); the squad on the
## sand boards it (Embark), "Back off" in the command bar takes it off the
## beach, a right click on the sand lands it again, and J puts the squad
## off onto the beach. Exit code 0 = passed.
##
## With `--boat-shots=<dir>` (needs a window) it saves pictures of the
## landing on the way (`just boat-shots`).

const TIME_SCALE := 4.0
const SCREEN := Vector2i(1280, 720)
const BoatOrders := preload("res://scenes/selection/boat_orders.gd")
const TransportOrders := preload("res://scenes/selection/transport_orders.gd")

var main: Node3D
var _failures := 0
var _shots := ""
var _shot_index := 0


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--boat-shots="):
			_shots = arg.trim_prefix("--boat-shots=")
			DirAccess.make_dir_recursive_absolute(_shots)
	if get_tree().root.size.x < SCREEN.x:
		get_tree().root.size = SCREEN
	await _run()
	Engine.time_scale = 1.0
	print("self-test: boats %s (%d failures)" % ["ok" if _failures == 0 else "FAILED", _failures])
	get_tree().quit(1 if _failures > 0 else 0)


func _run() -> void:
	var sim_host: SimHost = main.sim_host
	var own: Array[int] = []
	for i in sim_host.unit_ids().size():
		if sim_host.unit_owners()[i] == sim_host.local_player():
			own.append(sim_host.unit_ids()[i])
	sim_host.set_hold_fire(PackedInt64Array(own), true)
	await _frames(10)
	var boat := _first("us_lcvp")
	var squad := _first("us_inf_rifle_squad")
	_check(boat >= 0 and squad >= 0, "boat_test.toml units missing")
	if _failures > 0:
		return
	var view: Node3D = main.get("boat_view")
	_check(view != null, "no boat view")
	_check(sim_host.is_boat(boat) and not sim_host.is_boat(squad), "is_boat wrong")
	var beach := Vector3(-695.0, 0.0, -1000.0)
	_look_at(Vector3(-700.0, 0.0, -1080.0))

	# "Land" from the command bar, then the click on the beach.
	var selection: Control = main.selection
	var bar: Control = main.hud.command_bar
	selection.set_selection([boat])
	await _frames(12)
	var land: Button = bar.buttons.get("boat_land")
	_check(land != null and land.is_visible_in_tree() and not land.disabled, "no Land button for the boat")
	bar.press("boat_land")
	_check(selection.command_mode == BoatOrders.MODE, "Land does not wait for a click")
	BoatOrders.land_at(selection, beach)
	selection.set_command_mode("")
	Engine.time_scale = TIME_SCALE
	await _until(func() -> bool: return sim_host.boat_phase(boat) == "run_in", 2400)
	_check(view.wake_drawn > 0, "no wake behind the boat")
	_look_at(_position(boat), 55.0)
	await _shot("1_running_in")
	await _until(func() -> bool: return sim_host.boat_phase(boat) == "beached", 1200)
	_check(sim_host.boat_phase(boat) == "beached", "the boat did not beach (%s)" % sim_host.boat_phase(boat))
	await _frames(90)
	_check(view.ramps_moved > 0, "the ramp did not move")
	var ramp := _ramp(boat)
	_check(ramp != null and absf(ramp.rotation.x) > 0.5, "the ramp is not down (%s)" % (str(ramp.rotation) if ramp != null else "no Ramp node"))
	_look_at(_position(boat), 45.0)
	await _shot("2_ramp_down")

	# The squad boards over the ramp.
	sim_host.embark_units(PackedInt64Array([squad]), boat)
	await _until(func() -> bool: return _carrier(squad) == boat, 2400)
	_check(_carrier(squad) == boat, "the squad did not board the boat")
	bar._update()
	var off: Button = bar.buttons.get("boat_retract")
	_check(off != null and off.is_visible_in_tree() and not off.disabled, "no Back off button for the beached boat")
	bar.press("boat_retract")
	await _until(func() -> bool: return sim_host.boat_phase(boat) == "afloat", 1200)
	_check(sim_host.boat_phase(boat) == "afloat", "the boat did not back off (%s)" % sim_host.boat_phase(boat))
	await _frames(60)
	_check(ramp == null or absf(ramp.rotation.x) < 0.2, "the ramp did not come up")

	# Land again (a plain move onto the sand lands it), then J.
	sim_host.move_units(PackedInt64Array([boat]), beach.x + 10.0, beach.z + 20.0)
	await _until(func() -> bool: return sim_host.boat_phase(boat) == "beached", 2400)
	_check(sim_host.boat_phase(boat) == "beached", "a move onto the sand did not land the boat")
	_check(TransportOrders.disembark(selection), "J found nobody aboard")
	await _until(func() -> bool: return _carrier(squad) < 0, 1200)
	_check(_carrier(squad) < 0, "the squad did not get off")
	var at := _position(squad)
	_check(at.z > -1012.0, "the squad is not on the beach (%s)" % at)
	_look_at(at, 50.0)
	await _shot("3_ashore")
	Engine.time_scale = 1.0


func _look_at(point: Vector3, distance := 70.0) -> void:
	main.camera_rig.snap_to(point, distance, 0.4)


func _shot(name: String) -> void:
	if _shots.is_empty():
		return
	await _frames(20)
	_shot_index += 1
	var path := "%s/%s.png" % [_shots, name]
	get_viewport().get_texture().get_image().save_png(path)
	print("boat-shot: %s" % path)


func _ramp(boat: int) -> Node3D:
	var node: Node3D = main._unit_nodes.get(boat)
	return node.find_child("Ramp", true, false) as Node3D if node != null else null


func _until(done: Callable, ticks: int) -> void:
	var sim_host: SimHost = main.sim_host
	var deadline := sim_host.current_tick() + ticks
	var frames := ticks * 10
	while sim_host.current_tick() < deadline and frames > 0 and not done.call():
		frames -= 1
		await _frames(1)


func _first(type_id: String) -> int:
	var sim_host: SimHost = main.sim_host
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	for i in types.size():
		if types[i] == type_id:
			return ids[i]
	return -1


func _index(id: int) -> int:
	return main.sim_host.unit_ids().find(id)


func _carrier(id: int) -> int:
	var i := _index(id)
	return main.sim_host.unit_carriers()[i] if i >= 0 else -1


func _position(id: int) -> Vector3:
	var i := _index(id)
	return main.sim_host.current_positions()[i] if i >= 0 else Vector3.ZERO


func _check(ok: bool, message: String) -> void:
	if not ok:
		_failures += 1
		push_error("boat self-test: " + message)


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
