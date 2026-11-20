extends Control
## Alarms for what the player does not see (prompt 101). Presentation only.
##
## Every hit on an own unit or building and every own loss (combat event
## stream) is checked against the camera view (`in_view`: the projected
## point inside the screen, `VIEW_MARGIN` px in from the edges):
##
## | Event | In view | Out of view |
## |---|---|---|
## | own unit hit | nothing | feed "under fire", alarm sound, minimap ping, edge arrow (yellow) |
## | own building hit | nothing | feed "building under fire", alarm sound, ping, edge arrow (yellow) |
## | own unit lost | feed "lost" (normal time), no sound | feed "lost" for `LOST_LIFETIME`, loss sound, ping, edge arrow (red) |
##
## Events within `SPOT_RADIUS` of each other form one spot with one arrow;
## a spot alarms again only after `SPOT_COOLDOWN` (losses: `LOST_COOLDOWN`),
## so a second attack elsewhere alarms at once. The arrow stays
## `ARROW_SECONDS` after the spot's last event, points from the screen
## center to the spot and jumps there on a click. `alarm_jump` (Shift +
## Space) goes through the alarms, newest first; pressed again within
## `JUMP_CHAIN_SECONDS`, the next older one. An arrow that would lie on a
## HUD panel slides inwards along its direction. Replays and spectators get no
## alarms. Colors: the feed's (`event_feed.gd COLORS`).

const EventFeed := preload("res://scenes/hud/event_feed.gd")

## A point counts as seen this many pixels inside the screen edge.
const VIEW_MARGIN := 24.0
const SPOT_RADIUS := 60.0
const SPOT_COOLDOWN := 10.0
const LOST_COOLDOWN := 3.0
const ARROW_SECONDS := 6.0
## Losses out of view stay this long in the feed.
const LOST_LIFETIME := 30.0
const JUMP_CHAIN_SECONDS := 4.0
## Alarms kept for the jump key.
const MAX_ALARMS := 16
## Arrow size and its distance from the screen edge (pixels).
const ARROW_SIZE := 26.0
const EDGE_INSET := 40.0
## Rank of a spot's kind: a loss outranks a building hit, that a unit hit.
const RANK := {"fire": 0, "building": 1, "lost": 2}

var main: Node3D
var sim_host: SimHost
var feed: EventFeed
## Time of this frame's `_process` and `_draw` in microseconds (hud.gd).
var usec := 0
## Spots: `at`, `kind` ("fire", "building", "lost"), `time` (last event),
## `alarmed` (last alarm).
var spots: Array[Dictionary] = []
## Alarm places, oldest first (jump key).
var alarms: Array[Vector3] = []
## Alarms raised and events kept quiet so far (self-test).
var raised := 0
var quiet := 0
## Arrows shown in the last frame: screen position and spot (self-test).
var arrows: Array[Dictionary] = []

var _now := 0.0
var _jump_back := 0
var _last_jump := -1e9


func setup(main_node: Node3D, event_feed: EventFeed) -> void:
	main = main_node
	sim_host = main.sim_host
	feed = event_feed
	name = "Alerts"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var combat: Node = main.get("combat_view")
	if combat != null:
		combat.combat_event.connect(on_combat_event)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_now += delta
	spots = spots.filter(func(s: Dictionary) -> bool: return _now - float(s.time) < maxf(ARROW_SECONDS, SPOT_COOLDOWN))
	queue_redraw()
	usec += Time.get_ticks_usec() - began


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("alarm_jump", false, true):
		if jump_to_alarm():
			get_viewport().set_input_as_handled()


## A click on an arrow jumps; taken before the map sees it, but not where a
## HUD panel lies over the arrow.
func _input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button == null or not button.pressed or button.button_index != MOUSE_BUTTON_LEFT:
		return
	if click_arrow(button.position):
		get_viewport().set_input_as_handled()


## Jumps to the spot of the arrow at `screen` (also the self-test).
func click_arrow(screen: Vector2) -> bool:
	var hud: Node = main.get("hud")
	if hud != null and hud.covers(screen):
		return false
	for a in arrows:
		if (a.screen as Vector2).distance_to(screen) <= ARROW_SIZE:
			_jump(a.at)
			return true
	return false


## Whether the ground point `point` is in the camera's picture.
func in_view(point: Vector3) -> bool:
	var camera: Camera3D = main.camera
	if camera.is_position_behind(point):
		return false
	var rect := get_viewport_rect().grow(-VIEW_MARGIN)
	return rect.has_point(camera.unproject_position(point))


## Alarms are for players who play this match live.
func active() -> bool:
	return not sim_host.is_replay() and sim_host.local_player() >= 0


func on_combat_event(event: Dictionary) -> void:
	if not active():
		return
	var type: String = event["type"]
	var me := sim_host.local_player()
	if type == "destroyed":
		var id: int = event["unit"]
		if feed.owner_of(id) == me:
			report("lost", id, event["position"])
	elif type == "hit" and int(event["target"]) >= 0:
		var target: int = event["target"]
		if feed.owner_of(target) == me:
			report("building" if _is_building(target) else "fire", target, event["position"])


