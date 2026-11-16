extends RefCounted
## Procedural meshes of dug positions (prompt 052). Presentation only: the
## simulation lowers the ground by 0.5 m (2 m height raster) and gives the
## cover; these meshes make the cut read as a hand-dug position:
##
## - fire and communication trenches: duckboards on the floor, plank or
##   wattle revetment with posts, an irregular earth parapet on both sides
##   (thrown-up spoil), sandbags at the firing bays (zigzag corners), a
##   trodden path behind, torn parapet and scattered bags where a shell
##   struck. Pieces join with mitered ends.
## - MG nests (sandbag ring), dugouts (log roof under an earth mound with a
##   dark entrance), vehicle pits (earth berm), foxholes (spoil ring).
##
## Everything is built in world coordinates from the work's points and the
## ground heights of `SimHost.height_at`, with deterministic noise from the
## position (the same in every client and every rebuild).

## Half width of the trench at the walls (m); FM 5-15 standard trench is
## 4 ft wide at the top (catalog `fort_trench_section`).
const HALF_WIDTH := 0.6
const COMM_HALF_WIDTH := 0.45
## Heights relative to the undug ground (m).
const FLOOR := -0.38
const WALL_TOP := 0.5
const CREST := 0.72
const COMM_CREST := 0.42
## Parapet: crest this far beyond the wall, toe further out (m).
const CREST_OUT := 0.45
const TOE_OUT := 1.4
## Stations along a piece (m).
const STEP := 0.5

const EARTH := Color(0.36, 0.28, 0.19)
const EARTH_DARK := Color(0.27, 0.21, 0.14)
const SPOIL := Color(0.45, 0.36, 0.25)
const MUD := Color(0.22, 0.17, 0.12)
const PLANK := Color(0.42, 0.31, 0.2)
const PLANK_DARK := Color(0.3, 0.22, 0.14)
const WATTLE := Color(0.47, 0.38, 0.24)
const POST := Color(0.26, 0.19, 0.12)
const BAG := Color(0.55, 0.49, 0.36)
const BAG_DARK := Color(0.44, 0.39, 0.28)
const PATH := Color(0.31, 0.26, 0.18)
const SCORCH := Color(0.12, 0.1, 0.08)
const DOOR := Color(0.04, 0.035, 0.03)
const LOG := Color(0.33, 0.25, 0.17)

static var _material: StandardMaterial3D


static func material() -> StandardMaterial3D:
	if _material == null:
		_material = StandardMaterial3D.new()
		_material.vertex_color_use_as_albedo = true
		_material.vertex_color_is_srgb = true
		_material.roughness = 0.95
		_material.cull_mode = BaseMaterial3D.CULL_DISABLED
	return _material


## Deterministic noise in [-1, 1] from a world position and a channel.
static func noise(p: Vector3, channel: int = 0) -> float:
	var h := hash(Vector3i(roundi(p.x * 4.0), roundi(p.z * 4.0), channel))
	return float(h % 2001) / 1000.0 - 1.0


static func _shade(c: Color, p: Vector3, amount: float = 0.08) -> Color:
	var n := noise(p, 7) * amount
	return Color(clampf(c.r + n, 0, 1), clampf(c.g + n, 0, 1), clampf(c.b + n * 0.8, 0, 1))


## Ground height without the dig: the higher of the heights `reach` m to
## both sides (beyond the dug samples), less the 0.1 m parapet of the sim.
static func surface_at(sim_host: SimHost, p: Vector3, across: Vector3, reach: float = 2.6) -> float:
	var a := p + across * reach
	var b := p - across * reach
	var h := maxf(sim_host.height_at(a.x, a.z), sim_host.height_at(b.x, b.z))
	return maxf(h - 0.1, sim_host.height_at(p.x, p.z))


static func _quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3, color: Color) -> void:
	st.set_color(color)
	st.add_vertex(a)
	st.add_vertex(b)
	st.add_vertex(c)
	st.add_vertex(a)
	st.add_vertex(c)
	st.add_vertex(d)


