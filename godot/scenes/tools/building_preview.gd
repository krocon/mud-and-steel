extends Node
## Preview sheet of an environment building (prompt 185): the model as the
## game shows it (`env.gdshader` patterns, `EnvLook` biome look), laid out
## on one PNG next to an optional reference picture, so a blueprint can be
## compared and improved in quick loops. A development tool:
##
##     just building-preview env_house_town --ref docs/.vorlagen/x/referenz.webp
##     ... -- --id=<id> [--ref=<picture>] [--biome=forest|desert|winter] [--out=<dir>]
##            [--surface=plaster=ochre_red,spall=0.5,roof=mission] (prompt 186)
##
## (the recipe builds the model with Blender and imports it first). Needs a
## window: rendering does not work headless. The sheet (`SHEET` pixels):
##
## - header: id, names, size from the walls and roof of the model and from
##   the catalog, triangles per variant against the budget, surfaces;
## - a large oblique view of `Intact` (perspective, sun and shadows) and the
##   reference picture beside it (`--ref`, else an empty field);
## - the four orthographic elevations (+X, +Y, -X, -Y) and the top view, all
##   at one scale, with dimension lines for length, width, eaves and ridge;
## - the damage variants side by side, close views of a wall and the roof,
##   and the view at the game's start zoom (RTS camera distance and pitch).
##
## Output: `<out>/<id>_<biome>.png` (default `build/preview/<id>/`). Prints
## the time it took.

const SHEET := Vector2i(2400, 1800)
const MARGIN := 16
const HEADER_H := 100
const ROW_A_H := 700
const ROW_B_H := 470
const BACKGROUND := Color(0.93, 0.93, 0.92)
const PANEL := Color(0.97, 0.97, 0.96)
const INK := Color(0.12, 0.12, 0.13)
const DIM_INK := Color(0.25, 0.25, 0.3)
## Sky behind the perspective views and the ground they stand on.
const SKY := Color(0.74, 0.77, 0.82)
const GROUND := Color(0.47, 0.45, 0.40)
## Close views: meters across the picture.
const CLOSE_M := 4.5
## Frames to wait after moving the camera (shadows, TAA-free settle).
const SETTLE_FRAMES := 3
## Triangle budgets per variant by catalog category, as in
## `tools/blender/kit/env.py` (`BUDGETS`, `BUDGET_OVERRIDES`).
const BUDGETS := {
	"env_tree": 2000, "env_bush": 500, "env_grass": 50, "env_rock": 300,
	"env_building": 3000, "env_bridge": 3000, "env_fence": 2000, "env_pole": 800, "env_prop": 800,
}
const BUDGET_OVERRIDES := {
	"env_church": 5000, "env_mosque": 5000, "env_police_post": 5000,
	"env_block_desert": 6000, "env_block_desert_long": 6000, "env_block_town": 6000,
	"env_dump_ammo": 1500, "env_dump_fuel": 1500, "env_rock_outcrop": 1500, "env_rock_massif": 3000,
	"env_house_med_row_corner": 4500, "env_town_hall_med": 6000,
}
const ThumbnailRender := preload("res://scenes/tools/thumbnail_render.gd")

var id := ""
var ref_path := ""
var biome := "forest"
var out_dir := ""
## Repository folder (`--root=`): relative `--ref` and `--out` paths start there.
var root := ""

var _viewport: SubViewport
var _camera: Camera3D
var _ground: MeshInstance3D
var _environment: Environment
var _model: Node3D
## Sizes of the model in meters: walls (x length, z width), ground, eaves,
## ridge; whole bounds.
var _walls := AABB()
var _bounds := AABB()
var _eaves := 0.0
var _ridge := 0.0
var _sheet: SubViewport
var _canvas: Control
## `--surface=plaster=ochre_red,spall=0.5,roof=mission,...` (prompt 186):
## a `[surface]` table tried on the model without changing its blueprint.
var surface := {}
## `--check` (self-test): every placed picture's rectangle on the sheet.
var check := false
var _fields: Array[Rect2i] = []


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--id="):
			id = arg.trim_prefix("--id=")
		elif arg.begins_with("--ref="):
			ref_path = arg.trim_prefix("--ref=")
		elif arg.begins_with("--biome="):
			biome = arg.trim_prefix("--biome=")
		elif arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--surface="):
			for pair in arg.trim_prefix("--surface=").split(",", false):
				var kv := pair.split("=")
				if kv.size() == 2:
					surface[kv[0]] = float(kv[1]) if kv[1].is_valid_float() else kv[1]
		elif arg == "--check":
			check = true
		elif arg.begins_with("--root="):
			root = arg.trim_prefix("--root=")
	if root.is_empty():
		root = ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	if out_dir.is_empty():
		out_dir = "build/preview".path_join(id)
	if out_dir.is_relative_path():
		out_dir = root.path_join(out_dir)
	if not ref_path.is_empty() and ref_path.is_relative_path():
		ref_path = root.path_join(ref_path)
	EnvLook.still_trees()
	_run.call_deferred()


