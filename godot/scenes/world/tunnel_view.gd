extends Node3D
## Tunnel presentation (prompt 148): masonry portals (open / collapsed),
## subtle dashed alignment line on the ridge and surface markers above own /
## allied units inside a bore. Presentation only — all rules live in Rust.

const PORTAL_GLB := "res://assets/models/neutral/env_tunnel_portal.glb"
const DASH_LEN_M := 6.0
const GAP_LEN_M := 4.5
const MARKER_OWN := Color(0.28, 0.78, 1.0, 0.92)
const MARKER_ALLY := Color(0.32, 0.88, 0.48, 0.92)
const LINE_COLOR := Color(0.95, 0.88, 0.55, 0.65)

var sim_host: SimHost
var main: Node
var in_tunnel_ids: Dictionary = {}
var _tunnels: Array = []
var _portals_open := PackedByteArray()
var _portal_nodes: Array = [] # [{open: Node3D, collapsed: Node3D, pos: Vector3}]
var _line_node: MeshInstance3D
var _markers: Dictionary = {} # int -> MeshInstance3D
var _marker_mesh: Mesh
var _mat_own: StandardMaterial3D
var _mat_ally: StandardMaterial3D
var _portal_scene: PackedScene


func setup(host: SimHost, main_node: Node) -> void:
	sim_host = host
	main = main_node
	_clear()
	if sim_host == null:
		return
	_tunnels = sim_host.tunnels()
	if _tunnels.is_empty():
		return
	_portals_open = sim_host.tunnel_portals_open()
	if ResourceLoader.exists(PORTAL_GLB):
		_portal_scene = load(PORTAL_GLB) as PackedScene
	_build_portals()
	_build_alignment_lines()
	_init_marker_assets()


func _clear() -> void:
	for child in get_children():
		remove_child(child)
		child.queue_free()
	_tunnels.clear()
	_portals_open = PackedByteArray()
	_portal_nodes.clear()
	_markers.clear()
	in_tunnel_ids.clear()
	_line_node = null


func _init_marker_assets() -> void:
	var cyl := CylinderMesh.new()
	cyl.top_radius = 1.6
	cyl.bottom_radius = 0.2
	cyl.height = 2.4
	cyl.radial_segments = 6
	_marker_mesh = cyl
	_mat_own = StandardMaterial3D.new()
	_mat_own.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_mat_own.albedo_color = MARKER_OWN
	_mat_own.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_mat_own.no_depth_test = true
	_mat_ally = StandardMaterial3D.new()
	_mat_ally.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_mat_ally.albedo_color = MARKER_ALLY
	_mat_ally.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_mat_ally.no_depth_test = true


func _build_portals() -> void:
	if _portal_scene == null:
		return
	for ti in _tunnels.size():
		var tun: Dictionary = _tunnels[ti]
		var kind_prefix := "Rail" if String(tun.get("kind", "road")) == "rail" else "Road"
		var portals: Array = tun.get("portals", [])
		for pi in portals.size():
			var pd: Dictionary = portals[pi]
			var pos: Vector3 = pd["pos"]
			var yaw: float = float(pd["yaw"])
			var is_open: bool = bool(pd["open"])
			var holder := Node3D.new()
			holder.position = pos
			holder.rotation.y = yaw
			add_child(holder)
			var open_node := _extract_variant(kind_prefix)
			var col_node := _extract_variant(kind_prefix + "Collapsed")
			if open_node != null:
				open_node.visible = is_open
				holder.add_child(open_node)
			if col_node != null:
				col_node.visible = not is_open
				holder.add_child(col_node)
			_portal_nodes.append({"open": open_node, "collapsed": col_node, "pos": pos})


func _extract_variant(variant_name: String) -> Node3D:
	var root := _portal_scene.instantiate()
	var target := root.find_child(variant_name, true, false) as Node3D
	if target == null:
		root.free()
		return null
	if target.get_parent() != null:
		target.get_parent().remove_child(target)
	root.free()
	return target


