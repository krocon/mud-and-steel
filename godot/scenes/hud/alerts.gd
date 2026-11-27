extends Control
## Alarms for what the player does not see (prompts 101, 244). Presentation
## only.
##
## Every hit on an own unit or building and every own loss (combat event
## stream) is checked against the camera view (`in_view`: the projected
## point inside the screen, `VIEW_MARGIN` px in from the edges):
##
## | Event | In view | Out of view |
## |---|---|---|
## | own unit hit | nothing | feed "under fire", minimap ping, edge arrow (yellow), announcement if the attack is new |
## | own building hit | nothing | feed "building under fire", ping, edge arrow (yellow), announcement if the attack is new |
## | own unit lost | feed "lost" (normal time), no sound | feed "lost" for `LOST_LIFETIME`, loss sound, ping, edge arrow (red) |
## | own unit stuck (prompt 173) | feed "stuck" | feed "stuck" for `STUCK_LIFETIME`, ping, edge arrow (sand), no sound |
##
## "Stuck": the simulation stopped the unit because it got no closer to
## its target (`SimHost.take_move_events`); it waits for a new order and
## carries a state mark meanwhile (`state_marks.gd`).
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
##
## What is heard (prompt 244): instead of an alarm sound per spot, a voice
## of the player's faction says "We are under attack!"
## (`battle_sound.announce`; the alarm sound `ui_under_attack` where the
## faction has no recording), and only for a **new attack on a front
## sector** (`attack`):
##
## - Attack events: a hit on an own unit or building, an own loss, an own
##   supply point attacked (`supply_view.gd`). Each belongs to the first
##   sector whose first event lies within `SECTOR_RADIUS`, else it opens a
##   new one, and each keeps its sector hot, also one in view.
## - A sector ends after `SECTOR_QUIET` without an event. Only the event
##   that opens a sector can speak: a lasting battle is announced once, a
##   new attack there after a quiet minute again.
## - It speaks only out of view. An attack that begins in view stays silent
##   and its sector is hot all the same, so scrolling away later makes up
##   for nothing. A loss never speaks (it plays `ui_unit_lost` as before).
## - Two announcements are at least `ANNOUNCE_GAP` apart; one that falls
##   into the gap is left out, not queued.

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
const RANK := {"stuck": 0, "fire": 1, "building": 2, "lost": 3}
## Stuck units stay this long in the feed.
const STUCK_LIFETIME := 20.0
## Front sectors (prompt 244), all [provisional]: attack events up to this
## far (m, on the ground) from a sector's first event belong to it.
const SECTOR_RADIUS := 250.0
## A sector without an attack event for this long (s) is over; the next
## event there is a new attack.
const SECTOR_QUIET := 60.0
## Two announcements are at least this far apart (s), whatever the sector.
const ANNOUNCE_GAP := 8.0

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
## Front sectors under attack: `at` (first event), `time` (last event).
var sectors: Array[Dictionary] = []
## Announcements spoken, and new attacks out of view left out because the
## last announcement was less than `ANNOUNCE_GAP` ago (self-test).
var announced := 0
var announce_skipped := 0

var _now := 0.0
var _jump_back := 0
var _last_jump := -1e9
var _last_announce := -1e9


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
	_take_move_events()
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


## Own units the simulation stopped because they got stuck (prompt 173).
func _take_move_events() -> void:
	if sim_host == null:
		return
	var events := sim_host.take_move_events()
	if not active():
		return
	var me := sim_host.local_player()
	for event: Dictionary in events:
		var id: int = event["unit"]
		if event["type"] == "stuck" and feed.owner_of(id) == me:
			report("stuck", id, event["at"])


## One event of `kind` at `at` concerning own unit `id` (also the self-test).
func report(kind: String, id: int, at: Vector3) -> void:
	var seen := in_view(at)
	if kind != "stuck":
		attack(at, kind != "lost")
	if seen:
		quiet += 1
		if kind == "lost":
			feed.post("lost", tr("HUD_FEED_LOST") % feed.name_of(id), at)
		elif kind == "stuck":
			feed.post("stuck", tr("HUD_FEED_STUCK") % feed.name_of(id), at)
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
		"stuck":
			feed.post("stuck", tr("HUD_FEED_STUCK") % feed.name_of(id), at, STUCK_LIFETIME)
		"building":
			feed.post("fire", tr("HUD_FEED_BUILDING_UNDER_FIRE") % feed.name_of(id), at)
		_:
			feed.post("fire", tr("HUD_FEED_UNDER_FIRE") % feed.name_of(id), at)


## An attack event at `at` (prompt 244; also `supply_view.gd` and the
## self-test): keeps its front sector hot and, if it opens the sector out of
## view, announces the attack. `speaks` false (losses): the event only
## counts. Returns whether the announcement sounded.
func attack(at: Vector3, speaks := true) -> bool:
	sectors = sectors.filter(func(s: Dictionary) -> bool: return _now - float(s.time) < SECTOR_QUIET)
	var ground := Vector2(at.x, at.z)
	for s in sectors:
		if (s.at as Vector2).distance_to(ground) <= SECTOR_RADIUS:
			s.time = _now
			return false
	sectors.append({"at": ground, "time": _now})
	if not speaks or in_view(at):
		return false
	if _now - _last_announce < ANNOUNCE_GAP:
		announce_skipped += 1
		return false
	var sound: Node = main.get("battle_sound")
	if sound == null or not sound.announce("under_attack"):
		return false
	_last_announce = _now
	announced += 1
	return true


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
