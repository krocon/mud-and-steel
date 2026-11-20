extends Control
## Unit selection and orders (presentation only).
##
## Selection lives entirely in Godot; only the resulting orders go to SimHost.
## Only units of the local player can be selected.
##
## - Left click selects a unit, Shift adds or removes it, a click on empty
##   ground clears the selection. Dragging draws a selection box.
## - Double click selects all visible own units of the same type.
## - Ctrl+1…9 (or Cmd on macOS) stores the selection as a group, 1…9 recalls
##   it, pressing the same number again quickly centers the camera on it.
## - Right click sends the selection to the clicked ground point, H stops it.
## - Right click on a unit of another player attacks it, Ctrl+right click
##   (Cmd on macOS) fires at the ground point, F switches between hold fire
##   and fire at will (prompt 017). Destroyed units cannot be selected.
## - Infantry (prompt 022): right click on a building sends the selected
##   squads into it (other units drive there); Z stand, X kneel, V prone,
##   B automatic stance.
## - Prompt 031: Shift + right click appends a waypoint (`SimHost.queue_move`).
##   Targeted orders from the command bar or their keys (M move, R attack,
##   L fire at ground, P patrol, U occupy) wait for the next left click
##   (`command_mode`; Shift keeps the mode and queues moves); right click
##   cancels. Order keys are input actions, rebindable in the settings.
## - Indirect fire (prompt 032): G (`order_barrage`) fires a barrage (3
##   salvos within 25 m) and T (`order_smoke`) smoke (2 salvos within 15 m) of the selected mortars and guns at
##   the ground point under the cursor; Ctrl+right click fires at a point
##   until stopped, right click on an enemy fires at it while it is seen.
## - Transport (prompt 034, `transport_orders.gd`): right click with squads
##   on an own vehicle with seats mounts them, with vehicles on an own towed
##   gun hitches it; J dismounts, O unlimbers. Passengers cannot be
##   selected.
## - Formations (prompt 046, `formation_orders.gd`): right click moves in
##   the formation of the command bar (infantry halts to fight on the way),
##   right-drag sets the facing; M moves without halting, Y attack-moves,
##   I secures an area, `,` cycles the formation. Groups keep theirs.
## - Active unit (prompt 094): the unit clicked last (`active_unit`); the
##   unit panel marks its tile, the unit card shows it. Shift+C
##   (`camera_center`) or the panel's button lets the camera glide to it, C
##   (`camera_follow`) or the panel's toggle keeps it in the screen center
##   until the map is moved by hand.
##
## Sits full-screen in a CanvasLayer and draws the selection box itself. It
## ignores the mouse for GUI purposes and works from `_unhandled_input`.

