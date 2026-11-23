class_name SoldierLook
extends RefCounted
## Presentation helpers for the infantry soldiers of prompt 005.
##
## - apply(): uniform colors per faction and biome, procedural cloth, leather, metal,
##   wood and skin detail and camouflage garments (prompt 042, soldier.gdshader), and
##   per-instance variety from a seed (stature and face blend shapes, skin tone).
## - configure_animations(): loop flags from the clip metadata written by
##   tools/blender/build_soldier.py.
##
## Colors come from res://assets/models/infantry/uniform_palettes.json, the same file
## Blender uses.
##
## Rank (prompt 050, `rank` of apply()): from the second rank on the uniform cloth and
## canvas are sun-bleached and worn; veterans of factions in VETERAN_CAMO wear the
## faction's camouflage garment (half of them, elite all of them, forest biome only).

const PALETTE_PATH := "res://assets/models/infantry/uniform_palettes.json"
const CLIPS_PATH := "res://assets/models/infantry/animations.json"
const SHADER := preload("res://assets/shaders/soldier.gdshader")
const BIOMES: Array[String] = ["forest", "desert", "winter"]
const SLOTS: Array[String] = [
	"Cloth", "Canvas", "Leather", "Paint", "Skin", "Metal", "Wood", "Rubber", "Glass", "Insignia",
]
const ROUGHNESS := {"Metal": 0.45, "Glass": 0.1, "Paint": 0.7, "Leather": 0.6}
const SKIN_TONES: Array[float] = [1.0, 0.9, 1.08, 0.82]
## Camouflage choice for apply(): "auto" = the roles of the palette's `camo.<faction>.roles`
## wear it, "camo" = every soldier, "none" = nobody. Only in the forest biome.
const CAMO_MODES: Array[String] = ["auto", "camo", "none"]
const PATTERNS := {"splinter": 1, "amoeba": 2, "frogskin": 3}
## Rank index from which uniforms look worn, from which some (all) soldiers wear the
## camouflage garment (prompt 050, ranks of data/experience.toml).
const WORN_RANK := 1
const VETERAN_RANK := 2
const ELITE_RANK := 3
## Factions whose veterans got hold of camouflage garments: German Heer soldiers
## acquired Splittertarn smocks and helmet covers, Soviet scouts wore amoeba suits.
## US frogskin was withdrawn in Europe after mid-1944, so US veterans only look worn.
## [provisional]
## British veterans (prompt 117) look worn only: the Denison smock was the
## airborne troops' garment (their roles wear it by default).
const VETERAN_CAMO := {"de": true, "su": true, "us": false, "uk": false}
## Share the worn cloth moves towards WORN_TINT (sun, mud, washing).
const WORN_MIX := 0.18
const WORN_TINT := Color(0.72, 0.69, 0.6)

static var _palette: Dictionary = {}
static var _clips: Dictionary = {}
static var _materials: Dictionary = {}

static func palette() -> Dictionary:
	if _palette.is_empty():
		_palette = _load_json(PALETTE_PATH)
	return _palette


static func clips() -> Dictionary:
	if _clips.is_empty():
		_clips = _load_json(CLIPS_PATH).get("clips", {})
	return _clips


static func _load_json(path: String) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_error("SoldierLook: cannot open %s" % path)
		return {}
	var data: Variant = JSON.parse_string(file.get_as_text())
	return data if data is Dictionary else {}


## Shared material for one slot of one faction/biome (and skin tone index, camouflage).
static func slot_material(faction: String, biome: String, slot: String, tone := 0,
		camo := false, worn := false) -> Material:
	camo = camo and biome == "forest"
	worn = worn and slot in ["Cloth", "Canvas"]
	var key := "%s/%s/%s/%d/%s/%s" % [faction, biome, slot, tone, camo, worn]
	if _materials.has(key):
		return _materials[key]
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	mat.resource_name = slot
	mat.set_shader_parameter(&"slot", SLOTS.find(slot))
	var base := slot_color(faction, biome, slot, tone)
	if worn:
		base = base.lerp(WORN_TINT, WORN_MIX)
	mat.set_shader_parameter(&"base_color", base)
	mat.set_shader_parameter(&"roughness", ROUGHNESS.get(slot, 0.9))
	mat.set_shader_parameter(&"metallic", 0.6 if slot == "Metal" else 0.0)
	mat.set_shader_parameter(&"camo", 0)
	if biome == "winter":
		mat.set_shader_parameter(&"soil", float(palette().get("winter_soil", 0.0)))
	var spec: Dictionary = palette().get("camo", {}).get(faction, {})
	if camo and not spec.is_empty():
		var colors: Array = spec.get("colors", [])
		mat.set_shader_parameter(&"camo", PATTERNS.get(spec.get("pattern", ""), 0))
		for i in mini(colors.size(), 5):
			var uniform_name := "camo_base" if i == 0 else "camo_%d" % i
			mat.set_shader_parameter(StringName(uniform_name), Color.html(colors[i]))
		mat.set_shader_parameter(&"camo_scale", float(spec.get("scale", 0.1)))
		mat.set_shader_parameter(&"camo_min_z", float(spec.get("camo_min_z", -1.0)))
		mat.set_shader_parameter(&"camo_helmet", bool(spec.get("helmet", false)))
	_materials[key] = mat
	return mat


