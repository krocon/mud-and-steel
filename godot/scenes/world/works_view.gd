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
## - hides the model of a demolished bridge and shows it again once repaired.
##
## Created by `main.gd`.

const REFRESH := 0.25
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

var _refresh_left := 0.0
var _mines: MultiMeshInstance3D
## Work id -> node, with its state signature.
var _works: Dictionary[int, Node3D] = {}
var _work_keys: Dictionary[int, String] = {}
## Vehicle id -> track node.
var _tracks: Dictionary[int, Node3D] = {}
var _bridges_intact := PackedByteArray()


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
	for w: Dictionary in sim_host.field_works():
		var id: int = w.id
		seen[id] = true
		var key := "%s:%s:%s" % [w.kind, w.built, w.active]
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
			"foxhole", "trench":
				node.add_child(_strip(from, to, 0.6, 0.12, Color(0.32, 0.25, 0.17)))
		return node
	if not w.own:
		return node
	var width := 1.4
	if kind == "mines_at" or kind == "mines_ap":
		width = maxf(length, 4.0)
	node.add_child(_strip(from, to, width, 0.05, SITE))
	if w.active and (kind == "foxhole" or kind == "trench"):
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


## Hides the models of destroyed bridges in the map objects.
func _update_bridges() -> void:
	var intact := sim_host.bridges_intact()
	if intact == _bridges_intact:
		return
	var objects: Node = main.get("map_objects")
	var indices := sim_host.bridge_objects()
	var hidden := 0
	for i in intact.size():
		var was := _bridges_intact[i] if i < _bridges_intact.size() else 1
		if objects != null and objects.has_method("_instance_transform") and intact[i] != was:
			objects._instance_transform(indices[i], intact[i] == 0)
		if intact[i] == 0:
			hidden += 1
	_bridges_intact = intact
	bridges_hidden = hidden
