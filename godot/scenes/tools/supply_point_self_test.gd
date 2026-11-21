extends RefCounted
## Self-test part for readable supply points (prompt 107), run by
## `menu/flow_self_test.gd` at the start of match 4 (base building on
## test_small, which has supply points; `_check`, `_frames`).
##
## - Every point shows on the minimap as a neutral mark in the neutral color.
## - The border line's radius is the one of the data (`radius` of
##   `SimHost.supply_points`, from `[supply] radius_m`).
## - `supply_border` = `off` hides the lines, `subtle` weakens them.
## - The ground tooltip over a point names it with its income.
## - The own units take the nearest point: its minimap mark turns to the
##   own color and shape, and the feed says "supply point taken".

const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const SupplyView := preload("res://scenes/world/supply_view.gd")


func run(test: Node, main: Node3D) -> void:
	var host: SimHost = main.sim_host
	var supply: SupplyView = main.base_view.supply
	var minimap: Control = main.hud.minimap
	await test._frames(10)
	test._check(supply.known.size() >= 1 and minimap.point_marks.size() == supply.known.size(),
		"supply points: %d known, %d minimap marks" % [supply.known.size(), minimap.point_marks.size()])
	if supply.known.is_empty():
		return
	for mark: Dictionary in minimap.point_marks:
		test._check(mark["relation"] == "neutral" and mark["color"] == RelationColors.neutral(),
			"supply point not neutral on the minimap at the start: %s" % mark)
	# The border lies at the radius of the data.
	var data_radius: float = (host.supply_points()[0] as Dictionary)["radius"]
	test._check(data_radius > 1.0, "no supply radius from the data (%s)" % data_radius)
	for i in supply.known.size():
		var box: AABB = supply.borders[i].get_aabb()
		var radius := (box.size.x - SupplyView.LINE_WIDTH) * 0.5
		test._check(absf(radius - data_radius) < 0.2 and supply.borders[i].visible,
			"supply border radius %.2f m, data %.2f m (visible %s)" % [radius, data_radius, supply.borders[i].visible])
	# Strength setting.
	Settings.set_value("supply_border", "off")
	await test._frames(3)
	test._check(supply.borders.all(func(b: MeshInstance3D) -> bool: return not b.visible), "supply_border off leaves a line")
	Settings.set_value("supply_border", "subtle")
	await test._frames(3)
	var subtle: float = (supply.borders[0].material_override as StandardMaterial3D).albedo_color.a
	Settings.set_value("supply_border", "strong")
	await test._frames(3)
	var strong: float = (supply.borders[0].material_override as StandardMaterial3D).albedo_color.a
	test._check(supply.borders[0].visible and subtle > 0.0 and subtle < strong * 0.6,
		"subtle supply border not weaker (%.2f, strong %.2f)" % [subtle, strong])
	# The nearest point to the own units.
	var me := host.local_player()
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var positions := host.current_positions()
	var own: Array[int] = []
	var center := Vector3.ZERO
	for i in ids.size():
		if owners[i] == me and not main.base_view.structures.has(ids[i]):
			own.append(ids[i])
			center += positions[i]
	test._check(not own.is_empty(), "supply points: no own units to take one")
	if own.is_empty():
		return
	center /= own.size()
	var target := 0
	for i in supply.known.size():
		if center.distance_to(supply.known[i].position) < center.distance_to(supply.known[target].position):
			target = i
	var at: Vector3 = supply.known[target].position
	# The tooltip over it.
	var tooltip: Control = main.hud.ground_tooltip
	tooltip.show_at(at, Vector2(400, 300))
	var income: PackedInt64Array = supply.known[target].income
	test._check(tooltip.title == test.tr("GROUND_POINT") and tooltip.lines.size() >= 2 and tooltip.lines[1].contains(str(income[0])),
		"ground tooltip over a supply point: '%s' %s" % [tooltip.title, tooltip.lines])
	tooltip.visible = false
	main.selection._set_selection(own)
	main.selection.order_move_to(at)
	host.speed = 8.0
	var frames := 3000
	while int(supply.known[target].owner) != me and frames > 0:
		frames -= 1
		await test._frames(1)
	host.speed = 1.0
	await test._frames(5)
	var mark: Dictionary = minimap.point_marks[target]
	test._check(int(supply.known[target].owner) == me and mark["relation"] == "own" and mark["color"] == RelationColors.own(),
		"supply point not taken or not own on the minimap: owner %d, mark %s" % [supply.known[target].owner, mark])
	test._check(int(supply.posted.get("point_taken", 0)) >= 1, "no 'supply point taken' message (%s)" % supply.posted)
	main.selection._set_selection([])
	print("flow self-test: supply points ok (%d points, radius %.0f m, point %d taken, tick %d)" % [supply.known.size(), data_radius, target, host.current_tick()])
