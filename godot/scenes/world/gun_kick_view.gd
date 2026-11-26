extends RefCounted
## Gun recoil, hull rocking and raised indirect barrels (prompts 077, 208).
##
## Extracted from `effects.gd` so the kick/elevation state of `Gun` nodes lives
## in its own self-contained module. Presentation only: unit nodes never move.

## Recoil: the `Gun` node runs back along its axis (m per mm of caliber,
## at most RECOIL_MAX_M), in RECOIL_BACK_S, and forward again in
## RECOIL_RETURN_S.
const RECOIL_M_PER_MM := 0.005
const RECOIL_MAX_M := 0.6
## Heavy artillery (>= 120 mm, e.g. 122 mm M-30, 152 mm ML-20) recoils further
## along its longer cradle (prompt 229).
const RECOIL_HEAVY_CALIBER_MM := 120.0
const RECOIL_HEAVY_MAX_M := 0.78
const RECOIL_BACK_S := 0.05
const RECOIL_RETURN_S := 0.7
## The hull rocks: degrees at scale 1, frequency (Hz), decay time (s).
const ROCK_DEG := 0.8
const ROCK_HZ := 1.6
const ROCK_DECAY_S := 0.35
## A howitzer's barrel stays raised this long after its last round (s),
## or until the gun moves, then sinks at ELEVATE_DEG_PER_S.
const ELEVATION_HOLD_S := 25.0
const ELEVATE_DEG_PER_S := 12.0

## Recoils started since the start (self-test, tools).
var recoils := 0
## Unit id -> its gun's kick state: {gun, rest, model, model_rest, steep,
## brake, recoil, t, rock, axis, elevation, target, last, active}.
var _guns: Dictionary[int, Dictionary] = {}
## Indirect shells whose barrel elevation is not known yet: projectile id -> {unit, t}.
var _pending_elevations: Dictionary[int, Dictionary] = {}
## Projectile and shell velocities of this frame (id -> m/s), read once.
var _velocities: Dictionary[int, Vector3] = {}
var _velocities_frame := -1

var _effects: Node3D


func setup(effects_node: Node3D) -> void:
	_effects = effects_node


## The kick state of a unit's `Gun` node, found once per model ({} when none).
func gun_of(unit: int) -> Dictionary:
	var node: Node3D = _effects.main._unit_nodes.get(unit)
	var model := node.get_node_or_null("Model") as Node3D if node != null else null
	if _guns.has(unit):
		var entry: Dictionary = _guns[unit]
		if entry.model == model and is_instance_valid(entry.gun):
			return entry
		forget_unit(unit)
	if model == null:
		return {}
	var gun := model.find_child("Gun", true, false) as Node3D
	if gun == null:
		return {}
	var tip := Vector3.ZERO
	var socket := gun.get_node_or_null("Socket_Muzzle") as Node3D
	if socket != null:
		tip = socket.position
	var steep := tip.length() > 0.1 and tip.y / tip.length() > 0.5
	var entry := {
		"gun": gun, "rest": gun.transform, "model": model, "model_rest": model.transform,
		"steep": steep, "brake": bool(model.get_meta(&"muzzle_brake", false)),
		"recoil": 0.0, "t": 100.0, "rock": 0.0, "axis": Vector3.RIGHT,
		"elevation": 0.0, "target": 0.0, "last": -100.0, "active": false,
	}
	_guns[unit] = entry
	return entry


## Puts a unit's gun and hull back at rest and drops its state.
func forget_unit(unit: int) -> void:
	if not _guns.has(unit):
		return
	var entry: Dictionary = _guns[unit]
	if is_instance_valid(entry.gun):
		(entry.gun as Node3D).transform = entry.rest
	if is_instance_valid(entry.model):
		(entry.model as Node3D).transform = entry.model_rest
	_guns.erase(unit)


func queue_elevation(projectile: int, unit: int, now: float) -> void:
	_pending_elevations[projectile] = {"unit": unit, "t": now}


## Starts the recoil of the gun and the rocking of the hull.
func kick(entry: Dictionary, caliber: float, muzzle_class: Dictionary, s: float, direction: Vector3) -> void:
	var cap := RECOIL_HEAVY_MAX_M if caliber >= RECOIL_HEAVY_CALIBER_MM else RECOIL_MAX_M
	entry.recoil = minf(caliber * RECOIL_M_PER_MM * float(muzzle_class.recoil), cap)
	entry.rock = ROCK_DEG * s * float(muzzle_class.rock)
	var model: Node3D = entry.model
	var local := model.global_basis.inverse() * direction
	local.y = 0.0
	entry.axis = local.normalized().cross(Vector3.UP) if local.length() > 0.01 else Vector3.RIGHT
	entry.t = 0.0
	entry.active = true
	recoils += 1


