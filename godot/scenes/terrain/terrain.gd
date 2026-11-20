extends Node3D
## Terrain renderer (prompt 011): the simulation's height field as chunked,
## LOD-switched grids with a splatting shader. Presentation only.
##
## Data path: SimHost.terrain_layers(rect) computes heights, normals, splat
## weights and the track direction for a rectangle of samples (Rust, see
## crates/bridge/src/terrain_layers.rs). They live in GPU textures with one
## texel per map sample. All chunks share one flat grid mesh per LOD level;
## the vertex shader (assets/materials/terrain.gdshader) reads the heights.
##
## Every frame (when the camera moved) each chunk gets a LOD level from its
## distance to the camera, chunks outside the view frustum are dropped, and
## the rest are written into one MultiMesh per LOD level: five draw calls for
## the whole terrain, plus shadows. Each instance carries the strides of its
## four neighbors, so the shader closes the seams between levels.
##
## `heights_changed(rect)` re-reads a rectangle of samples from the SimHost and
## uploads it (craters in prompt 021); `debug_crater` exercises it now.
## No collision, no Godot physics: picking uses SimHost.ray_pick.

const SHADER := preload("res://assets/materials/terrain.gdshader")
const EDGE_SHADER := preload("res://assets/materials/terrain_edge.gdshader")
const ALBEDO_ARRAY := preload("res://assets/textures/terrain/terrain_albedo.png")
const NORMAL_ARRAY := preload("res://assets/textures/terrain/terrain_normal.png")
const MACRO := preload("res://assets/textures/terrain/terrain_macro.png")
const TallGrass := preload("res://scenes/world/tall_grass.gd")

## Material layers, in texture-array order (crates/cli/src/gen_textures.rs).
const LAYERS: Array[String] = [
	"grass", "dirt_track", "road_gravel", "sand", "mud",
	"snow", "river_pebbles", "river_silt", "rock", "earth",
]
## Tile size of every layer in meters (same order; the generator draws
## stones and blades for these sizes).
const LAYER_TILE_M: Array[float] = [4.0, 4.0, 3.0, 6.0, 5.0, 6.0, 3.0, 6.0, 8.0, 4.0]

## Cells per chunk side (64 cells = 128 m on the 2 m grid).
const CHUNK_CELLS := 64
## Number of LOD levels; level k uses every 2^k-th sample.
const LOD_LEVELS := 5
## Floats per MultiMesh instance: 3x4 transform + custom data.
const INSTANCE_FLOATS := 16

## Distance (m) up to which chunks use full detail; every doubling of the
## distance drops one level.
@export var lod0_distance := 140.0
## A chunk changes its level only when it is this share beyond the boundary
## (prompt 090): coarser from 10 % farther, finer from 10 % nearer. Without
## it a chunk at a boundary switches back and forth with every small camera
## move, and hill outlines and their shadows pop. 0: the level follows the
## distance alone.
@export var lod_hysteresis := 0.1
## Chunks this far outside the view frustum still count as visible, so hills
## just off screen keep casting their shadows into it.
@export var cull_margin := 40.0
## Depth of the map edge cut below the lowest point of the map, in meters.
@export var edge_depth := 12.0

## SimHost in a match, MapEditor in the map editor (prompt 028): both offer
## the same read functions.
var sim_host: Object
## Visible chunks per LOD level in the last update (for the debug overlay).
var visible_per_lod := PackedInt32Array()
## Duration of the last chunk update in microseconds (for the benchmark).
var last_update_usec := 0
## Number of `heights_changed` uploads so far (for tests).
var region_updates := 0

var _samples := Vector2i.ZERO
var _cell := 2.0
var _map_size := Vector2.ZERO
var _chunks := Vector2i.ZERO
var _ranges := PackedVector2Array()
var _lods := PackedInt32Array()
## No level from an earlier update to hold on to yet.
var _lods_fresh := true
var _material: ShaderMaterial
var _edge_material: ShaderMaterial
## BiomePreset applied by `apply_biome` (prompt 015), or null.
var _biome: Resource
var _images: Dictionary[String, Image] = {}
var _textures: Dictionary[String, ImageTexture] = {}
var _lod_nodes: Array[MultiMeshInstance3D] = []
var _buffers: Array[PackedFloat32Array] = []
var _edge: MeshInstance3D
var _last_camera := Transform3D()
var _last_fov := 0.0
var _dirty := true