## An oriented box: center, unit axes `x` (across) and `z` (along), size.
static func _box(st: SurfaceTool, center: Vector3, x: Vector3, z: Vector3, size: Vector3, color: Color) -> void:
	var y := Vector3.UP
	var hx := x * size.x * 0.5
	var hy := y * size.y * 0.5
	var hz := z * size.z * 0.5
	var c := [
		center - hx - hy - hz, center + hx - hy - hz, center + hx - hy + hz, center - hx - hy + hz,
		center - hx + hy - hz, center + hx + hy - hz, center + hx + hy + hz, center - hx + hy + hz,
	]
	var top := color.lightened(0.06)
	_quad(st, c[4], c[5], c[6], c[7], top)
	_quad(st, c[0], c[1], c[5], c[4], color)
	_quad(st, c[1], c[2], c[6], c[5], color.darkened(0.08))
	_quad(st, c[2], c[3], c[7], c[6], color)
	_quad(st, c[3], c[0], c[4], c[7], color.darkened(0.08))


## A sandbag: a box slightly squashed, jittered by the position.
static func _bag(st: SurfaceTool, at: Vector3, along: Vector3, seed_pos: Vector3) -> void:
	var yaw := noise(seed_pos, 3) * 0.25
	var z := along.rotated(Vector3.UP, yaw).normalized()
	var x := z.cross(Vector3.UP).normalized()
	var color := BAG if noise(seed_pos, 4) > -0.3 else BAG_DARK
	_box(st, at, x, z, Vector3(0.3, 0.14, 0.5), _shade(color, seed_pos, 0.05))


static func _instance(st: SurfaceTool, name: String) -> MeshInstance3D:
	st.generate_normals()
	var mesh := MeshInstance3D.new()
	mesh.name = name
	mesh.mesh = st.commit()
	mesh.material_override = material()
	mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	return mesh


static func _start() -> SurfaceTool:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	return st


