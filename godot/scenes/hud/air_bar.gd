extends Control
## Air support calls (prompts 033 and 095): one square tile per mission in
## the support column at the right edge (`hud.support`), under the
## resources. This node itself has no size; it owns the tiles, the keys and
## the target click.
##
## A tile shows the mission's symbol, under it the price (gold: fuel /
## ammunition / manpower) or the seconds left (blue), and a dark sector
## while it cools down; it is greyed while the call is blocked
## (`SimHost.air_options`). A tile (or its key, Shift + 1…5 by default,
## Shift + 8 for the circling recon)
## waits for a left click on the map, which calls the mission there
## (`SimHost.call_air`); right click or Escape cancels. The hint line shows
## only while a target is picked (`hud.hints`).
## Dragging instead of clicking (prompt 055) aims along the line
## (`SimHost.call_air_line`): the aircraft fly from its start to its end,
## bombers lay a carpet over it (at most `air_carpet_max_m`), transports
## drop their paratroopers along it.
## Presentation only: the simulation checks cooldown and price again.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const REFRESH := 0.2
## Side of a support tile at HUD factor 1.
const TILE_PX := 56.0
const MISSIONS: Array[Array] = [
	["recon", "HUD_AIR_RECON"],
	["recon_loiter", "HUD_AIR_RECON_LOITER"],
	["strafe", "HUD_AIR_STRAFE"],
	["attack", "HUD_AIR_ATTACK"],
	["bomb", "HUD_AIR_BOMB"],
	["airborne", "HUD_AIR_AIRBORNE"],
]
const HINT := "HUD_HINT_AIR"
## A drag shorter than this (m) is a click.
const DRAG_MIN_M := 15.0

var main: Node3D
var sim_host: SimHost
## Mission waiting for its target click ("" = none).
var pending := ""
## Tile per mission (self-test).
var buttons: Dictionary[String, Button] = {}
## Calls sent (self-test).
var calls := 0

var _refresh_left := 0.0
var _hint: Label
## Ground point where a drag started (null: none).
var _drag_from: Variant = null
var _options: Dictionary = {}
## Longest cooldown seen per mission, the full circle of its sector.
var _cooldown_full: Dictionary[String, float] = {}


## `column` takes the tiles (the support column of the HUD), `hints` the
## hint line.
func setup(main_node: Node3D, column: Control, hints: Control) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "AirBar"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	for k in MISSIONS.size():
		var spec: Array = MISSIONS[k]
		var b := HudTile.new("air_" + str(spec[0]), TILE_PX, true)
		b.name = "Air_" + spec[0]
		b.toggle_mode = true
		b.pressed.connect(_on_button.bind(spec[0]))
		column.add_child(b)
		buttons[spec[0]] = b
	_hint = Label.new()
	_hint.name = "AirHint"
	_hint.text = HINT
	_hint.visible = false
	_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_hint.add_theme_color_override(&"font_color", Color(1, 0.95, 0.6))
	_hint.add_theme_color_override(&"font_outline_color", Color.BLACK)
	_hint.add_theme_constant_override(&"outline_size", 5)
	hints.add_child(_hint)
	apply_scale()
	_refresh()


