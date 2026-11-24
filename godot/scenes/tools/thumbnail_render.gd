extends Node
## Renders the picker thumbnails (prompt 058): one PNG per id of
## `SimHost.buildable_ids()` (every base building, fortification and
## producible unit of every faction, from data/economy.toml), so new
## buildables get a picture by running it again. A development tool:
##
##     just thumbnails            # needs a window (no headless rendering)
##     ... -- --only=de_pz4_h,base_hq   # just these ids
##
## Every model is shown the same way: forest camouflage, orthographic camera
## from the front left and 30 degrees above (the models face -Z), a key
## light from the upper left with soft shadows, a cool fill light, grey
## ambient, a transparent background (the tile draws its own backdrop),
## framed to fill the picture. Output: `godot/assets/thumbnails/<id>.png`
## at `Thumbnail.SIZE`; run `just import` afterwards.

const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const BaseView := preload("res://scenes/world/base_view.gd")
const InfantryView := preload("res://scenes/world/infantry_view.gd")
const GunCrewView := preload("res://scenes/world/gun_crew_view.gd")
const MODELS_DIR := "res://assets/models/"
const BIOME := "forest"
## Camera direction from the model (front left, 30 degrees up).
const VIEW_YAW_DEG := -35.0
const VIEW_PITCH_DEG := 30.0
## Empty border around the model, as a share of the picture.
const MARGIN := 0.05
## The model is rendered this many times larger, cropped to its pixels and
## scaled down (tight framing and smooth edges).
const OVERSAMPLE := 3
const SETTLE_FRAMES := 4
## Where the soldiers of a squad stand in the picture (a tight group,
## spread evenly as seen by the camera).
const SQUAD_SPOTS: Array[Vector3] = [
	Vector3(0.0, 0.0, 0.0), Vector3(1.1, 0.0, -0.7), Vector3(-1.1, 0.0, 0.7),
	Vector3(0.9, 0.0, 1.0), Vector3(-0.9, 0.0, -1.0),
]

var _viewport: SubViewport
var _camera: Camera3D


func _ready() -> void:
	# Trees at rest (prompt 086): the same picture every time.
	EnvLook.still_trees()
	_viewport = SubViewport.new()
	_viewport.size = Thumbnail.SIZE * OVERSAMPLE
	_viewport.own_world_3d = true
	_viewport.transparent_bg = true
	_viewport.msaa_3d = Viewport.MSAA_8X
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(_viewport)
	var key := DirectionalLight3D.new()
	key.rotation_degrees = Vector3(-48, -70, 0)
	key.light_energy = 1.1
	key.shadow_enabled = true
	key.directional_shadow_mode = DirectionalLight3D.SHADOW_ORTHOGONAL
	_viewport.add_child(key)
	var fill := DirectionalLight3D.new()
	fill.rotation_degrees = Vector3(-20, 150, 0)
	fill.light_energy = 0.45
	fill.light_color = Color(0.8, 0.86, 1.0)
	_viewport.add_child(fill)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_CLEAR_COLOR
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.72, 0.73, 0.76)
	environment.ambient_light_energy = 0.6
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	_viewport.add_child(world_environment)
	_camera = Camera3D.new()
	_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	_camera.rotation_degrees = Vector3(-VIEW_PITCH_DEG, VIEW_YAW_DEG + 180.0, 0)
	_camera.current = true
	_viewport.add_child(_camera)
	_run.call_deferred()


func _run() -> void:
	var ids := Array(SimHost.buildable_ids())
	# `--raw=<dir>` also saves the unscaled render (three times the
	# thumbnail's size), to judge a model from closer up.
	var raw_dir := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--only="):
			ids = Array(arg.trim_prefix("--only=").split(",", false))
		elif arg.begins_with("--raw="):
			raw_dir = arg.trim_prefix("--raw=")
	if ids.is_empty():
		printerr("thumbnails: no buildable ids (is the bridge built?)")
		get_tree().quit(1)
		return
	var out := ProjectSettings.globalize_path(Thumbnail.DIR)
	DirAccess.make_dir_recursive_absolute(out)
	var missing: Array[String] = []
	for id: String in ids:
		var model := build_model(id)
		if model == null:
			missing.append(id)
			continue
		_viewport.add_child(model)
		_frame(model)
		for i in SETTLE_FRAMES:
			await RenderingServer.frame_post_draw
		if raw_dir != "":
			DirAccess.make_dir_recursive_absolute(raw_dir)
			_viewport.get_texture().get_image().save_png(raw_dir.path_join("%s.png" % id))
		var image := _fit(_viewport.get_texture().get_image())
		var file := out.path_join(id + ".png")
		if image.save_png(file) != OK:
			printerr("thumbnails: cannot write %s" % file)
			missing.append(id)
		else:
			print("thumbnails: %s" % file)
		_viewport.remove_child(model)
		model.free()
	if not missing.is_empty():
		printerr("thumbnails: no model for %s" % ", ".join(missing))
	print("thumbnails: %d of %d rendered" % [ids.size() - missing.size(), ids.size()])
	get_tree().quit(1 if not missing.is_empty() else 0)