## Builds textures, meshes and the map edge for the map loaded in `host`.
func build(host: Object) -> void:
	var started := Time.get_ticks_usec()
	sim_host = host
	_samples = host.grid_samples(1)
	_cell = host.cell_size()
	_map_size = host.map_size()
	_chunks = Vector2i(
		ceili(float(_samples.x - 1) / CHUNK_CELLS), ceili(float(_samples.y - 1) / CHUNK_CELLS)
	)
	_lods.resize(_chunks.x * _chunks.y)
	_ranges = host.terrain_chunk_ranges(CHUNK_CELLS)

	var layers: Dictionary = host.terrain_layers(0, 0, _samples.x, _samples.y)
	_images["height"] = Image.create_from_data(
		_samples.x, _samples.y, false, Image.FORMAT_RF, (layers["height"] as PackedFloat32Array).to_byte_array()
	)
	_images["normal"] = Image.create_from_data(_samples.x, _samples.y, false, Image.FORMAT_RG8, layers["normal"])
	for key: String in ["splat0", "splat1", "track", "field"]:
		_images[key] = Image.create_from_data(_samples.x, _samples.y, false, Image.FORMAT_RGBA8, layers[key])
	for key: String in _images:
		_textures[key] = ImageTexture.create_from_image(_images[key])

	_material = ShaderMaterial.new()
	_material.shader = SHADER
	_material.set_shader_parameter("height_tex", _textures["height"])
	_material.set_shader_parameter("normal_tex", _textures["normal"])
	_material.set_shader_parameter("splat0_tex", _textures["splat0"])
	_material.set_shader_parameter("splat1_tex", _textures["splat1"])
	_material.set_shader_parameter("track_tex", _textures["track"])
	# Fields (prompt 088): masks and row direction.
	_material.set_shader_parameter("field_tex", _textures["field"])
	_material.set_shader_parameter("samples", _samples)
	_material.set_shader_parameter("half_size", _map_size * 0.5)
	_material.set_shader_parameter("cell", _cell)
	_material.set_shader_parameter("albedo_array", ALBEDO_ARRAY)
	_material.set_shader_parameter("normal_array", NORMAL_ARRAY)
	_material.set_shader_parameter("macro_tex", MACRO)
	_material.set_shader_parameter("layer_tile", PackedFloat32Array(LAYER_TILE_M))
	# Water level per sample (prompt 014): wet band above the waterline.
	var levels: PackedFloat32Array = host.water_levels()
	if levels.size() == _samples.x * _samples.y:
		var water := Image.create_from_data(_samples.x, _samples.y, false, Image.FORMAT_RF, levels.to_byte_array())
		_material.set_shader_parameter("water_tex", ImageTexture.create_from_image(water))
		_material.set_shader_parameter("has_water", true)

	var bounds := _map_bounds()
	for level in LOD_LEVELS:
		var node := MultiMeshInstance3D.new()
		node.name = "Lod%d" % level
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.use_custom_data = true
		multimesh.mesh = _grid_mesh(level)
		multimesh.instance_count = _chunks.x * _chunks.y
		multimesh.visible_instance_count = 0
		node.multimesh = multimesh
		node.material_override = _material
		# The shader moves the vertices; culling happens per chunk here.
		node.custom_aabb = bounds
		add_child(node)
		_lod_nodes.append(node)
		var buffer := PackedFloat32Array()
		buffer.resize(multimesh.instance_count * INSTANCE_FLOATS)
		_buffers.append(buffer)
	visible_per_lod.resize(LOD_LEVELS)

	_edge_material = ShaderMaterial.new()
	_edge_material.shader = EDGE_SHADER
	_edge_material.set_shader_parameter("macro_tex", MACRO)
	_edge_material.set_shader_parameter("albedo_array", ALBEDO_ARRAY)
	_edge = MeshInstance3D.new()
	_edge.name = "Edge"
	add_child(_edge)
	_build_edge()
	if _biome:
		apply_biome(_biome)
	_apply_relief()
	_dirty = true
	print("terrain: map '%s', %d x %d samples, %d x %d chunks of %d m, built in %d ms" % [
		host.map_name(), _samples.x, _samples.y, _chunks.x, _chunks.y,
		int(CHUNK_CELLS * _cell), (Time.get_ticks_usec() - started) / 1000
	])


func _process(_delta: float) -> void:
	if sim_host == null:
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var moved := not camera.global_transform.is_equal_approx(_last_camera) or camera.fov != _last_fov
	if moved or _dirty:
		update_chunks(camera)


