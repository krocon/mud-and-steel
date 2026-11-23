extends RefCounted
## Orders of boats (prompt 119). Presentation only: the simulation plans
## the way over the water, finds where the boat grounds and checks the
## cargo.
##
## - "Land" (`selection.command_mode` = `boat_land`): the next left click
##   on a shore sends the selected boats there; they run onto the beach
##   and lower the ramp.
## - Right click with boats selected on ground they cannot float on (a
##   beach, a bank): they land there.
## - "Back off": beached boats raise the ramp and back off.
## - Right click with vehicles or guns selected on an own boat: they drive
##   aboard once it lies on the beach (`TransportOrders`).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Language := preload("res://scenes/menu/language.gd")
const MODE := "boat_land"


## The selected boats.
static func boats(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id: int in selection.selected:
		if selection.sim_host.is_boat(id):
			result.append(id)
	return result


## The selected boats lying on a beach (they can back off).
static func beached(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id in boats(selection):
		if selection.sim_host.boat_phase(id) in ["beached", "ramp", "run_in"]:
			result.append(id)
	return result


## Lands the selected boats at the ground point under `screen_position`;
## true when the order went out (the mode ends).
static func click(selection: Control, screen_position: Vector2) -> bool:
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if hit == null:
		return false
	return land_at(selection, hit as Vector3)


## Lands the selected boats at `point` (also used by the self-test).
static func land_at(selection: Control, point: Vector3) -> bool:
	var units := PackedInt64Array(boats(selection))
	if units.is_empty():
		return true
	if selection.sim_host.land_boats(units, point.x, point.z):
		selection.move_ordered.emit(point)
	return true


## A right click of a selection of only boats on ground they cannot float
## on lands them there; false otherwise (the click moves them).
static func right_click(selection: Control, screen_position: Vector2) -> bool:
	var own := boats(selection)
	if own.is_empty() or own.size() != selection.selected.size():
		return false
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if hit == null:
		return false
	var point := hit as Vector3
	if selection.sim_host.boat_can_float(own[0], point.x, point.z):
		return false
	return land_at(selection, point)


## The selected beached boats back off.
static func retract(selection: Control) -> void:
	var units := PackedInt64Array(beached(selection))
	if not units.is_empty():
		selection.sim_host.retract_boats(units)


## Hint while the landing mode waits for its click.
static func hint(selection: Control) -> String:
	if selection.command_mode != MODE:
		return ""
	return Language.t("HUD_HINT_LAND")
