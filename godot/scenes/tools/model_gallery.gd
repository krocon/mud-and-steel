extends Node3D
## Model gallery: every model of the game in three categories, a biome switch
## (forest / desert / winter), a free orbit camera and the catalog values of
## the selected model. A development tool, not part of the game.
##
## - Units: kit models under res://assets/models/<faction>/ (prompt 004),
##   rows by faction, columns by category, camouflage per biome. Infantry
##   squads (prompt 006) are `.tscn` scenes built by tools/blender/build_squad.py
##   from the soldiers of prompt 005 (root meta `extras` has `squad = true`);
##   they get SoldierLook colors instead of the camo shader and play their clips.
## - Infantry: soldiers under res://assets/models/<faction>/infantry/
##   (prompt 005), rows by faction, playable animation clips.
## - Environment: res://assets/models/neutral/ (prompt 007), rows by group
##   (vegetation, props, buildings, ...), every variant side by side, tree
##   variants with their far mesh, biome look via EnvLook.
##
## Controls: category and biome buttons, clip list (infantry), right mouse
## drag = orbit, middle drag = pan, wheel = zoom, left click = select and focus,
## 1/2/3 = forest/desert/winter, Tab = next category, F = frame all,
## left/right = previous/next clip (infantry), A = next clip for all squads.
##
## `-- --self-test`: loads all models of all categories, checks imports
## (shaders, sockets, catalog extras, variants, skeletons, clips), plays every
## clip and switches through all biomes, quits with 0/1.
## `-- --screenshots <dir> [--category units|infantry|environment] [--focus <id>]
## [--yaw <deg>] [--clip <name>]`: saves one screenshot per biome and quits
## (needs a window).
## `-- --tour`: scripted camera tour through all categories and biomes for
## recording, timed for `--fixed-fps 30`; hides the UI and quits at the end:
##     godot --path godot --write-movie <abs path>.avi --fixed-fps 30 \
##         res://scenes/tools/model_gallery.tscn -- --tour

const MODEL_ROOT := "res://assets/models/"
const Atmosphere := preload("res://scenes/world/atmosphere.gd")
const UNIT_FACTIONS: Array[String] = ["de", "su", "us"]
const CATEGORIES: Array[String] = ["units", "infantry", "environment"]
const CATEGORY_NAMES: Dictionary[String, String] = {
	"units": "Units", "infantry": "Infantry", "environment": "Environment"}
const ROW_SPACING := 14.0
const GAP := 3.0
## Models every gallery must show (acceptance of prompts 004, 005, 007).
const REQUIRED: Array[String] = ["de_pz4_h", "su_t34_76_1943", "us_m4a3_75", "de_pak40"]
const REQUIRED_INFANTRY: Array[String] = ["rifleman", "mg_gunner"]
const REQUIRED_ENV: Array[String] = [
	"env_tree_deciduous", "env_tree_conifer", "env_tree_birch", "env_tree_palm", "env_bush",
	"env_grass_clump", "env_rock", "env_house_farm", "env_barn", "env_house_stone", "env_house_mud",
	"env_church", "env_bridge_wood", "env_bridge_stone", "env_fence_wood", "env_wall_stone", "env_hedge",
	"fort_barbed_wire", "fort_hedgehog", "fort_sandbag_position", "fort_bunker_mg", "fort_trench_section",
	"fort_mine_at", "fort_mine_ap", "fort_mine_sign"]
## Environment rows: group name -> catalog categories.
const ENV_GROUPS: Array = [
	["Trees", ["env_tree"]],
	["Bushes, grass, rocks", ["env_bush", "env_grass", "env_rock"]],
	["Props", ["env_prop", "env_pole"]],
	["Buildings", ["env_building"]],
	["Bridges", ["env_bridge"]],
	["Fences, walls, hedges", ["env_fence"]],
	["Fortifications", ["fort_bunker", "fort_trench", "fort_sandbag", "fort_obstacle", "fort_wire",
		"fort_minefield_at", "fort_minefield_ap"]],
]
const BIOME_NAMES: Dictionary[String, String] = {"forest": "Forest", "desert": "Desert", "winter": "Winter"}
const GROUND_COLORS: Dictionary[String, Color] = {
	"forest": Color(0.33, 0.36, 0.24), "desert": Color(0.66, 0.58, 0.43), "winter": Color(0.74, 0.76, 0.78)}

