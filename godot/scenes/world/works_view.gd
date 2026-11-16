extends Node3D
## Field works (prompt 035). Presentation only: the simulation decides where
## mines lie, who knows them, what is built and what blocks; this script
##
## - marks the mines the local player's team knows (`SimHost.known_mines`):
##   own and allied ones yellow, found hostile ones red; anti-tank mines
##   larger than anti-personnel ones. Hidden mines are never drawn (the
##   simulation does not hand them out).
## - draws finished wire (`fort_barbed_wire`) and anti-tank obstacles
##   (`fort_hedgehog`) with the models of prompt 007, for everybody like
##   map objects; foxholes and trenches are the dug ground itself (crater
##   events update the terrain) plus a low earth parapet strip.
## - draws own sites as translucent outlines, with dust while engineers dig.
## - lays a thrown track beside vehicles that hit an anti-tank mine.
## - prompt 052: hand-dug trenches, MG nests, dugouts, vehicle pits and
##   foxholes as procedural meshes (`trench_builder.gd`; pieces join with
##   mitered ends, sandbags at the zigzag corners, torn parapets where a
##   shell struck), camouflage nets and concrete teeth as models, an earth
##   collar around dug-in vehicles, and the preview of a line being drawn
##   (length, pieces, total price).
## - hides the model of a demolished bridge and shows it again once repaired.
##
## Created by `main.gd`.

const TrenchBuilder := preload("res://scenes/world/trench_builder.gd")
const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const REFRESH := 0.25
## Kinds drawn by `trench_builder.gd`.
const DUG := ["trench", "comm_trench", "foxhole", "mg_nest", "dugout", "vehicle_pit"]
## Ends of trench pieces closer than this (m) are joined.
const JOIN_M := 0.3
## Colors of the line preview.
const PREVIEW := Color(1.0, 0.85, 0.3, 0.45)
## Colors of mine markers and sites.
const OWN_MINE := Color(1.0, 0.85, 0.2)
const ENEMY_MINE := Color(0.95, 0.2, 0.15)
const SITE := Color(0.95, 0.85, 0.35, 0.35)
## Length of one wire model (catalog `fort_barbed_wire`, 3 m).
const WIRE_MODEL_M := 3.0

var main: Node3D
var sim_host: SimHost
## Counters for the self-test.
var mine_markers := 0
var work_nodes := 0
var dust_emitters := 0
var tracks_drawn := 0
var bridges_hidden := 0
## Prompt 052, for the self-test.
var trench_meshes := 0
var dug_in_drawn := 0
var preview_text := ""

var _refresh_left := 0.0
var _mines: MultiMeshInstance3D
## Work id -> node, with its state signature.
var _works: Dictionary[int, Node3D] = {}
var _work_keys: Dictionary[int, String] = {}
## Vehicle id -> track node.
var _tracks: Dictionary[int, Node3D] = {}
var _bridges_intact := PackedByteArray()
## Joints of trench pieces: work id -> {miter_from, miter_to, bay_from, bay_to}.
var _joints: Dictionary[int, Dictionary] = {}
## All works of the last refresh (dugout entrances, nest facings).
var _all_works: Array[Dictionary] = []
var _damaged: Dictionary[int, bool] = {}
## Vehicle id -> earth collar node.
var _collars: Dictionary[int, Node3D] = {}
var _preview: MeshInstance3D
var _preview_label: Label3D


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_mines = MultiMeshInstance3D.new()
	_mines.name = "Mines"
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var disc := CylinderMesh.new()
	disc.top_radius = 1.0
	disc.bottom_radius = 1.0
	disc.height = 0.06
	disc.radial_segments = 12
	disc.rings = 1
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	disc.material = mat
	mm.mesh = disc
	_mines.multimesh = mm
	_mines.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_mines)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_draw_preview()
	_refresh_left -= delta
	if _refresh_left > 0.0:
		return
	_refresh_left = REFRESH
	refresh()


## Brings everything up to date (also called by the self-test).
func refresh() -> void:
	_draw_mines()
	_draw_works()
	_draw_tracks()
	_update_bridges()
	_draw_dug_in()


func _draw_mines() -> void:
	var mines: Array[Dictionary] = sim_host.known_mines()
	var mm := _mines.multimesh
	mm.instance_count = mines.size()
	for i in mines.size():
		var m: Dictionary = mines[i]
		var r := 0.6 if m.anti_tank else 0.35
		var p: Vector3 = m.position
		mm.set_instance_transform(i, Transform3D(Basis().scaled(Vector3(r, 1.0, r)), p + Vector3(0, 0.12, 0)))
		mm.set_instance_color(i, OWN_MINE if m.friendly else ENEMY_MINE)
	mine_markers = mines.size()


