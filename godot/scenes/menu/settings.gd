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
const Language := preload("res://scenes/menu/language.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")

## Keyboard actions the player can rebind, with the translation keys of
## their labels in the settings screen. Mouse bindings, control groups
## (1…9) and the picker's digit keys stay fixed [provisional]. A binding is a
## physical keycode, optionally with modifier masks (Shift+1).
const BINDABLE: Dictionary[String, String] = {
	"camera_pan_forward": "SET_KEY_CAMERA_PAN_FORWARD",
	"camera_pan_back": "SET_KEY_CAMERA_PAN_BACK",
	"camera_pan_left": "SET_KEY_CAMERA_PAN_LEFT",
	"camera_pan_right": "SET_KEY_CAMERA_PAN_RIGHT",
	"camera_rotate_left": "SET_KEY_CAMERA_ROTATE_LEFT",
	"camera_rotate_right": "SET_KEY_CAMERA_ROTATE_RIGHT",
	"camera_reset_north": "SET_KEY_CAMERA_RESET_NORTH",
	# Orders and HUD (prompt 031).
	"order_move": "SET_KEY_ORDER_MOVE",
	"order_attack": "SET_KEY_ORDER_ATTACK",
	"order_attack_ground": "SET_KEY_ORDER_ATTACK_GROUND",
	"order_patrol": "SET_KEY_ORDER_PATROL",
	"order_garrison": "SET_KEY_ORDER_GARRISON",
	"order_stop": "SET_KEY_ORDER_STOP",
	"order_fire_mode": "SET_KEY_ORDER_FIRE_MODE",
	"stance_stand": "SET_KEY_STANCE_STAND",
	"stance_kneel": "SET_KEY_STANCE_KNEEL",
	"stance_prone": "SET_KEY_STANCE_PRONE",
	"stance_auto": "SET_KEY_STANCE_AUTO",
	"camera_follow": "SET_KEY_CAMERA_FOLLOW",
	"event_jump": "SET_KEY_EVENT_JUMP",
	"order_barrage": "SET_KEY_ORDER_BARRAGE",
	"order_smoke": "SET_KEY_ORDER_SMOKE",
	# Teams (prompt 037).
	"ping_allies": "SET_KEY_PING_ALLIES",
	# Transport (prompt 034).
	"order_disembark": "SET_KEY_ORDER_DISEMBARK",
	"order_unlimber": "SET_KEY_ORDER_UNLIMBER",
	# Formations (prompt 046).
	"order_attack_move": "SET_KEY_ORDER_ATTACK_MOVE",
	"order_secure": "SET_KEY_ORDER_SECURE",
	"order_formation": "SET_KEY_ORDER_FORMATION",
	# Air calls and the rocket strike (prompts 033, 054), rebindable since
	# prompt 043; their actions are added at run time (RUNTIME_ACTIONS).
	"air_recon": "SET_KEY_AIR_RECON",
	"air_strafe": "SET_KEY_AIR_STRAFE",
	"air_attack": "SET_KEY_AIR_ATTACK",
	"air_bomb": "SET_KEY_AIR_BOMB",
	"air_airborne": "SET_KEY_AIR_AIRBORNE",
	"strike_rockets": "SET_KEY_STRIKE_ROCKETS",
	# Unit card of the selected unit (prompt 066).
	"unit_card": "SET_KEY_UNIT_CARD",
}

## Actions that are not in `project.godot`, with their default key
## (physical keycode with modifier masks). Added to the InputMap once.
const RUNTIME_ACTIONS: Dictionary[String, int] = {
	"air_recon": KEY_1 | KEY_MASK_SHIFT,
	"air_strafe": KEY_2 | KEY_MASK_SHIFT,
	"air_attack": KEY_3 | KEY_MASK_SHIFT,
	"air_bomb": KEY_4 | KEY_MASK_SHIFT,
	"air_airborne": KEY_5 | KEY_MASK_SHIFT,
	"strike_rockets": KEY_6 | KEY_MASK_SHIFT,
	# Plain I is "Secure" (prompt 046); every letter key is taken.
	"unit_card": KEY_I | KEY_MASK_SHIFT,
}

## Health bars over units (prompt 031): `damaged` (damaged and selected
## units), `selected` (selected units only), `never`.
const HEALTH_BAR_MODES: Array[String] = ["damaged", "selected", "never"]