var _biome := "forest"
var _category := "units"
var _holders: Dictionary[String, Node3D] = {}
## Selectable items per category: Node3D with metas gallery_kind, gallery_path, ...
var _items: Dictionary[String, Array] = {"units": [], "infantry": [], "environment": []}
var _models: Array[Node3D] = []  # unit models (prompt 004 checks)
var _soldiers: Array[Dictionary] = []  # {node, faction, role, player}
var _env: Array[Node3D] = []  # environment model roots
var _selected: Node3D
var _info: Label
var _biome_buttons: Dictionary[String, Button] = {}
var _category_buttons: Dictionary[String, Button] = {}
var _clip_select: OptionButton
var _ground: MeshInstance3D
## Lit like the game: sun color and energy, sky, ambient light and grading
## of the biome preset (prompt 015); the sun direction stays a studio light.
var _environment: Environment
var _sky_material: ProceduralSkyMaterial
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
	for c in CATEGORIES:
		var h := Node3D.new()
		h.name = c.capitalize()
		add_child(h)
		_holders[c] = h
	_load_units()
	_load_infantry()
	_load_environment()
	_set_biome("forest")
	_set_category(_arg("--category", "units"))
	if "--tour" in _args:
		_tour.call_deferred()
	if _self_test:
		get_tree().create_timer(240.0).timeout.connect(func() -> void:
			printerr("self-test: FAIL timeout")
			get_tree().quit(1))
		_run_self_test.call_deferred()


func _arg(flag: String, default: String) -> String:
	var i := _args.find(flag)
	return _args[i + 1] if i >= 0 and i + 1 < _args.size() else default


func _process(_delta: float) -> void:
	_frame += 1
	if "--screenshots" in _args:
		_screenshots(_arg("--screenshots", "build/renders"))


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
	_environment = e
	_sky_material = mat
	env.environment = e
	add_child(env)

	_sun = DirectionalLight3D.new()
	_sun.shadow_enabled = true
	_sun.light_energy = 1.3
	_sun.directional_shadow_max_distance = 400.0
	add_child(_sun)
	_sun.look_at_from_position(Vector3(30, 50, 20), Vector3.ZERO, Vector3.UP)

	_ground = MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(1200, 1200)
	_ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = GROUND_COLORS["forest"]
	gmat.roughness = 1.0
	_ground.material_override = gmat
	add_child(_ground)

	_camera = Camera3D.new()
	_camera.fov = 35.0
	_camera.far = 4000.0
	add_child(_camera)


func _build_ui() -> void:
	var layer := CanvasLayer.new()
	layer.visible = "--tour" not in _args
	add_child(layer)
	var bar := HBoxContainer.new()
	bar.position = Vector2(12, 12)
	layer.add_child(bar)
	var title := Label.new()
	title.text = "Model gallery  "
	bar.add_child(title)
	for c in CATEGORIES:
		var b := Button.new()
		b.text = CATEGORY_NAMES[c]
		b.toggle_mode = true
		b.pressed.connect(_set_category.bind(c))
		bar.add_child(b)
		_category_buttons[c] = b
	var sep := Label.new()
	sep.text = "   biome:"
	bar.add_child(sep)
	for biome: String in UnitCamo.BIOMES:
		var b := Button.new()
		b.text = BIOME_NAMES[biome]
		b.toggle_mode = true
		b.pressed.connect(_set_biome.bind(biome))
		bar.add_child(b)
		_biome_buttons[biome] = b
	_clip_select = OptionButton.new()
	for clip_name: String in SoldierLook.clips():
		_clip_select.add_item(clip_name)
	_clip_select.item_selected.connect(func(_i: int) -> void: _play_selected_clip())
	bar.add_child(_clip_select)
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


func _label(parent: Node3D, text: String, height: float, size := 0.01) -> void:
	var label := Label3D.new()
	label.text = text
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.pixel_size = size
	label.position = Vector3(0, height, 0)
	parent.add_child(label)


