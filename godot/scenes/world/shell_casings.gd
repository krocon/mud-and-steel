extends Node3D
## Spent brass artillery shell casings ejected behind towed guns (prompt 229).
##
## Presentation only: renders up to MAX_CASINGS spent 6-sided brass cylinders
## in a single MultiMeshInstance3D (1 draw call, no shadow casting, disabled on
## graphics quality "low"). Never read by the simulation.

const MAX_CASINGS := 512
## Delay (s) between shot and the spent casing hitting the ground behind the breech.
const EJECT_DELAY_S := 0.4
## Minimum caliber (mm) that ejects visible brass casings.
const MIN_CALIBER_MM := 50.0
## Base dimensions (m) of the unit cylinder mesh (scaled by caliber / 105 mm).
const BASE_LENGTH_M := 0.56
const BASE_RADIUS_M := 0.056

## Total casings placed on the ground (self-test, stats).
var spawned := 0
var enabled := true:
	set(value):
		enabled = value
		if _mmi != null:
			_mmi.visible = value

var _sim_host: SimHost
var _mmi: MultiMeshInstance3D
var _mm: MultiMesh
var _next := 0
var _seq := 0
## Queued ejections waiting for EJECT_DELAY_S: {t, x, z, yaw, caliber, seed}.
var _pending: Array[Dictionary] = []


func setup(sim_host: SimHost) -> void:
	_sim_host = sim_host
	var mesh := CylinderMesh.new()
	mesh.top_radius = BASE_RADIUS_M * 0.92
	mesh.bottom_radius = BASE_RADIUS_M
	mesh.height = BASE_LENGTH_M
	mesh.radial_segments = 6
	mesh.rings = 1
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.84, 0.67, 0.27)
	mat.metallic = 0.75
	mat.roughness = 0.34
	mesh.material = mat
	_mm = MultiMesh.new()
	_mm.transform_format = MultiMesh.TRANSFORM_3D
	_mm.mesh = mesh
	_mm.instance_count = MAX_CASINGS
	_mm.visible_instance_count = 0
	var hidden := Transform3D(Basis.from_scale(Vector3.ZERO), Vector3(0.0, -1000.0, 0.0))
	for i in MAX_CASINGS:
		_mm.set_instance_transform(i, hidden)
	_mmi = MultiMeshInstance3D.new()
	_mmi.name = "ShellCasings"
	_mmi.multimesh = _mm
	_mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_mmi.ignore_occlusion_culling = true
	_mmi.visible = enabled
	add_child(_mmi)


## Queues a spent casing behind `origin` along `-forward` (rear of breech).
func queue_ejection(now: float, unit: int, origin: Vector3, forward: Vector3, caliber: float) -> void:
	if not enabled or _mm == null or caliber < MIN_CALIBER_MM:
		return
	var flat := Vector3(forward.x, 0.0, forward.z)
	var dir := flat.normalized() if flat.length() > 0.01 else Vector3.FORWARD
	var right := Vector3(-dir.z, 0.0, dir.x)
	_seq += 1
	var h := _hash(unit * 73856093 ^ _seq * 19349663)
	var r1 := float(h & 0xFFFF) / 65535.0
	var r2 := float((h >> 16) & 0xFFFF) / 65535.0
	var back_m := 1.25 + r1 * 1.15
	var side_m := (r2 - 0.5) * 1.1
	var at := origin - dir * back_m + right * side_m
	_pending.append({
		"t": now + EJECT_DELAY_S,
		"x": at.x,
		"z": at.z,
		"yaw": atan2(-dir.x, -dir.z) + (r1 - 0.5) * 2.4,
		"caliber": caliber,
		"roll": (r2 - 0.5) * 0.22,
	})


func update(now: float) -> void:
	if _pending.is_empty() or _mm == null or _sim_host == null:
		return
	var keep: Array[Dictionary] = []
	for item: Dictionary in _pending:
		if float(item.t) > now:
			keep.append(item)
			continue
		if not enabled:
			continue
		var x: float = item.x
		var z: float = item.z
		var scale_f := clampf(float(item.caliber) / 105.0, 0.55, 1.5)
		var y := _sim_host.height_at(x, z) + BASE_RADIUS_M * scale_f * 0.85
		var basis := Basis(Vector3.UP, float(item.yaw)) * Basis(Vector3.RIGHT, PI * 0.5 + float(item.roll))
		basis = basis.scaled(Vector3(scale_f, scale_f, scale_f))
		_mm.set_instance_transform(_next, Transform3D(basis, Vector3(x, y, z)))
		_next = (_next + 1) % MAX_CASINGS
		spawned += 1
		if _mm.visible_instance_count < MAX_CASINGS:
			_mm.visible_instance_count = mini(spawned, MAX_CASINGS)
	_pending = keep


static func _hash(v: int) -> int:
	var x := (v ^ (v >> 16)) * 0x45d9f3b
	x = (x ^ (x >> 16)) * 0x45d9f3b
	return x ^ (x >> 16)
