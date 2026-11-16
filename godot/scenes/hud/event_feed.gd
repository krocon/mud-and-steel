extends VBoxContainer
## Event feed (prompt 031), at the left edge.
##
## Short messages with a place: own unit lost, building completed, unit
## produced, enemy sighted (at most every SIGHTED_SECONDS), own units under
## fire (at most every FIRE_SECONDS; every hit on an own unit also pings
## the minimap, at most one ping per PING_SECONDS within PING_RADIUS).
## Clicking a message moves the camera there; the `event_jump` action
## (Space) jumps to the newest one. Messages fade after LIFETIME seconds.
##
## Everything comes from the snapshot and the combat event stream; nothing
## here reaches the simulation. Exception (prompt 037): `ping_allies` (K)
## marks the ground under the mouse for the allies (`SimHost.ping`, a
## command without effect); pings of allies appear here and on the minimap.

signal ping(point: Vector3)

const GroundPick := preload("res://scenes/camera/ground_pick.gd")

const MAX_MESSAGES := 6
const LIFETIME := 12.0
const POLL := 0.25
const SIGHTED_SECONDS := 15.0
const FIRE_SECONDS := 10.0
const PING_SECONDS := 2.0
const PING_RADIUS := 60.0
const COLORS := {
	"lost": Color(1.0, 0.45, 0.4), "fire": Color(1.0, 0.7, 0.35), "sighted": Color(1.0, 0.85, 0.4),
	"built": Color(0.55, 0.9, 1.0), "ready": Color(0.6, 1.0, 0.6),
	"ping": Color(0.45, 0.7, 1.0),
}

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Every message so far, oldest first: `text`, `kind`, `position`, `time`.
var history: Array[Dictionary] = []

var _now := 0.0
var _poll_left := 0.0
var _initialized := false
var _known_own: Dictionary[int, bool] = {}
var _seen_enemies: Dictionary[int, bool] = {}
var _completed: Dictionary[int, bool] = {}
var _last_sighted := -1e9
var _last_fire := -1e9
var _pings: Array[Dictionary] = []
## Owner per unit id, refreshed every poll (hits come many per frame).
var _owners: Dictionary[int, int] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "EventFeed"
	set_anchors_preset(Control.PRESET_CENTER_LEFT)
	offset_left = 8
	offset_right = 330
	offset_top = -40
	offset_bottom = 120
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_constant_override(&"separation", 2)
	var combat: Node = main.get("combat_view")
	if combat != null:
		combat.combat_event.connect(_on_combat_event)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_now += delta
	_poll_left -= delta
	if _poll_left <= 0.0:
		_poll_left = POLL
		_poll()
	for child: Control in get_children():
		var age: float = _now - float(child.get_meta(&"time", _now))
		if age > LIFETIME:
			remove_child(child)
			child.queue_free()
		elif age > LIFETIME - 2.0:
			child.modulate.a = (LIFETIME - age) / 2.0
	usec += Time.get_ticks_usec() - began


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("event_jump", false, true) and not history.is_empty():
		jump_to(history.size() - 1)
		get_viewport().set_input_as_handled()
	if event.is_action_pressed("ping_allies", false, true):
		var mouse := get_viewport().get_mouse_position()
		var hit: Variant = GroundPick.from_screen(main.camera, mouse)
		if hit != null:
			ping_at(hit)
			get_viewport().set_input_as_handled()


## Pings the ground point for the allies (prompt 037).
func ping_at(point: Vector3) -> void:
	sim_host.ping(point.x, point.z)


## Moves the camera to message `index` of `history`.
func jump_to(index: int) -> void:
	var rig: Node3D = main.camera_rig
	rig.center_on(history[index].position)
	if main.selection.follow:
		main.selection.set_follow(false)


