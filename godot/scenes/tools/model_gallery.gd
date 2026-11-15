extends Node3D
## Model gallery: every unit model under res://assets/models/<faction>/,
## arranged by faction (rows) and category (columns), with a camouflage
## switch (forest / desert / winter), a free orbit camera and the catalog
## values of the selected model. A development tool, not part of the game.
##
## Controls: right mouse drag = orbit, middle drag = pan, wheel = zoom,
## left click = select and focus, 1/2/3 = forest/desert/winter, F = frame all,
## A = next animation clip for all infantry squads.
##
## Infantry squads (prompt 006) are `.tscn` scenes built by tools/blender/build_squad.py
## from the soldiers of prompt 005; root meta `extras` has `squad = true`. They get
## SoldierLook colors instead of the camo shader and play their clips.
##
## `-- --self-test`: loads all models, checks the kit import (camo shader,
## sockets, catalog extras), switches through all biomes, quits with 0/1.
## `-- --screenshots <dir> [--focus <id>] [--yaw <deg>]`: saves one screenshot per biome
## (of all models, or close up of one) and quits (needs a window).

const MODEL_ROOT := "res://assets/models/"
const FACTIONS: Array[String] = ["de", "su", "us", "neutral"]
const ROW_SPACING := 14.0
const GAP := 3.0
## Models every gallery must show (prompt 004 acceptance).
const REQUIRED: Array[String] = ["de_pz4_h", "su_t34_76_1943", "us_m4a3_75", "de_pak40"]
const BIOME_NAMES: Dictionary[String, String] = {"forest": "Forest", "desert": "Desert", "winter": "Winter"}

var _models: Array[Node3D] = []
var _biome := "forest"
var _selected: Node3D
var _info: Label
var _biome_buttons: Dictionary[String, Button] = {}
var _ground: MeshInstance3D
var _sun: DirectionalLight3D

# orbit camera
var _camera: Camera3D
var _focus := Vector3.ZERO
## Models face -Z, so the camera starts on the -Z side looking at their fronts.
var _yaw := deg_to_rad(215.0)
var _pitch := deg_to_rad(28.0)
var _distance := 30.0

## Squad clip shown in the gallery; "" = each member's own clip from the squad file.
var _squad_clip := ""

var _args := OS.get_cmdline_user_args()
var _self_test := "--self-test" in _args
var _frame := 0


func _ready() -> void:
	_build_environment()
	_build_ui()
	_load_models()
	_set_biome("forest")
	_frame_all()
	if _self_test:
		_run_self_test()


func _process(_delta: float) -> void:
	_frame += 1
	var i := _args.find("--screenshots")
	if i >= 0 and i + 1 < _args.size():
		_screenshots(_args[i + 1])


# --- scene -------------------------------------------------------------------

func _build_environment() -> void:
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	var mat := ProceduralSkyMaterial.new()
	mat.sky_top_color = Color(0.45, 0.58, 0.75)
	mat.sky_horizon_color = Color(0.72, 0.75, 0.78)
	mat.ground_horizon_color = Color(0.5, 0.48, 0.44)
	mat.ground_bottom_color = Color(0.3, 0.28, 0.25)
	sky.sky_material = mat
	e.sky = sky
	e.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	e.ambient_light_energy = 0.8
	e.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	e.ssao_enabled = true
	env.environment = e
	add_child(env)

	_sun = DirectionalLight3D.new()
	_sun.shadow_enabled = true
	_sun.light_energy = 1.3
	_sun.directional_shadow_max_distance = 200.0
	add_child(_sun)
	_sun.look_at_from_position(Vector3(30, 50, 20), Vector3.ZERO, Vector3.UP)

	_ground = MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(400, 400)
	_ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = Color(0.36, 0.38, 0.28)
	gmat.roughness = 1.0
	_ground.material_override = gmat
	add_child(_ground)

	_camera = Camera3D.new()
	_camera.fov = 35.0
	_camera.far = 2000.0
	add_child(_camera)


