extends PanelContainer
## Mission panel (prompt 039): the mission's title (the tutorial's step),
## its latest message and the objectives, top left (the top right corner
## belongs to the resources and the support column, prompt 095). Presentation only: the
## script runs in the simulation (`docs/MISSIONS.md`).
##
## Also draws a ring around the area of every active objective, moves the
## camera where the script asks, and sends the signals the script waits for
## and only the presentation knows: `camera` (the view was panned, zoomed
## or turned) and `select` (own units are selected). A won mission is
## recorded in the progress file (`menu/missions.gd`).
##
## Prompt 108: a column of reinforcements that starts driving in (`spawn`
## with `arrive = "drive"`) posts "Reinforcements arriving" (own and allied)
## or "Enemy reinforcements" to the event feed at its entry point (a click
## jumps there) and pings the minimap.
##
## Prompt 181: when the script says the mission can hardly be won any more
## (action `hopeless`), a panel in the middle says so and offers Restart
## (same mission and difficulty), Load last save (the newest saved game of
## this map) and Play on. Once per match; not in replays.
##
## Prompt 180: `{key:<action>}` in a mission text shows the key the action
## is bound to now (`Settings.action_key_name`), so hints follow the
## player's bindings; the signal `pause` is sent while the tactical pause is
## on. A tutorial chapter's title reads "<title> – step n of m".
##
## All texts come from the mission file; the few own ones are the
## translation keys below (prompt 043).

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const TEXT_STEP := "HUD_MISSION_STEP"
const TEXT_DONE := "HUD_MISSION_DONE"
const TEXT_FAILED := "HUD_MISSION_FAILED"
const TEXT_ALL_DONE := "HUD_MISSION_ALL_DONE"
const TEXT_REINFORCEMENTS := "HUD_FEED_REINFORCEMENTS"
const TEXT_ENEMY_REINFORCEMENTS := "HUD_FEED_ENEMY_REINFORCEMENTS"
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
## Arriving columns reported to the event feed (self-test).
var arrivals_reported := 0
## The "hardly winnable" panel (prompt 181) and its buttons.
var hopeless_panel: PanelContainer
var hopeless_text: Label
var restart_button: Button
var load_button: Button
var go_on_button: Button
## Tick of the warning already shown (-1: none; -2: not read yet, the
## warning of a loaded game stays quiet).
var hopeless_shown := -2

var _message_tick := -2
var _camera_tick := -1
## Arrivals already known (-1: not read yet; those of a loaded game stay quiet).
var _arrivals_seen := -1
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
	set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	position = Vector2(12.0, 12.0)
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
	_build_hopeless()
	refresh()


func _build_hopeless() -> void:
	hopeless_panel = PanelContainer.new()
	hopeless_panel.name = "HopelessPanel"
	hopeless_panel.theme = MenuTheme.get_theme()
	hopeless_panel.visible = false
	hopeless_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	hopeless_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	hopeless_panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	hopeless_panel.custom_minimum_size = Vector2(460, 0)
	var box := VBoxContainer.new()
	hopeless_panel.add_child(box)
	box.add_child(MenuTheme.title("HUD_HOPELESS_TITLE", MenuTheme.FONT_SIZE + 4))
	hopeless_text = Label.new()
	hopeless_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hopeless_text.custom_minimum_size = Vector2(440, 0)
	box.add_child(hopeless_text)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	box.add_child(buttons)
	restart_button = _hopeless_button(buttons, "HUD_HOPELESS_RESTART", restart)
	load_button = _hopeless_button(buttons, "HUD_HOPELESS_LOAD", load_last_save)
	go_on_button = _hopeless_button(buttons, "HUD_HOPELESS_GO_ON", func() -> void:
		hopeless_panel.visible = false)
	# Beside the mission panel, on the same HUD layer.
	main.get_node("Hud").add_child.call_deferred(hopeless_panel)


func _hopeless_button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(140, 40)
	button.pressed.connect(action)
	parent.add_child(button)
	return button


## Shows the warning once when the script raised it.
func _show_hopeless() -> void:
	var tick := int(info.get("hopeless_tick", -1))
	if hopeless_shown == -2:
		hopeless_shown = tick
		return
	if tick < 0 or tick == hopeless_shown or sim_host.is_replay():
		return
	hopeless_shown = tick
	if sim_host.match_result()["decided"]:
		return
	show_warning(with_keys(Missions.text(_path(), info.get("hopeless", ""))))


## Opens the warning panel with `text` (also the self-test).
func show_warning(text: String) -> void:
	hopeless_text.text = text
	load_button.disabled = last_save().is_empty()
	hopeless_panel.visible = true


## Starts the running mission again at its difficulty.
func restart() -> void:
	hopeless_panel.visible = false
	Missions.restart(get_tree(), str(info.get("id", "")), str(info.get("level", "normal")))


## Path of the newest saved game of this map, "" if none.
func last_save() -> String:
	for entry in GameFiles.list(GameFiles.SAVE_DIR, GameFiles.SAVE_EXTENSION):
		if entry["info"].get("map", "") == sim_host.map_name():
			return entry["path"]
	return ""


## Loads the newest saved game of this map.
func load_last_save() -> void:
	var path := last_save()
	if path.is_empty():
		return
	hopeless_panel.visible = false
	MatchSetup.pending = {"load": path}
	get_tree().change_scene_to_file(MatchSetup.MAIN_SCENE)


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
		title.text = tr(TEXT_STEP) % [Missions.text(_path(), info["title"]), mini(done + 1, objectives.size()), objectives.size()]
	else:
		title.text = Missions.text(_path(), info["title"])
	if info["message_tick"] != _message_tick:
		_message_tick = info["message_tick"]
		message.text = with_keys(Missions.text(_path(), info["message"]))
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
	_report_arrivals(info.get("arrivals", []))
	_send_signals(info["wanted"])
	_show_hopeless()


## `text` with every `{key:<action>}` replaced by the key bound to the
## action now ("–" if none).
static func with_keys(text: String) -> String:
	var at := text.find("{key:")
	while at >= 0:
		var end := text.find("}", at)
		if end < 0:
			break
		var action := text.substr(at + 5, end - at - 5)
		var key := Settings.action_key_name(action)
		if key.is_empty():
			key = "–"
		text = text.substr(0, at) + key + text.substr(end + 1)
		at = text.find("{key:", at + key.length())
	return text


## Event feed message and minimap ping for every new arriving column.
func _report_arrivals(arrivals: Array) -> void:
	if _arrivals_seen < 0:
		_arrivals_seen = arrivals.size()
		return
	var hud: Node = main.get("hud")
	var feed: Node = hud.get("event_feed") if hud != null else null
	while _arrivals_seen < arrivals.size():
		var a: Dictionary = arrivals[_arrivals_seen]
		_arrivals_seen += 1
		if feed == null:
			continue
		var at: Vector3 = a["entry"]
		if sim_host.relation(int(a["owner"])) == 2:
			feed.post("enemy_reinforce", tr(TEXT_ENEMY_REINFORCEMENTS), at)
		else:
			feed.post("reinforce", tr(TEXT_REINFORCEMENTS), at)
		feed.add_ping(at)
		arrivals_reported += 1


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
				label.text = "• " + with_keys(Missions.text(_path(), o["text"]))
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
			"pause":
				var pause: Node = main.get("tactical_pause")
				fulfilled = pause != null and bool(pause.get("active"))
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
		Missions.mark_done(info.get("id", ""), str(info.get("level", "normal")))
		print("Mission '%s' won at tick %d" % [info.get("id", ""), result["tick"]])
