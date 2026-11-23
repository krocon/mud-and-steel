extends Node3D
## Boats (prompt 119). Presentation only: the simulation decides where a
## boat drives, where it grounds, when its ramp is down and when it sinks;
## this script
##
## - lowers and raises the bow ramp of landing craft (`Ramp`, hinged at its
##   bottom edge) as `SimHost.boats()` reports it;
## - rocks boats on the water a little (never aground);
## - draws a bow wave and a wake of foam patches behind moving boats;
## - lets a destroyed boat settle by the stern and go down.
##
## Created by `main.gd`.

const REFRESH := 0.1
## How far a ramp swings down when open (degrees).
const RAMP_OPEN_DEG := 72.0
## Rocking: amplitude (degrees) and period (s) of roll and pitch.
const ROLL_DEG := 1.6
const PITCH_DEG := 0.8
const ROLL_PERIOD := 4.2
const PITCH_PERIOD := 3.1
## Wake: a foam patch every this many meters of travel, alive this long.
const WAKE_STEP_M := 2.5
const WAKE_SECONDS := 6.0
const MAX_WAKE := 160
## Sinking: how deep a destroyed boat goes (m) and how long it takes (s).
const SINK_DEPTH_M := 3.0
const SINK_SECONDS := 8.0
const FOAM := Color(0.92, 0.95, 0.96, 0.55)
## Share of the draft the model is drawn higher than its keel lies: the
## well floor of a landing craft lies below the water line, and the water
## surface would show inside the boat [provisional].
const LIFT_SHARE := 0.6

var main: Node3D
var sim_host: SimHost
## Counters for the self-test.
var ramps_moved := 0
var wake_drawn := 0
var sunk := 0

var _refresh_left := 0.0
## Unit id -> the latest boat view of `SimHost.boats()`.
var _boats: Dictionary[int, Dictionary] = {}
## Unit id -> ramp opening shown (0..1).
var _ramp_shown: Dictionary[int, float] = {}
## Unit id -> position of the last wake patch.
var _last_wake: Dictionary[int, Vector3] = {}
var _wake: Array[Dictionary] = []
## Unit id -> seconds since it went down.
var _sinking: Dictionary[int, float] = {}
var _foam_mesh: QuadMesh
var _foam_material: StandardMaterial3D
var _time := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_foam_material = StandardMaterial3D.new()
	_foam_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_foam_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_foam_material.albedo_color = FOAM
	_foam_material.cull_mode = BaseMaterial3D.CULL_DISABLED
	_foam_mesh = QuadMesh.new()
	_foam_mesh.size = Vector2(1.0, 1.0)
	_foam_mesh.orientation = PlaneMesh.FACE_Y
	_foam_mesh.material = _foam_material


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_time += delta
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh()
	for id: int in _boats:
		_animate(id, _boats[id], delta)
	_update_wake(delta)
	_update_sinking(delta)


func _refresh() -> void:
	var seen := {}
	for b: Dictionary in sim_host.boats():
		var id := int(b["unit"])
		seen[id] = true
		_boats[id] = b
	for id: int in _boats.keys():
		if not seen.has(id):
			_boats.erase(id)
			_ramp_shown.erase(id)
			_last_wake.erase(id)
			# Gone from the list: destroyed (the wreck node stays).
			if main._unit_nodes.has(id) and not _sinking.has(id):
				_sinking[id] = 0.0
				sunk += 1


func _model(id: int) -> Node3D:
	var node: Node3D = main._unit_nodes.get(id)
	if node == null:
		return null
	return node.get_node_or_null("Model") as Node3D


