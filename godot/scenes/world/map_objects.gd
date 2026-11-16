extends Node3D
## Map objects and decoration (prompt 013). Presentation only.
##
## Objects (trees, houses, walls, hedges, props) belong to the simulation:
## `SimHost.object_groups(CHUNK_M)` returns them as MultiMesh transform
## buffers per kind (catalog id + model variant) and chunk. Each group gets one
## MultiMeshInstance3D per mesh part of the variant (buildings: walls, roof,
## floor, debris), all sharing the buffer. Trees switch to their `_Far` mesh
## at FAR_DISTANCE through visibility ranges per chunk. Shadows only for
## larger objects; small props disappear beyond their view distance.
##
## Decoration (grass, flowers, bushes, small stones, stumps) exists only here:
## `SimHost.scatter_deco` scatters it in Rust per DECO_CHUNK_M square from the
## terrain type, a patch noise and the wood density, never on buildings,
## walls, hedges or props. Only squares near the camera are built (a few per
## frame) and freed again when the camera moves away. Same seed, same
## decoration.
##
## The biome look (EnvLook) and wind come from the env shaders of prompt 007.
##
## Bridges (prompt 014) stand at the simulation's deck height: the bridge
## places them there, and each model is lowered by its own deck height (glTF
## extra `deck_height_m` of the variant). Reeds grow in a band along the
## water's edge (`shore`: signed meters, negative in the water).

## Chunk size of objects in meters (same as the terrain chunks).
const CHUNK_M := 128.0
## Distance (m) at which trees switch to their far mesh.
const FAR_DISTANCE := 220.0
const MODELS_DIR := "res://assets/models/neutral/"

## Per catalog id prefix: view distance (0 = unlimited) and shadow casting.
const OBJECT_CLASSES: Array[Dictionary] = [
	{"prefix": "env_tree_stump", "range": 400.0, "shadow": true},
	{"prefix": "env_tree_", "range": 0.0, "shadow": true},
	{"prefix": "env_house", "range": 0.0, "shadow": true},
	{"prefix": "env_barn", "range": 0.0, "shadow": true},
	{"prefix": "env_church", "range": 0.0, "shadow": true},
	{"prefix": "env_hedge", "range": 1400.0, "shadow": true},
	{"prefix": "env_wall", "range": 900.0, "shadow": true},
	{"prefix": "env_fence", "range": 500.0, "shadow": false},
	{"prefix": "env_hay", "range": 900.0, "shadow": true},
	{"prefix": "env_rock", "range": 700.0, "shadow": true},
	{"prefix": "env_pole", "range": 700.0, "shadow": true},
	{"prefix": "env_log", "range": 450.0, "shadow": true},
	{"prefix": "env_bridge", "range": 0.0, "shadow": true},
	{"prefix": "fort_", "range": 700.0, "shadow": true},
	{"prefix": "", "range": 300.0, "shadow": false},
]

