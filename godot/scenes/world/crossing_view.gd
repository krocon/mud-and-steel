extends Node3D
## Rivers, bridges, bridge-laying tanks, camouflage and paratroopers
## (prompt 055). Presentation only: the simulation decides when a deck
## collapses, who falls, who swims or drowns, where a bridge is laid, who is
## camouflaged and where paratroopers land; this script
##
## - breaks a collapsing deck into pieces that tumble into the river, with
##   splashes where they and the units on it hit the water;
## - folds the carried bridge of a bridge-laying tank open while it lays it
##   (and shut while it takes it up), hides it while it lies across the gap
##   and draws the laid bridge as a treadway deck between its ends;
## - puts branches (forest and winter) or a net (desert) on camouflaged
##   vehicles and guns, sparse while they are put up [provisional];
## - draws paratroopers under their canopies;
## - tells `infantry_view.gd` which squads swim (their soldiers float at the
##   water surface).
##
## Created by `main.gd`.

const REFRESH := 0.2
const GRAVITY := 9.81
## Pieces of a collapsing deck at most, and how long they sink.
const MAX_PIECES := 12
const SINK_SECONDS := 5.0
## Soldiers swim with their head and shoulders out of the water.
const SWIM_DEPTH_M := 1.3
const BUSH_PATH := "res://assets/models/neutral/env_bush.glb"
## Laid treadway bridge: track width and gap, girder depth (m).
const TRACK_W := 1.1
const TRACK_GAP := 0.7
const DECK_THICK := 0.35
const STEEL := Color(0.24, 0.27, 0.2)
const WOOD := Color(0.42, 0.3, 0.19)
const STONE := Color(0.55, 0.53, 0.5)
## Canopies drawn per stick at most.
const MAX_CANOPIES := 8

var main: Node3D
var sim_host: SimHost
## Counters for the self-test.
var collapses := 0
var pieces_spawned := 0
var splashes := 0
var laid_drawn := 0
var camo_drawn := 0
var chutes_drawn := 0

var _refresh_left := 0.0
var _decks: Array = []
var _deck_count := -1
var _pieces: Array[Dictionary] = []
var _laid: Dictionary[int, Node3D] = {}
var _camo: Dictionary[int, Node3D] = {}
var _camo_state: Dictionary[int, int] = {}
var _chutes: Dictionary[int, Node3D] = {}
var _chute_targets: Dictionary[int, Vector3] = {}
var _swimming: Dictionary[int, bool] = {}
var _bush: PackedScene
var _materials: Dictionary[String, StandardMaterial3D] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	if ResourceLoader.exists(BUSH_PATH):
		_bush = load(BUSH_PATH) as PackedScene


func _process(delta: float) -> void:
	if sim_host == null:
		return
	for event: Dictionary in sim_host.take_crossing_events():
		_on_event(event)
	_update_pieces(delta)
	_update_chutes(delta)
	_refresh_left -= delta
	if _refresh_left > 0.0:
		return
	_refresh_left = REFRESH
	refresh()


## Brings bridges, camouflage, parachutes and swimmers up to date (also
## called by the self-test).
func refresh() -> void:
	_swimming.clear()
	for id in sim_host.swimming_units():
		_swimming[id] = true
	_update_layers()
	_update_camouflage()
	_refresh_chutes()


## Whether the squad `unit_id` swims (read by `infantry_view.gd`).
func is_swimming(unit_id: int) -> bool:
	return _swimming.has(unit_id)


## Height of a swimming soldier at `position`: at the water surface.
func swim_height(position: Vector3) -> float:
	var level := sim_host.water_level_at(position.x, position.z)
	if is_nan(level):
		return position.y
	return maxf(position.y, level - SWIM_DEPTH_M)


func _on_event(event: Dictionary) -> void:
	match str(event.get("type", "")):
		"collapsed":
			_collapse(int(event["bridge"]), event["at"])
		"fell", "drowned":
			_splash(event["at"], 120.0 if event["type"] == "fell" else 40.0)


