extends PanelContainer
## Mission panel (prompts 039, 108, 180, 181, 189, 237):
## - Top-left collapsible objectives panel with compact progress header
##   (`OBJECTIVES 1/3 · OPTIONAL 0/1 · HIDE ▲` / `SHOW ▼`) that pulses subtly
##   on objective updates and stays compact when collapsed.
## - Atmospheric skippable chapter intro card over the loaded 3D battlefield
##   when starting a campaign mission interactively (skipped automatically in
##   replays, headless self-tests and `--skip-menu`).
## - Forwards new mission messages to `Subtitles.radio(...)` bottom-center
##   (supporting optional `"Speaker: message"` prefixes) while keeping the
##   latest message readable inside the panel.
## - Draws objective area rings, reports arriving reinforcement columns,
##   raises the "hardly winnable" warning panel, sends presentation signals
##   (`camera`, `select`, `pause`) and records victories/stars. Presentation only.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const Subtitles := preload("res://scenes/hud/subtitles.gd")
const TEXT_STEP := "HUD_MISSION_STEP"
const TEXT_DONE := "HUD_MISSION_DONE"
const TEXT_FAILED := "HUD_MISSION_FAILED"
const TEXT_ALL_DONE := "HUD_MISSION_ALL_DONE"
const TEXT_OPTIONAL := "HUD_MISSION_OPTIONAL"
const TEXT_REINFORCEMENTS := "HUD_FEED_REINFORCEMENTS"
const TEXT_ENEMY_REINFORCEMENTS := "HUD_FEED_ENEMY_REINFORCEMENTS"
## The camera counts as moved after this many meters, this zoom factor or
## this turn (radians).
const CAMERA_MOVE_M := 12.0
const CAMERA_ZOOM := 0.15
const CAMERA_TURN := 0.3
const WIDTH := 380.0
const RING_COLOR := Color(0.95, 0.8, 0.3)
const CHAPTER_DURATION_S := 4.0

var _mission_path := ""
var main: Node3D
var sim_host: SimHost
var title: Label
var message: Label
var objectives_box: VBoxContainer
## Compact collapsible header button (prompt 237; self-test).
var collapse_button: Button
var collapsed := false
## Atmospheric chapter intro overlay (prompt 237; self-test).
var chapter_overlay: Control
var chapter_card_shown := false
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

var _body: VBoxContainer
var _message_tick := -2
var _camera_tick := -1
## Arrivals already known (-1: not read yet; those of a loaded game stay quiet).
var _arrivals_seen := -1
var _objective_key := ""
var _pulse_left := 0.0
var _chapter_age := -1.0
var _chapter_paused_sim := false
## Camera at the moment the `camera` signal became wanted.
var _camera_from: Array = []
var _ring_mesh: MeshInstance3D
var _frame := 0


