extends RefCounted
## Ground scatter (prompt 191). Presentation only.
##
## Small things everywhere on the ground, so the meadow reads as a grainy
## surface with many small light and dark spots and shadows instead of a
## flat green: low shrubs (0.4-0.9 m, round, dark), fern and herb clumps,
## dry stalks with seed heads (dock, thistle) and field stones (0.1-0.4 m).
## All meshes are procedural (no model files, no sources needed), opaque
## triangles in one material (`assets/shaders/ground_scatter.gdshader`),
## within the decoration budgets of docs/ASSETS.md (shrub <= 500, clump
## <= 50, rock <= 300 triangles; here 109 / 36 / 36 / 25 / 30).
##
## Placed by the decoration lattice of `map_objects.gd` (layers
## "scatter_*", `SimHost.scatter_deco` with `edge` and `macro`): density by
## terrain type, wood density, 25 m patches and 90 m groups, denser in a 4 m
## band along hedges, walls, houses, tracks, roads, fields and at the edge
## of woods. Range and density per graphics quality (`graphics_quality.gd`:
## scatter_density, scatter_range, scatter_shadows). The shader shrinks the
## scatter into the ground towards the range (no popping), keeps herbs to
## lush and stalks to dry meadow, and flattens it under vehicle tracks
## (`set_tracks`, the raster of `ground_tracks.gd`). Craters clear it like
## every decoration layer (`craters_changed`).
##
## Vertex color: r = height share (0 ground .. 1 top), g = wind weight,
## b = random shade, a = part (0 shrub, 0.25 herb, 0.5 stalk, 0.75 seed
## head, 1 stone).

const MATERIAL := preload("res://assets/materials/env/ground_scatter.tres")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")

enum Kind { SHRUB, HERB, STALKS, STONE_SMALL, STONE_LARGE }

const PART_SHRUB := 0.0
const PART_HERB := 0.25
const PART_STALK := 0.5
const PART_HEAD := 0.75
const PART_STONE := 1.0

## Density factor per layer and biome (desert: scrub, dry stalks and many
## stones, no fern; winter: snow covers the low parts).
const BIOME_DENSITY: Dictionary[String, Dictionary] = {
	"forest": {},
	"desert": {"scatter_shrubs": 0.6, "scatter_herbs": 0.0, "scatter_stalks": 1.2, "scatter_stones": 2.0},
	"winter": {"scatter_shrubs": 0.7, "scatter_herbs": 0.0, "scatter_stalks": 0.6, "scatter_stones": 0.4},
}
const LOOKS: Dictionary[String, int] = {"forest": 0, "desert": 1, "winter": 2}

static var _meshes: Dictionary[int, Mesh] = {}


## The mesh of `kind`, built once.
static func mesh(kind: int) -> Mesh:
	if not _meshes.has(kind):
		_meshes[kind] = _build(kind)
	return _meshes[kind]


## Range (m), density factor and shadows of the quality `level`.
static func quality(level: String) -> Dictionary:
	var s := GraphicsQuality.settings(level)
	return {
		"range": float(s.get("scatter_range", 150.0)),
		"density": float(s.get("scatter_density", 1.0)),
		"shadows": bool(s.get("scatter_shadows", true)),
	}


## Density factor of `layer` in `biome`.
static func biome_density(layer: String, biome: String) -> float:
	return float(BIOME_DENSITY.get(biome, {}).get(layer, 1.0))


## Biome look and range on the shared material.
static func apply_look(biome: String, range_m: float) -> void:
	MATERIAL.set_shader_parameter("scatter_look", LOOKS.get(biome, 0))
	MATERIAL.set_shader_parameter("scatter_range", range_m)


## The vehicle track raster (`ground_tracks.gd`); null turns it off.
static func set_tracks(texture: Texture2D, origin: Vector2, size: Vector2) -> void:
	MATERIAL.set_shader_parameter("scatter_tracks", texture)
	MATERIAL.set_shader_parameter("scatter_track_origin", origin)
	MATERIAL.set_shader_parameter("scatter_track_size", size)
	MATERIAL.set_shader_parameter("scatter_has_tracks", texture != null)


