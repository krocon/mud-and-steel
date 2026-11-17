extends RefCounted
## Tall grass (prompt 073). Presentation only.
##
## Builds the plants of the terrain type `tall_grass` (`data/terrain.toml`,
## id 8), drawn as two decoration layers of `map_objects.gd`: "tall_grass"
## (grass tufts, herbs, rosettes on a dense lattice, frayed at the edges of
## the stand) and "tall_shrubs" (shrubs with ocher leaves and seed heads in
## loose groups). Like the meadow tufts of prompt 057 every blade and leaf
## is a set of opaque triangles: no cards, no alpha test.
##
## Beyond `near_range` of the quality level the shader drops blades by
## their rank (UV2.y) until only `FAR_SHARE` of them are left at `switch`,
## and widens the rest, so no blade is drawn thinner than about half a
## pixel. From `switch` on the squares draw the far mesh, which holds only
## those blades: the swap shows nothing, and far plants cost a third of the
## vertices. Beyond `range` the terrain
## shader draws tall grass itself
## (`assets/materials/terrain_tall_grass.gdshaderinc`). The tufts shrink
## into the ground towards that distance, so nothing pops.
##
## Vertex data for the shader (`assets/shaders/tall_grass.gdshader`):
## COLOR r = height on the blade (0 root .. 1 tip), g = wind weight,
## b = random shade, a = kind (0 grass, 0.33 herb leaf, 0.67 shrub leaf,
## 1 seed head or stalk); UV = the blade's side direction times the side
## (-1, +1; 0 at the tip) in the mesh frame, UV2.x = half the blade width
## there (the shader widens blades that would be thinner than a pixel) and
## UV2.y = the blade's rank (0..1; blades above the share kept at a distance
## collapse).

const MATERIAL := preload("res://assets/materials/env/tall_grass.tres")

## Grass tufts: blades, height range (m), root radius (m), lean range
## (radians from upright at the root), droop (extra bend towards the tip),
## blade width at the root (m), seed.
const TUFTS: Array[Dictionary] = [
	# Tall, overhanging tuft (most of the stand).
	{"kind": "tuft", "blades": 22, "height": Vector2(1.0, 1.55), "radius": 0.22, "lean": Vector2(0.0, 0.22),
		"droop": Vector2(0.15, 1.0), "width": 0.05, "seed": 31},
	# Lower, denser tuft.
	{"kind": "tuft", "blades": 20, "height": Vector2(0.65, 1.1), "radius": 0.18, "lean": Vector2(0.05, 0.3),
		"droop": Vector2(0.25, 1.1), "width": 0.042, "seed": 37},
	# Broad-leaved herb: a stem with leaves.
	{"kind": "herb", "leaves": 10, "height": Vector2(0.6, 1.0), "leaf": Vector2(0.24, 0.08), "seed": 41},
	# Rosette of long arching fronds.
	{"kind": "rosette", "fronds": 11, "length": Vector2(0.7, 1.05), "width": 0.07, "seed": 43},
]
## Shrubs (own layer, in groups): woody stems with ocher leaves and seed
## heads above the grass.
const SHRUBS: Array[Dictionary] = [
	{"kind": "shrub", "stems": 9, "height": Vector2(1.6, 2.0), "leaves": 180, "seeds": 14, "seed": 53},
	{"kind": "shrub", "stems": 7, "height": Vector2(1.4, 1.75), "leaves": 140, "seeds": 18, "seed": 59},
]

## Per quality level: plants drawn up to `range` (m), all blades up to
## `near_range`, only the far mesh's blades from `switch` on, density factor
## of the tuft layer, and whether shrubs cast shadows.
const QUALITY: Dictionary[String, Dictionary] = {
	"low": {"range": 55.0, "near_range": 20.0, "switch": 35.0, "density": 0.75, "shadows": false},
	"medium": {"range": 65.0, "near_range": 28.0, "switch": 45.0, "density": 0.9, "shadows": true},
	"high": {"range": 80.0, "near_range": 35.0, "switch": 58.0, "density": 1.0, "shadows": true},
}
## Share of the blades (by rank) in the far mesh.
const FAR_SHARE := 0.35

