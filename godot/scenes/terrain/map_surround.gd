extends Node3D
## Map surround (prompt 222): low-cost ground, water and sparse far-tree skirt
## continuing beyond the four map edges so the wide camera perspective (and
## tilted classic views) look out over receding land/sea instead of empty sky.
## Presentation only: never playable, never picked, never casts shadows.
##
## Geometry (Variant A): one watertight mesh (`SurroundLand`) made of four side
## strips + four corner grids across `RING_DIST` (0..1100 m). Ring 0 vertices
## sit on every map border sample and snap their height in the vertex shader to
## the adjacent playable chunk's current LOD (`edge_lod_tex`), guaranteeing zero
## gap at any LOD transition or camera angle.
##
## Coastal and river edges: wherever the map border is water, the surround bed
## slopes gently below water level and one `SurroundWater` mesh extends the sea
## or river channel smoothly out to the horizon.
##
## Vegetation: sparse existing `<Variant>_Far` tree meshes (no new models, no
## shadows, at most 4 sides x 2-3 species MultiMeshes), seeded deterministically
## from the map name and scaled to the measured tree density near each edge.

const SURROUND_SHADER := preload("res://assets/materials/terrain_surround.gdshader")
const WATER_SURROUND_SHADER := preload("res://assets/materials/water_surround.gdshader")
const ALBEDO_ARRAY := preload("res://assets/textures/terrain/terrain_albedo.png")
const MACRO := preload("res://assets/textures/terrain/terrain_macro.png")
const MeadowTufts := preload("res://scenes/world/meadow_tufts.gd")
const Water := preload("res://scenes/world/water.gd")

const MODELS_DIR := "res://assets/models/neutral/"
const CHUNK_CELLS := 64
## Outward distances (m) of the surround rings from the map border (1100 m
## covers the farthest wide-perspective reach and classic tilted views).
const RING_DIST: Array[float] = [0.0, 8.0, 24.0, 56.0, 120.0, 240.0, 440.0, 720.0, 1100.0]
## Sample stride along the map edge for each ring (powers of two for clean
## 4:1 / 2:1 watertight stitching with zero T-junctions).
const RING_STEP: Array[int] = [1, 4, 8, 16, 32, 32, 32, 64, 64]
## Angular segments across each corner quadrant (4 segments = 5 points per ring).
const CORNER_SEGMENTS := 4
## Maximum surround far-trees per side (keeps draw calls and triangle count tiny).
const MAX_TREES_PER_SIDE := 700
## Tree scatter band (m outward from map border) and grid cell (m).
## Starts 60 m beyond the border so coarse `_Far` tree meshes stay at natural
## mid-to-far distance in the haze rather than standing right beside the camera.
const TREE_MIN_DIST := 60.0
const TREE_MAX_DIST := 680.0
const TREE_CELL_M := 22.0

## Biome -> far tree species `[catalog_id, variant, weight]`.
const BIOME_TREES: Dictionary[String, Array] = {
	"forest": [
		["env_tree_deciduous", "Oak", 0.58],
		["env_tree_deciduous", "Beech", 0.32],
		["env_tree_conifer", "Spruce", 0.10],
	],
	"winter": [
		["env_tree_deciduous", "Bare", 0.45],
		["env_tree_conifer", "Spruce", 0.55],
	],
	"mediterranean": [
		["env_tree_olive", "Olive", 0.45],
		["env_tree_stone_pine", "StonePine", 0.35],
		["env_tree_cypress", "Cypress", 0.20],
	],
	"desert": [
		["env_tree_palm", "Palm", 1.0],
	],
}

var _samples := Vector2i.ZERO
var _chunks := Vector2i.ZERO
var _cell := 2.0
var _map_size := Vector2.ZERO
var _map_name := ""
var _map_seed := 1
var _relief := 10.0
var _land_base_h := 0.0
var _snow_line := 1.0e6
var _mountain := 0.0
var _biome: Resource
var _env_biome := "forest"
var _enabled := true
var _tree_density_scale := 1.0

var _land_node: MeshInstance3D
var _land_material: ShaderMaterial
var _water_node: MeshInstance3D
var _water_material: ShaderMaterial
var _trees_root: Node3D
var _lod_image: Image
var _lod_texture: ImageTexture
var _last_edge_lods := PackedByteArray()

## Cached per-edge profiles for fast mesh building, tree placement, and tests.
## Order: 0 = North (-Z, i=0..sx-1), 1 = East (+X, j=0..sy-1),
## 2 = South (+Z, i=sx-1..0), 3 = West (-X, j=sy-1..0).
## Each entry holds arrays indexed by canonical sample `0..n-1`.
var _edges: Array[Dictionary] = []
var _has_border_water := false
var _water_base_level := 0.0
## Measured border tree density per edge (bins of 64 m, trees per 10000 m^2).
var _edge_tree_density: Array[PackedFloat32Array] = []
var _map_tree_density := 0.0
var _loaded_models: Dictionary[String, Node3D] = {}

## Diagnostics for self-test / debug.
var land_triangles := 0
var water_triangles := 0
var tree_instances := 0


func is_active() -> bool:
	return _enabled and _land_node != null and _land_node.visible


func build(host: Object, height_img: Image, splat0_img: Image, splat1_img: Image,
		height_tex: ImageTexture, water_levels: PackedFloat32Array) -> void:
	_samples = host.grid_samples(1)
	_cell = host.cell_size()
	_map_size = host.map_size()
	_chunks = Vector2i(
		ceili(float(_samples.x - 1) / CHUNK_CELLS),
		ceili(float(_samples.y - 1) / CHUNK_CELLS)
	)
	_map_name = str(host.map_name())
	_map_seed = _fnv1a(_map_name)

	_extract_edges(height_img, splat0_img, splat1_img, water_levels)
	_measure_edge_trees(host)
	_ensure_nodes(height_tex)
	_build_land_and_water()
	_build_trees()
	if _biome != null:
		apply_biome(_biome)
	_apply_visibility()


## Rebuilds the land/water skirt when border heights or water changed (editor /
## border craters). Preserves materials and LOD texture.
func rebuild_geometry(height_img: Image, splat0_img: Image, splat1_img: Image,
		water_levels: PackedFloat32Array) -> void:
	if _land_node == null:
		return
	_extract_edges(height_img, splat0_img, splat1_img, water_levels)
	_build_land_and_water()
	_build_trees()
	_apply_visibility()