## Tilt-shift depth of field (`atmosphere.gd`, `DOF_MODES`): `off`, `subtle`
## (soft screen edges at close zoom, prompt 057), `miniature` (strong, like a
## photo of a model).
const DOF_MODES: Array[String] = ["off", "subtle", "miniature"]

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
	# Tilt-shift depth of field, one of DOF_MODES.
	"depth_of_field": "subtle",
	# Shadows of passing clouds (prompt 087).
	"cloud_shadows": true,
	# Language and accessibility (prompt 043).
	"language": "auto",
	"ui_scale": 1.0,
	"font_size": 1.0,
	# Air bar and order grid folded open (prompt 092).
	"hud_air_open": true,
	"hud_commands_open": true,
	"color_palette": "standard",
	"subtitles": false,
	"screen_shake": true,
	# Display (prompt 043); "" resolution = leave the window as it is.
	"window_mode": "windowed",
	"resolution": "",
	"vsync": true,
	"fps_limit": 0,
	"render_scale": 1.0,
	"upscaler": "fsr1",
}

## `language`: `auto` (system language) or a code of `Language.LANGUAGES`.
## `window_mode`: one of WINDOW_MODES. `fps_limit`: one of FPS_LIMITS (0 =
## none). `upscaler`: one of UPSCALERS, used while `render_scale` < 1.
const WINDOW_MODES: Array[String] = ["windowed", "fullscreen", "exclusive"]
const RESOLUTIONS: Array[String] = ["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"]
const FPS_LIMITS: Array[int] = [0, 30, 60, 120, 144, 240]
const UPSCALERS: Array[String] = ["bilinear", "fsr1", "fsr2"]
const FONT_SIZES: Array[float] = [0.85, 1.0, 1.15, 1.3]

static var values := {}
## Physical keycode of the first keyboard event per bindable action.
static var bindings: Dictionary[String, int] = {}
static var _default_bindings: Dictionary[String, int] = {}
static var _loaded := false
## Display values last applied (`_apply_display`).
static var _applied := {}


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
	for action in RUNTIME_ACTIONS:
		if not InputMap.has_action(action):
			InputMap.add_action(action)
			_set_first_key(action, RUNTIME_ACTIONS[action])
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
		# Older files stored the depth of field as a switch.
		if file.get_value("game", "depth_of_field", "") is bool:
			values["depth_of_field"] = "subtle" if file.get_value("game", "depth_of_field") else "off"
		for action in BINDABLE:
			var code: int = file.get_value("keys", action, bindings[action])
			if code > 0:
				bindings[action] = code
	values["quality"] = values["quality"] if values["quality"] in GraphicsQuality.LEVELS else GraphicsQuality.DEFAULT
	values["health_bars"] = values["health_bars"] if values["health_bars"] in HEALTH_BAR_MODES else "damaged"
	values["depth_of_field"] = values["depth_of_field"] if values["depth_of_field"] in DOF_MODES else "subtle"
	for key: String in ["language", "color_palette", "window_mode", "upscaler", "resolution"]:
		var allowed: Array = {
			"language": ["auto"] + Language.LANGUAGES.keys(), "color_palette": RelationColors.PALETTES,
			"window_mode": WINDOW_MODES, "upscaler": UPSCALERS, "resolution": [""] + RESOLUTIONS,
		}[key]
		if not values[key] in allowed:
			values[key] = DEFAULTS[key]
	values["ui_scale"] = clampf(values["ui_scale"], 0.75, 1.5)
	values["font_size"] = clampf(values["font_size"], 0.85, 1.3)
	values["render_scale"] = clampf(values["render_scale"], 0.5, 1.0)
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
	var plain := DisplayServer.keyboard_get_keycode_from_physical((code & KEY_CODE_MASK) as Key)
	return OS.get_keycode_string((plain | (code & KEY_MODIFIER_MASK)) as Key)


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
	Language.apply(values["language"])
	RelationColors.palette = values["color_palette"]
	MenuTheme.set_font_scale(values["font_size"])
	_apply_display()


## How many window pixels one interface pixel takes on this screen without
## the setting (prompt 092): the scale the system uses for it (2 on a Retina
## display; elsewhere from its dots per inch, 96 = 1, in quarters). The
## window is as large as the screen's real pixels, so without this factor
## every text is half as high there. 1 headless and while recording
## (pictures and videos stay the same on every machine);
## `-- --display-scale=<n>` sets it for tests and pictures.
static func display_scale() -> float:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--display-scale="):
			return clampf(arg.trim_prefix("--display-scale=").to_float(), 0.5, 4.0)
	if DisplayServer.get_name() == "headless" or OS.has_feature("movie"):
		return 1.0
	var screen := DisplayServer.window_get_current_screen()
	var factor := DisplayServer.screen_get_scale(screen)
	if is_equal_approx(factor, 1.0):
		factor = snappedf(DisplayServer.screen_get_dpi(screen) / 96.0, 0.25)
	return clampf(factor, 1.0, 3.0)


