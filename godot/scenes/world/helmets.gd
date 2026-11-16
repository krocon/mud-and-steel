extends Node3D
## Helmets that fly off (prompt 048): a presentation effect when a soldier is
## thrown by a blast or hit at the head. The helmet (or cap) on his Gear mesh
## is hidden with the shader's instance uniform `helmet_off`, and a simple
## helmet shell flies off on a ballistic arc, tumbling, and stays on the
## ground. Randomness comes from the caller's presentation RNG; nothing goes
## back to the simulation. [provisional]

## Helmets lying around at most; the oldest disappears first.
const MAX_HELMETS := 12
const GRAVITY := 9.81

## Flying and lying helmets: {node, velocity, spin, ground_y, flying}.
var _helmets: Array[Dictionary] = []
## Helmets knocked off in total (statistics, self-test).
var knocked_off := 0


## Takes the headgear off `member` and throws a helmet shell from his head
## with `velocity` (m/s). `ground_y` is where it comes to rest.
func knock_off(member: Node3D, faction: String, biome: String, velocity: Vector3, spin: Vector3,
		ground_y: float) -> bool:
	var gear := member.find_child("Gear", true, false) as GeometryInstance3D
	var skeleton := SoldierLook.find_skeleton(member)
	if gear == null or skeleton == null or gear.get_instance_shader_parameter(&"helmet_off") == 1.0:
		return false
	gear.set_instance_shader_parameter(&"helmet_off", 1.0)
	var head := skeleton.find_bone("Head")
	var at := member.global_position + Vector3.UP * 1.7
	if head >= 0:
		at = skeleton.global_transform * (skeleton.get_bone_global_pose(head).origin) + Vector3.UP * 0.12
	var shell := MeshInstance3D.new()
	shell.name = "Helmet"
	var mesh := SphereMesh.new()
	mesh.radius = 0.13
	mesh.height = 0.13
	mesh.is_hemisphere = true
	mesh.radial_segments = 10
	mesh.rings = 3
	shell.mesh = mesh
	shell.material_override = SoldierLook.slot_material(faction, biome, "Paint")
	shell.scale = Vector3(1.0, 1.0, 1.1)
	add_child(shell)
	shell.global_position = at
	_helmets.append({"node": shell, "velocity": velocity, "spin": spin, "ground_y": ground_y,
		"flying": true})
	while _helmets.size() > MAX_HELMETS:
		(_helmets.pop_front()["node"] as Node).queue_free()
	knocked_off += 1
	return true


## Puts the headgear back on (a member reused for a new soldier).
static func restore(member: Node3D) -> void:
	var gear := member.find_child("Gear", true, false) as GeometryInstance3D
	if gear != null:
		gear.set_instance_shader_parameter(&"helmet_off", 0.0)


func flying() -> int:
	var n := 0
	for h: Dictionary in _helmets:
		if h["flying"]:
			n += 1
	return n


func _process(delta: float) -> void:
	for h: Dictionary in _helmets:
		if not h["flying"]:
			continue
		var node: Node3D = h["node"]
		var v: Vector3 = h["velocity"]
		v.y -= GRAVITY * delta
		h["velocity"] = v
		node.global_position += v * delta
		var spin: Vector3 = h["spin"]
		if spin.length() > 0.001:
			node.rotate(spin.normalized(), spin.length() * delta)
		var rest := float(h["ground_y"]) + 0.03
		if node.global_position.y <= rest and v.y < 0.0:
			node.global_position.y = rest
			h["flying"] = false
			# Comes to rest on its rim or upside down.
			var up := Vector3.UP if randf() < 0.5 else Vector3.DOWN
			node.global_basis = Basis(Vector3.UP, randf() * TAU) * Basis(Vector3.RIGHT, 0.0 if up.y > 0.0 else PI)