## One event of `kind` at `at` concerning own unit `id` (also the self-test).
func report(kind: String, id: int, at: Vector3) -> void:
	var seen := in_view(at)
	if seen:
		quiet += 1
		if kind == "lost":
			feed.post("lost", tr("HUD_FEED_LOST") % feed.name_of(id), at)
		return
	var spot := _spot_at(at)
	var cooldown := LOST_COOLDOWN if kind == "lost" else SPOT_COOLDOWN
	if spot.is_empty():
		spot = {"at": at, "kind": kind, "time": _now, "alarmed": -1e9}
		spots.append(spot)
	spot.time = _now
	if RANK[kind] > RANK[spot.kind]:
		spot.kind = kind
	elif _now - float(spot.alarmed) < cooldown:
		return
	spot.alarmed = _now
	raised += 1
	alarms.append(at)
	if alarms.size() > MAX_ALARMS:
		alarms.pop_front()
	feed.add_ping(at)
	var sound: Node = main.get("battle_sound")
	match kind:
		"lost":
			feed.post("lost", tr("HUD_FEED_LOST") % feed.name_of(id), at, LOST_LIFETIME)
			if sound != null:
				sound.notice("ui_unit_lost", true)
		"building":
			feed.post("fire", tr("HUD_FEED_BUILDING_UNDER_FIRE") % feed.name_of(id), at)
			if sound != null:
				sound.notice("ui_under_attack", true)
		_:
			feed.post("fire", tr("HUD_FEED_UNDER_FIRE") % feed.name_of(id), at)
			if sound != null:
				sound.notice("ui_under_attack", true)


## Jumps to the newest alarm, or pressed again soon, to the next older one.
func jump_to_alarm() -> bool:
	if alarms.is_empty():
		return false
	_jump_back = _jump_back + 1 if _now - _last_jump < JUMP_CHAIN_SECONDS else 0
	_last_jump = _now
	var index := alarms.size() - 1 - (_jump_back % alarms.size())
	_jump(alarms[index])
	return true


func _jump(at: Vector3) -> void:
	main.camera_rig.center_on(at)
	if main.selection.follow:
		main.selection.set_follow(false)


func _spot_at(at: Vector3) -> Dictionary:
	for s in spots:
		if (s.at as Vector3).distance_to(at) < SPOT_RADIUS:
			return s
	return {}


func _is_building(id: int) -> bool:
	var base_view: Node = main.get("base_view")
	return base_view != null and (base_view.structures as Dictionary).has(id)


func _draw() -> void:
	var began := Time.get_ticks_usec()
	arrows.clear()
	var camera: Camera3D = main.camera
	var rect := get_viewport_rect().grow(-EDGE_INSET)
	var center := rect.get_center()
	for s in spots:
		if _now - float(s.time) > ARROW_SECONDS or in_view(s.at):
			continue
		var dir: Vector2
		if camera.is_position_behind(s.at):
			var local: Vector3 = camera.global_transform.affine_inverse() * (s.at as Vector3)
			dir = Vector2(local.x, local.z)
		else:
			dir = camera.unproject_position(s.at) - center
		if dir.length() < 0.001:
			continue
		dir = dir.normalized()
		var scale_x := (rect.size.x * 0.5) / absf(dir.x) if absf(dir.x) > 0.0001 else INF
		var scale_y := (rect.size.y * 0.5) / absf(dir.y) if absf(dir.y) > 0.0001 else INF
		var tip := center + dir * minf(scale_x, scale_y)
		# Not on a HUD panel: slide inwards along the direction.
		var hud: Node = main.get("hud")
		var steps := 0
		while hud != null and steps < 40 and (hud.covers(tip) or hud.covers(tip - dir * ARROW_SIZE)):
			tip -= dir * 12.0
			steps += 1
		var color: Color = EventFeed.COLORS["lost" if s.kind == "lost" else "fire"]
		var fade := clampf((ARROW_SECONDS - (_now - float(s.time))) / 1.0, 0.0, 1.0)
		_arrow(tip, dir, Color(color, color.a * fade))
		arrows.append({"screen": tip - dir * ARROW_SIZE * 0.5, "at": s.at, "kind": s.kind, "dir": dir})
	usec += Time.get_ticks_usec() - began


func _arrow(tip: Vector2, dir: Vector2, color: Color) -> void:
	var side := Vector2(-dir.y, dir.x)
	var base := tip - dir * ARROW_SIZE
	var points := PackedVector2Array([tip, base + side * ARROW_SIZE * 0.5, base - side * ARROW_SIZE * 0.5])
	var shadow := PackedVector2Array()
	for p in points:
		shadow.append(p + Vector2(1.5, 1.5))
	draw_colored_polygon(shadow, Color(0, 0, 0, 0.5 * color.a))
	draw_colored_polygon(points, color)
	draw_polyline(points + PackedVector2Array([points[0]]), Color(0, 0, 0, 0.7 * color.a), 1.5, true)