## Look per EnvLook biome: palette (linear), stand density and height
## factors, snow on the tips, bent blades (winter: broken, brown).
const LOOKS: Dictionary[String, Dictionary] = {
	"forest": {"tip": Color(0.72, 0.8, 0.3), "mid": Color(0.3, 0.5, 0.14), "root": Color(0.03, 0.05, 0.018),
		"dry": Color(0.66, 0.62, 0.26), "herb": Color(0.36, 0.55, 0.16), "shrub": Color(0.5, 0.48, 0.2),
		"seed": Color(0.6, 0.6, 0.22), "density": 1.0, "height": 1.0, "snow": 0.0, "bent": 0.0},
	"desert": {"tip": Color(0.8, 0.7, 0.43), "mid": Color(0.56, 0.46, 0.25), "root": Color(0.16, 0.12, 0.07),
		"dry": Color(0.82, 0.74, 0.5), "herb": Color(0.34, 0.36, 0.17), "shrub": Color(0.5, 0.38, 0.2),
		"seed": Color(0.6, 0.48, 0.3), "density": 0.6, "height": 0.8, "snow": 0.0, "bent": 0.15},
	"winter": {"tip": Color(0.74, 0.6, 0.4), "mid": Color(0.52, 0.4, 0.25), "root": Color(0.2, 0.15, 0.09),
		"dry": Color(0.76, 0.64, 0.44), "herb": Color(0.45, 0.36, 0.22), "shrub": Color(0.42, 0.3, 0.18),
		"seed": Color(0.55, 0.42, 0.28), "density": 0.3, "height": 0.7, "snow": 0.65, "bent": 0.6},
}

static var _meshes: Dictionary[String, Mesh] = {}


## Near (all blades) or far mesh (the blades of rank below FAR_SHARE) of
## variant `index` of `TUFTS` (layer "tall_grass") or `SHRUBS` (layer
## "tall_shrubs"), built once.
static func mesh(layer: String, index: int, far := false) -> Mesh:
	var key := "%s:%d:%s" % [layer, index, far]
	if not _meshes.has(key):
		var specs: Array[Dictionary] = SHRUBS if layer == "tall_shrubs" else TUFTS
		_meshes[key] = _build(specs[clampi(index, 0, specs.size() - 1)], FAR_SHARE if far else 1.0)
	return _meshes[key]


## Variants of a layer.
static func variants(layer: String) -> int:
	return SHRUBS.size() if layer == "tall_shrubs" else TUFTS.size()


static func quality(level: String) -> Dictionary:
	return QUALITY.get(level, QUALITY["high"])


static func look(env_biome: String) -> Dictionary:
	return LOOKS.get(env_biome, LOOKS["forest"])


## Sets the biome palette on a material that includes
## `tall_grass_common.gdshaderinc` (the tufts and the terrain).
static func apply_look(material: ShaderMaterial, env_biome: String) -> void:
	var l := look(env_biome)
	for key: String in ["tip", "mid", "root", "dry", "herb", "shrub", "seed"]:
		material.set_shader_parameter("tall_" + key, l[key])
	material.set_shader_parameter("tall_snow", l.snow)
	material.set_shader_parameter("tall_bent", l.bent)
	material.set_shader_parameter("tall_height", l.height)


## Sets the distances of a quality level on a material that includes the
## common shader part, so tufts and terrain hand over at the same distance.
## `range_m` 0: no tufts, the terrain draws tall grass at every distance.
static func apply_quality(material: ShaderMaterial, level: String, range_m: float) -> void:
	var q := quality(level)
	material.set_shader_parameter("tall_range", range_m)
	material.set_shader_parameter("tall_near", q.near_range)
	material.set_shader_parameter("tall_switch", q.switch)
	material.set_shader_parameter("tall_far_share", FAR_SHARE)


# --- Mesh building ------------------------------------------------------------


## Collects the arrays of one mesh.
class Builder:
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var indices := PackedInt32Array()
	## Blades of a higher rank are left out (far mesh).
	var max_rank := 1.0

	## A tapering strip along `points` (root first) ending in a point: two
	## vertices per point but the last. `widths` per point (full width),
	## `kind` the COLOR.a code, `shade` COLOR.b, `height` the height that
	## counts as the top (COLOR.r = 1), `rank` UV2.y.
	func strip(points: Array[Vector3], widths: PackedFloat32Array, side: Vector3, normal: Vector3,
			kind: float, shade: float, height: float, rank: float) -> void:
		if rank > max_rank:
			return
		var n := points.size()
		var first := verts.size()
		var side2 := Vector2(side.x, side.z)
		for k in n:
			var t := clampf(points[k].y / maxf(height, 0.01), 0.0, 1.0)
			var color := Color(t, t, shade, kind)
			if k == n - 1:
				_vertex(points[k], normal, color, Vector2.ZERO, 0.0, rank)
				break
			for s: float in [-1.0, 1.0]:
				_vertex(points[k] + side * widths[k] * 0.5 * s, normal, color, side2 * s, widths[k] * 0.5, rank)
		for k in n - 2:
			var a := first + k * 2
			indices.append_array([a, a + 2, a + 1, a + 1, a + 2, a + 3])
		var last := first + (n - 2) * 2
		indices.append_array([last, last + 2, last + 1])

	func _vertex(at: Vector3, normal: Vector3, color: Color, side: Vector2, half_width: float, rank: float) -> void:
		verts.append(at)
		normals.append(normal)
		colors.append(color)
		uvs.append(side)
		uv2s.append(Vector2(half_width, rank))

	func mesh() -> ArrayMesh:
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = verts
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_COLOR] = colors
		arrays[Mesh.ARRAY_TEX_UV] = uvs
		arrays[Mesh.ARRAY_TEX_UV2] = uv2s
		arrays[Mesh.ARRAY_INDEX] = indices
		var m := ArrayMesh.new()
		m.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		m.surface_set_material(0, MATERIAL)
		return m


