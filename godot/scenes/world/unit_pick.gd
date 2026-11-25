extends RefCounted
## Which unit's model is under a screen point (prompt 167): the hover of
## `unit_highlight.gd` and the clicks of `selection.gd` use this one rule.
## Presentation only, no physics: models are projected with the camera.
##
## - **Vehicles, guns, buildings:** the box around the unit's meshes (in the
##   unit's frame, measured once) projected to the screen; the point is on
##   the model when it is inside that outline or at most `PICK_PX` from it.
## - **Squads:** every drawn soldier as an upright segment from his feet to
##   his head (lying: a flat segment along his body), as wide as a man.
##
## When several models contain the point, the one nearest to the camera wins.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")

## Tolerance around a model in screen pixels (scaled with the HUD).
const PICK_PX := 5.0
## A soldier's height by stance (stand, kneel, prone) and half his width,
## in meters; a lying soldier's half length.
const SOLDIER_HEIGHT: Array[float] = [1.75, 1.15, 0.45]
const SOLDIER_HALF_WIDTH := 0.3
const LYING_HALF_LENGTH := 0.9
## Radius around a squad's position its soldiers stand in (meters), for
## the quick test before the soldiers are looked at.
const SQUAD_RADIUS_M := 30.0

var main: Node3D
## Per unit id: the box around its meshes in the unit's frame.
var _boxes: Dictionary[int, AABB] = {}
var _pick_calls := 0


func _init(main_node: Node3D) -> void:
	main = main_node


## Removes cached boxes for units no longer in the scene (prompt 206).
func prune_caches() -> void:
	if main == null:
		return
	var unit_nodes: Dictionary = main._unit_nodes
	for id: int in _boxes.keys():
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node):
			_boxes.erase(id)


## Id of the living unit whose model is under `screen`, or -1. `kind`:
## "own" (selectable own units: no passengers riding in a vehicle),
## "enemy" (units of enemies), "any" (every drawn unit but passengers).
func unit_at(screen: Vector2, kind := "any") -> int:
	_pick_calls += 1
	if _pick_calls % 240 == 0:
		prune_caches()
	var sim_host: SimHost = main.sim_host
	var camera: Camera3D = main.camera
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var relations := sim_host.unit_relations()
	var carriers := sim_host.unit_carriers()
	var stances := sim_host.unit_stances()
	var me := sim_host.local_player()
	var tolerance := HudTheme.px(PICK_PX)
	var camera_position := camera.global_position
	var right := camera.global_basis.x
	var view: Node = main.get("infantry_view")
	var best := -1
	var best_depth := INF
	for i in ids.size():
		if alive[i] == 0 or (carriers[i] >= 0 and stances[i] >= 0):
			continue
		if kind == "own" and owners[i] != me:
			continue
		if kind == "enemy" and relations[i] != 2:
			continue
		var node: Node3D = main._unit_nodes.get(ids[i])
		if node == null or not node.visible:
			continue
		var center := node.global_position
		var depth := camera_position.distance_to(center)
		if depth >= best_depth or camera.is_position_behind(center):
			continue
		var squad := stances[i] >= 0
		var box := AABB() if squad else _box(ids[i], node)
		# Quick test: the point is near the unit's spot at all.
		var reach := SQUAD_RADIUS_M if squad else box.get_longest_axis_size()
		var at := camera.unproject_position(center)
		var reach_px := at.distance_to(camera.unproject_position(center + right * reach))
		if at.distance_to(screen) > reach_px + tolerance:
			continue
		var hit := _squad_hit(ids[i], screen, tolerance, stances[i], camera, view) if squad \
				else _box_hit(node, box, screen, tolerance, camera)
		if hit:
			best = ids[i]
			best_depth = depth
	return best


## The box around a unit's meshes in its own frame (measured once: a
## turning turret changes it a little).
func _box(id: int, node: Node3D) -> AABB:
	if _boxes.has(id):
		return _boxes[id]
	var box := AABB()
	var first := true
	var to_local := node.global_transform.affine_inverse()
	for mesh: Node in node.find_children("*", "MeshInstance3D", true, false):
		var instance := mesh as MeshInstance3D
		if instance.name == "SelectionRing" or instance.mesh == null:
			continue
		var part := (to_local * instance.global_transform) * instance.get_aabb()
		box = part if first else box.merge(part)
		first = false
	if first:
		box = AABB(Vector3(-1.5, 0.0, -2.5), Vector3(3.0, 2.0, 5.0))
	_boxes[id] = box
	return box


func _box_hit(node: Node3D, box: AABB, screen: Vector2, tolerance: float, camera: Camera3D) -> bool:
	var points := PackedVector2Array()
	var xform := node.global_transform
	for k in 8:
		var corner := xform * box.get_endpoint(k)
		if camera.is_position_behind(corner):
			return false
		points.append(camera.unproject_position(corner))
	var hull := Geometry2D.convex_hull(points)
	if Geometry2D.is_point_in_polygon(screen, hull):
		return true
	for k in hull.size() - 1:
		var closest := Geometry2D.get_closest_point_to_segment(screen, hull[k], hull[k + 1])
		if closest.distance_to(screen) <= tolerance:
			return true
	return false


func _squad_hit(id: int, screen: Vector2, tolerance: float, stance: int, camera: Camera3D,
		view: Node) -> bool:
	if view == null:
		return false
	var right := camera.global_basis.x
	for key: int in (view._by_unit.get(id, []) as Array):
		var s: Object = view._soldiers.get(key)
		if s == null or s.dead or not (s.node.visible or s.impostor):
			continue
		var feet: Vector3 = s.position
		var a: Vector3
		var b: Vector3
		if stance == 2:
			var yaw: float = s.yaw
			var along := Vector3(-sin(yaw), 0.0, -cos(yaw)) * LYING_HALF_LENGTH
			a = feet + Vector3.UP * 0.25 - along
			b = feet + Vector3.UP * 0.25 + along
		else:
			a = feet
			b = feet + Vector3.UP * SOLDIER_HEIGHT[clampi(stance, 0, 2)]
		if camera.is_position_behind(a) or camera.is_position_behind(b):
			continue
		var sa := camera.unproject_position(a)
		var sb := camera.unproject_position(b)
		var half := sa.distance_to(camera.unproject_position(a + right * SOLDIER_HALF_WIDTH))
		var closest := Geometry2D.get_closest_point_to_segment(screen, sa, sb)
		if closest.distance_to(screen) <= half + tolerance:
			return true
	return false
