extends RefCounted
## Orders of bridge-laying tanks and camouflage (prompt 055). Presentation
## only: the simulation finds the gap, checks the tank and the units.
##
## - "Lay bridge" (`selection.command_mode` = `bridge_lay`): the next left
##   click near a narrow river or ditch sends the selected bridge-laying
##   tanks there; the simulation picks the gap nearest to the click.
## - "Take up": the selected bridge-laying tanks fetch their laid bridge.
## - "Camouflage": the selected vehicles and guns put up branches and nets,
##   or take them off when all of them already have them.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const MODE := "bridge_lay"


## The selected bridge-laying tanks.
static func layers(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id: int in selection.selected:
		if selection.sim_host.is_bridgelayer(id):
			result.append(id)
	return result


## The selected units that may put up camouflage.
static func camouflageable(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id: int in selection.selected:
		if selection.sim_host.can_camouflage(id):
			result.append(id)
	return result


## Lays the bridges at the ground point under `screen_position`; true when
## the order went out (the mode ends).
static func click(selection: Control, screen_position: Vector2) -> bool:
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if hit == null:
		return false
	return lay_at(selection, hit as Vector3)


## Lays the bridges of the selected tanks at the gap nearest to `point`
## (also used by the self-test). Returns whether the order went out.
static func lay_at(selection: Control, point: Vector3) -> bool:
	var units := PackedInt64Array(layers(selection))
	if units.is_empty():
		return true
	selection.sim_host.lay_bridge(units, point.x, point.z)
	return true


## The selected tanks take their bridges up.
static func pick_up(selection: Control) -> void:
	var units := PackedInt64Array(layers(selection))
	if not units.is_empty():
		selection.sim_host.pick_up_bridge(units)


## Puts camouflage on the selection, or takes it off if every one has it.
static func toggle_camouflage(selection: Control) -> void:
	var units := camouflageable(selection)
	if units.is_empty():
		return
	var all_on := true
	for id in units:
		if selection.sim_host.camouflage_state(id) == 0:
			all_on = false
	selection.sim_host.set_camouflage(PackedInt64Array(units), not all_on)


## Hint while the bridge mode waits for its click.
static func hint(selection: Control) -> String:
	if selection.command_mode != MODE:
		return ""
	return "Lay bridge: left click a narrow river or ditch"
