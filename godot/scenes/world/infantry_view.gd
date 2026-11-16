extends Node3D
## Draws the soldiers of every infantry squad (prompt 022). Presentation only:
## the simulation decides where every soldier stands, his stance, hit points
## and cover; this script places the members of the squad scenes of prompt
## 006/008 there and picks their animation clips.
##
## A squad unit node (from `main.gd`) holds the squad scene as its "Model";
## its `Member_NN` children are the soldiers (`NN` = soldier index + 1).
## Every frame each member gets its interpolated position and a clip:
##
## - dead: `death_a` / `death_b` / `death_c` once, then the body stays;
## - moving: `walk` or `run` (standing, kneeling), `crawl` (prone), played at
##   `speed / authored speed` so the feet do not slide;
## - fired in the last FIRE_SECONDS: `fire_<stance>`, aiming for AIM_SECONDS
##   after a shot: `aim_<stance>`;
## - else `idle`, `kneel` or `prone`.
## Squads in a building are hidden (their soldiers stand at the windows).
##
## Animation LOD: every AnimationPlayer runs in manual mode. Members within
## NEAR_M of the camera advance every frame and cast shadows, up to MID_M
## every MID_STEP frames, beyond with their squad (every FAR_SQUAD_STEP
## frames, see `_skip_squad`) and show only their
## body (gear and weapon hidden: 4 instead of 11 draw calls per soldier).

const NEAR_M := 70.0
const MID_M := 260.0
const MID_STEP := 3
## Soldiers farther than this from the camera are drawn as MultiMesh
## instances (about 8 px tall at 1080p).
const IMPOSTOR_M := 450.0
## Squads farther than this outside the camera's view are not updated.
const CULL_MARGIN_M := 25.0
## Squads beyond MID_M are updated every this many frames (a soldier moves
## about 0.3 m in that time, under a pixel at that distance).
const FAR_SQUAD_STEP := 4
const FIRE_SECONDS := 0.35
const AIM_SECONDS := 2.5
## Game speed (m/s) from which soldiers run instead of walk.
const RUN_FROM := 2.6
## Death clips, picked by soldier seed (death_c: prompt 042).
const DEATH_CLIPS: Array[String] = ["death_a", "death_b", "death_c"]
## Below this game speed (m/s) a soldier counts as standing.
const MOVING_FROM := 0.15
const BLEND_SECONDS := 0.2
const TURN_RATE := 8.0
## Non-locomotion clips are authored in real time; the game runs at factor 2.
const TIME_FACTOR := 2.0
const STANCE_SUFFIX: Array[String] = ["stand", "kneel", "prone"]
const IDLE_CLIPS: Array[String] = ["idle", "kneel", "prone"]

var main: Node3D
var sim_host: SimHost
## Members drawn in the last frame per LOD band (near, mid, far), and hidden.
var counts := [0, 0, 0, 0]
## Time of the last `_process` in microseconds (benchmark).
var process_usec := 0
## Soldier state by key (unit id * 256 + index).
var _soldiers: Dictionary[int, Dictionary] = {}
## Keys of the soldiers of every unit id, from the last frame.
var _by_unit: Dictionary[int, Array] = {}
var _frame := 0
## Far soldiers per faction: MultiMesh node, instance transforms by soldier
## key, factions whose MultiMesh needs an upload; impostor keys per unit.
var _impostor_nodes: Dictionary[String, MultiMeshInstance3D] = {}
var _impostors: Dictionary[String, Dictionary] = {}
var _impostor_dirty: Dictionary[String, bool] = {}
var _impostor_keys: Dictionary[int, Array] = {}
## Faction prefix per unit id (`_faction_of`).
var _factions: Dictionary[int, String] = {}
var _biome := ""


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	main.combat_view.combat_event.connect(_on_combat_event)


## The squad scene of a catalog id (`assets/models/<faction>/<id>.tscn`), or
## null if the id has none (vehicles have `.glb` models).
static func squad_model(models_dir: String, type_id: String) -> Node3D:
	if type_id.is_empty():
		return null
	var path := "%s%s/%s.tscn" % [models_dir, type_id.get_slice("_", 0), type_id]
	if not ResourceLoader.exists(path):
		return null
	var model := (load(path) as PackedScene).instantiate() as Node3D
	model.set_meta(&"squad", true)
	return model


