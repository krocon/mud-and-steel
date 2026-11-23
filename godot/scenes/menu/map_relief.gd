extends SubViewportContainer
## Small 3D relief of a map for the map picker (prompt 201): a mesh from the
## coarse height grid of `SimHost.map_preview_build` (about 128 vertices
## along the longer edge) with the top view as its texture (water shows in
## its color; no objects, no trees), heights exaggerated by `exaggeration`.
## Turns slowly by itself; left drag turns it, the wheel zooms, like
## `unit_model_view.gd`. Presentation only.

const SPIN_DEG_PER_S := 8.0
const PITCH_MIN := 15.0
const PITCH_MAX := 80.0
## The map's longer edge in scene units.
const EXTENT := 2.0

## Height factor (1 = true heights).
var exaggeration := 2.0:
	set(value):
		exaggeration = value
		if _mesh_instance != null:
			_mesh_instance.scale = Vector3(1.0, exaggeration, 1.0)

var _viewport: SubViewport
var _camera: Camera3D
var _mesh_instance: MeshInstance3D
var _yaw := 20.0
var _pitch := 42.0
var _zoom := 1.0
var _spinning := true
var _dragging := false
## The entry the mesh was built for.
var _built_for: Dictionary = {}


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
	key.rotation_degrees = Vector3(-40, -135, 0)
	key.light_energy = 1.0
	_viewport.add_child(key)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_CLEAR_COLOR
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.8, 0.8, 0.82)
	environment.ambient_light_energy = 0.55
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	_viewport.add_child(world_environment)
	_camera = Camera3D.new()
	_camera.fov = 35.0
	_camera.current = true
	_viewport.add_child(_camera)
	_mesh_instance = MeshInstance3D.new()
	_viewport.add_child(_mesh_instance)
	_place_camera()


## Builds the relief of a preview entry (`map_preview.gd`).
func show_relief(entry: Dictionary) -> void:
	if entry == _built_for:
		return
	_built_for = entry
	var columns: int = entry.get("relief_columns", 0)
	var rows: int = entry.get("relief_rows", 0)
	var heights: PackedFloat32Array = entry.get("relief", PackedFloat32Array())
	var size_m: Vector2 = entry.get("size_m", Vector2.ONE)
	if columns < 2 or rows < 2 or heights.size() != columns * rows:
		_mesh_instance.mesh = null
		return
	var k := EXTENT / maxf(size_m.x, size_m.y)
	var lowest := heights[0]
	for h in heights:
		lowest = minf(lowest, h)
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	for r in rows:
		for c in columns:
			var u := float(c) / (columns - 1)
			var v := float(r) / (rows - 1)
			tool.set_uv(Vector2(u, v))
			tool.add_vertex(Vector3((u - 0.5) * size_m.x * k, (heights[r * columns + c] - lowest) * k, (v - 0.5) * size_m.y * k))
	for r in rows - 1:
		for c in columns - 1:
			var i := r * columns + c
			for index in [i, i + 1, i + columns, i + 1, i + columns + 1, i + columns]:
				tool.add_index(index)
	tool.generate_normals()
	var material := StandardMaterial3D.new()
	material.albedo_texture = entry.get("texture")
	material.roughness = 1.0
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	_mesh_instance.mesh = tool.commit()
	_mesh_instance.material_override = material
	_mesh_instance.scale = Vector3(1.0, exaggeration, 1.0)


func _process(delta: float) -> void:
	if _spinning and not _dragging and is_visible_in_tree():
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
			_zoom = maxf(_zoom * 0.9, 0.5)
		elif button.pressed and button.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_zoom = minf(_zoom * 1.1, 1.6)
		_place_camera()
		accept_event()
		return
	var motion := event as InputEventMouseMotion
	if motion != null and _dragging:
		_yaw -= motion.relative.x * 0.5
		_pitch = clampf(_pitch + motion.relative.y * 0.35, PITCH_MIN, PITCH_MAX)
		_place_camera()
		accept_event()


func _place_camera() -> void:
	var direction := Basis.from_euler(Vector3(deg_to_rad(-_pitch), deg_to_rad(_yaw), 0.0)) * Vector3.BACK
	var distance := EXTENT * 1.45 * _zoom
	_camera.transform = Transform3D(Basis.looking_at(-direction, Vector3.UP), direction * distance)
