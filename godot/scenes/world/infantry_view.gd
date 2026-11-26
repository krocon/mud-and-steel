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
## The squad's rank (prompt 050) changes the soldiers' kit: worn uniforms,
## captured camouflage garments (`SoldierLook.apply`).
##
## The distances below are meters with the classic 30 degrees field of
## view; with another one they count by the size on the screen
## (`rts_camera.gd` `detail_distance_factor`, prompt 217).
##
## Animation LOD (prompts 016, 206): every AnimationPlayer runs in manual mode.
## Members within NEAR_M of the camera cast shadows and advance every frame when
## active (moving, aiming, firing, reloading, diving, falling, care) or every
## NEAR_IDLE_STEP frames while in a calm idle/cover pose. Beyond DETAIL_M
## (provisional 150 m) gear and weapon meshes are hidden. Up to MID_M animations
## advance every MID_STEP frames; beyond MID_M squads update every
## FAR_SQUAD_STEP frames (`_skip_squad`), and beyond IMPOSTOR_M soldiers draw as
## MultiMesh impostors.

const NEAR_M := 70.0
## Distance (m) beyond which Gear and Weapon detail meshes hide (prompt 206, provisional).
const DETAIL_M := 150.0
const MID_M := 260.0
const MID_STEP := 3
## Mid-band calm idle poses advance every this many frames (prompt 206, provisional).
const MID_IDLE_STEP := 6
## Near-band calm idle poses advance every this many frames (prompt 206, provisional).
const NEAR_IDLE_STEP := 3
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
const FIRE_MS := int(FIRE_SECONDS * 1000.0)
const AIM_MS := int(AIM_SECONDS * 1000.0)
## Game speed (m/s) from which upright soldiers run instead of walk (prompt 231:
## light squads at 3.89 m/s run, exhausted squads at 2.72 m/s and heavy teams
## at 2.78 m/s fall back to walk).
const RUN_FROM := 3.0
## Game speed (m/s) below which kneeling squads stay in `crouch_walk` (2.33 m/s
## off road, 2.92 m/s on road).
const CROUCH_RUN_FROM := 3.2
## Death clips, picked by soldier seed (death_c: prompt 042).
const DEATH_CLIPS: Array[String] = ["death_a", "death_b", "death_c"]
## Below this game speed (m/s) a soldier counts as standing.
const MOVING_FROM := 0.15
const MOVING_FROM_SQ := MOVING_FROM * MOVING_FROM
const BLEND_SECONDS := 0.2
const TURN_RATE := 8.0
## Non-locomotion clips are authored in real time; the game runs at factor 2.
const TIME_FACTOR := 2.0
const STANCE_SUFFIX: Array[String] = ["stand", "kneel", "prone"]
const IDLE_CLIPS: Array[String] = ["idle", "kneel", "prone"]
## Prompt 047: a soldier clearing a jammed weapon (`SimHost.soldier_tactics`).
const JAMMED := 1
## Prompt 196: a soldier who threw himself down under close fire lies.
const DOWN := 8
const RELOAD_CLIPS: Array[String] = ["reload", "reload_kneel", "reload_prone"]
const PRUNE_INTERVAL := 240
const Motion := preload("res://scenes/world/soldier_motion.gd")
const RtsCamera := preload("res://scenes/camera/rts_camera.gd")
const Helmets := preload("res://scenes/world/helmets.gd")
const Care := preload("res://scenes/world/care_view.gd")


