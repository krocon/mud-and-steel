extends VBoxContainer
## Subtitles (prompt 043): a text line for every audible warning (UI notices
## of prompt 029) and battle callout (prompt 053), for players who play
## without sound or hear badly. Setting `subtitles` (off by default
## [provisional]). Bottom center above the unit panel; a line fades after
## `SECONDS`, at most `MAX_LINES` at once. Presentation only.
##
## Other scripts call the static `say(text)`; it does nothing while no match
## HUD exists or the setting is off.

const Settings := preload("res://scenes/menu/settings.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")
const SECONDS := 3.5
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
	custom_minimum_size = Vector2(700, 0)
	offset_left = -350
	offset_right = 350


func _exit_tree() -> void:
	if _instance == self:
		_instance = null


## Shows `text` (already translated) if subtitles are on.
static func say(text: String) -> void:
	if not is_instance_valid(_instance) or not Settings.get_value("subtitles"):
		return
	_instance._add(text)


## The line for a UI notice class, if it is a warning.
static func notice(sound_class: String) -> void:
	if sound_class in NOTICES:
		say(Language.t(NOTICES[sound_class]))


func _add(text: String) -> void:
	while get_child_count() >= MAX_LINES:
		var old := get_child(0)
		remove_child(old)
		old.queue_free()
		_ends.pop_front()
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


func _process(delta: float) -> void:
	_time += delta
	while not _ends.is_empty() and _time >= _ends[0]:
		_ends.pop_front()
		var old := get_child(0)
		remove_child(old)
		old.queue_free()

