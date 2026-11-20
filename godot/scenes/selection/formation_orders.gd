extends RefCounted
## Formation orders of the selection (prompt 046). Presentation only: picks
## formation, facing and move mode; the simulation lays the formation out
## (Auto by the ground at the target) and decides who halts to fight.
##
## - Right click on the ground: `SimHost.formation_move` in the formation of
##   the command bar, infantry halts to fight enemies in range on the way
##   (`ENGAGE_INFANTRY`); right-drag sets the facing (press: target, release:
##   direction), the formation stands across it and tanks turn their front
##   that way.
## - M (move, then click): plain move, nobody halts (`ENGAGE_NEVER`).
## - Y (attack-move, then click): everybody halts to fight (`ENGAGE_ALL`).
## - I (secure, then click): hold a circle of `SECURE_RADIUS` meters.
## - `,` cycles the formation; control groups remember theirs.

## Display names (translation keys) by formation code (`SimHost.formation_names` order).
const LABELS: Array[String] = [
	"HUD_FORMATION_AUTO", "HUD_FORMATION_LINE", "HUD_FORMATION_WEDGE",
	"HUD_FORMATION_COLUMN", "HUD_FORMATION_BROAD_WEDGE", "HUD_FORMATION_DISPERSED",
]
const ENGAGE_NEVER := 0
const ENGAGE_INFANTRY := 1
const ENGAGE_ALL := 2
## Radius of a secured area in meters [provisional].
const SECURE_RADIUS := 80


## Moves the selection (or `ids`) to `point` in the selected formation,
## facing `facing` (zero: direction of travel).
static func move(selection: Control, ids: Array, point: Vector3, facing: Vector3, engage: int) -> void:
	if ids.is_empty():
		return
	selection.sim_host.formation_move(PackedInt64Array(ids), point.x, point.z,
		selection.formation, facing.x, facing.z, engage)
	selection.formation_ordered.emit(point, facing, ids.duplicate())


## The selection secures the circle around `point`.
static func secure(selection: Control, point: Vector3, facing := Vector3.ZERO) -> void:
	if selection.selected.is_empty():
		return
	selection.sim_host.secure_area(PackedInt64Array(selection.selected), point.x, point.z,
		SECURE_RADIUS, selection.formation, facing.x, facing.z)
	selection.formation_ordered.emit(point, facing, selection.selected.duplicate())
	selection.move_ordered.emit(point)


## Label (translation key) of a formation code.
static func label(code: int) -> String:
	return LABELS[code] if code >= 0 and code < LABELS.size() else LABELS[0]