## Typed per-soldier presentation state (prompt 206): avoids per-frame String
## dictionary hashing and empty `{}` default allocations.
class SoldierState:
	var node: Node3D
	var player: AnimationPlayer
	var faction := ""
	var seed := 0
	var clip := ""
	var dead := false
	var fired := -100000
	var yaw := 0.0
	var accum := 0.0
	var phase := 0
	var meshes: Array[GeometryInstance3D] = []
	var details: Array[GeometryInstance3D] = []
	var shadows := false
	var far := false
	var impostor := false
	var rank := 0
	var position := Vector3.ZERO
	var last_pos := Vector3.INF
	var last_yaw := INF
	var last_speed_scale := -1.0
	var last_stance := -1
	var roll_dir := 0
	var hold_yaw := false
	var care_yaw := false
	var care := false
	var care_lying := false
	var care_last := Vector3.ZERO
	var down := false
	var getting_up := false
	var has_knock_from := false
	var knock_from := Vector3.ZERO
	var knocked := 0
	var turning := 0.0
	var lead: SkeletonModifier3D = null
	var skeleton: Skeleton3D = null
	var has_hit_dir := false
	var hit_dir := Vector3.FORWARD
	var has_flight := false
	var flight_to := Vector3.ZERO
	var flight_left := 0.0
	var flight_distance := 0.0
	var flight_counted := false
	var bandage_done := false
	var bandage: MeshInstance3D = null
	var stretcher: Node3D = null
	var is_idle := false
	var impostor_pos := Vector3.INF
	var impostor_yaw := INF
	var impostor_pose := -1


var main: Node3D
var sim_host: SimHost
## Members drawn in the last frame per LOD band (near, mid, far), and hidden.
var counts := [0, 0, 0, 0]
## Time of the last `_process` in microseconds (benchmark).
var process_usec := 0
## Soldier state by key (unit id * 256 + index).
var _soldiers: Dictionary[int, SoldierState] = {}
## Keys of the soldiers of every unit id, maintained incrementally (prompt 206).
var _by_unit: Dictionary[int, Array] = {}
## Squads whose members have all been initialized once (enables O(1) squad skipping).
var _squad_ready: Dictionary[int, bool] = {}
## Accumulated delta (s) for skipped squads, applied when drawn again.
var _squad_accum: Dictionary[int, float] = {}
var _frame := 0
## Tick-cached snapshot arrays and squad spans (prompt 206): avoids 19 bridge
## FFI array copies on interpolated frames where `sim_host.current_tick()` did not change.
var _snap_tick := -1
var _snap_unit_index: Dictionary[int, int] = {}
var _snap_squad_spans: Array = []  # Array of [unit_id, u_index, k_start, k_end]
var _snap_stances := PackedInt32Array()
var _snap_garrisons := PackedInt32Array()
var _snap_aims := PackedFloat32Array()
var _snap_ranks := PackedInt32Array()
var _snap_ids := PackedInt64Array()
var _snap_indices := PackedInt32Array()
var _snap_current := PackedVector3Array()
var _snap_previous := PackedVector3Array()
var _snap_hp := PackedInt32Array()
var _snap_knocked := PackedInt32Array()
var _snap_knock_from := PackedVector3Array()
var _snap_covers := PackedInt32Array()
var _snap_throw_from := PackedVector3Array()
var _snap_flights := PackedInt32Array()
var _snap_tactics := PackedInt32Array()
var _snap_wounds := PackedByteArray()
var _snap_aid := PackedByteArray()
var _snap_aid_targets := PackedVector3Array()
## Factor on camera distances for the detail levels this frame (1 with the
## classic field of view).
var _detail_factor := 1.0
## Far soldiers per faction: MultiMesh node, instance transforms by soldier
## key, factions whose MultiMesh needs an upload; impostor keys per unit.
var _impostor_nodes: Dictionary[String, MultiMeshInstance3D] = {}
var _impostors: Dictionary[String, Dictionary] = {}
var _impostor_dirty: Dictionary[String, bool] = {}
var _impostor_keys: Dictionary[int, Array] = {}
## Faction prefix per unit id (`_faction_of`).
var _factions: Dictionary[int, String] = {}
var _biome := ""
## Cached authored clip speeds (`clip -> speed`, prompt 206).
var _clip_speeds: Dictionary[String, float] = {}
## Ragdolls, flying helmets and the clip choice of prompt 048
## (`soldier_motion.gd`).
var motion: Motion
## Wounded men and their helpers of prompt 049 (`care_view.gd`).
var care: Care


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	main.combat_view.combat_event.connect(_on_combat_event)
	motion = Motion.new()
	motion.name = "SoldierMotion"
	add_child(motion)
	motion.setup(self)
	care = Care.new()
	care.name = "CareView"
	add_child(care)
	care.setup(self)


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
	if event["type"] == "hit":
		motion.on_hit(event)
		return
	if event["type"] != "shot":
		return
	var unit_id := int(event["unit"])
	if not _by_unit.has(unit_id):
		return
	var keys: Array = _by_unit[unit_id]
	if keys.is_empty():
		return
	# The shooter is the soldier nearest to the muzzle.
	var muzzle: Vector3 = event["position"]
	var best: SoldierState = null
	var best_d := INF
	for key: int in keys:
		var s: SoldierState = _soldiers.get(key)
		if s == null:
			continue
		var d: float = s.position.distance_squared_to(muzzle)
		if d < best_d:
			best_d = d
			best = s
	if best != null:
		best.fired = Time.get_ticks_msec()