## Square size of decoration chunks in meters.
const DECO_CHUNK_M := 64.0
## Decoration squares built per frame at most (keeps the frame time flat).
const DECO_BUILDS_PER_FRAME := 6
## Decoration layers. `density` per terrain id (grass, field track, road,
## sand, mud, snow, shallow water, deep water) is the chance per lattice
## point of `spacing` meters; `wood` scales it in dense wood.
const DECO_LAYERS: Array[Dictionary] = [
	{"name": "grass", "model": "env_grass_clump", "variants": ["Grass", "GrassTall", "Flowers"],
		"weights": [0.62, 0.3, 0.08], "spacing": 0.7, "seed": 11,
		"density": [0.8, 0.12, 0.0, 0.12, 0.3, 0.0, 0.0, 0.0], "wood": 0.45, "patchiness": 0.6,
		"scale": Vector2(0.45, 0.9), "sink": 0.02, "range": 100.0, "shadow": false,
		"tint": Color(1.45, 1.4, 1.15)},
	{"name": "bushes", "model": "env_bush", "variants": ["Hazel", "Round", "Wide"],
		"weights": [0.4, 0.35, 0.25], "spacing": 8.0, "seed": 23,
		"density": [0.035, 0.0, 0.0, 0.03, 0.03, 0.0, 0.0, 0.0], "wood": 7.0, "patchiness": 0.9,
		"scale": Vector2(0.7, 1.15), "sink": 0.1, "range": 500.0, "shadow": true},
	{"name": "stones", "model": "env_rock", "variants": ["Small"], "weights": [1.0],
		"spacing": 5.0, "seed": 37,
		"density": [0.03, 0.06, 0.0, 0.12, 0.05, 0.0, 0.15, 0.0], "wood": 1.5, "patchiness": 0.8,
		"scale": Vector2(0.5, 1.4), "sink": 0.05, "range": 160.0, "shadow": false},
	{"name": "reeds", "model": "env_grass_clump", "variants": ["Reeds", "GrassTall"],
		"weights": [0.85, 0.15], "spacing": 0.8, "seed": 53,
		"density": [0.25, 0.0, 0.0, 0.45, 0.5, 0.0, 0.6, 0.0], "wood": 1.0, "patchiness": 1.0,
		"scale": Vector2(0.7, 1.25), "sink": 0.05, "range": 140.0, "shadow": false,
		"shore": Vector2(-1.5, 1.5), "tint": Color(1.35, 1.3, 1.05)},
	{"name": "bank_bushes", "model": "env_bush", "variants": ["Hazel", "Wide"],
		"weights": [0.6, 0.4], "spacing": 6.0, "seed": 59,
		"density": [0.07, 0.0, 0.0, 0.05, 0.07, 0.0, 0.0, 0.0], "wood": 1.0, "patchiness": 1.0,
		"scale": Vector2(0.7, 1.2), "sink": 0.1, "range": 500.0, "shadow": true,
		"shore": Vector2(3.0, 10.0)},
	{"name": "stumps", "model": "env_tree_stump", "variants": ["Broken", "Sawn"],
		"weights": [0.6, 0.4], "spacing": 14.0, "seed": 41,
		"density": [0.006, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], "wood": 20.0, "patchiness": 0.5,
		"scale": Vector2(0.8, 1.2), "sink": 0.05, "range": 260.0, "shadow": true},
]

## Time of the last `_process` in microseconds (benchmark, prompt 016).
var process_usec := 0
## SimHost, or MapEditor in the map editor (prompt 028).
var sim_host: Object
var camera: Camera3D
## Biome look of EnvLook ("forest", "desert", "winter").
var biome := "forest"
## Instances of map objects (= SimHost.object_count()).
var object_instances := 0
## MultiMeshInstance3D nodes of map objects.
var object_nodes := 0
## Decoration instances currently built, and their squares.
var deco_instances := 0
var deco_chunks := 0
## Duration of `build` in milliseconds.
var build_msec := 0.0
## Objects drawn in another stage than their map variant (prompt 020):
## object index -> the node drawn instead (null: only hidden).
var swapped: Dictionary[int, Node3D] = {}
## Objects charred by fire (prompt 041): object index -> amount 0..1.
var charred: Dictionary[int, float] = {}

var _objects_root: Node3D
## Single nodes of damaged and destroyed objects (prompt 020).
var _destroyed_root: Node3D
var _deco_root: Node3D
var _models: Dictionary[String, Node3D] = {}
## Per decoration layer: Vector2i square -> Array of MultiMeshInstance3D.
var _deco: Array[Dictionary] = []
## Per decoration layer: the meshes of its variants.
var _deco_meshes: Array[Array] = []
var _last_deco_origin := Vector3.INF
var _deco_enabled := true
## "kind:chunk_x:chunk_y" -> the MultiMeshInstance3D nodes of that group.
var _group_nodes: Dictionary[String, Array] = {}
## "kind:chunk_x:chunk_y" -> the transform buffer of that group (prompt 020).
var _group_buffers: Dictionary[String, PackedFloat32Array] = {}
## Objects whose MultiMesh instance is collapsed (prompt 020).
var hidden: Dictionary[int, bool] = {}
## Draw the hidden minefields of the map editor (`fort_mine_*`, prompt
## 035). Matches leave them out: the simulation turns them into hidden
## mines; the editor sets this.
var show_minefields := false
## Kind index -> "<catalog id>:<variant>".
var _kinds: PackedStringArray = []