signal move_ordered(point: Vector3)
signal attack_ordered(target: int)
signal garrison_ordered(building: int)
## A shelling order on map object `object` (prompt 093).
signal object_attack_ordered(object: int)
## The targeted order waiting for a click changed ("" = none).
signal command_mode_changed(mode: String)
signal barrage_ordered(point: Vector3, smoke: bool)
## A mount or tow order on the own unit `target` (prompt 034).
signal embark_ordered(target: int)
## The formation for the next moves changed (code, prompt 046).
signal formation_changed(code: int)
## A formation move or secure order went out (prompt 099: the target
## preview shows the slots for a moment): target, facing (zero: direction
## of travel), unit ids.
signal formation_ordered(point: Vector3, facing: Vector3, ids: Array)

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const TransportOrders := preload("res://scenes/selection/transport_orders.gd")
const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const FormationOrders := preload("res://scenes/selection/formation_orders.gd")
const CrossingOrders := preload("res://scenes/selection/crossing_orders.gd")
const ShellOrders := preload("res://scenes/selection/shell_orders.gd")
const Settings := preload("res://scenes/menu/settings.gd")
## Pointer travel in pixels before a left press becomes a box drag.
const DRAG_THRESHOLD := 6.0
## Minimum click radius around a unit in pixels (for far zoom levels).
const MIN_PICK_RADIUS := 14.0
## A unit's pick point sits this high above the ground (hull center, meters).
const PICK_HEIGHT := 1.2
## Two presses of the same group key within this time center the camera.
const DOUBLE_PRESS_MS := 400
## Stance actions (prompt 022, actions since 031): stance code for
## `SimHost.set_stance`.
const STANCE_ACTIONS := {"stance_stand": 0, "stance_kneel": 1, "stance_prone": 2, "stance_auto": -1}
## Actions that start a targeted order (prompt 031): action -> mode.
const MODE_ACTIONS := {
	"order_move": "move", "order_attack": "attack", "order_attack_ground": "attack_ground",
	"order_patrol": "patrol", "order_garrison": "garrison",
	"order_attack_move": "attack_move", "order_secure": "secure",
	"order_attack_object": "attack_object",
}
## Hint (translation key) shown while a targeted order waits for its click.
const MODE_HINTS := {
	"move": "HUD_HINT_MOVE",
	"attack": "HUD_HINT_ATTACK",
	"attack_ground": "HUD_HINT_GROUND",
	"attack_object": "HUD_HINT_SHELL",
	"patrol": "HUD_HINT_PATROL",
	"garrison": "HUD_HINT_OCCUPY",
	"barrage": "HUD_HINT_BARRAGE",
	"smoke": "HUD_HINT_SMOKE",
	"attack_move": "HUD_HINT_ADVANCE",
	"secure": "HUD_HINT_SECURE",
	"bridge_lay": "HUD_HINT_LAY_BRIDGE",
}
## Barrage actions (prompt 032, actions since 031): smoke?, salvos, radius
## in meters. They fire at the ground point under the cursor at once.
const BARRAGE_ACTIONS := {"order_barrage": [false, 3, 25], "order_smoke": [true, 2, 15]}
const BOX_FILL := Color(0.45, 0.85, 0.45, 0.12)
const BOX_LINE := Color(0.55, 0.95, 0.55, 0.9)
const FACING_LINE := Color(1.0, 0.9, 0.4, 0.9)

var sim_host: SimHost
var camera: Camera3D
var camera_rig: Node3D
## Selected unit ids, ascending.
var selected: Array[int] = []
## The active unit (prompt 094): the selected unit the player clicked last,
## -1 without one. A box, a double click or a group keep it if it is among
## the new selection; otherwise the selected unit nearest to it (or to the
## box, the screen center; the first of a group) takes over. Local state
## only: the simulation knows nothing of it.
var active_unit := -1
## Control groups 1..9: number -> unit ids.
var groups: Dictionary[int, Array] = {}
## Formation of the next moves (code, prompt 046) and per control group.
var formation := 0
var group_formations: Dictionary[int, int] = {}
## Whether a unit id is a building (prompt 023): box selections that also
## catch units leave buildings out. Set by `main.gd`.
var is_structure := func(_id: int) -> bool: return false

## Targeted order waiting for the next left click ("" = none, prompt 031).
var command_mode := ""
## Map object under the cursor while a shelling order waits (prompt 093;
## -1 = none).
var shell_target := -1
var _pressing := false
var _dragging := false
var _double_click := false
## Selection before the first click of a possible double click, restored when
## the double click centers the camera on empty ground.
var _selection_before_click: Array = []
var _active_before_click := -1
## Follow mode (key C, toggle in the unit panel): the camera keeps the
## active unit in the screen center. Ends when the player moves the map by
## hand or no selected unit is left alive; without a selection it waits.
var follow := false
var _follow_hooked := false
## Unit the camera sits on while following (-1: glide to the active unit).
var _follow_unit := -1
## Where the active unit is drawn this frame (between two ticks).
var _active_position := Vector3.ZERO
var _press_position := Vector2.ZERO
var _drag_position := Vector2.ZERO
var _last_group_key := -1
var _last_group_time := -DOUBLE_PRESS_MS
## Right press on the ground, waiting for the release (facing by drag).
var _right_pressing := false
var _right_shift := false
var _right_press := Vector2.ZERO
var _right_drag := Vector2.ZERO


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)
	load_follow()