## The interface scale for a window `height` pixels high: the setting times
## the screen's own scale (`display_scale`), but enlarged at most so that
## the interface keeps 720 pixels of room (menus are built for 1600×900)
## [provisional]. Headless (self-test) it is the setting.
static func ui_scale_for(height: int) -> float:
	var wanted: float = values.get("ui_scale", 1.0) * display_scale()
	if wanted <= 1.0 or height <= 0 or DisplayServer.get_name() == "headless":
		return wanted
	return clampf(float(height) / 720.0, 1.0, wanted)


## Fits the interface scale to the window again when its size changes
## (maximized, full screen, dragged): the 720-pixel limit depends on it.
static func _on_window_resized() -> void:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and DisplayServer.get_name() != "headless":
		tree.root.content_scale_factor = ui_scale_for(tree.root.size.y)


## UI scale, window, VSync, frame limit and render scale (prompt 043). Only
## what changed since the last call is applied, so tools that set VSync or
## the window themselves keep it; nothing of it runs headless.
static func _apply_display() -> void:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return
	var root := tree.root
	var wanted := {}
	for key in ["ui_scale", "window_mode", "resolution", "vsync", "fps_limit", "render_scale", "upscaler"]:
		wanted[key] = values[key]
	var changed := func(key: String) -> bool:
		return not _applied.has(key) or _applied[key] != wanted[key]
	if changed.call("ui_scale") or changed.call("window_mode") or changed.call("resolution"):
		root.content_scale_factor = ui_scale_for(root.size.y)
	if not root.size_changed.is_connected(_on_window_resized):
		root.size_changed.connect(_on_window_resized)
	if changed.call("fps_limit"):
		Engine.max_fps = wanted["fps_limit"]
	if changed.call("render_scale") or changed.call("upscaler"):
		var scale: float = wanted["render_scale"]
		root.scaling_3d_scale = scale
		root.scaling_3d_mode = Viewport.SCALING_3D_MODE_BILINEAR if scale >= 1.0 else {
			"bilinear": Viewport.SCALING_3D_MODE_BILINEAR, "fsr1": Viewport.SCALING_3D_MODE_FSR,
			"fsr2": Viewport.SCALING_3D_MODE_FSR2,
		}[wanted["upscaler"]]
	if DisplayServer.get_name() != "headless" and not _applied.is_empty():
		if changed.call("vsync"):
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if wanted["vsync"] else DisplayServer.VSYNC_DISABLED)
		if changed.call("window_mode"):
			DisplayServer.window_set_mode({
				"windowed": DisplayServer.WINDOW_MODE_WINDOWED, "fullscreen": DisplayServer.WINDOW_MODE_FULLSCREEN,
				"exclusive": DisplayServer.WINDOW_MODE_EXCLUSIVE_FULLSCREEN,
			}[wanted["window_mode"]])
		if (changed.call("resolution") or changed.call("window_mode")) and wanted["window_mode"] == "windowed" \
				and not str(wanted["resolution"]).is_empty():
			var parts: PackedStringArray = str(wanted["resolution"]).split("x")
			var size := Vector2i(int(parts[0]), int(parts[1]))
			var screen := DisplayServer.screen_get_usable_rect(DisplayServer.window_get_current_screen())
			size = size.min(screen.size)
			DisplayServer.window_set_size(size)
			DisplayServer.window_set_position(screen.position + (screen.size - size) / 2)
	elif DisplayServer.get_name() != "headless":
		# First call of the run: the engine started windowed with VSync on;
		# apply only what differs, so a plain start keeps its window.
		if not wanted["vsync"]:
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		if wanted["window_mode"] != "windowed" or not str(wanted["resolution"]).is_empty():
			_applied = {"window_mode": "windowed", "resolution": "", "vsync": wanted["vsync"]}
			_apply_display()
			return
	_applied = wanted


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
			return key.get_physical_keycode_with_modifiers()
	return 0


static func _set_first_key(action: String, code: int) -> void:
	if not InputMap.has_action(action) or code <= 0:
		return
	var key := InputEventKey.new()
	key.physical_keycode = (code & KEY_CODE_MASK) as Key
	key.shift_pressed = code & KEY_MASK_SHIFT != 0
	key.ctrl_pressed = code & KEY_MASK_CTRL != 0
	key.alt_pressed = code & KEY_MASK_ALT != 0
	key.meta_pressed = code & KEY_MASK_META != 0
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
