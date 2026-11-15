@tool
extends EditorScenePostImport
## Import script for unit models built by the Blender kit (tools/blender/kit).
##
## Set as default for scene imports in project.godot ([importer_defaults]).
## Acts only on models whose root carries the kit's glTF extras
## (`kit_version`); everything else is left untouched. Replaces the slot
## materials (Paint, Metal, Rubber, ...) with the shared unit_camo shader
## materials from res://assets/materials/unit/.
## Environment objects (glTF extras `env_kit_version`, prompt 007) get the
## env shader materials from res://assets/materials/env/ instead.
## Aircraft (prompt 008): the `ShadowProxy` mesh becomes shadow-only and
## hidden; the game shows it (and turns the real meshes' shadows off) when
## the aircraft flies high.

const MATERIAL_DIR := "res://assets/materials/unit/"
const ENV_MATERIAL_DIR := "res://assets/materials/env/"


func _post_import(scene: Node) -> Object:
	var env := _has_extra(scene, "env_kit_version")
	if not env and not _is_kit_model(scene):
		return scene
	var dir := ENV_MATERIAL_DIR if env else MATERIAL_DIR
	var replaced := 0
	var unknown: Array[String] = []
	for node in _all(scene):
		var mi := node as MeshInstance3D
		if mi == null or mi.mesh == null:
			continue
		for i in mi.mesh.get_surface_count():
			var old := mi.mesh.surface_get_material(i)
			var slot := old.resource_name if old else ""
			var path := dir + slot.to_lower() + ".tres"
			if slot.is_empty() or not ResourceLoader.exists(path):
				unknown.append("%s/%s" % [mi.name, slot])
				continue
			mi.mesh.surface_set_material(i, load(path))
			replaced += 1
		if mi.name == "ShadowProxy":
			mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
			mi.visible = false
	if not unknown.is_empty():
		push_error("unit_post_import: %s: unknown material slots %s" % [get_source_file(), unknown])
	print("unit_post_import: %s: %d surfaces use the %s shader" % [
		get_source_file(), replaced, "env" if env else "unit_camo"])
	return scene


func _is_kit_model(scene: Node) -> bool:
	return _has_extra(scene, "kit_version")


func _has_extra(scene: Node, key: String) -> bool:
	for node in _all(scene):
		if node.has_meta(&"extras") and (node.get_meta(&"extras") as Dictionary).has(key):
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
