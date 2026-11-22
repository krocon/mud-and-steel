extends RefCounted
## Light streaks (prompt 199): shells and tracers in flight are drawn as
## light, a point or a short line along the flight, not as a body
## (`light_streak.gdshader`). Used by `artillery_view.gd` (indirect shells,
## with a faint vapour trail) and `combat_view.gd` (tracers of direct fire).
## Presentation only.

const SHADER := preload("res://assets/shaders/light_streak.gdshader")
## Pixels (at 1080 lines) beyond the real width a streak may take on
## screen: the culling box must not drop it near the edge of the view.
const CULL_MARGIN_M := 6.0
## Shortest streak in meters (a paused shell is a point, but the quad
## needs a length to keep a valid basis).
const MIN_LENGTH_M := 0.01


## One quad shared by every streak.
static func mesh() -> QuadMesh:
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	return quad


static func material(color: Color, energy: float, min_px: float, max_px: float, tail_share: float) -> ShaderMaterial:
	var m := ShaderMaterial.new()
	m.shader = SHADER
	m.set_shader_parameter("color", color)
	m.set_shader_parameter("energy", energy)
	m.set_shader_parameter("min_px", min_px)
	m.set_shader_parameter("max_px", max_px)
	m.set_shader_parameter("tail_share", tail_share)
	return m


## A node for the pool: no shadow, a margin for the pixel width.
static func node(quad: Mesh, mat: Material) -> MeshInstance3D:
	var n := MeshInstance3D.new()
	n.mesh = quad
	n.material_override = mat
	n.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	n.extra_cull_margin = CULL_MARGIN_M
	return n


## Transform of the quad from `tail` to `head`, `width` meters wide.
static func transform(head: Vector3, tail: Vector3, width: float) -> Transform3D:
	var along := head - tail
	var length := along.length()
	var direction := along / length if length > 0.0001 else Vector3.FORWARD
	if length < MIN_LENGTH_M:
		along = direction * MIN_LENGTH_M
	var helper := Vector3.UP if absf(direction.y) < 0.99 else Vector3.RIGHT
	var side := direction.cross(helper).normalized()
	var normal := side.cross(direction).normalized()
	return Transform3D(Basis(side * width, along, normal * width), (head + tail) * 0.5)
