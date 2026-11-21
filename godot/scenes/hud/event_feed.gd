extends VBoxContainer
## Event feed (prompt 031), at the left edge.
##
## Short messages with a place: own unit lost, building completed, unit
## produced, enemy sighted (at most every SIGHTED_SECONDS); supply point
## taken, lost or under attack come from `world/supply_view.gd` (prompt 107). Hits on own
## units and buildings and own losses come from `alerts.gd` (prompt 101):
## loud only where the camera does not look; it also pings the minimap
## (`add_ping`: at most one ping per PING_SECONDS within PING_RADIUS).
## Messages with a longer lifetime (losses out of view) are not pushed out
## by newer short ones.
## Clicking a message moves the camera there; the `event_jump` action
## (Space) jumps to the newest one. Messages fade after LIFETIME seconds.
##
## Everything comes from the snapshot and the combat event stream; nothing
## here reaches the simulation. Exception (prompt 037): `ping_allies` (K)
## marks the ground under the mouse for the allies (`SimHost.ping`, a
## command without effect); pings of allies appear here and on the minimap.

signal ping(point: Vector3)

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Settings := preload("res://scenes/menu/settings.gd")

const MAX_MESSAGES := 6
const LIFETIME := 12.0
const POLL := 0.25
const SIGHTED_SECONDS := 15.0
const PING_SECONDS := 2.0
const PING_RADIUS := 60.0
## Prompt 049: "man wounded" at most every WOUNDED_SECONDS.
const WOUNDED_SECONDS := 8.0
const COLORS := {
	"lost": Color(1.0, 0.45, 0.4), "fire": Color(1.0, 0.7, 0.35), "sighted": Color(1.0, 0.85, 0.4),
	"built": Color(0.55, 0.9, 1.0), "ready": Color(0.6, 1.0, 0.6),
	"ping": Color(0.45, 0.7, 1.0), "wounded": Color(1.0, 0.6, 0.55),
	# Supply points (prompt 107, `world/supply_view.gd`).
	"point_taken": Color(0.6, 1.0, 0.6), "point_lost": Color(1.0, 0.45, 0.4),
	"point_attack": Color(1.0, 0.7, 0.35),
	# Mission reinforcements driving in (prompt 108, `mission_hud.gd`).
	"reinforce": Color(0.55, 0.9, 1.0), "enemy_reinforce": Color(1.0, 0.7, 0.35),
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
var _pings: Array[Dictionary] = []
## Owner per unit id, refreshed every poll (hits come many per frame).
var _owners: Dictionary[int, int] = {}
## Wounded men per own unit at the last poll (prompt 049).
var _wounded: Dictionary[int, int] = {}
var _last_wounded := -1e9


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


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_now += delta
	_poll_left -= delta
	if _poll_left <= 0.0:
		_poll_left = POLL
		_poll()
	for child: Control in get_children():
		var age: float = _now - float(child.get_meta(&"time", _now))
		var lifetime: float = child.get_meta(&"lifetime", LIFETIME)
		if age > lifetime:
			remove_child(child)
			child.queue_free()
		elif age > lifetime - 2.0:
			child.modulate.a = (lifetime - age) / 2.0
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


## Adds a message (also used by the self-test); `lifetime` in seconds.
func post(kind: String, text: String, position: Vector3, lifetime := LIFETIME) -> void:
	history.append({"kind": kind, "text": text, "position": position, "time": _now})
	var index := history.size() - 1
	var b := Button.new()
	b.text = text
	b.flat = true
	b.alignment = HORIZONTAL_ALIGNMENT_LEFT
	b.focus_mode = Control.FOCUS_NONE
	b.add_theme_font_size_override(&"font_size", MenuTheme.font(14))
	b.add_theme_color_override(&"font_color", COLORS.get(kind, Color.WHITE))
	b.tooltip_text = tr("HUD_TIP_FEED") % Settings.action_key_name("event_jump")
	b.set_meta(&"time", _now)
	b.set_meta(&"lifetime", lifetime)
	b.pressed.connect(jump_to.bind(index))
	add_child(b)
	while get_child_count() > MAX_MESSAGES:
		# The oldest short message goes first; long ones only if all are.
		var old := get_child(0)
		for child in get_children():
			if float(child.get_meta(&"lifetime", LIFETIME)) <= LIFETIME:
				old = child
				break
		remove_child(old)
		old.queue_free()


## Pings the minimap at `at` unless a ping is near it already.
func add_ping(at: Vector3) -> void:
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
				if _initialized and sim_host.unit_is_crew(id):
					post("lost", tr("HUD_FEED_CREW_BAILED") % name_of(id), positions[i])
				elif _initialized and not structures.has(id):
					post("ready", tr("HUD_FEED_READY") % name_of(id), positions[i])
		elif sim_host.relation(owners[i]) == 2:
			visible_enemies[id] = true
			if not _seen_enemies.has(id) and newly_sighted < 0:
				newly_sighted = i
	if _initialized and newly_sighted >= 0 and _now - _last_sighted >= SIGHTED_SECONDS:
		_last_sighted = _now
		post("sighted", tr("HUD_FEED_SIGHTED") % name_of(ids[newly_sighted]), positions[newly_sighted])
	_seen_enemies = visible_enemies
	_poll_wounded(ids, owners, positions, me)
	for id: int in structures:
		var s: Dictionary = structures[id]
		if int(s.get("owner", -1)) != me or float(s.get("built", 0.0)) < 1.0 or _completed.has(id):
			continue
		_completed[id] = true
		if _initialized:
			var index := Array(ids).find(id)
			post("built", tr("HUD_FEED_COMPLETED") % s.get("name", tr("HUD_FEED_BUILDING")), positions[index] if index >= 0 else Vector3.ZERO)
	_initialized = true
	# Pings of the allies and our own (prompt 037).
	for p: Dictionary in sim_host.take_pings():
		var at: Vector3 = p["position"]
		if not p["own"]:
			post("ping", tr("HUD_FEED_PING") % p["player"], at)
		ping.emit(at)


## "Man wounded" when an own squad has more wounded men than at the last
## poll (prompt 049), at most every WOUNDED_SECONDS.
func _poll_wounded(ids: PackedInt64Array, owners: PackedInt32Array, positions: PackedVector3Array, me: int) -> void:
	var counts := sim_host.unit_wounded_counts()
	var now_wounded: Dictionary[int, int] = {}
	var newly := -1
	for id: int in counts:
		var c: Vector2i = counts[id]
		now_wounded[id] = c.x + c.y
		if _initialized and c.x + c.y > int(_wounded.get(id, 0)) and newly < 0:
			var index := Array(ids).find(id)
			if index >= 0 and owners[index] == me:
				newly = index
	_wounded = now_wounded
	if newly >= 0 and _now - _last_wounded >= WOUNDED_SECONDS:
		_last_wounded = _now
		post("wounded", tr("HUD_FEED_WOUNDED") % name_of(ids[newly]), positions[newly])


func owner_of(id: int) -> int:
	if _owners.has(id):
		return _owners[id]
	var index := sim_host.unit_ids().find(id)
	return sim_host.unit_owners()[index] if index >= 0 else -1


func name_of(id: int) -> String:
	var info := sim_host.unit_info(id)
	return info.get("name", tr("HUD_FEED_UNIT"))