func _process(delta: float) -> void:
	if sim_host == null or main == null:
		return
	var began := Time.get_ticks_usec()
	_draw(delta)
	process_usec = Time.get_ticks_usec() - began


func _refresh_snapshot(tick: int) -> void:
	_snap_tick = tick
	var unit_ids := sim_host.unit_ids()
	_snap_stances = sim_host.unit_stances()
	_snap_garrisons = sim_host.unit_garrisons()
	_snap_aims = sim_host.unit_aim_yaws()
	_snap_ranks = sim_host.unit_ranks()
	_snap_unit_index.clear()
	for i in unit_ids.size():
		if _snap_stances[i] >= 0:
			_snap_unit_index[unit_ids[i]] = i
	_snap_ids = sim_host.soldier_units()
	_snap_indices = sim_host.soldier_indices()
	_snap_current = sim_host.soldier_positions()
	_snap_previous = sim_host.soldier_previous_positions()
	_snap_hp = sim_host.soldier_hp()
	_snap_knocked = sim_host.soldier_knocked()
	_snap_knock_from = sim_host.soldier_knock_from()
	_snap_covers = sim_host.soldier_cover_objects()
	_snap_throw_from = sim_host.soldier_throw_from()
	_snap_flights = sim_host.soldier_flights()
	_snap_tactics = sim_host.soldier_tactics()
	_snap_wounds = sim_host.soldier_wounds()
	_snap_aid = sim_host.soldier_aid_kinds()
	_snap_aid_targets = sim_host.soldier_aid_targets()
	_snap_squad_spans.clear()
	var n := _snap_ids.size()
	var k := 0
	while k < n:
		var uid := _snap_ids[k]
		var k0 := k
		k += 1
		while k < n and _snap_ids[k] == uid:
			k += 1
		if _snap_unit_index.has(uid):
			_snap_squad_spans.append([uid, _snap_unit_index[uid], k0, k])


