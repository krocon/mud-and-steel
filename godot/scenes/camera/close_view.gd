extends Node
## Close view: `camera_close_flat` (Shift + F, or the button beside the
## minimap) glides the camera in to `DISTANCE` at the flattest tilt, onto
## the active unit if one is selected, else onto the screen center: a look
## at the fight from up close. Pressed again, the zoom and tilt from before
## come back (the camera stays where it is). The counterpart of the
## overview (`overview.gd`, Shift + O): entering one leaves the other.
## Presentation only.

## Camera distance of the close view in meters (at least the zoom limit).
const DISTANCE := 22.0

var main: Node3D
var active := false
var _saved := {}


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "CloseView"
	process_mode = Node.PROCESS_MODE_ALWAYS


func toggle() -> void:
	if active:
		leave()
	else:
		enter()


func enter() -> void:
	if active:
		return
	var overview: Node = main.get("overview")
	if overview != null and overview.active:
		overview.leave()
	var rig: Node3D = main.camera_rig
	active = true
	_saved = {"distance": rig.target_distance, "tilt": rig.target_tilt}
	rig._zoom_anchor = rig.get_viewport().get_visible_rect().size * 0.5
	# (The same size on the screen in every camera perspective.)
	rig.target_distance = maxf(DISTANCE * rig.zoom_scale, rig.min_distance)
	# The flattest view the limits allow (pitch() clamps it).
	rig.target_tilt = -PI
	var unit: int = main.selection.active_unit if main.get("selection") != null else -1
	var at: Variant = _unit_position(unit)
	if at != null:
		rig.center_on(at)


func leave() -> void:
	if not active:
		return
	var rig: Node3D = main.camera_rig
	active = false
	rig._zoom_anchor = rig.get_viewport().get_visible_rect().size * 0.5
	rig.target_distance = clampf(_saved.distance, rig.min_distance, rig.max_distance)
	rig.target_tilt = _saved.tilt


## Ends the mode without restoring (another view took over).
func drop() -> void:
	active = false


func _unit_position(unit: int) -> Variant:
	if unit < 0:
		return null
	var node: Node3D = main._unit_nodes.get(unit)
	return node.global_position if node != null else null


func _unhandled_input(event: InputEvent) -> void:
	var photo: Node = main.get("photo_mode")
	if photo != null and photo.active:
		return
	if event.is_action_pressed("camera_close_flat", false, true):
		toggle()
		get_viewport().set_input_as_handled()
