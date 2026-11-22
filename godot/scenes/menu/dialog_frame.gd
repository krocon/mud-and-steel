extends Container
## Frame of a menu dialog: fills the screen and places its panel (the
## first visible child) centered, as wide as the panel needs (at most the
## room there is) and, by `fill`:
##
## - `true`: the whole height of the screen less the margin. The panel
##   gives the extra room to its part that grows (a list, the tabs), so a
##   large window shows more instead of a scroll bar.
## - `false`: as high as the content wants, at most the screen. Put the
##   content in a `ScrollContainer` inside the panel and name it `content`:
##   on a small window it scrolls instead of running off the screen.
##
## Replaces a `CenterContainer` with a fixed minimum size, which left room
## unused on large windows and cut dialogs off on small ones.

## Whether the panel takes the whole height.
var fill := true
## The control inside the panel's `ScrollContainer` (fit mode).
var content: Control:
	set(value):
		content = value
		if content != null:
			content.minimum_size_changed.connect(queue_sort)


## Puts `inner` into a vertical `ScrollContainer` inside `panel` and makes
## it the content of this frame (fit mode).
func scroll_in(panel: Control, inner: Control) -> ScrollContainer:
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	panel.add_child(scroll)
	inner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(inner)
	content = inner
	return scroll


func _init(fill_height := true) -> void:
	fill = fill_height
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


## Space kept free around the panel.
func margin() -> float:
	return roundf(clampf(size.y * 0.04, 12.0, 48.0))


func _notification(what: int) -> void:
	if what == NOTIFICATION_SORT_CHILDREN:
		_place()


func _place() -> void:
	var room := size - Vector2.ONE * 2.0 * margin()
	for child in get_children():
		var panel := child as Control
		if panel == null or not panel.visible:
			continue
		var wanted := panel.get_combined_minimum_size()
		var height := room.y
		if not fill:
			var inner := content.get_combined_minimum_size().y if content != null else 0.0
			height = minf(wanted.y + inner, room.y)
		var width := minf(wanted.x, room.x)
		fit_child_in_rect(panel, Rect2((size - Vector2(width, height)) * 0.5, Vector2(width, height)).abs())
		return


func _get_minimum_size() -> Vector2:
	return Vector2.ZERO
