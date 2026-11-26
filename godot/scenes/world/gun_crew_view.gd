extends Node3D
## Gun crews as figures (prompt 193). Presentation only: since prompt 177
## every towed gun and heavy mortar has a crew as a count in the simulation
## (catalog `crew`, `[gun_crew]` in data/combat.toml); this script stands
## one soldier at each `Socket_Crew_NN` of the model for every man left
## (`SimHost.unit_crews`) and picks his clip. Nothing is read back.
##
## - **Places and roles:** places are taken in PRIORITY order of the model's
##   sockets: the layer (Richtschütze, socket 01, `gun_aim`), the loader
##   (socket 02, `gun_load`), the ammunition bearer (socket 03, `gun_load`
##   half a cycle later), the gun commander (the last socket, standing,
##   binoculars on the chest), the rest kneel at the ammunition. Models per
##   role and faction: ROLE_MODELS; uniform and camouflage like the
##   infantry (`SoldierLook`, faction, biome, the unit's rank).
## - **The count:** when the simulation's count drops, one figure falls
##   (ragdoll within the infantry's budget, else a death clip; the body
##   stays where it fell, out of the gun's node). Which one is fixed by the
##   unit id and the count (`KillSequence.roll`), never the layer while
##   another man is left; the others then move up to the important places
##   and walk there. A destroyed gun takes all its men with it; an
##   abandoned gun (0 men) has nobody at it.
## - **Work:** the loader loads in the rhythm of the reload (the clip ends
##   when the simulation's next round is due), the layer cranks while the
##   gun is in action or turning, the others push the trail while the gun
##   swings round. A shot makes everyone but the loader duck (DUCK_S; big
##   calibers longer, no own clip for covering the ears). Pinned down: the
##   layer and the loader crouch behind the shield (`cover_low`), the others
##   lie flat; the loader only loads in the last stretch before the next
##   round, which the simulation already delays.
## - **March:** while towed the men ride the tractor and are not drawn;
##   after unlimbering they walk from the tractor to their places.
## - **Sight:** the figures are children of the gun's model, so they turn
##   with gun and shield and hide with the unit in the fog of war.
## - **Cost:** animation players in manual mode like the infantry: within
##   NEAR_M every frame with shadows, up to MID_M every MID_STEP frames,
##   up to STATIC_M frozen in their pose, beyond not drawn (a man is under
##   10 px there; no MultiMesh impostors [provisional]).

