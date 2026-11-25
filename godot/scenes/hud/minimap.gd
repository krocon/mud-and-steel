extends PanelContainer
## Minimap (prompts 031 and 095), bottom left, always shown.
##
## - North is always up; an "N" marks it on the frame. The map does not
##   turn with the camera: a click, a drag and the dots keep their places,
##   and the view trapezoid shows where the camera looks (wider and
##   farther the flatter the camera is tilted, prompt 096). Which way north
##   lies on the screen is shown by the camera control above the map
##   (`camera_gizmo.gd`), or, while that is switched off, by the needle of
##   the north button, which turns with the camera.
## - `column`, right of the map (placed by `hud.gd`): north (turns the
##   camera back, key `camera_reset_north`; only while the camera control
##   is switched off), zoom (back to the camera's starting distance) and
##   pause (the tactical pause of prompt 102; not in a network match,
##   where nothing pauses), then the camera views: overview (Shift + O),
##   close and flat (Shift + F, `close_view.gd`) and the jump to where the
##   fighting is (Shift + G, `action_spots.gd`).
## - Ground: `SimHost.minimap_image` (one pixel per 8 m vision cell:
##   terrain, hill shading, roads, forest, buildings), built once and all
##   drawn from the start; out of sight dimmed by the mask of
##   `fog_of_war.gd` (the same grid) with its tint and desaturation
##   (prompt 098), in one shader.
## - Units: one `MultiMeshInstance2D` whose buffer the bridge fills
##   (`minimap_dots`), colored by player; only units in the local
##   player's snapshot (own units, enemies in sight, wrecks are left out).
## - Supply points (prompt 107) as marks in the holder's color with a
##   shape of their own: neutral a hollow circle, own a filled circle,
##   allied a filled square, enemy a filled diamond; the capture progress
##   as an arc in the taker's color; a contested point blinks white. They
##   show what the player knows (`supply_view.gd known`: enemy points out
##   of sight as last seen).
## - Active mission objectives with a place as a gold ring with four ticks
##   (the color of the rings on the ground, `mission_hud.gd`).
## - The camera's view as a trapezoid (screen corners on the ground), combat
##   pings (expanding rings) and rings around selected units.
## - Left click / drag moves the camera there, right click moves the
##   selection there (Shift: waypoint).

