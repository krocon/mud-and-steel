extends Control
## Battlefield world feedback overlay (prompt 236). Presentation only:
##
## - Promotion pop-ups: rising golden banner ("PROMOTED" / "BEFÖRDERT" + stars
##   + rank name) directly over newly promoted own units (`veterancy_view.gd`).
## - World notices: short rising warning above a clicked world spot (e.g. full
##   or enemy-occupied building from `selection.gd`).
## - Building garrison roof plaques: faction bar, house icon, `occupants/capacity`,
##   veterancy stars and compact health bar directly over occupied buildings
##   (fog-of-war faithful for enemies).
## - Supply point labels: discreet status/income label over the hovered supply
##   point (or all known points while Tab / "show all" is held).
##
## Placed inside the MatchHud CanvasLayer so F1 (HUD off), photo mode and
## `trailer_shots.gd` (`hud = false`) hide everything automatically.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const HudIcons := preload("res://scenes/hud/hud_icons.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")

const SCAN_SECONDS := 0.25
const HOVER_SECONDS := 0.1
const POPUP_DURATION := 2.1
const POPUP_RISE_PX := 38.0
const POPUP_LIFT_M := 4.2
const ROOF_LIFT_M := 6.8
const SUPPLY_LIFT_M := 8.2
const GOLD := Color(1.0, 0.84, 0.26)
const NOTICE_COLOR := Color(1.0, 0.52, 0.36)

var main: Node3D
var sim_host: SimHost
## Frame time in microseconds (hud.gd).
var usec := 0
## Counters for self-test.
var promotions_shown := 0
var notices_shown := 0
var building_plaques_drawn := 0
var supply_labels_drawn := 0

var _popups: Array[Dictionary] = []
## Occupied building records from last scan: [{world, owner, relation, occ, cap, hp, rank}].
var _buildings: Array[Dictionary] = []
var _supply_indices: Array[int] = []
var _scan_left := 0.0
var _hover_left := 0.0
var _draw_key := []


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "WorldFeedback"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


## Spawns a rising promotion pop-up above world position `at` (veterancy_view.gd).
func add_promotion(at: Vector3, rank: int, rank_name: String) -> void:
	var stars := "★".repeat(clampi(rank, 1, 3))
	_popups.append({
		"world": at + Vector3.UP * POPUP_LIFT_M,
		"title": "%s %s" % [stars, tr("HUD_PROMOTED")],
		"sub": rank_name,
		"color": GOLD,
		"age": 0.0,
		"duration": POPUP_DURATION,
	})
	promotions_shown += 1
	_draw_key = []
	queue_redraw()


## Spawns a short rising world notice above `at` (e.g. full / hostile building).
func add_world_notice(at: Vector3, text: String, color := NOTICE_COLOR) -> void:
	if text.is_empty():
		return
	_popups.append({
		"world": at + Vector3.UP * POPUP_LIFT_M,
		"title": text,
		"sub": "",
		"color": color,
		"age": 0.0,
		"duration": 1.8,
	})
	notices_shown += 1
	_draw_key = []
	queue_redraw()


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	if sim_host == null or not is_visible_in_tree():
		usec += Time.get_ticks_usec() - began
		return
	var animating := false
	for i in range(_popups.size() - 1, -1, -1):
		var p: Dictionary = _popups[i]
		p["age"] = float(p["age"]) + delta
		if float(p["age"]) >= float(p["duration"]):
			_popups.remove_at(i)
		else:
			animating = true
	_scan_left -= delta
	if _scan_left <= 0.0:
		_scan_left = SCAN_SECONDS
		_scan_buildings()
	_hover_left -= delta
	if _hover_left <= 0.0:
		_hover_left = HOVER_SECONDS
		_scan_supply_hover()
	if animating:
		queue_redraw()
	else:
		var has_static := not _buildings.is_empty() or not _supply_indices.is_empty()
		var key := [
			_buildings.size(), _supply_indices,
			main.camera.global_transform if has_static else Transform3D.IDENTITY,
			sim_host.current_tick() if has_static else 0,
			RelationColors.palette, HudTheme.scale,
		]
		if key != _draw_key:
			_draw_key = key
			queue_redraw()
	usec += Time.get_ticks_usec() - began


