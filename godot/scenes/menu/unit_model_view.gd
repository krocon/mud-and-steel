extends SubViewportContainer
## A unit's 3D model to turn and zoom (prompt 066), for the unit card of the
## reference in the main menu.
##
## The model is the one the picker thumbnails are rendered from
## (`tools/thumbnail_render.gd` `build_model`: kit models with the faction's
## camouflage of the biome, squads from their squad scene, base buildings as
## houses), lit the same way, in its own world. Left drag turns it, the
## wheel zooms; it turns slowly by itself until the player takes over.
## Presentation only.

const ThumbnailRender := preload("res://scenes/tools/thumbnail_render.gd")
const BIOMES: Array[String] = ["forest", "desert", "winter"]
const SPIN_DEG_PER_S := 14.0
## Camera distance as a share of the one that shows the model's bounding
## sphere whole (a vehicle is far smaller than its sphere).
const FILL := 0.72
const PITCH_MIN := 5.0
const PITCH_MAX := 70.0

## Catalog id and biome shown.
var type_id := ""
var biome := "forest"
## Whether a model is shown (false if the type has none).
var has_model := false

var _viewport: SubViewport
var _camera: Camera3D
var _model: Node3D
var _center := Vector3.ZERO
var _radius := 1.0
var _yaw := -35.0
var _pitch := 22.0
var _zoom := 1.0
var _spinning := true
var _dragging := false


func _init() -> void:
	stretch = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	_viewport = SubViewport.new()
	_viewport.own_world_3d = true
	_viewport.transparent_bg = true
	_viewport.msaa_3d = Viewport.MSAA_4X
	_viewport.render_target_update_mode = SubViewport.UPDATE_WHEN_VISIBLE
	add_child(_viewport)
	var key := DirectionalLight3D.new()
	key.rotation_degrees = Vector3(-48, -70, 0)
	key.light_energy = 1.1
	key.shadow_enabled = true
	_viewport.add_child(key)
	var fill := DirectionalLight3D.new()
	fill.rotation_degrees = Vector3(-20, 150, 0)
	fill.light_energy = 0.45
	fill.light_color = Color(0.8, 0.86, 1.0)
	_viewport.add_child(fill)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_CLEAR_COLOR
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.72, 0.73, 0.76)
	environment.ambient_light_energy = 0.6
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	_viewport.add_child(world_environment)
	_camera = Camera3D.new()
	_camera.fov = 30.0
	_camera.current = true
	_viewport.add_child(_camera)


## Shows the model of `id` in `in_biome`; keeps the view angle when only
## the camouflage changes.
func show_model(id: String, in_biome := "forest") -> void:
	var same := id == type_id
	type_id = id
	biome = in_biome
	if _model != null:
		_viewport.remove_child(_model)
		_model.free()
		_model = null
	_model = ThumbnailRender.build_model(id, biome)
	has_model = _model != null
	if not has_model:
		return
	_viewport.add_child(_model)
	var bounds := ThumbnailRender.visible_bounds(_model)
	_center = bounds.get_center()
	_radius = maxf(bounds.size.length() * 0.5, 0.5)
	if not same:
		_yaw = -35.0
		_pitch = 22.0
		_zoom = 1.0
		_spinning = true
	_place_camera()


func _process(delta: float) -> void:
	if has_model and _spinning and not _dragging and is_visible_in_tree():
		_yaw += SPIN_DEG_PER_S * delta
		_place_camera()


func _gui_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT:
			_dragging = button.pressed
			if button.pressed:
				_spinning = false
		elif button.pressed and button.button_index == MOUSE_BUTTON_WHEEL_UP:
			_zoom = maxf(_zoom * 0.9, 0.45)
		elif button.pressed and button.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_zoom = minf(_zoom * 1.1, 1.8)
		_place_camera()
		accept_event()
		return
	var motion := event as InputEventMouseMotion
	if motion != null and _dragging:
		turn(motion.relative.x * 0.5, motion.relative.y * 0.35)
		accept_event()


## Turns the view by the angles in degrees (also used by the self-test).
func turn(yaw_deg: float, pitch_deg: float) -> void:
	_spinning = false
	_yaw -= yaw_deg
	_pitch = clampf(_pitch + pitch_deg, PITCH_MIN, PITCH_MAX)
	_place_camera()


## Camera yaw in degrees (self-test).
func yaw() -> float:
	return _yaw


func _place_camera() -> void:
	if not has_model:
		return
	var direction := Basis.from_euler(Vector3(deg_to_rad(-_pitch), deg_to_rad(_yaw + 180.0), 0.0)) * Vector3.BACK
	var distance := _radius / sin(deg_to_rad(_camera.fov * 0.5)) * FILL * _zoom
	# Not `look_at`: the view may be set up before it is in the scene tree.
	_camera.transform = Transform3D(Basis.looking_at(-direction, Vector3.UP), _center + direction * distance)