## Updates the tiny `max_chunks x 4` R8 texture carrying the current LOD stride
## of every playable border chunk, so Ring 0 vertices match the playable edge
## with zero crack at every LOD level. Uploads only when a border LOD changed.
func update_edge_lods(chunk_lods: PackedInt32Array) -> void:
	if _lod_image == null or chunk_lods.is_empty():
		return
	var w := _lod_image.get_width()
	var nx := _chunks.x
	var ny := _chunks.y
	var bytes := PackedByteArray()
	bytes.resize(w * 4)
	bytes.fill(1)
	for cx in nx:
		bytes[0 * w + cx] = 1 << chunk_lods[cx]
		bytes[2 * w + cx] = 1 << chunk_lods[(ny - 1) * nx + cx]
	for cy in ny:
		bytes[1 * w + cy] = 1 << chunk_lods[cy * nx + (nx - 1)]
		bytes[3 * w + cy] = 1 << chunk_lods[cy * nx]
	if bytes == _last_edge_lods:
		return
	_last_edge_lods = bytes
	_lod_image.set_data(w, 4, false, Image.FORMAT_R8, bytes)
	_lod_texture.update(_lod_image)


func apply_biome(preset: Resource) -> void:
	_biome = preset
	if preset == null:
		return
	var next_env: String = preset.env_biome
	var trees_dirty := next_env != _env_biome
	_env_biome = next_env
	if _land_material != null:
		_land_material.set_shader_parameter("layer_source", preset.layer_source)
		var tints := PackedColorArray(preset.layer_tint)
		tints.resize(10)
		for i in tints.size():
			if tints[i] == Color(0, 0, 0, 0):
				tints[i] = Color.WHITE
		_land_material.set_shader_parameter("layer_tint", tints)
		_land_material.set_shader_parameter("use_biome", true)
		_land_material.set_shader_parameter("snow_cover", preset.snow_cover)
		_land_material.set_shader_parameter("grass_variation", preset.grass_variation)
		_land_material.set_shader_parameter("meadow_strength", preset.meadow_strength)
		MeadowTufts.apply_palette(_land_material, preset.env_biome, preset.meadow_tint)
	if _water_material != null:
		_water_material.set_shader_parameter("shallow_color", preset.water_shallow)
		_water_material.set_shader_parameter("deep_color", preset.water_deep)
		_water_material.set_shader_parameter("shore_color", preset.water_shore)
	if trees_dirty and _trees_root != null:
		_build_trees()
	elif _trees_root != null:
		_apply_tree_look()


func set_relief(mountain_val: float, snow_line_val: float) -> void:
	_mountain = mountain_val
	_snow_line = snow_line_val
	if _land_material != null:
		_land_material.set_shader_parameter("mountain", mountain_val)
		_land_material.set_shader_parameter("snow_line", snow_line_val)


func set_wetness(value: float) -> void:
	if _land_material != null:
		_land_material.set_shader_parameter("wetness", value)


func set_quality(surround_on: bool, tree_scale: float) -> void:
	var rebuild_trees := not is_equal_approx(_tree_density_scale, tree_scale)
	_enabled = surround_on
	_tree_density_scale = tree_scale
	if rebuild_trees and _trees_root != null:
		_build_trees()
	_apply_visibility()


func _apply_visibility() -> void:
	if _land_node != null:
		_land_node.visible = _enabled
	if _water_node != null:
		_water_node.visible = _enabled and _has_border_water and water_triangles > 0
	if _trees_root != null:
		_trees_root.visible = _enabled and _tree_density_scale > 0.0


## Evaluates the surround height at any world point (x, z) on or outside the
## map border (matches the built mesh vertices; used by tree scatter and tests).
func height_at(x: float, z: float) -> float:
	if _edges.is_empty():
		return 0.0
	var hx := _map_size.x * 0.5
	var hz := _map_size.y * 0.5
	var dx := maxf(absf(x) - hx, 0.0)
	var dz := maxf(absf(z) - hz, 0.0)
	var d := sqrt(dx * dx + dz * dz)
	if dx > 0.0 and dz > 0.0:
		# Corner quadrant: blend the two adjacent edge evaluations by angle.
		var corner_x := -hx if x < 0.0 else hx
		var corner_z := -hz if z < 0.0 else hz
		var e_h := 0 if z < 0.0 else 2
		var e_v := 3 if x < 0.0 else 1
		var idx_h := 0 if x < 0.0 else _samples.x - 1
		var idx_v := 0 if z < 0.0 else _samples.y - 1
		var angle := atan2(dz, dx)
		var wt_h := sin(angle) * sin(angle)
		var wt_v := 1.0 - wt_h
		var h1 := _eval_edge_height(e_h, idx_h, d, corner_x, z)
		var h2 := _eval_edge_height(e_v, idx_v, d, x, corner_z)
		return h1 * wt_h + h2 * wt_v
	if dz >= dx:
		var edge_id := 0 if z <= 0.0 else 2
		var idx := clampi(roundi((x + hx) / _cell), 0, _samples.x - 1)
		return _eval_edge_height(edge_id, idx, dz, x, z)
	var edge_id := 3 if x <= 0.0 else 1
	var idx := clampi(roundi((z + hz) / _cell), 0, _samples.y - 1)
	return _eval_edge_height(edge_id, idx, dx, x, z)


## Maximum height discrepancy (m) between Ring-0 surround border vertices and
## `host.height_at()` across all four map borders (for self-test).
func max_edge_height_error(host: Object) -> float:
	if _edges.is_empty():
		return INF
	var hx := _map_size.x * 0.5
	var hz := _map_size.y * 0.5
	var worst := 0.0
	var step_x := maxi(1, (_samples.x - 1) / 32)
	var step_y := maxi(1, (_samples.y - 1) / 32)
	for i in range(0, _samples.x, step_x):
		var x := i * _cell - hx
		worst = maxf(worst, absf(_edges[0].h0[i] - host.height_at(x, -hz)))
		worst = maxf(worst, absf(_edges[2].h0[i] - host.height_at(x, hz)))
	for j in range(0, _samples.y, step_y):
		var z := j * _cell - hz
		worst = maxf(worst, absf(_edges[1].h0[j] - host.height_at(hx, z)))
		worst = maxf(worst, absf(_edges[3].h0[j] - host.height_at(-hx, z)))
	return worst