# --- loading -----------------------------------------------------------------

func _load_units() -> void:
	var row := 0
	for faction in UNIT_FACTIONS:
		var entries: Array[Dictionary] = []
		for path in _model_paths(MODEL_ROOT + faction):
			var scene := load(path) as PackedScene
			if scene == null:
				push_error("model_gallery: cannot load %s" % path)
				continue
			var model := scene.instantiate() as Node3D
			# faction fortifications (prompt 008) are environment models: category Environment
			if not EnvLook.root_extras(model).is_empty():
				model.free()
				continue
			entries.append({"model": model, "cat": UnitCamo.catalog_values(model), "path": path})
		if entries.is_empty():
			continue
		# columns by category, then id
		entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			return [a.cat.get("cat_category", ""), a.cat.get("cat_id", "")] < [b.cat.get("cat_category", ""), b.cat.get("cat_id", "")])
		var x := 0.0
		for i in entries.size():
			var model: Node3D = entries[i].model
			model.set_meta(&"gallery_path", entries[i].path)
			model.set_meta(&"gallery_faction", faction)
			model.set_meta(&"gallery_kind", "unit")
			_holders["units"].add_child(model)
			if _is_squad(model):
				_setup_squad(model)
			var box := _aabb(model)
			model.position = Vector3(x - box.position.x, 0.0, row * ROW_SPACING)
			x += box.size.x + GAP
			if not _is_squad(model):
				UnitCamo.add_markings(model, faction, 100 * (row + 1) + i + 1)
			_label(model, str(entries[i].cat.get("cat_name_en", model.name)), _aabb(model).end.y - model.position.y + 0.6)
			_models.append(model)
			_items["units"].append(model)
		row += 1


func _load_infantry() -> void:
	var row := 0
	for faction in UNIT_FACTIONS:
		var dir_path := MODEL_ROOT + faction + "/infantry"
		var paths := _model_paths(dir_path)
		for i in paths.size():
			var scene := load(paths[i]) as PackedScene
			if scene == null:
				push_error("model_gallery: cannot load %s" % paths[i])
				continue
			var node := scene.instantiate() as Node3D
			var role := paths[i].get_file().get_basename()
			node.set_meta(&"gallery_path", paths[i])
			node.set_meta(&"gallery_faction", faction)
			node.set_meta(&"gallery_kind", "soldier")
			node.set_meta(&"gallery_role", role)
			_holders["infantry"].add_child(node)
			node.position = Vector3(i * 1.6, 0, row * 2.6)
			var player := SoldierLook.find_player(node)
			if player:
				SoldierLook.configure_animations(player)
			_label(node, "%s %s" % [faction, role], 2.15, 0.004)
			_soldiers.append({"node": node, "faction": faction, "role": role, "player": player})
			_items["infantry"].append(node)
		row += 1


func _load_environment() -> void:
	var by_cat: Dictionary[String, Array] = {}
	var paths := _model_paths(MODEL_ROOT + "neutral")
	var faction_paths: Array[String] = []
	for faction in UNIT_FACTIONS:  # faction fortifications (prompt 008)
		for path in _model_paths(MODEL_ROOT + faction):
			if path.ends_with(".glb"):
				faction_paths.append(path)
	for path in paths + faction_paths:
		var scene := load(path) as PackedScene
		if scene == null:
			push_error("model_gallery: cannot load %s" % path)
			continue
		var model := scene.instantiate() as Node3D
		var extras := EnvLook.root_extras(model)
		if extras.is_empty():
			if path in faction_paths:  # a unit model: loaded by _load_units
				model.free()
				continue
			push_error("model_gallery: %s is not an environment model" % path)
			model.free()
			continue
		model.set_meta(&"gallery_path", path)
		model.set_meta(&"gallery_kind", "env")
		var cat: String = extras.get("cat_category", "")
		if not by_cat.has(cat):
			by_cat[cat] = []
		by_cat[cat].append(model)
	var z := 0.0
	for group: Array in ENV_GROUPS:
		var models: Array = []
		for cat: String in group[1]:
			models.append_array(by_cat.get(cat, []))
			by_cat.erase(cat)
		if models.is_empty():
			continue
		var depth := _place_env_row(models, z, group[0])
		z += depth + 8.0
	for cat: String in by_cat:
		var depth := _place_env_row(by_cat[cat], z, cat)
		z += depth + 8.0


