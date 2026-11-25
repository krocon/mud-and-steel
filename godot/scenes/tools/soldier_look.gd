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
static var _combined_materials: Dictionary = {}
## Merged 1-surface soldier meshes keyed by original Mesh resource (prompt 207).
static var _merged_meshes: Dictionary = {}

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


## Shared combined material reading slot from UV2.y (prompt 207): carries all
## 10 slot colors/roughnesses/metallics in uniform arrays so Body, Gear and
## Weapon each draw in a single surface (11 -> 3 draw calls per soldier).
static func combined_material(faction: String, biome: String, tone := 0,
		camo := false, worn := false) -> Material:
	camo = camo and biome == "forest"
	var key := "%s/%s/%d/%s/%s" % [faction, biome, tone, camo, worn]
	if _combined_materials.has(key):
		return _combined_materials[key]
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	mat.resource_name = "SoldierCombined"
	mat.set_shader_parameter(&"slot", -1)
	var colors: Array[Color] = []
	var roughs := PackedFloat32Array()
	var metals := PackedFloat32Array()
	for s_name: String in SLOTS:
		var c := slot_color(faction, biome, s_name, tone)
		if worn and (s_name == "Cloth" or s_name == "Canvas"):
			c = c.lerp(WORN_TINT, WORN_MIX)
		colors.append(c)
		roughs.append(float(ROUGHNESS.get(s_name, 0.9)))
		metals.append(0.6 if s_name == "Metal" else 0.0)
	mat.set_shader_parameter(&"base_color", colors[0])
	mat.set_shader_parameter(&"slot_color", colors)
	mat.set_shader_parameter(&"slot_rough", roughs)
	mat.set_shader_parameter(&"slot_metal", metals)
	mat.set_shader_parameter(&"camo", 0)
	if biome == "winter":
		mat.set_shader_parameter(&"soil", float(palette().get("winter_soil", 0.0)))
	var spec: Dictionary = palette().get("camo", {}).get(faction, {})
	if camo and not spec.is_empty():
		var camo_colors: Array = spec.get("colors", [])
		mat.set_shader_parameter(&"camo", PATTERNS.get(spec.get("pattern", ""), 0))
		for i in mini(camo_colors.size(), 5):
			var uniform_name := "camo_base" if i == 0 else "camo_%d" % i
			mat.set_shader_parameter(StringName(uniform_name), Color.html(camo_colors[i]))
		mat.set_shader_parameter(&"camo_scale", float(spec.get("scale", 0.1)))
		mat.set_shader_parameter(&"camo_min_z", float(spec.get("camo_min_z", -1.0)))
		mat.set_shader_parameter(&"camo_helmet", bool(spec.get("helmet", false)))
	_combined_materials[key] = mat
	return mat