func _draw(delta: float) -> void:
	_frame += 1
	if _frame % PRUNE_INTERVAL == 0:
		prune_caches()
	var biome: String = main.atmosphere.preset.camo_biome if main.atmosphere != null else "forest"
	if biome != _biome:
		_biome = biome
		for s: SoldierState in _soldiers.values():
			SoldierLook.apply(s.node, s.faction, _biome, s.seed, "auto", s.rank)
		# The far MultiMeshes carry the old palette: build them again.
		for node: Node in _impostor_nodes.values():
			node.queue_free()
		_impostor_nodes.clear()
		_impostors.clear()
		_impostor_dirty.clear()
		for s: SoldierState in _soldiers.values():
			s.impostor = false
			s.impostor_pose = -1
		_impostor_keys.clear()
	var tick := sim_host.current_tick()
	if tick != _snap_tick:
		_refresh_snapshot(tick)
	var unit_index := _snap_unit_index
	var stances := _snap_stances
	var garrisons := _snap_garrisons
	var aims := _snap_aims
	var ranks := _snap_ranks
	var indices := _snap_indices
	var current := _snap_current
	var previous := _snap_previous
	var hp := _snap_hp
	var knocked := _snap_knocked
	var knock_from := _snap_knock_from
	var covers := _snap_covers
	var throw_from := _snap_throw_from
	var flights := _snap_flights
	var tactics := _snap_tactics
	var wounds := _snap_wounds
	var aid := _snap_aid
	var aid_targets := _snap_aid_targets
	var alpha := sim_host.interpolation_alpha()
	var tick_rate := float(sim_host.tick_rate())
	var tick_rate_sq := tick_rate * tick_rate
	var camera: Camera3D = main.camera
	var camera_position: Vector3 = camera.global_position
	_detail_factor = RtsCamera.detail_distance_factor(camera)
	var inv_factor := 1.0 / maxf(_detail_factor, 1e-4)
	var near_sq := (NEAR_M * inv_factor) * (NEAR_M * inv_factor)
	var detail_sq := (DETAIL_M * inv_factor) * (DETAIL_M * inv_factor)
	var mid_sq := (MID_M * inv_factor) * (MID_M * inv_factor)
	var impostor_sq := (IMPOSTOR_M * inv_factor) * (IMPOSTOR_M * inv_factor)
	var now := Time.get_ticks_msec()
	var turn := clampf(delta * TURN_RATE, 0.0, 1.0)
	var standing_turn := turn * motion.STANDING_TURN
	counts = [0, 0, 0, 0]
	var frustum := camera.get_frustum()
	var crossing: Node = main.crossing_view
	var unit_nodes: Dictionary = main._unit_nodes
	var flights_size := flights.size()
	var tactics_size := tactics.size()
	var ranks_size := ranks.size()
	for span: Array in _snap_squad_spans:
		var unit_id: int = span[0]
		var node: Node3D = unit_nodes.get(unit_id)
		if node == null or not node.visible:
			continue
		var u: int = span[1]
		var k0: int = span[2]
		var k1: int = span[3]
		if _skip_squad(node, unit_id, frustum, camera_position, mid_sq):
			if _squad_ready.has(unit_id):
				_squad_accum[unit_id] = float(_squad_accum.get(unit_id, 0.0)) + delta
				continue
			# First-time setup: a member never set up keeps the scene's
			# shadows, which cost draw calls even off screen.
			var all_ready := true
			for k in range(k0, k1):
				var key_skipped := unit_id * 256 + indices[k]
				var skipped: SoldierState = _soldiers.get(key_skipped)
				if skipped == null:
					skipped = _make(node, unit_id, key_skipped, indices[k])
				if skipped == null:
					all_ready = false
				else:
					skipped.accum += delta
			if all_ready:
				_squad_ready[unit_id] = true
			continue
		var squad_dt := delta
		if _squad_accum.has(unit_id):
			squad_dt += float(_squad_accum[unit_id])
			_squad_accum.erase(unit_id)
		var is_swimming: bool = crossing != null and bool(crossing.is_swimming(unit_id))
		var garrisoned := garrisons[u] >= 0
		var aim_u := aims[u]
		var stance_u := stances[u]
		var rank_u := ranks[u] if u < ranks_size else 0
		var all_made := true
		for k in range(k0, k1):
			var key := unit_id * 256 + indices[k]
			var s: SoldierState = _soldiers.get(key)
			if s == null:
				s = _make(node, unit_id, key, indices[k])
				if s == null:
					all_made = false
					continue
			var member := s.node
			if rank_u != s.rank:
				s.rank = rank_u
				SoldierLook.apply(member, s.faction, _biome, s.seed, "auto", s.rank)
			var cur_p := current[k]
			var prev_p := previous[k]
			var position := cur_p if cur_p == prev_p else prev_p.lerp(cur_p, alpha)
			# Prompt 194: thrown by a blast, he flies the simulation's arc to
			# his landing point (a ragdoll flies there on its own).
			if 2 * k + 1 < flights_size and flights[2 * k + 1] > 0 \
					and float(flights[2 * k]) + alpha < float(flights[2 * k + 1]):
				var total := float(flights[2 * k + 1])
				var since := float(flights[2 * k]) + alpha
				position = motion.flight_position(s, throw_from[k], cur_p, since / total,
						(total - since) / maxf(tick_rate, 1.0))
			elif s.has_flight:
				motion.flight_ended(s)
			# Swimming squads (prompt 055) float at the water surface.
			if is_swimming:
				position.y = crossing.swim_height(position)
			s.position = position
			# In a building, or a crew member who did not get out (prompt 049).
			var wound_k := wounds[k]
			if garrisoned or wound_k == Care.Wound.GONE:
				if member.visible:
					member.visible = false
				counts[3] += 1
				if s.impostor:
					_drop_impostor(s, key)
				continue
			var dx := cur_p.x - prev_p.x
			var dz := cur_p.z - prev_p.z
			var step_sq := dx * dx + dz * dz
			var hp_k := hp[k]
			var moving := hp_k > 0 and (step_sq * tick_rate_sq) > MOVING_FROM_SQ
			var speed := sqrt(step_sq) * tick_rate if moving else 0.0
			var yaw: float = atan2(-dx, -dz) if moving else aim_u
			# Prompt 048 (`soldier_motion.gd`): the cheap common case is checked
			# here, the rest only for the few soldiers that need it.
			if moving and stance_u == 2:
				yaw = motion.body_yaw(s, yaw, aim_u, moving, stance_u, knocked[k] > 0)
			else:
				s.roll_dir = 0
			if not s.hold_yaw and not s.care_yaw:
				if absf(angle_difference(s.yaw, yaw)) > 1e-4:
					s.yaw = lerp_angle(s.yaw, yaw, turn if moving else standing_turn)
				else:
					s.yaw = yaw
			var dist_sq := position.distance_squared_to(camera_position)
			if dist_sq > impostor_sq:
				if member.visible:
					member.visible = false
				counts[2] += 1
				var lying := hp_k <= 0 or wound_k == Care.Wound.DOWN or wound_k == Care.Wound.DOWN_STABLE
				_set_impostor(s, unit_id, key, position, lying, stance_u)
				continue
			if s.impostor:
				_drop_impostor(s, key)
			if not member.visible:
				member.visible = true
			# Stillness fast-path (prompt 206): avoid invalidating 24-bone skeletons
			# when position and yaw have not changed.
			if position.distance_squared_to(s.last_pos) > 1e-8:
				member.position = position
				s.last_pos = position
			if absf(s.yaw - s.last_yaw) > 1e-4:
				member.rotation.y = s.yaw
				s.last_yaw = s.yaw
			var aid_k := aid[k]
			var knocked_k := knocked[k]
			if not s.dead and (wound_k != 0 or aid_k != 0 or s.care) \
					and care.animate(s, hp_k, knocked_k, wound_k, aid_k, aid_targets[k], position, moving, speed):
				s.is_idle = false
			elif s.dead:
				s.is_idle = false
			elif hp_k > 0 and knocked_k == 0 and not s.down:
				# Remember the last blast point: a killing blast changes it.
				s.knock_from = knock_from[k]
				s.has_knock_from = true
				s.knocked = 0
				var flags: int = tactics[k] if k < tactics_size else 0
				# Down under close fire (prompt 196): he dives and lies whatever
				# the squad's stance.
				var stance: int = 2 if flags & DOWN else stance_u
				_animate(s, false, moving, speed, stance, now, covers[k], flags)
			elif not motion.animate(s, hp_k <= 0, knocked_k, knock_from[k], position, camera_position, tick_rate):
				_animate(s, hp_k <= 0, moving, speed, stance_u, now, covers[k])
			else:
				s.is_idle = false
			if dist_sq < near_sq or s.lead != null:
				motion.lead(s, yaw, dist_sq < near_sq)
			_advance(s, squad_dt, dist_sq, near_sq, mid_sq, detail_sq)
		if all_made:
			_squad_ready[unit_id] = true
	_flush_impostors(unit_index)