func _animate(id: int, b: Dictionary, delta: float) -> void:
	var model := _model(id)
	if model == null:
		return
	var phase: String = b["phase"]
	if not model.has_meta(&"lifted"):
		model.set_meta(&"lifted", true)
		model.position.y += float(b.get("draft", 0.0)) * LIFT_SHARE
	# The ramp follows the simulation, smoothed.
	var ramp := model.find_child("Ramp", true, false) as Node3D
	if ramp != null:
		if not ramp.has_meta(&"rest"):
			ramp.set_meta(&"rest", ramp.transform)
		var want: float = b["ramp"]
		var shown: float = _ramp_shown.get(id, 0.0)
		var next := move_toward(shown, want, delta * 1.5)
		if not is_equal_approx(next, shown) or not _ramp_shown.has(id):
			_ramp_shown[id] = next
			var rest: Transform3D = ramp.get_meta(&"rest")
			# Front is -Z: the top of the ramp swings forward and down.
			ramp.transform.origin = rest.origin
			ramp.transform.basis = Basis(Vector3.RIGHT, -deg_to_rad(RAMP_OPEN_DEG) * next) * rest.basis
			ramps_moved += 1
	# Rocking on the water, still aground.
	var afloat := phase == "afloat" or phase == "landing"
	var phase_offset := float(id % 7)
	var roll := deg_to_rad(ROLL_DEG) * sin(TAU * _time / ROLL_PERIOD + phase_offset) if afloat else 0.0
	var pitch := deg_to_rad(PITCH_DEG) * sin(TAU * _time / PITCH_PERIOD + phase_offset * 1.7) if afloat else 0.0
	model.rotation = Vector3(lerpf(model.rotation.x, pitch, 0.1), model.rotation.y, lerpf(model.rotation.z, roll, 0.1))
	# Wake behind a boat under way.
	var node: Node3D = main._unit_nodes.get(id)
	if not node.visible or not (afloat or phase == "run_in" or phase == "retract"):
		_last_wake.erase(id)
		return
	var draft: float = b.get("draft", 0.5)
	var at := node.global_position + Vector3(0.0, draft + 0.04, 0.0)
	# (the water line: keel plus draft; the model is drawn higher)
	var last: Vector3 = _last_wake.get(id, Vector3.INF)
	if last == Vector3.INF:
		_last_wake[id] = at
	elif at.distance_to(last) >= WAKE_STEP_M:
		var forward := -node.global_basis.z
		var length := (node.get_meta(&"length_m", 0.0) as float)
		if length <= 0.0:
			length = _length_of(node)
			node.set_meta(&"length_m", length)
		_spawn_foam(at - forward * length * 0.5, 2.0, forward)
		_spawn_foam(at + forward * length * 0.45, 1.2, forward)
		_last_wake[id] = at


func _length_of(node: Node3D) -> float:
	var model := node.get_node_or_null("Model") as Node3D
	if model == null:
		return 6.0
	var size := 0.0
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := child as MeshInstance3D
		if mesh.mesh != null:
			size = maxf(size, mesh.mesh.get_aabb().size.z)
	return clampf(size, 3.0, 20.0)


func _spawn_foam(at: Vector3, width: float, forward: Vector3) -> void:
	if _wake.size() >= MAX_WAKE:
		var old: Dictionary = _wake.pop_front()
		(old["node"] as Node3D).queue_free()
	var patch := MeshInstance3D.new()
	patch.mesh = _foam_mesh
	patch.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(patch)
	patch.global_position = at
	patch.rotation.y = atan2(-forward.x, -forward.z)
	patch.scale = Vector3(width, 1.0, width * 0.7)
	_wake.append({"node": patch, "age": 0.0, "width": width})
	wake_drawn += 1


func _update_wake(delta: float) -> void:
	var i := 0
	while i < _wake.size():
		var w: Dictionary = _wake[i]
		w["age"] = float(w["age"]) + delta
		var t: float = w["age"] / WAKE_SECONDS
		var patch: MeshInstance3D = w["node"]
		if t >= 1.0:
			patch.queue_free()
			_wake.remove_at(i)
			continue
		# The patch spreads and fades.
		var width: float = w["width"] * (1.0 + 2.5 * t)
		patch.scale = Vector3(width, 1.0, width * 0.7)
		patch.transparency = t
		i += 1


func _update_sinking(delta: float) -> void:
	for id: int in _sinking.keys():
		var model := _model(id)
		if model == null:
			_sinking.erase(id)
			continue
		var before: float = _sinking[id]
		var age := before + delta
		_sinking[id] = age
		var t := clampf(age / SINK_SECONDS, 0.0, 1.0)
		var step := (_ease(t) - _ease(clampf(before / SINK_SECONDS, 0.0, 1.0))) * SINK_DEPTH_M
		model.position.y -= step
		# By the stern first, then she settles.
		model.rotation.x = lerpf(0.0, deg_to_rad(-9.0), _ease(minf(1.0, t * 2.0))) * (1.0 - t * 0.5)
		if t >= 1.0:
			_sinking.erase(id)


static func _ease(t: float) -> float:
	return t * t * (3.0 - 2.0 * t)
