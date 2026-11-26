extends Node3D
## Fog of war (prompts 018 and 098). Presentation only.
##
## The simulation decides what the local player sees (`SimHost.vision_image`:
## one byte per 8 m vision cell, 0 never seen, 128 explored, 255 visible).
## The whole map is known from the start (prompt 098): everything the local
## player does not see right now lies under one translucent layer (darker,
## cooler, less saturated); only what own and allied units see is clear.
## Never seen and explored look the same; the states stay for `state_at`
## and `share_of` (effects, sounds, tests).
##
## Drawn as two screen-space quads in front of the camera. Both read the
## depth buffer, reconstruct the world position of every pixel and look up
## its cell in the mask from `SimHost.vision_mask_fade` (255 visible, 0
## not; each cell fades over `FADE_SECONDS`, so the edge glides instead of
## jumping 8 m). The mask is sampled as a cubic B-spline and cut with a
## narrow smoothstep, so the edge is a round curve with a soft seam of a
## few meters and no cell corners.
## - `FogDesaturate`, first in the transparent pass: mixes the opaque scene
##   (screen texture) towards its own gray. Water, smoke and tracers come
##   later and are not desaturated (the screen texture does not have them).
## - `FogOverlay`, last: multiplies everything (water included) by
##   `DIM_TINT`. Order markers drawn after it stay bright
##   (`MARKER_PRIORITY`).
## Enemy units out of sight are not in the snapshot at all; `main.gd`
## hides their nodes.

## Color everything out of sight is multiplied by (sRGB): about 60 %
## brightness, a little cooler. The minimap uses the same values.
const DIM_TINT := Color(0.76, 0.76, 0.78)
## How far colors out of sight move towards gray (0 none, 1 gray).
const DESATURATION := 0.45
## Half width of the soft edge in mask units: 0.15 is about 3.5 m.
const EDGE_SOFTNESS := 0.15
## Seconds a cell takes to fade between seen and not seen.
const FADE_SECONDS := 0.3
## Cells of "not seen" around the grid, so the map edge fades like the rest.
const BORDER := 2
## Render priority of the dimming quad; order markers that must stay bright
## above the layer use `MARKER_PRIORITY`.
const DIM_PRIORITY := 100
const MARKER_PRIORITY := 110

const DESATURATE_SHADER: Shader = preload("res://assets/shaders/fog_desaturate.gdshader")
const DIM_SHADER: Shader = preload("res://assets/shaders/fog_dim.gdshader")

## Whether the layer is drawn (debug switch; the simulation is unaffected).
var enabled := true
## Mask uploads so far (self-test).
var updates := 0

var _host: SimHost
## The dimming quad (`FogOverlay`) and the desaturating one.
var _quad: MeshInstance3D
var _desaturate_quad: MeshInstance3D
var _materials: Array[ShaderMaterial] = []
var _image: Image
var _texture: ImageTexture
var _size := Vector2i.ZERO
var _version := 0
## Cell states of the last read, and the grid's cell size and origin.
var _bytes := PackedByteArray()
var _cell := 0.0
var _origin := Vector2.ZERO


func setup(host: SimHost, camera: Camera3D) -> void:
	_host = host
	_size = host.vision_grid_size()
	if _size.x <= 0 or _size.y <= 0:
		return
	_cell = host.vision_cell_size()
	_origin = host.vision_origin()
	_desaturate_quad = _make_quad("FogDesaturate", DESATURATE_SHADER, Material.RENDER_PRIORITY_MIN, camera)
	_quad = _make_quad("FogOverlay", DIM_SHADER, DIM_PRIORITY, camera)
	for material in _materials:
		material.set_shader_parameter("fog_origin", _origin - Vector2(BORDER, BORDER) * _cell)
		material.set_shader_parameter("fog_size", Vector2(padded_size()) * _cell)
		material.set_shader_parameter("fog_texels", Vector2(padded_size()))
		material.set_shader_parameter("edge_softness", EDGE_SOFTNESS)
	_materials[0].set_shader_parameter("desaturation", DESATURATION)
	_materials[1].set_shader_parameter("dim_tint", DIM_TINT)
	_version = host.vision_version() - 1
	_refresh(0.0)