func _splash(at: Vector3, caliber: float) -> void:
	var effects: Node = main.get("effects")
	if effects == null:
		return
	var level := sim_host.water_level_at(at.x, at.z)
	var p := Vector3(at.x, at.y if is_nan(level) else level, at.z)
	effects.ground_hit(p, caliber)
	splashes += 1


func _material(key: String, color: Color) -> StandardMaterial3D:
	if not _materials.has(key):
		var m := StandardMaterial3D.new()
		m.albedo_color = color
		m.roughness = 0.9
		_materials[key] = m
	return _materials[key]


func _bridge_decks() -> Array:
	var count := sim_host.bridges_intact().size()
	if count != _deck_count:
		_deck_count = count
		_decks = sim_host.bridge_decks()
	return _decks


# --- Collapsing decks -----------------------------------------------------------

## Deck `bridge` broke: its pieces tumble into the river.
func _collapse(bridge: int, at: Vector3) -> void:
	collapses += 1
	var decks := _bridge_decks()
	if bridge < 0 or bridge >= decks.size():
		return
	var deck: Dictionary = decks[bridge]
	var start: Vector3 = deck["start"]
	var end: Vector3 = deck["end"]
	var width: float = deck["width_m"]
	var laid: bool = int(deck["unit"]) >= 0
	var stone := not laid and width >= 5.0
	var color := STEEL if laid else (STONE if stone else WOOD)
	var thick := DECK_THICK if laid else (1.2 if stone else 0.45)
	var length := start.distance_to(end)
	var count := clampi(int(length / 2.5), 4, MAX_PIECES)
	var axis := (end - start).normalized()
	var yaw := atan2(-axis.x, -axis.z)
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(bridge * 7919 + int(at.x))
	var seg := length / count
	for k in count:
		var t := (k + 0.5) / count
		var center := start.lerp(end, t)
		var piece := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(width * rng.randf_range(0.6, 1.0), thick, seg * rng.randf_range(0.7, 0.95))
		box.material = _material("deck_%s" % color.to_html(), color)
		piece.mesh = box
		add_child(piece)
		piece.global_transform = Transform3D(Basis(Vector3.UP, yaw), center)
		var level := sim_host.water_level_at(center.x, center.z)
		var floor_y := sim_host.ground_height_at(center.x, center.z) if is_nan(level) else level
		# The middle drops first and fastest (the charge was there).
		var middle := 1.0 - absf(t - 0.5) * 2.0
		_pieces.append({
			"node": piece,
			"velocity": Vector3(rng.randf_range(-1.5, 1.5), rng.randf_range(0.5, 3.0) * middle, rng.randf_range(-1.5, 1.5)),
			"spin": Vector3(rng.randf_range(-1.5, 1.5), rng.randf_range(-0.6, 0.6), rng.randf_range(-1.5, 1.5)),
			"delay": (1.0 - middle) * 0.6 + rng.randf_range(0.0, 0.2),
			"floor": floor_y,
			"sink": -1.0,
		})
		pieces_spawned += 1


func _update_pieces(delta: float) -> void:
	for i in range(_pieces.size() - 1, -1, -1):
		var p: Dictionary = _pieces[i]
		var node: MeshInstance3D = p["node"]
		if not is_instance_valid(node):
			_pieces.remove_at(i)
			continue
		if float(p["delay"]) > 0.0:
			p["delay"] = float(p["delay"]) - delta
			continue
		var sink: float = p["sink"]
		if sink < 0.0:
			var v: Vector3 = p["velocity"]
			v.y -= GRAVITY * delta
			p["velocity"] = v
			node.global_position += v * delta
			var spin: Vector3 = p["spin"]
			node.rotate_object_local(Vector3.RIGHT, spin.x * delta)
			node.rotate_object_local(Vector3.FORWARD, spin.z * delta)
			if node.global_position.y <= float(p["floor"]):
				p["sink"] = 0.0
				_splash(node.global_position, 150.0)
		else:
			sink += delta
			p["sink"] = sink
			node.global_position.y -= 0.3 * delta
			if sink >= SINK_SECONDS:
				node.queue_free()
				_pieces.remove_at(i)


