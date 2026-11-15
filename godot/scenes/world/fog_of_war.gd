extends Node3D
## Fog of war (prompt 018). Presentation only.
##
## The simulation decides what the local player sees (`SimHost.vision_image`:
## one byte per 8 m vision cell, 0 never seen, 128 explored, 255 visible).
## This node draws it as a screen-space overlay: a quad in front of the
## camera reads the depth buffer, reconstructs the world position of every
## pixel and darkens it by the state of its cell. One shader covers terrain,
## vegetation, map objects, units and water without touching their
## materials. The texture is filtered linearly and sampled five times, so
## the edges are soft. Enemy units out of sight are not in the snapshot at
## all; `main.gd` hides their nodes.

const SHADER_CODE := """
shader_type spatial;
render_mode unshaded, depth_test_disabled, depth_draw_never, cull_disabled, blend_mix, fog_disabled, shadows_disabled;

uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;
uniform sampler2D fog_map : filter_linear, repeat_disable;
/// World x/z of the grid's corner and its size in meters.
uniform vec2 fog_origin;
uniform vec2 fog_size = vec2(1.0);
/// Cell size in uv, for the soft-edge taps.
uniform vec2 texel = vec2(0.01);
uniform vec4 unexplored_color : source_color = vec4(0.02, 0.025, 0.03, 0.86);
uniform vec4 explored_color : source_color = vec4(0.04, 0.05, 0.07, 0.5);

void vertex() {
	POSITION = vec4(VERTEX.xy, 1.0, 1.0);
}

float cell_state(vec2 uv) {
	if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {
		return 0.0;
	}
	return texture(fog_map, uv).r;
}

void fragment() {
	float depth = texture(depth_texture, SCREEN_UV).x;
	if (depth <= 0.0) {
		discard; // sky (reverse z: the far plane is 0)
	}
	vec4 world = INV_VIEW_MATRIX * INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	vec2 uv = (world.xz / world.w - fog_origin) / fog_size;
	float v = cell_state(uv) * 0.4;
	v += cell_state(uv + vec2(texel.x, 0.0) * 0.6) * 0.15;
	v += cell_state(uv - vec2(texel.x, 0.0) * 0.6) * 0.15;
	v += cell_state(uv + vec2(0.0, texel.y) * 0.6) * 0.15;
	v += cell_state(uv - vec2(0.0, texel.y) * 0.6) * 0.15;
	float visible = smoothstep(0.7, 0.98, v);
	float explored = smoothstep(0.15, 0.5, v);
	vec4 color = mix(unexplored_color, explored_color, explored);
	ALBEDO = color.rgb;
	ALPHA = color.a * (1.0 - visible);
}
"""

## Whether the overlay is drawn (debug switch; the simulation is unaffected).
var enabled := true
## Texture updates so far (self-test).
var updates := 0

var _host: SimHost
var _quad: MeshInstance3D
var _material: ShaderMaterial
var _image: Image
var _texture: ImageTexture
var _size := Vector2i.ZERO
var _version := 0


func setup(host: SimHost, camera: Camera3D) -> void:
	_host = host
	_size = host.vision_grid_size()
	if _size.x <= 0 or _size.y <= 0:
		return
	var shader := Shader.new()
	shader.code = SHADER_CODE
	_material = ShaderMaterial.new()
	_material.shader = shader
	# After every other transparent surface (water, tracers).
	_material.render_priority = Material.RENDER_PRIORITY_MAX
	var cell := host.vision_cell_size()
	_material.set_shader_parameter("fog_origin", host.vision_origin())
	_material.set_shader_parameter("fog_size", Vector2(_size) * cell)
	_material.set_shader_parameter("texel", Vector2(1.0 / _size.x, 1.0 / _size.y))
	var mesh := QuadMesh.new()
	mesh.size = Vector2(2, 2)
	mesh.material = _material
	_quad = MeshInstance3D.new()
	_quad.name = "FogOverlay"
	_quad.mesh = mesh
	_quad.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Never culled: the vertex shader puts it on the screen.
	_quad.custom_aabb = AABB(Vector3(-1e6, -1e6, -1e6), Vector3(2e6, 2e6, 2e6))
	camera.add_child(_quad)
	_version = host.vision_version() - 1
	_refresh()


func _process(_delta: float) -> void:
	_refresh()


## Uploads the cell states when the simulation reports a change.
func _refresh() -> void:
	if _host == null or _quad == null:
		return
	_quad.visible = enabled and not _host.is_reveal_all()
	var version := _host.vision_version()
	if version == _version:
		return
	_version = version
	var bytes := _host.vision_image()
	if bytes.size() != _size.x * _size.y:
		return
	if _image == null:
		_image = Image.create_from_data(_size.x, _size.y, false, Image.FORMAT_L8, bytes)
		_texture = ImageTexture.create_from_image(_image)
		_material.set_shader_parameter("fog_map", _texture)
	else:
		_image.set_data(_size.x, _size.y, false, Image.FORMAT_L8, bytes)
		_texture.update(_image)
	updates += 1


## Share of cells with the given state byte (0, 128, 255), for tests.
func share_of(state: int) -> float:
	if _host == null:
		return 0.0
	var bytes := _host.vision_image()
	if bytes.is_empty():
		return 0.0
	return float(bytes.count(state)) / bytes.size()


## The state byte of the cell containing world point `p` (x, z).
func state_at(p: Vector3) -> int:
	if _host == null or _size.x <= 0:
		return -1
	var cell := _host.vision_cell_size()
	var origin := _host.vision_origin()
	var c := clampi(int(floor((p.x - origin.x) / cell)), 0, _size.x - 1)
	var r := clampi(int(floor((p.z - origin.y) / cell)), 0, _size.y - 1)
	return _host.vision_image()[r * _size.x + c]


## Reveals everything (debug) or shows the fog again.
func set_fog_enabled(on: bool) -> void:
	enabled = on
	_host.set_reveal_all(not on)
	_refresh()