class ChapterCardOverlay extends Control:
	signal dismissed

	var campaign_title := ""
	var chapter_label := ""
	var mission_title := ""
	var date_label := ""
	var tagline := ""
	var nodes: Array[Dictionary] = []
	var alpha := 1.0

	func _ready() -> void:
		set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		mouse_filter = Control.MOUSE_FILTER_STOP

	func _gui_input(event: InputEvent) -> void:
		var btn := event as InputEventMouseButton
		if btn != null and btn.pressed:
			dismissed.emit()
			accept_event()

	func _input(event: InputEvent) -> void:
		if not visible:
			return
		var key := event as InputEventKey
		if key != null and key.pressed and not key.echo and (key.keycode == KEY_SPACE or key.keycode == KEY_ESCAPE or key.keycode == KEY_ENTER):
			dismissed.emit()
			get_viewport().set_input_as_handled()

	func _draw() -> void:
		if alpha <= 0.001:
			return
		var w := size.x
		var h := size.y
		# Calm dark cinematic veil + stronger top/bottom vignette over live 3D battlefield.
		draw_rect(Rect2(Vector2.ZERO, size), Color(0.03, 0.04, 0.05, 0.56 * alpha))
		draw_rect(Rect2(Vector2.ZERO, Vector2(w, h * 0.16)), Color(0.02, 0.02, 0.03, 0.45 * alpha))
		draw_rect(Rect2(Vector2(0.0, h * 0.78), Vector2(w, h * 0.22)), Color(0.02, 0.02, 0.03, 0.55 * alpha))
		var font := get_theme_default_font()
		var cx := w * 0.5
		var y := h * 0.34
		if not campaign_title.is_empty():
			var sz := MenuTheme.font(16)
			var text := campaign_title.to_upper()
			var tw := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, sz).x
			draw_string(font, Vector2(cx - tw * 0.5, y), text, HORIZONTAL_ALIGNMENT_LEFT, -1, sz, Color(0.92, 0.78, 0.36, alpha))
			y += float(sz) + 12.0
		if not chapter_label.is_empty():
			var sz := MenuTheme.font(14)
			var tw := font.get_string_size(chapter_label, HORIZONTAL_ALIGNMENT_LEFT, -1, sz).x
			draw_string(font, Vector2(cx - tw * 0.5, y), chapter_label, HORIZONTAL_ALIGNMENT_LEFT, -1, sz, Color(0.82, 0.82, 0.78, alpha))
			y += float(sz) + 18.0
		var title_sz := MenuTheme.font(34)
		var mtw := font.get_string_size(mission_title, HORIZONTAL_ALIGNMENT_LEFT, -1, title_sz).x
		draw_string(font, Vector2(cx - mtw * 0.5 + 2.0, y + 2.0), mission_title, HORIZONTAL_ALIGNMENT_LEFT, -1, title_sz, Color(0, 0, 0, 0.75 * alpha))
		draw_string(font, Vector2(cx - mtw * 0.5, y), mission_title, HORIZONTAL_ALIGNMENT_LEFT, -1, title_sz, Color(0.98, 0.96, 0.90, alpha))
		y += 20.0
		# Subtle horizontal rule.
		draw_line(Vector2(cx - 120.0, y), Vector2(cx + 120.0, y), Color(0.92, 0.78, 0.36, 0.7 * alpha), 1.5, true)
		y += 28.0
		if not date_label.is_empty():
			var sz := MenuTheme.font(18)
			var tw := font.get_string_size(date_label, HORIZONTAL_ALIGNMENT_LEFT, -1, sz).x
			draw_string(font, Vector2(cx - tw * 0.5, y), date_label, HORIZONTAL_ALIGNMENT_LEFT, -1, sz, Color(0.95, 0.85, 0.55, alpha))
			y += float(sz) + 14.0
		if not tagline.is_empty():
			var sz := MenuTheme.font(18)
			var tw := font.get_string_size(tagline, HORIZONTAL_ALIGNMENT_LEFT, -1, sz).x
			draw_string(font, Vector2(cx - tw * 0.5, y), tagline, HORIZONTAL_ALIGNMENT_LEFT, -1, sz, Color(0.92, 0.91, 0.86, 0.95 * alpha))
		# Bottom horizontal campaign timeline.
		var n := nodes.size()
		if n > 0:
			var ty := h * 0.84
			var half_span := minf(w * 0.34, 320.0)
			var pts: Array[Vector2] = []
			for i in n:
				var t := 0.5 if n == 1 else float(i) / float(n - 1)
				pts.append(Vector2(cx - half_span + half_span * 2.0 * t, ty))
			for i in n - 1:
				var done := bool(nodes[i].get("won", false))
				draw_line(pts[i], pts[i + 1], Color(0.92, 0.78, 0.36, (0.85 if done else 0.35) * alpha), 2.5, true)
			var lsz := MenuTheme.font(12)
			for i in n:
				var item: Dictionary = nodes[i]
				var current := bool(item.get("current", false))
				var won := bool(item.get("won", false))
				var r := 9.0 if current else 6.0
				if current:
					draw_arc(pts[i], r + 5.0, 0.0, TAU, 24, Color(1.0, 0.86, 0.38, alpha), 2.0, true)
				var col := Color(1.0, 0.84, 0.34, alpha) if current \
						else (Color(0.45, 0.88, 0.45, alpha) if won else Color(0.55, 0.56, 0.58, 0.7 * alpha))
				draw_circle(pts[i], r, col)
				var lbl := str(item.get("label", ""))
				if not lbl.is_empty():
					var lw := font.get_string_size(lbl, HORIZONTAL_ALIGNMENT_LEFT, -1, lsz).x
					draw_string(font, Vector2(pts[i].x - lw * 0.5, ty + r + float(lsz) + 6.0),
							lbl, HORIZONTAL_ALIGNMENT_LEFT, -1, lsz, Color(0.96, 0.93, 0.84, (1.0 if current else 0.65) * alpha))
		var skip := tr("HUD_CHAPTER_SKIP")
		var ssz := MenuTheme.font(13)
		var sw := font.get_string_size(skip, HORIZONTAL_ALIGNMENT_LEFT, -1, ssz).x
		draw_string(font, Vector2(cx - sw * 0.5, h * 0.94), skip, HORIZONTAL_ALIGNMENT_LEFT, -1, ssz, Color(0.76, 0.76, 0.74, 0.75 * alpha))


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "MissionHud"
	theme = MenuTheme.get_theme()
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	position = Vector2(12.0, 12.0)
	custom_minimum_size = Vector2(WIDTH, 0)
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_PASS
	box.add_theme_constant_override(&"separation", 4)
	add_child(box)
	# Compact collapsible header (prompt 237).
	collapse_button = Button.new()
	collapse_button.name = "ObjectivesToggle"
	collapse_button.flat = true
	collapse_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	collapse_button.focus_mode = Control.FOCUS_NONE
	collapse_button.add_theme_font_size_override(&"font_size", MenuTheme.font(13))
	collapse_button.add_theme_color_override(&"font_color", Color(0.96, 0.84, 0.42))
	collapse_button.pressed.connect(toggle_collapsed)
	box.add_child(collapse_button)
	_body = VBoxContainer.new()
	_body.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_body.add_theme_constant_override(&"separation", 4)
	box.add_child(_body)
	title = MenuTheme.title("", MenuTheme.FONT_SIZE + 2)
	_body.add_child(title)
	message = Label.new()
	message.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	message.custom_minimum_size = Vector2(WIDTH - 16.0, 0)
	_body.add_child(message)
	objectives_box = VBoxContainer.new()
	_body.add_child(objectives_box)
	_ring_mesh = MeshInstance3D.new()
	_ring_mesh.name = "ObjectiveRings"
	_ring_mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = RING_COLOR
	_ring_mesh.material_override = material
	main.add_child(_ring_mesh)
	_build_hopeless()
	_build_chapter_overlay()
	refresh()
	_maybe_start_chapter_card()