static func _build(kind: int) -> ArrayMesh:
	var b := _Builder.new()
	var rng := RandomNumberGenerator.new()
	rng.seed = 191 + kind
	match kind:
		Kind.SHRUB:
			# Three overlapping lumps, flattened domes with ragged outlines.
			_lump(b, rng, Vector3(0.0, 0.0, 0.0), 0.26, 0.4, PART_SHRUB, 7, 3, 0.3)
			_lump(b, rng, Vector3(0.17, 0.0, -0.06), 0.18, 0.28, PART_SHRUB, 6, 2, 0.35)
			_lump(b, rng, Vector3(-0.1, 0.0, 0.15), 0.17, 0.24, PART_SHRUB, 6, 2, 0.35)
		Kind.HERB:
			# Nine fronds of different length radiating from the root, arched
			# up and down again.
			for k in 9:
				var angle := TAU * (k + rng.randf_range(-0.35, 0.35)) / 9.0
				_frond(b, rng, angle, rng.randf_range(0.18, 0.45), rng.randf_range(0.1, 0.26))
		Kind.STALKS:
			# Three stalks with a seed head each, three dead leaves at the foot.
			for k in 3:
				var at := Vector3(rng.randf_range(-0.08, 0.08), 0.0, rng.randf_range(-0.08, 0.08))
				var top := at + Vector3(rng.randf_range(-0.08, 0.08), rng.randf_range(0.55, 0.95), rng.randf_range(-0.08, 0.08))
				_stalk(b, at, top, 0.024, rng.randf())
				_head(b, top, rng.randf_range(0.06, 0.09), rng.randf())
			for k in 3:
				_leaf(b, rng, TAU * k / 3.0 + rng.randf(), rng.randf_range(0.16, 0.24))
		Kind.STONE_SMALL:
			_lump(b, rng, Vector3.ZERO, 0.1, 0.07, PART_STONE, 5, 2, 0.3)
		Kind.STONE_LARGE:
			_lump(b, rng, Vector3.ZERO, 0.2, 0.15, PART_STONE, 6, 2, 0.3)
	return b.commit(MATERIAL)


## A dome of `radius` and `height` on the ground at `at`: `seg` segments,
## `rings` rings above the foot ring, radii jittered by `rough`.
static func _lump(b: _Builder, rng: RandomNumberGenerator, at: Vector3, radius: float, height: float,
		part: float, seg: int, rings: int, rough: float) -> void:
	var jitter: Array[float] = []
	for s in seg:
		jitter.append(1.0 + rng.randf_range(-rough, rough))
	var base := b.size()
	for r in rings + 1:
		# Ring r at elevation angle from 0 (foot, a little below ground) up.
		var e := PI * 0.5 * float(r) / float(rings + 1)
		for s in seg:
			var a := TAU * (s + 0.5 * r) / seg
			var j := jitter[s] * (1.0 + rng.randf_range(-rough, rough) * 0.5)
			var dir := Vector3(cos(a) * cos(e), sin(e), sin(a) * cos(e))
			var pos := at + Vector3(dir.x * radius * j, dir.y * height - 0.03, dir.z * radius * j)
			var n := Vector3(dir.x / radius, dir.y / height, dir.z / radius).normalized()
			b.add(pos, n, Color(sin(e), sin(e) * 0.8, rng.randf(), part))
	b.add(at + Vector3(0.0, height, 0.0), Vector3.UP, Color(1.0, 0.8, rng.randf(), part))
	var top := b.size() - 1
	for r in rings:
		for s in seg:
			var a0 := base + r * seg + s
			var a1 := base + r * seg + (s + 1) % seg
			var c0 := base + (r + 1) * seg + s
			var c1 := base + (r + 1) * seg + (s + 1) % seg
			b.tri(a0, a1, c0)
			b.tri(a1, c1, c0)
	for s in seg:
		b.tri(base + rings * seg + s, base + rings * seg + (s + 1) % seg, top)


