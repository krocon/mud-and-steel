extends RefCounted
## Player settings (prompt 027), stored in `user://settings.cfg`.
##
## Static, so menu, match and LAN lobby share one copy without an autoload.
## Every entry scene calls `ensure_loaded()`, which also applies the key
## bindings and the volume. Presentation only: nothing here reaches the
## simulation.
##
## The self-test (`--self-test`) uses `user://settings-selftest.cfg`, starting
## from the defaults, so it never touches the player's file. `--settings=<path>`
## picks another file.

const PATH := "user://settings.cfg"
const SELF_TEST_PATH := "user://settings-selftest.cfg"
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")

## Keyboard actions the player can rebind (all from project.godot), with
## their labels in the settings screen. Mouse bindings stay fixed.
const BINDABLE: Dictionary[String, String] = {
	"camera_pan_forward": "Pan forward",
	"camera_pan_back": "Pan back",
	"camera_pan_left": "Pan left",
	"camera_pan_right": "Pan right",
	"camera_rotate_left": "Rotate left",
	"camera_rotate_right": "Rotate right",
	"camera_reset_north": "Face north",
	# Orders and HUD (prompt 031).
	"order_move": "Move (then click)",
	"order_attack": "Attack (then click)",
	"order_attack_ground": "Fire at ground (then click)",
	"order_patrol": "Patrol (then click)",
	"order_garrison": "Occupy building (then click)",
	"order_stop": "Stop",
	"order_fire_mode": "Hold fire / fire at will",
	"stance_stand": "Stance: stand",
	"stance_kneel": "Stance: kneel",
	"stance_prone": "Stance: prone",
	"stance_auto": "Stance: automatic",
	"camera_follow": "Camera follows selection",
	"event_jump": "Jump to last event",
	"order_barrage": "Barrage at the cursor (artillery)",
	"order_smoke": "Smoke at the cursor (artillery)",
	# Teams (prompt 037).
	"ping_allies": "Mark the spot for allies",
}

## Health bars over units (prompt 031): `damaged` (damaged and selected
## units), `selected` (selected units only), `never`.
const HEALTH_BAR_MODES: Array[String] = ["damaged", "selected", "never"]

## Section "game" of the file. Sensitivities are factors on the camera's
## tuned speeds (0.25–3).
const DEFAULTS := {
	"quality": GraphicsQuality.DEFAULT,
	"master_volume": 0.8,
	# Volumes per audio bus and mute without window focus (prompt 029).
	"effects_volume": 1.0,
	"ui_volume": 0.8,
	"music_volume": 0.7,
	"ambience_volume": 0.8,
	"mute_unfocused": true,
	"pan_sensitivity": 1.0,
	"zoom_sensitivity": 1.0,
	"edge_scroll": true,
	"health_bars": "damaged",
}

static var values := {}
## Physical keycode of the first keyboard event per bindable action.
static var bindings: Dictionary[String, int] = {}
static var _default_bindings: Dictionary[String, int] = {}
static var _loaded := false


static func path() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--settings="):
			return arg.trim_prefix("--settings=")
	return SELF_TEST_PATH if "--self-test" in OS.get_cmdline_user_args() else PATH


## Loads the file once per run and applies bindings and volume.
static func ensure_loaded() -> void:
	if not _loaded:
		load_file()


static func load_file() -> void:
	_loaded = true
	if _default_bindings.is_empty():
		for action in BINDABLE:
			_default_bindings[action] = _first_key(action)
	values = DEFAULTS.duplicate()
	bindings = _default_bindings.duplicate()
	var file := ConfigFile.new()
	if file.load(path()) == OK:
		for key in DEFAULTS:
			var value: Variant = file.get_value("game", key, DEFAULTS[key])
			if typeof(value) == typeof(DEFAULTS[key]) or (DEFAULTS[key] is float and value is int):
				values[key] = value
		for action in BINDABLE:
			var code: int = file.get_value("keys", action, bindings[action])
			if code > 0:
				bindings[action] = code
	values["quality"] = values["quality"] if values["quality"] in GraphicsQuality.LEVELS else GraphicsQuality.DEFAULT
	values["health_bars"] = values["health_bars"] if values["health_bars"] in HEALTH_BAR_MODES else "damaged"
	apply()