const KillSequence := preload("res://scenes/world/kill_sequence.gd")
const RtsCamera := preload("res://scenes/camera/rts_camera.gd")
const Ragdolls := preload("res://scenes/world/ragdolls.gd")
const MODELS_DIR := "res://assets/models/"
## Socket order (0-based socket index; -1 = the last socket) by importance:
## layer, loader, ammunition bearer, commander, then the rest in order.
const LAYER := 0
const LOADER := 1
const BEARER := 2
const COMMANDER := 3
## Soldier model per role and faction (`<faction>/infantry/<model>.glb`);
## the first one that exists is used.
const ROLE_MODELS: Dictionary[String, Dictionary] = {
	"layer": {"de": ["rifleman"], "su": ["carbineer", "rifleman"], "us": ["carbineer", "rifleman"], "uk": ["rifleman"]},
	"loader": {"de": ["ammo_carrier"], "su": ["carbineer", "rifleman"], "us": ["ammo_carrier"], "uk": ["ammo_carrier", "rifleman"]},
	"bearer": {"de": ["ammo_carrier"], "su": ["rifleman"], "us": ["ammo_carrier"], "uk": ["ammo_carrier", "rifleman"]},
	"commander": {"de": ["squad_leader"], "su": ["scout"], "us": ["squad_leader"], "uk": ["squad_leader"]},
	"rest": {"de": ["rifleman"], "su": ["carbineer", "rifleman"], "us": ["carbineer", "rifleman"], "uk": ["rifleman"]},
}
const NEAR_M := 70.0
## Distance (m) beyond which Gear and Weapon detail meshes hide (prompt 206, provisional).
const DETAIL_M := 150.0
const MID_M := 260.0
const MID_STEP := 3
const NEAR_IDLE_STEP := 3
const STATIC_M := 450.0
## Guns farther than this outside the camera frustum skip animation updates (prompt 206).
const CULL_MARGIN_M := 25.0
const PRUNE_INTERVAL := 240
## Non-locomotion clips are authored in real time; the game runs at factor 2
## (as `infantry_view.gd`).
const TIME_FACTOR := 2.0
const BLEND_SECONDS := 0.2
## Seconds after a shot during which the gun counts as in action.
const ACTION_S := 20.0
const ACTION_MS := int(ACTION_S * 1000.0)
## Ducking after a shot (seconds), longer from BIG_CALIBER_MM.
const DUCK_S := 0.5
const BIG_DUCK_S := 0.9
const DUCK_MS := int(DUCK_S * 1000.0)
const BIG_DUCK_MS := int(BIG_DUCK_S * 1000.0)
const BIG_CALIBER_MM := 75.0
## The gun swings round (rad/s of its node): the trail party pushes.
const SWING_RATE := 0.25
## Walking speed of the men (m/s game speed: 1.4 m/s real at factor 2).
const WALK_SPEED := 2.8
## Below this distance (m) a walking man has arrived.
const ARRIVE_M := 0.08
## Bodies left on the field at most; the oldest go first.
const MAX_FALLEN := 300
## Death clips (prompt 206: avoid per-fall array literal).
const DEATH_CLIPS: Array[String] = ["death_a", "death_b", "death_c"]
## Thumbnail poses (clip, seconds into it) per role.
const STILL_POSES: Dictionary[String, Array] = {
	"layer": ["gun_aim", 0.3], "loader": ["gun_load", 1.6], "bearer": ["gun_load", 0.6],
	"commander": ["idle", 0.5], "rest": ["kneel", 0.4],
}


## Typed per-figure state (prompt 206).
class FigureState:
	var node: Node3D
	var player: AnimationPlayer
	var place := 0
	var role := ""
	var seed := 0
	var clip := ""
	var walk_to := Vector3.INF
	var accum := 0.0
	var shadows := false
	var far := false
	var meshes: Array[GeometryInstance3D] = []
	var details: Array[GeometryInstance3D] = []


## Typed per-gun crew state (prompt 206).
class CrewState:
	var figures: Array[FigureState] = []
	var slots: Array = []
	var facing: Array = []
	var men := 0
	var towed := false
	var fired := -1000000
	var caliber := 0.0
	var yaw := 0.0
	var dead := false
	var faction := ""
	var rank := 0
	var id := 0
	var next_seed := 0
	var model: Node3D
	var has_tractor_at := false
	var tractor_at := Vector3.ZERO


var main: Node3D
var sim_host: SimHost
## Off: no figures drawn (benchmark A/B, `--perf-battery`).
var enabled := true:
	set(value):
		enabled = value
		for crew: CrewState in _crews.values():
			for f: FigureState in crew.figures:
				f.node.visible = value and not crew.towed
## Figures drawn in the last frame per band (near, mid, static, hidden).
var counts := [0, 0, 0, 0]
## Figures that fell since the start (self-test).
var fallen_total := 0
## Time of the last `_process` in microseconds (benchmark).
var process_usec := 0
## Per gun id: typed CrewState (prompt 206).
var _crews: Dictionary[int, CrewState] = {}
var _fallen: Array[Node3D] = []
var _fallen_root: Node3D
var _frame := 0
var _biome := ""
## Pre-cached `gun_load` authored length and game duration (prompt 206).
var _load_length := -1.0
var _load_seconds := -1.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_fallen_root = Node3D.new()
	_fallen_root.name = "Fallen"
	add_child(_fallen_root)
	main.combat_view.combat_event.connect(_on_combat_event)


## Living figures at gun `id` (self-test).
func figures_at(id: int) -> int:
	var crew: CrewState = _crews.get(id)
	return crew.figures.size() if crew != null else 0