## Places the env models of one row; every variant (and far mesh) is moved
## apart inside its model. Returns the row depth.
func _place_env_row(models: Array, z: float, title: String) -> float:
	var x := 0.0
	var depth := 0.0
	var row_label := Node3D.new()
	_holders["environment"].add_child(row_label)
	row_label.position = Vector3(-6, 0, z)
	_label(row_label, title, 2.0, 0.02)
	for model: Node3D in models:
		_holders["environment"].add_child(model)
		model.position = Vector3(0, 0, z)
		var vx := 0.0
		var extras := EnvLook.root_extras(model)
		for v: Dictionary in EnvLook.variants(model):
			var names: Array[String] = [v.name]
			if v.get("far", false):
				names.append(v.name + "_Far")
			for n in names:
				var node := EnvLook.variant_node(model, n)
				if node == null:
					continue
				var box := _aabb(node)
				var size := box.size
				node.position.x += vx - (box.position.x - model.position.x)
				vx += maxf(size.x, 0.3) + maxf(1.0, size.x * 0.15)
				depth = maxf(depth, size.z)
				var tag := Node3D.new()
				node.add_child(tag)
				_label(tag, ("%s far" % v.name) if n.ends_with("_Far") else n,
					_aabb(node).end.y - node.global_position.y + 0.4, clampf(size.y * 0.0015, 0.004, 0.03))
		model.position.x = x
		# sunken objects (trenches) are lifted onto the gallery ground so that
		# the cut is visible; in the game the terrain is cut instead
		if extras.get("env_sunken", false):
			model.position.y = float(extras.get("cat_height_mm", 0)) / 1000.0
		var sub := Node3D.new()
		model.add_child(sub)
		sub.position = Vector3(vx * 0.5, 0, -maxf(depth * 0.5, 1.0) - 1.0)
		_label(sub, str(extras.get("cat_name_en", model.name)), 0.5, 0.012)
		x += vx + 4.0
		_env.append(model)
		_items["environment"].append(model)
	return depth


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


# --- switches ------------------------------------------------------------------

func _set_category(category: String) -> void:
	if not category in CATEGORIES:
		category = "units"
	_category = category
	for c: String in _holders:
		_holders[c].visible = c == category
	for c: String in _category_buttons:
		_category_buttons[c].button_pressed = c == category
	_clip_select.visible = category == "infantry"
	_selected = null
	_info.text = "Click a model to see its catalog values."
	_frame_all()


func _set_biome(biome: String) -> void:
	_biome = biome
	for b: String in _biome_buttons:
		_biome_buttons[b].button_pressed = b == biome
	for model in _models:
		if _is_squad(model):
			_squad_look(model, biome)
		else:
			UnitCamo.apply(model, UnitCamo.palette(model.get_meta(&"gallery_faction"), biome))
	for i in _soldiers.size():
		SoldierLook.apply(_soldiers[i].node, _soldiers[i].faction, biome, i)
	for model in _env:
		EnvLook.apply(model, biome)
	var gmat := _ground.material_override as StandardMaterial3D
	gmat.albedo_color = GROUND_COLORS[biome]
	_biome_light(biome)


## Sun color and energy, sky, ambient light and grading from the biome
## preset, so models look as in the game.
func _biome_light(biome: String) -> void:
	var preset: Resource = Atmosphere.load_preset(biome)
	_sun.light_color = preset.sun_color
	_sun.light_energy = preset.sun_energy
	_sky_material.sky_top_color = preset.sky_top
	_sky_material.sky_horizon_color = preset.sky_horizon
	_sky_material.ground_horizon_color = preset.ground_horizon
	_sky_material.ground_bottom_color = preset.ground_bottom
	_environment.ambient_light_energy = preset.ambient_energy
	_environment.tonemap_mode = Environment.TONE_MAPPER_AGX
	_environment.tonemap_exposure = preset.exposure
	_environment.tonemap_white = preset.white
	_environment.adjustment_enabled = true
	_environment.adjustment_contrast = preset.contrast
	_environment.adjustment_saturation = preset.saturation


