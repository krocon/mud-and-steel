extends Control
## Camera control (prompt 096): a round dial above the minimap that shows
## and sets how the camera is turned and tilted, and a home button.
##
## The dial shows a compass rose lying on the ground as the camera sees it:
## it turns with the camera ("N", highlighted, always points to the map's
## north on the screen) and the tilt squashes it to an ellipse (flat view =
## narrow ellipse, steep view = nearly a circle).
##
## - Drag on the dial: sideways turns, up and down tilts (`rig.orbit_by`:
##   no glide, nothing moves on after the release).
## - Click on N, E, S or W: the camera turns to look that way, smoothly.
## - Double click, or the home button: north and the tilt of the zoom.
## - Mouse wheel over the dial: tilt in steps instead of zoom.
##
## Presentation only: it changes nothing but the camera rig. It needs the
## rig alone (`setup`), so the map editor shows the same control.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const Settings := preload("res://scenes/menu/settings.gd")

## Diameter of the dial in pixels at HUD factor 1.
const DIAL_PX := 72.0
## Radius of the rose and of the marks of N and of the other directions, as
## shares of the dial's radius.
const ROSE := 0.66
const MARK_NORTH := 0.22
const MARK_OTHER := 0.17
## Pixels the pointer must travel before a press becomes a drag.
const DRAG_THRESHOLD := 4.0
## A pixel on the dial turns and tilts this many times as far as a pixel of
## an orbit drag on the map (the dial is small).
const DRAG_GAIN := 4.0
## Bearings of the marks (radians clockwise from north) and the translation
## keys of their letters.
const DIRECTIONS: Array[String] = ["HUD_COMPASS_N", "HUD_COMPASS_E", "HUD_COMPASS_S", "HUD_COMPASS_W"]

var rig: Node3D
var home_button: HudTile
## Time of this frame's `_process` and `_draw` in microseconds (hud.gd).
var usec := 0
## What the dial shows: the camera's yaw and elevation in radians.
var turn := 0.0
var elevation := PI / 4.0

var _pressing := false
var _dragged := false
var _travel := Vector2.ZERO


func setup(camera_rig: Node3D) -> void:
	rig = camera_rig
	name = "CameraGizmo"
	mouse_filter = Control.MOUSE_FILTER_STOP
	focus_mode = Control.FOCUS_NONE
	home_button = HudTile.new("home", HudTheme.SMALL_TILE_PX)
	home_button.name = "Home"
	home_button.pressed.connect(func() -> void: rig.home())
	add_child(home_button)
	_set_tooltips()
	apply_scale()


func _set_tooltips() -> void:
	tooltip_text = tr("HUD_TIP_CAMERA_GIZMO") % [
		Settings.action_key_name("camera_tilt_up"), Settings.action_key_name("camera_tilt_down")]
	home_button.tooltip_text = tr("HUD_TIP_CAMERA_HOME")


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and home_button != null:
		_set_tooltips()
		queue_redraw()


## Sizes for the HUD factor (`hud.gd` calls it when the factor changed).
func apply_scale() -> void:
	var dial := HudTheme.px(DIAL_PX)
	home_button.apply_scale()
	home_button.position = Vector2(dial + HudTheme.px(HudTheme.GAP), 0.0)
	custom_minimum_size = Vector2(home_button.position.x + home_button.size.x, dial)
	size = custom_minimum_size
	queue_redraw()


func _process(_delta: float) -> void:
	if rig == null or not is_visible_in_tree():
		return
	var began := Time.get_ticks_usec()
	var new_turn: float = rig.yaw
	var new_elevation: float = rig.pitch()
	if not is_equal_approx(new_turn, turn) or not is_equal_approx(new_elevation, elevation):
		turn = new_turn
		elevation = new_elevation
		queue_redraw()
	usec += Time.get_ticks_usec() - began


## Middle and radius of the dial in this control.
func dial_center() -> Vector2:
	return Vector2(size.y, size.y) * 0.5


func dial_radius() -> float:
	return size.y * 0.5


## Where the mark of direction `index` (0 north, 1 east, 2 south, 3 west)
## lies in this control.
func mark_position(index: int) -> Vector2:
	var angle := turn + index * PI * 0.5
	var radius := dial_radius() * ROSE
	return dial_center() + Vector2(sin(angle) * radius, -cos(angle) * radius * sin(elevation))


