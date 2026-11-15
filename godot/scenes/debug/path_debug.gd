extends Node3D
## Debug view of the simulation's pathfinding (presentation only).
##
## F3 toggles the path cost grid of the unit type of the first selected unit
## (red = blocked, green = cheap, yellow to purple = expensive); F4 toggles
## the remaining paths of all units (white, orange while a path is planned).
## `-- --path-debug` starts with both visible.
##
## The cost overlay is one mesh draped over the height field (at most
## MAX_VERTICES per axis) with the cost grid as a nearest-filtered texture.

## Upper bound of overlay vertices per axis.
const MAX_VERTICES := 256
## Overlay and lines float this high above the ground (meters).
const LIFT := 0.35
const PATH_COLOR := Color(1, 1, 1, 0.9)
const WAITING_COLOR := Color(1, 0.55, 0.1, 0.9)

var sim_host: SimHost
## Selection script; its first selected unit picks the cost profile.
var selection: Control

var _costs: MeshInstance3D
var _paths: MeshInstance3D
var _lines: ImmediateMesh
## Unit type shown by the cost overlay (-1: not built yet).
var _cost_type := -1


func _ready() -> void:
	_costs = MeshInstance3D.new()
	_costs.name = "PathCosts"
	_costs.visible = false
	_costs.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_costs)

	_lines = ImmediateMesh.new()
	_paths = MeshInstance3D.new()
	_paths.name = "Paths"
	_paths.mesh = _lines
	_paths.visible = false
	_paths.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var line_material := StandardMaterial3D.new()
	line_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	line_material.vertex_color_use_as_albedo = true
	line_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	line_material.no_depth_test = true
	_paths.material_override = line_material
	add_child(_paths)

	if "--path-debug" in OS.get_cmdline_user_args():
		show_costs(true)
		_paths.visible = true


func _unhandled_input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo:
		return
	match key.physical_keycode:
		KEY_F3:
			show_costs(not _costs.visible)
			get_viewport().set_input_as_handled()
		KEY_F4:
			_paths.visible = not _paths.visible
			get_viewport().set_input_as_handled()


func _process(_delta: float) -> void:
	if _costs.visible and _selected_type() != _cost_type:
		_build_costs(_selected_type())
	if _paths.visible:
		_draw_paths()


## Shows or hides the cost grid (built on first use and when the selected
## unit type changes).
func show_costs(visible_now: bool) -> void:
	_costs.visible = visible_now
	if visible_now and _selected_type() != _cost_type:
		_build_costs(_selected_type())


func paths_visible() -> bool:
	return _paths.visible


func costs_visible() -> bool:
	return _costs.visible


## Unit type of the first selected unit, else of the first unit.
func _selected_type() -> int:
	var types := sim_host.unit_types()
	if types.is_empty():
		return 0
	if selection != null and not selection.selected.is_empty():
		var index := sim_host.unit_ids().find(selection.selected[0])
		if index >= 0:
			return types[index]
	return types[0]


func _build_costs(unit_type: int) -> void:
	_cost_type = unit_type
	var cells := sim_host.path_grid_size()
	var pixels := sim_host.path_cost_image(unit_type)
	if cells.x <= 0 or pixels.size() != cells.x * cells.y * 4:
		_costs.mesh = null
		return
	var image := Image.create_from_data(cells.x, cells.y, false, Image.FORMAT_RGBA8, pixels)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	material.albedo_texture = ImageTexture.create_from_image(image)
	_costs.material_override = material

	# Grid vertices over the cell area, following the ground height.
	var origin := sim_host.path_grid_origin()
	var extent := Vector2(cells) * sim_host.path_cell_size()
	var steps := mini(MAX_VERTICES - 1, maxi(cells.x, cells.y))
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	for j in steps + 1:
		for i in steps + 1:
			var uv := Vector2(float(i) / steps, float(j) / steps)
			var p := origin + uv * extent
			vertices.append(Vector3(p.x, sim_host.height_at(p.x, p.y) + LIFT, p.y))
			uvs.append(uv)
	var indices := PackedInt32Array()
	for j in steps:
		for i in steps:
			var a := j * (steps + 1) + i
			var c := a + steps + 1
			indices.append_array([a, a + 1, c, a + 1, c + 1, c])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	_costs.mesh = mesh


## Line segments from every unit through its remaining waypoints, all in
## one surface.
func _draw_paths() -> void:
	_lines.clear_surfaces()
	var counts := sim_host.unit_path_counts()
	var points := sim_host.unit_path_points()
	var positions := sim_host.current_positions()
	var lift := Vector3(0, LIFT + 0.15, 0)
	if counts.is_empty():
		return
	_lines.surface_begin(Mesh.PRIMITIVE_LINES)
	var next := 0
	var any := false
	for i in mini(counts.size(), positions.size()):
		var count := counts[i]
		if count < 0:
			# Waiting for its path: a short orange stick.
			_lines.surface_set_color(WAITING_COLOR)
			_lines.surface_add_vertex(positions[i] + lift)
			_lines.surface_set_color(WAITING_COLOR)
			_lines.surface_add_vertex(positions[i] + lift * 12.0)
			any = true
			continue
		var from := positions[i]
		for k in count:
			_lines.surface_set_color(PATH_COLOR)
			_lines.surface_add_vertex(from + lift)
			_lines.surface_set_color(PATH_COLOR)
			_lines.surface_add_vertex(points[next + k] + lift)
			from = points[next + k]
			any = true
		next += count
	if not any:
		# An empty surface is an error; draw a degenerate segment instead.
		_lines.surface_add_vertex(Vector3.ZERO)
		_lines.surface_add_vertex(Vector3.ZERO)
	_lines.surface_end()