func _on_combat_event(event: Dictionary) -> void:
	if event["type"] != "shot":
		return
	var keys: Array = _by_unit.get(int(event["unit"]), [])
	if keys.is_empty():
		return
	# The shooter is the soldier nearest to the muzzle.
	var muzzle: Vector3 = event["position"]
	var best: Dictionary = {}
	var best_d := INF
	for key: int in keys:
		var s: Dictionary = _soldiers[key]
		var d: float = (s["position"] as Vector3).distance_squared_to(muzzle)
		if d < best_d:
			best_d = d
			best = s
	if not best.is_empty():
		best["fired"] = Time.get_ticks_msec()


func _process(delta: float) -> void:
	if sim_host == null or main == null:
		return
	var began := Time.get_ticks_usec()
	_draw(delta)
	process_usec = Time.get_ticks_usec() - began


func _draw(delta: float) -> void:
	_frame += 1
	var biome: String = main.atmosphere.preset.camo_biome if main.atmosphere != null else "forest"
	if biome != _biome:
		_biome = biome
		for s: Dictionary in _soldiers.values():
			SoldierLook.apply(s["node"], s["faction"], _biome, s["seed"])
		# The far MultiMeshes carry the old palette: build them again.
		for node: Node in _impostor_nodes.values():
			node.queue_free()
		_impostor_nodes.clear()
		_impostors.clear()
		_impostor_dirty.clear()
		for s: Dictionary in _soldiers.values():
			s["impostor"] = false
		_impostor_keys.clear()
	var unit_ids := sim_host.unit_ids()
	var stances := sim_host.unit_stances()
	var garrisons := sim_host.unit_garrisons()
	var aims := sim_host.unit_aim_yaws()
	var unit_index := {}
	for i in unit_ids.size():
		if stances[i] >= 0:
			unit_index[unit_ids[i]] = i
	var ids := sim_host.soldier_units()
	var indices := sim_host.soldier_indices()
	var current := sim_host.soldier_positions()
	var previous := sim_host.soldier_previous_positions()
	var hp := sim_host.soldier_hp()
	var alpha := sim_host.interpolation_alpha()
	var tick_rate := float(sim_host.tick_rate())
	var camera_position: Vector3 = main.camera.global_position
	var now := Time.get_ticks_msec()
	var turn := clampf(delta * TURN_RATE, 0.0, 1.0)
	counts = [0, 0, 0, 0]
	_by_unit.clear()
	var camera: Camera3D = main.camera
	var frustum := camera.get_frustum()
	# Per unit id: whether its squad is skipped this frame (prompt 016).
	var skip := {}
	var crossing: Node = main.get("crossing_view")
	for k in ids.size():
		var unit_id := ids[k]
		var node: Node3D = main._unit_nodes.get(unit_id)
		if node == null or not node.visible or not unit_index.has(unit_id):
			continue
		if not skip.has(unit_id):
			skip[unit_id] = _skip_squad(node, unit_id, frustum, camera_position)
		if skip[unit_id]:
			# Remember the members for shot lookups; their state catches up
			# when the squad is drawn again.
			var key_skipped := unit_id * 256 + indices[k]
			var skipped: Dictionary = _soldiers.get(key_skipped, {})
			if skipped.is_empty():
				# Set up now: a member never set up keeps the scene's
				# shadows, which cost draw calls even off screen.
				skipped = _make(node, key_skipped, indices[k])
			if not skipped.is_empty():
				(_by_unit.get_or_add(unit_id, []) as Array).append(key_skipped)
				skipped["accum"] = float(skipped["accum"]) + delta
			continue
		var key := unit_id * 256 + indices[k]
		var s: Dictionary = _soldiers.get(key, {})
		if s.is_empty():
			s = _make(node, key, indices[k])
			if s.is_empty():
				continue
		var member: Node3D = s["node"]
		var u: int = unit_index[unit_id]
		(_by_unit.get_or_add(unit_id, []) as Array).append(key)
		var position := previous[k].lerp(current[k], alpha)
		# Swimming squads (prompt 055) float at the water surface.
		if crossing != null and crossing.is_swimming(unit_id):
			position.y = crossing.swim_height(position)
		s["position"] = position
		if garrisons[u] >= 0:
			member.visible = false
			counts[3] += 1
			if s["impostor"]:
				_drop_impostor(s, key)
			continue
		var step := current[k] - previous[k]
		step.y = 0.0
		var speed := step.length() * tick_rate
		var moving := speed > MOVING_FROM and hp[k] > 0
		var yaw: float = atan2(-step.x, -step.z) if moving else aims[u]
		s["yaw"] = lerp_angle(s["yaw"], yaw, turn)
		var distance := position.distance_to(camera_position)
		if distance > IMPOSTOR_M:
			member.visible = false
			counts[2] += 1
			_set_impostor(s, unit_id, key, position, hp[k] <= 0, stances[u])
			continue
		if s["impostor"]:
			_drop_impostor(s, key)
		member.visible = true
		# Members are top-level nodes (`_make`): world transforms.
		member.position = position
		member.rotation.y = s["yaw"]
		_animate(s, hp[k] <= 0, moving, speed, stances[u], now)
		_advance(s, delta, distance)
	_flush_impostors(unit_index)


