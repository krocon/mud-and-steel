extends PanelContainer
## Air support call bar (prompt 033), docked above the command bar.
##
## One button per mission with its price and cooldown (`SimHost.air_options`).
## A button (or Shift + 1…4) waits for a left click on the map, which calls
## the mission there (`SimHost.call_air`); right click or Escape cancels.
## Presentation only: the simulation checks cooldown and price again.
## [provisional] keys and layout.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const REFRESH := 0.2
const MISSIONS: Array[Array] = [
	["recon", "Recon"],
	["strafe", "Fighters"],
	["attack", "Attack"],
	["bomb", "Bombers"],
]
const HINT := "Air support: left click the target  ·  right click cancels"

var main: Node3D
var sim_host: SimHost
## Mission waiting for its target click ("" = none).
var pending := ""
## Button per mission (self-test).
var buttons: Dictionary[String, Button] = {}
## Calls sent (self-test).
var calls := 0

var _refresh_left := 0.0
var _hint: Label
var _options: Dictionary = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "AirBar"
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	add_child(box)
	var row := HBoxContainer.new()
	box.add_child(row)
	var title := Label.new()
	title.text = "Air"
	title.add_theme_font_size_override(&"font_size", 13)
	row.add_child(title)
	for k in MISSIONS.size():
		var spec: Array = MISSIONS[k]
		var b := Button.new()
		b.name = "Air_" + spec[0]
		b.custom_minimum_size = Vector2(88, 40)
		b.focus_mode = Control.FOCUS_NONE
		b.toggle_mode = true
		b.add_theme_font_size_override(&"font_size", 12)
		b.pressed.connect(_on_button.bind(spec[0]))
		row.add_child(b)
		buttons[spec[0]] = b
	_hint = Label.new()
	_hint.text = HINT
	_hint.visible = false
	_hint.add_theme_font_size_override(&"font_size", 12)
	_hint.add_theme_color_override(&"font_color", Color(1, 0.95, 0.6))
	box.add_child(_hint)
	_refresh()


func _process(delta: float) -> void:
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh()


func _refresh() -> void:
	if sim_host == null:
		return
	_options.clear()
	for o: Dictionary in sim_host.air_options():
		_options[o["mission"]] = o
	visible = not _options.is_empty()
	for k in MISSIONS.size():
		var spec: Array = MISSIONS[k]
		var b: Button = buttons[spec[0]]
		var o: Dictionary = _options.get(spec[0], {})
		if o.is_empty():
			b.disabled = true
			continue
		var cost: PackedInt32Array = o["cost"]
		var cooldown: float = o["cooldown"]
		var blocked: String = o["blocked"]
		b.text = "%s\n%s" % [spec[1], ("%d s" % ceili(cooldown)) if cooldown > 0.0 else "%d/%d/%d" % [cost[0], cost[1], cost[2]]]
		b.disabled = not blocked.is_empty()
		b.tooltip_text = "%s (Shift+%d): %s\nFuel %d, ammo %d, manpower %d%s" % [
			spec[1], k + 1, str(o.get("aircraft_name", "")) if str(o["aircraft"]) != "" else "no aircraft",
			cost[0], cost[1], cost[2],
			"" if blocked.is_empty() else "\n" + blocked.capitalize()]
		b.set_pressed_no_signal(pending == spec[0])
	if not pending.is_empty() and buttons[pending].disabled:
		set_pending("")


func _on_button(mission: String) -> void:
	set_pending("" if pending == mission else mission)


## Waits for the target of `mission` ("" cancels).
func set_pending(mission: String) -> void:
	pending = mission
	_hint.visible = not pending.is_empty()
	for m: String in buttons:
		buttons[m].set_pressed_no_signal(m == pending)
	Input.set_default_cursor_shape(Input.CURSOR_CROSS if not pending.is_empty() else Input.CURSOR_ARROW)
	var selection: Node = main.get("selection")
	if not pending.is_empty() and selection != null:
		selection.set_command_mode("")


## Calls the pending mission on the ground point under `screen_position`.
func call_at(screen_position: Vector2) -> bool:
	var hit: Variant = GroundPick.from_screen(main.camera, screen_position)
	if hit == null or pending.is_empty():
		return false
	var point := hit as Vector3
	sim_host.call_air(pending, point.x, point.z)
	calls += 1
	set_pending("")
	return true


func _input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key != null and key.pressed and not key.echo and key.shift_pressed \
			and not key.ctrl_pressed and not key.meta_pressed and not key.alt_pressed:
		var k := int(key.keycode - KEY_1)
		if k >= 0 and k < MISSIONS.size():
			var mission: String = MISSIONS[k][0]
			if not buttons[mission].disabled:
				set_pending("" if pending == mission else mission)
			get_viewport().set_input_as_handled()
			return
	if pending.is_empty():
		return
	if key != null and key.pressed and key.keycode == KEY_ESCAPE:
		set_pending("")
		get_viewport().set_input_as_handled()
		return
	var button := event as InputEventMouseButton
	if button == null or not button.pressed:
		return
	var hud: Node = main.get("hud")
	if hud != null and hud.covers(button.position):
		return
	if get_global_rect().has_point(button.position):
		return
	if button.button_index == MOUSE_BUTTON_LEFT:
		call_at(button.position)
		get_viewport().set_input_as_handled()
	elif button.button_index == MOUSE_BUTTON_RIGHT:
		set_pending("")
		get_viewport().set_input_as_handled()