## The barrel's elevation from a shell's launch velocity; `snap`: at once.
func raise(entry: Dictionary, velocity: Vector3, snap: bool) -> void:
	var flat := Vector2(velocity.x, velocity.z).length()
	entry.target = clampf(atan2(velocity.y, flat), 0.0, deg_to_rad(80.0))
	if snap:
		entry.elevation = entry.target
	entry.active = true
	_pose(entry)


func update(delta: float, now: float) -> void:
	if not _pending_elevations.is_empty():
		for projectile: int in _pending_elevations.keys():
			var wait: Dictionary = _pending_elevations[projectile]
			var velocity := velocity_of(projectile)
			if velocity != Vector3.ZERO:
				var entry := gun_of(int(wait.unit))
				if not entry.is_empty():
					raise(entry, velocity, false)
				_pending_elevations.erase(projectile)
			elif now - float(wait.t) > 2.0:
				_pending_elevations.erase(projectile)
	for unit: int in _guns.keys():
		var entry: Dictionary = _guns[unit]
		if not entry.active:
			continue
		if not is_instance_valid(entry.gun) or not is_instance_valid(entry.model):
			_guns.erase(unit)
			continue
		entry.t += delta
		if entry.target > 0.0:
			var index: int = _effects._index_of(unit)
			var moving: bool = index >= 0 and _effects.sim_host.unit_moving()[index] != 0
			if moving or now - float(entry.last) > ELEVATION_HOLD_S:
				entry.target = 0.0
		entry.elevation = move_toward(entry.elevation, entry.target, deg_to_rad(ELEVATE_DEG_PER_S) * delta)
		_pose(entry)
		if entry.t > RECOIL_BACK_S + RECOIL_RETURN_S and entry.t > ROCK_DECAY_S * 5.0 \
				and entry.elevation == 0.0 and entry.target == 0.0:
			entry.active = false
			(entry.gun as Node3D).transform = entry.rest
			(entry.model as Node3D).transform = entry.model_rest


func prune(unit_nodes: Dictionary) -> void:
	for id: int in _guns.keys():
		var node: Variant = unit_nodes.get(id)
		if node == null or not is_instance_valid(node):
			_guns.erase(id)


## Sets the gun (elevation, recoil) and the hull (rocking) for the state.
func _pose(entry: Dictionary) -> void:
	var t: float = entry.t
	var back := 0.0
	if t < RECOIL_BACK_S:
		back = t / RECOIL_BACK_S
	elif t < RECOIL_BACK_S + RECOIL_RETURN_S:
		back = 1.0 - smoothstep(0.0, 1.0, (t - RECOIL_BACK_S) / RECOIL_RETURN_S)
	var rest: Transform3D = entry.rest
	var basis := rest.basis * Basis(Vector3.RIGHT, entry.elevation)
	(entry.gun as Node3D).transform = Transform3D(basis, rest.origin + basis * Vector3(0.0, 0.0, back * float(entry.recoil)))
	var angle := deg_to_rad(float(entry.rock)) * exp(-t / ROCK_DECAY_S) * sin(TAU * ROCK_HZ * t)
	var model_rest: Transform3D = entry.model_rest
	(entry.model as Node3D).transform = Transform3D(model_rest.basis * Basis(entry.axis, angle), model_rest.origin)


## Velocity (m/s) of a direct projectile or an indirect shell of this
## frame's snapshot, or ZERO.
func velocity_of(projectile: int) -> Vector3:
	var frame := Engine.get_process_frames()
	if frame != _velocities_frame:
		_velocities_frame = frame
		_velocities.clear()
		var sim_host: SimHost = _effects.sim_host
		var ids := sim_host.projectile_ids()
		var velocities := sim_host.projectile_velocities()
		for i in mini(ids.size(), velocities.size()):
			_velocities[ids[i]] = velocities[i]
		ids = sim_host.shell_ids()
		velocities = sim_host.shell_velocities()
		for i in mini(ids.size(), velocities.size()):
			_velocities[ids[i]] = velocities[i]
	return _velocities.get(projectile, Vector3.ZERO)