## Living figures at gun `id` that are drawn (self-test).
func drawn_at(id: int) -> int:
	var crew: CrewState = _crews.get(id)
	if crew == null:
		return 0
	var n := 0
	for f: FigureState in crew.figures:
		if f.node.is_visible_in_tree():
			n += 1
	return n


## Figure states at gun `id`, by place (self-test, shots).
func figures_of(id: int) -> Array:
	var crew: CrewState = _crews.get(id)
	return crew.figures if crew != null else []


func _on_combat_event(event: Dictionary) -> void:
	if event["type"] != "shot":
		return
	var crew: CrewState = _crews.get(int(event["unit"]))
	if crew == null:
		return
	crew.fired = Time.get_ticks_msec()
	crew.caliber = float(event.get("caliber_mm", 0.0))


func _process(delta: float) -> void:
	if sim_host == null or main == null:
		return
	var began := Time.get_ticks_usec()
	_draw(delta)
	process_usec = Time.get_ticks_usec() - began


func _ensure_load_length() -> void:
	if _load_length < 0.0:
		var clips: Dictionary = SoldierLook.clips()
		var entry: Variant = clips.get("gun_load")
		_load_length = float(entry.get("length", 3.0)) if entry is Dictionary else 3.0
		_load_seconds = _load_length / TIME_FACTOR


func _draw(delta: float) -> void:
	_frame += 1
	if _frame % PRUNE_INTERVAL == 0:
		prune_caches()
	counts = [0, 0, 0, 0]
	var crews := sim_host.unit_crews()
	if crews.is_empty():
		return
	_ensure_load_length()
	var ids := sim_host.unit_ids()
	var carriers := sim_host.unit_carriers()
	var alive := sim_host.unit_alive()
	var moving := sim_host.unit_moving()
	var types := sim_host.unit_type_ids()
	var ranks := sim_host.unit_ranks()
	var tick_rate := float(maxi(sim_host.tick_rate(), 1))
	var biome: String = main.atmosphere.preset.camo_biome if main.atmosphere != null else "forest"
	var camera: Camera3D = main.camera
	var camera_position: Vector3 = camera.global_position
	var frustum := camera.get_frustum()
	var inv_factor := 1.0 / maxf(RtsCamera.detail_distance_factor(camera), 1e-4)
	var near_sq := (NEAR_M * inv_factor) * (NEAR_M * inv_factor)
	var detail_sq := (DETAIL_M * inv_factor) * (DETAIL_M * inv_factor)
	var mid_sq := (MID_M * inv_factor) * (MID_M * inv_factor)
	var static_sq := (STATIC_M * inv_factor) * (STATIC_M * inv_factor)
	var now := Time.get_ticks_msec()
	var inv_delta := 1.0 / maxf(delta, 0.001)
	if biome != _biome:
		_biome = biome
		for crew: CrewState in _crews.values():
			for f: FigureState in crew.figures:
				SoldierLook.apply(f.node, crew.faction, _biome, f.seed, "auto", crew.rank)
	var unit_nodes: Dictionary = main._unit_nodes
	var ranks_size := ranks.size()
	for i in ids.size():
		var men := crews[i * 4]
		if men < 0:
			continue
		var id := ids[i]
		var node: Node3D = unit_nodes.get(id)
		if node == null:
			continue
		var crew: CrewState = _crews.get(id)
		var model: Node3D
		if crew == null:
			model = node.get_node_or_null("Model") as Node3D
			if model == null:
				continue
			crew = _new_crew(id, model, (types[i] as String).get_slice("_", 0), ranks[i] if i < ranks_size else 0, men)
			if crew == null:
				continue
			_crews[id] = crew
		else:
			model = crew.model
			if not is_instance_valid(model):
				continue
		if crew.dead:
			continue
		if alive[i] == 0:
			# The gun is destroyed: its men fall with it.
			while not crew.figures.is_empty():
				_fall(crew, crew.figures.size() - 1, camera_position, true)
			crew.dead = true
			continue
		var towed := carriers[i] >= 0
		if towed != crew.towed:
			_set_towed(crew, towed, unit_nodes.get(carriers[i]) if towed else null, model)
		_follow_count(crew, id, men, model, camera_position)
		if towed or not enabled or not node.visible:
			continue
		# Frustum culling (prompt 206): guns off screen skip pose/animation work.
		var pos := model.global_position
		var culled := false
		for plane in frustum:
			if plane.distance_to(pos) > CULL_MARGIN_M:
				culled = true
				break
		if culled:
			for f: FigureState in crew.figures:
				f.accum += delta
				if f.walk_to != Vector3.INF:
					var here := f.node.position
					var to_go := Vector3(f.walk_to.x - here.x, 0.0, f.walk_to.z - here.z)
					var step := WALK_SPEED * delta
					if to_go.length() <= maxf(step, ARRIVE_M):
						f.node.transform = place_transform(crew, f.place)
						f.walk_to = Vector3.INF
					else:
						f.node.position = here + to_go.normalized() * step
			continue
		var dist_sq := pos.distance_squared_to(camera_position)
		var band := 0 if dist_sq < near_sq else (1 if dist_sq < mid_sq else (2 if dist_sq < static_sq else 3))
		var far_detail := dist_sq >= detail_sq
		var yaw := node.rotation.y
		var swing := absf(angle_difference(crew.yaw, yaw)) * inv_delta > SWING_RATE
		crew.yaw = yaw
		var pinned := (crews[i * 4 + 2] & 1) != 0
		var reload_ticks := crews[i * 4 + 3]
		var reload_s := float(reload_ticks) / tick_rate
		var since_fire := now - crew.fired
		var action := since_fire < ACTION_MS or reload_ticks > 0
		var duck := since_fire < (BIG_DUCK_MS if crew.caliber >= BIG_CALIBER_MM else DUCK_MS)
		var is_moving := moving[i] != 0
		for f: FigureState in crew.figures:
			_animate(f, crew, pinned, reload_s, action, duck, swing, is_moving, band, far_detail, delta)