func _run() -> void:
	var began := Time.get_ticks_msec()
	if id.is_empty():
		printerr("building preview: --id=<catalog id> missing")
		get_tree().quit(1)
		return
	_model = ThumbnailRender.build_model(id, biome)
	if _model == null or not EnvLook.is_env_model(_model):
		printerr("building preview: no environment model for %s (run just env %s)" % [id, id])
		get_tree().quit(1)
		return
	if not surface.is_empty():
		var uniforms := EnvLook.surface_from(surface)
		for mesh in UnitCamo.meshes(_model):
			EnvLook.apply_surface(mesh, uniforms)
	_setup_world()
	_viewport.add_child(_model)
	var variants := EnvLook.variants(_model)
	var first: String = variants[0].name if not variants.is_empty() else ""
	if not first.is_empty():
		EnvLook.show_variant(_model, first)
	_measure(first)
	_setup_sheet()
	_header(variants)
	var a_w := (SHEET.x - 3 * MARGIN) / 2
	var row_a := HEADER_H + MARGIN
	# Row A: oblique view and the reference picture.
	_place(await _perspective(Vector2i(a_w, ROW_A_H), -35.0, 28.0, 1.0), Vector2(MARGIN, row_a),
		"Oblique (%s, %s)" % [first, biome])
	_reference(Vector2(2 * MARGIN + a_w, row_a), Vector2(a_w, ROW_A_H))
	# Row B: elevations and top view at one scale.
	var row_b := row_a + ROW_A_H + MARGIN + 24
	await _elevations(row_b)
	# Row C: damage variants, close views, game view.
	var row_c := row_b + ROW_B_H + MARGIN + 24
	var c_h := SHEET.y - row_c - MARGIN
	var x := float(MARGIN)
	var damage_w := 400
	for v: Dictionary in variants.slice(0, 3):
		EnvLook.show_variant(_model, v.name)
		_place(await _perspective(Vector2i(damage_w, c_h), -35.0, 28.0, 1.0), Vector2(x, row_c), str(v.name))
		x += damage_w + MARGIN
	if not first.is_empty():
		EnvLook.show_variant(_model, first)
	var close_w := 360
	_place(await _close_wall(Vector2i(close_w, c_h)), Vector2(x, row_c), "Wall, close (%.1f m wide)" % CLOSE_M)
	x += close_w + MARGIN
	_place(await _close_roof(Vector2i(close_w, c_h)), Vector2(x, row_c), "Roof, close (%.1f m wide)" % CLOSE_M)
	x += close_w + MARGIN
	var game_w := int(SHEET.x - MARGIN - x)
	var game := await _game_view(Vector2i(game_w, c_h))
	_place(game[0], Vector2(x, row_c), "Game view (start zoom, %d m)" % int(game[1]))
	# Save the sheet.
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var image := _sheet.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var file := out_dir.path_join("%s_%s.png" % [id, biome])
	if image.save_png(file) != OK:
		printerr("building preview: cannot write %s" % file)
		get_tree().quit(1)
		return
	print("building preview: %s (%.1f s)" % [file, (Time.get_ticks_msec() - began) / 1000.0])
	if check:
		var failed := _check(image)
		print("building preview self-test %s: %d fields%s" % ["PASSED" if failed.is_empty() else "FAILED",
			_fields.size(), "" if failed.is_empty() else ", empty: " + ", ".join(failed)])
		get_tree().quit(0 if failed.is_empty() else 1)
		return
	get_tree().quit()


