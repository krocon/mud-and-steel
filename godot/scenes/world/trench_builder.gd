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
##   dark entrance), vehicle pits (U-shaped sandbag and timber emplacement
##   with rear ramp, prompt 230), foxholes (spoil ring).
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


## A vehicle/gun emplacement (prompt 230): a rear-open U-shaped horseshoe
## (~260 degrees enclosed) with a sunken floor, inner timber plank revetment
## and vertical posts, 3-4 staggered courses of sandbags (~1.05-1.15 m above
## surrounding ground at the front), a sloped outer earth berm blending into
## terrain, and a rear ramp with timber treads. `facing` points out the front.
static func pit(sim_host: SimHost, at: Vector3, facing: Vector3 = Vector3(0, 0, -1), damaged: bool = false) -> MeshInstance3D:
	var st := _start()
	var front := Vector3(facing.x, 0.0, facing.z)
	front = front.normalized() if front.length() > 0.01 else Vector3(0, 0, -1)
	var right := Vector3(-front.z, 0.0, front.x)
	var floor_y := sim_host.height_at(at.x, at.z) + 0.04
	var rx := 2.95
	var rz := 3.65
	var span := deg_to_rad(128.0)  # 256 degrees enclosed, ~4.0 m open rear entrance
	var segs := 14
	var breach_a := deg_to_rad(noise(at, 19) * 55.0)

	var angles: Array[float] = []
	var p_in: Array[Vector3] = []
	var p_crest: Array[Vector3] = []
	var p_toe: Array[Vector3] = []
	var n_out: Array[Vector3] = []
	var tangents: Array[Vector3] = []
	var g_y: Array[float] = []
	var wall_top: Array[float] = []
	var crest_y: Array[float] = []
	var toe_y: Array[float] = []

	for k in segs + 1:
		var t := float(k) / float(segs)
		var a := lerpf(-span, span, t)
		var u := sin(a)
		var v := cos(a)
		var inner := at + right * (u * rx) + front * (v * rz)
		var out_n := (right * (u / rx) + front * (v / rz)).normalized()
		var tan_v := (right * (cos(a) * rx) + front * (-sin(a) * rz)).normalized()
		var crest := inner + out_n * 0.56
		var toe := inner + out_n * 2.15
		var ty := sim_host.height_at(toe.x, toe.z)
		var gy := maxf(ty, floor_y + 0.22)
		var end_taper := 1.0
		if k == 0 or k == segs:
			end_taper = 0.52
		elif k == 1 or k == segs - 1:
			end_taper = 0.88
		var in_breach := damaged and absf(a - breach_a) < deg_to_rad(22.0)
		var wt := gy + 0.50 * end_taper + noise(inner, 2) * 0.03
		if in_breach:
			wt -= 0.14
		var front_boost := 0.14 if absf(a) < deg_to_rad(85.0) else 0.0
		var cy := wt + (0.34 + front_boost) * end_taper + noise(crest, 5) * 0.06
		if in_breach:
			cy -= 0.34
		angles.append(a)
		p_in.append(inner)
		p_crest.append(crest)
		p_toe.append(toe)
		n_out.append(out_n)
		tangents.append(tan_v)
		g_y.append(gy)
		wall_top.append(wt)
		crest_y.append(cy)
		toe_y.append(ty)

	# Sunken floor fan across the horseshoe interior (hugs the dug bowl so
	# grass never pokes through near the inner timber wall).
	var floor_center := Vector3(at.x, floor_y, at.z)
	var edge_y: Array[float] = []
	for k in segs + 1:
		var ey := clampf(sim_host.height_at(p_in[k].x, p_in[k].z) + 0.04, floor_y, wall_top[k] - 0.32)
		edge_y.append(ey)
	for k in segs:
		var fcol := _shade(MUD, p_in[k], 0.03)
		if damaged and absf(angles[k] - breach_a) < deg_to_rad(28.0):
			fcol = fcol.lerp(SCORCH, 0.55)
		st.set_color(fcol)
		st.add_vertex(floor_center)
		st.add_vertex(Vector3(p_in[k].x, edge_y[k], p_in[k].z))
		st.add_vertex(Vector3(p_in[k + 1].x, edge_y[k + 1], p_in[k + 1].z))
	# Close the floor triangle toward the rear ramp threshold.
	var rear_l := p_in[0]
	var rear_r := p_in[segs]
	st.set_color(_shade(MUD, at - front * 2.0, 0.03))
	st.add_vertex(floor_center)
	st.add_vertex(Vector3(rear_r.x, edge_y[segs], rear_r.z))
	st.add_vertex(Vector3(rear_l.x, edge_y[0], rear_l.z))

	# Rear entry ramp: sloped compacted earth (always above terrain) + 4 timber tread planks.
	var ramp_rows := 4
	var ramp_len := 3.0
	for r in ramp_rows:
		var t0 := float(r) / float(ramp_rows)
		var t1 := float(r + 1) / float(ramp_rows)
		var wl0 := rear_l - front * (t0 * ramp_len)
		var wr0 := rear_r - front * (t0 * ramp_len)
		var wl1 := rear_l - front * (t1 * ramp_len)
		var wr1 := rear_r - front * (t1 * ramp_len)
		var yl0 := maxf(lerpf(edge_y[0], sim_host.height_at(wl0.x, wl0.z) + 0.045, t0), sim_host.height_at(wl0.x, wl0.z) + 0.045)
		var yr0 := maxf(lerpf(edge_y[segs], sim_host.height_at(wr0.x, wr0.z) + 0.045, t0), sim_host.height_at(wr0.x, wr0.z) + 0.045)
		var yl1 := maxf(lerpf(edge_y[0], sim_host.height_at(wl1.x, wl1.z) + 0.045, t1), sim_host.height_at(wl1.x, wl1.z) + 0.045)
		var yr1 := maxf(lerpf(edge_y[segs], sim_host.height_at(wr1.x, wr1.z) + 0.045, t1), sim_host.height_at(wr1.x, wr1.z) + 0.045)
		_quad(st, Vector3(wl0.x, yl0, wl0.z), Vector3(wr0.x, yr0, wr0.z),
				Vector3(wr1.x, yr1, wr1.z), Vector3(wl1.x, yl1, wl1.z),
				_shade(PATH, wl0.lerp(wr1, 0.5), 0.04))
	var ramp_w := rear_l.distance_to(rear_r) - 0.55
	for p_idx in 4:
		var tp := (float(p_idx) + 0.45) / 4.2
		var mid := rear_l.lerp(rear_r, 0.5) - front * (tp * ramp_len)
		var ty_m := sim_host.height_at(mid.x, mid.z)
		var py := maxf(lerpf((edge_y[0] + edge_y[segs]) * 0.5, ty_m + 0.05, tp), ty_m + 0.05) + 0.03
		var col := PLANK if p_idx % 2 == 0 else PLANK_DARK
		_box(st, Vector3(mid.x, py, mid.z), right, front,
				Vector3(ramp_w + noise(mid, p_idx) * 0.12, 0.06, 0.26), _shade(col, mid, 0.04))

	# Inner timber plank revetment (3 horizontal boards) + outer earth spoil berm.
	for k in segs:
		var a0 := p_in[k]
		var a1 := p_in[k + 1]
		var mid_a := (angles[k] + angles[k + 1]) * 0.5
		var in_breach := damaged and absf(mid_a - breach_a) < deg_to_rad(22.0)
		var f0 := floor_y - 0.10
		var f1 := floor_y - 0.10
		for b in 3:
			var b0 := float(b) / 3.0
			var b1 := float(b + 1) / 3.0
			var y00 := lerpf(f0, wall_top[k], b0)
			var y01 := lerpf(f0, wall_top[k], b1)
			var y10 := lerpf(f1, wall_top[k + 1], b0)
			var y11 := lerpf(f1, wall_top[k + 1], b1)
			var wcol := PLANK if (b + k) % 2 == 0 else PLANK_DARK
			wcol = _shade(wcol, a0 + Vector3(0, b, 0), 0.04)
			if in_breach:
				wcol = wcol.lerp(SCORCH, 0.55)
			_quad(st, Vector3(a0.x, y00, a0.z), Vector3(a1.x, y10, a1.z),
					Vector3(a1.x, y11, a1.z), Vector3(a0.x, y01, a0.z), wcol)
		# Top timber lip / earth backing bridge between inner wall and outer berm crest.
		var scol := _shade(SPOIL, p_crest[k], 0.06)
		if in_breach:
			scol = scol.lerp(SCORCH, 0.5)
		_quad(st, Vector3(a0.x, wall_top[k], a0.z), Vector3(a1.x, wall_top[k + 1], a1.z),
				Vector3(p_crest[k + 1].x, crest_y[k + 1], p_crest[k + 1].z),
				Vector3(p_crest[k].x, crest_y[k], p_crest[k].z), scol.darkened(0.06))
		# Outer sloped earth berm down to terrain.
		_quad(st, Vector3(p_crest[k].x, crest_y[k], p_crest[k].z),
				Vector3(p_crest[k + 1].x, crest_y[k + 1], p_crest[k + 1].z),
				Vector3(p_toe[k + 1].x, toe_y[k + 1] - 0.08, p_toe[k + 1].z),
				Vector3(p_toe[k].x, toe_y[k] - 0.08, p_toe[k].z), scol)
		if k % 3 == 1:
			var clod := p_crest[k].lerp(p_toe[k], 0.5)
			clod.y = lerpf(crest_y[k], toe_y[k], 0.5) + 0.04
			_box(st, clod, n_out[k], tangents[k], Vector3(0.28, 0.13, 0.34), _shade(SPOIL, clod, 0.08))

	# Vertical revetment support posts (every second station + both entrance cheeks).
	for k in range(0, segs + 1, 2):
		var h := wall_top[k] - floor_y + 0.16
		var pos := p_in[k] - n_out[k] * 0.05
		pos.y = floor_y + h * 0.5 - 0.06
		_box(st, pos, n_out[k], tangents[k], Vector3(0.13, h, 0.13), _shade(POST, pos, 0.04))

	# Close the two open rear berm ends with sloped earth cheeks so there are no hollow cross-sections.
	for end_k: int in [0, segs]:
		var end_in := p_in[end_k]
		var end_cr := p_crest[end_k]
		var end_to := p_toe[end_k]
		var end_y_in := floor_y - 0.06
		var end_y_cr := sim_host.height_at(end_cr.x, end_cr.z) - 0.06
		var end_col := _shade(EARTH_DARK, end_cr, 0.05)
		_quad(st, Vector3(end_in.x, end_y_in, end_in.z),
				Vector3(end_cr.x, end_y_cr, end_cr.z),
				Vector3(end_cr.x, crest_y[end_k], end_cr.z),
				Vector3(end_in.x, wall_top[end_k], end_in.z), end_col)
		st.set_color(_shade(SPOIL, end_to, 0.05))
		st.add_vertex(Vector3(end_cr.x, crest_y[end_k], end_cr.z))
		st.add_vertex(Vector3(end_cr.x, end_y_cr, end_cr.z))
		st.add_vertex(Vector3(end_to.x, toe_y[end_k] - 0.08, end_to.z))

	# Sandbag crown: 3 staggered courses along the whole U-wall, plus a 4th course
	# along the forward arc (~170 deg) for full hull-down parapet height (~1.05-1.15 m).
	var bag_count := 22
	for course in 4:
		for b in bag_count:
			var t := (float(b) + (0.5 if course % 2 == 1 else 0.0)) / float(bag_count)
			if t < 0.02 or t > 0.98:
				continue
			var a := lerpf(-span, span, t)
			# 4th course only across the front arc; 3rd course stops just short of the tapered tips.
			if course == 3 and absf(a) > deg_to_rad(85.0):
				continue
			if course == 2 and absf(a) > deg_to_rad(118.0):
				continue
			# Damage breach removes upper courses at the shell impact sector.
			if damaged and absf(a - breach_a) < deg_to_rad(20.0):
				if course >= 2 or (course == 1 and b % 2 == 0):
					continue
			var sf := t * float(segs)
			var si := mini(int(floor(sf)), segs - 1)
			var frac := sf - float(si)
			var wt_here := lerpf(wall_top[si], wall_top[si + 1], frac)
			var u := sin(a)
			var v := cos(a)
			var inner := at + right * (u * rx) + front * (v * rz)
			var out_n := (right * (u / rx) + front * (v / rz)).normalized()
			var tan_v := (right * (cos(a) * rx) + front * (-sin(a) * rz)).normalized()
			var bag_pos := inner + out_n * (0.19 + (0.04 if course % 2 == 1 else 0.0))
			bag_pos.y = wt_here + 0.07 + float(course) * 0.145
			var seed_p := bag_pos + Vector3(course * 3.0, 0.0, b * 1.7)
			var yaw := noise(seed_p, 3) * 0.16
			var bz := tan_v.rotated(Vector3.UP, yaw).normalized()
			var bx := bz.cross(Vector3.UP).normalized()
			var bcol := BAG if noise(seed_p, 4) > -0.3 else BAG_DARK
			if damaged and absf(a - breach_a) < deg_to_rad(28.0):
				bcol = bcol.lerp(SCORCH, 0.35)
			_box(st, bag_pos, bx, bz, Vector3(0.34, 0.15, 0.58), _shade(bcol, seed_p, 0.05))

	# Scattered sandbags around the impact breach when damaged.
	if damaged:
		var bu := sin(breach_a)
		var bv := cos(breach_a)
		var b_in := at + right * (bu * rx) + front * (bv * rz)
		var b_out := (right * (bu / rx) + front * (bv / rz)).normalized()
		var b_tan := (right * (cos(breach_a) * rx) + front * (-sin(breach_a) * rz)).normalized()
		for k in 7:
			var rad := -0.9 + noise(at, 40 + k) * 2.2
			var lat := noise(at, 50 + k) * 1.8
			var sp := b_in + b_out * rad + b_tan * lat
			sp.y = maxf(sim_host.height_at(sp.x, sp.z), floor_y) + 0.07
			_bag(st, sp, b_tan.rotated(Vector3.UP, noise(at, 60 + k) * 1.8), sp)

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