## One fern frond: a strip of root, arch and tip (four triangles).
static func _frond(b: _Builder, rng: RandomNumberGenerator, angle: float, length: float, rise: float) -> void:
	var out := Vector3(cos(angle), 0.0, sin(angle))
	var side := out.cross(Vector3.UP)
	var points: Array[Vector3] = [Vector3.ZERO, out * length * 0.45 + Vector3.UP * rise, out * length + Vector3.UP * rise * 0.35]
	var widths: Array[float] = [0.015, 0.05, 0.0]
	var heights: Array[float] = [0.0, 1.0, 0.7]
	var shade := rng.randf()
	var base := b.size()
	var n := (Vector3.UP * 0.85 + out * 0.3).normalized()
	for k in 3:
		var t := heights[k]
		if k < 2:
			for s in [-1.0, 1.0]:
				b.add(points[k] + side * widths[k] * s, n, Color(t, t, shade, PART_HERB))
		else:
			b.add(points[k], n, Color(t, t, shade, PART_HERB))
	b.tri(base, base + 2, base + 1)
	b.tri(base + 1, base + 2, base + 3)
	b.tri(base + 2, base + 4, base + 3)
	# The other half of the arch (lit from above like the first).
	b.tri(base, base + 1, base + 3)


## A thin stalk from `from` to `to`: one triangle, wide at the foot.
static func _stalk(b: _Builder, from: Vector3, to: Vector3, width: float, shade: float) -> void:
	var along := (to - from).normalized()
	var side := along.cross(Vector3.FORWARD).normalized() if absf(along.z) < 0.9 else along.cross(Vector3.RIGHT).normalized()
	var n := Vector3.UP
	var base := b.size()
	b.add(from - side * width, n, Color(0.0, 0.0, shade, PART_STALK))
	b.add(from + side * width, n, Color(0.0, 0.0, shade, PART_STALK))
	b.add(to, n, Color(1.0, 1.0, shade, PART_STALK))
	b.tri(base, base + 2, base + 1)
	# A second blade across, so the stalk shows from every side.
	var side2 := along.cross(side).normalized()
	b.add(from - side2 * width, n, Color(0.0, 0.0, shade, PART_STALK))
	b.add(from + side2 * width, n, Color(0.0, 0.0, shade, PART_STALK))
	b.tri(base + 3, base + 2, base + 4)


## A seed head: an elongated octahedron around `at` (eight triangles).
static func _head(b: _Builder, at: Vector3, radius: float, shade: float) -> void:
	var base := b.size()
	b.add(at + Vector3.UP * radius * 1.8, Vector3.UP, Color(1.0, 1.0, shade, PART_HEAD))
	b.add(at - Vector3.UP * radius * 1.2, Vector3.DOWN, Color(1.0, 1.0, shade, PART_HEAD))
	for k in 4:
		var a := TAU * k / 4.0
		var d := Vector3(cos(a), 0.0, sin(a))
		b.add(at + d * radius, d, Color(1.0, 1.0, shade, PART_HEAD))
	for k in 4:
		var r0 := base + 2 + k
		var r1 := base + 2 + (k + 1) % 4
		b.tri(base, r1, r0)
		b.tri(base + 1, r0, r1)


## A dead leaf lying at the foot of the stalks (two triangles).
static func _leaf(b: _Builder, rng: RandomNumberGenerator, angle: float, length: float) -> void:
	var out := Vector3(cos(angle), 0.0, sin(angle))
	var side := out.cross(Vector3.UP)
	var shade := rng.randf()
	var base := b.size()
	b.add(Vector3.UP * 0.02, Vector3.UP, Color(0.1, 0.0, shade, PART_STALK))
	b.add(out * length * 0.5 + side * 0.04 + Vector3.UP * 0.06, Vector3.UP, Color(0.3, 0.2, shade, PART_STALK))
	b.add(out * length * 0.5 - side * 0.04 + Vector3.UP * 0.06, Vector3.UP, Color(0.3, 0.2, shade, PART_STALK))
	b.add(out * length + Vector3.UP * 0.01, Vector3.UP, Color(0.2, 0.1, shade, PART_STALK))
	b.tri(base, base + 1, base + 2)
	b.tri(base + 1, base + 3, base + 2)


## Collects vertices and triangles of one mesh.
class _Builder:
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var indices := PackedInt32Array()

	func size() -> int:
		return verts.size()

	func add(pos: Vector3, normal: Vector3, color: Color) -> void:
		verts.append(pos)
		normals.append(normal)
		colors.append(color)

	func tri(a: int, b: int, c: int) -> void:
		indices.append_array([a, b, c])

	func commit(material: Material) -> ArrayMesh:
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = verts
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_COLOR] = colors
		arrays[Mesh.ARRAY_INDEX] = indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		mesh.surface_set_material(0, material)
		return mesh
