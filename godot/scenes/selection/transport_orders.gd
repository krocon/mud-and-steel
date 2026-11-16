extends RefCounted
## Transport orders of the selection (prompt 034). Presentation only: picks
## the order, the simulation checks seats, power and ownership.
##
## - Right click with squads selected on an own vehicle with seats (truck,
##   halftrack, tank): the squads mount it (`SimHost.embark_units`).
## - Right click with vehicles selected on an own towed gun: the nearest
##   vehicle strong enough hitches it (`SimHost.tow_gun`).
## - J (`order_disembark`): passengers of the selected vehicles get off.
## - K (`order_unlimber`): the selected vehicles drop their guns, selected
##   towed guns are unhitched.

## The order a right click at `screen_position` gives, if it is a
## transport order; false otherwise (the click then moves or attacks).
static func right_click(selection: Control, screen_position: Vector2) -> bool:
	if selection.selected.is_empty():
		return false
	var target: int = selection._unit_at(screen_position)
	if target < 0 or selection.selected.has(target):
		return false
	var sim_host: SimHost = selection.sim_host
	var squads: Array[int] = selection._selected_squads()
	if not squads.is_empty() and sim_host.unit_seats(target) > 0:
		sim_host.embark_units(PackedInt64Array(squads), target)
		selection.embark_ordered.emit(target)
		return true
	if sim_host.is_towed_gun(target):
		var tractors: Array[int] = []
		for id: int in selection.selected:
			if sim_host.can_tow(id, target):
				tractors.append(id)
		if not tractors.is_empty():
			sim_host.tow_gun(PackedInt64Array(tractors), target)
			selection.embark_ordered.emit(target)
			return true
	return false


## J: everybody off the selected vehicles.
static func disembark(selection: Control) -> bool:
	var loaded := _with(selection, selection.sim_host.unit_passengers(), 0)
	if loaded.is_empty():
		return false
	selection.sim_host.disembark_units(PackedInt64Array(loaded))
	return true


## K: the selected vehicles drop their guns; selected towed guns are
## unhitched.
static func unlimber(selection: Control) -> bool:
	var sim_host: SimHost = selection.sim_host
	var towing := _with(selection, sim_host.unit_towing(), -1)
	towing.append_array(_with(selection, sim_host.unit_carriers(), -1))
	if towing.is_empty():
		return false
	sim_host.unlimber_units(PackedInt64Array(towing))
	return true


## Whether the selection has a vehicle with passengers / a towing link
## (command bar).
static func counts(selection: Control) -> Vector2i:
	var sim_host: SimHost = selection.sim_host
	var towing := _with(selection, sim_host.unit_towing(), -1)
	towing.append_array(_with(selection, sim_host.unit_carriers(), -1))
	return Vector2i(_with(selection, sim_host.unit_passengers(), 0).size(), towing.size())


## Selected ids whose entry of `values` (order of `unit_ids`) differs from
## `none`.
static func _with(selection: Control, values: Variant, none: int) -> Array[int]:
	var ids: PackedInt64Array = selection.sim_host.unit_ids()
	var result: Array[int] = []
	for id: int in selection.selected:
		var i := ids.find(id)
		if i >= 0 and i < values.size() and int(values[i]) != none:
			result.append(id)
	return result