func _scan_buildings() -> void:
	_buildings.clear()
	var garrisons := sim_host.unit_garrisons()
	if garrisons.is_empty():
		return
	var ids := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var owners := sim_host.unit_owners()
	var health := sim_host.unit_health()
	var ranks := sim_host.unit_ranks()
	var positions := sim_host.current_positions()
	var me := sim_host.local_player()
	var effects: Node = main.get("effects")
	var by_building: Dictionary[int, Dictionary] = {}
	for i in mini(ids.size(), garrisons.size()):
		var b := garrisons[i]
		if b < 0 or alive[i] == 0:
			continue
		var owner := owners[i]
		var own := owner == me
		var allied := own or sim_host.relation(owner) == 1
		var pos := positions[i]
		if not allied and effects != null and not effects.visible_at.call(pos):
			continue
		if not by_building.has(b):
			var info := sim_host.building_info(b)
			if info.is_empty():
				continue
			by_building[b] = {
				"world": pos + Vector3.UP * ROOF_LIFT_M,
				"owner": owner,
				"relation": "own" if own else ("ally" if allied else "enemy"),
				"occ": int(info.get("occupants", 0)),
				"cap": int(info.get("capacity", 0)),
				"hp_sum": 0.0,
				"hp_count": 0,
				"rank": 0,
			}
		var rec: Dictionary = by_building[b]
		rec["hp_sum"] = float(rec["hp_sum"]) + health[i]
		rec["hp_count"] = int(rec["hp_count"]) + 1
		if i < ranks.size():
			rec["rank"] = maxi(int(rec["rank"]), ranks[i])
	for rec: Dictionary in by_building.values():
		var n := maxi(int(rec["hp_count"]), 1)
		rec["hp"] = clampf(float(rec["hp_sum"]) / float(n), 0.0, 1.0)
		_buildings.append(rec)


func _scan_supply_hover() -> void:
	_supply_indices.clear()
	var supply_view: Node = main.get("supply_view")
	if supply_view == null or not supply_view.marks_enabled:
		return
	var known: Array = supply_view.known
	if known.is_empty():
		return
	var highlight: Node = main.get("unit_highlight")
	if highlight != null and highlight.show_all:
		for i in known.size():
			_supply_indices.append(i)
		return
	var camera: Camera3D = main.camera
	if camera == null:
		return
	var hit: Variant = GroundPick.from_screen(camera, get_viewport().get_mouse_position())
	if hit != null:
		var idx: int = supply_view.point_at(hit as Vector3)
		if idx >= 0:
			_supply_indices.append(idx)


func _draw() -> void:
	var began := Time.get_ticks_usec()
	building_plaques_drawn = 0
	supply_labels_drawn = 0
	var camera: Camera3D = main.camera
	if camera == null:
		usec += Time.get_ticks_usec() - began
		return
	_draw_buildings(camera)
	_draw_supply_labels(camera)
	_draw_popups(camera)
	usec += Time.get_ticks_usec() - began


func _draw_buildings(camera: Camera3D) -> void:
	if _buildings.is_empty():
		return
	var font := get_theme_default_font()
	var font_size := int(round(HudTheme.px(12.0)))
	var icon_sz := HudTheme.px(13.0)
	var pad_x := HudTheme.px(6.0)
	var pad_y := HudTheme.px(4.0)
	var bar_h := HudTheme.px(3.0)
	for b: Dictionary in _buildings:
		var world: Vector3 = b["world"]
		if camera.is_position_behind(world):
			continue
		var screen := camera.unproject_position(world)
		var rel: String = b["relation"]
		var col := RelationColors.own() if rel == "own" \
				else (RelationColors.ally() if rel == "ally" else RelationColors.enemy())
		var label := "%d/%d" % [int(b["occ"]), int(b["cap"])]
		var rank := int(b["rank"])
		if rank > 0:
			label += " " + "★".repeat(clampi(rank, 1, 3))
		var text_sz := font.get_string_size(label, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size)
		var w := pad_x * 2.0 + icon_sz + HudTheme.px(4.0) + text_sz.x + HudTheme.px(4.0)
		var h := pad_y * 2.0 + maxf(icon_sz, float(font_size)) + bar_h + HudTheme.px(3.0)
		var rect := Rect2(screen - Vector2(w * 0.5, h), Vector2(w, h))
		draw_rect(rect, Color(0.05, 0.06, 0.08, 0.82))
		draw_rect(Rect2(rect.position, Vector2(HudTheme.px(3.5), rect.size.y)), col)
		draw_rect(rect, Color(col, 0.55), false, 1.0)
		var icon_rect := Rect2(rect.position + Vector2(pad_x + HudTheme.px(2.0), pad_y), Vector2(icon_sz, icon_sz))
		HudIcons.draw(self, "garrison", icon_rect, col, 1.3)
		var text_pos := Vector2(icon_rect.end.x + HudTheme.px(4.0), rect.position.y + pad_y + float(font_size) * 0.92)
		draw_string(font, text_pos, label, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.96, 0.95, 0.92))
		var bar_pos := Vector2(rect.position.x + pad_x + HudTheme.px(2.0), rect.end.y - pad_y - bar_h)
		var bar_w := rect.size.x - pad_x * 2.0 - HudTheme.px(2.0)
		draw_rect(Rect2(bar_pos, Vector2(bar_w, bar_h)), Color(0.18, 0.18, 0.2, 0.9))
		draw_rect(Rect2(bar_pos, Vector2(bar_w * float(b["hp"]), bar_h)), col)
		building_plaques_drawn += 1