func _play_selected_clip() -> void:
	if _clip_select.item_count == 0:
		return
	var clip_name := _clip_select.get_item_text(_clip_select.selected)
	for s in _soldiers:
		var player: AnimationPlayer = s.player
		if player != null and player.has_animation(clip_name):
			player.play(clip_name)


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
				_distance = maxf(1.0, _distance * 0.9)
			MOUSE_BUTTON_WHEEL_DOWN:
				_distance = minf(1500.0, _distance * 1.1)
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
			KEY_TAB: _set_category(CATEGORIES[(CATEGORIES.find(_category) + 1) % CATEGORIES.size()])
			KEY_LEFT, KEY_RIGHT:
				if _category == "infantry" and _clip_select.item_count > 0:
					var step := 1 if key.keycode == KEY_RIGHT else -1
					_clip_select.select(posmod(_clip_select.selected + step, _clip_select.item_count))
					_play_selected_clip()


func _physics_process(_delta: float) -> void:
	var dir := Vector3(sin(_yaw) * cos(_pitch), sin(_pitch), cos(_yaw) * cos(_pitch))
	_camera.global_position = _focus + dir * _distance
	_camera.look_at(_focus, Vector3.UP)


func _frame_all() -> void:
	var items: Array = _items[_category]
	if items.is_empty():
		return
	var box := _aabb(items[0])
	for m: Node3D in items:
		box = box.merge(_aabb(m))
	_focus = box.get_center()
	_distance = maxf(box.size.x, box.size.z) * 1.2 + 5.0


## Ray against the bounding boxes of the visible items (no physics).
func _pick(screen_pos: Vector2) -> void:
	var origin := _camera.project_ray_origin(screen_pos)
	var dir := _camera.project_ray_normal(screen_pos)
	var best: Node3D = null
	var best_t := INF
	for m: Node3D in _pickables():
		var hit: Variant = _aabb(m).intersects_ray(origin, dir)
		if hit != null:
			var t := origin.distance_to(hit as Vector3)
			if t < best_t:
				best_t = t
				best = m
	if best:
		_select(best)


## Units and soldiers are picked whole; environment models per variant.
func _pickables() -> Array[Node3D]:
	var out: Array[Node3D] = []
	if _category != "environment":
		out.assign(_items[_category])
		return out
	for model: Node3D in _env:
		for v: Dictionary in EnvLook.variants(model):
			for n: String in [v.name, v.name + "_Far"]:
				var node := EnvLook.variant_node(model, n)
				if node:
					out.append(node)
	return out


func _select(node: Node3D) -> void:
	_selected = node
	var box := _aabb(node)
	_focus = box.get_center()
	_distance = maxf(box.size.length() * 1.6, 3.0)
	_info.text = _describe(node)


