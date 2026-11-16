extends Node3D
## Base buildings, construction sites, rally points and supply points
## (prompt 023). Presentation only: everything comes from `SimHost`.
##
## Buildings are units of the simulation, so `main.gd` creates their nodes
## like any unit; `attach_model` gives the `base_*` types a borrowed house
## model of prompt 007 (with the damage variants of prompt 020). Here the
## model grows with the construction progress and switches to its
## "Damaged" variant at half its hit points and to "Ruin" when destroyed.
## Supply points are flag poles in the colour of their holder with a ring
## that fills while they are taken. Created by `main.gd`.

## Borrowed models of the base buildings (sizes match data/economy.toml).
const MODELS := {
	"base_hq": "env_house_stone",
	"base_barracks": "env_house_farm_brick",
	"base_depot": "env_house_farm",
	"base_factory": "env_barn",
	# Prompt 054: the ammunition dump of prompt 041; `strike_view.gd` adds
	# the launch frames. [provisional]
	"base_rocket_battery": "env_dump_ammo",
}
const MODELS_DIR := "res://assets/models/neutral/"
const StrikeView := preload("res://scenes/world/strike_view.gd")
## A site this far built shows at least this much of its model's height.
const MIN_SITE_SCALE := 0.12
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const OWN_COLOR := Color(0.35, 0.6, 0.95)
const ENEMY_COLOR := Color(0.85, 0.25, 0.2)
## Supply points of allies (prompt 037) [provisional].
const ALLY_COLOR := Color(0.4, 0.85, 0.85)
const NEUTRAL_COLOR := Color(0.85, 0.85, 0.8)
## Structures and supply points are read this often (s).
const READ_INTERVAL := 0.1

var main: Node3D
var sim_host: SimHost
## Last `SimHost.structures()`, by unit id.
var structures: Dictionary[int, Dictionary] = {}
var _supply_nodes: Array[Node3D] = []
var _rally_marker: MeshInstance3D
var _read_left := 0.0


## Whether `type_id` is a base building with a borrowed model.
static func is_building(type_id: String) -> bool:
	return MODELS.has(type_id)


## Puts the borrowed model of a base building into the unit node `node`
## (replacing the placeholder), turned so its long side lies along the
## building's length in the simulation (along the hull direction).
static func attach_model(node: Node3D, type_id: String, biome: String) -> void:
	var path: String = MODELS_DIR + MODELS[type_id] + ".glb"
	if not ResourceLoader.exists(path):
		return
	var placeholder := node.get_node_or_null("Model")
	if placeholder != null:
		node.remove_child(placeholder)
		placeholder.free()
	var model := (load(path) as PackedScene).instantiate() as Node3D
	model.name = "Model"
	EnvLook.apply(model, biome)
	EnvLook.show_variant(model, "Intact")
	var holder := Node3D.new()
	holder.name = "Model"
	model.name = "House"
	holder.add_child(model)
	# Buildings in the simulation have their length along the hull
	# direction (-z); turn models whose long side lies along x.
	var box := _intact_box(model)
	if box.size.x > box.size.z:
		model.rotation.y = PI * 0.5
	if type_id == "base_rocket_battery":
		holder.add_child(StrikeView.launchers())
	node.add_child(holder)
	holder.set_meta(&"stage", "Intact")


## Bounds of the "Intact" variant in model space.
static func _intact_box(model: Node3D) -> AABB:
	var variant := EnvLook.variant_node(model, "Intact")
	var box := AABB()
	var first := true
	var stack: Array[Node] = [variant if variant != null else model]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		stack.append_array(n.get_children())
		var mesh := n as MeshInstance3D
		if mesh == null or mesh.mesh == null:
			continue
		var b := mesh.transform * mesh.mesh.get_aabb()
		box = b if first else box.merge(b)
		first = false
	return box


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_rally_marker = MeshInstance3D.new()
	var pole := CylinderMesh.new()
	pole.top_radius = 0.15
	pole.bottom_radius = 0.15
	pole.height = 4.0
	_rally_marker.mesh = pole
	_rally_marker.material_override = _flat(RelationColors.own_or(OWN_COLOR))
	_rally_marker.visible = false
	add_child(_rally_marker)
	for point: Dictionary in sim_host.supply_points():
		var flag := _make_flag()
		flag.position = point.position
		add_child(flag)
		_supply_nodes.append(flag)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_read_left -= delta
	if _read_left <= 0.0:
		_read_left = READ_INTERVAL
		structures.clear()
		for s: Dictionary in sim_host.structures():
			structures[s.id] = s
		_update_supply()
	_update_buildings()
	_update_rally()


