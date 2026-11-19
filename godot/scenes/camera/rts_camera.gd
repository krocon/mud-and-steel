extends Node3D
## RTS camera rig: orbits a focus point on the ground.
##
## The rig node itself sits on the focus point. The camera is placed from four
## values: focus, distance (along the view ray), pitch (elevation above the
## horizon) and yaw (0 = looking north, i.e. -Z).
##
## The pitch is the zoom's curve plus the player's tilt (prompt 096):
## `pitch() = base_pitch() + tilt`, kept between `tilt_min_deg` and
## `tilt_max_deg`. With tilt 0 the view is the one the zoom gives. Yaw and
## tilt each have a target they follow smoothly (keys, clicks on the camera
## control); a drag (`orbit_by`) moves both at once, so nothing swings on
## after the release. Neither is stored: every match starts facing north
## with tilt 0.
##
## Presentation only: nothing here touches the simulation. All key bindings
## come from the input map in project.godot (actions named camera_*).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")

## Emitted when the player moves the map by hand (keys, edge, drag, trackpad).
signal panned_by_hand

@export_group("Zoom")
## Closest distance from camera to focus point, in meters.
@export var min_distance := 15.0
## Farthest distance from camera to focus point, in meters.
@export var max_distance := 600.0
## Start distance in meters.
@export var start_distance := 80.0
## Distance factor per mouse wheel notch (0.15 = 15 % closer or farther).
@export var zoom_step := 0.15
## How fast the distance follows the target (1/s, exponential smoothing).
@export var zoom_smoothing := 12.0

@export_group("Pitch")
## Elevation angle at the closest zoom, in degrees.
@export var pitch_near_deg := 45.0
## Elevation angle at the farthest zoom, in degrees.
@export var pitch_far_deg := 65.0
## Maps zoom level (0 = near, 1 = far, logarithmic) to the pitch blend (0..1).
## Without a curve, smoothstep is used.
@export var pitch_curve: Curve
## Flattest and steepest elevation the player's tilt may reach, in degrees.
## Below about 25 degrees the upper screen edge nears the horizon: the
## visible ground grows without bound and the sky shows.
@export var tilt_min_deg := 25.0
@export var tilt_max_deg := 85.0
## Tilt per key press or wheel notch on the camera control, in degrees.
@export var tilt_step_deg := 10.0
## How fast the tilt follows its target (1/s, exponential smoothing).
@export var tilt_smoothing := 10.0
## Degrees of turn or tilt per pixel of an orbit drag on the map.
@export var orbit_deg_per_pixel := 0.3
## true: dragging up tilts flatter instead of steeper (a setting).
@export var tilt_inverted := false

@export_group("Pan")
## Keyboard and edge pan speed in meters per second per meter of distance,
## so panning feels the same at every zoom level.
@export var pan_speed := 1.2
## Scroll when the mouse touches the window border.
@export var edge_scroll_enabled := true
## Width of the scroll border in pixels.
@export var edge_scroll_margin := 8.0

@export_group("Rotation")
## How fast the yaw follows Q/E steps (1/s, exponential smoothing).
@export var rotate_smoothing := 10.0

@export_group("Rendering")
## Field of view in degrees. Small, so the view is only slightly perspective.
@export var fov_deg := 30.0
## Shadow distance as a multiple of the camera distance: the least the
## shadows must reach. The distance in use is the next step above it, see
## `shadow_distance_for`. (world/atmosphere.gd sets this factor.)
@export var shadow_distance_factor := 2.5
## Far plane as a multiple of the camera distance.
@export var far_factor := 4.0
## The camera stays at least this many meters above the ground below it.
@export var min_clearance := 3.0
## ... and this many meters above the roof of a building it is over.
@export var building_clearance := 1.5
## Light whose shadow distance follows the zoom. Optional.
@export var sun: DirectionalLight3D

@export_group("Trackpad and mouse")  # l10n-ignore (editor inspector)
## Screen pixels the map moves per unit of a trackpad two-finger pan gesture
## (macOS reports trackpad scrolling as InputEventPanGesture).
@export var trackpad_pan_scale := 40.0
## How fast the camera glides to a point centered by double click.
@export var glide_smoothing := 8.0

