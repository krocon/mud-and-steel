extends Node
## Overview (prompt 103): `overview` (Shift + O) glides the camera out to
## `DISTANCE` (at most the map's size) at the steepest tilt, so a large part
## of the map is in the picture. There shadows and small decoration are off
## and every own unit shows its ring (the show-all marks of prompt 164).
## Pressed again, the old view comes back; a zoom in (wheel, pinch) comes
## back too, centered on the point under the pointer: a quick way to jump.
##
## Measured (prompt 103, `just bench-view --perf-distances=...`): the plain
## zoom limit of 600 m stays, 1000-1500 m with shadows and decoration cost
## 20-40 % of the frame rate; the overview switches them off instead.

const GroundPick := preload("res://scenes/camera/ground_pick.gd")

## Camera distance of the overview in meters (less on small maps).
const DISTANCE := 1200.0
## Share of the longer map side the overview distance may reach.
const MAP_SHARE := 0.9

var main: Node3D
var active := false
var _saved := {}


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "Overview"
	process_mode = Node.PROCESS_MODE_ALWAYS


## Overview distance for this map.
func distance() -> float:
	var size: Vector2 = main.sim_host.map_size()
	var rig: Node3D = main.camera_rig
	return clampf(maxf(size.x, size.y) * MAP_SHARE, rig.max_distance, DISTANCE)


func toggle() -> void:
	if active:
		leave()
	else:
		enter()


func enter() -> void:
	if active:
		return
	var close: Node = main.get("close_view")
	if close != null and close.active:
		close.drop()
	var rig: Node3D = main.camera_rig
	active = true
	_saved = {"focus": rig.focus, "distance": rig.target_distance, "tilt": rig.target_tilt,
		"max": rig.max_distance, "shadows": rig.sun.shadow_enabled if rig.sun != null else true}
	rig.max_distance = distance()
	rig._zoom_anchor = rig.get_viewport().get_visible_rect().size * 0.5
	rig.target_distance = rig.max_distance
	rig.target_tilt = PI
	if rig.sun != null:
		rig.sun.shadow_enabled = false
	_detail(false)


## Back to the view before; with `at`, centered there.
func leave(at: Variant = null) -> void:
	if not active:
		return
	var rig: Node3D = main.camera_rig
	active = false
	rig._zoom_anchor = rig.get_viewport().get_visible_rect().size * 0.5
	rig.max_distance = _saved.max
	rig.target_distance = minf(_saved.distance, rig.max_distance)
	rig.target_tilt = _saved.tilt
	rig.center_on(at if at != null else _saved.focus)
	if rig.sun != null:
		rig.sun.shadow_enabled = _saved.shadows
	_detail(true)


func _detail(on: bool) -> void:
	var objects: Node = main.get("map_objects")
	if objects != null and objects.has_method("set_decoration_visible"):
		objects.set_decoration_visible(on)
	var highlight: Node = main.get("unit_highlight")
	if highlight != null:
		highlight.force_show_all = not on


func _input(event: InputEvent) -> void:
	if not active:
		return
	var zoom_in := event.is_action_pressed("camera_zoom_in") or \
		(event is InputEventMagnifyGesture and (event as InputEventMagnifyGesture).factor > 1.0)
	if zoom_in:
		var mouse := get_viewport().get_mouse_position()
		leave(GroundPick.from_screen(main.camera, mouse))
		get_viewport().set_input_as_handled()


func _unhandled_input(event: InputEvent) -> void:
	var photo: Node = main.get("photo_mode")
	if photo != null and photo.active:
		return
	if event.is_action_pressed("overview", false, true):
		toggle()
		get_viewport().set_input_as_handled()