## Ids of the own structures among `ids`.
func own_structures(ids: Array) -> Array[int]:
	var me := sim_host.local_player()
	var out: Array[int] = []
	for id: int in ids:
		if structures.has(id) and int(structures[id].owner) == me:
			out.append(id)
	return out


func _update_buildings() -> void:
	var ids := sim_host.unit_ids()
	var health := sim_host.unit_health()
	for i in ids.size():
		var s: Dictionary = structures.get(ids[i], {})
		var node: Node3D = main._unit_nodes.get(ids[i])
		if node == null:
			continue
		var holder := node.get_node_or_null("Model") as Node3D
		if holder == null or not holder.has_meta(&"stage"):
			continue
		var built: float = s.get("built", 1.0)
		holder.scale = Vector3(1.0, maxf(MIN_SITE_SCALE, built), 1.0)
		var stage := "Intact"
		if health[i] <= 0.0:
			stage = "Ruin"
			holder.scale = Vector3.ONE
		elif health[i] < 0.5 and built >= 1.0:
			stage = "Damaged"
		if holder.get_meta(&"stage") != stage:
			holder.set_meta(&"stage", stage)
			var house := holder.get_node("House")
			# Models without damage variants (the dump: Intact, Burnt).
			if EnvLook.variant_node(house, stage) == null:
				stage = "Burnt" if stage == "Ruin" and EnvLook.variant_node(house, "Burnt") != null else "Intact"
			EnvLook.show_variant(house, stage)


## The rally point of the selected own producing building.
func _update_rally() -> void:
	var picked := own_structures(main.selection.selected)
	_rally_marker.visible = false
	if picked.size() != 1:
		return
	var s: Dictionary = structures.get(picked[0], {})
	var id: String = s.get("type_id", "")
	if id != "base_barracks" and id != "base_factory":
		return
	_rally_marker.visible = true
	_rally_marker.position = (s.rally as Vector3) + Vector3.UP * 2.0


func _side_color(player: int, me: int) -> Color:
	if player == me:
		return RelationColors.own_or(OWN_COLOR)
	return RelationColors.ally_or(ALLY_COLOR) if sim_host.relation(player) == 1 else RelationColors.enemy_or(ENEMY_COLOR)


func _update_supply() -> void:
	var points := sim_host.supply_points()
	var me := sim_host.local_player()
	for i in mini(points.size(), _supply_nodes.size()):
		var p: Dictionary = points[i]
		var flag := _supply_nodes[i]
		var owner: int = p.owner
		var color := NEUTRAL_COLOR if owner < 0 else _side_color(owner, me)
		(flag.get_node("Cloth") as MeshInstance3D).material_override = _flat(color)
		var ring := flag.get_node("Ring") as MeshInstance3D
		var taker: int = p.taker
		ring.visible = taker >= 0
		if taker >= 0:
			ring.material_override = _flat(_side_color(taker, me) * Color(1, 1, 1, 0.6))
			var r := 2.0 + 23.0 * float(p.progress)
			ring.scale = Vector3(r, 1.0, r)


func _make_flag() -> Node3D:
	var flag := Node3D.new()
	var pole := MeshInstance3D.new()
	var pole_mesh := CylinderMesh.new()
	pole_mesh.top_radius = 0.08
	pole_mesh.bottom_radius = 0.1
	pole_mesh.height = 7.0
	pole.mesh = pole_mesh
	pole.position.y = 3.5
	pole.material_override = _flat(Color(0.35, 0.3, 0.25))
	flag.add_child(pole)
	var cloth := MeshInstance3D.new()
	cloth.name = "Cloth"
	var cloth_mesh := BoxMesh.new()
	cloth_mesh.size = Vector3(2.4, 1.5, 0.05)
	cloth.mesh = cloth_mesh
	cloth.position = Vector3(1.25, 6.1, 0.0)
	cloth.material_override = _flat(NEUTRAL_COLOR)
	flag.add_child(cloth)
	var ring := MeshInstance3D.new()
	ring.name = "Ring"
	var ring_mesh := TorusMesh.new()
	ring_mesh.inner_radius = 0.92
	ring_mesh.outer_radius = 1.0
	ring.mesh = ring_mesh
	ring.position.y = 0.3
	ring.visible = false
	flag.add_child(ring)
	# The area that must be held (radius of data/economy.toml).
	var area := MeshInstance3D.new()
	var area_mesh := TorusMesh.new()
	area_mesh.inner_radius = 24.6
	area_mesh.outer_radius = 25.0
	area.mesh = area_mesh
	area.position.y = 0.25
	area.material_override = _flat(Color(1, 1, 1, 0.25))
	flag.add_child(area)
	return flag


static func _flat(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	if color.a < 1.0:
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	return m