static func _build(spec: Dictionary, max_rank: float) -> ArrayMesh:
	var b := Builder.new()
	b.max_rank = max_rank
	var rng := RandomNumberGenerator.new()
	rng.seed = int(spec.seed)
	match String(spec.kind):
		"tuft":
			_tuft(b, rng, spec)
		"herb":
			_herb(b, rng, spec)
		"rosette":
			_rosette(b, rng, spec)
		"shrub":
			_shrub(b, rng, spec)
	return b.mesh()


## Points of a blade that leaves `root` tilted by `lean` towards `out` and
## bends further by `droop` towards its tip (overhanging): `segments` + 1
## points.
static func _arc(root: Vector3, out: Vector3, length: float, lean: float, droop: float, segments: int) -> Array[Vector3]:
	var points: Array[Vector3] = [root]
	var at := root
	var steps := 12
	var ds := length / steps
	for s in steps:
		var t := (s + 0.5) / steps
		var a := lean + droop * t * t
		at += out * sin(a) * ds + Vector3.UP * cos(a) * ds
		if (s + 1) % (steps / segments) == 0:
			points.append(at)
	return points


static func _tuft(b: Builder, rng: RandomNumberGenerator, spec: Dictionary) -> void:
	var height: Vector2 = spec.height
	var lean_range: Vector2 = spec.lean
	var droop_range: Vector2 = spec.droop
	var radius := float(spec.radius)
	# The tuft leans a little to one side as a whole.
	var common := Vector3(1.0, 0.0, 0.0).rotated(Vector3.UP, rng.randf() * TAU)
	for i in int(spec.blades):
		var angle := rng.randf() * TAU
		var r := sqrt(rng.randf()) * radius
		var length := rng.randf_range(height.x, height.y)
		var outer := r / radius
		# Outer blades lean and hang over more.
		var lean := rng.randf_range(lean_range.x, lean_range.y) * (0.5 + outer)
		var droop := rng.randf_range(droop_range.x, droop_range.y) * (0.6 + 0.6 * outer)
		var width := float(spec.width) * rng.randf_range(0.75, 1.3)
		var radial := Vector3(cos(angle), 0.0, sin(angle)).rotated(Vector3.UP, rng.randf_range(-0.5, 0.5))
		var out := (radial * 0.55 + common * 0.45).normalized()
		var root := Vector3(cos(angle), 0.0, sin(angle)) * r
		var points := _arc(root, out, length, lean, droop, 3)
		var widths := PackedFloat32Array()
		for k in points.size():
			widths.append(width * (1.0 - 0.8 * float(k) / (points.size() - 1)))
		var n := (Vector3.UP * 0.8 + out * 0.4).normalized()
		b.strip(points, widths, out.cross(Vector3.UP).normalized(), n, 0.0, rng.randf(), height.y, rng.randf())


