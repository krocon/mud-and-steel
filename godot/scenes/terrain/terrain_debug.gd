extends MeshInstance3D
## Coarse debug mesh of the simulation's height field, until the real terrain
## rendering of prompt 011 exists.
##
## Reads heights and terrain ids from SimHost (presentation only), builds one
## ArrayMesh with at most MAX_SAMPLES vertices per axis and colors every vertex
## by its terrain type. No collision, no physics: mouse picking runs against
## the height field in SimHost.ray_pick.

const SHADER := preload("res://assets/materials/terrain_debug.gdshader")
## Upper bound of vertices per axis (test_large: 2001 samples, stride 8).
const MAX_SAMPLES := 256


## Builds the mesh from the map loaded in `sim_host`.
func build(sim_host: SimHost) -> void:
	var full := sim_host.grid_samples(1)
	var stride := maxi(1, ceili(float(maxi(full.x, full.y) - 1) / (MAX_SAMPLES - 1)))
	var size := sim_host.grid_samples(stride)
	var heights := sim_host.height_grid(stride)
	var terrain := sim_host.terrain_grid(stride)
	var colors := sim_host.terrain_colors()
	var map_size := sim_host.map_size()
	var cell := sim_host.cell_size()

	var vertices := PackedVector3Array()
	var vertex_colors := PackedColorArray()
	vertices.resize(size.x * size.y)
	vertex_colors.resize(size.x * size.y)
	for j in size.y:
		# The last row and column sit on the map edge even if the stride does
		# not divide the sample count.
		var z := minf(j * stride, full.y - 1) * cell - map_size.y * 0.5
		for i in size.x:
			var k := j * size.x + i
			var x := minf(i * stride, full.x - 1) * cell - map_size.x * 0.5
			vertices[k] = Vector3(x, heights[k], z)
			vertex_colors[k] = colors[terrain[k]] if terrain[k] < colors.size() else Color.MAGENTA

	var indices := PackedInt32Array()
	indices.resize((size.x - 1) * (size.y - 1) * 6)
	var n := 0
	for j in size.y - 1:
		for i in size.x - 1:
			var a := j * size.x + i
			var b := a + 1
			var c := a + size.x
			var d := c + 1
			# Clockwise seen from above: Godot's front faces.
			indices[n] = a
			indices[n + 1] = b
			indices[n + 2] = c
			indices[n + 3] = b
			indices[n + 4] = d
			indices[n + 5] = c
			n += 6

	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_COLOR] = vertex_colors
	arrays[Mesh.ARRAY_INDEX] = indices
	arrays[Mesh.ARRAY_NORMAL] = _normals(vertices, size)
	var array_mesh := ArrayMesh.new()
	array_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var material := ShaderMaterial.new()
	material.shader = SHADER
	array_mesh.surface_set_material(0, material)
	mesh = array_mesh
	print("terrain debug mesh: %d x %d vertices (stride %d) for map '%s'" % [
		size.x, size.y, stride, sim_host.map_name()
	])


## Smooth vertex normals from central differences of the grid.
func _normals(vertices: PackedVector3Array, size: Vector2i) -> PackedVector3Array:
	var normals := PackedVector3Array()
	normals.resize(vertices.size())
	for j in size.y:
		for i in size.x:
			var left := vertices[j * size.x + maxi(i - 1, 0)]
			var right := vertices[j * size.x + mini(i + 1, size.x - 1)]
			var up := vertices[maxi(j - 1, 0) * size.x + i]
			var down := vertices[mini(j + 1, size.y - 1) * size.x + i]
			normals[j * size.x + i] = (down - up).cross(right - left).normalized()
	return normals