## Builds all object instances of the map loaded in `host`.
func build(host: Object, view: Camera3D) -> void:
	var started := Time.get_ticks_usec()
	sim_host = host
	camera = view
	_objects_root = Node3D.new()
	_objects_root.name = "Objects"
	add_child(_objects_root)
	_deco_root = Node3D.new()
	_deco_root.name = "Decoration"
	add_child(_deco_root)
	_destroyed_root = Node3D.new()
	_destroyed_root.name = "Destroyed"
	add_child(_destroyed_root)

	var kinds: PackedStringArray = host.object_kinds()
	_kinds = kinds
	var parts_of_kind: Array = []
	for kind in kinds:
		parts_of_kind.append(_parts(kind.get_slice(":", 0), kind.get_slice(":", 1)))
	for group: Dictionary in host.object_groups(CHUNK_M):
		var kind: int = group.kind
		var id := kinds[kind].get_slice(":", 0)
		if id.begins_with("fort_mine_") and not show_minefields:
			continue
		var buffer: PackedFloat32Array = group.buffer
		var count := buffer.size() / 12
		object_instances += count
		var cls := _class_of(id)
		for part: Dictionary in parts_of_kind[kind]:
			var node := _multimesh(part.mesh, buffer, part.transform)
			node.name = "%s_%s_%d_%d" % [kinds[kind].replace(":", "_"), part.name, group.chunk.x, group.chunk.y]
			node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cls.shadow \
				else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			var range_end: float = cls.range
			match part.lod:
				"near":
					# Near trees cast their shadow through the far mesh (below).
					range_end = FAR_DISTANCE
					node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
				"shadow":
					range_end = FAR_DISTANCE
					node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
				"far":
					node.visibility_range_begin = FAR_DISTANCE
					node.visibility_range_begin_margin = FAR_DISTANCE * 0.1
			if range_end > 0.0:
				node.visibility_range_end = range_end
				node.visibility_range_end_margin = range_end * 0.1
			node.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
			node.set_meta(&"local", part.transform)
			_objects_root.add_child(node)
			object_nodes += 1
			var key := "%d:%d:%d" % [kind, group.chunk.x, group.chunk.y]
			if not _group_nodes.has(key):
				_group_nodes[key] = []
				_group_buffers[key] = buffer
			_group_nodes[key].append(node)

	for layer: Dictionary in DECO_LAYERS:
		var meshes: Array = []
		var model := _model(layer.model)
		for variant: String in layer.variants:
			var mesh_node := EnvLook.variant_node(model, variant) as MeshInstance3D if model else null
			meshes.append(mesh_node.mesh if mesh_node else null)
		_deco_meshes.append(meshes)
		_deco.append({})
	build_msec = (Time.get_ticks_usec() - started) / 1000.0
	print("map objects: %d instances in %d nodes, %d kinds, built in %.0f ms" % [
		object_instances, object_nodes, kinds.size(), build_msec
	])


func _process(_delta: float) -> void:
	process_usec = 0
	if camera == null or not _deco_enabled:
		return
	_rebuild_crater_squares()
	var origin := camera.global_position
	if origin.distance_to(_last_deco_origin) < 2.0:
		return
	var began := Time.get_ticks_usec()
	if _update_deco(origin):
		_last_deco_origin = origin
	process_usec = Time.get_ticks_usec() - began


## Switches the biome look (prompt 015): map objects are repainted, the
## decoration is rebuilt with the new look.
func set_biome(id: String) -> void:
	biome = id
	if _objects_root == null:
		return
	for node: GeometryInstance3D in _objects_root.get_children():
		_apply_look(node)
	for node in _destroyed_root.find_children("*", "GeometryInstance3D", true, false):
		_apply_look(node as GeometryInstance3D)
	for layer in _deco.size():
		for square: Vector2i in _deco[layer].keys():
			_free_deco(layer, square)
	_last_deco_origin = Vector3.INF


# --- Craters (prompt 021) ----------------------------------------------------

## Built decoration squares to rebuild after craters, per layer: square -> true.
var _crater_squares: Array[Dictionary] = []
## Decoration squares rebuilt because of craters (self-test, tools).
var crater_rebuilds := 0