# --- Border extraction -------------------------------------------------------


func _extract_edges(height_img: Image, splat0_img: Image, splat1_img: Image,
		water_levels: PackedFloat32Array) -> void:
	_edges.clear()
	_has_border_water = false
	var has_w := water_levels.size() == _samples.x * _samples.y
	var sx := _samples.x
	var sy := _samples.y
	var land_sum := 0.0
	var land_cnt := 0
	var water_sum := 0.0
	var water_cnt := 0
	var min_h := INF
	var max_h := -INF

	# Edge descriptors: [length, is_horizontal, fixed_coord, inward_dir]
	var specs := [
		[sx, true, 0, 1],
		[sy, false, sx - 1, -1],
		[sx, true, sy - 1, -1],
		[sy, false, 0, 1],
	]
	for spec: Array in specs:
		var n: int = spec[0]
		var horiz: bool = spec[1]
		var fixed: int = spec[2]
		var inward: int = spec[3]
		var h0 := PackedFloat32Array()
		var slope := PackedFloat32Array()
		var is_w := PackedFloat32Array()
		var w_lvl := PackedFloat32Array()
		var sand := PackedFloat32Array()
		var earth := PackedFloat32Array()
		var rock := PackedFloat32Array()
		h0.resize(n); slope.resize(n); is_w.resize(n); w_lvl.resize(n)
		sand.resize(n); earth.resize(n); rock.resize(n)
		var inner_off := mini(4, (sy if horiz else sx) - 1)
		for k in n:
			var ix := k if horiz else fixed
			var iy := fixed if horiz else k
			var h := height_img.get_pixel(ix, iy).r
			var ix_in := ix if horiz else clampi(fixed + inward * inner_off, 0, sx - 1)
			var iy_in := clampi(fixed + inward * inner_off, 0, sy - 1) if horiz else iy
			var h_in := height_img.get_pixel(ix_in, iy_in).r
			var sl := clampf((h - h_in) / maxf(float(inner_off) * _cell, 1.0), -0.16, 0.16)
			var s0 := splat0_img.get_pixel(ix, iy)
			var s1 := splat1_img.get_pixel(ix, iy)
			var wl := water_levels[iy * sx + ix] if has_w else -10000.0
			var w_splat := s1.b + s1.a
			var water_flag := 0.0
			if wl > -9000.0 and (h < wl + 0.12 or w_splat > 0.35):
				water_flag = 1.0
				_has_border_water = true
				water_sum += wl
				water_cnt += 1
			else:
				wl = NAN
				land_sum += h
				land_cnt += 1
			min_h = minf(min_h, h)
			max_h = maxf(max_h, h)
			h0[k] = h
			slope[k] = 0.0 if water_flag > 0.5 else sl
			is_w[k] = water_flag
			w_lvl[k] = wl
			sand[k] = clampf(s0.a, 0.0, 1.0)
			earth[k] = clampf(s0.g * 0.45 + s0.b * 0.35 + s1.r * 0.7, 0.0, 1.0)
			rock[k] = 0.0
		# Fill nearest water level along the edge for smooth shoreline overlap.
		var w_near := _propagate_water_level(w_lvl)
		_edges.append({
			"n": n,
			"h0": h0,
			"h_r1": _box_blur(h0, 8),
			"h_r2": _box_blur(h0, 28),
			"h_r3": _box_blur(h0, 80),
			"h_r4": _box_blur(h0, 180),
			"slope": _box_blur(slope, 6),
			"is_w": is_w,
			"is_w_soft": _box_blur(is_w, 4),
			"is_w_far": _box_blur(is_w, 16),
			"w_lvl": w_lvl,
			"w_near": w_near,
			"sand": _box_blur(sand, 4),
			"earth": _box_blur(earth, 4),
			"rock": rock,
		})
	_land_base_h = (land_sum / float(land_cnt)) if land_cnt > 0 else 0.0
	_water_base_level = (water_sum / float(water_cnt)) if water_cnt > 0 else 0.0
	_relief = maxf(max_h - min_h, 2.0)


static func _box_blur(src: PackedFloat32Array, radius: int) -> PackedFloat32Array:
	var n := src.size()
	var out := PackedFloat32Array()
	out.resize(n)
	if n == 0 or radius <= 0:
		return src.duplicate()
	var prefix := PackedFloat64Array()
	prefix.resize(n + 1)
	prefix[0] = 0.0
	for i in n:
		prefix[i + 1] = prefix[i] + src[i]
	for i in n:
		var a := maxi(0, i - radius)
		var b := mini(n - 1, i + radius)
		out[i] = float((prefix[b + 1] - prefix[a]) / float(b - a + 1))
	return out


static func _propagate_water_level(w_lvl: PackedFloat32Array) -> PackedFloat32Array:
	var n := w_lvl.size()
	var out := w_lvl.duplicate()
	var last := NAN
	for i in n:
		if not is_nan(out[i]):
			last = out[i]
		elif not is_nan(last):
			out[i] = last
	last = NAN
	for i in range(n - 1, -1, -1):
		if not is_nan(w_lvl[i]):
			last = w_lvl[i]
		elif is_nan(out[i]) and not is_nan(last):
			out[i] = last
	return out