## Adds a message (also used by the self-test).
func post(kind: String, text: String, position: Vector3) -> void:
	history.append({"kind": kind, "text": text, "position": position, "time": _now})
	var index := history.size() - 1
	var b := Button.new()
	b.text = text
	b.flat = true
	b.alignment = HORIZONTAL_ALIGNMENT_LEFT
	b.focus_mode = Control.FOCUS_NONE
	b.add_theme_font_size_override(&"font_size", 14)
	b.add_theme_color_override(&"font_color", COLORS.get(kind, Color.WHITE))
	b.tooltip_text = "Click: camera there. Space: the newest message."
	b.set_meta(&"time", _now)
	b.pressed.connect(jump_to.bind(index))
	add_child(b)
	while get_child_count() > MAX_MESSAGES:
		var old := get_child(0)
		remove_child(old)
		old.queue_free()


func _on_combat_event(event: Dictionary) -> void:
	var type: String = event["type"]
	if type == "destroyed":
		var id: int = event["unit"]
		if _owner_of(id) == sim_host.local_player():
			post("lost", "Lost: %s" % _name_of(id), event["position"])
	elif type == "hit" and int(event["target"]) >= 0:
		if _owner_of(event["target"]) != sim_host.local_player():
			return
		var at: Vector3 = event["position"]
		_add_ping(at)
		if _now - _last_fire >= FIRE_SECONDS:
			_last_fire = _now
			post("fire", "Under fire: %s" % _name_of(event["target"]), at)


func _add_ping(at: Vector3) -> void:
	_pings = _pings.filter(func(p: Dictionary) -> bool: return _now - float(p.time) < PING_SECONDS)
	for p in _pings:
		if (p.at as Vector3).distance_to(at) < PING_RADIUS:
			return
	_pings.append({"at": at, "time": _now})
	ping.emit(at)


## Units that appeared (produced), enemies that came into sight, buildings
## that were completed.
func _poll() -> void:
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var positions := sim_host.current_positions()
	var me := sim_host.local_player()
	var base_view: Node = main.get("base_view")
	var structures: Dictionary = base_view.structures if base_view != null else {}
	var visible_enemies: Dictionary[int, bool] = {}
	var newly_sighted := -1
	for i in ids.size():
		var id := ids[i]
		_owners[id] = owners[i]
		if alive[i] == 0:
			continue
		if owners[i] == me:
			if not _known_own.has(id):
				_known_own[id] = true
				if _initialized and not structures.has(id):
					post("ready", "Ready: %s" % _name_of(id), positions[i])
		elif sim_host.relation(owners[i]) == 2:
			visible_enemies[id] = true
			if not _seen_enemies.has(id) and newly_sighted < 0:
				newly_sighted = i
	if _initialized and newly_sighted >= 0 and _now - _last_sighted >= SIGHTED_SECONDS:
		_last_sighted = _now
		post("sighted", "Enemy sighted: %s" % _name_of(ids[newly_sighted]), positions[newly_sighted])
	_seen_enemies = visible_enemies
	for id: int in structures:
		var s: Dictionary = structures[id]
		if int(s.get("owner", -1)) != me or float(s.get("built", 0.0)) < 1.0 or _completed.has(id):
			continue
		_completed[id] = true
		if _initialized:
			var index := Array(ids).find(id)
			post("built", "Completed: %s" % s.get("name", "building"), positions[index] if index >= 0 else Vector3.ZERO)
	_initialized = true
	# Pings of the allies and our own (prompt 037).
	for p: Dictionary in sim_host.take_pings():
		var at: Vector3 = p["position"]
		if not p["own"]:
			post("ping", "Ally (player %d) marks a spot" % p["player"], at)
		ping.emit(at)


func _owner_of(id: int) -> int:
	if _owners.has(id):
		return _owners[id]
	var index := sim_host.unit_ids().find(id)
	return sim_host.unit_owners()[index] if index >= 0 else -1


func _name_of(id: int) -> String:
	var info := sim_host.unit_info(id)
	return info.get("name", "unit")
