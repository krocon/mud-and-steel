extends RefCounted
## Grass tufts of the meadow (prompt 057). Presentation only.
##
## Builds the tuft meshes that replace the grass clumps of prompt 013 in the
## forest biome (`map_objects.gd`, decoration layer "grass"): each tuft is a
## bunch of single blades, every blade a thin, curved, tapering strip of three
## opaque triangles (no alpha test, so early depth test works and nothing
## flickers at the edges of cards). Seen from the high camera the blades read
## as strokes in random directions with light tips over dark roots instead
## of the box outlines the crossed cards drew from above.
##
## Normals point mostly up (tilted a little along the blade), so blades are lit
## like the ground under them. Colors come from the shader
## (`assets/shaders/meadow_tuft.gdshader`): the meadow palette at the tuft's
## position. Vertex color: r = height on the blade, g = wind weight,
## b = random shade.
##
## Variants follow the 013 layer's variants: short ("Grass"), tall
## ("GrassTall") and a loose wide tuft ("Flowers": flowers are drawn on the
## ground by the terrain shader now).

const MATERIAL := preload("res://assets/materials/env/meadow_tuft.tres")

## Per variant: blades, height range (m), radius of the roots (m), lean range
## (radians from upright), blade width at the root (m), whether a blade is
## curved (three triangles) or straight (one), whether the tufts cast
## shadows, seed. Only the tall tufts are curved and cast shadows: shadows
## draw a tuft once per cascade, and short straight blades are a third of
## the triangles (prompt 016 budget).
const VARIANTS: Array[Dictionary] = [
	{"blades": 18, "height": Vector2(0.2, 0.42), "radius": 0.12, "lean": Vector2(0.35, 1.0), "width": 0.022,
		"curved": false, "shadow": false, "seed": 3},
	{"blades": 14, "height": Vector2(0.35, 0.62), "radius": 0.1, "lean": Vector2(0.25, 0.8), "width": 0.02,
		"curved": true, "shadow": true, "seed": 5},
	{"blades": 18, "height": Vector2(0.15, 0.32), "radius": 0.2, "lean": Vector2(0.6, 1.2), "width": 0.022,
		"curved": false, "shadow": false, "seed": 7},
]

static var _meshes: Array[Mesh] = []


## The tuft mesh of variant `index` (0 short, 1 tall, 2 wide), built once.
static func mesh(index: int) -> Mesh:
	if _meshes.is_empty():
		for v in VARIANTS:
			_meshes.append(_build(v))
	return _meshes[clampi(index, 0, _meshes.size() - 1)]


## Tufts are drawn up to this distance from the camera (m); beyond, the
## blades and colors of the ground shader carry the meadow.
const RANGE := 55.0
## Lattice spacing of the tufts (m); the 013 clumps used 0.7 m.
const SPACING := 0.55


## Whether tufts of variant `index` cast shadows.
static func casts_shadow(index: int) -> bool:
	return bool(VARIANTS[clampi(index, 0, VARIANTS.size() - 1)].shadow)


## Whether the meadow tufts replace the 013 grass clumps for this EnvLook
## biome. Only the temperate biome has a meadow; desert and winter keep the
## clumps, which follow their dry and snowy looks.
static func active(env_biome: String) -> bool:
	return env_biome == "forest"


static func _build(v: Dictionary) -> ArrayMesh:
	var rng := RandomNumberGenerator.new()
	rng.seed = int(v.seed)
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var indices := PackedInt32Array()
	var height: Vector2 = v.height
	var lean_range: Vector2 = v.lean
	for b in int(v.blades):
		var angle := rng.randf() * TAU
		var r := sqrt(rng.randf()) * float(v.radius)
		var out := Vector3(cos(angle), 0.0, sin(angle))
		var root := out * r
		var h := rng.randf_range(height.x, height.y)
		var lean := rng.randf_range(lean_range.x, lean_range.y)
		# Blades lean outwards (a little off the radial direction) and bend
		# more towards the tip.
		var lean_dir := out.rotated(Vector3.UP, rng.randf_range(-0.6, 0.6))
		var side := lean_dir.cross(Vector3.UP).normalized()
		var width := float(v.width) * rng.randf_range(0.75, 1.25)
		var shade := rng.randf()
		var points: Array[Vector3] = []
		for k in 3:
			var t := k / 2.0
			var bend := lean * (0.4 + 0.6 * t)
			points.append(root + lean_dir * sin(bend) * h * t + Vector3.UP * cos(bend) * h * t)
		var base := verts.size()
		# Root pair, middle pair (curved blades only), then the tip.
		var pairs := 2 if v.curved else 1
		for k in pairs:
			var w := width * (1.0 - 0.45 * k)
			for s in [-1.0, 1.0]:
				verts.append(points[k] + side * w * 0.5 * s)
				colors.append(Color(k * 0.5, k * 0.5, shade))
		verts.append(points[2])
		colors.append(Color(1.0, 1.0, shade))
		# Lit like the ground: up, tilted a little along the blade.
		var n := (Vector3.UP * 0.85 + lean_dir * 0.35).normalized()
		for k in pairs * 2 + 1:
			normals.append(n)
		if v.curved:
			indices.append_array([base, base + 2, base + 1, base + 1, base + 2, base + 3, base + 2, base + 4, base + 3])
		else:
			indices.append_array([base, base + 2, base + 1])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.surface_set_material(0, MATERIAL)
	return mesh