func _build_alignment_lines() -> void:
	var im := ImmediateMesh.new()
	im.surface_begin(Mesh.PRIMITIVE_LINES)
	for ti in _tunnels.size():
		var tun: Dictionary = _tunnels[ti]
		var pts: PackedVector3Array = tun.get("points", PackedVector3Array())
		for s in range(pts.size() - 1):
			var a := pts[s]
			var b := pts[s + 1]
			var d := Vector2(b.x - a.x, b.z - a.z)
			var total := d.length()
			if total < 1.0:
				continue
			var t := 0.0
			while t < total:
				var t1 := minf(t + DASH_LEN_M, total)
				var p0 := a.lerp(b, t / total)
				var p1 := a.lerp(b, t1 / total)
				p0.y = sim_host.surface_height_at(p0.x, p0.z) + 0.65
				p1.y = sim_host.surface_height_at(p1.x, p1.z) + 0.65
				im.surface_add_vertex(p0)
				im.surface_add_vertex(p1)
				t += DASH_LEN_M + GAP_LEN_M
	im.surface_end()
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = LINE_COLOR
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.no_depth_test = true
	_line_node = MeshInstance3D.new()
	_line_node.mesh = im
	_line_node.material_override = mat
	_line_node.visible = false
	add_child(_line_node)


func _process(_delta: float) -> void:
	if sim_host == null or _tunnels.is_empty():
		return
	_sync_portals()
	_sync_underground_units()


func _sync_portals() -> void:
	var state := sim_host.tunnel_portals_open()
	if state == _portals_open:
		return
	for i in mini(state.size(), _portal_nodes.size()):
		var is_open := state[i] != 0
		var was_open := _portals_open[i] != 0 if i < _portals_open.size() else true
		var entry: Dictionary = _portal_nodes[i]
		if entry["open"] != null:
			(entry["open"] as Node3D).visible = is_open
		if entry["collapsed"] != null:
			(entry["collapsed"] as Node3D).visible = not is_open
		if was_open and not is_open and main != null:
			var fx: Node = main.get("effects")
			if fx != null and fx.has_method("spawn_dust"):
				fx.spawn_dust(entry["pos"] as Vector3, 6.0)
	_portals_open = state


func _sync_underground_units() -> void:
	var list := sim_host.units_in_tunnel()
	in_tunnel_ids.clear()
	for id in list:
		in_tunnel_ids[id] = true
	var show_line := not in_tunnel_ids.is_empty()
	if not show_line and main != null and main.get("selection") != null:
		var mode: String = String(main.selection.command_mode)
		if mode == "engineer_bridge" or mode == "engineer_tunnel":
			show_line = true
	if _line_node != null:
		_line_node.visible = show_line
	if in_tunnel_ids.is_empty():
		for m: MeshInstance3D in _markers.values():
			m.visible = false
		return
	var ids := sim_host.unit_ids()
	var pos := sim_host.current_positions()
	var owners := sim_host.unit_owners()
	var local := sim_host.local_player()
	var seen := {}
	for i in ids.size():
		var id := ids[i]
		if not in_tunnel_ids.has(id):
			continue
		var own := owners[i] == local
		var ally := (not own) and sim_host.relation(owners[i]) == 1
		if not own and not ally:
			continue
		seen[id] = true
		var marker: MeshInstance3D = _markers.get(id)
		if marker == null:
			marker = MeshInstance3D.new()
			marker.mesh = _marker_mesh
			add_child(marker)
			_markers[id] = marker
		marker.material_override = _mat_own if own else _mat_ally
		var p := pos[i]
		var sy := sim_host.surface_height_at(p.x, p.z)
		marker.position = Vector3(p.x, sy + 2.2, p.z)
		marker.visible = true
	for id: int in _markers.keys():
		if not seen.has(id):
			(_markers[id] as MeshInstance3D).visible = false