func _draw_works() -> void:
	var seen: Dictionary[int, bool] = {}
	var dust := 0
	_all_works = sim_host.field_works()
	_damaged.clear()
	for id: int in sim_host.damaged_works():
		_damaged[id] = true
	_joints = _trench_joints(_all_works)
	for w: Dictionary in _all_works:
		var id: int = w.id
		seen[id] = true
		var key := "%s:%s:%s:%s" % [w.kind, w.built, w.active, _damaged.has(id)]
		if _joints.has(id):
			key += ":%s" % str(_joints[id])
		if _work_keys.get(id, "") != key:
			if _works.has(id):
				_works[id].queue_free()
			var node := _work_node(w)
			add_child(node)
			_works[id] = node
			_work_keys[id] = key
		if w.active and not w.built:
			dust += 1
	for id: int in _works.keys():
		if not seen.has(id):
			_works[id].queue_free()
			_works.erase(id)
			_work_keys.erase(id)
	work_nodes = _works.size()
	dust_emitters = dust
	var meshes := 0
	for node: Node3D in _works.values():
		for child in node.get_children():
			if child is MeshInstance3D and (child as MeshInstance3D).material_override == TrenchBuilder.material():
				meshes += 1
	trench_meshes = meshes


## The node of one work: models when built, an outline while it is a site.
func _work_node(w: Dictionary) -> Node3D:
	var node := Node3D.new()
	node.name = "Work_%d" % w.id
	var from: Vector3 = w.from
	var to: Vector3 = w.to
	var along := to - from
	along.y = 0.0
	var length := along.length()
	var dir := along / length if length > 0.01 else Vector3(1, 0, 0)
	var yaw := atan2(-dir.x, -dir.z)
	var kind: String = w.kind
	if w.built:
		match kind:
			"wire":
				var n := maxi(1, int(round(length / WIRE_MODEL_M)))
				for k in n:
					var t := (float(k) + 0.5) / float(n)
					var p := from.lerp(to, t)
					_add_model(node, "fort_barbed_wire", "Segment", p, yaw)
			"hedgehogs":
				_add_model(node, "fort_hedgehog", "Hedgehog", from, yaw)
			"teeth":
				_add_model(node, "fort_dragon_teeth", "Tooth", from, yaw + float(w.id % 7) * 0.05)
			"camo_net":
				_add_model(node, "fort_camo_net", "Net", from, float(w.id % 13) * 0.4)
			"trench", "comm_trench":
				var opts: Dictionary = _joints.get(w.id, {}).duplicate()
				opts["comm"] = kind == "comm_trench"
				opts["damaged"] = _damaged.has(w.id)
				opts["path"] = kind == "trench"
				node.add_child(TrenchBuilder.trench(sim_host, from, to, opts))
			"foxhole":
				node.add_child(TrenchBuilder.foxhole(sim_host, from))
			"mg_nest":
				node.add_child(TrenchBuilder.nest(sim_host, from, _away_from_trench(from), _damaged.has(w.id)))
			"dugout":
				node.add_child(TrenchBuilder.dugout(sim_host, from, _to_comm_trench(from)))
			"vehicle_pit":
				node.add_child(TrenchBuilder.pit(sim_host, from))
		return node
	if not w.own:
		return node
	var width := 1.4
	if kind == "mines_at" or kind == "mines_ap":
		width = maxf(length, 4.0)
	if kind in ["mg_nest", "dugout", "camo_net", "vehicle_pit", "bunker"] and length < 0.01:
		width = {"mg_nest": 3.0, "dugout": 4.0, "camo_net": 8.8, "vehicle_pit": 7.0, "bunker": 4.4}[kind]
	node.add_child(_strip(from, to, width, 0.05, SITE))
	if w.active and kind in DUG:
		node.add_child(_dust(from.lerp(to, 0.5)))
	return node


func _add_model(parent: Node3D, id: String, variant: String, at: Vector3, yaw: float) -> void:
	var objects: Node = main.get("map_objects")
	if objects == null or not objects.has_method("_variant_node"):
		return
	var model: Node3D = objects._variant_node(id, variant, Transform3D(Basis(Vector3.UP, yaw), at))
	if model != null:
		parent.add_child(model)