func is_selected(id: int) -> bool:
	return selected.has(id)


## A box drag that started on the map ends wherever the mouse is released,
## also over a HUD panel (prompt 031): the release and the motion are taken
## before the GUI sees them.
func _input(event: InputEvent) -> void:
	if _right_pressing and _on_right_drag(event):
		return
	if not _pressing:
		return
	var motion := event as InputEventMouseMotion
	if motion != null:
		_drag_position = motion.position
		if not _dragging and _drag_position.distance_to(_press_position) > DRAG_THRESHOLD:
			_dragging = true
		if _dragging:
			queue_redraw()
		return
	var button := event as InputEventMouseButton
	if button != null and button.button_index == MOUSE_BUTTON_LEFT and not button.pressed:
		_on_left_button(button)
		get_viewport().set_input_as_handled()


func _unhandled_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT:
			if not command_mode.is_empty() and button.pressed:
				click_command(button.position, button.shift_pressed)
			else:
				_on_left_button(button)
			get_viewport().set_input_as_handled()
		elif button.button_index == MOUSE_BUTTON_RIGHT and button.pressed:
			if not command_mode.is_empty():
				set_command_mode("")
			elif button.ctrl_pressed or button.meta_pressed:
				_order_attack_ground(button.position)
			elif TransportOrders.right_click(self, button.position):
				pass
			elif not _order_attack(button.position) and not selected.is_empty():
				# The move goes out on release: a drag sets the facing.
				_right_pressing = true
				_right_shift = button.shift_pressed
				_right_press = button.position
				_right_drag = button.position
			get_viewport().set_input_as_handled()
		return
	# Drag motion: see `_input`.
	var key := event as InputEventKey
	if key != null and key.pressed and not key.echo:
		if _on_action(key) or _on_key(key):
			get_viewport().set_input_as_handled()


## Order and stance keys (input actions, prompt 031). Modifiers must match
## exactly, so Ctrl+key stays free.
func _on_action(event: InputEvent) -> bool:
	if event.is_action_pressed("order_stop", false, true):
		stop_selected()
	elif event.is_action_pressed("order_fire_mode", false, true):
		toggle_hold_fire()
	elif event.is_action_pressed("camera_follow", false, true):
		set_follow(not follow, true)
	elif event.is_action_pressed("camera_center", false, true):
		center_on_active()
	elif event.is_action_pressed("order_disembark", false, true):
		TransportOrders.disembark(self)
	elif event.is_action_pressed("order_unlimber", false, true):
		TransportOrders.unlimber(self)
	elif event.is_action_pressed("order_formation", false, true):
		set_formation((formation + 1) % FormationOrders.LABELS.size())
	else:
		for action: String in BARRAGE_ACTIONS:
			if event.is_action_pressed(action, false, true):
				var b: Array = BARRAGE_ACTIONS[action]
				return order_barrage(get_viewport().get_mouse_position(), b[0], b[1], b[2])
		for action: String in STANCE_ACTIONS:
			if event.is_action_pressed(action, false, true):
				set_stance(STANCE_ACTIONS[action])
				return true
		for action: String in MODE_ACTIONS:
			if event.is_action_pressed(action, false, true):
				set_command_mode(MODE_ACTIONS[action])
				return true
		return false
	return true


## Starts (or with "" cancels) a targeted order; only with a selection.
func set_command_mode(mode: String) -> void:
	var next := mode if not selected.is_empty() else ""
	if next == command_mode:
		return
	command_mode = next
	shell_target = -1
	EngineerOrders.reset(self)
	Input.set_default_cursor_shape(Input.CURSOR_CROSS if not next.is_empty() else Input.CURSOR_ARROW)
	command_mode_changed.emit(command_mode)
	queue_redraw()


