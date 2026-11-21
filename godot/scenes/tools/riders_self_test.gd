extends Node
## Self-test of the rider tiles (prompt 109), started by `main.gd`:
##
##     godot --path godot --headless --fixed-fps 60 res://scenes/main.tscn -- --scenario=riders.toml --riders-test
##
## Part of `just selftest`. On `data/scenarios/riders.toml`: three small
## squads mount the Opel Blitz and it hitches the Pak 40. With the truck
## selected the unit panel shows four tiles (three squads, the gun); a
## click on tile 2 puts exactly that squad off, it is selected outside and
## the other two still ride; a click plus Shift + click on the gun tile
## puts the first squad off and unhitches the gun, both selected at the
## end; J puts the last squad off. A tile of an enemy vehicle never shows.
## Exit code 0 = passed.
##
## With `--riders-shot=<file.png>` (needs a window) it saves the unit panel
## with the four tiles and quits (`just riders-shot`).

const Settings := preload("res://scenes/menu/settings.gd")
const TIME_SCALE := 4.0
const SCREEN := Vector2i(1280, 720)

var main: Node3D
var _failures := 0
var _shot := ""


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--riders-shot="):
			_shot = arg.trim_prefix("--riders-shot=")
			# No ground tooltip in the picture.
			Settings.values["ground_tooltip"] = "off"
	# The headless window is 64 x 64 pixels: test on the project's size.
	if get_tree().root.size.x < SCREEN.x:
		get_tree().root.size = SCREEN
	await _run()
	Engine.time_scale = 1.0
	if _shot.is_empty():
		print("self-test: riders %s (%d failures)" % ["ok" if _failures == 0 else "FAILED", _failures])
	get_tree().quit(1 if _failures > 0 else 0)


func _run() -> void:
	var sim_host: SimHost = main.sim_host
	# Hold fire, so the Pak 40 does not end the match by killing the enemy.
	var own: Array[int] = []
	for i in sim_host.unit_ids().size():
		if sim_host.unit_owners()[i] == sim_host.local_player():
			own.append(sim_host.unit_ids()[i])
	sim_host.set_hold_fire(PackedInt64Array(own), true)
	await _frames(10)
	var truck := _first("de_opel_blitz")
	var gun := _first("de_pak40")
	var squads: Array[int] = [_first("de_inf_mg42_team"), _first("de_inf_sniper"), _first("de_inf_medic")]
	var enemy := _first("su_zis5")
	_check(truck >= 0 and gun >= 0 and not squads.has(-1) and enemy >= 0, "riders.toml units missing")
	if _failures > 0:
		return
	squads.sort()
	Engine.time_scale = TIME_SCALE
	sim_host.embark_units(PackedInt64Array(squads), truck)
	await _until(func() -> bool: return squads.all(func(s: int) -> bool: return _carrier(s) == truck), 2400)
	_check(squads.all(func(s: int) -> bool: return _carrier(s) == truck), "squads did not mount the truck")
	sim_host.tow_gun(PackedInt64Array([truck]), gun)
	await _until(func() -> bool: return _carrier(gun) == truck and not _loading(gun) and not _loading(truck), 2400)
	_check(_carrier(gun) == truck, "the truck did not hitch the gun")
	await _until(func() -> bool: return not _moving(truck), 600)
	Engine.time_scale = 1.0

	var selection: Control = main.selection
	var panel: Control = main.hud.unit_panel
	var riders = panel.riders
	main.camera_rig.snap_to(_position(truck), 40.0, 0.0)
	selection.set_selection([truck])
	await _frames(20)
	var expected: Array[int] = squads.duplicate()
	expected.append(gun)
	_check(riders.visible and riders.rider_ids == expected, "rider tiles %s, expected %s" % [riders.rider_ids, expected])
	_check(riders._riders_of(enemy).is_empty(), "an enemy vehicle shows riders")
	var strength: Array[String] = []
	for tile: Control in riders._grid.get_children():
		strength.append(tile.strength)
	_check(strength == ["3/3", "2/2", "2/2", ""], "rider strengths %s, expected 3/3, 2/2, 2/2 and none for the gun" % [strength])
	if not _shot.is_empty():
		# The mouse away from the panel: no tooltip in the picture.
		get_viewport().warp_mouse(Vector2(40.0, 40.0))
		await _frames(90)
		await _save_shot(panel)
		return

	# Click on tile 2: exactly this squad gets off and is selected outside.
	var second: int = squads[1]
	await _click_tile(riders, 1, false)
	_check(riders.leaving == [second], "click on tile 2 left %s pending, expected [%d]" % [riders.leaving, second])
	Engine.time_scale = TIME_SCALE
	await _until(func() -> bool: return _carrier(second) < 0, 600)
	await _frames(3)
	Engine.time_scale = 1.0
	_check(_carrier(second) < 0, "squad %d still rides after its tile was clicked" % second)
	_check(_carrier(squads[0]) == truck and _carrier(squads[2]) == truck, "the other two squads got off too")
	_check(selection.selected == [second], "selection %s after getting off, expected [%d]" % [selection.selected, second])

	# Click plus Shift + click: a squad and the gun, both selected at the end.
	selection.set_selection([truck])
	await _frames(20)
	var left: Array[int] = [squads[0], squads[2], gun]
	_check(riders.rider_ids == left, "rider tiles %s after one got off, expected %s" % [riders.rider_ids, left])
	await _click_tile(riders, 0, false)
	await _click_tile(riders, 2, true)
	_check(riders.leaving == [squads[0], gun], "collected %s, expected %s" % [riders.leaving, [squads[0], gun]])
	Engine.time_scale = TIME_SCALE
	await _until(func() -> bool: return _carrier(squads[0]) < 0 and _carrier(gun) < 0, 1200)
	await _frames(3)
	Engine.time_scale = 1.0
	_check(_carrier(squads[0]) < 0 and _carrier(gun) < 0, "squad %d or gun %d still aboard" % [squads[0], gun])
	_check(_carrier(squads[2]) == truck, "the last squad got off with the others")
	var both: Array[int] = [squads[0], gun]
	both.sort()
	_check(selection.selected == both, "selection %s, expected %s" % [selection.selected, both])

	# J: everybody off.
	selection.set_selection([truck])
	await _frames(20)
	_check(riders.rider_ids == [squads[2]], "rider tiles %s, expected [%d]" % [riders.rider_ids, squads[2]])
	_key(KEY_J)
	Engine.time_scale = TIME_SCALE
	await _until(func() -> bool: return _carrier(squads[2]) < 0, 600)
	await _frames(15)
	Engine.time_scale = 1.0
	_check(_carrier(squads[2]) < 0, "squad %d still rides after J" % squads[2])
	_check(not riders.visible and riders.rider_ids.is_empty(), "rider tiles stay after J: %s" % [riders.rider_ids])
	_check(selection.selected == [truck], "J changed the selection to %s" % [selection.selected])
	_check(_index(enemy) >= 0 and sim_host.unit_alive()[_index(enemy)] != 0, "the match ended early (enemy truck destroyed)")