## Names of the fields that are empty (one flat color) or a sheet of the
## wrong size.
func _check(image: Image) -> Array[String]:
	var failed: Array[String] = []
	if image.get_size() != SHEET:
		failed.append("sheet size %s" % image.get_size())
	for k in _fields.size():
		var region := image.get_region(_fields[k])
		region.resize(32, 32, Image.INTERPOLATE_NEAREST)
		var low := 1.0
		var high := 0.0
		for y in 32:
			for x in 32:
				var v := region.get_pixel(x, y).get_luminance()
				low = minf(low, v)
				high = maxf(high, v)
		if high - low < 0.08:
			failed.append("field %d at %s" % [k, _fields[k].position])
	if _fields.size() < 12:
		failed.append("only %d fields" % _fields.size())
	return failed


func _setup_world() -> void:
	_viewport = SubViewport.new()
	_viewport.own_world_3d = true
	_viewport.msaa_3d = Viewport.MSAA_8X
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	_viewport.size = Vector2i(512, 512)
	add_child(_viewport)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-42, -60, 0)
	sun.light_energy = 1.25
	sun.light_color = Color(1.0, 0.97, 0.9)
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 200.0
	_viewport.add_child(sun)
	var fill := DirectionalLight3D.new()
	fill.rotation_degrees = Vector3(-20, 150, 0)
	fill.light_energy = 0.35
	fill.light_color = Color(0.8, 0.86, 1.0)
	_viewport.add_child(fill)
	_environment = Environment.new()
	_environment.background_mode = Environment.BG_COLOR
	_environment.background_color = SKY
	_environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	_environment.ambient_light_color = Color(0.72, 0.74, 0.78)
	_environment.ambient_light_energy = 0.55
	_environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world := WorldEnvironment.new()
	world.environment = _environment
	_viewport.add_child(world)
	_ground = MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(400, 400)
	var ground_material := StandardMaterial3D.new()
	ground_material.albedo_color = GROUND
	ground_material.roughness = 1.0
	plane.material = ground_material
	_ground.mesh = plane
	_ground.position.y = -0.02
	_viewport.add_child(_ground)
	_camera = Camera3D.new()
	_camera.current = true
	_viewport.add_child(_camera)


## Walls (`<variant>_Walls`), roof (`_Roof`: lowest point = eaves, highest
## = ridge; on flat roofs with parapets of several heights these are the
## lowest and the highest roof part) and whole bounds of a variant.
func _measure(variant: String) -> void:
	var node := EnvLook.variant_node(_model, variant) if not variant.is_empty() else _model
	_bounds = ThumbnailRender.visible_bounds(node if node != null else _model)
	_walls = _bounds
	_eaves = _bounds.end.y
	_ridge = _bounds.end.y
	var walls := _model.find_child(variant + "_Walls", true, false) as VisualInstance3D
	if walls != null:
		_walls = walls.global_transform * walls.get_aabb()
	var roof := _model.find_child(variant + "_Roof", true, false) as VisualInstance3D
	if roof != null:
		var box := roof.global_transform * roof.get_aabb()
		_eaves = box.position.y
		_ridge = box.end.y


func _setup_sheet() -> void:
	_sheet = SubViewport.new()
	_sheet.size = SHEET
	_sheet.transparent_bg = false
	_sheet.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(_sheet)
	var back := ColorRect.new()
	back.color = BACKGROUND
	back.size = SHEET
	_sheet.add_child(back)
	_canvas = Control.new()
	_canvas.size = SHEET
	_sheet.add_child(_canvas)


func _label(text: String, at: Vector2, size := 18, color := INK) -> Label:
	var label := Label.new()
	label.text = text
	label.position = at
	label.add_theme_font_size_override(&"font_size", size)
	label.add_theme_color_override(&"font_color", color)
	_canvas.add_child(label)
	return label