## Removes cached state for units whose nodes were removed from the scene (prompt 206).
func prune_caches() -> void:
	if main == null:
		return
	var unit_nodes: Dictionary = main._unit_nodes
	for unit_id: int in _by_unit.keys():
		var node: Variant = unit_nodes.get(unit_id)
		if node == null or not is_instance_valid(node):
			for key: int in _by_unit[unit_id]:
				var s: SoldierState = _soldiers.get(key)
				if s != null and s.impostor:
					_drop_impostor(s, key)
				_soldiers.erase(key)
			_by_unit.erase(unit_id)
			_squad_ready.erase(unit_id)
			_squad_accum.erase(unit_id)
			_factions.erase(unit_id)
	for unit_id: int in _factions.keys():
		var node: Variant = unit_nodes.get(unit_id)
		if node == null or not is_instance_valid(node):
			_factions.erase(unit_id)


## Far soldiers (prompt 016): beyond IMPOSTOR_M a soldier is a few pixels
## tall; instead of his skinned model (several draw calls, a skeleton) he is
## one instance of a MultiMesh per faction that shows the body in its rest
## pose, upright, lower when kneeling, lying when prone or dead.
func _set_impostor(s: SoldierState, unit_id: int, key: int, position: Vector3, dead: bool, stance: int) -> void:
	var pose := 2 if (dead or stance == 2) else (1 if stance == 1 else 0)
	if s.impostor and pose == s.impostor_pose and position.distance_squared_to(s.impostor_pos) <= 1e-8 \
			and absf(s.yaw - s.impostor_yaw) <= 1e-4:
		return
	var faction := s.faction
	if not _impostor_nodes.has(faction):
		var made := _make_impostor(s)
		if made == null:
			return
		_impostor_nodes[faction] = made
		_impostors[faction] = {}
	var basis := Basis(Vector3.UP, s.yaw)
	if pose == 2:
		basis = basis * Basis(Vector3.RIGHT, -PI * 0.5)
	elif pose == 1:
		basis = basis.scaled_local(Vector3(1.0, 0.65, 1.0))
	(_impostors[faction] as Dictionary)[key] = Transform3D(basis, position)
	_impostor_dirty[faction] = true
	s.impostor_pos = position
	s.impostor_yaw = s.yaw
	s.impostor_pose = pose
	if not s.impostor:
		s.impostor = true
		(_impostor_keys.get_or_add(unit_id, []) as Array).append(key)