## Carries out the waiting targeted order at `screen_position`. Shift
## keeps the mode (and makes moves waypoints); otherwise it ends once the
## order went out.
func click_command(screen_position: Vector2, shift := false) -> void:
	var done := false
	match command_mode:
		"move":
			done = _order_move(screen_position, shift, FormationOrders.ENGAGE_NEVER)
		"attack_move":
			done = _order_move(screen_position, shift, FormationOrders.ENGAGE_ALL)
		"secure":
			var center: Variant = GroundPick.from_screen(camera, screen_position)
			if center != null:
				FormationOrders.secure(self, center as Vector3)
				done = true
		"attack":
			done = _order_attack(screen_position)
		"attack_ground":
			done = _order_attack_ground(screen_position)
		"attack_object":
			done = ShellOrders.click(self, screen_position)
		"patrol":
			var hit: Variant = GroundPick.from_screen(camera, screen_position)
			if hit != null and not selected.is_empty():
				var point := hit as Vector3
				sim_host.patrol_units(PackedInt64Array(selected), point.x, point.z)
				move_ordered.emit(point)
				done = true
		"garrison":
			done = _order_garrison(screen_position)
		"barrage", "smoke":
			var b: Array = BARRAGE_ACTIONS["order_" + command_mode]
			done = order_barrage(screen_position, b[0], b[1], b[2])
		"bridge_lay":
			# Bridge-laying tanks (prompt 055).
			done = CrossingOrders.click(self, screen_position)
		_:
			# Engineer orders (prompt 035), `work_<kind>`.
			done = EngineerOrders.click(self, screen_position)
	if done and not shift:
		set_command_mode("")


func _on_left_button(button: InputEventMouseButton) -> void:
	if button.pressed:
		_pressing = true
		_dragging = false
		_double_click = button.double_click
		if not button.double_click:
			_selection_before_click = selected.duplicate()
			_active_before_click = active_unit
		_press_position = button.position
		_drag_position = button.position
		return
	if not _pressing:
		return
	_pressing = false
	if _dragging:
		_dragging = false
		queue_redraw()
		_select_in_box(Rect2(_press_position, Vector2.ZERO).expand(button.position), button.shift_pressed)
	elif _double_click:
		var id := _unit_at(button.position)
		if id >= 0:
			_select_same_type(id)
		else:
			_center_on_ground(button.position)
	else:
		_click(_unit_at(button.position), button.shift_pressed)


## Control groups (fixed keys 1…9).
func _on_key(key: InputEventKey) -> bool:
	if key.keycode < KEY_1 or key.keycode > KEY_9:
		return false
	var number := int(key.keycode - KEY_0)
	if key.ctrl_pressed or key.meta_pressed:
		groups[number] = selected.duplicate()
		group_formations[number] = formation
		_last_group_key = -1
	else:
		recall_group(number)
	return true


## Selects group `number`; a second recall within DOUBLE_PRESS_MS also centers
## the camera on it.
func recall_group(number: int) -> void:
	var now := Time.get_ticks_msec()
	var again := number == _last_group_key and now - _last_group_time <= DOUBLE_PRESS_MS
	_last_group_key = number
	_last_group_time = now
	var members: Array = groups.get(number, [])
	var living := members.filter(func(id: int) -> bool: return _own_index(id) >= 0)
	# The active unit stays if it is in the group, else the group's first.
	_set_selection(living, -1 if living.is_empty() or living.has(active_unit) else int(living[0]))
	if group_formations.has(number):
		set_formation(group_formations[number])
	if again and not selected.is_empty():
		_center_camera_on_selection()


## Stance for the selected squads: -1 automatic, 0 stand, 1 kneel, 2 prone.
func set_stance(stance: int) -> void:
	var squads := _selected_squads()
	if not squads.is_empty():
		sim_host.set_stance(PackedInt64Array(squads), stance)


## Selected units that are infantry squads.
func _selected_squads() -> Array[int]:
	var stances := sim_host.unit_stances()
	var squads: Array[int] = []
	for id: int in selected:
		var index := _index_of(id)
		if index >= 0 and stances[index] >= 0:
			squads.append(id)
	return squads


func stop_selected() -> void:
	if not selected.is_empty():
		sim_host.stop_units(PackedInt64Array(selected))