## Removes cached crews for units whose nodes were removed or destroyed (prompt 206).
func prune_caches() -> void:
	if main == null:
		return
	var unit_nodes: Dictionary = main._unit_nodes
	for id: int in _crews.keys():
		var crew: CrewState = _crews[id]
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node) or (crew.dead and crew.figures.is_empty()):
			_crews.erase(id)


## A new crew for gun `id`: its places from the model's sockets and one
## figure per man.
func _new_crew(id: int, model: Node3D, faction: String, rank: int, men: int) -> CrewState:
	var slots := slots_of(model)
	if slots.is_empty():
		return null
	var crew := CrewState.new()
	crew.slots = slots
	crew.facing = facings_of(slots)
	crew.faction = faction
	crew.rank = rank
	crew.id = id
	crew.model = model
	for k in mini(men, slots.size()):
		_add_figure(crew, model, k)
	crew.men = men
	return crew


func _add_figure(crew: CrewState, model: Node3D, place: int) -> void:
	var seed := crew.id * 64 + crew.next_seed
	crew.next_seed += 1
	var role := role_of_place(place, crew.slots.size())
	var figure := make_figure(crew.faction, role, _biome if not _biome.is_empty() else "forest", seed, crew.rank)
	if figure == null:
		return
	model.add_child(figure)
	figure.transform = place_transform(crew, place)
	var meshes: Array[GeometryInstance3D] = []
	var details: Array[GeometryInstance3D] = []
	for mesh: GeometryInstance3D in figure.find_children("*", "GeometryInstance3D", true, false):
		meshes.append(mesh)
		# Shadows off until band == 0 (matches infantry_view.gd, prompt 206).
		mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if mesh.name != "Body":
			details.append(mesh)
	var f := FigureState.new()
	f.node = figure
	f.player = SoldierLook.find_player(figure)
	f.place = place
	f.role = role
	f.seed = seed
	f.meshes = meshes
	f.details = details
	crew.figures.append(f)


## The figure's transform at `place` of `crew` (model space).
func place_transform(crew: CrewState, place: int) -> Transform3D:
	var socket := socket_of_place(place, crew.slots.size())
	var origin: Vector3 = (crew.slots[socket] as Transform3D).origin
	return Transform3D(Basis(Vector3.UP, float(crew.facing[socket])), origin)


