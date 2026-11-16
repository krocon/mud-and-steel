extends PanelContainer
## Mission panel (prompt 039): the mission's title (the tutorial's step),
## its latest message and the objectives, top right. Presentation only: the
## script runs in the simulation (`docs/MISSIONS.md`).
##
## Also draws a ring around the area of every active objective, moves the
## camera where the script asks, and sends the signals the script waits for
## and only the presentation knows: `camera` (the view was panned, zoomed
## or turned) and `select` (own units are selected). A won mission is
## recorded in the progress file (`menu/missions.gd`).
##
## All texts come from the mission file; the few own ones are the
## translation keys below (prompt 043).

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const TEXT_STEP := "HUD_MISSION_STEP"
const TEXT_DONE := "HUD_MISSION_DONE"
const TEXT_FAILED := "HUD_MISSION_FAILED"
const TEXT_ALL_DONE := "HUD_MISSION_ALL_DONE"
## The camera counts as moved after this many meters, this zoom factor or
## this turn (radians).
const CAMERA_MOVE_M := 12.0
const CAMERA_ZOOM := 0.15
const CAMERA_TURN := 0.3
const WIDTH := 380.0
const RING_COLOR := Color(0.95, 0.8, 0.3)

var _mission_path := ""
var main: Node3D
var sim_host: SimHost
var title: Label
var message: Label
var objectives_box: VBoxContainer
## The last state read from `SimHost.mission_info`.
var info: Dictionary = {}
## Signals sent while they were wanted, by name.
var sent: Dictionary[String, bool] = {}
## The mission was recorded as won.
var recorded := false
## Rings drawn around objective areas (self-test).
var rings := 0

var _message_tick := -2
var _camera_tick := -1
var _objective_key := ""
## Camera at the moment the `camera` signal became wanted.
var _camera_from: Array = []
var _ring_mesh: MeshInstance3D
var _frame := 0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "MissionHud"
	theme = MenuTheme.get_theme()
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
	grow_horizontal = Control.GROW_DIRECTION_BEGIN
	position = Vector2(get_viewport_rect().size.x - WIDTH - 12.0, 12.0)
	custom_minimum_size = Vector2(WIDTH, 0)
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(box)
	title = MenuTheme.title("", MenuTheme.FONT_SIZE + 2)
	box.add_child(title)
	message = Label.new()
	message.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	message.custom_minimum_size = Vector2(WIDTH - 16.0, 0)
	box.add_child(message)
	objectives_box = VBoxContainer.new()
	box.add_child(objectives_box)
	_ring_mesh = MeshInstance3D.new()
	_ring_mesh.name = "ObjectiveRings"
	_ring_mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = RING_COLOR
	_ring_mesh.material_override = material
	main.add_child(_ring_mesh)
	refresh()


func _process(_delta: float) -> void:
	_frame += 1
	# Messages and signals every few frames: the state changes per tick.
	if _frame % 3 == 0:
		refresh()
	if _frame % 30 == 0:
		_record_victory()


## Reads the mission state and updates panel, rings, camera and signals.
func refresh() -> void:
	info = sim_host.mission_info()
	if not info.get("active", false):
		visible = false
		return
	var objectives: Array = info["objectives"]
	var shown := objectives.filter(func(o: Dictionary) -> bool: return o["state"] != "hidden")
	if info["tutorial"]:
		var done := objectives.filter(func(o: Dictionary) -> bool: return o["state"] == "done").size()
		title.text = tr(TEXT_STEP) % [mini(done + 1, objectives.size()), objectives.size()]
	else:
		title.text = Missions.text(_path(), info["title"])
	if info["message_tick"] != _message_tick:
		_message_tick = info["message_tick"]
		message.text = Missions.text(_path(), info["message"])
		message.visible = not message.text.is_empty()
	var key := ",".join(shown.map(func(o: Dictionary) -> String: return o["id"] + ":" + o["state"]))
	if key != _objective_key:
		_objective_key = key
		_show_objectives(shown)
		_draw_rings(shown)
	if info["camera_tick"] != _camera_tick:
		_camera_tick = info["camera_tick"]
		if info["camera"] != null and not sim_host.is_replay():
			main.camera_rig.center_on(info["camera"])
	_send_signals(info["wanted"])