func _header(variants: Array) -> void:
	var extras := EnvLook.root_extras(_model)
	var name_en := str(extras.get("cat_name_en", ""))
	var name_de := str(extras.get("cat_name_de", ""))
	_label("%s  ·  %s  ·  %s" % [id, name_en, name_de], Vector2(MARGIN, 8), 26)
	var category := str(extras.get("cat_category", ""))
	var catalog := ""
	if extras.has("cat_length_mm"):
		catalog = "   catalog: %.1f × %.1f m, height %.1f m" % [float(extras.get("cat_length_mm", 0)) / 1000.0,
			float(extras.get("cat_width_mm", 0)) / 1000.0, float(extras.get("cat_height_mm", 0)) / 1000.0]
	_label("Model walls: %.1f m (x) × %.1f m (z), eaves %.1f m, ridge %.1f m, overall %.1f × %.1f × %.1f m%s" % [
		_walls.size.x, _walls.size.z, _eaves - _bounds.position.y, _ridge - _bounds.position.y,
		_bounds.size.x, _bounds.size.z, _bounds.size.y, catalog], Vector2(MARGIN, 40), 17)
	var budget: int = BUDGET_OVERRIDES.get(id, BUDGETS.get(category, 2000))
	var parts: Array[String] = []
	var surfaces := 0
	for v: Dictionary in variants:
		var node := EnvLook.variant_node(_model, v.name)
		var counts := _triangles(node)
		surfaces = maxi(surfaces, counts.y)
		parts.append("%s %d%s" % [v.name, counts.x, " (!)" if counts.x > budget else ""])
	var tried := "" if surface.is_empty() else "   ·   tried surface: %s" % JSON.stringify(surface)
	_label("Triangles per variant (budget %d): %s   ·   surfaces: up to %d   ·   biome %s%s" % [
		budget, ", ".join(parts), surfaces, biome, tried], Vector2(MARGIN, 64), 17)


## Triangles and surfaces of every mesh below `node`.
static func _triangles(node: Node) -> Vector2i:
	var result := Vector2i.ZERO
	if node == null:
		return result
	var meshes: Array[Node] = node.find_children("*", "MeshInstance3D", true, false)
	if node is MeshInstance3D:
		meshes.append(node)
	for m: Node in meshes:
		var mesh := (m as MeshInstance3D).mesh as ArrayMesh
		if mesh == null:
			continue
		for s in mesh.get_surface_count():
			result.y += 1
			var indices: int = mesh.surface_get_array_index_len(s)
			result.x += (indices if indices > 0 else mesh.surface_get_array_len(s)) / 3
	return result


## Draws `image` at `at` with a caption above it.
func _place(image: Image, at: Vector2, caption: String) -> void:
	var rect := TextureRect.new()
	rect.texture = ImageTexture.create_from_image(image)
	rect.position = at
	_canvas.add_child(rect)
	_fields.append(Rect2i(Vector2i(at), image.get_size()))
	if not caption.is_empty():
		_label(caption, at - Vector2(0, 24), 16, DIM_INK)


func _reference(at: Vector2, size: Vector2) -> void:
	var frame := ColorRect.new()
	frame.color = PANEL
	frame.position = at
	frame.size = size
	_canvas.add_child(frame)
	if ref_path.is_empty():
		_label("No reference picture (--ref=<file>)", at + Vector2(20, 20), 18, DIM_INK)
		return
	var picture := Image.load_from_file(ref_path)
	if picture == null or picture.is_empty():
		_label("Reference picture not readable: %s" % ref_path, at + Vector2(20, 20), 18, DIM_INK)
		return
	var scale := minf(size.x / picture.get_width(), size.y / picture.get_height())
	var fitted := Vector2i((Vector2(picture.get_size()) * scale).round())
	picture.resize(fitted.x, fitted.y, Image.INTERPOLATE_LANCZOS)
	_place(picture, at + (size - Vector2(fitted)) * 0.5, "")
	_label("Reference: %s" % ref_path.get_file(), at - Vector2(0, 24), 16, DIM_INK)


## Renders the viewport at `size` and returns the picture.
func _render(size: Vector2i) -> Image:
	_viewport.size = size
	for i in SETTLE_FRAMES:
		await RenderingServer.frame_post_draw
	return _viewport.get_texture().get_image()