## Base color of a slot (sRGB) for faction, biome and skin tone.
static func slot_color(faction: String, biome: String, slot: String, tone := 0) -> Color:
	var colors: Dictionary = palette().get("fixed", {}).duplicate()
	colors.merge(palette().get("factions", {}).get(faction, {}).get(biome, {}), true)
	var color := Color.html(colors.get(slot, "#ff00ff"))
	if slot == "Skin":
		color = color * SKIN_TONES[tone % SKIN_TONES.size()]
		color.a = 1.0
	return color


## Whether a soldier of this role wears the camouflage garment in mode `camo_mode`.
static func wears_camo(faction: String, role: String, camo_mode: String) -> bool:
	if camo_mode == "camo":
		return true
	if camo_mode != "auto":
		return false
	var roles: Array = palette().get("camo", {}).get(faction, {}).get("roles", [])
	return role in roles


## Role of a soldier model: the file name of its .glb (godot/assets/models/<faction>/
## infantry/<role>.glb), also for squad members instanced from it.
static func role_of(model: Node) -> String:
	if model.scene_file_path != "":
		return model.scene_file_path.get_file().get_basename()
	for child in model.get_children():
		if child.scene_file_path != "":
			return child.scene_file_path.get_file().get_basename()
	return ""


## Recolors a soldier for faction and biome; seed picks stature, face and skin tone.
## camo_mode: see CAMO_MODES. rank: the squad's rank (prompt 050, 0 = recruit).
static func apply(model: Node, faction: String, biome: String, seed := 0, camo_mode := "auto",
		rank := 0) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed
	var tone := rng.randi_range(0, SKIN_TONES.size() - 1)
	var stature := rng.randf_range(-1.0, 1.0)
	var face := rng.randi_range(0, 2)
	var camo := wears_camo(faction, role_of(model), camo_mode) \
			or (camo_mode == "auto" and veteran_camo(faction, rank, seed))
	var worn := rank >= WORN_RANK
	for mesh_instance: MeshInstance3D in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := mesh_instance.mesh
		if mesh == null:
			continue
		for i in mesh.get_surface_count():
			var slot := _slot_name(mesh.surface_get_material(i))
			if slot in SLOTS:
				mesh_instance.set_surface_override_material(
					i, slot_material(faction, biome, slot, tone, camo, worn)
				)
		_set_blend_shape(mesh_instance, "Stocky", maxf(stature, 0.0))
		_set_blend_shape(mesh_instance, "Slim", maxf(-stature, 0.0))
		_set_blend_shape(mesh_instance, "FaceB", 1.0 if face == 1 else 0.0)
		_set_blend_shape(mesh_instance, "FaceC", 1.0 if face == 2 else 0.0)


## Whether a soldier of a squad of `rank` wears the camouflage garment he got hold of
## (prompt 050): elite all, veterans every second one (by seed).
static func veteran_camo(faction: String, rank: int, seed: int) -> bool:
	if not VETERAN_CAMO.get(faction, false):
		return false
	if rank >= ELITE_RANK:
		return true
	return rank >= VETERAN_RANK and (seed / 7919) % 2 == 0


static func _slot_name(material: Material) -> String:
	if material == null:
		return ""
	# Imported names may carry a suffix (".001") when Blender duplicated a material.
	return material.resource_name.get_slice(".", 0)


static func _set_blend_shape(mesh_instance: MeshInstance3D, shape: String, value: float) -> void:
	var idx := mesh_instance.find_blend_shape_by_name(shape)
	if idx >= 0:
		mesh_instance.set_blend_shape_value(idx, value)


## Sets loop modes from the clip metadata. Returns the number of clips configured.
static func configure_animations(player: AnimationPlayer) -> int:
	var count := 0
	for clip_name: String in clips():
		if not player.has_animation(clip_name):
			continue
		var anim := player.get_animation(clip_name)
		var loop: bool = clips()[clip_name].get("loop", false)
		anim.loop_mode = Animation.LOOP_LINEAR if loop else Animation.LOOP_NONE
		count += 1
	return count


static func find_player(model: Node) -> AnimationPlayer:
	var players := model.find_children("*", "AnimationPlayer", true, false)
	return players[0] if not players.is_empty() else null


static func find_skeleton(model: Node) -> Skeleton3D:
	var skeletons := model.find_children("*", "Skeleton3D", true, false)
	return skeletons[0] if not skeletons.is_empty() else null
