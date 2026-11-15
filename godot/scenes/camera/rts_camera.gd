extends Node3D
## RTS camera rig: orbits a focus point on the ground.
##
## The rig node itself sits on the focus point. The camera is placed from four
## values: focus, distance (along the view ray), pitch (elevation above the
## horizon, derived from the zoom level) and yaw (0 = looking north, i.e. -Z).
##
## Presentation only: nothing here touches the simulation. All key bindings
## come from the input map in project.godot (actions named camera_*).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")

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
## Shadow distance as a multiple of the camera distance.
@export var shadow_distance_factor := 2.5
## Far plane as a multiple of the camera distance.
@export var far_factor := 4.0
## The camera stays at least this many meters above the ground below it.
@export var min_clearance := 3.0
## Light whose shadow distance follows the zoom. Optional.
@export var sun: DirectionalLight3D

## Area the focus point must stay in (x/z plane, meters).
var bounds := Rect2(-1000, -1000, 2000, 2000)

var focus := Vector3.ZERO
var distance := 80.0
var target_distance := 80.0
var yaw := 0.0
var target_yaw := 0.0

var _zoom_anchor := Vector2.ZERO
var _dragging := false

@onready var camera: Camera3D = $Camera3D


func _ready() -> void:
	if DisplayServer.get_name() == "headless":
		edge_scroll_enabled = false
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
	_apply()


## Zoom level from 0 (closest) to 1 (farthest), logarithmic in the distance.
func zoom_level() -> float:
	return clampf(log(distance / min_distance) / log(max_distance / min_distance), 0.0, 1.0)


## Elevation angle above the horizon in radians for the current zoom.
func pitch() -> float:
	var t := zoom_level()
	var blend := pitch_curve.sample(t) if pitch_curve else smoothstep(0.0, 1.0, t)
	return deg_to_rad(lerpf(pitch_near_deg, pitch_far_deg, blend))


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("camera_zoom_in"):
		_zoom_by(1.0 / (1.0 + zoom_step * _wheel_factor(event)), event)
	elif event.is_action_pressed("camera_zoom_out"):
		_zoom_by(1.0 + zoom_step * _wheel_factor(event), event)
	elif event is InputEventMagnifyGesture:
		_zoom_by(1.0 / (event as InputEventMagnifyGesture).factor, event)
	elif event.is_action_pressed("camera_rotate_left"):
		target_yaw += PI / 4.0
	elif event.is_action_pressed("camera_rotate_right"):
		target_yaw -= PI / 4.0
	elif event.is_action_pressed("camera_reset_north"):
		# Turn the shortest way back to a multiple of a full turn.
		target_yaw = roundf(yaw / TAU) * TAU
	elif event.is_action_pressed("camera_drag"):
		_dragging = true
	elif event.is_action_released("camera_drag"):
		_dragging = false
	elif event is InputEventMouseMotion and _dragging:
		_drag(event as InputEventMouseMotion)
	else:
		return
	get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	var direction := Input.get_vector(
		"camera_pan_left", "camera_pan_right", "camera_pan_forward", "camera_pan_back"
	)
	if direction == Vector2.ZERO and edge_scroll_enabled:
		direction = _edge_direction()
	if direction != Vector2.ZERO:
		var step := direction * pan_speed * distance * delta
		focus += Vector3(step.x, 0.0, step.y).rotated(Vector3.UP, yaw)

	yaw = lerpf(yaw, target_yaw, _smooth(rotate_smoothing, delta))
	if absf(yaw - target_yaw) < 0.0001:
		yaw = target_yaw

	if distance != target_distance:
		var anchor_before: Variant = GroundPick.from_screen(camera, _zoom_anchor)
		# Interpolate in log space so every wheel notch feels the same.
		distance = exp(lerpf(log(distance), log(target_distance), _smooth(zoom_smoothing, delta)))
		if absf(distance - target_distance) < 0.001:
			distance = target_distance
		_apply()
		# Keep the ground point under the cursor where it was.
		var anchor_after: Variant = GroundPick.from_screen(camera, _zoom_anchor)
		if anchor_before != null and anchor_after != null:
			focus += (anchor_before as Vector3) - (anchor_after as Vector3)

	_apply()


func _zoom_by(factor: float, event: InputEvent) -> void:
	target_distance = clampf(target_distance * factor, min_distance, max_distance)
	var mouse := event as InputEventMouse
	_zoom_anchor = (
		mouse.position if mouse else get_viewport().get_visible_rect().size * 0.5
	)


## Grab-the-ground drag: the point under the cursor follows the cursor.
func _drag(motion: InputEventMouseMotion) -> void:
	var before: Variant = GroundPick.from_screen(camera, motion.position - motion.relative)
	var after: Variant = GroundPick.from_screen(camera, motion.position)
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
	camera.look_at(focus, Vector3.UP)

	camera.near = maxf(0.05, distance * 0.02)
	camera.far = distance * far_factor
	if sun:
		sun.directional_shadow_max_distance = distance * shadow_distance_factor


static func _smooth(rate: float, delta: float) -> float:
	return 1.0 - exp(-rate * delta)


static func _wheel_factor(event: InputEvent) -> float:
	var button := event as InputEventMouseButton
	# Trackpads send fractional wheel events through `factor`; mice send 0 or 1.
	if button and button.factor > 0.0:
		return button.factor
	return 1.0