func _draw_supply_labels(camera: Camera3D) -> void:
	if _supply_indices.is_empty():
		return
	var supply_view: Node = main.get("supply_view")
	if supply_view == null:
		return
	var known: Array = supply_view.known
	var font := get_theme_default_font()
	var font_size := int(round(HudTheme.px(12.0)))
	var pad_x := HudTheme.px(7.0)
	var pad_y := HudTheme.px(4.0)
	for idx in _supply_indices:
		if idx < 0 or idx >= known.size():
			continue
		var k: Dictionary = known[idx]
		var world: Vector3 = (k.position as Vector3) + Vector3.UP * SUPPLY_LIFT_M
		if camera.is_position_behind(world):
			continue
		var rel: String = k.relation
		var col := RelationColors.own() if rel == "own" \
				else (RelationColors.ally() if rel == "ally" \
				else (RelationColors.enemy() if rel == "enemy" else RelationColors.neutral()))
		var status := tr("HUD_SUPPLY_CONTESTED") if bool(k.contested) \
				else (tr("HUD_SUPPLY_CAPTURING") % roundi(float(k.progress) * 100.0) if int(k.taker) >= 0 \
				else tr("HUD_SUPPLY_" + rel.to_upper()))
		var inc: PackedInt32Array = k.get("income", PackedInt32Array())
		var income_str := ""
		if inc.size() >= 3:
			var parts: PackedStringArray = PackedStringArray()
			if inc[0] > 0:
				parts.append("+%d %s" % [inc[0], tr("HUD_RES_FUEL")])
			if inc[1] > 0:
				parts.append("+%d %s" % [inc[1], tr("HUD_RES_AMMO")])
			if inc[2] > 0:
				parts.append("+%d %s" % [inc[2], tr("HUD_RES_MANPOWER")])
			income_str = " ".join(parts)
		var title := str(k.get("name", ""))
		var line := status if title.is_empty() else "%s  ·  %s" % [title, status]
		if not income_str.is_empty():
			line += "  ·  " + income_str
		var sz := font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size)
		var w := sz.x + pad_x * 2.0
		var h := float(font_size) + pad_y * 2.0
		var p := camera.unproject_position(world)
		var rect := Rect2(p - Vector2(w * 0.5, h), Vector2(w, h))
		draw_rect(rect, Color(0.05, 0.06, 0.08, 0.78))
		draw_rect(rect, Color(col, 0.65), false, 1.0)
		draw_string(font, rect.position + Vector2(pad_x, pad_y + float(font_size) * 0.88),
				line, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.95, 0.94, 0.90))
		supply_labels_drawn += 1


func _draw_popups(camera: Camera3D) -> void:
	if _popups.is_empty():
		return
	var font := get_theme_default_font()
	var title_sz := int(round(HudTheme.px(14.0)))
	var sub_sz := int(round(HudTheme.px(11.0)))
	var pad_x := HudTheme.px(8.0)
	var pad_y := HudTheme.px(4.0)
	for p: Dictionary in _popups:
		var world: Vector3 = p["world"]
		if camera.is_position_behind(world):
			continue
		var age := float(p["age"])
		var duration := maxf(float(p["duration"]), 0.1)
		var t := clampf(age / duration, 0.0, 1.0)
		var alpha := minf(age / 0.15, 1.0) * clampf((duration - age) / 0.55, 0.0, 1.0)
		var screen := camera.unproject_position(world) - Vector2(0.0, HudTheme.px(POPUP_RISE_PX) * t)
		var title: String = p["title"]
		var sub: String = p["sub"]
		var col: Color = p["color"]
		var tw := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, title_sz).x
		var sw := font.get_string_size(sub, HORIZONTAL_ALIGNMENT_LEFT, -1, sub_sz).x if not sub.is_empty() else 0.0
		var w := maxf(tw, sw) + pad_x * 2.0
		var h := float(title_sz) + (float(sub_sz) + HudTheme.px(2.0) if not sub.is_empty() else 0.0) + pad_y * 2.0
		var rect := Rect2(screen - Vector2(w * 0.5, h * 0.5), Vector2(w, h))
		draw_rect(rect, Color(0.06, 0.06, 0.07, 0.82 * alpha))
		draw_rect(rect, Color(col, 0.85 * alpha), false, 1.4)
		var tx := rect.position.x + (w - tw) * 0.5
		var ty := rect.position.y + pad_y + float(title_sz) * 0.9
		draw_string(font, Vector2(tx, ty), title, HORIZONTAL_ALIGNMENT_LEFT, -1, title_sz, Color(col, alpha))
		if not sub.is_empty():
			var sx := rect.position.x + (w - sw) * 0.5
			var sy := ty + float(sub_sz) + HudTheme.px(2.0)
			draw_string(font, Vector2(sx, sy), sub, HORIZONTAL_ALIGNMENT_LEFT, -1, sub_sz, Color(0.95, 0.93, 0.88, alpha))