## One trench piece from `from` to `to`. `opts`:
##   comm (bool): communication trench (narrower, lower parapet, no bays)
##   miter_from / miter_to (Vector3): across vector at the ends (mitered,
##     already scaled), default the piece's own across vector
##   bay_from / bay_to (bool): firing bay at that end (sandbags)
##   damaged (bool): a shell struck it
##   path (bool): trodden path behind (on the +across side: inside a
##     counter-clockwise ring)
static func trench(sim_host: SimHost, from: Vector3, to: Vector3, opts: Dictionary) -> MeshInstance3D:
	var comm: bool = opts.get("comm", false)
	var along := Vector3(to.x - from.x, 0, to.z - from.z)
	var length := along.length()
	if length < 0.05:
		along = Vector3(0, 0, 1)
		length = 0.05
	along /= length
	var across := Vector3(-along.z, 0, along.x)
	var miter_from: Vector3 = opts.get("miter_from", across)
	var miter_to: Vector3 = opts.get("miter_to", across)
	var hw := COMM_HALF_WIDTH if comm else HALF_WIDTH
	var crest := COMM_CREST if comm else CREST
	var n := maxi(1, ceili(length / STEP))
	var damaged: bool = opts.get("damaged", false)
	var hole := Vector2(-1, -1)
	if damaged:
		var mid := 0.3 + 0.4 * (noise(from + to, 9) * 0.5 + 0.5)
		hole = Vector2(mid - 0.18, mid + 0.18)
	var revet_wattle := noise(from + to, 5) > 0.2
	var st := _start()
	# Stations: base point, across vector (mitered at the ends), surface.
	var points: Array[Vector3] = []
	var acrosses: Array[Vector3] = []
	var surfaces: Array[float] = []
	for k in n + 1:
		var t := float(k) / float(n)
		var p := from.lerp(to, t)
		var a := across
		if k == 0:
			a = miter_from
		elif k == n:
			a = miter_to
		points.append(p)
		acrosses.append(a)
		surfaces.append(surface_at(sim_host, p, across))
	for k in n:
		var t0 := float(k) / float(n)
		var t1 := float(k + 1) / float(n)
		var p0 := points[k]
		var p1 := points[k + 1]
		var a0 := acrosses[k]
		var a1 := acrosses[k + 1]
		var s0 := surfaces[k]
		var s1 := surfaces[k + 1]
		# Floor (mud) and duckboard slats.
		_quad(st, p0 - a0 * hw + Vector3.UP * (s0 + FLOOR - p0.y), p0 + a0 * hw + Vector3.UP * (s0 + FLOOR - p0.y),
				p1 + a1 * hw + Vector3.UP * (s1 + FLOOR - p1.y), p1 - a1 * hw + Vector3.UP * (s1 + FLOOR - p1.y),
				_shade(MUD, p0))
		var slat_center := p0.lerp(p1, 0.5)
		slat_center.y = lerpf(s0, s1, 0.5) + FLOOR + 0.04
		_box(st, slat_center, across, along, Vector3(hw * 1.4, 0.04, 0.12), _shade(PLANK_DARK, slat_center, 0.06))
		for side: float in [-1.0, 1.0]:
			var w0 := p0 + a0 * hw * side
			var w1 := p1 + a1 * hw * side
			var wall0 := Vector3(w0.x, s0 + FLOOR, w0.z)
			var wall1 := Vector3(w1.x, s1 + FLOOR, w1.z)
			var torn := t0 >= hole.x and t1 <= hole.y and side > 0.0
			var top0 := s0 + (WALL_TOP * 0.3 if torn else WALL_TOP) + noise(w0, 1) * 0.04
			var top1 := s1 + (WALL_TOP * 0.3 if torn else WALL_TOP) + noise(w1, 1) * 0.04
			# Revetment in three bands (planks or wattle).
			var bands := 3
			for b in bands:
				var f0 := float(b) / bands
				var f1 := float(b + 1) / bands
				var lo0 := lerpf(wall0.y, top0, f0)
				var hi0 := lerpf(wall0.y, top0, f1) - 0.015
				var lo1 := lerpf(wall1.y, top1, f0)
				var hi1 := lerpf(wall1.y, top1, f1) - 0.015
				var base := WATTLE if revet_wattle else PLANK
				var c := _shade(base, w0 + Vector3(0, b, 0), 0.07)
				if torn:
					c = _shade(EARTH_DARK, w0, 0.05)
				_quad(st, Vector3(w0.x, lo0, w0.z), Vector3(w1.x, lo1, w1.z), Vector3(w1.x, hi1, w1.z),
						Vector3(w0.x, hi0, w0.z), c)
			# Posts every ~1.4 m.
			if k % 3 == 0 and not torn:
				var post := Vector3(w0.x, (wall0.y + top0) * 0.5, w0.z) - across * side * 0.04
				_box(st, post, across, along, Vector3(0.09, top0 - wall0.y + 0.08, 0.09), _shade(POST, post, 0.04))
			# Parapet: inner face, crest, outer slope to the toe.
			var crest_h := crest * (0.35 if torn else 1.0) + noise(w0, 2) * 0.08
			var crest_h1 := crest * (0.35 if torn else 1.0) + noise(w1, 2) * 0.08
			var out0 := a0 * side
			var out1 := a1 * side
			var c0 := w0 + out0 * (CREST_OUT + noise(w0, 6) * 0.06)
			var c1 := w1 + out1 * (CREST_OUT + noise(w1, 6) * 0.06)
			var toe0 := w0 + out0 * (TOE_OUT + noise(w0, 8) * 0.15)
			var toe1 := w1 + out1 * (TOE_OUT + noise(w1, 8) * 0.15)
			var toe_y0 := sim_host.height_at(toe0.x, toe0.z) - 0.06
			var toe_y1 := sim_host.height_at(toe1.x, toe1.z) - 0.06
			var earth := _shade(SCORCH if torn else (SPOIL if side > 0.0 else EARTH), c0, 0.06)
			_quad(st, Vector3(w0.x, top0, w0.z), Vector3(w1.x, top1, w1.z),
					Vector3(c1.x, s1 + crest_h1, c1.z), Vector3(c0.x, s0 + crest_h, c0.z), earth.darkened(0.05))
			_quad(st, Vector3(c0.x, s0 + crest_h, c0.z), Vector3(c1.x, s1 + crest_h1, c1.z),
					Vector3(toe1.x, toe_y1, toe1.z), Vector3(toe0.x, toe_y0, toe0.z), earth)
			# Thrown-up clods on the outer slope.
			if noise(w0, 11) > 0.55:
				var clod := c0.lerp(toe0, 0.5)
				clod.y = lerpf(s0 + crest_h, toe_y0, 0.5) + 0.05
				_box(st, clod, across, along, Vector3(0.25, 0.12, 0.3), _shade(SPOIL, clod, 0.08))
		# Trodden path behind the trench.
		if opts.get("path", false):
			var q0 := p0 + a0 * (hw + TOE_OUT + 1.4)
			var q1 := p1 + a1 * (hw + TOE_OUT + 1.4)
			var r0 := p0 + a0 * (hw + TOE_OUT + 2.3 + noise(p0, 12) * 0.3)
			var r1 := p1 + a1 * (hw + TOE_OUT + 2.3 + noise(p1, 12) * 0.3)
			var lift := 0.035
			_quad(st, Vector3(q0.x, sim_host.height_at(q0.x, q0.z) + lift, q0.z),
					Vector3(q1.x, sim_host.height_at(q1.x, q1.z) + lift, q1.z),
					Vector3(r1.x, sim_host.height_at(r1.x, r1.z) + lift, r1.z),
					Vector3(r0.x, sim_host.height_at(r0.x, r0.z) + lift, r0.z), _shade(PATH, q0, 0.04))
	# Sandbags at the firing bays: two courses on the crest of both sides.
	for end: Array in [[opts.get("bay_from", false), 0.0], [opts.get("bay_to", false), 1.0]]:
		if comm or not end[0]:
			continue
		var t: float = end[1]
		var base := from.lerp(to, t)
		var s := surface_at(sim_host, base, across)
		var a: Vector3 = miter_from if t == 0.0 else miter_to
		for side: float in [-1.0, 1.0]:
			for course in 2:
				for b in 3:
					var u := (float(b) - 1.0) * 0.52 + (0.26 if course == 1 else 0.0)
					var at := base + along * u * (1.0 if t == 0.0 else -1.0) * 1.0 + a * side * (hw + 0.22)
					at.y = s + WALL_TOP + 0.08 + course * 0.14
					_bag(st, at, along, at + Vector3(course, 0, b))
	# Scattered bags and a scorched floor where a shell struck.
	if damaged:
		var at := from.lerp(to, (hole.x + hole.y) * 0.5)
		var s := surface_at(sim_host, at, across)
		for b in 5:
			var off := across * (noise(at, 20 + b) * 2.4) + along * (noise(at, 30 + b) * 1.4)
			var p := at + off
			p.y = maxf(sim_host.height_at(p.x, p.z), s + FLOOR) + 0.08
			_bag(st, p, along.rotated(Vector3.UP, noise(at, 40 + b) * 1.5), p)
	return _instance(st, "Trench")