func _describe(node: Node3D) -> String:
	var kind: String = node.get_meta(&"gallery_kind", "variant")
	if kind == "soldier":
		var player := SoldierLook.find_player(node)
		return "%s %s\nClips: %s\nTriangles: %d\nModel: %s" % [node.get_meta(&"gallery_faction"),
			node.get_meta(&"gallery_role"), ", ".join(player.get_animation_list()) if player else "-",
			_triangles(node), node.get_meta(&"gallery_path")]
	if kind == "env":
		var e := EnvLook.root_extras(node)
		var names: Array[String] = []
		for v: Dictionary in EnvLook.variants(node):
			names.append(v.name)
		return "%s (%s)\n%s\nCategory: %s\nCatalog size: %.2f m long, %.2f wide, %.2f high\nVariants: %s\nTriangles (all variants): %d\nModel: %s" % [
			e.get("cat_name_en", ""), e.get("cat_id", ""), e.get("cat_name_de", ""), e.get("cat_category", ""),
			e.get("cat_length_mm", 0) / 1000.0, e.get("cat_width_mm", 0) / 1000.0, e.get("cat_height_mm", 0) / 1000.0,
			", ".join(names), _triangles(node), node.get_meta(&"gallery_path", "")]
	if kind == "variant":
		var model := node
		while model and not model.has_meta(&"gallery_kind"):
			model = model.get_parent() as Node3D
		var c := EnvLook.root_extras(model)
		var box := _aabb(node)
		var lines: Array[String] = [
			"%s (%s)" % [c.get("cat_name_en", ""), c.get("cat_id", "")],
			"%s" % c.get("cat_name_de", ""),
			"Category: %s" % c.get("cat_category", ""),
			"Variant: %s" % node.name,
			"Catalog size: %.2f m long, %.2f wide, %.2f high" % [c.get("cat_length_mm", 0) / 1000.0,
				c.get("cat_width_mm", 0) / 1000.0, c.get("cat_height_mm", 0) / 1000.0],
			"Variant size: %.2f m long, %.2f wide, %.2f high" % [box.size.z, box.size.x, box.size.y],
			"Meshes: %d, triangles: %d" % [UnitCamo.meshes(node).size(), _triangles(node)],
		]
		var extras: Dictionary = node.get_meta(&"extras", {})
		for k: String in extras:
			if k != "env_variant":
				lines.append("%s: %s" % [k, extras[k]])
		lines.append("Model: %s" % model.get_meta(&"gallery_path", ""))
		return "\n".join(lines)
	var c := UnitCamo.catalog_values(node)
	if c.is_empty():
		return "%s\n(no catalog values)" % node.name
	var lines: Array[String] = [
		"%s (%s)" % [c.get("cat_name_en", ""), c.get("cat_id", "")],
		"%s" % c.get("cat_name_de", ""),
		"%s, %d-%d" % [c.get("cat_designation", ""), c.get("cat_year_from", 0), c.get("cat_year_to", 0)],
		"Category: %s" % c.get("cat_category", ""),
	]
	if c.has("cat_length_mm"):  # squads have no size of their own
		var dims := "Size: %.2f m long" % (c.get("cat_length_mm", 0) / 1000.0)
		if c.has("cat_length_hull_mm"):
			dims += " (hull %.2f)" % (c.cat_length_hull_mm / 1000.0)
		dims += ", %.2f wide, %.2f high" % [c.get("cat_width_mm", 0) / 1000.0, c.get("cat_height_mm", 0) / 1000.0]
		lines.append(dims)
	if c.has("cat_mass_kg"):
		lines.append("Mass: %.1f t, crew %d" % [c.cat_mass_kg / 1000.0, c.get("cat_crew", 0)])
	if c.has("cat_speed_road_kmh_x10"):
		var speed := "Speed: %.0f km/h road, %.0f km/h off-road" % [
			c.cat_speed_road_kmh_x10 / 10.0, c.get("cat_speed_offroad_kmh_x10", 0) / 10.0]
		if c.has("cat_range_road_km"):
			speed += ", range %d km" % c.cat_range_road_km
		lines.append(speed)
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
	lines.append("Triangles: %d" % _triangles(node))
	lines.append("Model: %s" % node.get_meta(&"gallery_path", ""))
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
	var box := AABB(model.global_position, Vector3.ZERO)
	var first := true
	for g in UnitCamo.meshes(model):
		var b: AABB = g.global_transform * g.get_aabb()
		box = b if first else box.merge(b)
		first = false
	return box


# --- checks ------------------------------------------------------------------

func _run_self_test() -> void:
	var errors: Array[String] = []
	_check_units(errors)
	_check_infantry(errors)
	_check_environment(errors)
	# switch through all categories and biomes, play every clip a few frames
	for c in CATEGORIES:
		_set_category(c)
		await get_tree().process_frame
	_set_category("infantry")
	for biome: String in UnitCamo.BIOMES:
		_set_biome(biome)
		_check_biome(biome, errors)
		for i in _clip_select.item_count:
			_clip_select.select(i)
			_play_selected_clip()
			await get_tree().process_frame
		for s in _soldiers:
			var player: AnimationPlayer = s.player
			if player and not player.is_playing():
				errors.append("%s/%s: clip %s does not play" % [s.faction, s.role,
					_clip_select.get_item_text(_clip_select.selected)])
				break
	if errors.is_empty():
		print("self-test: gallery ok, %d unit models, %d soldiers, %d environment objects (%d variants), 3 biomes" % [
			_models.size(), _soldiers.size(), _env.size(), _env_variant_count()])
		get_tree().quit(0)
	else:
		for e in errors:
			printerr("self-test: FAIL " + e)
		get_tree().quit(1)


