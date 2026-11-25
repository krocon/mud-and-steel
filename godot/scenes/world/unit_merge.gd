extends RefCounted
## Merges the static parts of a vehicle model into few meshes (prompt 016).
## Presentation only.
##
## A kit model has 30–50 mesh parts (hull, wheels, rollers, tracks,
## hatches, ...), each its own draw call in every pass and its own set of
## camouflage instance uniforms. Only the turret and the gun move at run
## time (`combat_view.gd`; recoil and raised barrels since prompt 077,
## `effects.gd`), so the parts outside the turret become one mesh on the
## model root, the parts of the turret (cupola, MGs) one mesh on the
## `Turret` node and the `Gun` (mantlet, barrel, muzzle brake) one on the
## `Gun` node, with one surface per material.
##
## Prompt 077: whether the gun has a muzzle brake is read from the gun's
## mesh once per unit type (`model.get_meta(&"muzzle_brake")`): the kit
## (tools/blender/kit/weapons.py) builds a brake 1.35 to 1.9 barrel radii
## wide at the muzzle, a plain barrel ends at most 1.15 radii wide (thread
## protector), so no new data path is needed. Original
## MeshInstance3D nodes that carry sockets (muzzle, exhaust, fire,
## markings) or the turret stay as empty nodes; the others are freed. The merged meshes
## are built once per unit type with automatic LODs and shared by all
## units of that type. Shadow-only parts (`ShadowProxy`) stay as they are.

## Merged meshes per catalog id: Array of {"parent": NodePath from the
## model root ("." or the turret's path), "mesh": ArrayMesh, "shadow": cast
## shadow setting}.
static var _cache: Dictionary[String, Array] = {}
## Whether the gun of a catalog id has a muzzle brake (prompt 077).
static var _brakes: Dictionary[String, bool] = {}
## Track links per meter per catalog id (prompt 204; 0.0 for non-tracked units).
static var _track_lpm: Dictionary[String, float] = {}
## Fallback track links per meter when a track mesh has no usable UV edge (1 / 0.15 m).
const DEFAULT_TRACK_LPM := 1.0 / 0.15
## Shared combined material reading slot from UV2.x (prompt 207).
const UNIT_CAMO_SHADER := preload("res://assets/shaders/unit_camo.gdshader")
const COMBINED_MATERIAL := preload("res://assets/materials/unit/combined.tres")
## A muzzle at least this many barrel radii wide is a muzzle brake; the
## slotted brake (1.35) and flash hiders stay below (judged: they throw no
## visible side blasts).
const BRAKE_RATIO := 1.45
## Behind the muzzle face (m): the brake lies within BRAKE_ZONE_M; the
## barrel's radius is the narrowest ring of vertices within BARREL_ZONE_M
## (the kit's cylinders have rings only at their ends).
const BRAKE_ZONE_M := 0.45
const BARREL_ZONE_M := Vector2(0.1, 1.5)
const SLICE_M := 0.05


## Replaces the parts of `model` (an instance of a kit .glb) by merged
## meshes. Returns the number of parts merged (0: nothing to do).
static func merge(model: Node3D, type_id: String) -> int:
	# Tanks carry "Turret"; towed AA guns (flaks) carry a 360-degree "Mount"
	# on top of their static "Carriage" (prompt 204).
	var turret := model.find_child("Turret", true, false) as Node3D
	if turret == null:
		turret = model.find_child("Mount", true, false) as Node3D
	var gun := model.find_child("Gun", true, false) as Node3D
	var parts := _parts(model)
	if not _brakes.has(type_id):
		_brakes[type_id] = has_muzzle_brake(gun)
	model.set_meta(&"muzzle_brake", _brakes[type_id])
	if not _track_lpm.has(type_id):
		_track_lpm[type_id] = _measure_track_lpm(parts)
	model.set_meta(&"track_links_per_m", _track_lpm[type_id])
	if parts.size() < 2:
		return 0
	if not _cache.has(type_id):
		_cache[type_id] = _build(model, turret, gun, parts)
	for entry: Dictionary in _cache[type_id]:
		var instance := MeshInstance3D.new()
		instance.name = "Merged"
		instance.mesh = entry["mesh"]
		instance.cast_shadow = entry["shadow"]
		model.get_node(entry["parent"] as NodePath).add_child(instance)
	for part in parts:
		part.mesh = null
	# Emptied parts without children (wheels, rollers, hatches, ...) go
	# away: every node below a moving unit is notified of every move.
	# Parts that carry sockets, the turret or a merged mesh stay.
	for i in range(parts.size() - 1, -1, -1):
		# Freeing a leaf may free its emptied parents, which come later.
		if is_instance_valid(parts[i]):
			_free_if_empty(parts[i])
	return parts.size()


