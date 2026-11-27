extends RefCounted
## Engineer orders of the selection (prompt 035, fluid drag lines prompt 235).
## Presentation only: picks the order, the simulation checks engineers, ground,
## money and limits.
##
## The command bar's engineer buttons set the targeted mode `work_<kind>`
## (`selection.command_mode`):
##
## - AT mines, AP mines, foxhole, vehicle pit, MG nest, dugout, camo net,
##   clear, bridge: one left click.
## - Sandbags, wire, obstacles, teeth, trench, comm. trench (`DRAG_LINES`):
##   fluid left-press, drag, release. A short click without dragging
##   (`< SHORT_DRAG_M`) places a standard 2-section line perpendicular to the
##   engineers / camera direction. Right click or Escape cancels the drag.
## - Bunker: two left clicks (its place at a trench edge, then facing).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Language := preload("res://scenes/menu/language.gd")
## Kinds of the command bar: id -> [label (translation key), needs a line].
const KINDS := {
	"mines_at": ["HUD_WORK_MINES_AT", false],
	"mines_ap": ["HUD_WORK_MINES_AP", false],
	"wire": ["HUD_WORK_WIRE", true],
	"sandbags": ["HUD_WORK_SANDBAGS", true],
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
## Line works drawn by left-press, drag and release (prompt 235).
const DRAG_LINES := ["sandbags", "wire", "hedgehogs", "trench", "comm_trench", "teeth"]
## Kinds whose pieces are points along the line (one more than segments).
const POINT_LINES := ["hedgehogs", "teeth"]
## Drag shorter than this (m) places a 2-section default line perpendicular to
## the engineer/camera direction.
const SHORT_DRAG_M := 1.5
const START_META := &"work_line_start"
const DRAG_META := &"work_line_drag"


## The kind of a mode ("" if it is no engineer mode).
static func kind_of(mode: String) -> String:
	return mode.trim_prefix("work_") if mode.begins_with("work_") else ""


## Whether `mode` is a drag-to-draw line work (prompt 235).
static func is_drag_line(mode: String) -> bool:
	return kind_of(mode) in DRAG_LINES


## Whether a drag-line gesture is currently active on `selection`.
static func dragging(selection: Control) -> bool:
	return selection.has_meta(DRAG_META)


## Starts a left-button drag for a line work at `screen_position`.
static func begin_drag(selection: Control, screen_position: Vector2) -> bool:
	var kind := kind_of(selection.command_mode)
	if not (kind in DRAG_LINES):
		return false
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if hit == null:
		return false
	var p := hit as Vector3
	var start := Vector2(p.x, p.z)
	selection.set_meta(DRAG_META, {"kind": kind, "from": start, "to": start})
	selection.set_meta(START_META, [kind, start])
	selection.queue_redraw()
	return true


## Updates the current drag endpoint from `screen_position`.
static func update_drag(selection: Control, screen_position: Vector2) -> void:
	if not selection.has_meta(DRAG_META):
		return
	var hit: Variant = GroundPick.from_screen(selection.camera, screen_position)
	if hit == null:
		return
	var p := hit as Vector3
	var state: Dictionary = selection.get_meta(DRAG_META)
	state["to"] = Vector2(p.x, p.z)
	selection.set_meta(DRAG_META, state)


## Cancels an active drag without sending an order.
static func cancel_drag(selection: Control) -> void:
	if selection.has_meta(DRAG_META):
		selection.remove_meta(DRAG_META)
	if selection.has_meta(START_META):
		selection.remove_meta(START_META)
	selection.queue_redraw()


## Finishes the drag at `screen_position` and sends the work order. Returns
## whether an order was issued (so non-Shift mode should end).
static func end_drag(selection: Control, screen_position: Vector2) -> bool:
	if not selection.has_meta(DRAG_META):
		return false
	update_drag(selection, screen_position)
	var state: Dictionary = selection.get_meta(DRAG_META)
	cancel_drag(selection)
	var kind: String = state["kind"]
	var units := PackedInt64Array(engineers(selection))
	if units.is_empty():
		return true
	var endpoints := _resolved_endpoints(selection, kind, state["from"] as Vector2, state["to"] as Vector2)
	selection.sim_host.order_works(units, kind, endpoints[0], endpoints[1])
	return true


## Active drag endpoints `[kind, from_2d, to_2d]` (empty if not dragging and
## not waiting on `START_META`). Used by `order_preview.gd`.
static func active_line(selection: Control, cursor: Vector3) -> Dictionary:
	var kind := kind_of(selection.command_mode)
	if kind.is_empty():
		return {}
	var raw_from: Vector2
	var raw_to: Vector2
	if selection.has_meta(DRAG_META):
		var state: Dictionary = selection.get_meta(DRAG_META)
		if state["kind"] != kind:
			return {}
		raw_from = state["from"] as Vector2
		raw_to = state["to"] as Vector2
	elif selection.has_meta(START_META):
		var start: Array = selection.get_meta(START_META)
		if start[0] != kind:
			return {}
		raw_from = start[1] as Vector2
		raw_to = Vector2(cursor.x, cursor.z)
	else:
		return {}
	if kind in DRAG_LINES:
		var resolved := _resolved_endpoints(selection, kind, raw_from, raw_to)
		return {"kind": kind, "from": resolved[0], "to": resolved[1], "short": (raw_to - raw_from).length() < SHORT_DRAG_M}
	return {"kind": kind, "from": raw_from, "to": raw_to, "short": false}


## Computes `[from_2d, to_2d]`: if the drag is shorter than `SHORT_DRAG_M`,
## returns a 2-section default line centered at `from` perpendicular to the
## engineer-to-point (or camera forward) direction.
static func _resolved_endpoints(selection: Control, kind: String, from: Vector2, to: Vector2) -> Array[Vector2]:
	if (to - from).length() >= SHORT_DRAG_M:
		return [from, to]
	var sim_host: SimHost = selection.sim_host
	var cost := sim_host.work_cost(kind)
	var piece_m := float(cost[4]) / 1000.0 if cost.size() >= 5 and cost[4] > 0 else 3.0
	var towards := _default_facing(selection, from)
	var along := Vector2(-towards.y, towards.x).normalized()
	if along.length_squared() < 0.01:
		along = Vector2(1.0, 0.0)
	var half := along * piece_m  # 2 * piece_m total -> 2 standard sections
	return [from - half, from + half]


static func _default_facing(selection: Control, at: Vector2) -> Vector2:
	var eng := engineers(selection)
	var sim_host: SimHost = selection.sim_host
	if not eng.is_empty():
		var ids := sim_host.unit_ids()
		var positions := sim_host.current_positions()
		var center := Vector2.ZERO
		var count := 0
		for id: int in eng:
			var idx := ids.find(id)
			if idx >= 0 and idx < positions.size():
				center += Vector2(positions[idx].x, positions[idx].z)
				count += 1
		if count > 0:
			var v := at - center / float(count)
			if v.length() > 0.25:
				return v.normalized()
	var camera: Camera3D = selection.camera
	if camera != null:
		var fwd := -camera.global_transform.basis.z
		var f2 := Vector2(fwd.x, fwd.z)
		if f2.length() > 0.01:
			return f2.normalized()
	return Vector2(0.0, -1.0)


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
	var active := active_line(selection, cursor)
	if active.is_empty():
		return {}
	var kind: String = active["kind"]
	var sim_host: SimHost = selection.sim_host
	var a: Vector2 = active["from"]
	var b: Vector2 = active["to"]
	var from := Vector3(a.x, sim_host.height_at(a.x, a.y), a.y)
	var to := Vector3(b.x, sim_host.height_at(b.x, b.y), b.y)
	var length := (b - a).length()
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
	return {"kind": kind, "from": from, "to": to, "length": length, "pieces": pieces, "cost": cost, "text": text}


## Forgets a line start or active drag (the mode changed).
static func reset(selection: Control) -> void:
	if selection.has_meta(DRAG_META):
		selection.remove_meta(DRAG_META)
	if selection.has_meta(START_META):
		selection.remove_meta(START_META)


## The selected engineer squads.
static func engineers(selection: Control) -> Array[int]:
	var result: Array[int] = []
	for id: int in selection.selected:
		if selection.sim_host.is_engineer(id):
			result.append(id)
	return result


## Hint while an engineer mode waits for its click or drag ("" for other modes).
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
	if kind in DRAG_LINES:
		if selection.has_meta(DRAG_META):
			return Language.t("HUD_HINT_LINE_DRAGGING") % label
		return Language.t("HUD_HINT_LINE_DRAG") % label
	if selection.has_meta(START_META):
		return Language.t("HUD_HINT_LINE_END") % label
	return Language.t("HUD_HINT_LINE_START") % label