func _build_chapter_overlay() -> void:
	chapter_overlay = ChapterCardOverlay.new()
	chapter_overlay.name = "ChapterCardOverlay"
	chapter_overlay.visible = false
	chapter_overlay.dismissed.connect(dismiss_chapter_card)
	main.get_node("Hud").add_child.call_deferred(chapter_overlay)


## Collapses or expands the mission objectives body (prompt 237; self-test).
func set_collapsed(value: bool) -> void:
	collapsed = value
	if _body != null:
		_body.visible = not collapsed
	_update_header(info.get("objectives", []).filter(func(o: Dictionary) -> bool: return o["state"] != "hidden"))
	size = Vector2.ZERO


func toggle_collapsed() -> void:
	set_collapsed(not collapsed)


## Shows the atmospheric chapter intro card if this mission belongs to a
## campaign and we are in an interactive session (never in replays, headless
## self-tests or `--skip-menu`).
func _maybe_start_chapter_card() -> void:
	if sim_host.is_replay() or MatchSetup.flow_test != null or DisplayServer.get_name() == "headless":
		return
	for arg in OS.get_cmdline_user_args():
		if arg == "--skip-menu" or arg.contains("test") or arg.contains("bench") or arg.contains("shots"):
			return
	var mid := str(info.get("id", ""))
	if mid.is_empty():
		return
	var found := _find_campaign_entry(mid)
	if found.is_empty():
		return
	show_chapter_card(found["campaign"], int(found["index"]), true)