## A dug MG nest: floor, a horseshoe of sandbags (three courses, open to
## the rear), earth banked against it. `facing` points to the open front.
static func nest(sim_host: SimHost, at: Vector3, facing: Vector3, damaged: bool) -> MeshInstance3D:
	var st := _start()
	var s := surface_at(sim_host, at, Vector3(1, 0, 0), 3.0)
	var r := 1.55
	_disc(st, Vector3(at.x, s + FLOOR + 0.02, at.z), 1.25, MUD)
	var front := facing.normalized() if facing.length() > 0.01 else Vector3(0, 0, -1)
	var base_angle := atan2(front.x, front.z)
	var count := 14
	for course in 3:
		for k in count:
			# 300 degrees around the front, open at the back.
			var ang := base_angle + deg_to_rad(-150.0 + 300.0 * (float(k) + (0.5 if course == 1 else 0.0)) / count)
			var dir := Vector3(sin(ang), 0, cos(ang))
			var p := at + dir * r
			var missing := damaged and course == 2 and k > count / 3 and k < count / 2
			if missing:
				continue
			p.y = s + 0.07 + course * 0.14
			_bag(st, p, Vector3(dir.z, 0, -dir.x), p + Vector3(course, 0, 0))
			if course == 0:
				# Earth banked outside.
				var o0 := at + dir * (r + 0.25)
				var o1 := at + dir * (r + 1.3)
				var nxt := ang + deg_to_rad(300.0 / count)
				var d1 := Vector3(sin(nxt), 0, cos(nxt))
				var o2 := at + d1 * (r + 1.3)
				var o3 := at + d1 * (r + 0.25)
				_quad(st, Vector3(o0.x, s + 0.42, o0.z), Vector3(o3.x, s + 0.42, o3.z),
						Vector3(o2.x, sim_host.height_at(o2.x, o2.z) - 0.05, o2.z),
						Vector3(o1.x, sim_host.height_at(o1.x, o1.z) - 0.05, o1.z), _shade(SPOIL, o0, 0.06))
	return _instance(st, "MgNest")


