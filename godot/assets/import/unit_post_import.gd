@tool
extends EditorScenePostImport
## Import script for unit models built by the Blender kit (tools/blender/kit).
##
## Set as default for scene imports in project.godot ([importer_defaults]).
## Acts only on models whose root carries the kit's glTF extras
## (`kit_version`); everything else is left untouched. Replaces the slot
## materials (Paint, Metal, Rubber, ...) with the shared unit_camo shader
## materials from res://assets/materials/unit/.

const MATERIAL_DIR := "res://assets/materials/unit/"


func _post_import(scene: Node) -> Object:
	if not _is_kit_model(scene):
		return scene
	var replaced := 0
	var unknown: Array[String] = []
	for node in _all(scene):
		var mi := node as MeshInstance3D
		if mi == null or mi.mesh == null:
			continue
		for i in mi.mesh.get_surface_count():
			var old := mi.mesh.surface_get_material(i)
			var slot := old.resource_name if old else ""
			var path := MATERIAL_DIR + slot.to_lower() + ".tres"
			if slot.is_empty() or not ResourceLoader.exists(path):
				unknown.append("%s/%s" % [mi.name, slot])
				continue
			mi.mesh.surface_set_material(i, load(path))
			replaced += 1
	if not unknown.is_empty():
		push_error("unit_post_import: %s: unknown material slots %s" % [get_source_file(), unknown])
	print("unit_post_import: %s: %d surfaces use the unit_camo shader" % [get_source_file(), replaced])
	return scene


func _is_kit_model(scene: Node) -> bool:
	for node in _all(scene):
		if node.has_meta(&"extras") and (node.get_meta(&"extras") as Dictionary).has("kit_version"):
			return true
	return false


func _all(root: Node) -> Array[Node]:
	var out: Array[Node] = []
	var stack: Array[Node] = [root]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		out.append(n)
		stack.append_array(n.get_children())
	return out