## Assigns LOD levels, culls against the camera frustum and fills the
## MultiMesh buffers.
func update_chunks(camera: Camera3D) -> void:
	var started := Time.get_ticks_usec()
	_last_camera = camera.global_transform
	_last_fov = camera.fov
	_dirty = false
	var eye := camera.global_position
	var planes := camera.get_frustum()
	var chunk_m := CHUNK_CELLS * _cell
	var half := _map_size * 0.5
	var nx := _chunks.x
	var ny := _chunks.y
	# LOD of every chunk from the distance between camera and chunk box.
	for cy in ny:
		var z0 := cy * chunk_m - half.y
		var dz := maxf(maxf(z0 - eye.z, eye.z - (z0 + chunk_m)), 0.0)
		for cx in nx:
			var k := cy * nx + cx
			var x0 := cx * chunk_m - half.x
			var dx := maxf(maxf(x0 - eye.x, eye.x - (x0 + chunk_m)), 0.0)
			var range_y := _ranges[k]
			var dy := maxf(maxf(range_y.x - eye.y, eye.y - range_y.y), 0.0)
			var dist := sqrt(dx * dx + dy * dy + dz * dz)
			if _lods_fresh or lod_hysteresis <= 0.0:
				_lods[k] = _level_at(dist)
			else:
				_lods[k] = clampi(_lods[k], _level_at(dist / (1.0 + lod_hysteresis)), _level_at(dist * (1.0 + lod_hysteresis)))
	_lods_fresh = false

	# Visible chunks per level.
	# Plain Arrays (references): appending to an element changes it in place.
	var lists: Array[Array] = []
	for level in LOD_LEVELS:
		lists.append([])
	for cy in ny:
		var z0 := cy * chunk_m - half.y
		for cx in nx:
			var k := cy * nx + cx
			var x0 := cx * chunk_m - half.x
			var range_y := _ranges[k]
			var lo := Vector3(x0 - cull_margin, range_y.x - cull_margin, z0 - cull_margin)
			var hi := Vector3(x0 + chunk_m + cull_margin, range_y.y + cull_margin, z0 + chunk_m + cull_margin)
			if _box_in_frustum(planes, lo, hi):
				lists[_lods[k]].append(k)

	var counts := PackedInt32Array()
	counts.resize(LOD_LEVELS)
	for level in LOD_LEVELS:
		var stride := 1 << level
		# Packed arrays are values in GDScript: fill a local one, store it back.
		var buffer := _buffers[level]
		var o := 0
		for k: int in lists[level]:
			var cx := k % nx
			var cy := k / nx
			# Transform rows (basis identity, origin at the chunk corner).
			buffer[o] = 1.0
			buffer[o + 1] = 0.0
			buffer[o + 2] = 0.0
			buffer[o + 3] = cx * chunk_m - half.x
			buffer[o + 4] = 0.0
			buffer[o + 5] = 1.0
			buffer[o + 6] = 0.0
			buffer[o + 7] = 0.0
			buffer[o + 8] = 0.0
			buffer[o + 9] = 0.0
			buffer[o + 10] = 1.0
			buffer[o + 11] = cy * chunk_m - half.y
			# Neighbor strides: north (-z), east (+x), south (+z), west (-x).
			buffer[o + 12] = (1 << _lods[k - nx]) if cy > 0 else stride
			buffer[o + 13] = (1 << _lods[k + 1]) if cx < nx - 1 else stride
			buffer[o + 14] = (1 << _lods[k + nx]) if cy < ny - 1 else stride
			buffer[o + 15] = (1 << _lods[k - 1]) if cx > 0 else stride
			o += INSTANCE_FLOATS
		_buffers[level] = buffer
		var multimesh := _lod_nodes[level].multimesh
		multimesh.buffer = buffer
		multimesh.visible_instance_count = lists[level].size()
		counts[level] = lists[level].size()
	visible_per_lod = counts
	last_update_usec = Time.get_ticks_usec() - started


## Re-reads the samples in `rect` (sample coordinates) from the SimHost and
## uploads them. Normals and blurred rasters reach a few samples beyond a
## height change, so the rectangle is grown before reading.
## LOD level of a chunk `dist` meters from the camera, without hysteresis.
func _level_at(dist: float) -> int:
	if dist < lod0_distance:
		return 0
	return mini(LOD_LEVELS - 1, int(log(dist / lod0_distance) / log(2.0)) + 1)