## Far soldiers (prompt 016): beyond IMPOSTOR_M a soldier is a few pixels
## tall; instead of his skinned model (several draw calls, a skeleton) he is
## one instance of a MultiMesh per faction that shows the body in its rest
## pose, upright, lower when kneeling, lying when prone or dead.
func _set_impostor(s: Dictionary, unit_id: int, key: int, position: Vector3, dead: bool, stance: int) -> void:
	var faction: String = s["faction"]
	if not _impostor_nodes.has(faction):
		var made := _make_impostor(s)
		if made == null:
			return
		_impostor_nodes[faction] = made
		_impostors[faction] = {}
	var basis := Basis(Vector3.UP, float(s["yaw"]))
	if dead or stance == 2:
		basis = basis * Basis(Vector3.RIGHT, -PI * 0.5)
	elif stance == 1:
		basis = basis.scaled_local(Vector3(1.0, 0.65, 1.0))
	(_impostors[faction] as Dictionary)[key] = Transform3D(basis, position)
	_impostor_dirty[faction] = true
	if not s["impostor"]:
		s["impostor"] = true
		(_impostor_keys.get_or_add(unit_id, []) as Array).append(key)


func _drop_impostor(s: Dictionary, key: int) -> void:
	s["impostor"] = false
	var faction: String = s["faction"]
	if _impostors.has(faction):
		(_impostors[faction] as Dictionary).erase(key)
		_impostor_dirty[faction] = true
	var keys: Array = _impostor_keys.get(key / 256, [])
	keys.erase(key)


## The far MultiMesh of a soldier's faction: his body mesh with the
## faction's uniform materials.
func _make_impostor(s: Dictionary) -> MultiMeshInstance3D:
	var body := (s["node"] as Node).find_child("Body", true, false) as MeshInstance3D
	if body == null or body.mesh == null:
		return null
	# Without the automatic LODs: the whole MultiMesh counts as one far
	# object, and the lowest LOD of a body is a few scattered triangles.
	var source := body.mesh
	var mesh := ArrayMesh.new()
	for i in source.get_surface_count():
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, source.surface_get_arrays(i))
		mesh.surface_set_material(i, body.get_active_material(i))
	var node := MultiMeshInstance3D.new()
	node.name = "FarSoldiers_" + str(s["faction"])
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.multimesh = MultiMesh.new()
	node.multimesh.transform_format = MultiMesh.TRANSFORM_3D
	node.multimesh.mesh = mesh
	add_child(node)
	return node


## Removes the impostors of squads that left the snapshot or are hidden
## (fog of war) and uploads the changed MultiMeshes.
func _flush_impostors(unit_index: Dictionary) -> void:
	for unit_id: int in _impostor_keys.keys():
		var node: Node3D = main._unit_nodes.get(unit_id)
		if unit_index.has(unit_id) and node != null and node.visible:
			continue
		for key: int in (_impostor_keys[unit_id] as Array).duplicate():
			var s: Dictionary = _soldiers.get(key, {})
			if not s.is_empty():
				_drop_impostor(s, key)
		_impostor_keys.erase(unit_id)
	for faction: String in _impostor_dirty:
		var transforms: Dictionary = _impostors[faction]
		var mm: MultiMesh = (_impostor_nodes[faction] as MultiMeshInstance3D).multimesh
		mm.instance_count = transforms.size()
		var i := 0
		for xform: Transform3D in transforms.values():
			mm.set_instance_transform(i, xform)
			i += 1
	_impostor_dirty.clear()


## Squad LOD (prompt 016): squads outside the camera's view (grown by
## CULL_MARGIN_M, so shadows and soldiers at the screen edge stay right) are
## not updated at all, and squads beyond MID_M only every FAR_SQUAD_STEP frames
## (staggered by unit id): 2250 soldiers cost 6 ms per frame otherwise.
## Their animation time accumulates and is applied when they are drawn.
func _skip_squad(node: Node3D, unit_id: int, frustum: Array[Plane], camera_position: Vector3) -> bool:
	var center := node.global_position
	for plane in frustum:
		if plane.distance_to(center) > CULL_MARGIN_M:
			return true
	if center.distance_to(camera_position) > MID_M:
		return (_frame + unit_id) % FAR_SQUAD_STEP != 0
	return false


