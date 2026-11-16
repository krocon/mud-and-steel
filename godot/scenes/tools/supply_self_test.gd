extends RefCounted
## Self-test part for supply and repair (prompt 036), run by `self_test.gd`
## after the other match tests (`_check`, `_frames`, `_ids_of`, `_tanks`).
##
## The own tanks have driven and fired by now: their stocks are below full,
## the unit panel shows the ammunition and fuel bars, and a tank is no
## supply source (no ring). The test match has no trucks or base, so the
## rings of real sources are covered by the simulation tests
## (`crates/sim/tests/logistics.rs`).


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var hud: Control = main.hud
	var tanks := _alive(sim_host, test._tanks(test._ids_of(true)))
	test._check(not tanks.is_empty() and hud != null, "supply: no own tank or no HUD")
	if tanks.is_empty() or hud == null:
		return
	var ids := sim_host.unit_ids()
	var ammo := sim_host.unit_ammo()
	var fuel := sim_host.unit_fuel()
	var used_ammo := false
	var used_fuel := false
	for tank in tanks:
		var i := ids.find(tank)
		test._check(ammo[i] >= 0 and ammo[i] <= 100, "tank %d has no ammunition stock (%d)" % [tank, ammo[i]])
		test._check(fuel[i] >= 0 and fuel[i] <= 100, "tank %d has no fuel stock (%d)" % [tank, fuel[i]])
		used_ammo = used_ammo or ammo[i] < 100
		used_fuel = used_fuel or fuel[i] < 100
	test._check(used_fuel, "no own tank used fuel")
	var squads := _alive(sim_host, test._ids_of(true, true))
	if not squads.is_empty():
		test._check(sim_host.unit_fuel()[ids.find(squads[0])] == -1, "a squad has a fuel stock")
	var tank: int = tanks[0]
	main.selection._set_selection([tank])
	await test._frames(20)
	var panel: Control = hud.unit_panel
	test._check(panel._ammo_bar.visible and panel._fuel_bar.visible, "unit panel shows no stock bars")
	var i := ids.find(tank)
	var now := sim_host.unit_ammo()[sim_host.unit_ids().find(tank)]
	test._check(int(panel._ammo_bar.value) == maxi(now, 0), "ammunition bar shows %d, stock %d" % [panel._ammo_bar.value, now])
	test._check(panel._fuel_bar.tooltip_text.begins_with("Fuel "), "fuel bar has no tooltip")
	test._check((fuel[i] == 0) == panel.supply_warning.contains("OUT OF FUEL"), "fuel warning wrong: %s" % panel.supply_warning)
	test._check(hud.supply_rings.rings == 0, "a tank draws a supply ring")
	test._check(sim_host.supply_radius(tank).is_empty(), "a tank counts as supply source")
	main.selection._set_selection([])
	print("self-test: supply ok (tank %d: ammunition %d %%, fuel %d %%; any fired: %s)" % [
		tank, ammo[ids.find(tank)], fuel[ids.find(tank)], used_ammo])


func _alive(sim_host: SimHost, ids: Array[int]) -> Array[int]:
	var all := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var result: Array[int] = []
	for id in ids:
		var i := all.find(id)
		if i >= 0 and alive[i] != 0:
			result.append(id)
	return result