## Hold fire for the selection, or fire at will if all of it holds fire.
func toggle_hold_fire() -> void:
	if selected.is_empty():
		return
	var hold := sim_host.unit_hold_fire()
	var all_hold := true
	for id: int in selected:
		var index := _index_of(id)
		all_hold = all_hold and index >= 0 and hold[index] != 0
	sim_host.set_hold_fire(PackedInt64Array(selected), not all_hold)


## Attacks the living unit of another player under the cursor; false if
## there is none (or nothing is selected).
func _order_attack(screen_position: Vector2) -> bool:
	if selected.is_empty():
		return false
	var target := _unit_at(screen_position, false)
	if target < 0:
		return false
	sim_host.attack_unit(PackedInt64Array(selected), target)
	attack_ordered.emit(target)
	return true


## Barrage (or smoke) of the selected units with an indirect weapon at
## the ground point under `screen_position`; false if none can fire.
func order_barrage(screen_position: Vector2, smoke: bool, salvos: int, radius: int) -> bool:
	var guns: Array[int] = []
	for id in selected:
		var w := sim_host.unit_indirect(id)
		if w.y > 0.0 and (not smoke or w.z > 0.0):
			guns.append(id)
	if guns.is_empty():
		return false
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit == null:
		return false
	var point := hit as Vector3
	sim_host.barrage(PackedInt64Array(guns), point.x, point.z, radius, salvos, smoke)
	barrage_ordered.emit(point, smoke)
	return true


func _order_attack_ground(screen_position: Vector2) -> bool:
	if selected.is_empty():
		return false
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit != null:
		var point := hit as Vector3
		sim_host.attack_ground(PackedInt64Array(selected), point.x, point.z)
	return hit != null


## Occupy order of the command bar: squads into the building under the
## cursor (false if there is none or no squad is selected).
func _order_garrison(screen_position: Vector2) -> bool:
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit == null:
		return false
	var point := hit as Vector3
	var building := sim_host.building_at(point.x, point.z)
	var squads := _selected_squads()
	if building < 0 or squads.is_empty():
		return false
	sim_host.garrison_units(PackedInt64Array(squads), building)
	garrison_ordered.emit(building)
	return true


func _order_move(screen_position: Vector2, queue := false,
		engage := FormationOrders.ENGAGE_INFANTRY, facing := Vector3.ZERO) -> bool:
	if selected.is_empty():
		return false
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit == null:
		return false
	order_move_to(hit as Vector3, queue, facing, engage)
	return true


## The formation for the next moves (code, prompt 046).
func set_formation(code: int) -> void:
	if code == formation:
		return
	formation = code
	formation_changed.emit(code)


## The move a held right button would give now (prompt 099): `[target,
## facing]` (facing zero below the drag threshold: direction of travel),
## or `[]` while the right button is not held for a move.
func pending_move() -> Array:
	if not _right_pressing:
		return []
	var a: Variant = GroundPick.from_screen(camera, _right_press)
	if a == null:
		return []
	var facing := Vector3.ZERO
	if _right_drag.distance_to(_right_press) > DRAG_THRESHOLD:
		var b: Variant = GroundPick.from_screen(camera, _right_drag)
		if b != null:
			facing = (b as Vector3) - (a as Vector3)
			facing.y = 0.0
	return [a as Vector3, facing]


## Right-drag (prompt 046): motion while the right button is held, and the
## release, which sends the move (with a facing if the pointer travelled).
func _on_right_drag(event: InputEvent) -> bool:
	var motion := event as InputEventMouseMotion
	if motion != null:
		_right_drag = motion.position
		queue_redraw()
		return false
	var button := event as InputEventMouseButton
	if button == null or button.button_index != MOUSE_BUTTON_RIGHT or button.pressed:
		return false
	_right_pressing = false
	queue_redraw()
	var facing := Vector3.ZERO
	if button.position.distance_to(_right_press) > DRAG_THRESHOLD:
		var a: Variant = GroundPick.from_screen(camera, _right_press)
		var b: Variant = GroundPick.from_screen(camera, button.position)
		if a != null and b != null:
			facing = (b as Vector3) - (a as Vector3)
			facing.y = 0.0
	_order_move(_right_press, _right_shift, FormationOrders.ENGAGE_INFANTRY, facing)
	get_viewport().set_input_as_handled()
	return true