static func save_file() -> bool:
	var file := ConfigFile.new()
	for key in values:
		file.set_value("game", key, values[key])
	for action in bindings:
		file.set_value("keys", action, bindings[action])
	var error := file.save(path())
	if error != OK:
		push_error("cannot save settings to %s: %s" % [path(), error_string(error)])
	return error == OK


## Back to the defaults (not saved).
static func reset() -> void:
	values = DEFAULTS.duplicate()
	bindings = _default_bindings.duplicate()
	apply()


static func get_value(key: String) -> Variant:
	ensure_loaded()
	return values.get(key, DEFAULTS.get(key))


static func set_value(key: String, value: Variant) -> void:
	ensure_loaded()
	values[key] = value
	apply()


## Binds `action` to the physical key `code` (replacing its first keyboard
## event). A key used by another bindable action is taken from it, so no key
## does two things.
static func bind(action: String, code: int) -> void:
	ensure_loaded()
	for other in bindings:
		if other != action and bindings[other] == code:
			bindings[other] = bindings[action]
	bindings[action] = code
	apply()


## Name of the key bound to `action` ("" if none), for button hints.
static func action_key_name(action: String) -> String:
	ensure_loaded()
	var code: int = bindings.get(action, 0)
	return key_name(code) if code > 0 else ""


## Name of the physical key `code` on the current keyboard layout.
static func key_name(code: int) -> String:
	if code <= 0:
		return "–"
	if DisplayServer.get_name() == "headless":
		return OS.get_keycode_string(code as Key)
	return OS.get_keycode_string(DisplayServer.keyboard_get_keycode_from_physical(code as Key))


## Graphics quality: `--quality=<level>` wins over the saved setting.
static func quality() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--quality="):
			return GraphicsQuality.from_cmdline()
	return get_value("quality")


## Applies key bindings and volumes (one per audio bus, prompt 029). The
## camera reads its factors through `apply_camera`.
static func apply() -> void:
	for action in bindings:
		_set_first_key(action, bindings[action])
	SoundBuses.apply(values)
	UiSound.install()


## Sets the camera's speeds from its tuned base values times the factors.
static func apply_camera(rig: Node) -> void:
	ensure_loaded()
	if not rig.has_meta(&"base_pan_speed"):
		rig.set_meta(&"base_pan_speed", rig.pan_speed)
		rig.set_meta(&"base_zoom_step", rig.zoom_step)
		rig.set_meta(&"base_edge_scroll", rig.edge_scroll_enabled)
	rig.pan_speed = rig.get_meta(&"base_pan_speed") * float(get_value("pan_sensitivity"))
	rig.zoom_step = rig.get_meta(&"base_zoom_step") * float(get_value("zoom_sensitivity"))
	rig.edge_scroll_enabled = rig.get_meta(&"base_edge_scroll") and bool(get_value("edge_scroll"))


static func _first_key(action: String) -> int:
	for event in InputMap.action_get_events(action):
		var key := event as InputEventKey
		if key != null:
			return key.physical_keycode
	return 0


static func _set_first_key(action: String, code: int) -> void:
	if not InputMap.has_action(action) or code <= 0:
		return
	var key := InputEventKey.new()
	key.physical_keycode = code as Key
	var events: Array[InputEvent] = []
	var replaced := false
	for event in InputMap.action_get_events(action):
		if event is InputEventKey and not replaced:
			events.append(key)
			replaced = true
		else:
			events.append(event)
	if not replaced:
		events.push_front(key)
	InputMap.action_erase_events(action)
	for event in events:
		InputMap.action_add_event(action, event)
