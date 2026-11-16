extends CanvasLayer
## Loading screen (prompt 027): shown by `main.gd` while it builds a match
## started from the menu or the LAN lobby. `main.gd` builds the match in
## steps (simulation and path grid, terrain, atmosphere, map objects) and
## waits a frame after each, so this screen is drawn between them.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")

var _step: Label
var _bar: ProgressBar
var _started_ms := Time.get_ticks_msec()


func _init(map_name: String) -> void:
	layer = 100
	var root := Control.new()
	root.theme = MenuTheme.get_theme()
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	# Swallow the mouse while loading.
	root.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(root)
	root.add_child(MenuTheme.background())
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.add_child(center)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(520, 0)
	center.add_child(box)
	box.add_child(MenuTheme.title("LOADING_TITLE", MenuTheme.HEADING_SIZE))
	var info := MenuTheme.hint(tr("LOADING_MAP") % map_name)
	info.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(info)
	_bar = ProgressBar.new()
	_bar.custom_minimum_size = Vector2(0, 18)
	_bar.show_percentage = false
	_bar.max_value = 1.0
	box.add_child(_bar)
	_step = MenuTheme.hint("")
	_step.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(_step)
	MatchSetup.last_loading_steps = []


## Shows the step about to run (a translation key or text) and how far
## loading is (0–1).
func set_step(text: String, fraction: float) -> void:
	_step.text = text
	_bar.value = fraction
	MatchSetup.last_loading_steps.append(text)


func finish() -> void:
	print("Mud and Steel: loaded in %d ms" % (Time.get_ticks_msec() - _started_ms))
	queue_free()