## Count changes: men fall or come back.
func _follow_count(crew: CrewState, id: int, men: int, model: Node3D, camera_position: Vector3) -> void:
	var figures := crew.figures
	var had := crew.men
	if men == had:
		return
	crew.men = men
	var size := crew.slots.size()
	if men < had:
		for n in range(had - 1, men - 1, -1):
			if figures.is_empty():
				break
			# Count after this loss = n: who falls is fixed by unit and count;
			# the layer only as the last man.
			var k := 0
			if figures.size() > 1:
				k = 1 + int(KillSequence.roll(id, n, 193) * (figures.size() - 1))
			_fall(crew, k, camera_position, false)
		# The others move up to the important places.
		for k in figures.size():
			var f := figures[k]
			if f.place != k:
				f.place = k
				f.walk_to = place_transform(crew, k).origin
	else:
		while figures.size() < mini(men, size):
			_add_figure(crew, model, figures.size())
			if crew.towed:
				figures.back().node.visible = false


## Figure `k` of `crew` falls; its body stays on the field.
func _fall(crew: CrewState, k: int, camera_position: Vector3, blast: bool) -> void:
	var figures := crew.figures
	var f := figures[k]
	figures.remove_at(k)
	fallen_total += 1
	var figure := f.node
	if not figure.is_visible_in_tree() or crew.towed:
		figure.queue_free()
		return
	var at := figure.global_transform
	figure.get_parent().remove_child(figure)
	_fallen_root.add_child(figure)
	figure.global_transform = at
	for mesh: GeometryInstance3D in f.meshes:
		mesh.material_overlay = null
	var player := f.player
	var clip := DEATH_CLIPS[f.seed % 3]
	if player != null and player.has_animation(clip):
		player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_IDLE
		player.speed_scale = TIME_FACTOR
		player.play(clip, BLEND_SECONDS)
	var infantry: Node = main.infantry_view
	var ragdolls: Ragdolls = infantry.motion.ragdolls if infantry != null and infantry.get("motion") != null else null
	if ragdolls != null and ragdolls.can_start(at.origin, camera_position):
		var away := -at.basis.z
		away.y = 0.0
		away = away.normalized() if away.length() > 0.01 else Vector3.BACK
		var velocity := away * (2.4 if blast else 1.2) + Vector3.UP * (3.0 if blast else 0.3)
		ragdolls.start(figure, velocity, Vector3.UP.cross(away) * 2.5, at.origin.y, "dead")
	_fallen.append(figure)
	while _fallen.size() > MAX_FALLEN:
		var old: Node3D = _fallen.pop_front()
		if is_instance_valid(old):
			old.queue_free()


## Hitched: the men ride and are not drawn. Unhitched: they climb off at
## the tractor and walk to their places.
func _set_towed(crew: CrewState, towed: bool, tractor: Node3D, model: Node3D) -> void:
	crew.towed = towed
	var start := Vector3.INF
	if not towed and crew.has_tractor_at:
		start = model.global_transform.affine_inverse() * crew.tractor_at
	if towed and tractor != null:
		crew.tractor_at = tractor.global_position
		crew.has_tractor_at = true
	for k in crew.figures.size():
		var f := crew.figures[k]
		var figure := f.node
		figure.visible = not towed and enabled
		if towed:
			figure.transform = place_transform(crew, f.place)
			f.walk_to = Vector3.INF
		elif start != Vector3.INF:
			var to := place_transform(crew, f.place).origin
			# A little apart, not all from one point.
			var side := Vector3(float(k % 3 - 1) * 0.6, 0.0, float(k / 3) * 0.6)
			figure.transform = Transform3D(figure.transform.basis, start + side)
			f.walk_to = to