func _env_variant_count() -> int:
	var n := 0
	for m in _env:
		n += EnvLook.variants(m).size()
	return n


func _check_units(errors: Array[String]) -> void:
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
		print("self-test: unit %s loaded, %d camo surfaces, %d triangles" % [id, camo_surfaces, _triangles(model)])
	for req in REQUIRED:
		if not ids.has(req):
			errors.append("required model %s not in the gallery" % req)


func _check_infantry(errors: Array[String]) -> void:
	var clips: Dictionary = SoldierLook.clips()
	if clips.is_empty():
		errors.append("no clip metadata (animations.json)")
	var roles := {}
	for s in _soldiers:
		var tag := "%s/%s" % [s.faction, s.role]
		roles[s.faction] = roles.get(s.faction, []) + [s.role]
		var player: AnimationPlayer = s.player
		if player == null:
			errors.append("%s: no AnimationPlayer" % tag)
			continue
		for clip_name: String in clips:
			if not player.has_animation(clip_name):
				errors.append("%s: clip %s missing" % [tag, clip_name])
		if SoldierLook.find_skeleton(s.node) == null:
			errors.append("%s: no Skeleton3D" % tag)
	for faction in UNIT_FACTIONS:
		for role in REQUIRED_INFANTRY:
			if role not in roles.get(faction, []):
				errors.append("infantry %s/%s not in the gallery" % [faction, role])
	print("self-test: %d soldiers, %d clips" % [_soldiers.size(), clips.size()])


func _check_environment(errors: Array[String]) -> void:
	var env_shader := load("res://assets/shaders/env.gdshader") as Shader
	var fol_shader := load("res://assets/shaders/env_foliage.gdshader") as Shader
	var ids: Array[String] = []
	for model in _env:
		var extras := EnvLook.root_extras(model)
		var id: String = extras.get("cat_id", "")
		ids.append(id)
		var surfaces := 0
		for g in UnitCamo.meshes(model):
			var mi := g as MeshInstance3D
			if mi == null or mi.mesh == null:
				continue
			for s in mi.mesh.get_surface_count():
				var mat := mi.mesh.surface_get_material(s) as ShaderMaterial
				if mat == null or not (mat.shader == env_shader or mat.shader == fol_shader):
					errors.append("%s/%s: surface %d does not use an env shader" % [id, mi.name, s])
				else:
					surfaces += 1
		var variants := EnvLook.variants(model)
		if variants.is_empty():
			errors.append("%s: no variants in the extras" % id)
		for v: Dictionary in variants:
			var node := EnvLook.variant_node(model, v.name)
			if node == null:
				errors.append("%s: variant %s missing" % [id, v.name])
			elif UnitCamo.meshes(node).is_empty():
				errors.append("%s: variant %s has no mesh" % [id, v.name])
			if v.get("far", false) and EnvLook.variant_node(model, v.name + "_Far") == null:
				errors.append("%s: far mesh of %s missing" % [id, v.name])
			if extras.get("cat_category", "") == "env_tree" and not v.get("far", false):
				errors.append("%s: tree variant %s has no far mesh" % [id, v.name])
		print("self-test: env %s loaded, %d variants, %d surfaces, %d triangles" % [
			id, variants.size(), surfaces, _triangles(model)])
	for req in REQUIRED_ENV:
		if not ids.has(req):
			errors.append("required environment object %s not in the gallery" % req)


func _check_biome(biome: String, errors: Array[String]) -> void:
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
	for s in _soldiers:
		for mi: MeshInstance3D in s.node.find_children("*", "MeshInstance3D", true, false):
			for i in mi.mesh.get_surface_count():
				if mi.get_surface_override_material(i) == null:
					errors.append("%s/%s: %s surface %d has no slot material in %s" % [
						s.faction, s.role, mi.name, i, biome])
					break
	var look: Dictionary = EnvLook.LOOK[biome]
	for model in _env:
		for g in UnitCamo.meshes(model):
			if not is_equal_approx(float(g.get_instance_shader_parameter(&"env_snow")), look.snow):
				errors.append("%s: env look not applied for %s" % [model.name, biome])
				break


