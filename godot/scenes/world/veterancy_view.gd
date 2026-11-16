extends Node3D
## Experience on the battlefield (prompt 050). Presentation only: the
## simulation counts the points, ranks and kills.
##
## - Kill rings: every enemy vehicle or gun a unit destroyed is a white ring
##   painted round its gun barrel (as crews did), from RING_START of the
##   barrel towards the muzzle, at most MAX_RINGS. The rings are children of
##   the model's `Gun` node, so they turn with the turret; the barrel axis
##   runs from the gun's origin to its `Socket_Muzzle`, the ring radius
##   follows the caliber of the main weapon.
## - Promotions of own units appear in the event feed.
##
## The soldiers' kit per rank: `infantry_view.gd` / `SoldierLook`; the rank
## insignia: `hud/rank_badge.gd` in the unit panel.

const REFRESH := 0.5
const MAX_RINGS := 12
## Share of the barrel (gun origin to muzzle) where the first ring sits.
const RING_START := 0.42
const RING_SPACING_M := 0.06
const RING_WIDTH_M := 0.03
## Outer barrel radius per caliber (wall about 0.35 calibers), plus paint.
const BARREL_RADIUS_PER_CALIBER := 0.85
const PAINT_M := 0.006

var main: Node3D
var sim_host: SimHost
## Rings shown per unit id (self-test).
var rings_shown: Dictionary[int, int] = {}
## Promotion messages posted so far (self-test).
var promotions_posted := 0

var _left := 0.0
var _ranks: Dictionary[int, int] = {}
var _rank_names := PackedStringArray()
var _material: StandardMaterial3D
var _meshes: Dictionary[int, CylinderMesh] = {}
var _calibers: Dictionary[int, float] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_material = StandardMaterial3D.new()
	_material.albedo_color = Color(0.86, 0.85, 0.8)
	_material.roughness = 0.9


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_left -= delta
	if _left > 0.0:
		return
	_left = REFRESH
	refresh()


## Updates rings and promotion messages now (also used by the self-test).
func refresh() -> void:
	var ids := sim_host.unit_ids()
	var ranks := sim_host.unit_ranks()
	var kills := sim_host.unit_vehicle_kills()
	if ranks.size() != ids.size() or kills.size() != ids.size():
		return
	var owners := sim_host.unit_owners()
	var me := sim_host.local_player()
	for i in ids.size():
		var id := ids[i]
		var before: int = _ranks.get(id, -1)
		_ranks[id] = ranks[i]
		if before >= 0 and ranks[i] > before and owners[i] == me:
			_post_promotion(id, i, ranks[i])
		var want := mini(kills[i], MAX_RINGS)
		if want > 0 or rings_shown.has(id):
			_show_rings(id, i, want)


func _post_promotion(id: int, index: int, rank: int) -> void:
	if _rank_names.is_empty():
		_rank_names = sim_host.rank_names()
	var hud: Node = main.get("hud")
	if hud == null or hud.get("event_feed") == null:
		return
	var rank_name := _rank_names[rank] if rank < _rank_names.size() else tr("HUD_RANK_N") % rank
	var unit_name := str(sim_host.unit_info(id).get("name", tr("HUD_FEED_UNIT")))
	var positions := sim_host.current_positions()
	var at := positions[index] if index < positions.size() else Vector3.ZERO
	hud.event_feed.post("ready", tr("HUD_FEED_PROMOTED").format({"unit": unit_name, "rank": rank_name}), at)
	promotions_posted += 1


func _show_rings(id: int, index: int, count: int) -> void:
	var node: Node3D = main._unit_nodes.get(id)
	if node == null:
		return
	var model := node.get_node_or_null("Model") as Node3D
	if model == null:
		return
	var gun := model.find_child("Gun", true, false) as Node3D
	if gun == null:
		return
	var rings := gun.get_node_or_null("KillRings") as Node3D
	if rings != null and rings.get_child_count() == count:
		rings_shown[id] = count
		return
	if rings == null:
		rings = Node3D.new()
		rings.name = "KillRings"
		gun.add_child(rings)
	for child in rings.get_children():
		rings.remove_child(child)
		child.queue_free()
	rings_shown[id] = count
	var muzzle := gun.find_child("Socket_Muzzle", true, false) as Node3D
	if muzzle == null:
		muzzle = model.find_child("Socket_Muzzle", true, false) as Node3D
	if muzzle == null or count <= 0:
		return
	var tip := gun.to_local(muzzle.global_position)
	var length := tip.length()
	if length < 0.3:
		return
	var axis := tip / length
	var gun_scale := gun.global_transform.basis.get_scale().x
	var radius := _barrel_radius(id, index) / maxf(gun_scale, 0.001)
	var mesh := _ring_mesh(radius)
	var turn := Basis(Quaternion(Vector3.UP, axis))
	for k in count:
		var along := length * RING_START + k * RING_SPACING_M / maxf(gun_scale, 0.001)
		if along > length * 0.95:
			break
		var ring := MeshInstance3D.new()
		ring.mesh = mesh
		ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		ring.transform = Transform3D(turn, axis * along)
		rings.add_child(ring)


## Outer radius of the main gun's barrel in meters, from its caliber.
func _barrel_radius(id: int, index: int) -> float:
	var types := sim_host.unit_types()
	var type_index: int = types[index] if index < types.size() else -1
	if not _calibers.has(type_index):
		var caliber := 75.0
		var weapons: Array = sim_host.unit_info(id).get("weapons", [])
		if not weapons.is_empty():
			caliber = float(weapons[0].get("caliber_mm", caliber))
		_calibers[type_index] = caliber
	return _calibers[type_index] / 1000.0 * BARREL_RADIUS_PER_CALIBER + PAINT_M


func _ring_mesh(radius: float) -> CylinderMesh:
	var key := roundi(radius * 1000.0)
	if not _meshes.has(key):
		var mesh := CylinderMesh.new()
		mesh.top_radius = radius
		mesh.bottom_radius = radius
		mesh.height = RING_WIDTH_M
		mesh.radial_segments = 14
		mesh.rings = 0
		mesh.cap_top = false
		mesh.cap_bottom = false
		mesh.material = _material
		_meshes[key] = mesh
	return _meshes[key]