static func _free_if_empty(node: Node) -> void:
	while node != null and node is MeshInstance3D and (node as MeshInstance3D).mesh == null \
			and node.get_child_count() == 0:
		var parent := node.get_parent()
		parent.remove_child(node)
		node.free()
		node = parent


## Mesh parts that can be merged: every MeshInstance3D with a mesh, except
## shadow-only proxies.
static func _parts(model: Node) -> Array[MeshInstance3D]:
	var out: Array[MeshInstance3D] = []
	var stack: Array[Node] = [model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		# The carried bridge of a bridge-laying tank moves while it is laid
		# and hides while it lies across a gap (prompt 055): not merged.
		# The bow ramp of a landing craft (prompt 119) opens: not merged.
		if n.name == &"Bridge" or n.name == &"Ramp":
			continue
		stack.append_array(n.get_children())
		var mesh := n as MeshInstance3D
		if mesh != null and mesh.mesh != null \
				and mesh.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY:
			out.append(mesh)
	return out


## Builds the merged meshes: parts are grouped by their carrier (model root,
## turret/mount or gun) and cast-shadow setting. All `unit_camo` surfaces of a
## carrier are combined into a single indexed surface encoding `slot` in UV2.x
## (prompt 207), reducing a tank from ~12–18 surfaces to 3.
static func _build(model: Node3D, turret: Node3D, gun: Node3D, parts: Array[MeshInstance3D]) -> Array:
	# Key "carrier|shadow" -> {carrier, shadow, tools: {material -> SurfaceTool}}.
	var groups := {}
	for part in parts:
		var carrier: Node3D = model
		if gun != null and (part == gun or gun.is_ancestor_of(part)):
			carrier = gun
		elif turret != null and (part == turret or turret.is_ancestor_of(part)):
			carrier = turret
		var key := "%s|%d" % [carrier.name, part.cast_shadow]
		if not groups.has(key):
			groups[key] = {"carrier": carrier, "shadow": part.cast_shadow, "tools": {}}
		var tools: Dictionary = groups[key]["tools"]
		var xform := _relative(part, carrier)
		for s in part.mesh.get_surface_count():
			var material := part.get_active_material(s)
			var slot_id := _unit_camo_slot(material)
			var target_mat: Material = COMBINED_MATERIAL if slot_id >= 0 else material
			if not tools.has(target_mat):
				var st := SurfaceTool.new()
				st.begin(Mesh.PRIMITIVE_TRIANGLES)
				tools[target_mat] = st
			var st: SurfaceTool = tools[target_mat]
			if slot_id >= 0:
				var stamped := _stamp_slot_uv2(part.mesh, s, slot_id)
				if stamped != null:
					st.append_from(stamped, 0, xform)
			else:
				st.append_from(part.mesh, s, xform)
	var out := []
	for key: String in groups:
		var group: Dictionary = groups[key]
		var importer := ImporterMesh.new()
		for material: Material in group["tools"]:
			var st: SurfaceTool = group["tools"][material]
			st.index()
			importer.add_surface(Mesh.PRIMITIVE_TRIANGLES, st.commit_to_arrays(), [], {}, material)
		# Same angles as the glTF importer's defaults.
		importer.generate_lods(25.0, 60.0, [])
		var carrier: Node3D = group["carrier"]
		out.append({
			"parent": model.get_path_to(carrier) if carrier != model else NodePath("."),
			"mesh": importer.get_mesh(),
			"shadow": group["shadow"],
		})
	return out


## Returns the `unit_camo` slot index (0..7) for `material`, or -1 if it is
## not a single-slot `unit_camo` material.
static func _unit_camo_slot(material: Material) -> int:
	var sm := material as ShaderMaterial
	if sm == null or sm.shader != UNIT_CAMO_SHADER:
		return -1
	var v: Variant = sm.get_shader_parameter(&"slot")
	if v == null:
		return 0
	var s := int(v)
	return s if s >= 0 else -1


## Returns a 1-surface ArrayMesh copied from `mesh` surface `s` with uniform
## vertex attributes (VERTEX, NORMAL, COLOR, UV, UV2) and `UV2.x = slot_id`.
static func _stamp_slot_uv2(mesh: Mesh, s: int, slot_id: int) -> ArrayMesh:
	var arrays := mesh.surface_get_arrays(s)
	var verts: Variant = arrays[Mesh.ARRAY_VERTEX]
	if not (verts is PackedVector3Array) or (verts as PackedVector3Array).is_empty():
		return null
	var n := (verts as PackedVector3Array).size()
	if not (arrays[Mesh.ARRAY_NORMAL] is PackedVector3Array):
		var norms := PackedVector3Array()
		norms.resize(n)
		norms.fill(Vector3.UP)
		arrays[Mesh.ARRAY_NORMAL] = norms
	if not (arrays[Mesh.ARRAY_COLOR] is PackedColorArray):
		var cols := PackedColorArray()
		cols.resize(n)
		cols.fill(Color.WHITE)
		arrays[Mesh.ARRAY_COLOR] = cols
	if not (arrays[Mesh.ARRAY_TEX_UV] is PackedVector2Array):
		var uvs := PackedVector2Array()
		uvs.resize(n)
		uvs.fill(Vector2.ZERO)
		arrays[Mesh.ARRAY_TEX_UV] = uvs
	var uv2s := PackedVector2Array()
	uv2s.resize(n)
	uv2s.fill(Vector2(float(slot_id), 0.0))
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	# Strip tangents so all surfaces share the exact same attribute mask
	# (`unit_camo.gdshader` uses derivative relief, not tangent normal maps).
	arrays[Mesh.ARRAY_TANGENT] = null
	var tmp := ArrayMesh.new()
	tmp.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return tmp


## Whether the mesh of `gun` widens at its muzzle (`Socket_Muzzle`, the
## barrel along -Z) to a muzzle brake: the widest vertex near the muzzle
## face against the widest a little further back.
static func has_muzzle_brake(gun: Node3D) -> bool:
	var mesh := gun as MeshInstance3D
	if mesh == null or mesh.mesh == null:
		return false
	var socket := gun.get_node_or_null("Socket_Muzzle") as Node3D
	if socket == null:
		return false
	var tip := socket.position
	# Widest vertex per slice along the barrel.
	var slices: Dictionary[int, float] = {}
	for s in mesh.mesh.get_surface_count():
		var vertices: PackedVector3Array = mesh.mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX]
		for v in vertices:
			var back := v.z - tip.z
			if back < -0.02 or back > BARREL_ZONE_M.y:
				continue
			var k := int(round(back / SLICE_M))
			slices[k] = maxf(slices.get(k, 0.0), Vector2(v.x - tip.x, v.y - tip.y).length())
	var muzzle := 0.0
	var barrel := INF
	for k: int in slices:
		var back := k * SLICE_M
		if back <= BRAKE_ZONE_M:
			muzzle = maxf(muzzle, slices[k])
		if back >= BARREL_ZONE_M.x:
			barrel = minf(barrel, slices[k])
	return barrel < INF and muzzle > barrel * BRAKE_RATIO