## The map samples in `rect` changed (craters): every built decoration
## square that overlaps them is rebuilt (a few per frame). `scatter_deco`
## leaves deformed samples empty, so nothing grows in a crater.
func craters_changed(rect: Rect2i) -> void:
	if _deco.is_empty():
		return
	var cell: float = sim_host.cell_size()
	var low := Vector2(rect.position) * cell / DECO_CHUNK_M
	var high := Vector2(rect.end) * cell / DECO_CHUNK_M
	if _crater_squares.size() != _deco.size():
		_crater_squares.resize(_deco.size())
		for layer in _deco.size():
			_crater_squares[layer] = {}
	for j in range(floori(low.y), floori(high.y) + 1):
		for i in range(floori(low.x), floori(high.x) + 1):
			var square := Vector2i(i, j)
			for layer in _deco.size():
				if _deco[layer].has(square):
					_crater_squares[layer][square] = true


func _rebuild_crater_squares() -> void:
	var budget := DECO_BUILDS_PER_FRAME
	for layer in _crater_squares.size():
		for square: Vector2i in _crater_squares[layer].keys():
			if budget == 0:
				return
			_crater_squares[layer].erase(square)
			if not _deco[layer].has(square):
				continue
			_free_deco(layer, square)
			_build_deco(layer, square)
			crater_rebuilds += 1
			budget -= 1


# --- Destruction (prompt 020) ------------------------------------------------


## Seconds a felled tree takes to hit the ground; flattened fences fall faster.
const TREE_FALL_SECONDS := 1.6
const FLAT_FALL_SECONDS := 0.35


## Draws object `state.object` in its new stage (a dictionary of
## `SimHost.take_object_events` / `changed_objects`): its instance in the
## MultiMesh is hidden and, unless nothing is left, a node with the damaged
## or ruin variant, a falling tree with a stump, or a flattened fence takes
## its place. `animate` false puts fallen things down at once.
func set_object_state(state: Dictionary, animate := true) -> void:
	var index: int = state.object
	var found: Variant = _instance_transform(index, true)
	if found == null:
		return
	var transform: Transform3D = found
	var variant: String = state.variant
	var fall_deg: int = state.fall_deg
	if swapped.has(index):
		var old: Node3D = swapped[index]
		if old != null:
			old.queue_free()
		swapped.erase(index)
	var id: String = state.id
	var node: Node3D = null
	if state.stage == "intact":
		_instance_transform(index, false)
		return
	if state.stage == "damaged" or state.remains == "ruin":
		node = _variant_node(id, variant, transform)
	elif state.remains == "fallen":
		node = Node3D.new()
		node.add_child(_variant_node("env_tree_stump", "Broken", Transform3D(transform.basis, Vector3.ZERO)))
		var tree: Node3D = _falling(_variant_node(id, variant, Transform3D(transform.basis, Vector3.ZERO)),
			fall_deg, TREE_FALL_SECONDS if animate else 0.0, deg_to_rad(86.0))
		node.add_child(tree)
		node.position = transform.origin
	elif state.remains == "flat":
		node = Node3D.new()
		node.add_child(_falling(_variant_node(id, variant, Transform3D(transform.basis, Vector3.ZERO)),
			fall_deg, FLAT_FALL_SECONDS if animate else 0.0, deg_to_rad(84.0)))
		node.position = transform.origin
	if node != null:
		node.name = "Destroyed_%d" % index
		_destroyed_root.add_child(node)
		if charred.has(index):
			_char_node(node, charred[index])
	swapped[index] = node


## Draws object `index` charred by fire (prompt 041), `amount` 0..1. An
## object still drawn in its MultiMesh (a burnt-out mud house can stay
## intact) gets a node of its own first.
func set_charred(index: int, amount: float) -> void:
	charred[index] = maxf(amount, charred.get(index, 0.0))
	var node: Node3D = swapped.get(index)
	if node == null and not swapped.has(index):
		var state: Dictionary = sim_host.object_state(index)
		var found: Variant = _instance_transform(index, true)
		if found == null or state.is_empty():
			return
		node = _variant_node(state.id, state.variant, found)
		node.name = "Charred_%d" % index
		_destroyed_root.add_child(node)
		swapped[index] = node
	if node != null:
		_char_node(node, charred[index])


func _char_node(node: Node, amount: float) -> void:
	if node is GeometryInstance3D:
		(node as GeometryInstance3D).set_instance_shader_parameter(&"env_char", amount)
	for child in node.get_children():
		_char_node(child, amount)