## The direction whose mark is at `point`, or -1.
func direction_at(point: Vector2) -> int:
	var best := -1
	var best_distance := INF
	for index in DIRECTIONS.size():
		var reach := maxf(dial_radius() * (MARK_NORTH if index == 0 else MARK_OTHER), 7.0) + 1.0
		var distance := point.distance_to(mark_position(index))
		if distance <= reach and distance < best_distance:
			best = index
			best_distance = distance
	return best


func _gui_input(event: InputEvent) -> void:
	if rig == null:
		return
	var button := event as InputEventMouseButton
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed and button.double_click:
				_pressing = false
				rig.home()
			elif button.pressed:
				_pressing = true
				_dragged = false
				_travel = Vector2.ZERO
			else:
				if _pressing and not _dragged:
					var index := direction_at(button.position)
					if index >= 0:
						rig.face(index * PI * 0.5)
				_pressing = false
				_dragged = false
				queue_redraw()
		elif button.pressed and button.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			# Trackpads send fractional notches through `factor`.
			var notches := button.factor if button.factor > 0.0 else 1.0
			rig.tilt_step(notches if button.button_index == MOUSE_BUTTON_WHEEL_UP else -notches)
		accept_event()
		return
	var motion := event as InputEventMouseMotion
	if motion != null and _pressing:
		_travel += motion.relative
		var gain := DRAG_GAIN / maxf(HudTheme.scale, 0.1)
		if _dragged:
			rig.orbit_by(motion.relative * gain)
		elif _travel.length() > DRAG_THRESHOLD:
			# The way up to here counts: the drag loses nothing.
			_dragged = true
			rig.orbit_by(_travel * gain)
			queue_redraw()
		accept_event()


func _draw() -> void:
	var began := Time.get_ticks_usec()
	var center := dial_center()
	var radius := dial_radius()
	var line := maxf(1.4 * HudTheme.scale, 1.0)
	draw_circle(center, radius, HudTheme.PANEL)
	draw_arc(center, radius - 0.5, 0.0, TAU, 48, HudTheme.GOLD if _dragged else HudTheme.PANEL_BORDER, 1.0, true)
	# The rose: its rim, the two axes, the north half in gold with a tip.
	var rose := radius * ROSE
	var squash := sin(elevation)
	var rim := PackedVector2Array()
	for k in 49:
		var angle := TAU * k / 48.0
		rim.append(center + Vector2(sin(angle) * rose, -cos(angle) * rose * squash))
	draw_polyline(rim, Color(HudTheme.TEXT_DIM, 0.55), line, true)
	var marks: Array[Vector2] = []
	for index in DIRECTIONS.size():
		marks.append(mark_position(index))
	draw_line(marks[1], marks[3], Color(HudTheme.TEXT_DIM, 0.45), line, true)
	draw_line(center, marks[2], Color(HudTheme.TEXT_DIM, 0.45), line, true)
	draw_line(center, marks[0], HudTheme.GOLD, line * 1.5, true)
	draw_circle(center, line * 1.4, HudTheme.TEXT_DIM)
	# The marks, the far ones first so the near ones lie over them.
	var order: Array = range(DIRECTIONS.size())
	order.sort_custom(func(a: int, b: int) -> bool: return marks[a].y < marks[b].y)
	var font := ThemeDB.fallback_font
	for index: int in order:
		var north := index == 0
		var mark := radius * (MARK_NORTH if north else MARK_OTHER)
		draw_circle(marks[index], mark, HudTheme.GOLD if north else HudTheme.TILE)
		if not north:
			draw_arc(marks[index], mark - 0.5, 0.0, TAU, 20, HudTheme.TILE_BORDER, 1.0, true)
		var text := tr(DIRECTIONS[index])
		var text_size := maxi(roundi(mark * (1.45 if north else 1.5)), 7)
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, text_size).x
		draw_string(font, marks[index] + Vector2(-width * 0.5, text_size * 0.36), text, HORIZONTAL_ALIGNMENT_LEFT, -1, text_size,
			Color(0.08, 0.07, 0.04) if north else HudTheme.TEXT)
	usec += Time.get_ticks_usec() - began
