extends Node
## Tactical pause and game speed in single player (prompt 102).
##
## The pause stops the scene tree like the pause menu, so the simulation
## (`SimHost` runs no tick), the units, particles, animations and the 3D
## sounds stand. Unlike the pause menu, the HUD layer (selection, orders,
## minimap, tooltips, circles, previews), the camera and the unit highlight
## keep running (`PROCESS_MODE_ALWAYS` while the pause lasts): orders given
## now wait in `SimHost` and go out with the first tick after the pause, in
## the order they were given. A pause is no input of the simulation: a
## replay of a paused match is the same as one without. Music plays
## `MUSIC_DUCK_DB` quieter; a quiet line at the top says the game is
## paused (no shade over the picture).
##
## Speed (`game_speed`, cycles 1×, 2×, ½×): `SimHost.speed`, the number of
## ticks per frame; the rules do not change.
##
## The Esc menu still pauses everything; closed again, the tactical pause
## goes on. Network matches and replays have neither (the replay bar has its
## own pause and speeds).
##
## Actions: `tactical_pause` (Shift + P), `game_speed` (the key right of
## Ü / [ on US keyboards); the pause tile beside the minimap toggles it.

const Settings := preload("res://scenes/menu/settings.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")

const SPEEDS: Array[float] = [1.0, 2.0, 0.5]
const MUSIC_DUCK_DB := -9.0

var main: Node3D
## The tactical pause is on.
var active := false
var speed := 1.0
var _hint: Label
var _music_db := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "TacticalPause"
	process_mode = Node.PROCESS_MODE_ALWAYS
	_hint = Label.new()
	_hint.name = "PauseHint"
	_hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_hint.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
	_hint.add_theme_color_override(&"font_color", HudTheme.GOLD)
	_hint.add_theme_color_override(&"font_outline_color", Color(0, 0, 0, 0.85))
	_hint.add_theme_constant_override(&"outline_size", 4)
	_hint.visible = false
	main.get_node("Hud").add_child(_hint)


## Whether this match may pause and change speed: single player, live.
func available() -> bool:
	var host: SimHost = main.sim_host
	return not host.is_networked() and not host.is_replay()


func toggle() -> void:
	if active:
		resume()
	else:
		pause()


func pause() -> void:
	if active or not available():
		return
	active = true
	get_tree().paused = true
	_keep_running(true)
	var bus := AudioServer.get_bus_index("Music")
	if bus >= 0:
		_music_db = AudioServer.get_bus_volume_db(bus)
		AudioServer.set_bus_volume_db(bus, _music_db + MUSIC_DUCK_DB)
	_update_hint()


func resume() -> void:
	if not active:
		return
	active = false
	_keep_running(false)
	if not main.game_menu.is_paused():
		get_tree().paused = false
	var bus := AudioServer.get_bus_index("Music")
	if bus >= 0:
		AudioServer.set_bus_volume_db(bus, _music_db)
	_update_hint()


## Next speed of `SPEEDS`.
func cycle_speed() -> void:
	if not available():
		return
	set_speed(SPEEDS[(SPEEDS.find(speed) + 1) % SPEEDS.size()])


func set_speed(value: float) -> void:
	speed = value
	main.sim_host.speed = value
	_update_hint()


## The Esc menu opened (everything stops) or closed (the HUD runs again if
## the tactical pause is on, and the tree stays paused).
func menu_opened() -> void:
	if active:
		_keep_running(false)


func menu_closed() -> void:
	if active:
		get_tree().paused = true
		_keep_running(true)


func _keep_running(on: bool) -> void:
	var mode := Node.PROCESS_MODE_ALWAYS if on else Node.PROCESS_MODE_INHERIT
	for node: Node in [main.get_node("Hud"), main.camera_rig, main.get("unit_highlight")]:
		if node != null:
			node.process_mode = mode


func _update_hint() -> void:
	var parts: PackedStringArray = []
	if active:
		parts.append(tr("HUD_PAUSED") % Settings.action_key_name("tactical_pause"))
	if not is_equal_approx(speed, 1.0):
		parts.append(tr("HUD_SPEED") % ("½" if speed < 1.0 else str(int(speed))))  # l10n-ignore
	_hint.text = "   ".join(parts)
	_hint.add_theme_font_size_override(&"font_size", HudTheme.font(16))
	_hint.visible = not parts.is_empty()
	_hint.position.y = HudTheme.px(64)
	_hint.size.x = 0
	_hint.position.x = (_hint.get_viewport_rect().size.x - _hint.get_minimum_size().x) * 0.5


func _unhandled_input(event: InputEvent) -> void:
	if main.game_menu.is_paused():
		return
	if event.is_action_pressed("tactical_pause", false, true):
		if available():
			toggle()
			get_viewport().set_input_as_handled()
	elif event.is_action_pressed("game_speed", false, true):
		if available():
			cycle_speed()
			get_viewport().set_input_as_handled()