## Poses and moves one figure.
func _animate(f: FigureState, crew: CrewState, pinned: bool, reload_s: float, action: bool,
		duck: bool, swing: bool, is_moving: bool, band: int, far_detail: bool, delta: float) -> void:
	var figure := f.node
	counts[band] += 1
	var show := band < 3
	if figure.visible != show:
		figure.visible = show
	if not show:
		return
	var shadows := band == 0
	if shadows != f.shadows:
		f.shadows = shadows
		var setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows \
				else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		for mesh: GeometryInstance3D in f.meshes:
			mesh.cast_shadow = setting
	if far_detail != f.far:
		f.far = far_detail
		for mesh: GeometryInstance3D in f.details:
			mesh.visible = not far_detail
	var clip := ""
	var speed := TIME_FACTOR
	var walk_to := f.walk_to
	if walk_to != Vector3.INF:
		var here := figure.position
		var to_go := Vector3(walk_to.x - here.x, 0.0, walk_to.z - here.z)
		var step := WALK_SPEED * delta
		if to_go.length() <= maxf(step, ARRIVE_M):
			figure.transform = place_transform(crew, f.place)
			f.walk_to = Vector3.INF
		else:
			var dir := to_go.normalized()
			figure.position = here + dir * step
			figure.basis = Basis(Vector3.UP, atan2(-dir.x, -dir.z))
			clip = "walk"
			speed = WALK_SPEED / 1.4
	if clip.is_empty():
		clip = _work_clip(f, pinned, reload_s, action, duck, swing, is_moving)
		if clip == "gun_load":
			speed = _load_speed(f, reload_s)
	_play(f, clip, speed, band, delta)


## The clip of a man at his place.
func _work_clip(f: FigureState, pinned: bool, reload_s: float, action: bool,
		duck: bool, swing: bool, is_moving: bool) -> String:
	var role := f.role
	var load_s := _load_seconds
	if pinned:
		if role == "loader" and reload_s > 0.0 and reload_s <= load_s:
			return "gun_load"
		return "cover_low" if role == "layer" or role == "loader" else "prone"
	if is_moving:
		return "walk"
	if duck and role != "loader":
		return "crouch"
	if swing and role != "layer" and role != "commander":
		return "walk"
	if not action:
		return "idle" if role == "commander" else "kneel"
	match role:
		"layer":
			return "gun_aim"
		"loader", "bearer":
			# Fast guns (light flak) load all the time; the others in the last
			# stretch before the next round, after that they wait.
			if reload_s <= 0.0 or reload_s > load_s * 1.5:
				return "kneel" if role == "bearer" else "crouch"
			return "gun_load"
		"commander":
			return "idle"
	return "kneel"


## Play speed of `gun_load` so the round is in when the gun fires.
func _load_speed(f: FigureState, left: float) -> float:
	var player := f.player
	if left <= 0.05 or player == null or f.clip != "gun_load":
		return TIME_FACTOR
	var remaining := _load_length - fmod(player.current_animation_position, _load_length)
	# Faster than real time only, never slow motion below the game's pace.
	return clampf(remaining / left, TIME_FACTOR, TIME_FACTOR * 3.0)


func _play(f: FigureState, clip: String, speed: float, band: int, delta: float) -> void:
	var player := f.player
	if player == null:
		return
	if clip != f.clip and player.has_animation(clip):
		f.clip = clip
		player.play(clip, BLEND_SECONDS if band == 0 else 0.0)
		if clip == "gun_load" and f.role == "bearer":
			# Half a cycle behind the loader.
			player.seek(_load_length * 0.5, true)
	f.accum += delta
	if band == 2:
		f.accum = 0.0
		if player.current_animation_position == 0.0:
			player.advance(0.0)
		return
	var is_idle := clip == "idle" or clip == "kneel"
	var step := MID_STEP if band == 1 else (NEAR_IDLE_STEP if band == 0 and is_idle else 1)
	if step == 1 or (_frame + f.seed) % step == 0:
		player.advance(f.accum * speed)
		f.accum = 0.0


# --- Shared with the thumbnails and the unit reference -----------------------------

## Places of a gun model: the transforms of its `Socket_Crew_NN` (model
## space), in socket order.
static func slots_of(model: Node3D) -> Array:
	var sockets: Array[Node3D] = []
	for child in model.find_children("Socket_Crew_*", "Node3D", true, false):
		sockets.append(child as Node3D)
	sockets.sort_custom(func(a: Node3D, b: Node3D) -> bool: return a.name < b.name)
	var result: Array = []
	for socket in sockets:
		result.append(_local(model, socket))
	return result


