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
## - Prompt 052: communication trench and concrete teeth are lines; vehicle
##   pit, MG nest, dugout and camouflage net one click; a bunker two clicks
##   (its place, then the direction it faces). While a line waits for its
##   end, `works_view.gd` draws it with length, pieces and the total price
##   (`line_preview`).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Language := preload("res://scenes/menu/language.gd")
## Kinds of the command bar: id -> [label (translation key), needs a line].
const KINDS := {
	"mines_at": ["HUD_WORK_MINES_AT", false],
	"mines_ap": ["HUD_WORK_MINES_AP", false],
	"wire": ["HUD_WORK_WIRE", true],
	"hedgehogs": ["HUD_WORK_HEDGEHOGS", true],
	"foxhole": ["HUD_WORK_FOXHOLE", false],
	"trench": ["HUD_WORK_TRENCH", true],
	"clear": ["HUD_WORK_CLEAR", false],
	"bridge": ["HUD_WORK_BRIDGE", false],
	"comm_trench": ["HUD_WORK_COMM_TRENCH", true],
	"teeth": ["HUD_WORK_TEETH", true],
	"vehicle_pit": ["HUD_WORK_VEHICLE_PIT", false],
	"mg_nest": ["HUD_WORK_MG_NEST", false],
	"dugout": ["HUD_WORK_DUGOUT", false],
	"camo_net": ["HUD_WORK_CAMO_NET", false],
	"bunker": ["HUD_WORK_BUNKER", true],
}
## Kinds whose pieces are points along the line (one more than segments).
const POINT_LINES := ["hedgehogs", "teeth"]
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


## The line an engineer mode is drawing towards `cursor` (a ground point):
## `{from, to, length, pieces, cost, text}`, empty when no line waits.
static func line_preview(selection: Control, cursor: Vector3) -> Dictionary:
	var kind := kind_of(selection.command_mode)
	if kind.is_empty() or not selection.has_meta(START_META):
		return {}
	var start: Variant = selection.get_meta(START_META)
	if (start as Array)[0] != kind:
		return {}
	var sim_host: SimHost = selection.sim_host
	var a: Vector2 = (start as Array)[1]
	var from := Vector3(a.x, sim_host.height_at(a.x, a.y), a.y)
	var to := cursor
	var length := Vector2(to.x - from.x, to.z - from.z).length()
	var cost := sim_host.work_cost(kind)
	var pieces := 1
	if cost.size() >= 5 and cost[4] > 0 and kind != "bunker":
		var n := maxi(1, ceili(length * 1000.0 / float(cost[4])))
		pieces = n + 1 if kind in POINT_LINES and length > 0.0 else n
	var text := "%.0f m" % length
	if cost.size() >= 3:
		var total := [cost[0] * pieces, cost[1] * pieces, cost[2] * pieces]
		var per_m := ""
		if length >= 1.0 and kind != "bunker":
			per_m = "  " + Language.t("HUD_LINE_PER_M") % (float(total[0] + total[1] + total[2]) / length)
		text += "  ·  " + Language.t("HUD_LINE_COST") % [pieces, total[0], total[1], total[2]] + per_m
	return {"from": from, "to": to, "length": length, "pieces": pieces, "cost": cost, "text": text}


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
	var label := Language.t(KINDS[kind][0])
	match kind:
		"clear":
			return Language.t("HUD_HINT_WORK_CLEAR")
		"bridge":
			return Language.t("HUD_HINT_WORK_BRIDGE")
	if not KINDS[kind][1]:
		return Language.t("HUD_HINT_WORK_POINT") % label
	if kind == "bunker":
		if selection.has_meta(START_META):
			return Language.t("HUD_HINT_BUNKER_FACING")
		return Language.t("HUD_HINT_BUNKER_START")
	if selection.has_meta(START_META):
		return Language.t("HUD_HINT_LINE_END") % label
	return Language.t("HUD_HINT_LINE_START") % label