## Path of the running mission's file, for its translations (prompt 043).
func _path() -> String:
	if _mission_path.is_empty():
		_mission_path = Missions.path_of(str(info.get("id", "")))
	return _mission_path


func _show_objectives(shown: Array) -> void:
	for child in objectives_box.get_children():
		child.queue_free()
	var open := 0
	for o: Dictionary in shown:
		var label := Label.new()
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(WIDTH - 16.0, 0)
		match o["state"]:
			"done":
				label.text = tr(TEXT_DONE) % Missions.text(_path(), o["text"])
				label.add_theme_color_override("font_color", MenuTheme.VICTORY)
			"failed":
				label.text = tr(TEXT_FAILED) % Missions.text(_path(), o["text"])
				label.add_theme_color_override("font_color", MenuTheme.DEFEAT)
			_:
				label.text = "• " + Missions.text(_path(), o["text"])
				open += 1
		objectives_box.add_child(label)
	if open == 0 and not shown.is_empty():
		objectives_box.add_child(MenuTheme.hint(TEXT_ALL_DONE))


## Rings on the ground around the areas of active objectives.
func _draw_rings(shown: Array) -> void:
	var mesh := ImmediateMesh.new()
	rings = 0
	for o: Dictionary in shown:
		if o["state"] != "active" or o["position"] == null or float(o["radius"]) <= 0.0:
			continue
		var center: Vector3 = o["position"]
		var r: float = o["radius"]
		mesh.surface_begin(Mesh.PRIMITIVE_LINE_STRIP)
		for k in 65:
			var angle := TAU * k / 64.0
			var x := center.x + cos(angle) * r
			var z := center.z + sin(angle) * r
			mesh.surface_add_vertex(Vector3(x, sim_host.height_at(x, z) + 0.4, z))
		mesh.surface_end()
		rings += 1
	_ring_mesh.mesh = mesh


## Sends the wanted signals the player's view fulfils, once each.
func _send_signals(wanted: PackedStringArray) -> void:
	for signal_name: String in sent.keys():
		if not wanted.has(signal_name):
			sent.erase(signal_name)
	if not wanted.has("camera"):
		_camera_from = []
	if sim_host.is_replay():
		return
	var rig: Node3D = main.camera_rig
	for signal_name in wanted:
		if sent.has(signal_name):
			continue
		var fulfilled := false
		match signal_name:
			"camera":
				if _camera_from.is_empty():
					_camera_from = [rig.focus, rig.target_distance, rig.target_yaw]
				else:
					var focus: Vector3 = _camera_from[0]
					fulfilled = Vector2(rig.focus.x - focus.x, rig.focus.z - focus.z).length() > CAMERA_MOVE_M \
						or absf(rig.target_distance / maxf(_camera_from[1], 0.01) - 1.0) > CAMERA_ZOOM \
						or absf(angle_difference(rig.target_yaw, _camera_from[2])) > CAMERA_TURN
			"select":
				var local := sim_host.local_player()
				var owners := sim_host.unit_owners()
				var ids := sim_host.unit_ids()
				for i in ids.size():
					if owners[i] == local and main.selection.is_selected(ids[i]):
						fulfilled = true
						break
		if fulfilled:
			sent[signal_name] = true
			sim_host.mission_signal(signal_name)


## A won mission counts in the progress file.
func _record_victory() -> void:
	if recorded or sim_host.is_replay():
		return
	var result: Dictionary = sim_host.match_result()
	if result["decided"] and result["winner"] == sim_host.local_player():
		recorded = true
		Missions.mark_done(info.get("id", ""))
		print("Mission '%s' won at tick %d" % [info.get("id", ""), result["tick"]])