## Shadow distance steps (prompt 090): 50 m times a power of this ratio.
## A shadow distance that glides with the zoom changes the size of a shadow
## texel on the ground every frame, so every shadow edge crawls while
## zooming; between two steps the shadow maps stay the same.
const SHADOW_STEP_BASE := 50.0
const SHADOW_STEP_RATIO := 1.41421356
## A smaller step is taken only when it is this much more than needed, so
## zooming back and forth at a boundary does not switch every time.
const SHADOW_STEP_HYSTERESIS := 0.12
## Near plane as a share of the shadow distance step. The cascades start
## at the near plane, so it steps with them (at most 2 % of the distance).
const NEAR_PER_SHADOW_STEP := 0.02 / (SHADOW_STEP_RATIO * (1.0 + SHADOW_STEP_HYSTERESIS))

## Shadows and far plane reach this many times as far as the ground at the
## upper screen edge (`ground_reach`): the shadows fade out over the last
## fifth of their distance, and ground falling away lies deeper.
const SHADOW_PER_REACH := 1.3
const FAR_PER_REACH := 1.6

## Area the focus point must stay in (x/z plane, meters).
var bounds := Rect2(-1000, -1000, 2000, 2000)

var focus := Vector3.ZERO
var distance := 80.0
var target_distance := 80.0
var yaw := 0.0
var target_yaw := 0.0
## The player's tilt in radians, added to the zoom's pitch (0 = as the zoom
## gives it, positive = steeper), and the value it glides to.
var tilt := 0.0
var target_tilt := 0.0

## false (`-- --no-shadow-steps`, for comparisons): the shadow distance
## glides with the zoom as before prompt 090.
var shadow_steps := true
## Shadow distance step in use (m), 0 before the first frame.
var shadow_step := 0.0

var _zoom_anchor := Vector2.ZERO
var _dragging := false
## Point the focus glides to (double click on the ground), or null.
var _glide_target: Variant = null
## Follow mode (`track`, prompt 094): what is left of the way from where
## the focus was to the tracked point (x/z), and whether `track` was called
## since the last `_process`.
var _track_offset := Vector3.ZERO
var _tracked := false

@onready var camera: Camera3D = $Camera3D


func _ready() -> void:
	if DisplayServer.get_name() == "headless":
		edge_scroll_enabled = false
	shadow_steps = "--no-shadow-steps" not in OS.get_cmdline_user_args()
	focus = Vector3(global_position.x, 0.0, global_position.z)
	distance = clampf(start_distance, min_distance, max_distance)
	target_distance = distance
	camera.fov = fov_deg
	_zoom_anchor = get_viewport().get_visible_rect().size * 0.5
	_apply()


## Places the camera immediately, without smoothing (for tests and screenshots).
func snap_to(new_focus: Vector3, new_distance: float, new_yaw := 0.0) -> void:
	focus = new_focus
	distance = clampf(new_distance, min_distance, max_distance)
	target_distance = distance
	yaw = new_yaw
	target_yaw = new_yaw
	# The tilt is the player's and stays; a glide to its target ends here.
	tilt = target_tilt
	# The same picture however the camera got here (tests, screenshots).
	shadow_step = 0.0
	_apply()


## Glides the focus smoothly to `point` (e.g. a double click on the ground).
## Any manual pan stops the glide.
func center_on(point: Vector3) -> void:
	_glide_target = point


## Keeps `point` in the middle of the screen (follow mode, prompt 094); to
## be called every frame, after the units are placed. With `restart` the
## focus glides there from where it is; the rest of that way shrinks every
## frame, and once it is gone the focus sits on the point itself. So the
## camera does not trail behind a driving unit as a glide to its position
## would. Zoom and rotation stay free (the zoom goes to the screen center
## meanwhile); a hand pan is reported by `panned_by_hand` as always.
func track(point: Vector3, restart: bool, delta: float) -> void:
	_glide_target = null
	_tracked = true
	if restart:
		_track_offset = Vector3(focus.x - point.x, 0.0, focus.z - point.z)
	else:
		_track_offset *= exp(-glide_smoothing * delta)
		if _track_offset.length() < 0.05:
			_track_offset = Vector3.ZERO
	focus = point + _track_offset
	_apply()


## Whether the camera is still gliding to a point set by `center_on`.
func is_gliding() -> bool:
	return _glide_target != null


## Zoom level from 0 (closest) to 1 (farthest), logarithmic in the distance.
func zoom_level() -> float:
	return clampf(log(distance / min_distance) / log(max_distance / min_distance), 0.0, 1.0)


## Elevation angle above the horizon in radians: the zoom's angle plus the
## player's tilt, within the tilt limits.
func pitch() -> float:
	return clampf(base_pitch() + tilt, deg_to_rad(tilt_min_deg), deg_to_rad(tilt_max_deg))