## A real left click (Shift + click) in the middle of tile `n`.
func _click_tile(riders: Control, n: int, shift: bool) -> void:
	var tiles: Array[Node] = riders._grid.get_children()
	_check(n < tiles.size(), "no rider tile %d" % n)
	if n >= tiles.size():
		return
	var at: Vector2 = (tiles[n] as Control).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = at
	motion.global_position = at
	get_viewport().push_input(motion)
	for pressed in [true, false]:
		var click := InputEventMouseButton.new()
		click.button_index = MOUSE_BUTTON_LEFT
		click.pressed = pressed
		click.shift_pressed = shift
		click.position = at
		click.global_position = at
		get_viewport().push_input(click)
	await _frames(2)


func _key(keycode: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		event.physical_keycode = keycode
		event.pressed = pressed
		get_viewport().push_input(event)


## The unit panel as a picture (window mode only).
func _save_shot(panel: Control) -> void:
	await _frames(30)
	var image := get_viewport().get_texture().get_image()
	var factor := Vector2(image.get_size()) / get_viewport().get_visible_rect().size
	var area := panel.get_global_rect().grow(8.0)
	area.position.y -= 60.0
	area.size.y += 60.0
	var rect := Rect2i(Rect2(area.position * factor, area.size * factor))
	rect = rect.intersection(Rect2i(Vector2i.ZERO, image.get_size()))
	image.get_region(rect).save_png(_shot)
	get_viewport().get_texture().get_image().save_png(_shot.get_basename() + "_full.png")
	print("riders-shot: %s" % _shot)


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
	var owners := sim_host.unit_owners()
	var ids := sim_host.unit_ids()
	for i in types.size():
		if types[i] == type_id and (owners[i] == 0) == type_id.begins_with("de_"):
			return ids[i]
	return -1


func _index(id: int) -> int:
	return main.sim_host.unit_ids().find(id)


func _carrier(id: int) -> int:
	var i := _index(id)
	return main.sim_host.unit_carriers()[i] if i >= 0 else -1


func _loading(id: int) -> bool:
	var i := _index(id)
	return i >= 0 and main.sim_host.unit_loading()[i] != 0


func _moving(id: int) -> bool:
	var i := _index(id)
	return i >= 0 and main.sim_host.unit_moving()[i] != 0


func _position(id: int) -> Vector3:
	var i := _index(id)
	return main.sim_host.current_positions()[i] if i >= 0 else Vector3.ZERO


func _check(ok: bool, message: String) -> void:
	if not ok:
		_failures += 1
		push_error("riders self-test: " + message)


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
