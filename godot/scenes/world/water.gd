extends Node3D
## Water surfaces of rivers and lakes (prompt 014). Presentation only.
##
## The simulation knows water only as terrain types; `SimHost.water_mesh()`
## derives the surface in Rust (crates/bridge/src/water.rs): water level from
## the banks, distance to the shore and to stones, flow velocity along the
## banks towards the map's `water_flow_deg`. This node turns it into one
## mesh with `water.gdshader` (flow-mapped normals, foam, refraction,
## absorption, sky reflection) and takes the water colors from the biome.

const SHADER := preload("res://assets/materials/water.gdshader")

## Vertices, triangles and build time of the water mesh (tools, self-test).
var vertex_count := 0
var triangle_count := 0
var build_msec := 0.0

var _mesh_instance: MeshInstance3D
var _material: ShaderMaterial


## Builds the water mesh of the map loaded in `host` (nothing without water).
## `host`: SimHost, or MapEditor in the map editor (prompt 028).
func build(host: Object) -> void:
	var started := Time.get_ticks_usec()
	var data: Dictionary = host.water_mesh()
	var vertices: PackedVector3Array = data.get("vertices", PackedVector3Array())
	var indices: PackedInt32Array = data.get("indices", PackedInt32Array())
	if indices.is_empty():
		return
	var normals := PackedVector3Array()
	normals.resize(vertices.size())
	normals.fill(Vector3.UP)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = data.uv
	arrays[Mesh.ARRAY_TEX_UV2] = data.uv2
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	_material = ShaderMaterial.new()
	_material.shader = SHADER
	_material.set_shader_parameter("normal_a", _noise_normal(11, 0.045, 5.0))
	_material.set_shader_parameter("normal_b", _noise_normal(23, 0.02, 2.5))
	_material.set_shader_parameter("foam_tex", _foam_noise())
	mesh.surface_set_material(0, _material)
	_mesh_instance = MeshInstance3D.new()
	_mesh_instance.name = "WaterSurface"
	_mesh_instance.mesh = mesh
	_mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_mesh_instance)
	vertex_count = vertices.size()
	triangle_count = indices.size() / 3
	build_msec = (Time.get_ticks_usec() - started) / 1000.0
	print("water: %d vertices, %d triangles, built in %.0f ms" % [vertex_count, triangle_count, build_msec])


## Water colors of a BiomePreset (prompt 015).
func apply_biome(preset: Resource) -> void:
	if _material == null or preset == null:
		return
	_material.set_shader_parameter("shallow_color", preset.water_shallow)
	_material.set_shader_parameter("deep_color", preset.water_deep)
	_material.set_shader_parameter("foam_color", preset.water_foam)
	_material.set_shader_parameter("clarity", preset.water_clarity)


## Shows or hides the water (debug menu).
func set_water_visible(on: bool) -> void:
	if _mesh_instance:
		_mesh_instance.visible = on


func shader_parameter(name: StringName) -> Variant:
	return _material.get_shader_parameter(name) if _material else null


## Seamless normal map from simplex noise (no texture files needed).
static func _noise_normal(seed_value: int, frequency: float, bump: float) -> NoiseTexture2D:
	var noise := FastNoiseLite.new()
	noise.seed = seed_value
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = frequency
	noise.fractal_octaves = 4
	var tex := NoiseTexture2D.new()
	tex.width = 256
	tex.height = 256
	tex.seamless = true
	tex.as_normal_map = true
	tex.bump_strength = bump
	tex.generate_mipmaps = true
	tex.noise = noise
	return tex


## Foam pattern: cellular noise, bright along the cell borders.
static func _foam_noise() -> NoiseTexture2D:
	var noise := FastNoiseLite.new()
	noise.seed = 5
	noise.noise_type = FastNoiseLite.TYPE_CELLULAR
	noise.cellular_return_type = FastNoiseLite.RETURN_DISTANCE2_SUB
	noise.frequency = 0.05
	noise.fractal_octaves = 2
	var tex := NoiseTexture2D.new()
	tex.width = 256
	tex.height = 256
	tex.seamless = true
	tex.generate_mipmaps = true
	tex.noise = noise
	return tex
