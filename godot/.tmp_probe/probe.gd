extends SceneTree
const Ragdolls := preload("res://scenes/world/ragdolls.gd")

var member: Node3D
var rag: Node3D
var frames := 0
var hips_start := Vector3.ZERO

func _initialize() -> void:
	var scene: PackedScene = load("res://assets/models/us/infantry/rifleman.glb")
	member = scene.instantiate()
	root.add_child(member)
	rag = Ragdolls.new()
	root.add_child(rag)

func _physics_process(_delta: float) -> bool:
	frames += 1
	if frames == 2:
		var ok: bool = rag.start(member, Vector3.ZERO, Vector3.ZERO, 0.0, "down")
		print("started ", ok)
	if frames == 5:
		hips_start = rag.lying(member)["hips"]
		member.position += Vector3(10, 0, 0)
	if frames == 8:
		var h: Vector3 = rag.lying(member)["hips"]
		print("hips moved by ", h - hips_start)
		return true
	return false