func _screenshots(dir: String) -> void:
	# a few frames per biome so that shadows and SSAO settle
	var step := 40
	var biomes: Array[String] = UnitCamo.BIOMES
	var idx := _frame / step
	if _frame % step == 0 and idx >= 1 and idx <= biomes.size():
		var img := get_viewport().get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(dir)
		var path := "%s/gallery_%s_%s.png" % [dir, _category, _biome]
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
		var focus := _arg("--focus", "")
		if not focus.is_empty():
			for m: Node3D in _items[_category]:
				var id: String = UnitCamo.catalog_values(m).get("cat_id", "")
				if id == focus or m.get_meta(&"gallery_role", "") == focus:
					_select(m)
					_distance *= 0.8
		var y := _args.find("--yaw")
		if y >= 0 and y + 1 < _args.size():
			_yaw = deg_to_rad(float(_args[y + 1]))
		var clip := _arg("--clip", "")
		for i in _clip_select.item_count:
			if _clip_select.get_item_text(i) == clip:
				_clip_select.select(i)
				_play_selected_clip()


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
	# Unarmed squads (e.g. a lone medic) are valid when the catalog lists no weapon.
	var catalog_armed := not str(cat.get("cat_weapons", "")).strip_edges().is_empty()
	if armed == 0 and catalog_armed:
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


# --- recording tour (-- --tour) ---------------------------------------------------

const TOUR_FPS := 30


func _tour() -> void:
	# Units: overview through all biomes, then close-ups of the tanks.
	_set_category("units")
	await _tour_overview(0.62, 18.0, 9.0)
	var looks: Array[String] = ["forest", "desert", "winter"]
	var tanks: Array[String] = ["de_pz4_h", "su_t34_76_1943", "us_m4a3_75"]
	for i in tanks.size():
		_set_biome(looks[i])
		await _tour_closeup(tanks[i], 3.0)
	_set_biome("forest")
	# Infantry: walking, then firing.
	_set_category("infantry")
	await _tour_clip("walk")
	await _tour_overview(0.55, 24.0, 5.0)
	await _tour_clip("fire_kneel")
	await _tour_orbit(4.0, 20.0)
	# Environment: overview through all biomes.
	_set_category("environment")
	_set_biome("forest")
	await _tour_overview(0.42, 34.0, 10.0, 35.0)
	get_tree().quit()


## Frames the category and orbits by `degrees` over `seconds`, switching the
## biome in thirds (forest, desert, winter).
func _tour_overview(zoom: float, pitch_deg: float, seconds: float, degrees := 70.0) -> void:
	_frame_all()
	_distance *= zoom
	_pitch = deg_to_rad(pitch_deg)
	_yaw = deg_to_rad(205.0)
	var frames := roundi(seconds * TOUR_FPS)
	for i in frames:
		var third := mini(2, 3 * i / frames)
		if UnitCamo.BIOMES[third] != _biome:
			_set_biome(UnitCamo.BIOMES[third])
		_yaw += deg_to_rad(degrees) / frames
		await get_tree().process_frame


func _tour_orbit(seconds: float, degrees: float) -> void:
	var frames := roundi(seconds * TOUR_FPS)
	for i in frames:
		_yaw += deg_to_rad(degrees) / frames
		await get_tree().process_frame


func _tour_closeup(id: String, seconds: float) -> void:
	for m: Node3D in _items["units"]:
		if UnitCamo.catalog_values(m).get("cat_id", "") == id:
			var box := _aabb(m)
			_focus = box.get_center()
			_distance = maxf(box.size.x, box.size.z) * 1.6
			_pitch = deg_to_rad(22.0)
			_yaw = deg_to_rad(215.0)
			await _tour_orbit(seconds, 50.0)
			return


func _tour_clip(clip: String) -> void:
	for i in _clip_select.item_count:
		if _clip_select.get_item_text(i) == clip:
			_clip_select.select(i)
			_play_selected_clip()
	await get_tree().process_frame