## LOD level of every chunk in the last update (for tests).
func chunk_lods() -> PackedInt32Array:
	return _lods


## The next update takes every level from the distance alone, as after a
## jump of the camera (tests and pictures: the same from wherever it came).
func reset_lods() -> void:
	_lods_fresh = true
	_dirty = true


func heights_changed(rect: Rect2i) -> void:
	regions_changed([rect])


## Like `heights_changed` for several rectangles (craters, prompt 021): each
## is re-read and blitted, the textures are uploaded once.
func regions_changed(rects: Array[Rect2i]) -> void:
	var edge := false
	var read: Array[Rect2i] = []
	for rect in rects:
		var grown := rect.grow(8)
		var layers: Dictionary = sim_host.terrain_layers(grown.position.x, grown.position.y, grown.size.x, grown.size.y)
		if layers.is_empty():
			continue
		var clipped: Rect2i = layers["rect"]
		var w := clipped.size.x
		var h := clipped.size.y
		var parts := {
			"height": Image.create_from_data(w, h, false, Image.FORMAT_RF, (layers["height"] as PackedFloat32Array).to_byte_array()),
			"normal": Image.create_from_data(w, h, false, Image.FORMAT_RG8, layers["normal"]),
			"splat0": Image.create_from_data(w, h, false, Image.FORMAT_RGBA8, layers["splat0"]),
			"splat1": Image.create_from_data(w, h, false, Image.FORMAT_RGBA8, layers["splat1"]),
			"track": Image.create_from_data(w, h, false, Image.FORMAT_RGBA8, layers["track"]),
			"field": Image.create_from_data(w, h, false, Image.FORMAT_RGBA8, layers["field"]),
		}
		for key: String in parts:
			_images[key].blit_rect(parts[key], Rect2i(Vector2i.ZERO, clipped.size), clipped.position)
		edge = edge or clipped.position.x == 0 or clipped.position.y == 0 \
			or clipped.end.x >= _samples.x or clipped.end.y >= _samples.y
		read.append(clipped)
	if read.is_empty():
		return
	for key: String in _images:
		if _textures.has(key):
			_textures[key].update(_images[key])
	# Chunk height ranges: only the chunks under the rectangles where the
	# host can (the whole map took 11 ms on test_large).
	if sim_host.has_method("terrain_chunk_ranges_in"):
		for rect in read:
			for r: Vector3 in sim_host.terrain_chunk_ranges_in(CHUNK_CELLS, rect):
				_ranges[int(r.x)] = Vector2(r.y, r.z)
	else:
		_ranges = sim_host.terrain_chunk_ranges(CHUNK_CELLS)
	if edge:
		_build_edge()
	region_updates += 1
	_dirty = true


## Presses a presentation-only test crater into the ground at `point` and
## rebuilds the affected chunks (debug menu, self-test).
func debug_crater(point: Vector3, radius := 6.0, depth := 1.5) -> void:
	var rect: Rect2i = sim_host.debug_dent(point.x, point.z, radius, depth)
	if rect.has_area():
		heights_changed(rect)


## Re-reads the water levels of the wet band (map editor, prompt 028: after
## painting water or digging a river).
func refresh_water() -> void:
	var levels: PackedFloat32Array = sim_host.water_levels()
	var has_water := levels.size() == _samples.x * _samples.y
	if has_water:
		var water := Image.create_from_data(_samples.x, _samples.y, false, Image.FORMAT_RF, levels.to_byte_array())
		_material.set_shader_parameter("water_tex", ImageTexture.create_from_image(water))
	_material.set_shader_parameter("has_water", has_water)


## Displayed height at a sample (meters), from the uploaded raster.
func displayed_height(i: int, j: int) -> float:
	return _images["height"].get_pixel(i, j).r


## Sample coordinates of a world point.
func sample_of(point: Vector3) -> Vector2i:
	return Vector2i(
		roundi((point.x + _map_size.x * 0.5) / _cell), roundi((point.z + _map_size.y * 0.5) / _cell)
	)