func _find_campaign_entry(mission_id: String) -> Dictionary:
	for c: Dictionary in Missions.campaigns():
		if not c.get("ok", false):
			continue
		var missions: Array = c.get("missions", [])
		for k in missions.size():
			if String(missions[k].get("id", "")) == mission_id:
				return {"campaign": c, "index": k}
	return {}


## Displays the chapter intro card for mission `chapter_index` of campaign `c`
## (also directly callable from self-tests).
func show_chapter_card(c: Dictionary, chapter_index: int, pause_sim := false) -> void:
	if chapter_overlay == null:
		return
	var missions: Array = c.get("missions", [])
	if chapter_index < 0 or chapter_index >= missions.size():
		return
	var m: Dictionary = missions[chapter_index]
	var cpath := String(c.get("path", ""))
	chapter_overlay.campaign_title = Missions.text(cpath, String(c.get("title", "")))
	chapter_overlay.chapter_label = tr("HUD_CHAPTER_LABEL") % (chapter_index + 1)
	chapter_overlay.mission_title = Missions.text(_path(), String(info.get("title", m.get("id", ""))))
	var d_raw := String(m.get("date_label", ""))
	chapter_overlay.date_label = Missions.text(cpath, d_raw) if not d_raw.is_empty() else ""
	var t_raw := String(m.get("tagline", ""))
	var intro_raw := String(m.get("intro", ""))
	chapter_overlay.tagline = Missions.text(cpath, t_raw) if not t_raw.is_empty() else Missions.text(cpath, intro_raw)
	var nodes: Array[Dictionary] = []
	for k in missions.size():
		var mm: Dictionary = missions[k]
		var dl := String(mm.get("date_label", ""))
		nodes.append({
			"label": Missions.text(cpath, dl) if not dl.is_empty() else "#%d" % (k + 1),
			"won": Missions.is_done(String(mm.get("id", ""))),
			"current": k == chapter_index,
		})
	chapter_overlay.nodes = nodes
	chapter_overlay.alpha = 1.0
	chapter_overlay.visible = true
	chapter_overlay.queue_redraw()
	_chapter_age = 0.0
	chapter_card_shown = true
	if pause_sim and sim_host != null and not sim_host.is_replay():
		_chapter_paused_sim = true
		sim_host.speed = 0.0


func dismiss_chapter_card() -> void:
	if chapter_overlay != null:
		chapter_overlay.visible = false
	_chapter_age = -1.0
	if _chapter_paused_sim and sim_host != null:
		_chapter_paused_sim = false
		if sim_host.speed <= 0.001:
			sim_host.speed = 1.0


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


func _process(delta: float) -> void:
	if _chapter_age >= 0.0 and chapter_overlay != null and chapter_overlay.visible:
		_chapter_age += delta
		if _chapter_age >= CHAPTER_DURATION_S:
			dismiss_chapter_card()
		else:
			var fade_in := clampf(_chapter_age / 0.4, 0.0, 1.0)
			var fade_out := clampf((CHAPTER_DURATION_S - _chapter_age) / 0.8, 0.0, 1.0)
			chapter_overlay.alpha = minf(fade_in, fade_out)
			chapter_overlay.queue_redraw()
	if _pulse_left > 0.0:
		_pulse_left = maxf(_pulse_left - delta, 0.0)
		var pulse := 0.5 + 0.5 * sin(_pulse_left * TAU * 2.5)
		collapse_button.add_theme_color_override(&"font_color",
				Color(0.96, 0.84, 0.42).lerp(Color(1.0, 1.0, 0.92), pulse * (_pulse_left / 1.5)))
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
		var was_init := _message_tick == -2
		_message_tick = info["message_tick"]
		var raw_msg := with_keys(Missions.text(_path(), info["message"]))
		message.text = raw_msg
		message.visible = not raw_msg.is_empty()
		if not raw_msg.is_empty() and (not was_init or _message_tick >= 0):
			var parts := _split_speaker(raw_msg)
			Subtitles.radio(parts[1], parts[0])
	# Prompt 189: timers and counts change the lines too.
	var key := ",".join(shown.map(func(o: Dictionary) -> String:
		return "%s:%s:%d:%d" % [o["id"], o["state"], int(o.get("timer_s", -1)), int(o.get("progress", -1))]))
	if key != _objective_key:
		var had_key := not _objective_key.is_empty()
		_objective_key = key
		_show_objectives(shown)
		_update_header(shown)
		_draw_rings(shown)
		if had_key:
			_pulse_left = 1.5
	if info["camera_tick"] != _camera_tick:
		_camera_tick = info["camera_tick"]
		if info["camera"] != null and not sim_host.is_replay():
			main.camera_rig.center_on(info["camera"])
	_report_arrivals(info.get("arrivals", []))
	_send_signals(info["wanted"])
	_show_hopeless()