# --- Bridge-laying tanks and laid bridges -------------------------------------------

func _update_layers() -> void:
	var shown := {}
	for layer: Dictionary in sim_host.bridge_layers():
		var unit := int(layer["unit"])
		var state: String = layer["state"]
		var node: Node3D = main._unit_nodes.get(unit)
		var carried: Node3D = node.find_child("Bridge", true, false) as Node3D if node != null else null
		if carried != null:
			if not carried.has_meta(&"rest"):
				carried.set_meta(&"rest", carried.transform)
			var rest: Transform3D = carried.get_meta(&"rest")
			match state:
				"laid", "fetching", "lost":
					carried.visible = false
				"laying", "picking_up":
					carried.visible = true
					# The bridge swings forward over the bow while it opens.
					var p: float = layer["progress"]
					var pivot := Vector3(0.0, rest.origin.y, -2.0)
					var swing := Transform3D(Basis(Vector3.RIGHT, -p * PI * 0.5), Vector3.ZERO)
					carried.transform = Transform3D.IDENTITY.translated(pivot) * swing \
							* Transform3D.IDENTITY.translated(-pivot) * rest
				_:
					carried.visible = true
					carried.transform = rest
	for deck: Dictionary in _bridge_decks_now():
		var bridge := int(deck["bridge"])
		if int(deck["unit"]) < 0 or not bool(deck["intact"]):
			continue
		shown[bridge] = true
		var key := "%s|%s" % [deck["start"], deck["end"]]
		var node: Node3D = _laid.get(bridge)
		if node != null and node.get_meta(&"key", "") != key:
			node.queue_free()
			node = null
		if node == null:
			node = _make_laid(deck)
			node.set_meta(&"key", key)
			add_child(node)
			_laid[bridge] = node
	for bridge: int in _laid.keys():
		if not shown.has(bridge):
			if is_instance_valid(_laid[bridge]):
				_laid[bridge].queue_free()
			_laid.erase(bridge)
	laid_drawn = _laid.size()


## Every bridge with its deck (refetched while laid bridges change).
func _bridge_decks_now() -> Array:
	_decks = sim_host.bridge_decks()
	_deck_count = _decks.size()
	return _decks


## A treadway deck from the laid bridge's start to its end: two tracks on
## girders, ramps at both ends.
func _make_laid(deck: Dictionary) -> Node3D:
	var start: Vector3 = deck["start"]
	var end: Vector3 = deck["end"]
	var root := Node3D.new()
	root.name = "LaidBridge%d" % int(deck["bridge"])
	var length := start.distance_to(end)
	var axis := (end - start).normalized()
	var flat := Vector3(axis.x, 0.0, axis.z).normalized()
	var yaw := atan2(-flat.x, -flat.z)
	var pitch := asin(clampf(axis.y, -1.0, 1.0))
	root.transform = Transform3D(Basis(Vector3.UP, yaw) * Basis(Vector3.RIGHT, pitch), start.lerp(end, 0.5))
	var mat := _material("treadway", STEEL)
	for side in [-1.0, 1.0]:
		var track := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(TRACK_W, DECK_THICK, length)
		box.material = mat
		track.mesh = box
		track.position = Vector3(side * (TRACK_GAP + TRACK_W) * 0.5, DECK_THICK * 0.5 + 0.05, 0.0)
		root.add_child(track)
	var girders := maxi(2, int(length / 3.0))
	for k in girders + 1:
		var g := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(2.0 * TRACK_W + TRACK_GAP, 0.25, 0.25)
		box.material = mat
		g.mesh = box
		g.position = Vector3(0.0, -0.05, -length * 0.5 + length * k / girders)
		root.add_child(g)
	return root


# --- Camouflage --------------------------------------------------------------------