## Ground colors of a biome (prompt 015): texture layer per material layer,
## tint, snow cover, dry/lush variation and the map edge. `preset` is a
## BiomePreset (res://assets/biomes/biome_preset.gd). Before `build` it is
## kept and applied when the materials exist.
func apply_biome(preset: Resource) -> void:
	_biome = preset
	if _material == null or preset == null:
		return
	_material.set_shader_parameter("layer_source", preset.layer_source)
	var tints := PackedColorArray(preset.layer_tint)
	tints.resize(LAYER_TILE_M.size())
	for i in tints.size():
		if tints[i] == Color(0, 0, 0, 0):
			tints[i] = Color.WHITE
	_material.set_shader_parameter("layer_tint", tints)
	_material.set_shader_parameter("use_biome", true)
	_material.set_shader_parameter("snow_cover", preset.snow_cover)
	_material.set_shader_parameter("grass_variation", preset.grass_variation)
	_material.set_shader_parameter("meadow_strength", preset.meadow_strength)
	_material.set_shader_parameter("flower_density", preset.flower_density)
	# Gusts over tall grass (prompt 073) follow the biome's wind.
	_material.set_shader_parameter("wind_strength", preset.wind_strength)
	_material.set_shader_parameter("wind_direction", preset.wind_direction)
	_material.set_shader_parameter("wind_speed", preset.wind_speed)
	TallGrass.apply_look(_material, preset.env_biome)
	_edge_material.set_shader_parameter("top_color", preset.edge_top)
	_edge_material.set_shader_parameter("soil_tint", preset.edge_tint)
	_apply_relief()


## Tall grass (prompt 073, called by map_objects.gd): palette of the
## EnvLook biome and the distances of the quality level, the same as on the
## plants; `range_m` 0 draws tall grass on the ground at every distance.
func set_tall_grass(env_biome: String, level: String, range_m: float) -> void:
	if _material == null:
		return
	TallGrass.apply_look(_material, env_biome)
	TallGrass.apply_quality(_material, level, range_m)


## Track raster of tall grass (prompt 073, world/grass_trails.gd): pressed
## flat (R) and burnt (G), over the world rectangle `origin`, `size`.
func set_trails(texture: Texture2D, origin: Vector2, size: Vector2) -> void:
	if _material == null:
		return
	_material.set_shader_parameter("trail_tex", texture)
	_material.set_shader_parameter("trail_origin", origin)
	_material.set_shader_parameter("trail_size", size)


## Vehicle tracks (prompt 104, world/ground_tracks.gd): ruts (R), churned
## (G) and worn ground (B) in the window `origin`, `size` (m); no texture:
## no tracks.
func set_ground_tracks(texture: Texture2D, origin: Vector2, size: Vector2) -> void:
	if _material == null:
		return
	_material.set_shader_parameter("ground_track_tex", texture)
	_material.set_shader_parameter("ground_track_origin", origin)
	_material.set_shader_parameter("ground_track_size", size)
	_material.set_shader_parameter("has_ground_tracks", texture != null)


## Shows tall grass as tall grass (1) or as plain meadow (0, benchmark A/B).
func set_tall_strength(value: float) -> void:
	if _material != null:
		_material.set_shader_parameter("tall_strength", value)


## Mountain look (prompt 060): scree and a snow line on maps whose height
## range exceeds `MOUNTAIN_RELIEF` (fully at its second value). The snow
## line lies at a fraction of the height range: `SNOW_LINE` for forest
## maps, `SNOW_LINE_WINTER` in winter, none in the desert [provisional].
const MOUNTAIN_RELIEF := Vector2(120.0, 220.0)
const SNOW_LINE := 0.8
const SNOW_LINE_WINTER := 0.45


func _apply_relief() -> void:
	if _material == null or _ranges.is_empty():
		return
	var low := INF
	var high := -INF
	for r in _ranges:
		low = minf(low, r.x)
		high = maxf(high, r.y)
	var relief := high - low
	var mountain := smoothstep(MOUNTAIN_RELIEF.x, MOUNTAIN_RELIEF.y, relief)
	_material.set_shader_parameter("mountain", mountain)
	var id: String = _biome.id if _biome else "forest"
	var line := 1.0e6
	if mountain > 0.0 and id != "desert":
		line = low + relief * (SNOW_LINE_WINTER if id == "winter" else SNOW_LINE)
	_material.set_shader_parameter("snow_line", line)


## Rain-soaked ground (prompt 040, world/weather_look.gd), 0..1.
func set_wetness(value: float) -> void:
	if _material:
		_material.set_shader_parameter("wetness", value)


## Current value of a terrain shader parameter (for the self-test).
func shader_parameter(name: StringName) -> Variant:
	return _material.get_shader_parameter(name) if _material else null


func set_show_lod(enabled: bool) -> void:
	_material.set_shader_parameter("show_lod", enabled)