func apply_scale() -> void:
	for m: String in buttons:
		(buttons[m] as HudTile).apply_scale()
	_hint.add_theme_font_size_override(&"font_size", HudTheme.font(14))


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
		var mission: String = spec[0]
		var b: HudTile = buttons[mission]
		var o: Dictionary = _options.get(mission, {})
		b.visible = visible
		if o.is_empty():
			b.disabled = true
			b.footer = ""
			b.cooldown = 0.0
			continue
		var cost: PackedInt32Array = o["cost"]
		var cooldown: float = o["cooldown"]
		var blocked: String = o["blocked"]
		if cooldown > 0.0:
			_cooldown_full[mission] = maxf(_cooldown_full.get(mission, 0.0), cooldown)
			b.footer = tr("COMMON_SECONDS") % ceili(cooldown)
			b.footer_color = HudTheme.BLUE
			b.cooldown = cooldown / _cooldown_full[mission]
		else:
			_cooldown_full.erase(mission)
			b.footer = "%d/%d/%d" % [cost[0], cost[1], cost[2]]
			b.footer_color = HudTheme.RED if blocked == "not enough resources" else HudTheme.GOLD
			b.cooldown = 0.0
		b.key_text = Settings.action_key_name("air_" + mission).replace("Shift+", "")
		b.disabled = not blocked.is_empty()
		var reason := Picker.blocked_text(blocked)
		b.tooltip_text = tr("HUD_TIP_AIR").format({
			"mission": tr(spec[1]), "key": Settings.action_key_name("air_" + mission),
			"aircraft": str(o.get("aircraft_name", "")) if str(o["aircraft"]) != "" else tr("HUD_BLOCKED_NO_AIRCRAFT"),
			"fuel": cost[0], "ammo": cost[1], "manpower": cost[2]}) \
			+ ("" if reason.is_empty() else "\n" + reason[0].to_upper() + reason.substr(1))
		b.set_pressed_no_signal(pending == mission)
	if not pending.is_empty() and buttons[pending].disabled:
		set_pending("")


func _on_button(mission: String) -> void:
	set_pending("" if pending == mission else mission)


## Waits for the target of `mission` ("" cancels).
func set_pending(mission: String) -> void:
	pending = mission
	_drag_from = null
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


## Calls the pending mission along the ground line `from` → `to` (a drag;
## also used by the self-test).
func call_along(from: Vector3, to: Vector3) -> bool:
	if pending.is_empty():
		return false
	sim_host.call_air_line(pending, Vector2(from.x, from.z), Vector2(to.x, to.z))
	calls += 1
	set_pending("")
	return true


func _input(event: InputEvent) -> void:
	var key := event as InputEventKey
	# Keys of the calls (Shift+1…5 by default, rebindable, prompt 043).
	if key != null and key.pressed and not key.echo:
		for spec in MISSIONS:
			var mission: String = spec[0]
			if InputMap.has_action("air_" + mission) and key.is_action_pressed("air_" + mission, false, true):
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
	var motion := event as InputEventMouseMotion
	if motion != null and _drag_from != null:
		var over: Variant = GroundPick.from_screen(main.camera, motion.position)
		if over != null:
			var d := Vector2((over as Vector3).x - (_drag_from as Vector3).x, (over as Vector3).z - (_drag_from as Vector3).z).length()
			_hint.text = tr("HUD_HINT_AIR_DRAG") % roundi(minf(d, sim_host.air_carpet_max_m()))
		return
	var button := event as InputEventMouseButton
	if button == null:
		return
	if not button.pressed:
		# The end of a drag (prompt 055): a run along the line, or a click.
		if button.button_index == MOUSE_BUTTON_LEFT and _drag_from != null:
			var from: Vector3 = _drag_from
			_drag_from = null
			_hint.text = HINT
			var end: Variant = GroundPick.from_screen(main.camera, button.position)
			if end != null and Vector2((end as Vector3).x - from.x, (end as Vector3).z - from.z).length() >= DRAG_MIN_M:
				call_along(from, end as Vector3)
			else:
				sim_host.call_air(pending, from.x, from.z)
				calls += 1
				set_pending("")
			get_viewport().set_input_as_handled()
		return
	var hud: Node = main.get("hud")
	if hud != null and hud.covers(button.position):
		return
	if get_global_rect().has_point(button.position):
		return
	if button.button_index == MOUSE_BUTTON_LEFT:
		_drag_from = GroundPick.from_screen(main.camera, button.position)
		get_viewport().set_input_as_handled()
	elif button.button_index == MOUSE_BUTTON_RIGHT:
		set_pending("")
		get_viewport().set_input_as_handled()