func _update_camouflage() -> void:
	var ids := sim_host.camouflaged_units()
	var wanted := {}
	var biome: String = main.atmosphere.preset.camo_biome if main.get("atmosphere") != null else "forest"
	for id in ids:
		wanted[id] = true
		var state := sim_host.camouflage_state(id)
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var camo: Node3D = _camo.get(id)
		var key := state if state < 2 else 2
		if camo != null and (not is_instance_valid(camo) or _camo_state.get(id, -1) != key):
			if is_instance_valid(camo):
				camo.queue_free()
			camo = null
		if camo == null:
			camo = _make_camo(node, id, biome, key == 1)
			if camo == null:
				continue
			_camo[id] = camo
			_camo_state[id] = key
	for id: int in _camo.keys():
		if not wanted.has(id):
			if is_instance_valid(_camo[id]):
				_camo[id].queue_free()
			_camo.erase(id)
			_camo_state.erase(id)
	camo_drawn = _camo.size()


## Bounds of a unit's model in the unit node's space.
func _model_bounds(node: Node3D) -> AABB:
	var model := node.get_node_or_null("Model") as Node3D
	var out := AABB()
	var first := true
	var stack: Array[Node] = [model if model != null else node]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		stack.append_array(n.get_children())
		var mesh := n as MeshInstance3D
		if mesh == null or mesh.mesh == null or mesh.name == "SelectionRing":
			continue
		var xform := Transform3D.IDENTITY
		var p: Node = mesh
		while p != null and p != node:
			if p is Node3D:
				xform = (p as Node3D).transform * xform
			p = p.get_parent()
		var box := xform * mesh.get_aabb()
		out = box if first else out.merge(box)
		first = false
	if first:
		return AABB(Vector3(-1.5, 0.0, -3.0), Vector3(3.0, 2.5, 6.0))
	return out


## Branches (forest, winter) or a net (desert) over the hull; `sparse` while
## the crew is still putting them up.
func _make_camo(node: Node3D, id: int, biome: String, sparse: bool) -> Node3D:
	var bounds := _model_bounds(node)
	var root := Node3D.new()
	root.name = "Camouflage"
	node.add_child(root)
	if biome == "desert":
		_add_net(root, bounds, Color(0.72, 0.62, 0.42), sparse)
		return root
	if _bush == null:
		_add_net(root, bounds, Color(0.28, 0.36, 0.2), sparse)
		return root
	var rng := RandomNumberGenerator.new()
	rng.seed = id * 104729 + 17
	var count := 4 if sparse else 9
	var top := bounds.position.y + bounds.size.y * 0.75
	for k in count:
		var bush := _bush.instantiate() as Node3D
		var x := bounds.position.x + bounds.size.x * rng.randf_range(0.1, 0.9)
		var z := bounds.position.z + bounds.size.z * rng.randf_range(0.05, 0.95)
		var y := top if k % 3 != 0 else bounds.position.y + bounds.size.y * 0.35
		var s := rng.randf_range(0.45, 0.7)
		bush.transform = Transform3D(Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * 0.8, s)), Vector3(x, y - 0.3, z))
		root.add_child(bush)
	if biome == "winter":
		# Whitewashed nets with a few branches.
		_add_net(root, bounds, Color(0.86, 0.87, 0.86), true)
	return root