func _eval_edge_height(edge_id: int, idx: int, d: float, wx: float, wz: float) -> float:
	var e: Dictionary = _edges[edge_id]
	var i := clampi(idx, 0, int(e.n) - 1)
	if d <= 0.001:
		return e.h0[i]
	var w_now: float = e.is_w[i]
	var w_soft: float = lerpf(e.is_w[i], e.is_w_soft[i], smoothstep(0.0, 24.0, d))
	var w_far: float = lerpf(w_soft, e.is_w_far[i], smoothstep(30.0, 220.0, d))
	var wl: float = e.w_near[i]
	if is_nan(wl):
		wl = _water_base_level

	# Progressive lateral smoothing of border height with outward distance.
	var h: float = e.h0[i]
	if d < 24.0:
		h = lerpf(e.h0[i], e.h_r1[i], smoothstep(0.0, 24.0, d))
	elif d < 90.0:
		h = lerpf(e.h_r1[i], e.h_r2[i], smoothstep(24.0, 90.0, d))
	elif d < 280.0:
		h = lerpf(e.h_r2[i], e.h_r3[i], smoothstep(90.0, 280.0, d))
	else:
		h = lerpf(e.h_r3[i], e.h_r4[i], smoothstep(280.0, 900.0, d))

	# Gentle outward slope continuation so hills crest smoothly instead of folding.
	var sl: float = e.slope[i]
	h += sl * minf(d, 48.0) * exp(-d / 85.0) * 0.45

	# Far relaxation toward calm countryside level (keeps mountain foothills high).
	h = lerpf(h, _land_base_h, smoothstep(260.0, 1100.0, d) * 0.42)

	# Deterministic rolling hills far from the border (zero at d = 0 and on water).
	var amp := clampf(3.0 + _relief * 0.06, 3.0, 16.0)
	var noise_mask := smoothstep(10.0, 160.0, d) * (1.0 - smoothstep(750.0, 1100.0, d) * 0.35) * (1.0 - w_far)
	if noise_mask > 0.001:
		var n1 := _vnoise(wx * 0.0032 + float(_map_seed % 997) * 0.13, wz * 0.0032 + float((_map_seed / 997) % 991) * 0.17) - 0.5
		var n2 := _vnoise(wx * 0.0085 - 17.3, wz * 0.0085 + 29.1) - 0.5
		h += (n1 * 0.72 + n2 * 0.28) * 2.0 * amp * noise_mask

	# Coastal separation: water stays seabed below water level; dry land stays above.
	if _has_border_water:
		if w_now > 0.5 or w_far > 0.5:
			var bed := minf(e.h0[i], wl - lerpf(0.6, 5.5, smoothstep(0.0, 70.0, d)))
			h = lerpf(h, bed, clampf(maxf(w_now * (1.0 - smoothstep(0.0, 16.0, d)), w_far), 0.0, 1.0))
		elif w_soft < 0.2 and not is_nan(e.w_near[i]):
			h = maxf(h, wl + 0.25)
	return h


func _eval_edge_water(edge_id: int, idx: int, d: float) -> Vector2:
	# Returns Vector2(water_mask 0..1, water_level).
	var e: Dictionary = _edges[edge_id]
	var i := clampi(idx, 0, int(e.n) - 1)
	if not _has_border_water:
		return Vector2(0.0, _water_base_level)
	var wm: float = e.is_w[i]
	if d > 0.001:
		var w_soft: float = lerpf(e.is_w[i], e.is_w_soft[i], smoothstep(0.0, 24.0, d))
		wm = lerpf(w_soft, e.is_w_far[i], smoothstep(30.0, 220.0, d))
	var wl: float = e.w_near[i]
	if is_nan(wl):
		wl = _water_base_level
	return Vector2(wm, wl)


func _eval_edge_mat(edge_id: int, idx: int, d: float) -> Color:
	var e: Dictionary = _edges[edge_id]
	var i := clampi(idx, 0, int(e.n) - 1)
	var sand_w: float = e.sand[i] * (1.0 - smoothstep(140.0, 480.0, d) * 0.55)
	# Fade road/track/mud splat at the border smoothly into open countryside.
	var earth_w: float = e.earth[i] * (1.0 - smoothstep(0.0, 28.0, d))
	return Color(sand_w, earth_w, 0.0, 0.0)


# --- Nodes and materials -----------------------------------------------------


func _ensure_nodes(height_tex: ImageTexture) -> void:
	var max_chunks := maxi(_chunks.x, _chunks.y)
	_last_edge_lods = PackedByteArray()
	_last_edge_lods.resize(max_chunks * 4)
	_last_edge_lods.fill(1)
	_lod_image = Image.create_from_data(max_chunks, 4, false, Image.FORMAT_R8, _last_edge_lods)
	_lod_texture = ImageTexture.create_from_image(_lod_image)

	if _land_material == null:
		_land_material = ShaderMaterial.new()
		_land_material.shader = SURROUND_SHADER
		_land_material.set_shader_parameter("albedo_array", ALBEDO_ARRAY)
		_land_material.set_shader_parameter("macro_tex", MACRO)
	_land_material.set_shader_parameter("height_tex", height_tex)
	_land_material.set_shader_parameter("edge_lod_tex", _lod_texture)
	_land_material.set_shader_parameter("samples", _samples)
	_land_material.set_shader_parameter("chunks", _chunks)

	if _land_node == null:
		_land_node = MeshInstance3D.new()
		_land_node.name = "SurroundLand"
		_land_node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(_land_node)

	if _water_material == null:
		_water_material = ShaderMaterial.new()
		_water_material.shader = WATER_SURROUND_SHADER
		_water_material.set_shader_parameter("normal_a", Water._noise_normal(11, 0.045, 5.0))
		_water_material.set_shader_parameter("normal_b", Water._noise_normal(23, 0.02, 2.5))
	if _water_node == null:
		_water_node = MeshInstance3D.new()
		_water_node.name = "SurroundWater"
		_water_node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(_water_node)

	if _trees_root == null:
		_trees_root = Node3D.new()
		_trees_root.name = "SurroundTrees"
		add_child(_trees_root)


# --- Mesh generation (4 side strips + 4 watertight corners) ------------------