## Elevation angle the zoom alone gives at `at_distance` meters (the
## current distance if negative), in radians.
func base_pitch(at_distance := -1.0) -> float:
	var t := zoom_level()
	if at_distance > 0.0:
		t = clampf(log(at_distance / min_distance) / log(max_distance / min_distance), 0.0, 1.0)
	var blend := pitch_curve.sample(t) if pitch_curve else smoothstep(0.0, 1.0, t)
	return deg_to_rad(lerpf(pitch_near_deg, pitch_far_deg, blend))


## The tilts that reach the flattest and the steepest view at the zoom the
## camera is heading for (x = lowest, y = highest, radians).
func tilt_range() -> Vector2:
	var base := base_pitch(target_distance)
	return Vector2(deg_to_rad(tilt_min_deg) - base, deg_to_rad(tilt_max_deg) - base)


## Tilts by `steps` key steps (positive = steeper), smoothly.
func tilt_step(steps: float) -> void:
	var limits := tilt_range()
	target_tilt = clampf(target_tilt + deg_to_rad(tilt_step_deg) * steps, limits.x, limits.y)


## Turns by `turn` and tilts by `lift` radians at once (a drag): no glide,
## so nothing moves on after the release. A turn or tilt that is still
## gliding keeps its remaining way.
func orbit(turn: float, lift: float) -> void:
	yaw += turn
	target_yaw += turn
	var limits := tilt_range()
	var before := target_tilt
	target_tilt = clampf(target_tilt + lift, limits.x, limits.y)
	tilt += target_tilt - before
	_apply()


## An orbit drag of `pixels` on the map or on the camera control: the near
## side of the map follows the pointer sideways (as a turntable seen from
## the front), up is steeper (flatter with `tilt_inverted`).
func orbit_by(pixels: Vector2) -> void:
	var rate := deg_to_rad(orbit_deg_per_pixel)
	orbit(-pixels.x * rate, pixels.y * rate * (1.0 if tilt_inverted else -1.0))


## Turns smoothly, the shortest way, until the camera looks along `bearing`
## (radians clockwise from north: 0 north, PI / 2 east).
func face(bearing: float) -> void:
	target_yaw = yaw + angle_difference(yaw, -bearing)


## Back to north and to the tilt of the zoom, smoothly (the home button).
func home() -> void:
	face(0.0)
	target_tilt = 0.0


## Sets the tilt at once, within the limits (tests, pictures).
func snap_tilt(new_tilt: float) -> void:
	var limits := tilt_range()
	target_tilt = clampf(new_tilt, limits.x, limits.y)
	tilt = target_tilt
	shadow_step = 0.0
	_apply()


## How deep into the picture the ground at the upper screen edge lies, as a
## multiple of the camera distance (flat ground): 1.4 at 45 degrees, 2.4 at
## 25. Shadows and the far plane must reach at least this far.
func ground_reach() -> float:
	return edge_depths(pitch()).y


## View depth of the ground at the lower (x) and the upper (y) screen edge
## for a camera `elevation` radians above the horizon, as multiples of the
## camera distance (flat ground; the focus in the screen center is at 1).
func edge_depths(elevation: float) -> Vector2:
	var half := deg_to_rad(fov_deg) * 0.5
	var scale := sin(elevation) * cos(half)
	return Vector2(scale / sin(minf(elevation + half, PI * 0.5)), scale / sin(maxf(elevation - half, deg_to_rad(6.0))))