const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const Settings := preload("res://scenes/menu/settings.gd")
## Longer side of the map on screen, pixels at HUD factor 1 (it shrinks on
## a small window, it does not grow with the font size).
const SIZE := 200.0
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const DOT_PX := 3.0
const PING_SECONDS := 1.6
## Player colors: own units first in `_colors()`.
const OWN_COLOR := Color(0.35, 0.95, 0.4)
## Allies (prompt 037) [provisional].
const ALLY_COLOR := Color(0.35, 0.6, 1.0)
const ENEMY_COLOR := Color(1.0, 0.3, 0.22)
const VIEW_COLOR := Color(1, 1, 1, 0.9)
const PING_COLOR := Color(1.0, 0.25, 0.2)
const SupplyView := preload("res://scenes/world/supply_view.gd")
const MissionHud := preload("res://scenes/hud/mission_hud.gd")
## Half size of a supply point mark, pixels at HUD factor 1.
const POINT_PX := 5.0
const OUTLINE := Color(0.05, 0.05, 0.05, 0.9)
const SHADER_CODE := """
shader_type canvas_item;
uniform sampler2D ground_map : filter_linear, source_color;
uniform sampler2D fog_map : filter_linear;
uniform bool use_fog = false;
// Map rectangle within the vision grid's texture (UV).
uniform vec2 uv_scale = vec2(1.0);
// Vision grid uv to uv in the padded fog mask (fog_of_war.fog_uv_transform).
uniform vec4 fog_uv = vec4(1.0, 1.0, 0.0, 0.0);
// The main view's layer (fog_of_war.gd): same tint and desaturation.
uniform vec3 dim_tint = vec3(0.6, 0.63, 0.7);
uniform float desaturation = 0.6;
void fragment() {
	vec2 uv = UV * uv_scale;
	vec3 ground = texture(ground_map, uv).rgb;
	vec3 color = ground;
	if (use_fog) {
		// The whole map is drawn; out of sight it is dimmed (prompt 098).
		float seen = smoothstep(0.3, 0.7, texture(fog_map, uv * fog_uv.xy + fog_uv.zw).r);
		vec3 gray = vec3(dot(ground, vec3(0.2126, 0.7152, 0.0722)));
		vec3 dimmed = mix(ground, gray, desaturation) * dim_tint;
		color = mix(dimmed, ground, seen);
	}
	COLOR = vec4(color, 1.0);
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
## Supply point marks drawn in the last frame (self-test): `at` (pixels),
## `relation`, `color`, `contested`.
var point_marks: Array[Dictionary] = []
## Mission objective marks drawn in the last frame (self-test).
var objective_marks := 0

## Buttons right of the map, a child of the HUD (`hud.gd` places it).
var column: VBoxContainer
var north_button: HudTile
var zoom_button: HudTile
var pause_button: HudTile
## Camera views (overview, close and flat) and the jump to the fighting.
var overview_button: HudTile
var close_button: HudTile
var action_button: HudTile

var _area: Control
var _overlay: Control
var _north: Label
var _start_distance := 0.0
var _fit := 0.0
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
	add_theme_stylebox_override(&"panel", HudTheme.box(HudTheme.PANEL, HudTheme.PANEL_BORDER, 5.0))
	mouse_filter = Control.MOUSE_FILTER_STOP
	_start_distance = main.camera_rig.target_distance / main.camera_rig.zoom_scale

	_area = Control.new()
	_area.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_area.clip_contents = true
	add_child(_area)
	_ground = ColorRect.new()
	_ground.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_material = ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = SHADER_CODE
	_material.shader = shader
	_ground.material = _material
	var tint: Color = FogOfWar.DIM_TINT
	_material.set_shader_parameter("dim_tint", Vector3(tint.r, tint.g, tint.b))
	_material.set_shader_parameter("desaturation", FogOfWar.DESATURATION)
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
	overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.draw.connect(_draw_overlay.bind(overlay))
	overlay.name = "Overlay"
	_area.add_child(overlay)
	_overlay = overlay
	# The north mark sits on the top edge of the frame.
	_north = Label.new()
	_north.name = "North"
	_north.text = "N"  # l10n-ignore
	_north.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_north.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_north.add_theme_font_override(&"font", HudTheme.serif())
	_north.add_theme_color_override(&"font_color", HudTheme.TEXT)
	_north.add_theme_stylebox_override(&"normal", HudTheme.box(Color(HudTheme.PANEL, 0.95), HudTheme.PANEL_BORDER, 1.0))
	_north.top_level = true
	add_child(_north)
	_build_column()
	apply_scale()


## The buttons right of the map.
func _build_column() -> void:
	column = VBoxContainer.new()
	column.name = "MinimapButtons"
	column.mouse_filter = Control.MOUSE_FILTER_STOP
	north_button = HudTile.new("north", HudTheme.SMALL_TILE_PX)
	north_button.name = "North"
	north_button.pressed.connect(reset_north)
	column.add_child(north_button)
	zoom_button = HudTile.new("zoom", HudTheme.SMALL_TILE_PX)
	zoom_button.name = "Zoom"
	zoom_button.pressed.connect(reset_zoom)
	column.add_child(zoom_button)
	pause_button = HudTile.new("pause", HudTheme.SMALL_TILE_PX)
	pause_button.name = "Pause"
	pause_button.visible = not sim_host.is_networked()
	# The tactical pause (prompt 102); Esc opens the pause menu.
	pause_button.pressed.connect(func() -> void:
		var pause: Node = main.get("tactical_pause")
		if pause != null:
			pause.toggle())
	column.add_child(pause_button)
	overview_button = HudTile.new("overview", HudTheme.SMALL_TILE_PX)
	overview_button.name = "Overview"
	overview_button.pressed.connect(func() -> void:
		var overview: Node = main.get("overview")
		if overview != null:
			overview.toggle())
	column.add_child(overview_button)
	close_button = HudTile.new("close_view", HudTheme.SMALL_TILE_PX)
	close_button.name = "CloseView"
	close_button.pressed.connect(func() -> void:
		var close: Node = main.get("close_view")
		if close != null:
			close.toggle())
	column.add_child(close_button)
	action_button = HudTile.new("action_jump", HudTheme.SMALL_TILE_PX)
	action_button.name = "ActionJump"
	action_button.pressed.connect(func() -> void:
		var hud: Node = main.get("hud")
		if hud != null:
			hud.action_spots.jump())
	column.add_child(action_button)
	_set_tooltips()


func _set_tooltips() -> void:
	north_button.tooltip_text = tr("HUD_TIP_NORTH") % Settings.action_key_name("camera_reset_north")
	zoom_button.tooltip_text = tr("HUD_TIP_ZOOM_RESET")
	pause_button.tooltip_text = tr("HUD_TIP_PAUSE")
	overview_button.tooltip_text = tr("HUD_TIP_OVERVIEW") % Settings.action_key_name("overview")
	close_button.tooltip_text = tr("HUD_TIP_CLOSE_VIEW") % Settings.action_key_name("camera_close_flat")
	action_button.tooltip_text = tr("HUD_TIP_ACTION_JUMP") % Settings.action_key_name("action_jump")


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and north_button != null:
		_set_tooltips()


## Shows the north button (`hud.gd`: while the camera control is off).
func show_north_button(show: bool) -> void:
	if north_button.visible != show:
		north_button.visible = show
		column.size = Vector2.ZERO


## Turns the camera back to north the shortest way (as the key does).
func reset_north() -> void:
	var rig: Node3D = main.camera_rig
	rig.target_yaw = roundf(rig.yaw / TAU) * TAU


## Back to the distance the camera started the match with.
func reset_zoom() -> void:
	main.camera_rig.target_distance = _start_distance * main.camera_rig.zoom_scale


## Sizes for the HUD factor (`hud.gd` calls it when the factor changed).
func apply_scale() -> void:
	var fit := minf(HudTheme.scale, 1.0)
	scale_px = SIZE * fit / maxf(map_size.x, map_size.y)
	var area_size := (map_size * scale_px).round()
	_area.custom_minimum_size = area_size
	_ground.size = area_size
	_overlay.size = area_size
	size = Vector2.ZERO
	_north.add_theme_font_size_override(&"font_size", HudTheme.font(12))
	_north.size = Vector2.ZERO
	column.add_theme_constant_override(&"separation", int(HudTheme.px(HudTheme.GAP)))
	for tile: HudTile in [north_button, zoom_button, pause_button, overview_button, close_button, action_button]:
		tile.apply_scale()
	column.size = Vector2.ZERO


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
		_material.set_shader_parameter("fog_uv", fog.fog_uv_transform())
	var buffer := sim_host.minimap_dots(map_origin, scale_px, DOT_PX, _colors())
	dot_count = buffer.size() / 12
	_dots.multimesh.instance_count = dot_count
	if dot_count > 0:
		_dots.multimesh.buffer = buffer
	for ping in _pings:
		ping.age += delta
	_pings = _pings.filter(func(p: Dictionary) -> bool: return p.age < PING_SECONDS)
	_update_view()
	_overlay.queue_redraw()
	_north.global_position = global_position + Vector2((size.x - _north.size.x) * 0.5, -_north.size.y * 0.5).round()
	north_button.turn = main.camera_rig.yaw
	# The views show whether they are on.
	var overview: Node = main.get("overview")
	overview_button.marked = overview != null and overview.active
	var close: Node = main.get("close_view")
	close_button.marked = close != null and close.active
	usec += Time.get_ticks_usec() - began


## Player colors indexed by player id: the local player green, allies
## blue (prompt 037), enemies red.
func _colors() -> PackedColorArray:
	var colors := PackedColorArray()
	var me := sim_host.local_player()
	var last := maxi(me + 1, 2)
	for p: Dictionary in sim_host.match_players():
		last = maxi(last, int(p["id"]) + 1)
	for player in last:
		match sim_host.relation(player):
			0:
				colors.append(RelationColors.own_or(OWN_COLOR))
			1:
				colors.append(RelationColors.ally_or(ALLY_COLOR))
			_:
				colors.append(RelationColors.enemy_or(ENEMY_COLOR))
	colors.append(RelationColors.enemy_or(ENEMY_COLOR))
	return colors


## A ping at the world point (combat from the event feed; marks of the
## allies, prompt 037).
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
	_draw_objectives(overlay)
	_draw_points(overlay)
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


## Supply points as marks (prompt 107).
func _draw_points(overlay: Control) -> void:
	point_marks.clear()
	var base_view: Node = main.get("base_view")
	if base_view == null or base_view.get("supply") == null:
		return
	var supply: SupplyView = base_view.supply
	var r := POINT_PX * minf(HudTheme.scale, 1.0) + 1.0
	var blink := fmod(Time.get_ticks_msec() / 1000.0 * SupplyView.BLINK_HZ, 1.0) < 0.5
	for k: Dictionary in supply.known:
		var at := to_minimap(k.position).round()
		var relation: String = k.relation
		var color := SupplyView.color_of(relation)
		var outline := Color.WHITE if k.contested and blink else OUTLINE
		match relation:
			"own":
				overlay.draw_circle(at, r, color)
				overlay.draw_arc(at, r, 0.0, TAU, 16, outline, 1.5)
			"ally":
				var box := Rect2(at - Vector2(r, r) * 0.85, Vector2(r, r) * 1.7)
				overlay.draw_rect(box, color)
				overlay.draw_rect(box, outline, false, 1.5)
			"enemy":
				var d := PackedVector2Array([at + Vector2(0, -r * 1.2), at + Vector2(r * 1.2, 0), at + Vector2(0, r * 1.2), at + Vector2(-r * 1.2, 0)])
				overlay.draw_colored_polygon(d, color)
				d.append(d[0])
				overlay.draw_polyline(d, outline, 1.5)
			_:
				overlay.draw_arc(at, r, 0.0, TAU, 16, OUTLINE, 3.5)
				overlay.draw_arc(at, r, 0.0, TAU, 16, color, 2.0)
		var taker: int = k.taker
		if taker >= 0 and float(k.progress) > 0.0:
			var tc := SupplyView.color_of(supply.relation_of(taker))
			overlay.draw_arc(at, r + 3.0, -PI * 0.5, -PI * 0.5 + TAU * float(k.progress), 20, tc, 2.0)
		point_marks.append({"at": at, "relation": relation, "color": color, "contested": k.contested})


## Active mission objectives with a place: a gold ring with four ticks.
func _draw_objectives(overlay: Control) -> void:
	objective_marks = 0
	var mission: Node = main.get("mission_hud")
	if mission == null or not mission.visible:
		return
	var info: Dictionary = mission.info
	for o: Dictionary in info.get("objectives", []):
		if o["state"] != "active" or o["position"] == null:
			continue
		var at := to_minimap(o["position"]).round()
		var r := POINT_PX * minf(HudTheme.scale, 1.0) + 2.0
		overlay.draw_arc(at, r, 0.0, TAU, 20, OUTLINE, 4.0)
		overlay.draw_arc(at, r, 0.0, TAU, 20, MissionHud.RING_COLOR, 2.0)
		for dir: Vector2 in [Vector2.UP, Vector2.DOWN, Vector2.LEFT, Vector2.RIGHT]:
			overlay.draw_line(at + dir * (r + 1.0), at + dir * (r + 4.0), MissionHud.RING_COLOR, 2.0)
		objective_marks += 1


func _gui_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	var mouse := event as InputEventMouse
	if mouse == null:
		return
	var local := mouse.global_position - _area.global_position
	if button != null:
		if button.button_index == MOUSE_BUTTON_LEFT and button.pressed and button.alt_pressed:
			# Alt + left click marks the spot for the allies (prompt 037).
			var at := to_world(local)
			sim_host.ping(at.x, at.z)
			accept_event()
		elif button.button_index == MOUSE_BUTTON_LEFT:
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