func _drop_impostor(s: SoldierState, key: int) -> void:
	s.impostor = false
	s.impostor_pose = -1
	var faction := s.faction
	if _impostors.has(faction):
		(_impostors[faction] as Dictionary).erase(key)
		_impostor_dirty[faction] = true
	var unit_id := key / 256
	if _impostor_keys.has(unit_id):
		(_impostor_keys[unit_id] as Array).erase(key)


## The far MultiMesh of a soldier's faction: his body mesh with the
## faction's uniform materials.
func _make_impostor(s: SoldierState) -> MultiMeshInstance3D:
	var body := s.node.find_child("Body", true, false) as MeshInstance3D
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
	node.name = "FarSoldiers_" + s.faction
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
			var s: SoldierState = _soldiers.get(key)
			if s != null:
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
func _skip_squad(node: Node3D, unit_id: int, frustum: Array[Plane], camera_position: Vector3,
		mid_sq: float) -> bool:
	var center := node.global_position
	for plane in frustum:
		if plane.distance_to(center) > CULL_MARGIN_M:
			return true
	if center.distance_squared_to(camera_position) > mid_sq:
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


func _authored_speed(clip: String) -> float:
	if not _clip_speeds.has(clip):
		var clips: Dictionary = SoldierLook.clips()
		var entry: Variant = clips.get(clip)
		_clip_speeds[clip] = float(entry.get("speed", 1.0)) if entry is Dictionary else 1.0
	return _clip_speeds[clip]