## Merges all slot surfaces of a soldier Mesh (Body, Gear or Weapon) into a
## single surface encoding slot index in UV2.y, preserving skinning and all
## blend shapes (prompt 207). Cached once per source Mesh resource.
static func _merge_soldier_mesh(src: Mesh) -> ArrayMesh:
	if _merged_meshes.has(src):
		return _merged_meshes[src]
	var src_am := src as ArrayMesh
	if src_am == null:
		return null
	var surf_count := src_am.get_surface_count()
	if surf_count == 0:
		return null
	var slot_ids: Array[int] = []
	for s in surf_count:
		var slot_idx := SLOTS.find(_slot_name(src_am.surface_get_material(s)))
		if slot_idx < 0:
			return null
		slot_ids.append(slot_idx)
	var verts := PackedVector3Array()
	var norms := PackedVector3Array()
	var tangs := PackedFloat32Array()
	var cols := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var bones := PackedInt32Array()
	var weights := PackedFloat32Array()
	var indices := PackedInt32Array()
	var has_tangents := false
	var has_bones := false
	var bones_per_vert := 4
	var src_arrays: Array = []
	for s in surf_count:
		var a := src_am.surface_get_arrays(s)
		src_arrays.append(a)
		if a[Mesh.ARRAY_TANGENT] is PackedFloat32Array and not (a[Mesh.ARRAY_TANGENT] as PackedFloat32Array).is_empty():
			has_tangents = true
		if a[Mesh.ARRAY_BONES] is PackedInt32Array and not (a[Mesh.ARRAY_BONES] as PackedInt32Array).is_empty():
			has_bones = true
			var vc := (a[Mesh.ARRAY_VERTEX] as PackedVector3Array).size()
			if vc > 0:
				bones_per_vert = maxi(bones_per_vert, (a[Mesh.ARRAY_BONES] as PackedInt32Array).size() / vc)
	var blend_count := src_am.get_blend_shape_count()
	var bs_verts: Array[PackedVector3Array] = []
	var bs_norms: Array[PackedVector3Array] = []
	var bs_tangs: Array[PackedFloat32Array] = []
	var bs_has_norms := false
	var bs_has_tangs := false
	for b in blend_count:
		bs_verts.append(PackedVector3Array())
		bs_norms.append(PackedVector3Array())
		bs_tangs.append(PackedFloat32Array())
	for s in surf_count:
		var a: Array = src_arrays[s]
		var sv: PackedVector3Array = a[Mesh.ARRAY_VERTEX]
		var n := sv.size()
		if n == 0:
			continue
		var v_offset := verts.size()
		verts.append_array(sv)
		if a[Mesh.ARRAY_NORMAL] is PackedVector3Array and (a[Mesh.ARRAY_NORMAL] as PackedVector3Array).size() == n:
			norms.append_array(a[Mesh.ARRAY_NORMAL] as PackedVector3Array)
		else:
			var pad_n := PackedVector3Array()
			pad_n.resize(n)
			pad_n.fill(Vector3.UP)
			norms.append_array(pad_n)
		if has_tangents:
			if a[Mesh.ARRAY_TANGENT] is PackedFloat32Array and (a[Mesh.ARRAY_TANGENT] as PackedFloat32Array).size() == n * 4:
				tangs.append_array(a[Mesh.ARRAY_TANGENT] as PackedFloat32Array)
			else:
				var pad_t := PackedFloat32Array()
				pad_t.resize(n * 4)
				for i in n:
					pad_t[i * 4] = 1.0
					pad_t[i * 4 + 3] = 1.0
				tangs.append_array(pad_t)
		if a[Mesh.ARRAY_COLOR] is PackedColorArray and (a[Mesh.ARRAY_COLOR] as PackedColorArray).size() == n:
			cols.append_array(a[Mesh.ARRAY_COLOR] as PackedColorArray)
		else:
			var pad_c := PackedColorArray()
			pad_c.resize(n)
			pad_c.fill(Color.WHITE)
			cols.append_array(pad_c)
		if a[Mesh.ARRAY_TEX_UV] is PackedVector2Array and (a[Mesh.ARRAY_TEX_UV] as PackedVector2Array).size() == n:
			uvs.append_array(a[Mesh.ARRAY_TEX_UV] as PackedVector2Array)
		else:
			var pad_uv := PackedVector2Array()
			pad_uv.resize(n)
			uvs.append_array(pad_uv)
		var slot_f := float(slot_ids[s])
		var su2 := PackedVector2Array()
		su2.resize(n)
		if a[Mesh.ARRAY_TEX_UV2] is PackedVector2Array and (a[Mesh.ARRAY_TEX_UV2] as PackedVector2Array).size() == n:
			var src_u2 := a[Mesh.ARRAY_TEX_UV2] as PackedVector2Array
			for i in n:
				su2[i] = Vector2(src_u2[i].x, slot_f)
		else:
			su2.fill(Vector2(0.0, slot_f))
		uv2s.append_array(su2)
		if has_bones:
			var sb: PackedInt32Array = a[Mesh.ARRAY_BONES] if a[Mesh.ARRAY_BONES] is PackedInt32Array else PackedInt32Array()
			var sw: PackedFloat32Array = a[Mesh.ARRAY_WEIGHTS] if a[Mesh.ARRAY_WEIGHTS] is PackedFloat32Array else PackedFloat32Array()
			if sb.size() == n * bones_per_vert and sw.size() == n * bones_per_vert:
				bones.append_array(sb)
				weights.append_array(sw)
			elif sb.size() == n * 4 and bones_per_vert == 8:
				var pb := PackedInt32Array()
				var pw := PackedFloat32Array()
				pb.resize(n * 8)
				pw.resize(n * 8)
				for i in n:
					for j in 4:
						pb[i * 8 + j] = sb[i * 4 + j]
						pw[i * 8 + j] = sw[i * 4 + j]
				bones.append_array(pb)
				weights.append_array(pw)
			else:
				var pb := PackedInt32Array()
				var pw := PackedFloat32Array()
				pb.resize(n * bones_per_vert)
				pw.resize(n * bones_per_vert)
				for i in n:
					pw[i * bones_per_vert] = 1.0
				bones.append_array(pb)
				weights.append_array(pw)
		if a[Mesh.ARRAY_INDEX] is PackedInt32Array and not (a[Mesh.ARRAY_INDEX] as PackedInt32Array).is_empty():
			var si := a[Mesh.ARRAY_INDEX] as PackedInt32Array
			var old_len := indices.size()
			indices.resize(old_len + si.size())
			for i in si.size():
				indices[old_len + i] = si[i] + v_offset
		else:
			var old_len := indices.size()
			indices.resize(old_len + n)
			for i in n:
				indices[old_len + i] = v_offset + i
		if blend_count > 0:
			var bs_surf := src_am.surface_get_blend_shape_arrays(s)
			for b in blend_count:
				var ba: Array = bs_surf[b] if b < bs_surf.size() else []
				if ba.size() == Mesh.ARRAY_MAX and ba[Mesh.ARRAY_VERTEX] is PackedVector3Array and (ba[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() == n:
					bs_verts[b].append_array(ba[Mesh.ARRAY_VERTEX] as PackedVector3Array)
					if ba[Mesh.ARRAY_NORMAL] is PackedVector3Array and (ba[Mesh.ARRAY_NORMAL] as PackedVector3Array).size() == n:
						bs_has_norms = true
						bs_norms[b].append_array(ba[Mesh.ARRAY_NORMAL] as PackedVector3Array)
					else:
						bs_norms[b].append_array(norms.slice(v_offset, v_offset + n))
					if has_tangents:
						if ba[Mesh.ARRAY_TANGENT] is PackedFloat32Array and (ba[Mesh.ARRAY_TANGENT] as PackedFloat32Array).size() == n * 4:
							bs_has_tangs = true
							bs_tangs[b].append_array(ba[Mesh.ARRAY_TANGENT] as PackedFloat32Array)
						else:
							bs_tangs[b].append_array(tangs.slice(v_offset * 4, (v_offset + n) * 4))
				else:
					bs_verts[b].append_array(sv)
					bs_norms[b].append_array(norms.slice(v_offset, v_offset + n))
					if has_tangents:
						bs_tangs[b].append_array(tangs.slice(v_offset * 4, (v_offset + n) * 4))
	var out := ArrayMesh.new()
	if blend_count > 0:
		out.blend_shape_mode = src_am.blend_shape_mode
		for b in blend_count:
			out.add_blend_shape(src_am.get_blend_shape_name(b))
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = norms
	if has_tangents:
		arrays[Mesh.ARRAY_TANGENT] = tangs
	arrays[Mesh.ARRAY_COLOR] = cols
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	if has_bones:
		arrays[Mesh.ARRAY_BONES] = bones
		arrays[Mesh.ARRAY_WEIGHTS] = weights
	arrays[Mesh.ARRAY_INDEX] = indices
	var blend_arrays: Array = []
	for b in blend_count:
		var ba := []
		ba.resize(Mesh.ARRAY_MAX)
		ba[Mesh.ARRAY_VERTEX] = bs_verts[b]
		if bs_has_norms:
			ba[Mesh.ARRAY_NORMAL] = bs_norms[b]
		if has_tangents and bs_has_tangs:
			ba[Mesh.ARRAY_TANGENT] = bs_tangs[b]
		blend_arrays.append(ba)
	var flags := 0
	if has_bones and bones_per_vert == 8:
		flags |= Mesh.ARRAY_FLAG_USE_8_BONE_WEIGHTS
	out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, blend_arrays, {}, flags)
	_merged_meshes[src] = out
	_merged_meshes[out] = out
	return out


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
	var combined := combined_material(faction, biome, tone, camo, worn)
	for mesh_instance: MeshInstance3D in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := mesh_instance.mesh
		if mesh == null:
			continue
		var merged := _merge_soldier_mesh(mesh)
		if merged != null:
			if mesh_instance.mesh != merged:
				mesh_instance.mesh = merged
			mesh_instance.set_surface_override_material(0, combined)
		else:
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
