extends Node
## Portraits of unit types for the unit panel (prompt 031, [provisional]).
##
## There are no thumbnail files: the first time a type is shown, a fresh
## model of the type (built like the unit's, by `main.gd`) is rendered once
## in its own small `SubViewport` (own world, a key light, the camera
## framing the model's bounds from the front left), and the texture is kept
## for the match. (Duplicating the unit's own model does not work: its
## static parts are merged at run time, prompt 016.) Headless runs get the
## texture too; it simply stays empty there.

const SIZE := Vector2i(160, 112)

## Catalog id -> ViewportTexture.
var _textures: Dictionary[String, Texture2D] = {}


## The portrait of `type_id`; `build` makes the model the first time
## (returns a Node3D, or null).
func texture_for(type_id: String, build: Callable) -> Texture2D:
	if _textures.has(type_id):
		return _textures[type_id]
	var copy := build.call() as Node3D
	if copy == null:
		_textures[type_id] = null
		return null
	var viewport := SubViewport.new()
	viewport.name = "Portrait_" + type_id
	viewport.size = SIZE
	viewport.own_world_3d = true
	viewport.transparent_bg = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(viewport)
	copy.transform = Transform3D.IDENTITY
	viewport.add_child(copy)
	_stop_animations(copy)
	var bounds := _bounds(copy, Transform3D.IDENTITY)
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-50, -35, 0)
	viewport.add_child(light)
	var environment := Environment.new()
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.75, 0.75, 0.78)
	environment.ambient_light_energy = 0.7
	environment.background_mode = Environment.BG_CLEAR_COLOR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.fov = 30.0
	var center := bounds.get_center()
	var radius := maxf(bounds.size.length() * 0.5, 0.5)
	# Front left, slightly from above (models face -Z).
	var direction := Vector3(-0.75, 0.45, -1.0).normalized()
	camera.position = center + direction * radius / sin(deg_to_rad(camera.fov * 0.5)) * 1.05
	viewport.add_child(camera)
	camera.look_at(center, Vector3.UP)
	camera.current = true
	_textures[type_id] = viewport.get_texture()
	return _textures[type_id]


## Bounds of every visual instance below `node`, in its parent's space.
func _bounds(node: Node, parent: Transform3D) -> AABB:
	var xform := parent
	if node is Node3D:
		xform = parent * (node as Node3D).transform
	var result := AABB()
	var first := true
	var visual := node as VisualInstance3D
	if visual != null:
		result = xform * visual.get_aabb()
		first = false
	for child in node.get_children():
		var inner := _bounds(child, xform)
		if inner.size == Vector3.ZERO:
			continue
		result = inner if first else result.merge(inner)
		first = false
	return result


func _stop_animations(node: Node) -> void:
	for player in node.find_children("*", "AnimationPlayer", true, false):
		(player as AnimationPlayer).pause()
