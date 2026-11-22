extends Node
## Places where the local player fights right now, for `action_jump`
## (Shift + G, or the button beside the minimap). Presentation only.
##
## A combat event counts when an own unit or building is hit, an own shot
## hits an enemy unit, or a unit of either side is destroyed by the other:
## its place joins the spot within `SPOT_RADIUS` or opens a new one. A spot
## lives `SPOT_SECONDS` after its last event, in view or not (unlike the
## alarms of `alerts.gd`, which only report what is out of view).
##
## The key goes to the most recent spot; pressed again within
## `CHAIN_SECONDS` it goes on to the next, in the order of the first press
## (newest first), and round again. The toast says "Combat 2 / 3", or that
## nothing is going on. Replays and spectators have no spots.

const SPOT_RADIUS := 80.0
const SPOT_SECONDS := 20.0
const CHAIN_SECONDS := 6.0
const MAX_SPOTS := 12

var main: Node3D
var sim_host: SimHost
var feed: Node
## Shows the "Combat n / m" line (`zoom_toast.gd`).
var toast: Node
## Spots: `at` (mean of its events), `time` (last event), `events`.
var spots: Array[Dictionary] = []
## Spots jumped to so far (self-test).
var jumps := 0

var _now := 0.0
var _chain: Array[Vector3] = []
var _chain_index := -1
var _last_jump := -100.0


func setup(main_node: Node3D, event_feed: Node, toast_part: Node) -> void:
	main = main_node
	sim_host = main.sim_host
	feed = event_feed
	toast = toast_part
	name = "ActionSpots"
	var combat: Node = main.get("combat_view")
	if combat != null:
		combat.combat_event.connect(on_combat_event)


func _process(delta: float) -> void:
	_now += delta
	spots = spots.filter(func(s: Dictionary) -> bool: return _now - float(s.time) < SPOT_SECONDS)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("action_jump", false, true):
		jump()
		get_viewport().set_input_as_handled()


func active() -> bool:
	return not sim_host.is_replay() and sim_host.local_player() >= 0


func on_combat_event(event: Dictionary) -> void:
	if not active():
		return
	var me := sim_host.local_player()
	var at: Vector3 = event.get("position", Vector3.ZERO)
	match event["type"]:
		"hit":
			var target: int = event["target"]
			var shooter: int = event.get("shooter", -1)
			if target >= 0 and feed.owner_of(target) == me:
				add_event(at)
			elif target >= 0 and shooter >= 0 and feed.owner_of(shooter) == me:
				add_event(at)
		"destroyed":
			var unit: int = event["unit"]
			var by: int = event.get("by", -1)
			if feed.owner_of(unit) == me or (by >= 0 and feed.owner_of(by) == me):
				add_event(at)


## Adds one combat event at `at` (also the self-test).
func add_event(at: Vector3) -> void:
	for s in spots:
		if Vector2(s.at.x - at.x, s.at.z - at.z).length() < SPOT_RADIUS:
			var n: int = s.events
			s.at = (s.at * n + at) / (n + 1)
			s.events = n + 1
			s.time = _now
			return
	spots.append({"at": at, "time": _now, "events": 1})
	if spots.size() > MAX_SPOTS:
		spots.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.time > b.time)
		spots.resize(MAX_SPOTS)


## Goes to the next spot (see the class comment). False without any.
func jump() -> bool:
	if spots.is_empty():
		_chain.clear()
		if toast != null:
			toast.show_text(tr("HUD_NO_ACTION"))
		return false
	if _now - _last_jump > CHAIN_SECONDS or _chain.is_empty():
		var newest := spots.duplicate()
		newest.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.time > b.time)
		_chain.clear()
		for s in newest:
			_chain.append(s.at)
		_chain_index = 0
	else:
		_chain_index = (_chain_index + 1) % _chain.size()
	_last_jump = _now
	var at := _chain[_chain_index]
	# The spot may have moved with the fight: take its newest middle.
	for s in spots:
		if Vector2(s.at.x - at.x, s.at.z - at.z).length() < SPOT_RADIUS:
			at = s.at
			break
	main.camera_rig.center_on(at)
	if main.selection.follow:
		main.selection.set_follow(false)
	jumps += 1
	if toast != null:
		toast.show_text(tr("HUD_ACTION_N") % [_chain_index + 1, _chain.size()])
	return true
