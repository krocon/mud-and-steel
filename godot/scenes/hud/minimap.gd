extends PanelContainer
## Minimap (prompt 031), bottom right.
##
## - Ground: `SimHost.minimap_image` (one pixel per 8 m vision cell:
##   terrain, hill shading, roads, forest, buildings), built once, darkened
##   by the fog texture of `fog_of_war.gd` (the same grid) in one shader.
## - Units: one `MultiMeshInstance2D` whose buffer the bridge fills
##   (`minimap_dots`), colored by player; only units in the local
##   player's snapshot (own units, enemies in sight, wrecks are left out).
## - The camera's view as a trapezoid (screen corners on the ground), combat
##   pings (expanding rings) and rings around selected units.
## - Left click / drag moves the camera there, right click moves the
##   selection there (Shift: waypoint).

const GroundPick := preload("res://scenes/camera/ground_pick.gd")
## Longer side of the map on screen, pixels.
const SIZE := 220.0
const DOT_PX := 3.0
const PING_SECONDS := 1.6
## Player colors: own units first in `_colors()`.
const OWN_COLOR := Color(0.35, 0.95, 0.4)
const ENEMY_COLOR := Color(1.0, 0.3, 0.22)
const VIEW_COLOR := Color(1, 1, 1, 0.9)
const PING_COLOR := Color(1.0, 0.25, 0.2)
const SHADER_CODE := """
shader_type canvas_item;
uniform sampler2D ground_map : filter_linear, source_color;
uniform sampler2D fog_map : filter_linear;
uniform bool use_fog = false;
// Map rectangle within the vision grid's texture (UV).
uniform vec2 uv_scale = vec2(1.0);
void fragment() {
	vec2 uv = UV * uv_scale;
	vec3 ground = texture(ground_map, uv).rgb;
	float light = 1.0;
	if (use_fog) {
		// 0 never seen, 0.5 explored, 1 visible.
		float state = texture(fog_map, uv).r;
		light = state < 0.25 ? mix(0.12, 0.45, state * 4.0) : mix(0.45, 1.0, (state - 0.5) * 2.0);
		light = clamp(light, 0.12, 1.0);
	}
	COLOR = vec4(ground * light, 1.0);
}
"""

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` and `_draw` in microseconds (hud.gd).
var usec := 0
## Map origin (north-west corner, x/z) and size in meters.
var map_origin := Vector2.ZERO
var map_size := Vector2.ONE
## Pixels per meter.
var scale_px := 1.0
## Dots drawn in the last update (self-test).
var dot_count := 0
## Pings so far (self-test).
var ping_count := 0

var _area: Control
var _ground: ColorRect
var _material: ShaderMaterial
var _dots: MultiMeshInstance2D
var _pings: Array[Dictionary] = []
var _dragging := false
var _view_polygon := PackedVector2Array()


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "Minimap"
	map_size = sim_host.map_size()
	if map_size.x <= 0.0 or map_size.y <= 0.0:
		map_size = Vector2.ONE
	map_origin = -map_size * 0.5
	scale_px = SIZE / maxf(map_size.x, map_size.y)
	var area_size := map_size * scale_px
	set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	grow_horizontal = Control.GROW_DIRECTION_BEGIN
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	offset_right = -8
	offset_bottom = -8
	offset_left = -8 - area_size.x - 12
	offset_top = -8 - area_size.y - 12
	mouse_filter = Control.MOUSE_FILTER_STOP

	_area = Control.new()
	_area.custom_minimum_size = area_size
	_area.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_area.clip_contents = true
	add_child(_area)
	_ground = ColorRect.new()
	_ground.size = area_size
	_ground.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_material = ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = SHADER_CODE
	_material.shader = shader
	_ground.material = _material
	_area.add_child(_ground)
	_build_ground()

	_dots = MultiMeshInstance2D.new()
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_2D
	multimesh.use_colors = true
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	multimesh.mesh = quad
	_dots.multimesh = multimesh
	_area.add_child(_dots)
	# Overlay for the view, pings and selection rings, above the dots.
	var overlay := Control.new()
	overlay.size = area_size
	overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.draw.connect(_draw_overlay.bind(overlay))
	overlay.name = "Overlay"
	_area.add_child(overlay)


## The ground image: one pixel per vision cell, scaled to the map.
func _build_ground() -> void:
	var cells := sim_host.vision_grid_size()
	var bytes := sim_host.minimap_image()
	if cells.x <= 0 or cells.y <= 0 or bytes.size() != cells.x * cells.y * 3:
		_ground.color = Color(0.3, 0.35, 0.25)
		_ground.material = null
		return
	var image := Image.create_from_data(cells.x, cells.y, false, Image.FORMAT_RGB8, bytes)
	_material.set_shader_parameter("ground_map", ImageTexture.create_from_image(image))
	var cell := sim_host.vision_cell_size()
	_material.set_shader_parameter("uv_scale", map_size / (Vector2(cells) * cell))


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	var fog: Node = main.fog_of_war
	var texture: Texture2D = fog.fog_texture() if fog != null else null
	var fogged: bool = texture != null and fog.enabled and not sim_host.is_reveal_all()
	_material.set_shader_parameter("use_fog", fogged)
	if texture != null:
		_material.set_shader_parameter("fog_map", texture)
	var buffer := sim_host.minimap_dots(map_origin, scale_px, DOT_PX, _colors())
	dot_count = buffer.size() / 12
	_dots.multimesh.instance_count = dot_count
	if dot_count > 0:
		_dots.multimesh.buffer = buffer
	for ping in _pings:
		ping.age += delta
	_pings = _pings.filter(func(p: Dictionary) -> bool: return p.age < PING_SECONDS)
	_update_view()
	(_area.get_node("Overlay") as Control).queue_redraw()
	usec += Time.get_ticks_usec() - began


## Player colors indexed by player id: the local player green, the others
## red.
func _colors() -> PackedColorArray:
	var colors := PackedColorArray()
	var me := sim_host.local_player()
	for player in maxi(me + 1, 2):
		colors.append(OWN_COLOR if player == me else ENEMY_COLOR)
	colors.append(ENEMY_COLOR)
	return colors


## A combat ping at the world point (from the event feed).
func add_ping(point: Vector3) -> void:
	_pings.append({"at": to_minimap(point), "age": 0.0})
	ping_count += 1


## Map point (x, z) to minimap pixels.
func to_minimap(point: Vector3) -> Vector2:
	return (Vector2(point.x, point.z) - map_origin) * scale_px


## Minimap pixels to the ground point (with the simulation's height).
func to_world(pixel: Vector2) -> Vector3:
	var p := map_origin + pixel / scale_px
	p = p.clamp(map_origin, map_origin + map_size)
	return Vector3(p.x, sim_host.height_at(p.x, p.y), p.y)


## The camera's view: the screen corners projected onto the ground (the
## plane at the focus height where a ray misses it).
func _update_view() -> void:
	var camera: Camera3D = main.camera
	var screen := get_viewport().get_visible_rect().size
	var focus_y: float = main.camera_rig.focus.y
	_view_polygon.clear()
	for corner in [Vector2.ZERO, Vector2(screen.x, 0), screen, Vector2(0, screen.y)]:
		var hit: Variant = GroundPick.from_screen(camera, corner)
		var point: Vector3
		if hit != null:
			point = hit
		else:
			var origin := camera.project_ray_origin(corner)
			var direction := camera.project_ray_normal(corner)
			var t := 4000.0 if direction.y >= -0.001 else (focus_y - origin.y) / direction.y
			point = origin + direction * minf(t, 4000.0)
		_view_polygon.append(to_minimap(point))
	if not _view_polygon.is_empty():
		_view_polygon.append(_view_polygon[0])


func _draw_overlay(overlay: Control) -> void:
	var began := Time.get_ticks_usec()
	if _view_polygon.size() > 1:
		overlay.draw_polyline(_view_polygon, VIEW_COLOR, 1.5)
	for ping in _pings:
		var t: float = ping.age / PING_SECONDS
		overlay.draw_arc(ping.at, 4.0 + 14.0 * t, 0.0, TAU, 24, Color(PING_COLOR, 1.0 - t), 2.0)
	var selection: Control = main.selection
	if selection.selected.size() <= 64:
		var ids := sim_host.unit_ids()
		var positions := sim_host.current_positions()
		for id: int in selection.selected:
			var i := ids.find(id)
			if i >= 0:
				overlay.draw_arc(to_minimap(positions[i]), 3.5, 0.0, TAU, 10, Color(1, 1, 1, 0.9), 1.0)
	usec += Time.get_ticks_usec() - began


func _gui_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	var mouse := event as InputEventMouse
	if mouse == null:
		return
	var local := mouse.global_position - _area.global_position
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT:
			_dragging = button.pressed
			if button.pressed:
				look_at_pixel(local)
			accept_event()
		elif button.button_index == MOUSE_BUTTON_RIGHT and button.pressed:
			order_at_pixel(local, button.shift_pressed)
			accept_event()
		return
	var motion := event as InputEventMouseMotion
	if motion != null and _dragging:
		look_at_pixel(local)
		accept_event()


## Moves the camera to the map point under the minimap pixel.
func look_at_pixel(pixel: Vector2) -> void:
	var rig: Node3D = main.camera_rig
	rig.snap_to(to_world(pixel), rig.distance, rig.yaw)
	if main.selection.follow:
		main.selection.set_follow(false)


## Moves the selection to the map point (Shift: as a waypoint).
func order_at_pixel(pixel: Vector2, queue := false) -> void:
	main.selection.order_move_to(to_world(pixel), queue)