## Perspective view from `yaw_deg` (0 = from +Z) and `pitch_deg` up,
## framed to the bounds (`zoom` < 1 is closer).
func _perspective(size: Vector2i, yaw_deg: float, pitch_deg: float, zoom: float) -> Image:
	_ground.visible = true
	_environment.background_color = SKY
	_camera.projection = Camera3D.PROJECTION_PERSPECTIVE
	_camera.fov = 30.0
	var center := _bounds.get_center()
	var radius := _bounds.size.length() * 0.5
	var direction := Basis(Vector3.UP, deg_to_rad(yaw_deg)) * Basis(Vector3.RIGHT, -deg_to_rad(pitch_deg)) * Vector3.BACK
	var distance := radius / tan(deg_to_rad(_camera.fov * 0.5)) * 1.08 * zoom
	_camera.position = center + direction * distance
	_camera.look_at(center, Vector3.UP)
	_camera.near = 0.5
	_camera.far = distance * 3.0
	return await _render(size)


## Orthographic view along `-direction` at `ppm` pixels per meter.
func _ortho(size: Vector2i, direction: Vector3, up: Vector3, ppm: float, center: Vector3) -> Image:
	_ground.visible = false
	_environment.background_color = PANEL
	_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	_camera.keep_aspect = Camera3D.KEEP_HEIGHT
	_camera.size = size.y / ppm
	_camera.position = center + direction * 100.0
	_camera.look_at(center, up)
	_camera.near = 1.0
	_camera.far = 300.0
	return await _render(size)


## The four elevations and the top view at one scale, with dimension lines.
func _elevations(top: float) -> void:
	var panel := Vector2i((SHEET.x - 6 * MARGIN) / 5, ROW_B_H - 60)
	var length := _bounds.size.x
	var width := _bounds.size.z
	var height := _bounds.size.y
	var ppm := minf((panel.x - 70.0) / maxf(length, width), (panel.y - 20.0) / maxf(height, width))
	var center := _bounds.get_center()
	var views := [
		["Front (+X)", Vector3.RIGHT, width], ["Side (+Z)", Vector3.BACK, length],
		["Back (-X)", Vector3.LEFT, width], ["Side (-Z)", Vector3.FORWARD, length],
	]
	var lines := DimLines.new()
	lines.size = SHEET
	for k in views.size():
		var view: Array = views[k]
		var at := Vector2(MARGIN + k * (panel.x + MARGIN), top)
		_place(await _ortho(panel, view[1], Vector3.UP, ppm, center), at, view[0])
		# Ground line and the horizontal extent under the picture.
		var mid := at + Vector2(panel) * 0.5
		var ground_y := mid.y + (center.y - _bounds.position.y) * ppm
		var half: float = float(view[2]) * 0.5 * ppm
		lines.add_span(Vector2(mid.x - half, ground_y + 22), Vector2(mid.x + half, ground_y + 22),
			"%.1f m" % float(view[2]))
		if k == 0:
			# Heights at the right edge: ground, eaves, ridge.
			var x := mid.x + half + 26
			lines.add_height(x, ground_y, ppm, [[0.0, "0 m"],
				[_eaves - _bounds.position.y, "%.1f m eaves" % (_eaves - _bounds.position.y)],
				[_ridge - _bounds.position.y, "%.1f m ridge" % (_ridge - _bounds.position.y)]])
	var at_top := Vector2(MARGIN + 4 * (panel.x + MARGIN), top)
	# Top view: +X to the right, +Z down.
	_place(await _ortho(panel, Vector3.UP, Vector3.FORWARD, ppm, center), at_top, "Top")
	var mid_top := at_top + Vector2(panel) * 0.5
	lines.add_span(Vector2(mid_top.x - length * 0.5 * ppm, mid_top.y + width * 0.5 * ppm + 18),
		Vector2(mid_top.x + length * 0.5 * ppm, mid_top.y + width * 0.5 * ppm + 18), "%.1f m" % length)
	lines.add_span(Vector2(mid_top.x - length * 0.5 * ppm - 18, mid_top.y - width * 0.5 * ppm),
		Vector2(mid_top.x - length * 0.5 * ppm - 18, mid_top.y + width * 0.5 * ppm), "%.1f m" % width)
	_canvas.add_child(lines)
	_label("Scale %.1f px/m (all four elevations and the top view)" % ppm,
		Vector2(MARGIN, top + panel.y + 30), 14, DIM_INK)


