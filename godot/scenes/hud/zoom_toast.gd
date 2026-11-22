extends Label
## A short line in the upper middle of the screen (presentation only):
## "Zoom 150 %" while the zoom changes (100 % = the view a match starts
## with, `rts_camera.gd zoom_percent`), and the messages of the camera keys
## ("Combat 2 / 3", `action_spots.gd`). Shown `SECONDS`, then it fades.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const SECONDS := 1.2
const FADE := 0.4
## Distance of the line from the top, in parts of the screen height.
const TOP_SHARE := 0.2

var main: Node3D
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Text shown last (self-test).
var shown_text := ""

var _left := 0.0
var _last_distance := -1.0


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "ZoomToast"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	modulate.a = 0.0
	apply_scale()


func apply_scale() -> void:
	add_theme_font_size_override(&"font_size", HudTheme.font(18))
	add_theme_color_override(&"font_color", HudTheme.TEXT)
	add_theme_color_override(&"font_outline_color", Color(0, 0, 0, 0.85))
	add_theme_constant_override(&"outline_size", int(HudTheme.px(5)))
	var plate := HudTheme.box(Color(HudTheme.PANEL, 0.7), Color(0, 0, 0, 0), HudTheme.px(6), HudTheme.RADIUS)
	plate.content_margin_left = HudTheme.px(14)
	plate.content_margin_right = HudTheme.px(14)
	add_theme_stylebox_override(&"normal", plate)


## Shows `line` for `SECONDS`.
func show_text(line: String) -> void:
	text = line
	shown_text = line
	_left = SECONDS
	modulate.a = 1.0
	size = Vector2.ZERO


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	var rig: Node3D = main.camera_rig
	if rig != null:
		var wanted: float = rig.target_distance
		if _last_distance >= 0.0 and not is_equal_approx(wanted, _last_distance):
			show_text(tr("HUD_ZOOM_PERCENT") % rig.zoom_percent())
		_last_distance = wanted
	if _left > 0.0:
		_left -= delta
		modulate.a = clampf(_left / FADE, 0.0, 1.0)
	var screen := get_viewport_rect().size
	position = Vector2(roundf((screen.x - size.x) * 0.5), roundf(screen.y * TOP_SHARE))
	usec += Time.get_ticks_usec() - began