## The object transform of `index` (from its group buffer); with `hide`
## its MultiMesh instance is collapsed, otherwise it is drawn again. Null
## for an unknown object.
func _instance_transform(index: int, hide: bool) -> Variant:
	var inst: Dictionary = sim_host.object_instance(index, CHUNK_M)
	if inst.is_empty():
		return null
	var chunk: Vector2i = inst.chunk
	var key := "%d:%d:%d" % [inst.kind, chunk.x, chunk.y]
	if not _group_buffers.has(key):
		return null
	var b: PackedFloat32Array = _group_buffers[key]
	var o: int = inst.slot * 12
	var t := Transform3D(
		Vector3(b[o], b[o + 4], b[o + 8]), Vector3(b[o + 1], b[o + 5], b[o + 9]),
		Vector3(b[o + 2], b[o + 6], b[o + 10]), Vector3(b[o + 3], b[o + 7], b[o + 11]),
	)
	for node: MultiMeshInstance3D in _group_nodes[key]:
		var local: Transform3D = node.get_meta(&"local")
		var shown := t * local
		node.multimesh.set_instance_transform(inst.slot,
			Transform3D(Basis().scaled(Vector3.ZERO), shown.origin) if hide else shown)
	if hide:
		hidden[index] = true
	else:
		hidden.erase(index)
	return t


## A node with the mesh parts of `variant` of model `id` at `transform`
## (near meshes only: no far LOD for single objects).
func _variant_node(id: String, variant: String, transform: Transform3D) -> Node3D:
	var root := Node3D.new()
	root.transform = transform
	var cls := _class_of(id)
	for part: Dictionary in _parts(id, variant):
		if part.lod == "far" or part.lod == "shadow":
			continue
		var mesh := MeshInstance3D.new()
		mesh.mesh = part.mesh
		mesh.transform = part.transform
		mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cls.shadow \
			else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		_apply_look(mesh)
		root.add_child(mesh)
	return root


## Wraps `node` in a pivot at its base that tips it over towards
## `fall_deg` (simulation heading: 0 = north = -Z, 90 = east = +X) by
## `angle`, animated over `seconds` (0: at once).
func _falling(node: Node3D, fall_deg: int, seconds: float, angle: float) -> Node3D:
	var pivot := Node3D.new()
	pivot.add_child(node)
	var heading := deg_to_rad(float(fall_deg))
	var direction := Vector3(sin(heading), 0.0, -cos(heading))
	var axis := Vector3.UP.cross(direction).normalized()
	if seconds <= 0.0:
		pivot.basis = Basis(axis, angle)
		return pivot
	# The tween starts once the pivot is in the scene tree.
	pivot.tree_entered.connect(func() -> void:
		var t := pivot.create_tween()
		t.tween_method(func(a: float) -> void: pivot.basis = Basis(axis, a), 0.0, angle, seconds) \
			.set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_IN), CONNECT_ONE_SHOT)
	return pivot


## Shows or hides the map objects (debug menu).
func set_objects_visible(on: bool) -> void:
	_objects_root.visible = on
	_destroyed_root.visible = on


## Turns shadow casting of all map objects off (or back to their class).
func set_shadows(on: bool) -> void:
	for node: MultiMeshInstance3D in _objects_root.get_children():
		if not on:
			node.set_meta(&"shadow", node.cast_shadow) if not node.has_meta(&"shadow") else null
			node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		elif node.has_meta(&"shadow"):
			node.cast_shadow = node.get_meta(&"shadow")


## Shows or hides the decoration (debug menu); hidden decoration is freed.
func set_decoration_visible(on: bool) -> void:
	_deco_enabled = on
	_deco_root.visible = on
	if not on:
		for layer in _deco.size():
			for square: Vector2i in _deco[layer].keys():
				_free_deco(layer, square)
		_last_deco_origin = Vector3.INF