## A flat box from `from` to `to`, `width` across (outlines, parapets).
func _strip(from: Vector3, to: Vector3, width: float, height: float, color: Color) -> MeshInstance3D:
	var along := to - from
	along.y = 0.0
	var length := maxf(along.length(), width)
	var box := BoxMesh.new()
	box.size = Vector3(width, height, length)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	if color.a < 1.0:
		mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	box.material = mat
	var mesh := MeshInstance3D.new()
	mesh.mesh = box
	mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var mid := from.lerp(to, 0.5)
	var yaw := atan2(along.x, along.z) if along.length() > 0.01 else 0.0
	mesh.transform = Transform3D(Basis(Vector3.UP, yaw), mid + Vector3(0, height * 0.5 + 0.05, 0))
	return mesh


## Earth thrown up by digging engineers.
func _dust(at: Vector3) -> CPUParticles3D:
	var p := CPUParticles3D.new()
	p.amount = 16
	p.lifetime = 1.2
	p.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
	p.emission_sphere_radius = 1.5
	p.direction = Vector3.UP
	p.spread = 35.0
	p.initial_velocity_min = 1.0
	p.initial_velocity_max = 2.5
	p.gravity = Vector3(0, -4.0, 0)
	p.scale_amount_min = 0.15
	p.scale_amount_max = 0.35
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.36, 0.29, 0.2, 0.8)
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	quad.material = mat
	p.mesh = quad
	p.position = at + Vector3(0, 0.3, 0)
	return p


## A thrown track lies beside every vehicle that hit an anti-tank mine; it
## is a child of the vehicle's node, so it hides with it under fog of war.
func _draw_tracks() -> void:
	var off := {}
	for id: int in sim_host.tracks_off():
		off[id] = true
		if _tracks.has(id) and is_instance_valid(_tracks[id]):
			continue
		var unit: Node3D = main._unit_nodes.get(id)
		if unit == null:
			continue
		var box := BoxMesh.new()
		box.size = Vector3(0.45, 0.08, 3.2)
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(0.16, 0.15, 0.14)
		mat.roughness = 0.9
		box.material = mat
		var track := MeshInstance3D.new()
		track.name = "ThrownTrack"
		track.mesh = box
		# Beside the hull (models face -Z), slightly askew on the ground.
		track.transform = Transform3D(Basis(Vector3.UP, 0.25), Vector3(2.1, 0.05, 1.2))
		unit.add_child(track)
		_tracks[id] = track
	for id: int in _tracks.keys():
		if not off.has(id):
			if is_instance_valid(_tracks[id]):
				_tracks[id].queue_free()
			_tracks.erase(id)
	tracks_drawn = _tracks.size()


## Joints of the finished trench pieces: the mitered across vector at
## each end that meets another piece, and whether the end is a firing bay
## (the line turns there: a zigzag corner).
func _trench_joints(works: Array[Dictionary]) -> Dictionary[int, Dictionary]:
	var pieces: Array[Dictionary] = []
	for w in works:
		if w.built and (w.kind == "trench" or w.kind == "comm_trench"):
			pieces.append(w)
	var out: Dictionary[int, Dictionary] = {}
	for w in pieces:
		var d := _dir(w)
		var across := Vector3(-d.z, 0, d.x)
		var joint := {}
		for end: Array in [["from", -1.0], ["to", 1.0]]:
			var p: Vector3 = w[end[0]]
			for o in pieces:
				if o.id == w.id:
					continue
				var od := _dir(o)
				# The direction the line runs through the joint.
				var c: Vector3
				if end[1] > 0.0:
					if (o.from as Vector3).distance_to(p) < JOIN_M:
						c = od
					elif (o.to as Vector3).distance_to(p) < JOIN_M:
						c = -od
					else:
						continue
				else:
					if (o.to as Vector3).distance_to(p) < JOIN_M:
						c = od
					elif (o.from as Vector3).distance_to(p) < JOIN_M:
						c = -od
					else:
						continue
				var m := (across + Vector3(-c.z, 0, c.x)).normalized()
				var cosine := m.dot(across)
				if cosine > 0.3:
					joint["miter_" + end[0]] = m / cosine
				joint["bay_" + end[0]] = absf(d.dot(c)) < 0.995 and o.kind == "trench" and w.kind == "trench"
				break
		if not joint.is_empty():
			out[w.id] = joint
	return out


