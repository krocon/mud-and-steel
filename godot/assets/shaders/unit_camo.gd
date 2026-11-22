class_name UnitCamo
extends RefCounted
## Applies camouflage palettes and markings to imported unit models.
##
## Models built by the Blender kit (tools/blender/kit) carry glTF extras on
## their root node (`cat_id`, `cat_faction`, ...), slot materials replaced by
## the unit_camo shader on import, and Socket_Marking_* nodes whose local +Y
## is the surface normal. Presentation only.

const BIOMES: Array[String] = ["forest", "desert", "winter"]
const PALETTE_DIR := "res://assets/materials/camo/"
## German army vehicles were painted Dunkelgrau Nr. 46 (RAL 7021) until
## February 1943, then Dunkelgelb (RAL 7028) with camouflage colors. A
## German ground unit whose catalog `year_from` is before this year gets
## the early palette `de_<biome>_early` where there is one (forest,
## winter; the desert keeps its own colors). Aircraft and fortifications
## keep their palette.
const EARLY_UNTIL := 1943

static var _palettes: Dictionary[String, CamoPalette] = {}


static func palette(faction: String, biome: String) -> CamoPalette:
	var key := "%s_%s" % [faction, biome]
	if not _palettes.has(key):
		var path := PALETTE_DIR + key + ".tres"
		_palettes[key] = load(path) as CamoPalette if ResourceLoader.exists(path) else CamoPalette.new()
	return _palettes[key]


## The palette of a unit with catalog values `values` (the glTF extras,
## `catalog_values`): the early grey one for German vehicles introduced
## before 1943, else `palette(faction, biome)`.
static func palette_for(faction: String, biome: String, values: Dictionary) -> CamoPalette:
	if is_early(faction, values) and ResourceLoader.exists("%s%s_%s_early.tres" % [PALETTE_DIR, faction, biome]):
		return palette(faction, biome + "_early")
	return palette(faction, biome)


## Whether a unit wears the paint of before 1943 (see `EARLY_UNTIL`).
static func is_early(faction: String, values: Dictionary) -> bool:
	var year := int(values.get("cat_year_from", 0))
	return faction == "de" and year > 0 and year < EARLY_UNTIL \
		and not str(values.get("cat_category", "")).begins_with("aircraft") \
		and not str(values.get("cat_category", "")).begins_with("fort")


## Sets the palette as instance uniforms on every mesh below `model`.
static func apply(model: Node, pal: CamoPalette) -> void:
	for mesh in meshes(model):
		mesh.set_instance_shader_parameter(&"camo_base", pal.base)
		mesh.set_instance_shader_parameter(&"camo_color_1", pal.color_1)
		mesh.set_instance_shader_parameter(&"camo_color_2", pal.color_2)
		mesh.set_instance_shader_parameter(&"camo_pattern", float(pal.pattern))
		mesh.set_instance_shader_parameter(&"camo_scale", pal.scale)
		mesh.set_instance_shader_parameter(&"camo_coverage", pal.coverage)
		mesh.set_instance_shader_parameter(&"winter_wash", pal.winter_wash)
		mesh.set_instance_shader_parameter(&"snow", pal.snow)
		mesh.set_instance_shader_parameter(&"mud", pal.mud)
		mesh.set_instance_shader_parameter(&"mud_color", pal.mud_color)
		mesh.set_instance_shader_parameter(&"dust", pal.dust)
		mesh.set_instance_shader_parameter(&"wear", pal.wear)


static func meshes(model: Node) -> Array[GeometryInstance3D]:
	var out: Array[GeometryInstance3D] = []
	var stack: Array[Node] = [model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		# Parts emptied by UnitMerge carry no mesh (prompt 016).
		if n is MeshInstance3D and (n as MeshInstance3D).mesh != null:
			out.append(n)
		stack.append_array(n.get_children())
	return out


## Catalog values stored on the model root (glTF extras), or {} if none.
static func catalog_values(model: Node) -> Dictionary:
	var stack: Array[Node] = [model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		if n.has_meta(&"extras"):
			var extras: Dictionary = n.get_meta(&"extras")
			if extras.has("cat_id"):
				return extras
		stack.append_array(n.get_children())
	return {}


## Adds Decal nodes for national markings and tactical numbers at the
## Socket_Marking_* nodes. Returns the number of decals added.
static func add_markings(model: Node, faction: String, number: int) -> int:
	var count := 0
	var stack: Array[Node] = [model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		stack.append_array(n.get_children())
		if not n.name.begins_with("Socket_Marking_") or not n is Node3D:
			continue
		var extras: Dictionary = n.get_meta(&"extras", {})
		var kind: String = extras.get("marking", "national")
		var size: float = extras.get("marking_size", 0.5)
		var decal := Decal.new()
		decal.name = "Decal"
		var tex := UnitMarkings.number_texture(number) if kind == "number" else UnitMarkings.national_texture(faction)
		decal.texture_albedo = tex
		var aspect := float(tex.get_width()) / float(tex.get_height())
		decal.size = Vector3(size * aspect, 0.25, size)
		decal.upper_fade = 0.05
		decal.lower_fade = 0.05
		decal.normal_fade = 0.5
		n.add_child(decal)
		count += 1
	return count