## Builds missing decoration squares near `origin` (a few per call) and frees
## far ones. Returns true when nothing is left to build.
func _update_deco(origin: Vector3) -> bool:
	var size: Vector2 = sim_host.map_size()
	var squares := Vector2i(ceili(size.x / DECO_CHUNK_M), ceili(size.y / DECO_CHUNK_M))
	var half_diag := DECO_CHUNK_M * 0.7072
	var wanted: Array = []
	for layer in DECO_LAYERS.size():
		var range_m: float = DECO_LAYERS[layer].range
		# Free squares well beyond the range.
		for square: Vector2i in _deco[layer].keys():
			if _square_distance(square, origin) > range_m + half_diag + DECO_CHUNK_M:
				_free_deco(layer, square)
		# The camera can be high above the ground: only squares whose
		# center lies within range (plus half a diagonal) are needed.
		var height_above := maxf(origin.y - sim_host.height_at(origin.x, origin.z), 0.0)
		if height_above > range_m + half_diag:
			continue
		var reach := ceili((range_m + half_diag) / DECO_CHUNK_M)
		var center := Vector2i(
			floori((origin.x + size.x * 0.5) / DECO_CHUNK_M), floori((origin.z + size.y * 0.5) / DECO_CHUNK_M)
		)
		for j in range(maxi(center.y - reach, 0), mini(center.y + reach + 1, squares.y)):
			for i in range(maxi(center.x - reach, 0), mini(center.x + reach + 1, squares.x)):
				var square := Vector2i(i, j)
				if _deco[layer].has(square):
					continue
				var d := _square_distance(square, origin)
				if d <= range_m + half_diag:
					wanted.append([d, layer, square])
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for k in mini(wanted.size(), DECO_BUILDS_PER_FRAME):
		_build_deco(wanted[k][1], wanted[k][2])
	return wanted.size() <= DECO_BUILDS_PER_FRAME


func _square_distance(square: Vector2i, origin: Vector3) -> float:
	var size: Vector2 = sim_host.map_size()
	var cx := -size.x * 0.5 + (square.x + 0.5) * DECO_CHUNK_M
	var cz := -size.y * 0.5 + (square.y + 0.5) * DECO_CHUNK_M
	return origin.distance_to(Vector3(cx, sim_host.height_at(cx, cz), cz))


func _build_deco(layer_index: int, square: Vector2i) -> void:
	var layer: Dictionary = DECO_LAYERS[layer_index]
	var size: Vector2 = sim_host.map_size()
	var x0 := -size.x * 0.5 + square.x * DECO_CHUNK_M
	var z0 := -size.y * 0.5 + square.y * DECO_CHUNK_M
	var buffers: Array = sim_host.scatter_deco(
		x0, z0, DECO_CHUNK_M, layer.spacing, layer.seed, PackedFloat32Array(layer.density),
		layer.wood, layer.patchiness, PackedFloat32Array(layer.weights), layer.scale, layer.sink,
		layer.get("shore", Vector2.ZERO)
	)
	var nodes: Array = []
	for v in buffers.size():
		var buffer: PackedFloat32Array = buffers[v]
		var mesh: Mesh = _deco_meshes[layer_index][v]
		if buffer.is_empty() or mesh == null:
			continue
		var node := _multimesh(mesh, buffer, Transform3D.IDENTITY)
		_apply_look(node, layer.get("tint", Color.WHITE))
		node.name = "%s_%d_%d_%d" % [layer.name, v, square.x, square.y]
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if layer.shadow \
			else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.visibility_range_end = layer.range
		node.visibility_range_end_margin = layer.range * 0.15
		node.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
		_deco_root.add_child(node)
		nodes.append(node)
		deco_instances += buffer.size() / 12
	_deco[layer_index][square] = nodes
	deco_chunks += 1


func _free_deco(layer: int, square: Vector2i) -> void:
	for node: MultiMeshInstance3D in _deco[layer][square]:
		deco_instances -= node.multimesh.instance_count
		node.queue_free()
	_deco[layer].erase(square)
	deco_chunks -= 1


## A MultiMeshInstance3D drawing `mesh` at every transform of `buffer`
## (12 floats per instance), each multiplied by `local` (the part's place in
## its model).
func _multimesh(mesh: Mesh, buffer: PackedFloat32Array, local: Transform3D) -> MultiMeshInstance3D:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.mesh = mesh
	multimesh.instance_count = buffer.size() / 12
	if local.is_equal_approx(Transform3D.IDENTITY):
		multimesh.buffer = buffer
	else:
		for i in multimesh.instance_count:
			var o := i * 12
			var t := Transform3D(
				Vector3(buffer[o], buffer[o + 4], buffer[o + 8]),
				Vector3(buffer[o + 1], buffer[o + 5], buffer[o + 9]),
				Vector3(buffer[o + 2], buffer[o + 6], buffer[o + 10]),
				Vector3(buffer[o + 3], buffer[o + 7], buffer[o + 11]),
			)
			multimesh.set_instance_transform(i, t * local)
	var node := MultiMeshInstance3D.new()
	node.multimesh = multimesh
	_apply_look(node)
	return node


