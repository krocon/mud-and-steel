extends VBoxContainer
## Subtitles and mission radio messages (prompts 043, 237):
## - Warning/callout subtitles (`say`, `notice`): gated by the `subtitles`
##   setting (off by default [provisional]).
## - Mission radio messages (`radio`): shown by default bottom-center above the
##   unit panel with a clean dark backing strip, optional speaker name and
##   subtle radio-wave tag, auto-hidden when the Hud CanvasLayer hides (`F1`,
##   photo mode, `trailer_shots` with `hud = false`). Presentation only.

const Settings := preload("res://scenes/menu/settings.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
const SECONDS := 3.5
const RADIO_SECONDS := 5.5
const MAX_LINES := 3
## Notice classes (`battle_sound.notice`) that are warnings worth a line,
## with their text key. Clicks (`ui_order`, `ui_select`) are left out.
const NOTICES: Dictionary[String, String] = {
	"ui_under_attack": "SUB_UNDER_ATTACK",
	"ui_unit_lost": "SUB_UNIT_LOST",
	"ui_unit_ready": "SUB_UNIT_READY",
	"ui_build_complete": "SUB_BUILD_COMPLETE",
	"ui_victory": "SUB_VICTORY",
	"ui_defeat": "SUB_DEFEAT",
}

static var _instance: Node

## Lines shown so far (self-test).
var shown := 0
var radio_shown := 0
var _ends: Array[float] = []
var _time := 0.0


func _ready() -> void:
	_instance = self
	name = "Subtitles"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	alignment = BoxContainer.ALIGNMENT_END
	set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM)
	grow_horizontal = Control.GROW_DIRECTION_BOTH
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	offset_bottom = -200
	custom_minimum_size = Vector2(720, 0)
	offset_left = -360
	offset_right = 360
	add_theme_constant_override(&"separation", 4)


func _exit_tree() -> void:
	if _instance == self:
		_instance = null


## Shows `text` (already translated) if subtitles are on.
static func say(text: String) -> void:
	if not is_instance_valid(_instance) or not Settings.get_value("subtitles"):
		return
	_instance._add(text)


## Shows a mission radio message bottom-center (prompt 237; active by default
## during missions so tactical updates are never missed).
static func radio(text: String, speaker := "") -> void:
	if not is_instance_valid(_instance) or text.is_empty():
		return
	_instance._add_radio(text, speaker)


## The line for a UI notice class, if it is a warning.
static func notice(sound_class: String) -> void:
	if sound_class in NOTICES:
		say(Language.t(NOTICES[sound_class]))


func _trim_lines() -> void:
	while get_child_count() >= MAX_LINES:
		var old := get_child(0)
		remove_child(old)
		old.queue_free()
		_ends.pop_front()


func _add(text: String) -> void:
	_trim_lines()
	var label := Label.new()
	label.text = text
	label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_font_size_override("font_size", MenuTheme.font(20))
	label.add_theme_color_override("font_color", Color(1, 0.97, 0.85))
	label.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.9))
	label.add_theme_constant_override("outline_size", 6)
	add_child(label)
	_ends.append(_time + SECONDS)
	shown += 1


func _add_radio(text: String, speaker: String) -> void:
	_trim_lines()
	var panel := PanelContainer.new()
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.05, 0.06, 0.07, 0.82)
	style.border_color = Color(0.92, 0.78, 0.36, 0.75)
	style.border_width_left = 3
	style.border_width_top = 1
	style.border_width_right = 1
	style.border_width_bottom = 1
	style.corner_radius_top_left = 4
	style.corner_radius_top_right = 4
	style.corner_radius_bottom_left = 4
	style.corner_radius_bottom_right = 4
	style.content_margin_left = 14
	style.content_margin_right = 14
	style.content_margin_top = 6
	style.content_margin_bottom = 6
	panel.add_theme_stylebox_override(&"panel", style)
	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override(&"separation", 10)
	panel.add_child(row)
	var tag := Label.new()
	tag.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	tag.text = ("◈ " + speaker.to_upper()) if not speaker.is_empty() else ("◈ " + Language.t("HUD_RADIO_TAG"))
	tag.add_theme_font_size_override("font_size", MenuTheme.font(14))
	tag.add_theme_color_override("font_color", Color(0.98, 0.84, 0.38))
	tag.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.9))
	tag.add_theme_constant_override("outline_size", 4)
	row.add_child(tag)
	var msg := Label.new()
	msg.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	msg.text = text
	msg.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	msg.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	msg.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	msg.add_theme_font_size_override("font_size", MenuTheme.font(16))
	msg.add_theme_color_override("font_color", Color(0.98, 0.96, 0.90))
	msg.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.9))
	msg.add_theme_constant_override("outline_size", 4)
	row.add_child(msg)
	add_child(panel)
	_ends.append(_time + RADIO_SECONDS)
	radio_shown += 1


func _process(delta: float) -> void:
	_time += delta
	while not _ends.is_empty() and _time >= _ends[0]:
		_ends.pop_front()
		var old := get_child(0)
		remove_child(old)
		old.queue_free()