## Prepares member `index` of a squad node: look, animation player in
## manual mode. Null if the scene has no such member.
func _make(node: Node3D, unit_id: int, key: int, index: int) -> SoldierState:
	var model := node.get_node_or_null("Model")
	if model == null or not model.has_meta(&"squad"):
		return null
	var member := model.get_node_or_null("Member_%02d" % (index + 1)) as Node3D
	if member == null:
		return null
	var faction := _faction_of(unit_id)
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
	var s := SoldierState.new()
	s.node = member
	s.player = player
	s.faction = faction
	s.seed = seed
	s.phase = key % MID_IDLE_STEP
	s.meshes = meshes
	s.details = details
	_soldiers[key] = s
	if not _by_unit.has(unit_id):
		_by_unit[unit_id] = [key]
	else:
		(_by_unit[unit_id] as Array).append(key)
	return s


func _animate(s: SoldierState, dead: bool, moving: bool, speed: float, stance: int, now: int,
		cover := -1, tactics := 0) -> void:
	var player := s.player
	if player == null or s.dead:
		return
	var clip: String
	var scale := TIME_FACTOR
	var idle := false
	if dead:
		s.dead = true
		clip = DEATH_CLIPS[s.seed % DEATH_CLIPS.size()]
	elif (s.last_stance != stance and s.last_stance >= 0 or s.clip == "dive") \
			and motion.one_shot(s, stance, moving, now):
		s.is_idle = false
		return
	elif moving:
		clip = motion.moving_clip(s, stance, speed)
		if clip.is_empty():
			clip = "crawl" if stance == 2 else ("run" if speed >= RUN_FROM else "walk")
		scale = speed / maxf(_authored_speed(clip), 0.01)
	else:
		var since := now - s.fired
		var clamped := clampi(stance, 0, 2)
		var phase := 2 if since < FIRE_MS else (1 if since < AIM_MS else 0)
		if phase == 2:
			clip = "fire_" + STANCE_SUFFIX[clamped]
		elif phase == 1:
			clip = "aim_" + STANCE_SUFFIX[clamped]
		else:
			clip = IDLE_CLIPS[clamped]
		if tactics & JAMMED:
			clip = RELOAD_CLIPS[clamped]
		elif cover >= 0 or phase == 0:
			clip = motion.standing_clip(s, stance, phase, cover, clip)
			idle = phase == 0 and clip != "turn_in_place"
	s.last_stance = stance
	s.is_idle = idle
	if absf(scale - s.last_speed_scale) > 1e-4:
		player.speed_scale = scale
		s.last_speed_scale = scale
	if clip != s.clip and player.has_animation(clip):
		s.clip = clip
		player.play(clip, BLEND_SECONDS)


## Animation LOD by camera distance (prompts 016, 206).
func _advance(s: SoldierState, delta: float, dist_sq: float, near_sq: float,
		mid_sq: float, detail_sq: float) -> void:
	var player := s.player
	if player == null:
		return
	var band := 0 if dist_sq < near_sq else (1 if dist_sq < mid_sq else 2)
	counts[band] += 1
	var shadows := band == 0
	if shadows != s.shadows:
		s.shadows = shadows
		var setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows \
				else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		for mesh: GeometryInstance3D in s.meshes:
			mesh.cast_shadow = setting
	var far_detail := dist_sq >= detail_sq
	if far_detail != s.far:
		s.far = far_detail
		for mesh: GeometryInstance3D in s.details:
			mesh.visible = not far_detail and not Helmets.thrown(mesh)
	# A frozen ragdoll keeps its pose (prompt 048); only dead/down soldiers can be frozen.
	if (s.dead or s.down) and s.node.has_meta(&"frozen_pose"):
		s.accum = 0.0
		return
	# Dead soldier whose death clip finished needs no further skeleton evaluation.
	if s.dead and not player.is_playing():
		s.accum = 0.0
		return
	s.accum += delta
	# Far squads are only drawn every FAR_SQUAD_STEP frames (`_skip_squad`);
	# their soldiers advance whenever drawn. Calm idles advance every
	# NEAR_IDLE_STEP / MID_IDLE_STEP frames (prompt 206).
	var step := (MID_IDLE_STEP if s.is_idle else MID_STEP) if band == 1 \
			else (NEAR_IDLE_STEP if band == 0 and s.is_idle else 1)
	if step == 1 or (_frame + s.phase) % step == 0:
		player.advance(s.accum)
		s.accum = 0.0