func _build_land_and_water() -> void:
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var cols := PackedColorArray()
	var indices := PackedInt32Array()

	var w_verts := PackedVector3Array()
	var w_uvs := PackedVector2Array()
	var w_indices := PackedInt32Array()

	var hx := _map_size.x * 0.5
	var hz := _map_size.y * 0.5
	var rings := RING_DIST.size()

	# Helper: compute world (x, z) for edge `edge_id`, canonical sample `idx`, outward distance `d`.
	var edge_pos := func(edge_id: int, idx: int, d: float) -> Vector2:
		match edge_id:
			0: return Vector2(idx * _cell - hx, -hz - d)
			1: return Vector2(hx + d, idx * _cell - hz)
			2: return Vector2(idx * _cell - hx, hz + d)
			_: return Vector2(-hx - d, idx * _cell - hz)

	# 1. Four side strips, walked clockwise seen from above:
	#    0 (North): i = 0 -> sx-1
	#    1 (East):  j = 0 -> sy-1
	#    2 (South): i = sx-1 -> 0
	#    3 (West):  j = sy-1 -> 0
	var side_end_ring_verts: Array[Array] = [] # [edge][ring] = [first_v, last_v]
	for edge_id in 4:
		var n: int = _edges[edge_id].n
		var reverse := (edge_id == 2 or edge_id == 3)
		var ring_v_ids: Array[PackedInt32Array] = []
		var ring_samples: Array[PackedInt32Array] = []
		var ring_w_ids: Array[PackedInt32Array] = []
		var ring_w_mask: Array[PackedFloat32Array] = []
		for r in rings:
			var step := RING_STEP[r]
			var d := RING_DIST[r]
			var samples_r := _sample_list(n, step, reverse)
			var vids := PackedInt32Array()
			vids.resize(samples_r.size())
			var wids := PackedInt32Array()
			var wms := PackedFloat32Array()
			if _has_border_water:
				wids.resize(samples_r.size())
				wms.resize(samples_r.size())
			for k in samples_r.size():
				var s := samples_r[k]
				var p2: Vector2 = edge_pos.call(edge_id, s, d)
				var h := _eval_edge_height(edge_id, s, d, p2.x, p2.y)
				var wm_wl := _eval_edge_water(edge_id, s, d)
				var mat_c := _eval_edge_mat(edge_id, s, d)
				var vi := verts.size()
				verts.append(Vector3(p2.x, h, p2.y))
				uvs.append(Vector2(d, wm_wl.x))
				uv2s.append(Vector2(float(edge_id), float(s)) if r == 0 else Vector2(-1.0, 0.0))
				cols.append(mat_c)
				vids[k] = vi
				if _has_border_water:
					var wi := w_verts.size()
					w_verts.append(Vector3(p2.x, wm_wl.y, p2.y))
					w_uvs.append(Vector2(d, wm_wl.y - h))
					wids[k] = wi
					wms[k] = wm_wl.x
			ring_v_ids.append(vids)
			ring_samples.append(samples_r)
			ring_w_ids.append(wids)
			ring_w_mask.append(wms)

		# Stitch ring r -> r+1 (always clockwise: inner k->k+1, outer m->m+1).
		for r in rings - 1:
			_stitch_rings(ring_v_ids[r], ring_samples[r], ring_v_ids[r + 1], ring_samples[r + 1], indices)
			if _has_border_water:
				_stitch_water_rings(ring_w_ids[r], ring_samples[r], ring_w_mask[r],
					ring_w_ids[r + 1], ring_samples[r + 1], ring_w_mask[r + 1], w_indices)

		var ends: Array = []
		for r in rings:
			var vids := ring_v_ids[r]
			var wids := ring_w_ids[r] if _has_border_water else PackedInt32Array()
			ends.append([
				vids[0], vids[vids.size() - 1],
				wids[0] if not wids.is_empty() else -1,
				wids[wids.size() - 1] if not wids.is_empty() else -1,
			])
		side_end_ring_verts.append(ends)

	# 2. Four watertight corners connecting side e -> side (e+1)%4 clockwise:
	#    c=0 (NE): North end -> East start (+hx, -hz)
	#    c=1 (SE): East end  -> South start (+hx, +hz)
	#    c=2 (SW): South end -> West start (-hx, +hz)
	#    c=3 (NW): West end  -> North start (-hx, -hz)
	var corner_specs := [
		[0, 1, _samples.x - 1, 0, Vector2(hx, -hz), Vector2(0, -1), Vector2(1, 0)],
		[1, 2, _samples.y - 1, _samples.x - 1, Vector2(hx, hz), Vector2(1, 0), Vector2(0, 1)],
		[2, 3, 0, _samples.y - 1, Vector2(-hx, hz), Vector2(0, 1), Vector2(-1, 0)],
		[3, 0, 0, 0, Vector2(-hx, -hz), Vector2(-1, 0), Vector2(0, -1)],
	]
	for spec: Array in corner_specs:
		var e_in: int = spec[0]
		var e_out: int = spec[1]
		var idx_in: int = spec[2]
		var idx_out: int = spec[3]
		var origin: Vector2 = spec[4]
		var dir_in: Vector2 = spec[5]
		var dir_out: Vector2 = spec[6]
		var v0: int = side_end_ring_verts[e_in][0][1]
		var w0: int = side_end_ring_verts[e_in][0][3]
		var c_rings: Array[PackedInt32Array] = []
		var cw_rings: Array[PackedInt32Array] = []
		var cwm_rings: Array[PackedFloat32Array] = []
		for r in range(1, rings):
			var d := RING_DIST[r]
			var row := PackedInt32Array()
			row.resize(CORNER_SEGMENTS + 1)
			var wrow := PackedInt32Array()
			var wmrow := PackedFloat32Array()
			if _has_border_water:
				wrow.resize(CORNER_SEGMENTS + 1)
				wmrow.resize(CORNER_SEGMENTS + 1)
			# Reuse the exact end vertices of the two adjacent side strips!
			row[0] = side_end_ring_verts[e_in][r][1]
			row[CORNER_SEGMENTS] = side_end_ring_verts[e_out][r][0]
			if _has_border_water:
				wrow[0] = side_end_ring_verts[e_in][r][3]
				wrow[CORNER_SEGMENTS] = side_end_ring_verts[e_out][r][2]
				wmrow[0] = _eval_edge_water(e_in, idx_in, d).x
				wmrow[CORNER_SEGMENTS] = _eval_edge_water(e_out, idx_out, d).x
			for m in range(1, CORNER_SEGMENTS):
				var t := float(m) / float(CORNER_SEGMENTS)
				# Walk cleanly around the L-corner (square outer perimeter).
				var off: Vector2
				if t <= 0.5:
					off = dir_in * d + dir_out * (d * t * 2.0)
				else:
					off = dir_out * d + dir_in * (d * (1.0 - t) * 2.0)
				var p2 := origin + off
				var rad := off.length()
				var wt_out := sin(t * PI * 0.5)
				wt_out *= wt_out
				var wt_in := 1.0 - wt_out
				var h := _eval_edge_height(e_in, idx_in, rad, p2.x, p2.y) * wt_in \
					+ _eval_edge_height(e_out, idx_out, rad, p2.x, p2.y) * wt_out
				var wi_a := _eval_edge_water(e_in, idx_in, rad)
				var wi_b := _eval_edge_water(e_out, idx_out, rad)
				var wm := wi_a.x * wt_in + wi_b.x * wt_out
				var wl := wi_a.y * wt_in + wi_b.y * wt_out
				var mc := _eval_edge_mat(e_in, idx_in, rad).lerp(_eval_edge_mat(e_out, idx_out, rad), wt_out)
				var vi := verts.size()
				verts.append(Vector3(p2.x, h, p2.y))
				uvs.append(Vector2(rad, wm))
				uv2s.append(Vector2(-1.0, 0.0))
				cols.append(mc)
				row[m] = vi
				if _has_border_water:
					var wi := w_verts.size()
					w_verts.append(Vector3(p2.x, wl, p2.y))
					w_uvs.append(Vector2(rad, wl - h))
					wrow[m] = wi
					wmrow[m] = wm
			c_rings.append(row)
			cw_rings.append(wrow)
			cwm_rings.append(wmrow)
		# Fan from corner point v0 to ring 1.
		var w0_mask := maxf(_eval_edge_water(e_in, idx_in, 0.0).x, _eval_edge_water(e_out, idx_out, 0.0).x)
		for m in CORNER_SEGMENTS:
			indices.append_array([v0, c_rings[0][m], c_rings[0][m + 1]])
			if _has_border_water and (w0_mask > 0.01 or cwm_rings[0][m] > 0.01 or cwm_rings[0][m + 1] > 0.01):
				w_indices.append_array([w0, cw_rings[0][m], cw_rings[0][m + 1]])
		# Quads from ring r to ring r+1.
		for r in c_rings.size() - 1:
			var a_row := c_rings[r]
			var b_row := c_rings[r + 1]
			for m in CORNER_SEGMENTS:
				indices.append_array([
					a_row[m], b_row[m], b_row[m + 1],
					a_row[m], b_row[m + 1], a_row[m + 1],
				])
				if _has_border_water:
					var wa := cw_rings[r]
					var wb := cw_rings[r + 1]
					var ma := cwm_rings[r]
					var mb := cwm_rings[r + 1]
					if ma[m] > 0.01 or ma[m + 1] > 0.01 or mb[m] > 0.01 or mb[m + 1] > 0.01:
						w_indices.append_array([
							wa[m], wb[m], wb[m + 1],
							wa[m], wb[m + 1], wa[m + 1],
						])

	# Smooth area-weighted vertex normals across the whole skirt.
	var normals := _compute_normals(verts, indices)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_COLOR] = cols
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.surface_set_material(0, _land_material)
	_land_node.mesh = mesh
	land_triangles = indices.size() / 3

	water_triangles = w_indices.size() / 3
	if _has_border_water and water_triangles > 0:
		var w_normals := PackedVector3Array()
		w_normals.resize(w_verts.size())
		w_normals.fill(Vector3.UP)
		var w_arrays := []
		w_arrays.resize(Mesh.ARRAY_MAX)
		w_arrays[Mesh.ARRAY_VERTEX] = w_verts
		w_arrays[Mesh.ARRAY_NORMAL] = w_normals
		w_arrays[Mesh.ARRAY_TEX_UV] = w_uvs
		w_arrays[Mesh.ARRAY_INDEX] = w_indices
		var w_mesh := ArrayMesh.new()
		w_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, w_arrays)
		w_mesh.surface_set_material(0, _water_material)
		_water_node.mesh = w_mesh
	else:
		_water_node.mesh = null