## A dugout: an earth mound over a log roof with a dark entrance facing
## `entrance` (towards its communication trench).
static func dugout(sim_host: SimHost, at: Vector3, entrance: Vector3) -> MeshInstance3D:
	var st := _start()
	var s := surface_at(sim_host, at, Vector3(1, 0, 0), 3.5)
	var front := entrance.normalized() if entrance.length() > 0.01 else Vector3(0, 0, 1)
	var side := Vector3(-front.z, 0, front.x)
	# Mound: a low pyramid frustum of earth, 4.4 m x 4 m.
	var top := s + 0.6
	var h_in := [Vector2(-1.3, -1.1), Vector2(1.3, -1.1), Vector2(1.3, 1.1), Vector2(-1.3, 1.1)]
	var h_out := [Vector2(-2.4, -2.2), Vector2(2.4, -2.2), Vector2(2.4, 2.2), Vector2(-2.4, 2.2)]
	var pt := func(v: Vector2, y: float) -> Vector3:
		var p: Vector3 = at + side * v.x + front * v.y
		return Vector3(p.x, y, p.z)
	var inner: Array[Vector3] = []
	var outer: Array[Vector3] = []
	for k in 4:
		inner.append(pt.call(h_in[k], top + noise(at, k) * 0.06))
		var o: Vector3 = pt.call(h_out[k], 0.0)
		o.y = sim_host.height_at(o.x, o.z) - 0.05
		outer.append(o)
	_quad(st, inner[0], inner[1], inner[2], inner[3], _shade(EARTH, at, 0.05))
	for k in 4:
		var j := (k + 1) % 4
		_quad(st, outer[k], outer[j], inner[j], inner[k], _shade(SPOIL if k == 2 else EARTH, outer[k], 0.06))
	# Log ends showing under the earth at the front, the dark doorway.
	for l in 5:
		var p: Vector3 = pt.call(Vector2(-1.0 + l * 0.5, 2.25), top - 0.25)
		_box(st, p, side, front, Vector3(0.22, 0.22, 0.4), _shade(LOG, p, 0.05))
	var d0: Vector3 = pt.call(Vector2(-0.45, 2.3), s + FLOOR)
	var d1: Vector3 = pt.call(Vector2(0.45, 2.3), s + FLOOR)
	_quad(st, d0, d1, Vector3(d1.x, top - 0.38, d1.z), Vector3(d0.x, top - 0.38, d0.z), DOOR)
	for x in [-0.55, 0.55]:
		var post: Vector3 = pt.call(Vector2(x, 2.32), (s + FLOOR + top - 0.3) * 0.5)
		_box(st, post, side, front, Vector3(0.14, top - 0.3 - s - FLOOR, 0.14), _shade(POST, post, 0.04))
	return _instance(st, "Dugout")


