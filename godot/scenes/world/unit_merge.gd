extends RefCounted
## Merges the static parts of a vehicle model into few meshes (prompt 016).
## Presentation only.
##
## A kit model has 30–50 mesh parts (hull, wheels, rollers, tracks,
## hatches, ...), each its own draw call in every pass and its own set of
## camouflage instance uniforms. Only the turret moves at run time
## (`combat_view.gd`), so the parts outside the turret become one mesh on
## the model root and the parts of the turret (gun, cupola, MGs) one mesh on
## the `Turret` node, with one surface per material. Original
## MeshInstance3D nodes that carry sockets (muzzle, exhaust, fire,
## markings) or the turret stay as empty nodes; the others are freed. The merged meshes
## are built once per unit type with automatic LODs and shared by all
## units of that type. Shadow-only parts (`ShadowProxy`) stay as they are.

## Merged meshes per catalog id: Array of {"parent": NodePath from the
## model root ("." or the turret's path), "mesh": ArrayMesh, "shadow": cast
## shadow setting}.
static var _cache: Dictionary[String, Array] = {}


## Replaces the parts of `model` (an instance of a kit .glb) by merged
## meshes. Returns the number of parts merged (0: nothing to do).
static func merge(model: Node3D, type_id: String) -> int:
	var turret := model.find_child("Turret", true, false) as Node3D
	var parts := _parts(model)
	if parts.size() < 2:
		return 0
	if not _cache.has(type_id):
		_cache[type_id] = _build(model, turret, parts)
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
		if n.name == &"Bridge":
			continue
		stack.append_array(n.get_children())
		var mesh := n as MeshInstance3D
		if mesh != null and mesh.mesh != null \
				and mesh.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY:
			out.append(mesh)
	return out


## Builds the merged meshes: parts are grouped by their carrier (model root
## or turret) and cast-shadow setting, surfaces by material.
static func _build(model: Node3D, turret: Node3D, parts: Array[MeshInstance3D]) -> Array:
	# Key "carrier|shadow" -> {material -> SurfaceTool}, in first-seen order.
	var groups := {}
	for part in parts:
		var carrier: Node3D = turret if turret != null and (part == turret or turret.is_ancestor_of(part)) else model
		var key := "%s|%d" % ["turret" if carrier == turret else "root", part.cast_shadow]
		if not groups.has(key):
			groups[key] = {"carrier": carrier, "shadow": part.cast_shadow, "tools": {}}
		var tools: Dictionary = groups[key]["tools"]
		var xform := _relative(part, carrier)
		for s in part.mesh.get_surface_count():
			var material := part.get_active_material(s)
			if not tools.has(material):
				var st := SurfaceTool.new()
				st.begin(Mesh.PRIMITIVE_TRIANGLES)
				tools[material] = st
			(tools[material] as SurfaceTool).append_from(part.mesh, s, xform)
	var out := []
	for key: String in groups:
		var group: Dictionary = groups[key]
		var importer := ImporterMesh.new()
		for material: Material in group["tools"]:
			var st: SurfaceTool = group["tools"][material]
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
