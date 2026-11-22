extends Node
## Switches for the interface on and over the map (prompt 103): each kind of
## mark can be turned off in Settings → Interface, lines on the ground can
## be drawn subtle, and `hud_toggle` (F1) hides the whole HUD and every mark
## in the world at once (orders keep working; a short line names the key).
## The photo mode (`photo_mode.gd`) hides the same through `set_hidden`.
##
## | Setting | What |
## |---|---|
## | `show_selection_rings` | rings under selected units (`main._rings`) |
## | `show_routes` | routes and waypoints (`hud/waypoints.gd`) and the target preview |
## | `show_supply_rings` | supply and repair radii (`hud/supply_rings.gd`) |
## | `show_supply_points` | flags, capture rings and borders of supply points |
## | `show_impact_markers` | impact rings of the own shells |
## | `show_mission_rings` | rings around mission objectives |
## | `show_relation_tint` | the ally / enemy tint over other players' models (default off; while Alt/Option is held it shows anyway) |
## | `show_edge_arrows` | edge arrows of alarms (prompt 101) |
## | `ground_lines` | `strong` / `subtle`: lines and circles on the ground at half strength |
## | `supply_border` | `strong` / `subtle` / `off`: borders of supply point areas (prompt 107, `supply_view.gd`) |
##
## Range and sight circles have their own settings (prompt 099). Applied
## after `main.gd` and `unit_highlight.gd` placed their marks, every frame,
## also during the tactical pause.

const Settings := preload("res://scenes/menu/settings.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")

const TOGGLES: Array[String] = [
	"show_selection_rings", "show_routes", "show_supply_rings", "show_supply_points",
	"show_impact_markers", "show_mission_rings", "show_relation_tint", "show_edge_arrows",
]
## Strength of lines on the ground with `ground_lines` = `subtle`.
const SUBTLE_ALPHA := 0.45
## Seconds the "HUD hidden" line stays.
const NOTE_SECONDS := 4.0

var main: Node3D
## The HUD and the marks are hidden (key or photo mode).
var hidden := false
var _note_layer: CanvasLayer
var _note: Label
var _note_left := 0.0
var _tints := {}


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "ViewToggles"
	process_mode = Node.PROCESS_MODE_ALWAYS
	_note_layer = CanvasLayer.new()
	_note_layer.layer = 40
	add_child(_note_layer)
	_note = Label.new()
	_note.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_note.add_theme_color_override(&"font_color", HudTheme.GOLD)
	_note.add_theme_color_override(&"font_outline_color", Color(0, 0, 0, 0.85))
	_note.add_theme_constant_override(&"outline_size", 4)
	_note.visible = false
	_note_layer.add_child(_note)
	for m: StandardMaterial3D in [main._enemy_material, main._ally_material]:
		if m != null:
			_tints[m] = m.albedo_color.a


## Whether the mark of setting `key` shows now.
func shows(key: String) -> bool:
	return not hidden and bool(Settings.get_value(key))


## Hides or shows the HUD and every mark; `note`: say which key brings it back.
func set_hidden(on: bool, note := false) -> void:
	hidden = on
	var hud_layer := main.get_node("Hud") as CanvasLayer
	hud_layer.visible = not on
	if note and on:
		_note.text = tr("HUD_HIDDEN_NOTE") % Settings.action_key_name("hud_toggle")
		_note.add_theme_font_size_override(&"font_size", HudTheme.font(15))
		_note.position = Vector2(HudTheme.px(16), HudTheme.px(12))
		_note.visible = true
		_note_left = NOTE_SECONDS
	elif not on:
		_note.visible = false
	_apply()


func _unhandled_input(event: InputEvent) -> void:
	var photo: Node = main.get("photo_mode")
	if photo != null and photo.active:
		return
	if event.is_action_pressed("hud_toggle", false, true):
		set_hidden(not hidden, true)
		get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	if _note.visible:
		_note_left -= delta
		if _note_left <= 0.0:
			_note.visible = false
	_apply()


func _apply() -> void:
	if not shows("show_selection_rings"):
		for ring: Node3D in main._rings.values():
			ring.visible = false
	var hud: Node = main.get("hud")
	var subtle: bool = Settings.get_value("ground_lines") == "subtle"
	var line_alpha := SUBTLE_ALPHA if subtle else 1.0
	if hud != null:
		hud.waypoints.visible = shows("show_routes")
		hud.order_preview.visible = shows("show_routes")
		hud.supply_rings.visible = shows("show_supply_rings")
		hud.alerts.visible = shows("show_edge_arrows")
		for part: CanvasItem in [hud.waypoints, hud.order_preview, hud.supply_rings, hud.range_rings]:
			part.modulate.a = line_alpha
	var base_view: Node = main.get("base_view")
	if base_view != null:
		var flags: bool = shows("show_supply_points") and base_view.supply.marks_enabled
		for flag: Node3D in base_view.supply.flags:
			flag.visible = flags
	var artillery: Node = main.get("artillery_view")
	if artillery != null:
		artillery.markers_enabled = shows("show_impact_markers")
	var mission: Node = main.get("mission_hud")
	if mission != null and mission.get("_ring_mesh") != null:
		mission._ring_mesh.visible = shows("show_mission_rings")
	# Off by default (the historical paint shows); Alt/Option held is a
	# quick look who is who.
	var tint := shows("show_relation_tint") or (not hidden and Input.is_key_pressed(KEY_ALT))
	for m: StandardMaterial3D in _tints:
		m.albedo_color.a = _tints[m] if tint else 0.0
	var highlight: Node3D = main.get("unit_highlight")
	if highlight != null:
		highlight.visible = not hidden
	var marker: Node3D = main.get("target_marker")
	if marker != null and hidden:
		marker.visible = false