static func _sample_list(n: int, step: int, reverse: bool) -> PackedInt32Array:
	var list := PackedInt32Array()
	var i := 0
	while i < n - 1:
		list.append(i)
		i += step
	list.append(n - 1)
	if reverse:
		list.reverse()
	return list


## Watertight stitch between inner ring `v_in` (samples `s_in`) and coarser
## or equal outer ring `v_out` (samples `s_out`), both walked clockwise.
static func _stitch_rings(v_in: PackedInt32Array, s_in: PackedInt32Array,
		v_out: PackedInt32Array, s_out: PackedInt32Array, out: PackedInt32Array) -> void:
	if v_in.size() == v_out.size():
		for k in v_in.size() - 1:
			out.append_array([
				v_in[k], v_out[k], v_out[k + 1],
				v_in[k], v_out[k + 1], v_in[k + 1],
			])
		return
	var j := 0
	for k in v_out.size() - 1:
		var target := s_out[k + 1]
		var j_start := j
		while j + 1 < s_in.size() and s_in[j] != target:
			j += 1
		var mid := (j_start + j) >> 1
		for m in range(j_start, mid):
			out.append_array([v_in[m], v_out[k], v_in[m + 1]])
		out.append_array([v_in[mid], v_out[k], v_out[k + 1]])
		for m in range(mid, j):
			out.append_array([v_in[m], v_out[k + 1], v_in[m + 1]])


static func _stitch_water_rings(v_in: PackedInt32Array, s_in: PackedInt32Array, m_in: PackedFloat32Array,
		v_out: PackedInt32Array, s_out: PackedInt32Array, m_out: PackedFloat32Array,
		out: PackedInt32Array) -> void:
	if v_in.size() == v_out.size():
		for k in v_in.size() - 1:
			if m_in[k] > 0.01 or m_in[k + 1] > 0.01 or m_out[k] > 0.01 or m_out[k + 1] > 0.01:
				out.append_array([
					v_in[k], v_out[k], v_out[k + 1],
					v_in[k], v_out[k + 1], v_in[k + 1],
				])
		return
	var j := 0
	for k in v_out.size() - 1:
		var target := s_out[k + 1]
		var j_start := j
		var any_w := m_out[k] > 0.01 or m_out[k + 1] > 0.01
		while j + 1 < s_in.size() and s_in[j] != target:
			if m_in[j] > 0.01:
				any_w = true
			j += 1
		if m_in[j] > 0.01:
			any_w = true
		if not any_w:
			continue
		var mid := (j_start + j) >> 1
		for m in range(j_start, mid):
			out.append_array([v_in[m], v_out[k], v_in[m + 1]])
		out.append_array([v_in[mid], v_out[k], v_out[k + 1]])
		for m in range(mid, j):
			out.append_array([v_in[m], v_out[k + 1], v_in[m + 1]])


