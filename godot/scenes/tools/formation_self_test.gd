extends RefCounted
## Self-test part for formations and orders stage 2 (prompt 046), run by
## `self_test.gd` with its helpers (`_check`, `_frames`, `_ticks`, `_key`,
## `_mouse`, `_left_click`, `_screen_of_point`, `_ids_of`, `_tanks`,
## `_indices`, `_moving`, `_centroid`).
##
## The formation choice follows `selection.formation` and the `,` key; a
## right-drag sends the own tanks in a line facing the drag direction, and
## they arrive turned that way; Y + click attack-moves, the Secure button +
## click secures an area, which the waypoint overlay draws.

const TIME_SCALE := 4.0
const LINE := 1
const WEDGE := 2


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var selection: Control = main.selection
	var tanks := _alive(sim_host, test._tanks(test._ids_of(true)))
	test._check(tanks.size() >= 2, "formations: fewer than two own tanks left")
	if tanks.size() < 2:
		return
	tanks = tanks.slice(0, 3)
	var choice: OptionButton = main.hud.command_bar.formation_choice
	selection._set_selection(tanks)
	selection.set_formation(LINE)
	test._check(choice.selected == LINE, "formation choice shows %d, expected line" % choice.selected)
	test._key(KEY_COMMA)
	await test._frames(1)
	test._check(selection.formation == WEDGE, "',' gave formation %d, expected wedge" % selection.formation)
	selection.set_formation(LINE)

	# Right-drag: target at the press, facing along the drag (east).
	var center: Vector3 = test._centroid(tanks)
	var target := Vector3(center.x, 0.0, center.z - 30.0)
	target.y = sim_host.height_at(target.x, target.z)
	main.camera_rig.snap_to(target, 90.0, 0.0)
	await test._frames(3)
	var from: Vector2 = test._screen_of_point(target)
	var to: Vector2 = test._screen_of_point(target + Vector3(25.0, 0.0, 0.0))
	test._mouse(MOUSE_BUTTON_RIGHT, from, true)
	var motion := InputEventMouseMotion.new()
	motion.position = to
	motion.global_position = to
	motion.button_mask = MOUSE_BUTTON_MASK_RIGHT
	test.get_viewport().push_input(motion)
	test._mouse(MOUSE_BUTTON_RIGHT, to, false)
	Engine.time_scale = TIME_SCALE
	await test._ticks(5)
	test._check(test._moving(tanks), "the tanks did not start after the right-drag")
	var deadline: int = sim_host.current_tick() + 1600
	while sim_host.current_tick() < deadline and test._moving(tanks):
		await test._frames(1)
	await test._ticks(80)
	Engine.time_scale = 1.0
	var positions: Array[Vector3] = []
	var yaws := sim_host.unit_facing_yaws()
	var want := atan2(-1.0, -0.0)
	var worst := 0.0
	for i: int in test._indices(tanks):
		positions.append(sim_host.current_positions()[i])
		worst = maxf(worst, absf(angle_difference(yaws[i], want)))
	var spread_x := 0.0
	var spread_z := 0.0
	for p in positions:
		spread_x = maxf(spread_x, absf(p.x - target.x))
		spread_z = maxf(spread_z, absf(p.z - target.z))
	test._check(spread_x < 2.0 and spread_z > 5.0,
		"a line facing east should run north-south: spread x %.1f m, z %.1f m" % [spread_x, spread_z])
	test._check(worst < 0.1, "tanks face %.2f rad off the dragged direction" % worst)

	# Y: attack-move to the clicked point.
	test._key(KEY_Y)
	test._check(selection.command_mode == "attack_move", "Y did not start the attack-move order")
	test._left_click(test._screen_of_point(target + Vector3(0.0, 0.0, 30.0)))
	await test._ticks(2)
	var drill := sim_host.unit_drill(tanks[0])
	test._check(drill.x == 1.0 or drill.x == 2.0, "no attack-move after Y + click (%s)" % drill)

	# Secure button: hold an area.
	main.hud.command_bar.press("secure")
	test._left_click(test._screen_of_point(target))
	await test._ticks(2)
	drill = sim_host.unit_drill(tanks[0])
	test._check(drill.x == 3.0, "no secured area after Secure + click (%s)" % drill)
	await test._frames(2)
	test._check(main.hud.waypoints.secured > 0, "the secured area is not drawn")
	selection.stop_selected()
	await test._ticks(2)
	test._check(sim_host.unit_drill(tanks[0]).x == 0.0, "stop did not end securing")
	selection.set_formation(0)
	print("self-test: formations ok (line facing east, %.2f rad off; attack-move, secure)" % worst)


func _alive(sim_host: SimHost, ids: Array[int]) -> Array[int]:
	var all := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var result: Array[int] = []
	for id in ids:
		var i := all.find(id)
		if i >= 0 and alive[i] != 0:
			result.append(id)
	return result