func _make_quad(quad_name: String, shader: Shader, priority: int, camera: Camera3D) -> MeshInstance3D:
	var material := ShaderMaterial.new()
	material.shader = shader
	material.render_priority = priority
	_materials.append(material)
	var mesh := QuadMesh.new()
	mesh.size = Vector2(2, 2)
	mesh.material = material
	var quad := MeshInstance3D.new()
	quad.name = quad_name
	quad.mesh = mesh
	quad.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Never culled: the vertex shader puts it on the screen.
	quad.custom_aabb = AABB(Vector3(-1e6, -1e6, -1e6), Vector3(2e6, 2e6, 2e6))
	camera.add_child(quad)
	return quad


func _process(delta: float) -> void:
	_refresh(delta)


## Shows or hides the layer and uploads the mask while it changes.
func _refresh(delta: float) -> void:
	if _host == null or _quad == null:
		return
	var shown := enabled and not _host.is_reveal_all()
	_quad.visible = shown
	_desaturate_quad.visible = shown
	var bytes := _host.vision_mask_fade(delta, FADE_SECONDS, BORDER)
	var size := padded_size()
	if bytes.size() != size.x * size.y:
		return
	if _image == null:
		_image = Image.create_from_data(size.x, size.y, false, Image.FORMAT_L8, bytes)
		_texture = ImageTexture.create_from_image(_image)
		for material in _materials:
			material.set_shader_parameter("fog_map", _texture)
	else:
		_image.set_data(size.x, size.y, false, Image.FORMAT_L8, bytes)
		_texture.update(_image)
	updates += 1


## Mask texels per axis: the vision grid plus `BORDER` cells on each side.
func padded_size() -> Vector2i:
	return _size + Vector2i(BORDER, BORDER) * 2


## The mask as a texture (L8, one texel per vision cell plus the border:
## 255 seen, 0 not), shared with the minimap (prompt 031); null before the
## first upload. `fog_uv(grid_uv)` maps grid uv into it.
func fog_texture() -> ImageTexture:
	return _texture


## Scale (x, y) and offset (z, w) from uv over the vision grid to uv in the
## padded mask.
func fog_uv_transform() -> Vector4:
	var padded := Vector2(padded_size())
	var scale := Vector2(_size) / padded
	var offset := Vector2(BORDER, BORDER) / padded
	return Vector4(scale.x, scale.y, offset.x, offset.y)


## Share of cells with the given state byte (0, 128, 255), for tests.
func share_of(state: int) -> float:
	if _host == null:
		return 0.0
	var bytes := _host.vision_image()
	if bytes.is_empty():
		return 0.0
	return float(bytes.count(state)) / bytes.size()


## The state byte of the cell containing world point `p` (x, z). Reads the
## cached cell states: effects ask this for every vehicle and
## every shot, and fetching the whole image from the bridge each time cost
## 40 ms per frame with 250 tanks (prompt 016).
func state_at(p: Vector3) -> int:
	if _host == null or _size.x <= 0:
		return -1
	var version := _host.vision_version()
	if version != _version:
		_version = version
		_bytes = _host.vision_image()
	if _bytes.size() != _size.x * _size.y or _cell <= 0.0:
		return -1
	var c := int(floor((p.x - _origin.x) / _cell))
	var r := int(floor((p.z - _origin.y) / _cell))
	if c < 0 or c >= _size.x or r < 0 or r >= _size.y:
		return 0
	return _bytes[r * _size.x + c]


## Reveals everything (debug) or shows the layer again.
func set_fog_enabled(on: bool) -> void:
	enabled = on
	_host.set_reveal_all(not on)
	_refresh(0.0)