static func _compute_normals(verts: PackedVector3Array, indices: PackedInt32Array) -> PackedVector3Array:
	var n := verts.size()
	var acc := PackedVector3Array()
	acc.resize(n)
	acc.fill(Vector3.ZERO)
	for t in range(0, indices.size(), 3):
		var i0 := indices[t]
		var i1 := indices[t + 1]
		var i2 := indices[t + 2]
		# Clockwise seen from above: (v2 - v0) x (v1 - v0) points up (+Y).
		var fn := (verts[i2] - verts[i0]).cross(verts[i1] - verts[i0])
		acc[i0] += fn
		acc[i1] += fn
		acc[i2] += fn
	for i in n:
		var v := acc[i]
		acc[i] = v.normalized() if v.length_squared() > 1e-8 else Vector3.UP
	return acc


# --- Sparse deterministic surround trees (existing `_Far` meshes only) -------


func _measure_edge_trees(host: Object) -> void:
	_edge_tree_density.clear()
	var hx := _map_size.x * 0.5
	var hz := _map_size.y * 0.5
	var bin_m := 64.0
	var bins_x := maxi(1, ceili(_map_size.x / bin_m))
	var bins_y := maxi(1, ceili(_map_size.y / bin_m))
	var counts: Array[PackedFloat32Array] = []
	for cnt in [bins_x, bins_y, bins_x, bins_y]:
		var a := PackedFloat32Array()
		a.resize(cnt)
		a.fill(0.0)
		counts.append(a)
	var strip_m := 96.0
	var total_trees := 0
	if host.has_method(&"object_kinds") and host.has_method(&"object_groups"):
		var kinds: PackedStringArray = host.object_kinds()
		for group: Dictionary in host.object_groups(128.0):
			var kind_idx: int = group.kind
			if kind_idx < 0 or kind_idx >= kinds.size():
				continue
			var id := kinds[kind_idx].get_slice(":", 0)
			if not id.begins_with("env_tree_") or id == "env_tree_stump":
				continue
			var buf: PackedFloat32Array = group.buffer
			total_trees += buf.size() / 12
			for o in range(0, buf.size(), 12):
				var x := buf[o + 3]
				var z := buf[o + 11]
				if z + hz < strip_m:
					var b := clampi(int((x + hx) / bin_m), 0, bins_x - 1)
					counts[0][b] += 1.0
				if hx - x < strip_m:
					var b := clampi(int((z + hz) / bin_m), 0, bins_y - 1)
					counts[1][b] += 1.0
				if hz - z < strip_m:
					var b := clampi(int((x + hx) / bin_m), 0, bins_x - 1)
					counts[2][b] += 1.0
				if x + hx < strip_m:
					var b := clampi(int((z + hz) / bin_m), 0, bins_y - 1)
					counts[3][b] += 1.0
	# Convert raw counts per (64 m x 96 m) bin into normalized 0..1 woodland density.
	var bin_area := bin_m * strip_m
	for e in 4:
		var raw := counts[e]
		for i in raw.size():
			# ~35 trees per 6144 m^2 = dense woodland (1.0).
			raw[i] = clampf(raw[i] / (bin_area / 180.0), 0.0, 1.0)
		_edge_tree_density.append(_box_blur(raw, 1))
	var map_area := maxf(_map_size.x * _map_size.y, 1.0)
	_map_tree_density = clampf(float(total_trees) / (map_area / 220.0), 0.0, 1.0)


