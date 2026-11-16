extends RefCounted
## Engineer orders of the selection (prompt 035). Presentation only: picks
## the order, the simulation checks engineers, ground, money and limits.
##
## The command bar's engineer buttons set the targeted mode `work_<kind>`
## (`selection.command_mode`); the next left click(s) give the order to the
## selected engineer squads:
##
## - AT mines, AP mines, foxhole: one click (a mine section faces the
##   engineers' side).
## - Wire, obstacles, trench: two clicks, the start and the end of the line.
## - Clear: one click; the engineers clear the known mines, wire and
##   obstacles around it.
## - Bridge: one click near a bridge; an intact one is demolished, a
##   destroyed one repaired.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
## Kinds of the command bar: id -> [label, needs a line].
const KINDS := {
	"mines_at": ["AT mines", false],
	"mines_ap": ["AP mines", false],
	"wire": ["Wire", true],
	"hedgehogs": ["Obstacles", true],
	"foxhole": ["Foxhole", false],
	"trench": ["Trench", true],
	"clear": ["Clear", false],
	"bridge": ["Bridge", false],
}
const START_META := &"work_line_start"


## The kind of a mode ("" if it is no engineer mode).
static func kind_of(mode: String) -> String:
	return mode.trim_prefix("work_") if mode.begins_with("work_") else ""


## Carries out the engineer mode at `screen_position`; true when the order
## went out (the mode ends), false while a line waits for its end.
static func click(selection: Control, screen_position: Vector2) -> bool:
	var kind := kind_of(selection.command_mode)
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if kind.is_empty() or hit == null:
		return false
	return order(selection, kind, hit as Vector3)


## Gives the order of `kind` at the ground point `point` (two calls for a
## line; also used by the self-test).
static func order(selection: Control, kind: String, point: Vector3) -> bool:
	var sim_host: SimHost = selection.sim_host
	var units := PackedInt64Array(engineers(selection))
	if units.is_empty():
		return true
	var at := Vector2(point.x, point.z)
	match kind:
		"clear":
			sim_host.clear_works(units, at.x, at.y)
			return true
		"bridge":
			sim_host.order_bridge_work(units, at.x, at.y)
			return true
	if not KINDS.has(kind):
		return true
	if not KINDS[kind][1]:
		sim_host.order_works(units, kind, at, at)
		return true
	var start: Variant = selection.get_meta(START_META, null)
	if start == null or (start as Array)[0] != kind:
		selection.set_meta(START_META, [kind, at])
		selection.queue_redraw()
		return false
	selection.remove_meta(START_META)
	sim_host.order_works(units, kind, (start as Array)[1], at)
	return true


## Forgets a line start (the mode changed).
static func reset(selection: Control) -> void:
	if selection.has_meta(START_META):
		selection.remove_meta(START_META)


## The selected engineer squads.
static func engineers(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id: int in selection.selected:
		if selection.sim_host.is_engineer(id):
			result.append(id)
	return result


## Hint while an engineer mode waits for its click ("" for other modes).
static func hint(selection: Control) -> String:
	var kind := kind_of(selection.command_mode)
	if kind.is_empty() or not KINDS.has(kind):
		return ""
	var label: String = KINDS[kind][0]
	match kind:
		"clear":
			return "Clear: left click the area"
		"bridge":
			return "Bridge: left click a bridge (demolish / repair)"
	if not KINDS[kind][1]:
		return "%s: left click a point" % label
	if selection.has_meta(START_META):
		return "%s: left click the end of the line" % label
	return "%s: left click the start of the line" % label