## Moves the selection to the ground point (minimap, command bar); with
## `queue` the point becomes a waypoint after the current orders. Other
## moves go out in the selected formation, facing `facing` (zero: the
## direction of travel); `engage` says who halts to fight on the way.
func order_move_to(point: Vector3, queue := false, facing := Vector3.ZERO,
		engage := FormationOrders.ENGAGE_INFANTRY) -> void:
	if selected.is_empty():
		return
	if queue:
		sim_host.queue_move(PackedInt64Array(selected), point.x, point.z)
		move_ordered.emit(point)
		return
	# A building under the cursor: squads go in, vehicles drive there.
	var building := sim_host.building_at(point.x, point.z)
	var squads := _selected_squads() if building >= 0 else ([] as Array[int])
	if not squads.is_empty():
		sim_host.garrison_units(PackedInt64Array(squads), building)
		garrison_ordered.emit(building)
		var others := selected.filter(func(id: int) -> bool: return not squads.has(id))
		FormationOrders.move(self, others, point, facing, engage)
		move_ordered.emit(point)
		return
	FormationOrders.move(self, selected, point, facing, engage)
	move_ordered.emit(point)


## Selects exactly `ids` (unit panel, prompt 031); own living units only.
func set_selection(ids: Array) -> void:
	_set_selection(ids.filter(func(id: int) -> bool: return _own_index(id) >= 0))


## The clicked unit becomes the active one (prompt 094).
func _click(id: int, additive: bool) -> void:
	if not additive:
		_set_selection([] if id < 0 else [id], id)
	elif id >= 0:
		var next := selected.duplicate()
		if next.has(id):
			next.erase(id)
		else:
			next.append(id)
		_set_selection(next, id)


## Double click on empty ground: keep the selection the first click cleared
## and let the camera glide to the clicked point (which ends follow mode).
func _center_on_ground(screen_position: Vector2) -> void:
	_set_selection(_selection_before_click, _active_before_click)
	var hit: Variant = GroundPick.from_screen(camera, screen_position)
	if hit != null and camera_rig != null:
		set_follow(false)
		camera_rig.center_on(hit as Vector3)


## Turns follow mode on or off. `remember` (the player's own choice by key
## or button) keeps it in the settings for the next match; a hand pan that
## ends the mode does not.
func set_follow(on: bool, remember := false) -> void:
	_hook_camera()
	if on != follow:
		follow = on
		_follow_unit = -1
		queue_redraw()
	if remember and Settings.get_value("camera_follow_active") != on:
		Settings.set_value("camera_follow_active", on)
		Settings.save_file()


## Takes the follow mode over from the settings (start of a match).
func load_follow() -> void:
	follow = bool(Settings.get_value("camera_follow_active"))
	_follow_unit = -1
	queue_redraw()


## Lets the camera glide to the active unit; zoom and rotation stay.
func center_on_active() -> void:
	if active_unit < 0 or camera_rig == null:
		return
	if follow:
		# Already held in the center; glide back if it is still on its way.
		_follow_unit = -1
	else:
		camera_rig.center_on(_active_position)


## A hand pan ends follow mode.
func _hook_camera() -> void:
	if camera_rig != null and not _follow_hooked:
		camera_rig.panned_by_hand.connect(func() -> void: set_follow(false))
		_follow_hooked = true


func _process(delta: float) -> void:
	if command_mode == ShellOrders.MODE:
		ShellOrders.hover(self)
	if selected.is_empty():
		# Follow mode waits for the next selection and glides to it.
		_follow_unit = -1
		return
	_keep_active()
	if not follow or camera_rig == null:
		return
	_hook_camera()
	if active_unit < 0:
		# No selected unit is left alive.
		set_follow(false)
		return
	# The drawn position (between two ticks), so the picture does not step
	# with the 20 Hz of the simulation. `main.gd` placed the units before.
	camera_rig.track(_active_position, active_unit != _follow_unit, delta)
	_follow_unit = active_unit