## Flat grid of one chunk at a LOD level: vertices in meters relative to the
## chunk corner, UV = grid index, UV2 = (stride in samples, quads per side).
func _grid_mesh(level: int) -> ArrayMesh:
	var stride := 1 << level
	var quads := CHUNK_CELLS / stride
	var side := quads + 1
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var normals := PackedVector3Array()
	vertices.resize(side * side)
	uvs.resize(side * side)
	uv2s.resize(side * side)
	normals.resize(side * side)
	for j in side:
		for i in side:
			var k := j * side + i
			vertices[k] = Vector3(i * stride * _cell, 0.0, j * stride * _cell)
			uvs[k] = Vector2(i, j)
			uv2s[k] = Vector2(stride, quads)
			normals[k] = Vector3.UP
	var indices := PackedInt32Array()
	indices.resize(quads * quads * 6)
	var n := 0
	for j in quads:
		for i in quads:
			var a := j * side + i
			var b := a + 1
			var c := a + side
			var d := c + 1
			# Clockwise seen from above: Godot's front faces. Alternate the
			# diagonal so slopes do not get a visible grain.
			if (i + j) % 2 == 0:
				indices[n] = a; indices[n + 1] = b; indices[n + 2] = c
				indices[n + 3] = b; indices[n + 4] = d; indices[n + 5] = c
			else:
				indices[n] = a; indices[n + 1] = b; indices[n + 2] = d
				indices[n + 3] = a; indices[n + 4] = d; indices[n + 5] = c
			n += 6
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var chunk_m := CHUNK_CELLS * _cell
	mesh.custom_aabb = AABB(Vector3(0, -10000, 0), Vector3(chunk_m, 20000, chunk_m))
	return mesh


## Box around the whole map including the edge cut.
func _map_bounds() -> AABB:
	var lo := INF
	var hi := -INF
	for r in _ranges:
		lo = minf(lo, r.x)
		hi = maxf(hi, r.y)
	var half := _map_size * 0.5
	return AABB(Vector3(-half.x, lo - edge_depth - 1.0, -half.y), Vector3(_map_size.x, hi - lo + edge_depth + 2.0, _map_size.y))


## Vertical cut along the four map edges, from the ground down to
## `edge_depth` below the lowest point of the map.
func _build_edge() -> void:
	var lo := INF
	for r in _ranges:
		lo = minf(lo, r.x)
	var bottom := lo - edge_depth
	_edge_material.set_shader_parameter("bottom", bottom)
	var half := _map_size * 0.5
	var heights := _images["height"]
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	# Each wall: samples along one edge, outward normal, walked so the faces
	# point outwards (clockwise seen from outside).
	var walls := [
		[Vector2i(0, 0), Vector2i(1, 0), _samples.x, Vector3.FORWARD],
		[Vector2i(_samples.x - 1, 0), Vector2i(0, 1), _samples.y, Vector3.RIGHT],
		[Vector2i(_samples.x - 1, _samples.y - 1), Vector2i(-1, 0), _samples.x, Vector3.BACK],
		[Vector2i(0, _samples.y - 1), Vector2i(0, -1), _samples.y, Vector3.LEFT],
	]
	for wall: Array in walls:
		var start: Vector2i = wall[0]
		var step: Vector2i = wall[1]
		var count: int = wall[2]
		var normal: Vector3 = wall[3]
		var base := vertices.size()
		for t in count:
			var s := start + step * t
			var top := heights.get_pixel(s.x, s.y).r
			var x := s.x * _cell - half.x
			var z := s.y * _cell - half.y
			vertices.append(Vector3(x, top, z))
			vertices.append(Vector3(x, bottom, z))
			normals.append(normal)
			normals.append(normal)
			uvs.append(Vector2(0.0, 0.0))
			uvs.append(Vector2(0.0, top - bottom))
		for t in count - 1:
			var a := base + t * 2
			indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.surface_set_material(0, _edge_material)
	_edge.mesh = mesh


## Whether the box lo..hi is (at least partly) inside all frustum planes.
## Godot's frustum planes point outwards.
static func _box_in_frustum(planes: Array[Plane], lo: Vector3, hi: Vector3) -> bool:
	for plane in planes:
		var n := plane.normal
		# Corner of the box farthest inside (against the normal).
		var p := Vector3(lo.x if n.x > 0.0 else hi.x, lo.y if n.y > 0.0 else hi.y, lo.z if n.z > 0.0 else hi.z)
		if n.dot(p) - plane.d > 0.0:
			return false
	return true