## How much farther the visible ground reaches in front of (x) and behind
## (y) the focus than with the tilt of the zoom: (1, 1) with tilt 0, about
## (1.7, 3.7) tilted flat at the closest zoom. Haze and depth of field
## (`world/atmosphere.gd`) stretch by it, so the screen edges look as hazy
## and as soft at every tilt.
func view_stretch() -> Vector2:
	var now := edge_depths(pitch())
	var normal := edge_depths(base_pitch())
	return Vector2((1.0 - now.x) / maxf(1.0 - normal.x, 0.001), (now.y - 1.0) / maxf(normal.y - 1.0, 0.001))


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("camera_zoom_in"):
		_zoom_by(1.0 / (1.0 + zoom_step * _wheel_factor(event)), event)
	elif event.is_action_pressed("camera_zoom_out"):
		_zoom_by(1.0 + zoom_step * _wheel_factor(event), event)
	elif event is InputEventMagnifyGesture:
		_zoom_by(1.0 / (event as InputEventMagnifyGesture).factor, event)
	elif event is InputEventPanGesture:
		# Two fingers on the trackpad: the map follows the fingers.
		var pan := event as InputEventPanGesture
		_drag_by(pan.position, -pan.delta * trackpad_pan_scale)
	elif event.is_action_pressed("camera_rotate_left"):
		target_yaw += PI / 4.0
	elif event.is_action_pressed("camera_rotate_right"):
		target_yaw -= PI / 4.0
	elif event.is_action_pressed("camera_reset_north"):
		# Turn the shortest way back to a multiple of a full turn.
		target_yaw = roundf(yaw / TAU) * TAU
	elif event.is_action_pressed("camera_tilt_up", true, true):
		tilt_step(1.0)
	elif event.is_action_pressed("camera_tilt_down", true, true):
		tilt_step(-1.0)
	elif event.is_action_pressed("camera_drag"):
		_dragging = true
	elif event.is_action_released("camera_drag"):
		_dragging = false
	elif event is InputEventMouseMotion and (_dragging or (event as InputEventMouseMotion).alt_pressed):
		var motion := event as InputEventMouseMotion
		if motion.shift_pressed:
			# With Shift: turn and tilt around the screen center.
			orbit_by(motion.relative)
		else:
			# Middle mouse button held, or Option/Alt held: grab the map.
			_drag_by(motion.position, motion.relative)
	else:
		return
	get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	# Followed since the last frame: the zoom must not push the focus away.
	var tracking := _tracked
	_tracked = false
	var direction := Input.get_vector(
		"camera_pan_left", "camera_pan_right", "camera_pan_forward", "camera_pan_back"
	)
	if direction == Vector2.ZERO and edge_scroll_enabled:
		direction = _edge_direction()
	if direction != Vector2.ZERO:
		_glide_target = null
		panned_by_hand.emit()
		var step := direction * pan_speed * distance * delta
		focus += Vector3(step.x, 0.0, step.y).rotated(Vector3.UP, yaw)
	elif _glide_target != null:
		var target := _glide_target as Vector3
		focus = focus.lerp(target, _smooth(glide_smoothing, delta))
		if Vector2(focus.x - target.x, focus.z - target.z).length() < 0.05:
			focus = target
			_glide_target = null

	yaw = lerpf(yaw, target_yaw, _smooth(rotate_smoothing, delta))
	if absf(yaw - target_yaw) < 0.0001:
		yaw = target_yaw
	# Zooming moves the limits: the target stays what the player set, the
	# view itself is clamped in `pitch`.
	tilt = lerpf(tilt, target_tilt, _smooth(tilt_smoothing, delta))
	if absf(tilt - target_tilt) < 0.0001:
		tilt = target_tilt

	if distance != target_distance:
		var anchor_before: Variant = GroundPick.from_screen(camera, _zoom_anchor)
		# Interpolate in log space so every wheel notch feels the same.
		distance = exp(lerpf(log(distance), log(target_distance), _smooth(zoom_smoothing, delta)))
		if absf(distance - target_distance) < 0.001:
			distance = target_distance
		_apply()
		# Keep the ground point under the cursor where it was.
		var anchor_after: Variant = GroundPick.from_screen(camera, _zoom_anchor)
		# (Not while following a unit: it stays in the screen center.)
		if not tracking and anchor_before != null and anchor_after != null:
			focus += (anchor_before as Vector3) - (anchor_after as Vector3)

	_apply()


func _zoom_by(factor: float, event: InputEvent) -> void:
	target_distance = clampf(target_distance * factor, min_distance, max_distance)
	var mouse := event as InputEventMouse
	_zoom_anchor = (
		mouse.position if mouse else get_viewport().get_visible_rect().size * 0.5
	)


## Grab-the-ground drag: the ground point that was at `position - relative`
## (screen pixels) moves to `position`, i.e. the map follows the pointer.
func _drag_by(position: Vector2, relative: Vector2) -> void:
	_glide_target = null
	panned_by_hand.emit()
	var before: Variant = GroundPick.from_screen(camera, position - relative)
	var after: Variant = GroundPick.from_screen(camera, position)
	if before != null and after != null:
		focus += (before as Vector3) - (after as Vector3)
		_apply()