## Every frame: the active unit must still be selectable (alive, own, not
## mounted); otherwise the selected unit nearest to where it was takes
## over. Notes where the active unit is drawn.
func _keep_active() -> void:
	var index := _selectable_index(active_unit)
	if index < 0:
		active_unit = _nearest_selected(_active_position)
		index = _index_of(active_unit) if active_unit >= 0 else -1
	if index >= 0:
		_active_position = _drawn_position(index)


## Index of unit `id` if a click could select it (own, alive, no
## passenger), else -1.
func _selectable_index(id: int) -> int:
	var index := _own_index(id) if id >= 0 else -1
	if index >= 0 and sim_host.unit_carriers()[index] >= 0 and sim_host.unit_stances()[index] >= 0:
		return -1
	return index


## The selectable selected unit nearest to `point` (x/z), or -1.
func _nearest_selected(point: Vector3) -> int:
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var best := -1
	var best_distance := INF
	for i in _own_indices():
		if not selected.has(ids[i]):
			continue
		var distance := Vector2(positions[i].x - point.x, positions[i].z - point.z).length_squared()
		if distance < best_distance:
			best = ids[i]
			best_distance = distance
	return best


## Where unit `index` is drawn this frame: between its previous and its
## current tick position, as `main.gd` places it.
func _drawn_position(index: int) -> Vector3:
	var to := sim_host.current_positions()[index]
	var previous := sim_host.previous_positions()
	if index >= previous.size():
		return to
	return previous[index].lerp(to, sim_host.interpolation_alpha())


func _select_same_type(id: int) -> void:
	if id < 0:
		_set_selection([])
		return
	var ids := sim_host.unit_ids()
	var types := sim_host.unit_types()
	var wanted := types[_index_of(id)]
	var visible_rect := get_viewport().get_visible_rect()
	var result: Array[int] = []
	for i in _own_indices():
		if types[i] == wanted and _on_screen(i, visible_rect):
			result.append(ids[i])
	# Without the active unit among them: the one nearest the screen center.
	_set_selection(result, -1, camera_rig.focus if camera_rig != null else null)


func _select_in_box(box: Rect2, additive: bool) -> void:
	var ids := sim_host.unit_ids()
	var result: Array[int] = []
	if additive:
		result = selected.duplicate()
	var caught: Array[int] = []
	for i in _own_indices():
		if _on_screen(i, box) and not result.has(ids[i]):
			caught.append(ids[i])
	if caught.any(func(id: int) -> bool: return not is_structure.call(id)):
		caught = caught.filter(func(id: int) -> bool: return not is_structure.call(id))
	result.append_array(caught)
	# Without the active unit among them: the one nearest the box's center.
	_set_selection(result, -1, GroundPick.from_screen(camera, box.get_center()))


## Id of the own (or, with `own` false, another player's) living unit under
## `screen_position`, or -1. Picks the closest one whose screen radius
## contains the point.
func _unit_at(screen_position: Vector2, own := true) -> int:
	var ids := sim_host.unit_ids()
	var positions := sim_host.current_positions()
	var footprints := sim_host.unit_footprints()
	var best := -1
	var best_distance := INF
	for i in (_own_indices() if own else _other_indices()):
		var world := positions[i] + Vector3.UP * PICK_HEIGHT
		if camera.is_position_behind(world):
			continue
		var center := camera.unproject_position(world)
		var edge := camera.unproject_position(world + camera.global_basis.x * footprints[i] * 0.5)
		var radius := maxf(MIN_PICK_RADIUS, center.distance_to(edge))
		var distance := center.distance_to(screen_position)
		if distance <= radius and distance < best_distance:
			best = ids[i]
			best_distance = distance
	return best


func _on_screen(index: int, rect: Rect2) -> bool:
	var world := sim_host.current_positions()[index] + Vector3.UP * PICK_HEIGHT
	return not camera.is_position_behind(world) and rect.has_point(camera.unproject_position(world))