func _build_trees() -> void:
	if _trees_root == null:
		return
	for child in _trees_root.get_children():
		child.queue_free()
	tree_instances = 0
	if not _enabled or _tree_density_scale <= 0.0 or _edges.is_empty():
		return
	var species_list: Array = BIOME_TREES.get(_env_biome, BIOME_TREES["forest"])
	var meshes: Array[Mesh] = []
	var locals: Array[Transform3D] = []
	var ids: Array[String] = []
	var weights: Array[float] = []
	var weight_sum := 0.0
	for spec: Array in species_list:
		var part := _far_tree_part(str(spec[0]), str(spec[1]))
		if not part.is_empty():
			meshes.append(part.mesh)
			locals.append(part.transform)
			ids.append(str(spec[0]))
			weights.append(float(spec[2]))
			weight_sum += float(spec[2])
	if meshes.is_empty() or weight_sum <= 0.0:
		return

	var hx := _map_size.x * 0.5
	var hz := _map_size.y * 0.5
	var bin_m := 64.0
	# Desert keeps trees extremely rare; temperate/winter/mediterranean follow
	# measured border tree density with a gentle woodland baseline.
	var biome_cap := 0.12 if _env_biome == "desert" else 1.0
	var base_floor := 0.0 if _env_biome == "desert" else clampf(_map_tree_density * 0.35, 0.08, 0.35)

	# Build per side (4 sides = frustum culling drops the 3 sides behind camera).
	for side in 4:
		var per_species: Array[Array] = []
		for _s in meshes.size():
			per_species.append([])
		var rng := RandomNumberGenerator.new()
		rng.seed = _map_seed + side * 7919 + 131
		var span := (_map_size.x if (side == 0 or side == 2) else _map_size.y) + TREE_MAX_DIST
		var cols := ceili(span / TREE_CELL_M)
		var rows := ceili((TREE_MAX_DIST - TREE_MIN_DIST) / TREE_CELL_M)
		var placed := 0
		var max_side := int(MAX_TREES_PER_SIDE * _tree_density_scale)
		# Stride up if the map side is very long (4-6 km) so scatter stays balanced.
		var stride_cells := maxi(1, ceili(sqrt(float(cols * rows) / 2600.0)))
		var step_m := TREE_CELL_M * float(stride_cells)
		var eff_cols := ceili(span / step_m)
		var eff_rows := ceili((TREE_MAX_DIST - TREE_MIN_DIST) / step_m)
		for r in eff_rows:
			if placed >= max_side:
				break
			var d_base := TREE_MIN_DIST + (float(r) + 0.5) * step_m
			for c in eff_cols:
				if placed >= max_side:
					break
				var u := (float(c) + rng.randf()) * step_m - span * 0.5
				var d := d_base + (rng.randf() - 0.5) * step_m * 0.85
				if d < TREE_MIN_DIST or d > TREE_MAX_DIST:
					continue
				var wx: float
				var wz: float
				match side:
					0: wx = u; wz = -hz - d
					1: wx = hx + d; wz = u
					2: wx = -u; wz = hz + d
					_: wx = -hx - d; wz = -u
				# Avoid duplicating corner quadrants between adjacent sides: each
				# side owns its leading corner (u >= -half).
				var half_side := hx if (side == 0 or side == 2) else hz
				if u < -half_side:
					continue
				var coord := wx if (side == 0 or side == 2) else wz
				var clamped_c := clampf(coord, -half_side, half_side)
				var idx := clampi(roundi((clamped_c + half_side) / _cell), 0, int(_edges[side].n) - 1)
				var w_info := _eval_edge_water(side, idx, d)
				if w_info.x > 0.08:
					continue
				# Skip sandy beaches (coastal dunes stay open sand).
				if _edges[side].sand[idx] > 0.45 and _env_biome != "desert":
					continue
				var h := height_at(wx, wz)
				if h > _snow_line - 15.0:
					continue
				# Check slope roughly via finite difference so cliffs stay bare.
				var hx1 := height_at(wx + 8.0, wz)
				var hz1 := height_at(wx, wz + 8.0)
				var grade := Vector2(hx1 - h, hz1 - h).length() / 8.0
				if grade > 0.42:
					continue
				var bins := _edge_tree_density[side]
				var b := clampi(int((clamped_c + half_side) / bin_m), 0, bins.size() - 1)
				var edge_d := bins[b]
				# Patchy woodland clustering in world space (natural distant groves),
				# ramping in smoothly from TREE_MIN_DIST so trees form background woods.
				var clump := _vnoise(wx * 0.009 + 11.3, wz * 0.009 - 7.1)
				var near_ramp := smoothstep(TREE_MIN_DIST, TREE_MIN_DIST + 70.0, d)
				var chance := maxf(edge_d * 0.85, base_floor) * biome_cap * _tree_density_scale
				chance *= near_ramp * smoothstep(0.32, 0.72, clump) * (1.0 - smoothstep(460.0, TREE_MAX_DIST, d) * 0.65)
				if rng.randf() > chance:
					continue
				var pick := rng.randf() * weight_sum
				var sp := 0
				var acc := 0.0
				for k in weights.size():
					acc += weights[k]
					if pick <= acc:
						sp = k
						break
				var sc := rng.randf_range(0.85, 1.30)
				var rot := rng.randf() * TAU
				var basis := Basis(Vector3.UP, rot).scaled(Vector3(sc * 1.15, sc * 0.92, sc * 1.15))
				per_species[sp].append(Transform3D(basis, Vector3(wx, h - 0.5, wz)) * locals[sp])
				placed += 1

		for sp in meshes.size():
			var xforms: Array = per_species[sp]
			if xforms.is_empty():
				continue
			var mm := MultiMesh.new()
			mm.transform_format = MultiMesh.TRANSFORM_3D
			mm.mesh = meshes[sp]
			mm.instance_count = xforms.size()
			for i in xforms.size():
				mm.set_instance_transform(i, xforms[i])
			var node := MultiMeshInstance3D.new()
			node.name = "SurroundTrees_%d_%d" % [side, sp]
			node.multimesh = mm
			node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			node.set_meta(&"tree_id", ids[sp])
			EnvLook.apply_bend(node, ids[sp])
			_trees_root.add_child(node)
			tree_instances += xforms.size()
	_apply_tree_look()


func _apply_tree_look() -> void:
	if _trees_root == null:
		return
	# Slightly muted foliage to harmonize with the calmer surround ground (prompt 208).
	var muted := Vector4(0.88, 0.88, 0.90, 1.0)
	for child in _trees_root.get_children():
		var gi := child as GeometryInstance3D
		if gi == null:
			continue
		gi.set_instance_shader_parameter(&"env_foliage_mod", muted)
		gi.set_instance_shader_parameter(&"env_paint", EnvLook.NEUTRAL_PAINT)


func _far_tree_part(id: String, variant: String) -> Dictionary:
	if not _loaded_models.has(id):
		var path := MODELS_DIR + id + ".glb"
		var model: Node3D = null
		if ResourceLoader.exists(path):
			model = (load(path) as PackedScene).instantiate() as Node3D
		_loaded_models[id] = model
	var root: Node3D = _loaded_models[id]
	if root == null:
		return {}
	var mesh_node := EnvLook.variant_node(root, variant + "_Far") as MeshInstance3D
	if mesh_node == null:
		mesh_node = EnvLook.variant_node(root, variant) as MeshInstance3D
	if mesh_node == null or mesh_node.mesh == null:
		return {}
	var t := Transform3D.IDENTITY
	var n: Node = mesh_node
	while n != null and n != root:
		if n is Node3D:
			t = (n as Node3D).transform * t
		n = n.get_parent()
	return {"mesh": mesh_node.mesh, "transform": t}


func _exit_tree() -> void:
	for model: Node3D in _loaded_models.values():
		if model != null:
			model.free()
	_loaded_models.clear()


# --- Deterministic hash and value noise --------------------------------------


static func _fnv1a(text: String) -> int:
	var h := 2166136261
	for i in text.length():
		h = ((h ^ text.unicode_at(i)) * 16777619) & 0x7fffffff
	return h


static func _hash2(ix: int, iz: int) -> float:
	var n := (ix * 374761393 + iz * 668265263) & 0x7fffffff
	n = ((n ^ (n >> 13)) * 1274126177) & 0x7fffffff
	return float((n ^ (n >> 16)) & 0xffff) / 65535.0


static func _vnoise(x: float, z: float) -> float:
	var ix := floori(x)
	var iz := floori(z)
	var fx := x - float(ix)
	var fz := z - float(iz)
	fx = fx * fx * (3.0 - 2.0 * fx)
	fz = fz * fz * (3.0 - 2.0 * fz)
	var a := _hash2(ix, iz)
	var b := _hash2(ix + 1, iz)
	var c := _hash2(ix, iz + 1)
	var d := _hash2(ix + 1, iz + 1)
	return lerpf(lerpf(a, b, fx), lerpf(c, d, fx), fz)