## A net draped over `bounds`: a flat top and four sloping sides.
func _add_net(root: Node3D, bounds: AABB, color: Color, sparse: bool) -> void:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	mat.alpha_scissor_threshold = 0.5
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	var noise := FastNoiseLite.new()
	noise.frequency = 0.25
	var tex := NoiseTexture2D.new()
	tex.width = 64
	tex.height = 64
	tex.noise = noise
	mat.albedo_texture = tex
	mat.uv1_scale = Vector3(3, 3, 1)
	var top := bounds.position.y + bounds.size.y + 0.15
	var half := bounds.size * 0.5 + Vector3(0.35, 0.0, 0.35)
	var c := bounds.get_center()
	var corners := [
		Vector3(c.x - half.x, top, c.z - half.z), Vector3(c.x + half.x, top, c.z - half.z),
		Vector3(c.x + half.x, top, c.z + half.z), Vector3(c.x - half.x, top, c.z + half.z),
	]
	var spread := 0.9 if not sparse else 0.3
	var foot := bounds.position.y + 0.1
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var quad := func(a: Vector3, b: Vector3, d: Vector3, e: Vector3) -> void:
		st.set_uv(Vector2(0, 0)); st.add_vertex(a)
		st.set_uv(Vector2(1, 0)); st.add_vertex(b)
		st.set_uv(Vector2(1, 1)); st.add_vertex(d)
		st.set_uv(Vector2(0, 0)); st.add_vertex(a)
		st.set_uv(Vector2(1, 1)); st.add_vertex(d)
		st.set_uv(Vector2(0, 1)); st.add_vertex(e)
	quad.call(corners[0], corners[1], corners[2], corners[3])
	for k in 4:
		var a: Vector3 = corners[k]
		var b: Vector3 = corners[(k + 1) % 4]
		var out_a := Vector3(a.x + signf(a.x - c.x) * spread, foot, a.z + signf(a.z - c.z) * spread)
		var out_b := Vector3(b.x + signf(b.x - c.x) * spread, foot, b.z + signf(b.z - c.z) * spread)
		quad.call(a, b, out_b, out_a)
	st.generate_normals()
	var net := MeshInstance3D.new()
	net.name = "Net"
	net.mesh = st.commit()
	net.material_override = mat
	root.add_child(net)


# --- Paratroopers --------------------------------------------------------------------

func _refresh_chutes() -> void:
	var seen := {}
	for p: Dictionary in sim_host.parachutes():
		var id := int(p["id"])
		seen[id] = true
		var position: Vector3 = p["position"]
		_chute_targets[id] = position
		if not _chutes.has(id):
			var node := _make_stick(int(p["soldiers"]), int(p["owner"]))
			node.position = position
			add_child(node)
			_chutes[id] = node
	for id: int in _chutes.keys():
		if not seen.has(id):
			if is_instance_valid(_chutes[id]):
				_chutes[id].queue_free()
			_chutes.erase(id)
			_chute_targets.erase(id)
	chutes_drawn = _chutes.size()


func _update_chutes(delta: float) -> void:
	for id: int in _chutes:
		var node := _chutes[id]
		if is_instance_valid(node) and _chute_targets.has(id):
			node.position = node.position.lerp(_chute_targets[id], clampf(delta * 5.0, 0.0, 1.0))


## A stick of `soldiers` paratroopers: a canopy and a jumper each.
func _make_stick(soldiers: int, owner: int) -> Node3D:
	var root := Node3D.new()
	root.name = "Stick"
	var own := owner == sim_host.local_player()
	var canopy_mat := _material("canopy_own" if own else "canopy", Color(0.85, 0.84, 0.76) if own else Color(0.62, 0.6, 0.5))
	var man_mat := _material("jumper", Color(0.3, 0.3, 0.24))
	var n := clampi(soldiers, 1, MAX_CANOPIES)
	for k in n:
		var angle := TAU * k / n
		var ring := 3.0 + 2.0 * (k % 2)
		var at := Vector3(cos(angle) * ring, 0.0, sin(angle) * ring)
		var canopy := MeshInstance3D.new()
		var dome := SphereMesh.new()
		dome.radius = 3.0
		dome.height = 2.2
		dome.is_hemisphere = true
		dome.radial_segments = 12
		dome.rings = 4
		dome.material = canopy_mat
		canopy.mesh = dome
		canopy.position = at + Vector3(0.0, 6.0, 0.0)
		root.add_child(canopy)
		var man := MeshInstance3D.new()
		var body := CapsuleMesh.new()
		body.radius = 0.25
		body.height = 1.7
		body.material = man_mat
		man.mesh = body
		man.position = at
		root.add_child(man)
	return root