## A wall up close: orthographic, the +X facade at the first storey.
func _close_wall(size: Vector2i) -> Image:
	# The lower edge of the picture no lower than the ground.
	var half_height := CLOSE_M * size.y / size.x * 0.5
	var y := maxf(_bounds.position.y + minf(3.5, (_eaves - _bounds.position.y) * 0.4),
		_bounds.position.y + half_height - 0.2)
	var at := Vector3(_walls.end.x, y, _walls.get_center().z)
	return await _ortho(size, Vector3.RIGHT, Vector3.UP, size.x / CLOSE_M, at)


## The roof up close: orthographic, from the front left and above, at the
## middle of the roof's lower half.
func _close_roof(size: Vector2i) -> Image:
	var center := _bounds.get_center()
	var at := Vector3(center.x, lerpf(_eaves, _ridge, 0.45), center.z + _walls.size.z * 0.2)
	var direction := Vector3(0.0, 0.8, 0.6).normalized()
	var image: Image = await _ortho(size, direction, Vector3.UP, size.x / CLOSE_M, at)
	return image


## The view at the RTS camera's start distance and pitch (from
## `rts_camera.tscn`), looking at the building.
func _game_view(size: Vector2i) -> Array:
	var rig := (load("res://scenes/camera/rts_camera.tscn") as PackedScene).instantiate()
	var distance: float = rig.start_distance
	var pitch: float = rig.base_pitch(distance)
	var fov: float = rig.fov_deg
	rig.free()
	_ground.visible = true
	_environment.background_color = SKY
	_camera.projection = Camera3D.PROJECTION_PERSPECTIVE
	_camera.fov = fov
	var center := _bounds.get_center()
	var direction := Basis(Vector3.UP, deg_to_rad(-35.0)) * Basis(Vector3.RIGHT, -pitch) * Vector3.BACK
	_camera.position = center + direction * distance
	_camera.look_at(center, Vector3.UP)
	_camera.near = 0.5
	_camera.far = distance * 4.0
	return [await _render(size), distance]


## Dimension lines over the sheet.
class DimLines:
	extends Control

	const INK := Color(0.2, 0.2, 0.25)
	var _spans: Array = []
	var _heights: Array = []

	func add_span(a: Vector2, b: Vector2, text: String) -> void:
		_spans.append([a, b, text])

	## Ticks at `marks` ([meters, text]) above `ground_y` at x.
	func add_height(x: float, ground_y: float, ppm: float, marks: Array) -> void:
		_heights.append([x, ground_y, ppm, marks])

	func _draw() -> void:
		var font := get_theme_default_font()
		for span: Array in _spans:
			var a: Vector2 = span[0]
			var b: Vector2 = span[1]
			draw_line(a, b, INK, 1.5)
			var normal := (b - a).orthogonal().normalized() * 6.0
			draw_line(a - normal, a + normal, INK, 1.5)
			draw_line(b - normal, b + normal, INK, 1.5)
			var mid := (a + b) * 0.5
			var text: String = span[2]
			var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 15).x
			if absf(b.x - a.x) >= absf(b.y - a.y):
				draw_rect(Rect2(mid - Vector2(width * 0.5 + 4, 10), Vector2(width + 8, 18)), Color(0.93, 0.93, 0.92))
				draw_string(font, mid + Vector2(-width * 0.5, 5), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 15, INK)
			else:
				draw_string(font, mid + Vector2(-width - 10, 5), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 15, INK)
		for h: Array in _heights:
			var x: float = h[0]
			var ground_y: float = h[1]
			var ppm: float = h[2]
			var top_y := ground_y
			for mark: Array in h[3]:
				var y := ground_y - float(mark[0]) * ppm
				top_y = minf(top_y, y)
				draw_line(Vector2(x - 6, y), Vector2(x + 6, y), INK, 1.5)
				draw_dashed_line(Vector2(x - 40, y), Vector2(x - 6, y), INK, 1.0, 4.0)
				draw_string(font, Vector2(x + 10, y + 5), str(mark[1]), HORIZONTAL_ALIGNMENT_LEFT, -1, 15, INK)
			draw_line(Vector2(x, ground_y), Vector2(x, top_y), INK, 1.5)