func _build_ui() -> void:
	var layer := CanvasLayer.new()
	add_child(layer)
	var bar := HBoxContainer.new()
	bar.position = Vector2(12, 12)
	layer.add_child(bar)
	var title := Label.new()
	title.text = "Model gallery   camouflage:"
	bar.add_child(title)
	for biome: String in UnitCamo.BIOMES:
		var b := Button.new()
		b.text = BIOME_NAMES[biome]
		b.toggle_mode = true
		b.pressed.connect(_set_biome.bind(biome))
		bar.add_child(b)
		_biome_buttons[biome] = b
	var frame := Button.new()
	frame.text = "Frame all"
	frame.pressed.connect(_frame_all)
	bar.add_child(frame)

	var panel := PanelContainer.new()
	panel.position = Vector2(12, 56)
	layer.add_child(panel)
	_info = Label.new()
	_info.custom_minimum_size = Vector2(380, 0)
	_info.text = "Click a model to see its catalog values."
	panel.add_child(_info)


func _load_models() -> void:
	var row := 0
	for faction in FACTIONS:
		var paths := _model_paths(MODEL_ROOT + faction)
		if paths.is_empty():
			continue
		var entries: Array[Dictionary] = []
		for path in paths:
			var scene := load(path) as PackedScene
			if scene == null:
				push_error("model_gallery: cannot load %s" % path)
				continue
			var model := scene.instantiate() as Node3D
			var cat := UnitCamo.catalog_values(model)
			entries.append({"model": model, "cat": cat, "path": path})
		# columns by category, then id
		entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			return [a.cat.get("cat_category", ""), a.cat.get("cat_id", "")] < [b.cat.get("cat_category", ""), b.cat.get("cat_id", "")])
		var x := 0.0
		for i in entries.size():
			var model: Node3D = entries[i].model
			model.set_meta(&"gallery_path", entries[i].path)
			model.set_meta(&"gallery_faction", faction)
			add_child(model)
			if _is_squad(model):
				_setup_squad(model)
			var box := _aabb(model)
			model.position = Vector3(x - box.position.x, 0.0, row * ROW_SPACING)
			x += box.size.x + GAP
			if not _is_squad(model):
				UnitCamo.add_markings(model, faction, 100 * (row + 1) + i + 1)
			var label := Label3D.new()
			label.text = str(entries[i].cat.get("cat_name_en", model.name))
			label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
			label.pixel_size = 0.01
			label.position = Vector3(0, _aabb(model).end.y - model.position.y + 0.6, 0)
			model.add_child(label)
			_models.append(model)
		row += 1


func _model_paths(dir_path: String) -> Array[String]:
	var out: Array[String] = []
	var dir := DirAccess.open(dir_path)
	if dir == null:
		return out
	for f in dir.get_files():
		var name := f.trim_suffix(".import").trim_suffix(".remap")
		if (name.ends_with(".glb") or name.ends_with(".tscn")) and not out.has(dir_path + "/" + name):
			out.append(dir_path + "/" + name)
	out.sort()
	return out


func _set_biome(biome: String) -> void:
	_biome = biome
	for b: String in _biome_buttons:
		_biome_buttons[b].button_pressed = b == biome
	for model in _models:
		if _is_squad(model):
			_squad_look(model, biome)
		else:
			UnitCamo.apply(model, UnitCamo.palette(model.get_meta(&"gallery_faction"), biome))
	var gmat := _ground.material_override as StandardMaterial3D
	gmat.albedo_color = {"forest": Color(0.33, 0.36, 0.24), "desert": Color(0.66, 0.58, 0.43),
		"winter": Color(0.74, 0.76, 0.78)}[biome]


# --- camera and selection ----------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	var motion := event as InputEventMouseMotion
	if motion:
		if motion.button_mask & MOUSE_BUTTON_MASK_RIGHT:
			_yaw -= motion.relative.x * 0.005
			_pitch = clampf(_pitch + motion.relative.y * 0.005, deg_to_rad(3), deg_to_rad(88))
		elif motion.button_mask & MOUSE_BUTTON_MASK_MIDDLE:
			var right := _camera.global_basis.x
			var fwd := Vector3(-sin(_yaw), 0, -cos(_yaw))
			_focus += (-right * motion.relative.x + fwd * motion.relative.y) * _distance * 0.0015
	var button := event as InputEventMouseButton
	if button and button.pressed:
		match button.button_index:
			MOUSE_BUTTON_WHEEL_UP:
				_distance = maxf(2.0, _distance * 0.9)
			MOUSE_BUTTON_WHEEL_DOWN:
				_distance = minf(600.0, _distance * 1.1)
			MOUSE_BUTTON_LEFT:
				_pick(button.position)
	var key := event as InputEventKey
	if key and key.pressed and not key.echo:
		match key.keycode:
			KEY_1: _set_biome("forest")
			KEY_2: _set_biome("desert")
			KEY_3: _set_biome("winter")
			KEY_F: _frame_all()
			KEY_A: _next_squad_clip()