func _center_camera_on_selection() -> void:
	var positions := sim_host.current_positions()
	var sum := Vector3.ZERO
	for id: int in selected:
		sum += positions[_index_of(id)]
	camera_rig.snap_to(sum / selected.size(), camera_rig.distance, camera_rig.yaw)


## Selects `ids`. The active unit (prompt 094) becomes `prefer` if that is
## among them, else it stays if it is; otherwise the selected unit nearest
## to `near` (a ground point; without one: to the old active unit, or the
## screen center) takes over.
func _set_selection(ids: Array, prefer := -1, near: Variant = null) -> void:
	var next: Array[int] = []
	for id: int in ids:
		if not next.has(id):
			next.append(id)
	next.sort()
	selected = next
	if selected.is_empty():
		active_unit = -1
	elif selected.has(prefer):
		active_unit = prefer
	elif not selected.has(active_unit):
		var from: Variant = near
		if from == null:
			from = _active_position if active_unit >= 0 else (camera_rig.focus if camera_rig != null else Vector3.ZERO)
		active_unit = _nearest_selected(from as Vector3)
	if active_unit >= 0:
		_keep_active()
	if selected.is_empty() and not command_mode.is_empty():
		set_command_mode("")


## Indices (into the SimHost arrays) of the local player's living units
## (not passengers, prompt 034).
func _own_indices() -> Array[int]:
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var carriers := sim_host.unit_carriers()
	var stances := sim_host.unit_stances()
	var me := sim_host.local_player()
	var result: Array[int] = []
	for i in owners.size():
		if owners[i] == me and alive[i] != 0 and (carriers[i] < 0 or stances[i] < 0):
			result.append(i)
	return result


## Indices of the enemies' living units (allies are no targets, prompt 037).
func _other_indices() -> Array[int]:
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var relations := sim_host.unit_relations()
	var result: Array[int] = []
	for i in owners.size():
		if relations[i] == 2 and alive[i] != 0:
			result.append(i)
	return result


func _index_of(id: int) -> int:
	return sim_host.unit_ids().find(id)


## Index of `id` if it is an existing living own unit, else -1.
func _own_index(id: int) -> int:
	var index := _index_of(id)
	if index < 0 or sim_host.unit_owners()[index] != sim_host.local_player():
		return -1
	if sim_host.unit_alive()[index] == 0:
		return -1
	return index


func _draw() -> void:
	if command_mode == ShellOrders.MODE:
		ShellOrders.draw(self)
	if not command_mode.is_empty():
		var size := get_viewport_rect().size
		draw_string(ThemeDB.fallback_font, Vector2(size.x * 0.5 - 180, 70),
			(tr(MODE_HINTS[command_mode]) if MODE_HINTS.has(command_mode) else EngineerOrders.hint(self)) + "  ·  " + tr("HUD_HINT_RIGHT_CANCELS"),
			HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(1, 0.95, 0.6, 0.95))
	if _right_pressing and _right_drag.distance_to(_right_press) > DRAG_THRESHOLD:
		# Facing arrow of a right-drag (prompt 046).
		draw_line(_right_press, _right_drag, FACING_LINE, 2.0, true)
		var back := (_right_press - _right_drag).normalized() * 12.0
		draw_line(_right_drag, _right_drag + back.rotated(0.5), FACING_LINE, 2.0, true)
		draw_line(_right_drag, _right_drag + back.rotated(-0.5), FACING_LINE, 2.0, true)
	if follow:
		var font := ThemeDB.fallback_font
		draw_string(font, Vector2(16, 28), tr("HUD_CAMERA_FOLLOWS") % Settings.action_key_name("camera_follow"),
			HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(1, 1, 1, 0.85))
	if not _dragging:
		return
	var box := Rect2(_press_position, Vector2.ZERO).expand(_drag_position)
	draw_rect(box, BOX_FILL, true)
	draw_rect(box, BOX_LINE, false, 1.0)
