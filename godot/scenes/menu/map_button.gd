extends Button
## The "Map" field of the setup screens (prompt 201): shows the chosen
## map's name and opens the map picker (`map_picker.gd`) over `picker_parent`;
## a map chosen there becomes `map_name` and is announced by `map_changed`.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MapPicker := preload("res://scenes/menu/map_picker.gd")

signal map_changed(map_name: String)

## The chosen map (folder name).
var map_name := ""
## Where the picker goes (default: the parent of this button's scene).
var picker_parent: Node
## Passed to `MapPicker.open`.
var filter: Dictionary = {}
## The open picker, or null.
var picker: Control


func _init() -> void:
	alignment = HORIZONTAL_ALIGNMENT_LEFT
	text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	pressed.connect(open_picker)


## Sets the map without the picker (setup, self-tests, command line);
## emits `map_changed` when it changed.
func set_map(name: String) -> void:
	var changed := name != map_name
	map_name = name
	text = "%s  ▾" % MatchSetup.map_title(name)  # l10n-ignore (map name and a marker)
	if changed:
		map_changed.emit(name)


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and not map_name.is_empty():
		text = "%s  ▾" % MatchSetup.map_title(map_name)  # l10n-ignore (map name and a marker)


func open_picker() -> void:
	if picker != null:
		return
	var parent := picker_parent if picker_parent != null else get_tree().current_scene
	picker = MapPicker.open(parent, map_name, filter)
	picker.map_chosen.connect(func(name: String) -> void:
		set_map(name)
		grab_focus())
	picker.tree_exited.connect(func() -> void:
		picker = null
		if is_inside_tree():
			grab_focus())