## The model of `id` as the match shows it in `biome` (`forest`, `desert`,
## `winter`; null if there is none): base buildings borrow houses, squads
## are squad scenes, the rest are kit models with camouflage or
## fortifications with the environment look; guns show their crew (prompt
## 193). Also the model of the unit
## reference in the main menu (prompt 066, `menu/unit_model_view.gd`).
static func build_model(id: String, biome := BIOME) -> Node3D:
	if BaseView.is_building(id):
		var holder := Node3D.new()
		holder.add_child(Node3D.new())
		holder.get_child(0).name = "Model"
		BaseView.attach_model(holder, id, biome)
		return holder if holder.get_node_or_null("Model/House") != null else _discard(holder)
	var faction := id.get_slice("_", 0)
	var squad := InfantryView.squad_model(MODELS_DIR, id)
	if squad != null:
		var k := 0
		for child in squad.get_children():
			var player := SoldierLook.find_player(child)
			if child is Node3D and player != null:
				SoldierLook.apply(child, faction, biome, k)
				SoldierLook.configure_animations(player)
				if player.has_animation("idle"):
					player.play("idle")
					player.seek(0.0, true)
					player.pause()
				k += 1
		# Squad scenes stand in their real formation, often a wide line with
		# tiny soldiers in the picture: show the first SQUAD_SPOTS.size()
		# members in a tight group instead (the same for every squad).
		var shown := 0
		for child in squad.get_children():
			if not (child is Node3D and SoldierLook.find_player(child) != null):
				continue
			(child as Node3D).visible = shown < SQUAD_SPOTS.size()
			if shown < SQUAD_SPOTS.size():
				(child as Node3D).position = SQUAD_SPOTS[shown]
			shown += 1
		return squad
	var path := "%s%s/%s.glb" % [MODELS_DIR, faction, id]
	if not ResourceLoader.exists(path):
		# Environment objects and fortifications (env_*, fort_*) live in neutral/.
		path = "%sneutral/%s.glb" % [MODELS_DIR, id]
	if not ResourceLoader.exists(path):
		return null
	var model := (load(path) as PackedScene).instantiate() as Node3D
	if EnvLook.is_env_model(model):
		EnvLook.apply(model, biome)
		var variants := EnvLook.variants(model)
		if not variants.is_empty():
			EnvLook.show_variant(model, variants[0].name)
	else:
		UnitCamo.apply(model, UnitCamo.palette_for(faction, biome, UnitCamo.catalog_values(model)))
		UnitCamo.add_markings(model, faction, 101)
		# Towed guns and heavy mortars with their crew at work (prompt 193).
		if str(SimHost.catalog_card(id).get("group", "")) == "gun":
			GunCrewView.add_still_crew(model, faction, biome)
	for player in model.find_children("*", "AnimationPlayer", true, false):
		(player as AnimationPlayer).pause()
	return model


static func _discard(node: Node) -> Node3D:
	node.free()
	return null


## Orthographic framing: the model's bounds in camera space decide the
## picture's center and size.
func _frame(model: Node3D) -> void:
	var bounds := visible_bounds(model)
	var basis := _camera.global_transform.basis
	var inverse := basis.inverse()
	var low := Vector3.INF
	var high := -Vector3.INF
	for i in 8:
		var corner := inverse * bounds.get_endpoint(i)
		low = low.min(corner)
		high = high.max(corner)
	var aspect := float(Thumbnail.SIZE.x) / Thumbnail.SIZE.y
	var width := (high.x - low.x)
	var height := (high.y - low.y)
	# Generous: the picture is cropped to the model's pixels afterwards.
	_camera.size = maxf(height, width / aspect) * 1.1
	var center := (low + high) * 0.5
	_camera.position = basis * Vector3(center.x, center.y, high.z + 50.0)
	_camera.near = 0.1
	_camera.far = 200.0 + (high.z - low.z)


## Crops the render to its opaque pixels and scales that into the middle of
## a transparent `Thumbnail.SIZE` picture with a small margin.
static func _fit(render: Image) -> Image:
	render.convert(Image.FORMAT_RGBA8)
	var used := render.get_used_rect()
	var result := Image.create_empty(Thumbnail.SIZE.x, Thumbnail.SIZE.y, false, Image.FORMAT_RGBA8)
	if used.size.x <= 0 or used.size.y <= 0:
		return result
	var part := render.get_region(used)
	var room := Vector2(Thumbnail.SIZE) * (1.0 - 2.0 * MARGIN)
	var scale := minf(room.x / used.size.x, room.y / used.size.y)
	var size := Vector2i((Vector2(used.size) * scale).round()).max(Vector2i.ONE)
	part.resize(size.x, size.y, Image.INTERPOLATE_LANCZOS)
	# Bottom-aligned like standing on the tile's lower edge, centered across.
	var at := Vector2i((Thumbnail.SIZE.x - size.x) / 2, Thumbnail.SIZE.y - size.y - int(Thumbnail.SIZE.y * MARGIN))
	result.blit_rect(part, Rect2i(Vector2i.ZERO, size), at)
	return result


## Bounds of every visible mesh below `node` in world space.
static func visible_bounds(node: Node3D) -> AABB:
	var result := AABB()
	var first := true
	for child in node.find_children("*", "VisualInstance3D", true, false):
		var visual := child as VisualInstance3D
		if not visual.is_visible_in_tree() or visual is Light3D or visual is Decal:
			continue
		var box := visual.global_transform * visual.get_aabb()
		result = box if first else result.merge(box)
		first = false
	return result