static func _dir(w: Dictionary) -> Vector3:
	var d: Vector3 = (w.to as Vector3) - (w.from as Vector3)
	d.y = 0.0
	return d.normalized() if d.length() > 0.01 else Vector3(0, 0, 1)


## From the nearest finished trench towards `at` (an MG nest faces out).
func _away_from_trench(at: Vector3) -> Vector3:
	var best := INF
	var out := Vector3.ZERO
	for w in _all_works:
		if not w.built or not (w.kind == "trench" or w.kind == "comm_trench"):
			continue
		var p := Geometry3D.get_closest_point_to_segment(at, w.from, w.to)
		var d := p.distance_to(at)
		if d < best:
			best = d
			out = at - p
	out.y = 0.0
	return out


## From `at` towards the nearest end of a communication trench.
func _to_comm_trench(at: Vector3) -> Vector3:
	var best := INF
	var out := Vector3.ZERO
	for w in _all_works:
		if w.kind != "comm_trench":
			continue
		for p: Vector3 in [w.from, w.to]:
			var d := p.distance_to(at)
			if d < best:
				best = d
				out = p - at
	out.y = 0.0
	return out


## An earth collar around every dug-in vehicle (a child of its node, so it
## hides with it under fog of war): the hull disappears behind it.
func _draw_dug_in() -> void:
	var dug := {}
	for id: int in sim_host.dug_in_units():
		dug[id] = true
		if _collars.has(id) and is_instance_valid(_collars[id]):
			continue
		var unit: Node3D = main._unit_nodes.get(id)
		if unit == null:
			continue
		var torus := TorusMesh.new()
		torus.inner_radius = 2.6
		torus.outer_radius = 4.2
		torus.rings = 16
		torus.ring_segments = 6
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(0.43, 0.34, 0.23)
		mat.roughness = 1.0
		torus.material = mat
		var collar := MeshInstance3D.new()
		collar.name = "DugInCollar"
		collar.mesh = torus
		collar.scale = Vector3(0.85, 0.55, 1.35)
		collar.position = Vector3(0, 0.3, 0)
		unit.add_child(collar)
		_collars[id] = collar
	for id: int in _collars.keys():
		if not dug.has(id):
			if is_instance_valid(_collars[id]):
				_collars[id].queue_free()
			_collars.erase(id)
	dug_in_drawn = _collars.size()


## The line an engineer order is drawing: a strip from its start to the
## cursor with length, pieces and the total price (prompt 052).
func _draw_preview() -> void:
	var selection: Control = main.get("selection")
	var line: Dictionary = {}
	if selection != null:
		var hit: Variant = null
		var camera: Camera3D = selection.get("camera")
		if camera != null:
			hit = GroundPick.from_screen(camera, get_viewport().get_mouse_position())
		if hit != null:
			line = EngineerOrders.line_preview(selection, hit as Vector3)
	if line.is_empty():
		if _preview != null:
			_preview.visible = false
			_preview_label.visible = false
		preview_text = ""
		return
	if _preview == null:
		_preview = MeshInstance3D.new()
		_preview.name = "LinePreview"
		_preview.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(_preview)
		_preview_label = Label3D.new()
		_preview_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		_preview_label.no_depth_test = true
		_preview_label.font_size = 40
		_preview_label.outline_size = 8
		add_child(_preview_label)
	var from: Vector3 = line.from
	var to: Vector3 = line.to
	var node := _strip(from, to, 1.0, 0.08, PREVIEW)
	_preview.mesh = node.mesh
	_preview.transform = node.transform
	node.free()
	_preview.visible = true
	preview_text = line.text
	_preview_label.text = preview_text
	_preview_label.position = to + Vector3(0, 3.0, 0)
	_preview_label.visible = true


## Hides the models of destroyed bridges in the map objects.
func _update_bridges() -> void:
	var intact := sim_host.bridges_intact()
	if intact == _bridges_intact:
		return
	var objects: Node = main.get("map_objects")
	var indices := sim_host.bridge_objects()
	var hidden := 0
	for i in intact.size():
		# Bridges laid by bridge-laying tanks (prompt 055) have no map
		# object; `crossing_view.gd` draws them.
		if indices[i] >= 4294967295:
			continue
		var was := _bridges_intact[i] if i < _bridges_intact.size() else 1
		if objects != null and objects.has_method("_instance_transform") and intact[i] != was:
			objects._instance_transform(indices[i], intact[i] == 0)
		if intact[i] == 0:
			hidden += 1
	_bridges_intact = intact
	bridges_hidden = hidden