func _edge_direction() -> Vector2:
	if not DisplayServer.window_is_focused():
		return Vector2.ZERO
	var size := get_viewport().get_visible_rect().size
	var mouse := get_viewport().get_mouse_position()
	if mouse.x < 0.0 or mouse.y < 0.0 or mouse.x > size.x or mouse.y > size.y:
		return Vector2.ZERO
	var direction := Vector2.ZERO
	if mouse.x <= edge_scroll_margin:
		direction.x = -1.0
	elif mouse.x >= size.x - edge_scroll_margin:
		direction.x = 1.0
	if mouse.y <= edge_scroll_margin:
		direction.y = -1.0
	elif mouse.y >= size.y - edge_scroll_margin:
		direction.y = 1.0
	return direction.normalized()


## Clamps the focus and places the camera from focus, distance, pitch and yaw.
func _apply() -> void:
	focus.x = clampf(focus.x, bounds.position.x, bounds.end.x)
	focus.z = clampf(focus.z, bounds.position.y, bounds.end.y)
	focus.y = GroundPick.height_at(focus.x, focus.z)
	global_position = focus

	var elevation := pitch()
	var back := Vector3(0.0, sin(elevation), cos(elevation)).rotated(Vector3.UP, yaw)
	camera.global_position = focus + back * distance
	# Close to a hillside the camera would end up inside the ground: lift it.
	var ground := GroundPick.height_at(camera.global_position.x, camera.global_position.z)
	camera.global_position.y = maxf(camera.global_position.y, ground + min_clearance)
	if tilt < 0.0:
		# Tilted flat, a rise between camera and focus would hide the focus
		# (or cut through the near plane): the camera rises until the view
		# ray to the focus clears the ground under it.
		for part: float in [0.25, 0.5, 0.75]:
			var under := focus + back * (distance * part)
			var needed := GroundPick.height_at(under.x, under.z) + min_clearance * part - focus.y
			camera.global_position.y = maxf(camera.global_position.y, focus.y + needed / part)
	# Never inside a building (prompt 078): at the closest zoom the camera
	# is about 11 m up, below the roofs of apartment blocks, church and
	# town hall. Within 2 m of a building it rises over its roof.
	var top := GroundPick.object_top_at(camera.global_position.x, camera.global_position.z, 2.0)
	camera.global_position.y = maxf(camera.global_position.y, top + building_clearance)
	camera.look_at(focus, Vector3.UP)

	if shadow_steps:
		shadow_step = shadow_step_for(distance * shadow_reach(), shadow_step)
		camera.near = near_for(shadow_step)
	else:
		shadow_step = distance * shadow_reach()
		camera.near = maxf(0.05, distance * 0.02)
	# A flat view looks farther (prompt 096): with the tilt at 0 both
	# factors are the exported ones, as before.
	camera.far = distance * maxf(far_factor, ground_reach() * FAR_PER_REACH)
	if sun:
		sun.directional_shadow_max_distance = shadow_step


## Shadow distance wanted per meter of camera distance: the exported factor,
## more when the tilt shows ground beyond it.
func shadow_reach() -> float:
	return maxf(shadow_distance_factor, ground_reach() * SHADOW_PER_REACH)


## The smallest step that reaches `wanted` meters (0 = no step yet).
static func shadow_step_above(wanted: float) -> float:
	var n := ceili(log(maxf(wanted, 1.0) / SHADOW_STEP_BASE) / log(SHADOW_STEP_RATIO) - 0.0001)
	return SHADOW_STEP_BASE * pow(SHADOW_STEP_RATIO, n)


## The shadow distance step for `wanted` meters when `current` is in use:
## up at once when the shadows would end too early, down only when the
## smaller step still has `SHADOW_STEP_HYSTERESIS` to spare.
static func shadow_step_for(wanted: float, current: float) -> float:
	var step := shadow_step_above(wanted)
	if current <= 0.0 or step > current:
		return step
	if shadow_step_above(wanted * (1.0 + SHADOW_STEP_HYSTERESIS)) < current:
		return shadow_step_above(wanted * (1.0 + SHADOW_STEP_HYSTERESIS))
	return current


## Near plane for a shadow distance step: between 1.3 % and 2 % of the
## camera distance, and the same for as long as the step is.
func near_for(step: float) -> float:
	return maxf(0.05, step * NEAR_PER_SHADOW_STEP / maxf(shadow_reach(), 0.001))


static func _smooth(rate: float, delta: float) -> float:
	return 1.0 - exp(-rate * delta)


static func _wheel_factor(event: InputEvent) -> float:
	var button := event as InputEventMouseButton
	# Trackpads send fractional wheel events through `factor`; mice send 0 or 1.
	if button and button.factor > 0.0:
		return button.factor
	return 1.0
