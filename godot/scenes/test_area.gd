extends Node3D
## Placeholder landmarks on the 2 km test ground, so zoom and depth can be
## judged. Boxes stand on rings at growing distances from the center; farther
## rings get bigger boxes. Fixed layout, no randomness. Removed once real maps
## exist (prompt 010).

const RINGS: Array[Dictionary] = [
	{"radius": 20.0, "size": Vector3(3, 2.5, 6)},  # truck-sized
	{"radius": 50.0, "size": Vector3(8, 6, 10)},  # house-sized
	{"radius": 120.0, "size": Vector3(10, 8, 14)},
	{"radius": 250.0, "size": Vector3(12, 20, 12)},  # tower
	{"radius": 450.0, "size": Vector3(20, 15, 30)},  # barn
	{"radius": 700.0, "size": Vector3(30, 40, 30)},
	{"radius": 950.0, "size": Vector3(40, 60, 40)},
]
const BOXES_PER_RING := 8
const COLORS: Array[Color] = [
	Color(0.55, 0.25, 0.20),
	Color(0.70, 0.65, 0.55),
	Color(0.30, 0.40, 0.25),
	Color(0.35, 0.35, 0.45),
]


func _ready() -> void:
	var materials: Array[StandardMaterial3D] = []
	for color in COLORS:
		var material := StandardMaterial3D.new()
		material.albedo_color = color
		materials.append(material)

	for ring_index in RINGS.size():
		var ring := RINGS[ring_index]
		var mesh := BoxMesh.new()
		mesh.size = ring.size
		for i in BOXES_PER_RING:
			# Offset every other ring by half a step so rings do not line up.
			var angle := TAU * (i + 0.5 * (ring_index % 2)) / BOXES_PER_RING
			var box := MeshInstance3D.new()
			box.mesh = mesh
			box.material_override = materials[(ring_index + i) % materials.size()]
			box.position = Vector3(cos(angle), 0.0, sin(angle)) * ring.radius
			box.position.y = ring.size.y * 0.5
			box.rotation.y = angle
			add_child(box)
