extends Node3D
## Helmets that fly off (prompt 048): a presentation effect when a soldier is
## thrown by a blast or hit at the head. The helmet (or cap) on his Gear mesh
## is hidden with the shader's instance uniform `helmet_off`, and a simple
## helmet shell flies off on a ballistic arc, tumbling, and stays on the
## ground. Randomness comes from the caller's presentation RNG; nothing goes
## back to the simulation. [provisional]
##
## Prompt 194: a far throw by a big blast also tears the weapon from his
## hands: his weapon mesh is hidden and a copy flies off the same way; a
## knocked-down soldier picks it up again when he gets up (`pick_up`).

## Helmets lying around at most; the oldest disappears first.
const MAX_HELMETS := 12
const GRAVITY := 9.81

## Flying and lying helmets and weapons: {node, velocity, spin, ground_y,
## flying, weapon, member (weapons: whose it is)}.
var _helmets: Array[Dictionary] = []
## Helmets knocked off in total (statistics, self-test).
var knocked_off := 0
## Weapons torn away in total (prompt 194; statistics, self-test).
var weapons_thrown := 0


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
	_add(shell, velocity, spin, ground_y, null)
	knocked_off += 1
	return true


## Prompt 194: hides `member`'s weapon and throws a copy of it from where it
## is with `velocity` (m/s). False if he has none or it is gone already.
func throw_weapon(member: Node3D, velocity: Vector3, spin: Vector3, ground_y: float) -> bool:
	var weapon := _weapon_of(member)
	if weapon == null or weapon.mesh == null or weapon.has_meta(&"thrown"):
		return false
	# The skinned mesh in its rest pose, turned about its own middle.
	var center := weapon.mesh.get_aabb().get_center()
	var copy := MeshInstance3D.new()
	copy.name = "Weapon"
	copy.mesh = weapon.mesh
	for i in weapon.get_surface_override_material_count():
		copy.set_surface_override_material(i, weapon.get_surface_override_material(i))
	copy.position = -center
	var pivot := Node3D.new()
	pivot.name = "ThrownWeapon"
	pivot.add_child(copy)
	add_child(pivot)
	pivot.global_transform = weapon.global_transform * Transform3D(Basis.IDENTITY, center)
	weapon.visible = false
	weapon.set_meta(&"thrown", true)
	_add(pivot, velocity, spin, ground_y, member)
	weapons_thrown += 1
	return true


## Prompt 194: `member` gets up and has his weapon again; the copy lying
## on the ground goes.
func pick_up(member: Node3D) -> void:
	var weapon := _weapon_of(member)
	if weapon == null or not weapon.has_meta(&"thrown"):
		return
	weapon.remove_meta(&"thrown")
	weapon.visible = true
	for i in range(_helmets.size() - 1, -1, -1):
		if _helmets[i]["member"] == member:
			(_helmets[i]["node"] as Node).queue_free()
			_helmets.remove_at(i)


## Whether `mesh` is a weapon that flew off (the view keeps it hidden).
static func thrown(mesh: Node) -> bool:
	return mesh.has_meta(&"thrown")


static func _weapon_of(member: Node3D) -> MeshInstance3D:
	for mesh: Node in member.find_children("Weapon*", "MeshInstance3D", true, false):
		return mesh as MeshInstance3D
	return null


func _add(node: Node3D, velocity: Vector3, spin: Vector3, ground_y: float, member: Node3D) -> void:
	_helmets.append({"node": node, "velocity": velocity, "spin": spin, "ground_y": ground_y,
		"flying": true, "weapon": member != null, "member": member})
	while _helmets.size() > MAX_HELMETS:
		(_helmets.pop_front()["node"] as Node).queue_free()


## Puts the headgear back on (a member reused for a new soldier).
static func restore(member: Node3D) -> void:
	var gear := member.find_child("Gear", true, false) as GeometryInstance3D
	if gear != null:
		gear.set_instance_shader_parameter(&"helmet_off", 0.0)
	var weapon := _weapon_of(member)
	if weapon != null and weapon.has_meta(&"thrown"):
		weapon.remove_meta(&"thrown")
		weapon.visible = true


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
		var rest := float(h["ground_y"]) + (0.05 if h["weapon"] else 0.03)
		if node.global_position.y <= rest and v.y < 0.0:
			node.global_position.y = rest
			h["flying"] = false
			if h["weapon"]:
				# Lies flat: keeps its heading, loses its tilt.
				var forward := node.global_basis.z
				forward.y = 0.0
				node.global_basis = Basis(Vector3.UP, atan2(forward.x, forward.z) if forward.length() > 0.01 else randf() * TAU)
				continue
			# Comes to rest on its rim or upside down.
			var up := Vector3.UP if randf() < 0.5 else Vector3.DOWN
			node.global_basis = Basis(Vector3.UP, randf() * TAU) * Basis(Vector3.RIGHT, 0.0 if up.y > 0.0 else PI)