## Biome look as instance uniforms (EnvLook.apply does the same for single
## meshes); `tint` scales the foliage color (grass clumps match the lighter
## ground texture). Every node with instance uniforms takes a few slots of
## the global shader uniform buffer: project.godot raises
## `rendering/limits/global_shader_variables/buffer_size` for the ~10 000
## nodes of a 4 km map.
func _apply_look(node: GeometryInstance3D, tint := Color.WHITE) -> void:
	var look: Dictionary = EnvLook.LOOK.get(biome, EnvLook.LOOK["forest"])
	node.set_instance_shader_parameter(&"env_foliage", look.foliage * tint)
	node.set_instance_shader_parameter(&"env_dry", look.dry)
	node.set_instance_shader_parameter(&"env_snow", look.snow)
	node.set_instance_shader_parameter(&"env_dust", look.dust)
	node.set_instance_shader_parameter(&"env_paint", EnvLook.NEUTRAL_PAINT)


## The mesh parts of one model variant: [{name, mesh, transform, lod}] with
## lod "near" / "far" / "shadow" (far mesh casting the near shadow) for trees
## with a far mesh, "" otherwise.
func _parts(id: String, variant: String) -> Array:
	var model := _model(id)
	var parts: Array = []
	if model == null:
		push_warning("map objects: no model for %s" % id)
		return parts
	var node := EnvLook.variant_node(model, variant)
	if node == null:
		push_warning("map objects: %s has no variant %s" % [id, variant])
		return parts
	var far := EnvLook.variant_node(model, variant + "_Far") as MeshInstance3D
	# Bridges: the instance stands at the deck height, the model's deck lies
	# `deck_height_m` above its base.
	var lower := Transform3D.IDENTITY
	if id.begins_with("env_bridge"):
		lower = Transform3D(Basis.IDENTITY, Vector3(0.0, -_deck_height(node), 0.0))
	var meshes: Array[MeshInstance3D] = []
	if node is MeshInstance3D:
		meshes.append(node)
	for child in node.find_children("*", "MeshInstance3D", true, false):
		meshes.append(child)
	for mesh in meshes:
		parts.append({
			"name": mesh.name, "mesh": mesh.mesh, "transform": lower * _relative(mesh, model),
			"lod": "near" if far else "",
		})
	if far:
		var t := _relative(far, model)
		parts.append({"name": far.name, "mesh": far.mesh, "transform": t, "lod": "far"})
		parts.append({"name": far.name + "Shadow", "mesh": far.mesh, "transform": t, "lod": "shadow"})
	return parts


## Deck height of a bridge variant (glTF extra `deck_height_m` on the
## variant node or one of its parts), 0 if missing.
static func _deck_height(node: Node) -> float:
	var stack: Array[Node] = [node]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		if n.has_meta(&"extras"):
			var extras: Dictionary = n.get_meta(&"extras")
			if extras.has("deck_height_m"):
				return float(extras.deck_height_m)
		stack.append_array(n.get_children())
	push_warning("map objects: bridge %s has no deck_height_m" % node.name)
	return 0.0


## Transform of `node` relative to `root` (the parent chain, root excluded).
func _relative(node: Node3D, root: Node) -> Transform3D:
	var t := Transform3D.IDENTITY
	var n: Node = node
	while n != null and n != root:
		if n is Node3D:
			t = (n as Node3D).transform * t
		n = n.get_parent()
	return t


func _model(id: String) -> Node3D:
	if not _models.has(id):
		var path := MODELS_DIR + id + ".glb"
		var model: Node3D = null
		if ResourceLoader.exists(path):
			model = (load(path) as PackedScene).instantiate() as Node3D
		_models[id] = model
	return _models[id]


func _class_of(id: String) -> Dictionary:
	for cls in OBJECT_CLASSES:
		if id.begins_with(cls.prefix):
			return cls
	return OBJECT_CLASSES[-1]


func _exit_tree() -> void:
	for model: Node3D in _models.values():
		if model:
			model.free()
	_models.clear()
