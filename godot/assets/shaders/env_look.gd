class_name EnvLook
extends RefCounted
## Biome look and variant access for environment objects (prompt 007).
##
## Environment models (res://assets/models/neutral/<id>.glb, built by
## tools/blender/build_env.py) carry glTF extras on their root node
## (`env_kit_version`, `env_variants`, catalog values `cat_*`). Every variant
## is a direct child of that node: a single MeshInstance3D (instanceable, one
## mesh resource per variant) or a Node3D with `<Variant>_Walls`, `_Roof`,
## `_Floor`, `_Debris` ... meshes. Trees have a `<Variant>_Far` mesh for large
## distances. Presentation only.

const BIOMES: Array[String] = ["forest", "desert", "winter"]
## Instance uniforms of env.gdshader / env_foliage.gdshader per biome.
const LOOK: Dictionary[String, Dictionary] = {
	"forest": {"foliage": Color(0.25, 0.36, 0.13), "dry": 0.0, "snow": 0.0, "dust": 0.0},
	"desert": {"foliage": Color(0.33, 0.36, 0.17), "dry": 0.55, "snow": 0.0, "dust": 0.45},
	"winter": {"foliage": Color(0.17, 0.25, 0.13), "dry": 0.15, "snow": 0.85, "dust": 0.0},
}
const NEUTRAL_PAINT := Color(0.33, 0.35, 0.32)
## Distance (m) at which tree variants switch to their `_Far` mesh.
const FAR_DISTANCE := 180.0


## Sets the biome look on every mesh below `model`. `paint` recolors the
## Paint slot (military objects take a faction palette base color).
static func apply(model: Node, biome: String, paint := NEUTRAL_PAINT) -> void:
	var look: Dictionary = LOOK.get(biome, LOOK["forest"])
	for mesh in UnitCamo.meshes(model):
		mesh.set_instance_shader_parameter(&"env_foliage", look.foliage)
		mesh.set_instance_shader_parameter(&"env_dry", look.dry)
		mesh.set_instance_shader_parameter(&"env_snow", look.snow)
		mesh.set_instance_shader_parameter(&"env_dust", look.dust)
		mesh.set_instance_shader_parameter(&"env_paint", paint)
		set_roof_box(mesh)


## The horizontal bounds of the mesh (min x, min z, max x, max z) as
## `env_roof_box`: the roof snow of env.gdshader (prompt 080) needs how far
## the eave lies from the ridge (the model origin).
static func set_roof_box(node: GeometryInstance3D) -> void:
	var mesh: Mesh = null
	if node is MultiMeshInstance3D:
		var mm := (node as MultiMeshInstance3D).multimesh
		mesh = mm.mesh if mm else null
	elif node is MeshInstance3D:
		mesh = (node as MeshInstance3D).mesh
	if mesh == null:
		return
	var box := mesh.get_aabb()
	node.set_instance_shader_parameter(&"env_roof_box", Vector4(box.position.x, box.position.z, box.end.x, box.end.z))


static func is_env_model(model: Node) -> bool:
	return root_extras(model).has("env_kit_version")


## glTF extras of the model root, or {}.
static func root_extras(model: Node) -> Dictionary:
	var stack: Array[Node] = [model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		if n.has_meta(&"extras"):
			var extras: Dictionary = n.get_meta(&"extras")
			if extras.has("env_kit_version"):
				return extras
		stack.append_array(n.get_children())
	return {}


## Variant list [{name, role, far}] in blueprint order.
static func variants(model: Node) -> Array:
	var raw: Variant = root_extras(model).get("env_variants", "[]")
	var parsed: Variant = JSON.parse_string(str(raw))
	return parsed if parsed is Array else []


## The node of one variant (mesh or group), or null.
static func variant_node(model: Node, variant: String) -> Node3D:
	return model.find_child(variant, true, false) as Node3D


## Hides all variants except `variant` (and its far mesh).
static func show_variant(model: Node, variant: String) -> void:
	for v: Dictionary in variants(model):
		var n := variant_node(model, v.name)
		if n:
			n.visible = v.name == variant
		var far := variant_node(model, v.name + "_Far")
		if far:
			far.visible = v.name == variant


## Switches near and far meshes of tree variants by camera distance
## (Godot visibility ranges with a short fade).
static func set_far_ranges(model: Node, distance := FAR_DISTANCE) -> void:
	for v: Dictionary in variants(model):
		if not v.get("far", false):
			continue
		var near := variant_node(model, v.name) as GeometryInstance3D
		var far := variant_node(model, v.name + "_Far") as GeometryInstance3D
		if near == null or far == null:
			continue
		near.visibility_range_end = distance
		near.visibility_range_end_margin = distance * 0.1
		near.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
		far.visibility_range_begin = distance
		far.visibility_range_begin_margin = distance * 0.1
		far.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