## Splits an optional `"Speaker: text"` prefix (max 24 chars before colon,
## ignoring `http:` or keys); returns `[speaker, body]`.
static func _split_speaker(text: String) -> Array[String]:
	var colon := text.find(": ")
	if colon > 0 and colon <= 24:
		var head := text.substr(0, colon)
		if not head.contains(".") or head.ends_with("."):
			return [head, text.substr(colon + 2)]
	return ["", text]


func _update_header(shown: Array) -> void:
	if collapse_button == null:
		return
	var primary_total := 0
	var primary_done := 0
	var opt_total := 0
	var opt_done := 0
	for o: Dictionary in shown:
		if o.get("optional", false):
			opt_total += 1
			if o["state"] == "done":
				opt_done += 1
		else:
			primary_total += 1
			if o["state"] == "done":
				primary_done += 1
	var summary := (tr("HUD_MISSION_HEADER_OPT") % [primary_done, primary_total, opt_done, opt_total]) if opt_total > 0 \
			else (tr("HUD_MISSION_HEADER") % [primary_done, primary_total])
	var action := tr("HUD_MISSION_SHOW") if collapsed else tr("HUD_MISSION_HIDE")
	collapse_button.text = "%s  ·  %s" % [summary, action]


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
		var text := line_of(Missions.text(_path(), o["text"]), o)
		match o["state"]:
			"done":
				label.text = tr(TEXT_DONE) % text
				label.add_theme_color_override("font_color", MenuTheme.VICTORY)
			"failed":
				label.text = tr(TEXT_FAILED) % text
				label.add_theme_color_override("font_color", MenuTheme.DEFEAT)
			_:
				label.text = "• " + with_keys(text)
				open += 1
		objectives_box.add_child(label)
	if open == 0 and not shown.is_empty():
		objectives_box.add_child(MenuTheme.hint(TEXT_ALL_DONE))


## An objective's line (prompt 189): "Optional:" before a side objective,
## its count "(2/5)" and, while it runs, its timer "– 3:20" after it.
func line_of(text: String, o: Dictionary) -> String:
	if o.get("optional", false):
		text = tr(TEXT_OPTIONAL) % text
	if int(o.get("progress_of", -1)) > 0:
		text += " (%d/%d)" % [int(o["progress"]), int(o["progress_of"])]
	var left := int(o.get("timer_s", -1))
	if left >= 0 and o["state"] == "active":
		text += " – %d:%02d" % [left / 60, left % 60]
	return text


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
	# Prompt 190: in a cooperative mission the winner may be an ally.
	if result["decided"] and (result["winner"] == sim_host.local_player() or (result["winner"] >= 0 and sim_host.local_team_won())):
		recorded = true
		var rating: Dictionary = sim_host.mission_info().get("rating", {})
		Missions.mark_done(info.get("id", ""), str(info.get("level", "normal")), int(rating.get("stars", 0)))
		print("Mission '%s' won at tick %d" % [info.get("id", ""), result["tick"]])
