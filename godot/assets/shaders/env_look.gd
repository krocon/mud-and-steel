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

const BIOMES: Array[String] = ["forest", "desert", "winter", "mediterranean"]
## Instance uniforms of env.gdshader / env_foliage.gdshader per biome.
const LOOK: Dictionary[String, Dictionary] = {
	"forest": {"foliage": Color(0.25, 0.36, 0.13), "dry": 0.0, "snow": 0.0, "dust": 0.0},
	"desert": {"foliage": Color(0.33, 0.36, 0.17), "dry": 0.55, "snow": 0.0, "dust": 0.45},
	"winter": {"foliage": Color(0.17, 0.25, 0.13), "dry": 0.15, "snow": 0.85, "dust": 0.0},
	# Prompt 120: darker, leathery evergreen leaves, a little summer dryness
	# (no dust: it would turn plaster into the desert's mud brick).
	"mediterranean": {"foliage": Color(0.22, 0.31, 0.12), "dry": 0.12, "snow": 0.0, "dust": 0.0},
}
const NEUTRAL_PAINT := Color(0.33, 0.35, 0.32)
## Bend of the whole plant in the wind per catalog id (prompt 086,
## `env_bend`): conifers are stiff (the tip swings, the trunk hardly),
## birches soft, palms swing their fronds; everything else does not bend
## (bushes and hedges sway only with their branches).
const BEND: Dictionary[String, float] = {
	"env_tree_deciduous": 1.0, "env_tree_conifer": 0.6, "env_tree_birch": 1.35,
	"env_tree_palm": 1.15,
	# Prompt 120: the umbrella pine and the olive are stiff, the cypress
	# column sways a little.
	"env_tree_stone_pine": 0.5, "env_tree_cypress": 0.8, "env_tree_olive": 0.5,
}
## Materials whose wind the trees use (prompt 086): `still_trees` stops them
## for pictures that must be the same every time (thumbnails, gallery).
const TREE_WIND_MATERIALS: Array[String] = [
	"res://assets/materials/env/bark.tres",
	"res://assets/materials/env/foliage.tres",
]
## Distance (m) at which tree variants switch to their `_Far` mesh.
const FAR_DISTANCE := 180.0
## Surfaces per building (prompt 186): names of the blueprint's `[surface]`
## values; the index + 1 goes to the shader (0 = the look before, chosen
## per house). Same lists as `SURFACE_NAMES` in tools/blender/kit/env.py.
const PLASTERS: Array[String] = ["lime", "cream", "ochre", "ochre_red", "pink", "grey_green", "white"]
const BRICKS: Array[String] = ["red", "yellow", "dark"]
const BONDS: Array[String] = ["stretcher", "english"]
const ROOFS: Array[String] = ["clay", "mission", "beaver", "spanish", "slate"]
## Instance uniforms of a building without a `[surface]` table.
const SURFACE_DEFAULT: Array[Vector4] = [Vector4(0, 0, 0, 0), Vector4(0, 0, 0, 1)]


## Last biome passed to `apply_global()` (used by headless tests where
## `RendererDummy` returns `null` from `global_shader_parameter_get`).
static var current_biome := "forest"
static var current_look: Dictionary = LOOK["forest"]


## Sets the global biome shader uniforms (`env_foliage`, `env_dry`, `env_snow`,
## `env_dust`, prompt 208) for all environment shaders.
static func apply_global(biome: String) -> void:
	var key := biome if LOOK.has(biome) else "forest"
	var look: Dictionary = LOOK[key]
	current_biome = key
	current_look = look
	RenderingServer.global_shader_parameter_set(&"env_foliage", look.foliage)
	RenderingServer.global_shader_parameter_set(&"env_dry", float(look.dry))
	RenderingServer.global_shader_parameter_set(&"env_snow", float(look.snow))
	RenderingServer.global_shader_parameter_set(&"env_dust", float(look.dust))


## Active global `env_snow` value, falling back to `current_look` under the
## headless dummy renderer where `global_shader_parameter_get` returns `null`.
static func global_snow() -> float:
	var val: Variant = RenderingServer.global_shader_parameter_get(&"env_snow")
	return float(val) if val != null else float(current_look.snow)


## Sets the biome look globally and per-mesh paint/surface/roof bounds below
## `model`. `paint` recolors the Paint slot (military objects take a faction
## palette base color).
static func apply(model: Node, biome: String, paint := NEUTRAL_PAINT) -> void:
	apply_global(biome)
	var surface := surface_of(model)
	var custom := surface != SURFACE_DEFAULT
	for mesh in UnitCamo.meshes(model):
		if custom:
			apply_surface(mesh, surface)
		mesh.set_instance_shader_parameter(&"env_paint", paint)
		set_roof_box(mesh)


## The two surface uniforms (`env_surface_a`, `env_surface_b`,
## env_surface.gdshaderinc) of a model from its `env_surface` extra, or the
## defaults.
static func surface_of(model: Node) -> Array[Vector4]:
	var raw: Variant = root_extras(model).get("env_surface", "")
	if str(raw).is_empty():
		return SURFACE_DEFAULT
	var table: Variant = JSON.parse_string(str(raw))
	if not table is Dictionary:
		return SURFACE_DEFAULT
	return surface_from(table)


## The two surface uniforms of a `[surface]` table (names as in the
## blueprint; missing keys keep the look before prompt 186).
static func surface_from(t: Dictionary) -> Array[Vector4]:
	return [
		Vector4(PLASTERS.find(str(t.get("plaster", ""))) + 1, float(t.get("spall", 0.0)),
			BRICKS.find(str(t.get("brick", ""))) + 1, maxf(BONDS.find(str(t.get("bond", ""))), 0.0)),
		Vector4(ROOFS.find(str(t.get("roof", ""))) + 1, float(t.get("roof_row", 0.0)),
			float(t.get("roof_missing", 0.0)), float(t.get("moss", 1.0))),
	]


static func apply_surface(node: GeometryInstance3D, surface: Array[Vector4]) -> void:
	node.set_instance_shader_parameter(&"env_surface_a", surface[0])
	node.set_instance_shader_parameter(&"env_surface_b", surface[1])


## Sets the wind bend of a tree kind on `node` and its meshes (prompt 086).
static func apply_bend(node: Node, id: String) -> void:
	var bend: float = BEND.get(id, 0.0)
	if bend == 0.0:
		return
	if node is GeometryInstance3D:
		(node as GeometryInstance3D).set_instance_shader_parameter(&"env_bend", bend)
	for child in node.get_children():
		apply_bend(child, id)


## No wind on the trees (prompt 086): thumbnails and the model gallery
## render them at rest, so the pictures are the same every time.
static func still_trees() -> void:
	for path in TREE_WIND_MATERIALS:
		var mat := load(path) as ShaderMaterial
		if mat:
			mat.set_shader_parameter("wind_strength", 0.0)


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
