extends RefCounted
## Self-test part for experience and ranks (prompt 050), run by
## `self_test.gd` with its helpers (`_check`, `_frames`, `_ticks`,
## `_ids_of`, `_tanks`, `_indices`, `_positions_of`). It runs after the
## replay test: `SimHost.debug_set_experience` is no command and would make
## the replay differ.
##
## - The own tank that destroyed the enemy tank in the combat test has
##   experience and a kill: its barrel carries as many kill rings, the unit
##   panel shows its rank insignia, rank line and experience bar.
## - An own squad made elite by debug: the event feed reports the
##   promotion and its soldiers wear worn uniforms (and, for factions with
##   captured camouflage, the garment in the forest).


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var view: Node = main.get("veterancy_view")
	var hud: Control = main.get("hud")
	test._check(view != null and hud != null, "experience: no veterancy view or HUD")
	if view == null or hud == null:
		return
	# The tank with a kill from the combat test.
	var ids := sim_host.unit_ids()
	var kills := sim_host.unit_vehicle_kills()
	var killer := -1
	for tank: int in test._tanks(test._ids_of(true)):
		var i := ids.find(tank)
		if kills[i] > 0:
			killer = tank
			break
	test._check(killer >= 0, "experience: no own tank has a kill after the combat test")
	if killer >= 0:
		var xp := sim_host.unit_experience(killer)
		test._check(int(xp.get("xp", 0)) > 0 and int(xp.get("kills", 0)) > 0,
			"experience: the killer has no points or kills: %s" % xp)
		view.refresh()
		var count := mini(int(xp.vehicle_kills), view.MAX_RINGS)
		test._check(view.rings_shown.get(killer, 0) == count,
			"experience: %s rings shown, %d kills" % [view.rings_shown.get(killer, 0), count])
		var gun := (main._unit_nodes[killer] as Node3D).find_child("Gun", true, false)
		if gun != null:
			var rings := gun.get_node_or_null("KillRings")
			test._check(rings != null and rings.get_child_count() == count,
				"experience: no kill rings on the barrel of tank %d" % killer)
		main.selection._set_selection([killer])
		await test._frames(20)
		var panel: Control = hud.unit_panel
		test._check(panel._badge.visible and panel._xp_bar.visible, "experience: no rank insignia or bar")
		test._check(panel.rank_line.begins_with(str(xp.rank_name)) and panel.rank_line.contains("kill"),
			"experience: rank line '%s'" % panel.rank_line)
		test._check(panel._title.text.contains(panel.rank_line), "experience: rank not in the title")
		test._check(panel._xp_bar.tooltip_text.contains("experience"), "experience: bar without tooltip")
		main.selection._set_selection([])

	# A squad made elite: promotion message and kit.
	var squads: Array[int] = test._ids_of(true, true)
	var alive := sim_host.unit_alive()
	var squad := -1
	for id: int in squads:
		if alive[ids.find(id)] != 0:
			squad = id
			break
	if squad < 0:
		print("self-test: experience ok (no own squad left for the kit check)")
		return
	var squad_only: Array[int] = [squad]
	main.camera_rig.snap_to(test._positions_of(squad_only)[0], 40.0, 0.0)
	await test._frames(5)
	view.refresh()
	var posted: int = view.promotions_posted
	sim_host.debug_set_experience(squad, 100000)
	await test._ticks(2)
	await test._frames(3)
	var rank: int = sim_host.unit_ranks()[sim_host.unit_ids().find(squad)]
	test._check(rank == 3, "experience: the squad is rank %d, not elite" % rank)
	view.refresh()
	test._check(view.promotions_posted == posted + 1, "experience: no promotion message")
	var feed: Node = hud.event_feed
	test._check(not feed.history.is_empty() and str(feed.history[-1].text).begins_with("Promoted"),
		"experience: the event feed shows no promotion")
	# Prompt 066: the bonuses of the rank as their own line in the unit
	# panel and on the unit card; the type's ratings stay what they are.
	main.selection.set_selection(squad_only)
	await test._frames(15)
	var squad_type := str(sim_host.unit_info(squad).type_id)
	hud.show_unit_card(squad_type, squad)
	await test._frames(2)
	var rank_label := hud.unit_card.find_child("Rank", true, false) as Label
	var bonus: String = hud.unit_panel.bonus_line
	test._check(not bonus.is_empty() and rank_label != null and rank_label.visible and rank_label.text == bonus,
		"experience: rank bonus line of the panel '%s', of the card '%s'" % [bonus, rank_label.text if rank_label != null else ""])
	test._check(hud.unit_card.rating_bars.ratings == SimHost.unit_stats(squad_type).get("ratings", {}),
		"experience: the rank changed the ratings on the card")
	hud.unit_card.close()
	main.selection.set_selection([])
	var infantry: Node = main.infantry_view
	var dressed := 0
	var worn := 0
	for key: int in infantry._soldiers:
		if key / 256 != squad:
			continue
		var s: Dictionary = infantry._soldiers[key]
		if int(s["rank"]) != 3:
			continue
		dressed += 1
		if _wears_worn_cloth(s["node"]):
			worn += 1
	test._check(dressed > 0, "experience: no soldier of squad %d got the elite kit" % squad)
	test._check(worn == dressed, "experience: %d of %d elite soldiers in worn cloth" % [worn, dressed])
	print("self-test: experience ok (tank %d: %d rings; squad %d elite, %d soldiers re-dressed)" % [
		killer, view.rings_shown.get(killer, 0), squad, dressed])


## Whether the soldier's uniform cloth has the worn tint.
static func _wears_worn_cloth(member: Node) -> bool:
	for mesh: MeshInstance3D in member.find_children("*", "MeshInstance3D", true, false):
		for i in mesh.get_surface_override_material_count():
			var mat := mesh.get_surface_override_material(i) as ShaderMaterial
			if mat != null and mat.resource_name == "Cloth":
				return _is_worn(mat)
	return false


static func _is_worn(mat: ShaderMaterial) -> bool:
	var color: Color = mat.get_shader_parameter(&"base_color")
	# Worn cloth moved towards the pale tint: lighter than the fresh colors.
	for faction: String in ["de", "su", "us", "uk"]:
		for biome: String in SoldierLook.BIOMES:
			var fresh := SoldierLook.slot_color(faction, biome, "Cloth")
			if color.is_equal_approx(fresh.lerp(SoldierLook.WORN_TINT, SoldierLook.WORN_MIX)):
				return true
	return false
