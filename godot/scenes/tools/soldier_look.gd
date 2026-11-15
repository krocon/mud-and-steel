class_name SoldierLook
extends RefCounted
## Presentation helpers for the infantry soldiers of prompt 005.
##
## - apply(): uniform colors per faction and biome (material overrides by slot name)
##   and per-instance variety from a seed (stature and face blend shapes, skin tone).
## - configure_animations(): loop flags from the clip metadata written by
##   tools/blender/build_soldier.py.
##
## Colors come from res://assets/models/infantry/uniform_palettes.json, the same file
## Blender uses. The camouflage shader of prompt 004 can take over the slot materials
## later; the slot names stay the same.

const PALETTE_PATH := "res://assets/models/infantry/uniform_palettes.json"
const CLIPS_PATH := "res://assets/models/infantry/animations.json"
const BIOMES: Array[String] = ["forest", "desert", "winter"]
const SLOTS: Array[String] = [
	"Cloth", "Canvas", "Leather", "Paint", "Skin", "Metal", "Wood", "Rubber", "Glass", "Insignia",
]
const ROUGHNESS := {"Metal": 0.45, "Glass": 0.1, "Paint": 0.7, "Leather": 0.6}
const SKIN_TONES: Array[float] = [1.0, 0.9, 1.08, 0.82]

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


## Shared material for one slot of one faction/biome (and skin tone index).
static func slot_material(faction: String, biome: String, slot: String, tone := 0) -> Material:
	var key := "%s/%s/%s/%d" % [faction, biome, slot, tone]
	if _materials.has(key):
		return _materials[key]
	var colors: Dictionary = palette().get("fixed", {}).duplicate()
	colors.merge(palette().get("factions", {}).get(faction, {}).get(biome, {}), true)
	var mat := StandardMaterial3D.new()
	mat.resource_name = slot
	var color := Color.html(colors.get(slot, "#ff00ff"))
	if slot == "Skin":
		color = color * SKIN_TONES[tone % SKIN_TONES.size()]
		color.a = 1.0
	mat.albedo_color = color
	mat.vertex_color_use_as_albedo = true
	mat.roughness = ROUGHNESS.get(slot, 0.9)
	mat.metallic = 0.6 if slot == "Metal" else 0.0
	_materials[key] = mat
	return mat


## Recolors a soldier for faction and biome; seed picks stature, face and skin tone.
static func apply(model: Node, faction: String, biome: String, seed := 0) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed
	var tone := rng.randi_range(0, SKIN_TONES.size() - 1)
	var stature := rng.randf_range(-1.0, 1.0)
	var face := rng.randi_range(0, 2)
	for mesh_instance: MeshInstance3D in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := mesh_instance.mesh
		if mesh == null:
			continue
		for i in mesh.get_surface_count():
			var slot := _slot_name(mesh.surface_get_material(i))
			if slot in SLOTS:
				mesh_instance.set_surface_override_material(
					i, slot_material(faction, biome, slot, tone)
				)
		_set_blend_shape(mesh_instance, "Stocky", maxf(stature, 0.0))
		_set_blend_shape(mesh_instance, "Slim", maxf(-stature, 0.0))
		_set_blend_shape(mesh_instance, "FaceB", 1.0 if face == 1 else 0.0)
		_set_blend_shape(mesh_instance, "FaceC", 1.0 if face == 2 else 0.0)


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