func _physics_process(_delta: float) -> void:
	var dir := Vector3(sin(_yaw) * cos(_pitch), sin(_pitch), cos(_yaw) * cos(_pitch))
	_camera.global_position = _focus + dir * _distance
	_camera.look_at(_focus, Vector3.UP)


func _frame_all() -> void:
	if _models.is_empty():
		return
	var box := _aabb(_models[0])
	for m in _models:
		box = box.merge(_aabb(m))
	_focus = box.get_center()
	_distance = maxf(box.size.x, box.size.z) * 1.4 + 5.0


## Ray against the models' bounding boxes (presentation only, no physics).
func _pick(screen_pos: Vector2) -> void:
	var origin := _camera.project_ray_origin(screen_pos)
	var dir := _camera.project_ray_normal(screen_pos)
	var best: Node3D = null
	var best_t := INF
	for m in _models:
		var hit: Variant = _aabb(m).intersects_ray(origin, dir)
		if hit != null:
			var t := origin.distance_to(hit as Vector3)
			if t < best_t:
				best_t = t
				best = m
	if best:
		_select(best)


func _select(model: Node3D) -> void:
	_selected = model
	var box := _aabb(model)
	_focus = box.get_center()
	_distance = maxf(box.size.length() * 1.6, 6.0)
	_info.text = _describe(model)


func _describe(model: Node3D) -> String:
	var c := UnitCamo.catalog_values(model)
	if c.is_empty():
		return "%s\n(no catalog values)" % model.name
	var lines: Array[String] = [
		"%s (%s)" % [c.get("cat_name_en", ""), c.get("cat_id", "")],
		"%s" % c.get("cat_name_de", ""),
		"%s, %d-%d" % [c.get("cat_designation", ""), c.get("cat_year_from", 0), c.get("cat_year_to", 0)],
		"Category: %s" % c.get("cat_category", ""),
	]
	var dims := "Size: %.2f m long" % (c.get("cat_length_mm", 0) / 1000.0)
	if c.has("cat_length_hull_mm"):
		dims += " (hull %.2f)" % (c.cat_length_hull_mm / 1000.0)
	dims += ", %.2f wide, %.2f high" % [c.get("cat_width_mm", 0) / 1000.0, c.get("cat_height_mm", 0) / 1000.0]
	lines.append(dims)
	if c.has("cat_mass_kg"):
		lines.append("Mass: %.1f t, crew %d" % [c.cat_mass_kg / 1000.0, c.get("cat_crew", 0)])
	if c.has("cat_speed_road_kmh_x10"):
		lines.append("Speed: %.0f km/h road, %.0f km/h off-road, range %d km" % [
			c.cat_speed_road_kmh_x10 / 10.0, c.get("cat_speed_offroad_kmh_x10", 0) / 10.0,
			c.get("cat_range_road_km", 0)])
	if c.has("cat_armor_hull_front_mm"):
		lines.append("Armor: hull front %d mm, side %d mm, turret front %d mm" % [
			c.cat_armor_hull_front_mm, c.get("cat_armor_hull_side_mm", 0), c.get("cat_armor_turret_front_mm", 0)])
	if c.get("squad", false):
		lines.append("Squad: %d men" % c.get("cat_squad_size", 0))
		var roles: Array = c.get("squad_roles", [])
		var notes: Array = c.get("squad_notes", [])
		for k in roles.size():
			lines.append("  %d. %s (%s)" % [k + 1, notes[k] if k < notes.size() else "", roles[k]])
		lines.append("Clip: %s (A = next)" % (_squad_clip if _squad_clip else "per member"))
	lines.append("Weapons: %s" % c.get("cat_weapons", ""))
	lines.append("Triangles: %d" % _triangles(model))
	lines.append("Model: %s" % model.get_meta(&"gallery_path", ""))
	return "\n".join(lines)


