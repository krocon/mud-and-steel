extends HBoxContainer
## Control groups as small tabs (prompt 095), top center: one tab per group
## that has living units, with its number. A click recalls the group like
## its key (1…9; twice quickly also centers the camera); the tab of the
## group that is selected right now is lit. Nothing shows without groups.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const REFRESH := 0.2

var main: Node3D
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Tab per group number (self-test).
var tabs: Dictionary[int, Button] = {}

var _refresh_left := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "GroupTabs"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	alignment = BoxContainer.ALIGNMENT_CENTER
	for number in range(1, 10):
		var tab := Button.new()
		tab.name = "Group_%d" % number
		tab.text = str(number)
		tab.toggle_mode = true
		tab.visible = false
		tab.tooltip_text = "HUD_TIP_GROUP"
		tab.pressed.connect(func() -> void:
			main.selection.recall_group(number)
			refresh())
		add_child(tab)
		tabs[number] = tab
	apply_scale()


## Sizes for the HUD factor (`hud.gd` calls it when the factor changed).
func apply_scale() -> void:
	add_theme_constant_override(&"separation", int(HudTheme.px(HudTheme.GAP)))
	for number: int in tabs:
		var tab := tabs[number]
		HudTheme.style_button(tab, 14.0)
		tab.custom_minimum_size = Vector2(HudTheme.px(28), HudTheme.px(26))
	size = Vector2.ZERO


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		refresh()
	usec += Time.get_ticks_usec() - began


## Shows the tabs of the groups that still have a living unit.
func refresh() -> void:
	var selection: Control = main.selection
	var selected: Array = selection.selected.duplicate()
	selected.sort()
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var living := sim_host.unit_alive()
	for number: int in tabs:
		var alive: Array = []
		for id: int in selection.groups.get(number, []):
			var index := ids.find(id)
			if index >= 0 and living[index] != 0:
				alive.append(id)
		alive.sort()
		tabs[number].visible = not alive.is_empty()
		tabs[number].set_pressed_no_signal(not alive.is_empty() and alive == selected)
