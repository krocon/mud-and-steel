extends Node3D
## Passengers and towed guns (prompt 034). Presentation only: the
## simulation decides who rides where and where a towed gun stands (behind
## its tractor's hook, catalog sizes 1:1); this script
##
## - hides the squad of a passenger (its soldiers are not in the snapshot
##   while aboard; their members would stay where they were last drawn),
## - seats one figure per soldier aboard on the vehicle: at the model's
##   `Socket_Passenger_NN` empties (trucks, halftracks), or on the rear deck
##   of models without them (tank riders: two rows over the back half of the
##   model's bounding box, [provisional] until the kit has rider sockets).
##
## Figures are the faction's rifleman model in the `kneel` pose, children of
## the vehicle's model, so they move and hide (fog of war) with it. Their
## animation player runs in manual mode and is posed once.

const MODELS_DIR := "res://assets/models/"
const POSE := "kneel"
## Time into the pose clip at which the figures freeze.
const POSE_SECONDS := 0.4
## Seats are rebuilt at most this often (seconds); passenger counts change
## seldom.
const REFRESH := 0.25

var main: Node3D
var sim_host: SimHost
## Squads hidden because they ride: unit id -> true.
var hidden_squads: Dictionary[int, bool] = {}
## Seated figures per vehicle id.
var figures: Dictionary[int, Array] = {}
## Seat transforms (model space) per vehicle id.
var _seats: Dictionary[int, Array] = {}
var _refresh_left := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host


func _process(delta: float) -> void:
	if sim_host == null or main == null:
		return
	_refresh_left -= delta
	if _refresh_left > 0.0:
		return
	_refresh_left = REFRESH
	refresh()


## Hides riding squads and seats the passengers (also called by the
## self-test right after a tick).
func refresh() -> void:
	var ids := sim_host.unit_ids()
	var carriers := sim_host.unit_carriers()
	var stances := sim_host.unit_stances()
	var passengers := sim_host.unit_passengers()
	var types := sim_host.unit_type_ids()
	var riding := {}
	var loaded := {}
	for i in ids.size():
		var id := ids[i]
		if carriers[i] >= 0 and stances[i] >= 0:
			riding[id] = true
		if passengers[i] > 0:
			loaded[id] = i
	# Passengers take no orders: out of the selection.
	var selection: Control = main.selection
	if selection != null and selection.selected.any(func(id: int) -> bool: return riding.has(id)):
		selection._set_selection(selection.selected.filter(func(id: int) -> bool: return not riding.has(id)))
	for id: int in riding:
		var node: Node3D = main._unit_nodes.get(id)
		if node != null and node.visible:
			node.visible = false
			hidden_squads[id] = true
	for id: int in hidden_squads.keys():
		if riding.has(id):
			continue
		hidden_squads.erase(id)
		var node: Node3D = main._unit_nodes.get(id)
		if node != null:
			node.visible = true
	for id: int in figures.keys():
		if not loaded.has(id):
			_clear(id)
	for id: int in loaded:
		var i: int = loaded[id]
		_seat(id, passengers[i], (types[i] as String).get_slice("_", 0))


## Number of seated figures on vehicle `id` (self-test).
func seated(id: int) -> int:
	return (figures.get(id, []) as Array).size()


func _clear(id: int) -> void:
	for figure: Node in figures.get(id, []):
		if is_instance_valid(figure):
			figure.queue_free()
	figures.erase(id)


func _seat(id: int, count: int, faction: String) -> void:
	var node: Node3D = main._unit_nodes.get(id)
	var model := node.get_node_or_null("Model") as Node3D if node != null else null
	if model == null:
		return
	if not _seats.has(id):
		_seats[id] = _seats_of(model, count)
	var seats: Array = _seats[id]
	var list: Array = figures.get_or_add(id, [])
	var wanted := mini(count, seats.size())
	while list.size() > wanted:
		var figure: Node = list.pop_back()
		figure.queue_free()
	while list.size() < wanted:
		var figure := _figure(faction, id * 64 + list.size())
		if figure == null:
			return
		model.add_child(figure)
		figure.transform = seats[list.size()]
		list.append(figure)


## Seat transforms in model space: the passenger sockets, else rows over
## the rear half of the model's bounding box (models face -Z).
func _seats_of(model: Node3D, count: int) -> Array:
	var sockets: Array[Node3D] = []
	for child in model.find_children("Socket_Passenger_*", "Node3D", true, false):
		sockets.append(child as Node3D)
	if not sockets.is_empty():
		sockets.sort_custom(func(a: Node3D, b: Node3D) -> bool: return String(a.name) < String(b.name))
		var result: Array = []
		for socket in sockets:
			result.append(model.global_transform.affine_inverse() * socket.global_transform)
		return result
	var box := AABB()
	var first := true
	for mesh: VisualInstance3D in model.find_children("*", "VisualInstance3D", true, false):
		var local := model.global_transform.affine_inverse() * mesh.global_transform * mesh.get_aabb()
		box = local if first else box.merge(local)
		first = false
	if first:
		return []
	var result: Array = []
	var rows := ceili(count / 2.0)
	var deck := box.position.y + box.size.y * 0.55
	for k in count:
		var side := -1.0 if k % 2 == 0 else 1.0
		var row := k / 2
		var z := box.get_center().z + box.size.z * (0.08 + 0.38 * (row + 0.5) / maxf(rows, 1.0))
		var x := side * box.size.x * 0.3
		# Riders face outwards.
		var basis := Basis(Vector3.UP, -side * PI * 0.5)
		result.append(Transform3D(basis, Vector3(x, deck, z)))
	return result


func _figure(faction: String, seed: int) -> Node3D:
	var path := "%s%s/infantry/rifleman.glb" % [MODELS_DIR, faction]
	if not ResourceLoader.exists(path):
		return null
	var figure := (load(path) as PackedScene).instantiate() as Node3D
	figure.name = "Passenger"
	var biome: String = main.atmosphere.preset.camo_biome if main.atmosphere != null else "forest"
	SoldierLook.apply(figure, faction, biome, seed)
	for mesh: GeometryInstance3D in figure.find_children("*", "GeometryInstance3D", true, false):
		mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var player := SoldierLook.find_player(figure)
	if player != null:
		SoldierLook.configure_animations(player)
		player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
		if player.has_animation(POSE):
			player.play(POSE)
			player.advance(POSE_SECONDS)
	return figure