func _triangles(model: Node) -> int:
	var n := 0
	for g in UnitCamo.meshes(model):
		var mi := g as MeshInstance3D
		if mi == null or mi.mesh == null:
			continue
		for s in mi.mesh.get_surface_count():
			var arrays := mi.mesh.surface_get_arrays(s)
			var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
			n += idx.size() / 3 if idx.size() else (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
	return n


func _aabb(model: Node3D) -> AABB:
	var box := AABB()
	var first := true
	for g in UnitCamo.meshes(model):
		var b: AABB = g.global_transform * g.get_aabb()
		box = b if first else box.merge(b)
		first = false
	return box


# --- checks ------------------------------------------------------------------

func _run_self_test() -> void:
	var errors: Array[String] = []
	var ids: Array[String] = []
	var shader := load("res://assets/shaders/unit_camo.gdshader") as Shader
	for model in _models:
		var cat := UnitCamo.catalog_values(model)
		var id: String = cat.get("cat_id", "")
		if id.is_empty():
			continue  # not a kit model (e.g. the placeholder tank)
		ids.append(id)
		if _is_squad(model):
			_check_squad(model, cat, errors)
			continue
		var camo_surfaces := 0
		for g in UnitCamo.meshes(model):
			var mi := g as MeshInstance3D
			if mi == null or mi.mesh == null:
				continue
			for s in mi.mesh.get_surface_count():
				var mat := mi.mesh.surface_get_material(s) as ShaderMaterial
				if mat == null or mat.shader != shader:
					errors.append("%s/%s: surface %d does not use unit_camo" % [id, mi.name, s])
				else:
					camo_surfaces += 1
		if camo_surfaces == 0:
			errors.append("%s: no unit_camo surfaces" % id)
		# unarmed vehicles (trucks) have no muzzle
		if str(cat.get("cat_weapons", "")) != "" and model.find_child("Socket_Muzzle", true, false) == null:
			errors.append("%s: Socket_Muzzle missing" % id)
		if model.find_child("Decal", true, false) == null:
			errors.append("%s: no marking decal" % id)
		print("self-test: %s loaded, %d camo surfaces, %d triangles" % [id, camo_surfaces, _triangles(model)])
	for req in REQUIRED:
		if not ids.has(req):
			errors.append("required model %s not in the gallery" % req)
	for biome: String in UnitCamo.BIOMES:
		_set_biome(biome)
		for model in _models:
			if _is_squad(model):
				_check_squad_look(model, biome, errors)
				continue
			var faction: String = model.get_meta(&"gallery_faction")
			var pal := UnitCamo.palette(faction, biome)
			if pal.biome != biome or pal.faction != faction:
				errors.append("palette %s_%s missing" % [faction, biome])
				continue
			var hull := model.find_child("Hull", true, false) as GeometryInstance3D
			if hull == null:
				hull = model.find_child("Carriage", true, false) as GeometryInstance3D
			if hull and hull.get_instance_shader_parameter(&"winter_wash") != pal.winter_wash:
				errors.append("%s: winter_wash not applied for %s" % [model.name, biome])
	if errors.is_empty():
		print("self-test: gallery ok, %d models (%s), 3 biomes" % [ids.size(), ", ".join(ids)])
		get_tree().quit(0)
	else:
		for e in errors:
			printerr("self-test: FAIL " + e)
		get_tree().quit(1)


func _screenshots(dir: String) -> void:
	# a few frames per biome so that shadows and SSAO settle
	var step := 40
	var biomes: Array[String] = UnitCamo.BIOMES
	var idx := _frame / step
	if _frame % step == 0 and idx >= 1 and idx <= biomes.size():
		var img := get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(dir)
		var path := "%s/gallery_%s.png" % [dir, _biome]
		img.save_png(path)
		print("model_gallery: saved %s" % path)
		if idx < biomes.size():
			_set_biome(biomes[idx])
		else:
			get_tree().quit(0)
	elif _frame == 1:
		_yaw = deg_to_rad(210.0)
		_pitch = deg_to_rad(24.0)
		_frame_all()
		_distance *= 0.75
		var f := _args.find("--focus")
		if f >= 0 and f + 1 < _args.size():
			for m in _models:
				if UnitCamo.catalog_values(m).get("cat_id", "") == _args[f + 1]:
					_select(m)
					_distance *= 0.8
		var y := _args.find("--yaw")
		if y >= 0 and y + 1 < _args.size():
			_yaw = deg_to_rad(float(_args[y + 1]))


# --- infantry squads (prompt 006) ----------------------------------------------

func _is_squad(model: Node3D) -> bool:
	return UnitCamo.catalog_values(model).get("squad", false)


## Squad members are the direct children instanced from the soldier .glb files.
func _squad_members(model: Node3D) -> Array[Node3D]:
	var out: Array[Node3D] = []
	for child in model.get_children():
		if child is Node3D and SoldierLook.find_player(child) != null:
			out.append(child)
	return out


func _setup_squad(model: Node3D) -> void:
	for member in _squad_members(model):
		SoldierLook.configure_animations(SoldierLook.find_player(member))
	_play_squad(model)


func _play_squad(model: Node3D) -> void:
	var clips: Array = UnitCamo.catalog_values(model).get("squad_clips", [])
	var members := _squad_members(model)
	for k in members.size():
		var clip: String = _squad_clip
		if clip.is_empty():
			clip = clips[k] if k < clips.size() else "idle"
		var player := SoldierLook.find_player(members[k])
		if player.has_animation(clip):
			player.play(clip)
			# desynchronise the members a little (presentation only)
			player.seek(fmod(k * 0.37, maxf(player.current_animation_length, 0.01)), true)


func _squad_look(model: Node3D, biome: String) -> void:
	var faction: String = model.get_meta(&"gallery_faction")
	var members := _squad_members(model)
	for k in members.size():
		SoldierLook.apply(members[k], faction, biome, hash(model.name) + k)


func _next_squad_clip() -> void:
	var names: Array[String] = [""]
	for clip: String in SoldierLook.clips():
		names.append(clip)
	_squad_clip = names[(names.find(_squad_clip) + 1) % names.size()]
	for model in _models:
		if _is_squad(model):
			_play_squad(model)
	if _selected:
		_info.text = _describe(_selected)


func _check_squad(model: Node3D, cat: Dictionary, errors: Array[String]) -> void:
	var id: String = cat.get("cat_id", "")
	var members := _squad_members(model)
	if members.size() != int(cat.get("cat_squad_size", -1)):
		errors.append("%s: %d members, catalog squad_size %d" % [id, members.size(), cat.get("cat_squad_size", -1)])
	var armed := 0
	for member in members:
		var player := SoldierLook.find_player(member)
		var missing: Array[String] = []
		for clip: String in SoldierLook.clips():
			if not player.has_animation(clip):
				missing.append(clip)
		if not missing.is_empty():
			errors.append("%s/%s: clips missing %s" % [id, member.name, missing])
		if SoldierLook.find_skeleton(member) == null:
			errors.append("%s/%s: no skeleton" % [id, member.name])
		if member.find_child("Socket_Muzzle", true, false) != null:
			armed += 1
		if not player.is_playing():
			errors.append("%s/%s: no clip playing" % [id, member.name])
	if armed == 0:
		errors.append("%s: no armed member (Socket_Muzzle)" % id)
	print("self-test: %s loaded, squad of %d, %d armed, %d triangles" % [id, members.size(), armed, _triangles(model)])


func _check_squad_look(model: Node3D, biome: String, errors: Array[String]) -> void:
	var faction: String = model.get_meta(&"gallery_faction")
	# materials are cached per skin tone too, so compare the uniform color
	var want := (SoldierLook.slot_material(faction, biome, "Cloth") as StandardMaterial3D).albedo_color
	for member in _squad_members(model):
		var found := false
		for mi: MeshInstance3D in member.find_children("*", "MeshInstance3D", true, false):
			for s in mi.get_surface_override_material_count():
				var mat := mi.get_surface_override_material(s)
				var std := mat as StandardMaterial3D
				if std and std.resource_name == "Cloth":
					found = found or std.albedo_color == want
		if not found:
			errors.append("%s/%s: no %s_%s uniform" % [model.name, member.name, faction, biome])
			return