## A vehicle pit: an oval earth berm about 1 m high around a 7 m x 4 m pit.
static func pit(sim_host: SimHost, at: Vector3) -> MeshInstance3D:
	var st := _start()
	var s := surface_at(sim_host, at, Vector3(1, 0, 0), 4.5)
	_disc(st, Vector3(at.x, s - 0.42, at.z), 3.2, MUD)
	var count := 20
	for k in count:
		var a0 := TAU * k / count
		var a1 := TAU * (k + 1) / count
		var d0 := Vector3(sin(a0) * 1.35, 0, cos(a0))
		var d1 := Vector3(sin(a1) * 1.35, 0, cos(a1))
		var h0 := 1.0 + noise(at + d0, 2) * 0.12
		var h1 := 1.0 + noise(at + d1, 2) * 0.12
		var i0 := at + d0 * 3.0
		var i1 := at + d1 * 3.0
		var c0 := at + d0 * 3.6
		var c1 := at + d1 * 3.6
		var o0 := at + d0 * 5.4
		var o1 := at + d1 * 5.4
		var col := _shade(SPOIL, c0, 0.06)
		_quad(st, Vector3(i0.x, s - 0.4, i0.z), Vector3(i1.x, s - 0.4, i1.z),
				Vector3(c1.x, s + h1, c1.z), Vector3(c0.x, s + h0, c0.z), col.darkened(0.08))
		_quad(st, Vector3(c0.x, s + h0, c0.z), Vector3(c1.x, s + h1, c1.z),
				Vector3(o1.x, sim_host.height_at(o1.x, o1.z) - 0.05, o1.z),
				Vector3(o0.x, sim_host.height_at(o0.x, o0.z) - 0.05, o0.z), col)
	return _instance(st, "VehiclePit")


## A foxhole: a small dark hole with a ring of spoil.
static func foxhole(sim_host: SimHost, at: Vector3) -> MeshInstance3D:
	var st := _start()
	var s := surface_at(sim_host, at, Vector3(1, 0, 0), 2.4)
	_disc(st, Vector3(at.x, s - 0.3, at.z), 0.55, MUD.darkened(0.3))
	var count := 10
	for k in count:
		var a0 := TAU * k / count
		var a1 := TAU * (k + 1) / count
		var d0 := Vector3(sin(a0), 0, cos(a0))
		var d1 := Vector3(sin(a1), 0, cos(a1))
		var h := 0.28 + noise(at + d0, 2) * 0.06
		_quad(st, Vector3(at.x + d0.x * 0.55, s - 0.3, at.z + d0.z * 0.55),
				Vector3(at.x + d1.x * 0.55, s - 0.3, at.z + d1.z * 0.55),
				Vector3(at.x + d1.x * 0.95, s + h, at.z + d1.z * 0.95),
				Vector3(at.x + d0.x * 0.95, s + h, at.z + d0.z * 0.95), _shade(EARTH_DARK, at + d0, 0.05))
		var o0 := at + d0 * 1.9
		var o1 := at + d1 * 1.9
		_quad(st, Vector3(at.x + d0.x * 0.95, s + h, at.z + d0.z * 0.95),
				Vector3(at.x + d1.x * 0.95, s + h, at.z + d1.z * 0.95),
				Vector3(o1.x, sim_host.height_at(o1.x, o1.z) - 0.04, o1.z),
				Vector3(o0.x, sim_host.height_at(o0.x, o0.z) - 0.04, o0.z), _shade(SPOIL, o0, 0.06))
	return _instance(st, "Foxhole")


static func _disc(st: SurfaceTool, center: Vector3, radius: float, color: Color) -> void:
	var count := 14
	for k in count:
		var a0 := TAU * k / count
		var a1 := TAU * (k + 1) / count
		st.set_color(_shade(color, center + Vector3(k, 0, 0), 0.03))
		st.add_vertex(center)
		st.add_vertex(center + Vector3(sin(a0), 0, cos(a0)) * radius)
		st.add_vertex(center + Vector3(sin(a1), 0, cos(a1)) * radius)