static func _herb(b: Builder, rng: RandomNumberGenerator, spec: Dictionary) -> void:
	var height: Vector2 = spec.height
	var h := rng.randf_range(height.x, height.y)
	var leaf: Vector2 = spec.leaf
	# Stem (always kept).
	var stem: Array[Vector3] = [Vector3.ZERO, Vector3(0.02, h * 0.55, 0.0), Vector3(0.0, h, 0.03)]
	b.strip(stem, PackedFloat32Array([0.018, 0.012, 0.0]), Vector3.RIGHT, Vector3.UP, 0.33, 0.5, h, 0.0)
	var leaves := int(spec.leaves)
	for i in leaves:
		var t := 0.2 + 0.75 * float(i) / leaves
		var angle := i * 2.4 + rng.randf_range(-0.3, 0.3)
		var length := leaf.x * rng.randf_range(0.7, 1.2) * (1.15 - 0.5 * t)
		var width := leaf.y * rng.randf_range(0.8, 1.2)
		var out := Vector3(cos(angle), 0.0, sin(angle))
		var base := Vector3(0.0, h * t, 0.0)
		# A broad leaf: root, widest point, tip, drooping a little.
		var points: Array[Vector3] = [
			base, base + out * length * 0.45 + Vector3.UP * length * 0.12, base + out * length + Vector3.DOWN * length * 0.15,
		]
		b.strip(points, PackedFloat32Array([width * 0.2, width, 0.0]), out.cross(Vector3.UP).normalized(),
			(Vector3.UP * 0.9 + out * 0.3).normalized(), 0.33, rng.randf(), h, rng.randf())


static func _rosette(b: Builder, rng: RandomNumberGenerator, spec: Dictionary) -> void:
	var length_range: Vector2 = spec.length
	var fronds := int(spec.fronds)
	for i in fronds:
		var angle := TAU * float(i) / fronds + rng.randf_range(-0.2, 0.2)
		var length := rng.randf_range(length_range.x, length_range.y)
		var out := Vector3(cos(angle), 0.0, sin(angle))
		var points := _arc(Vector3.ZERO, out, length, rng.randf_range(0.35, 0.7), rng.randf_range(1.0, 1.6), 4)
		var widths := PackedFloat32Array()
		for k in points.size():
			var t := float(k) / (points.size() - 1)
			# Fronds: narrow at the root, widest in the middle.
			widths.append(float(spec.width) * (0.35 + 1.6 * t * (1.0 - t)))
		b.strip(points, widths, out.cross(Vector3.UP).normalized(), (Vector3.UP * 0.85 + out * 0.35).normalized(),
			0.33, rng.randf(), length * 0.7, rng.randf())


static func _shrub(b: Builder, rng: RandomNumberGenerator, spec: Dictionary) -> void:
	var height: Vector2 = spec.height
	var top := rng.randf_range(height.x, height.y)
	var tips: Array[Vector3] = []
	for i in int(spec.stems):
		var angle := rng.randf() * TAU
		var out := Vector3(cos(angle), 0.0, sin(angle))
		var points := _arc(out * 0.05, out, top * rng.randf_range(0.8, 1.05), rng.randf_range(0.2, 0.5),
			rng.randf_range(0.1, 0.3), 2)
		b.strip(points, PackedFloat32Array([0.025, 0.015, 0.0]), out.cross(Vector3.UP).normalized(), Vector3.UP,
			1.0, rng.randf(), top, 0.0)
		tips.append(points[points.size() - 1])
	# Leaves: a crown of small rhombi around the upper stems (about 0.4 m
	# across per stem), denser and lighter on top.
	for i in int(spec.leaves):
		var tip := tips[i % tips.size()]
		var t := rng.randf_range(0.5, 1.0)
		var spread := 0.55 * t
		var at := tip * t + Vector3(rng.randf_range(-spread, spread), rng.randf_range(-0.12, 0.06),
			rng.randf_range(-spread, spread))
		var angle := rng.randf() * TAU
		var out := Vector3(cos(angle), rng.randf_range(-0.4, 0.25), sin(angle)).normalized()
		var length := rng.randf_range(0.16, 0.26)
		var width := rng.randf_range(0.08, 0.12)
		var points: Array[Vector3] = [at, at + out * length * 0.5, at + out * length]
		# Most leaves stay in the far mesh: a shrub is a mass of leaves.
		b.strip(points, PackedFloat32Array([width * 0.3, width, 0.0]), Vector3(out.z, 0.0, -out.x).normalized(),
			(Vector3.UP * 0.8 + out * 0.4).normalized(), 0.67, rng.randf(), top, rng.randf() * 0.6)
	# Seed heads and blossoms: short stalks just above the crown, a small
	# cluster each.
	for i in int(spec.seeds):
		var tip := tips[i % tips.size()]
		var angle := rng.randf() * TAU
		var out := Vector3(cos(angle), 0.0, sin(angle))
		var points := _arc(tip + out * 0.08, out, rng.randf_range(0.12, 0.22), rng.randf_range(0.2, 0.6),
			rng.randf_range(0.3, 0.8), 2)
		b.strip(points, PackedFloat32Array([0.035, 0.05, 0.0]), out.cross(Vector3.UP).normalized(), Vector3.UP,
			1.0, rng.randf(), top, rng.randf())