## Faction prefix of a unit's catalog id; the type ids are read once per
## frame in which new soldiers appear, not once per soldier (prompt 016).
func _faction_of(unit_id: int) -> String:
	if not _factions.has(unit_id):
		var ids := sim_host.unit_ids()
		var types := sim_host.unit_type_ids()
		for i in ids.size():
			_factions[ids[i]] = (types[i] as String).get_slice("_", 0)
	return _factions.get(unit_id, "de")


## Prepares member `index` of a squad node: look, animation player in
## manual mode. Empty if the scene has no such member.
func _make(node: Node3D, key: int, index: int) -> Dictionary:
	var model := node.get_node_or_null("Model")
	if model == null or not model.has_meta(&"squad"):
		return {}
	var member := model.get_node_or_null("Member_%02d" % (index + 1)) as Node3D
	if member == null:
		return {}
	var faction := _faction_of(key / 256)
	var seed := key * 7919
	# Placed in world space every time they are drawn: a moving squad node
	# then does not drag hundreds of skeleton and attachment nodes along
	# every frame (prompt 016).
	member.top_level = true
	SoldierLook.apply(member, faction, _biome, seed)
	var player := SoldierLook.find_player(member)
	if player != null:
		SoldierLook.configure_animations(player)
		player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	var meshes: Array[GeometryInstance3D] = []
	var details: Array[GeometryInstance3D] = []
	for mesh: GeometryInstance3D in member.find_children("*", "GeometryInstance3D", true, false):
		meshes.append(mesh)
		# No shadows until `_advance` finds the soldier near the camera.
		mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if mesh.name != "Body":
			details.append(mesh)
	var s := {
		"node": member, "player": player, "faction": faction, "seed": seed,
		"clip": "", "dead": false, "fired": -100000, "yaw": 0.0, "accum": 0.0,
		"phase": key % MID_STEP, "meshes": meshes, "details": details, "shadows": false,
		"far": false, "impostor": false,
		"position": Vector3.ZERO,
	}
	_soldiers[key] = s
	return s


func _animate(s: Dictionary, dead: bool, moving: bool, speed: float, stance: int, now: int) -> void:
	var player: AnimationPlayer = s["player"]
	if player == null or s["dead"]:
		return
	var clip: String
	var scale := TIME_FACTOR
	if dead:
		s["dead"] = true
		clip = DEATH_CLIPS[int(s["seed"]) % DEATH_CLIPS.size()]
	elif moving:
		clip = "crawl" if stance == 2 else ("run" if speed >= RUN_FROM else "walk")
		var authored: float = SoldierLook.clips().get(clip, {}).get("speed", 1.0)
		scale = speed / maxf(authored, 0.01)
	else:
		var since := now - int(s["fired"])
		var suffix := STANCE_SUFFIX[clampi(stance, 0, 2)]
		if since < FIRE_SECONDS * 1000.0:
			clip = "fire_" + suffix
		elif since < AIM_SECONDS * 1000.0:
			clip = "aim_" + suffix
		else:
			clip = IDLE_CLIPS[clampi(stance, 0, 2)]
	player.speed_scale = scale
	if clip != s["clip"] and player.has_animation(clip):
		s["clip"] = clip
		player.play(clip, BLEND_SECONDS)


## Animation LOD by camera distance.
func _advance(s: Dictionary, delta: float, distance: float) -> void:
	var player: AnimationPlayer = s["player"]
	if player == null:
		return
	var band := 0 if distance < NEAR_M else (1 if distance < MID_M else 2)
	counts[band] += 1
	var shadows := band == 0
	if shadows != s["shadows"]:
		s["shadows"] = shadows
		var setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows \
				else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		for mesh: GeometryInstance3D in s["meshes"]:
			mesh.cast_shadow = setting
	var far := band == 2
	if far != s["far"]:
		s["far"] = far
		for mesh: GeometryInstance3D in s["details"]:
			mesh.visible = not far
	s["accum"] = float(s["accum"]) + delta
	# Far squads are only drawn every FAR_SQUAD_STEP frames (`_skip_squad`);
	# their soldiers advance whenever drawn.
	var step := MID_STEP if band == 1 else 1
	if step == 1 or (_frame + int(s["phase"])) % step == 0:
		player.advance(s["accum"])
		s["accum"] = 0.0