## The transform of `node` relative to `model` (also outside the tree).
static func _local(model: Node3D, node: Node3D) -> Transform3D:
	var t := Transform3D.IDENTITY
	var n: Node = node
	while n != null and n != model:
		if n is Node3D:
			t = (n as Node3D).transform * t
		n = n.get_parent()
	return t


## Yaw of the man at every place: on an arc around the mount (flak,
## mortars: socket 01 far out to the side) all face its center; behind a
## field gun's shield the first two face the breech, the others the front.
static func facings_of(slots: Array) -> Array:
	var result: Array = []
	if slots.is_empty():
		return result
	var arc := absf((slots[0] as Transform3D).origin.x) > 1.0
	for k in slots.size():
		var p: Vector3 = (slots[k] as Transform3D).origin
		var target := Vector3.ZERO if arc else Vector3(0.0, 0.0, p.z - 0.35)
		if not arc and k >= 2:
			target = p + Vector3.FORWARD
		var dir := target - p
		dir.y = 0.0
		result.append(atan2(-dir.x, -dir.z) if dir.length() > 0.01 else 0.0)
	return result


## Socket index of `place` (0 = most important) on a gun with `size` sockets.
static func socket_of_place(place: int, size: int) -> int:
	var order := place_order(size)
	return order[place] if place < order.size() else place


## Socket indices by importance: layer 01, loader 02, bearer 03, commander
## the last socket, then the rest.
static func place_order(size: int) -> Array[int]:
	var order: Array[int] = []
	for k in [LAYER, LOADER, BEARER]:
		if k < size:
			order.append(k)
	if size > 3:
		order.append(size - 1)
	for k in range(3, size - 1):
		order.append(k)
	return order


## Role at `place` of a gun with `size` places.
static func role_of_place(place: int, size: int) -> String:
	match place:
		0:
			return "layer"
		1:
			return "loader"
		2:
			return "bearer"
		3:
			return "commander" if size > 3 else "rest"
	return "rest"


## A soldier of `role` for `faction` in `biome`, posed by hand (manual
## animation mode), or null if the faction has no fitting model.
static func make_figure(faction: String, role: String, biome: String, seed: int, rank := 0) -> Node3D:
	var path := ""
	for name: String in (ROLE_MODELS[role] as Dictionary).get(faction, ["rifleman"]):
		var candidate := "%s%s/infantry/%s.glb" % [MODELS_DIR, faction, name]
		if ResourceLoader.exists(candidate):
			path = candidate
			break
	if path.is_empty():
		return null
	var figure := (load(path) as PackedScene).instantiate() as Node3D
	figure.name = "Crew_" + role
	SoldierLook.apply(figure, faction, biome, seed, "auto", rank)
	var player := SoldierLook.find_player(figure)
	if player != null:
		SoldierLook.configure_animations(player)
		player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	return figure


## A full crew in still working poses at every `Socket_Crew_NN` of `model`
## (thumbnails, unit reference). Returns the number of figures.
static func add_still_crew(model: Node3D, faction: String, biome: String) -> int:
	var slots := slots_of(model)
	var facing := facings_of(slots)
	for place in slots.size():
		var role := role_of_place(place, slots.size())
		var figure := make_figure(faction, role, biome, 1000 + place)
		if figure == null:
			return place
		model.add_child(figure)
		var socket := socket_of_place(place, slots.size())
		figure.transform = Transform3D(Basis(Vector3.UP, float(facing[socket])), (slots[socket] as Transform3D).origin)
		var player := SoldierLook.find_player(figure)
		var pose: Array = STILL_POSES[role]
		if player != null and player.has_animation(pose[0]):
			# Posed once it is in the tree (the model is built outside it).
			var apply := func() -> void:
				player.play(pose[0])
				player.seek(float(pose[1]), true)
			if figure.is_inside_tree():
				apply.call()
			else:
				figure.tree_entered.connect(apply, CONNECT_ONE_SHOT)
	return slots.size()