## Measures track links per meter from `Track_L` / `Track_R` UV.y spacing
## (prompt 204), returning 0.0 when the model has no track meshes.
static func _measure_track_lpm(parts: Array[MeshInstance3D]) -> float:
	var has_track := false
	var ratio_sum := 0.0
	var ratio_count := 0
	for part in parts:
		if not (part.name == &"Track_L" or part.name == &"Track_R"):
			continue
		has_track = true
		var mesh := part.mesh
		if mesh == null:
			continue
		for s in mesh.get_surface_count():
			var arrays := mesh.surface_get_arrays(s)
			var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			var uvs: Variant = arrays[Mesh.ARRAY_TEX_UV]
			var indices: Variant = arrays[Mesh.ARRAY_INDEX]
			if not (uvs is PackedVector2Array) or verts.is_empty():
				continue
			var uv_arr := uvs as PackedVector2Array
			var idx_arr := indices as PackedInt32Array if indices is PackedInt32Array else PackedInt32Array()
			var tri_count := (idx_arr.size() / 3) if not idx_arr.is_empty() else (verts.size() / 3)
			for t in mini(tri_count, 64):
				var i0 := idx_arr[t * 3] if not idx_arr.is_empty() else t * 3
				var i1 := idx_arr[t * 3 + 1] if not idx_arr.is_empty() else t * 3 + 1
				var i2 := idx_arr[t * 3 + 2] if not idx_arr.is_empty() else t * 3 + 2
				for edge: Array in [[i0, i1], [i1, i2], [i2, i0]]:
					var a: int = edge[0]
					var b: int = edge[1]
					var du := absf(uv_arr[a].x - uv_arr[b].x)
					var dv := absf(uv_arr[a].y - uv_arr[b].y)
					var dist := verts[a].distance_to(verts[b])
					if du < 0.01 and dv > 0.1 and dv < 10.0 and dist > 0.02:
						ratio_sum += dv / dist
						ratio_count += 1
						if ratio_count >= 16:
							return clampf(ratio_sum / float(ratio_count), 3.0, 14.0)
	if not has_track:
		return 0.0
	return clampf(ratio_sum / float(ratio_count), 3.0, 14.0) if ratio_count > 0 else DEFAULT_TRACK_LPM


## Transform of `node` relative to its ancestor `root` (the model is not in
## the scene tree yet, so global transforms are not available).
static func _relative(node: Node3D, root: Node3D) -> Transform3D:
	var xform := Transform3D.IDENTITY
	var n: Node = node
	while n != null and n != root:
		if n is Node3D:
			xform = (n as Node3D).transform * xform
		n = n.get_parent()
	return xform
