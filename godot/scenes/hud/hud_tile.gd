extends Button
## A square symbol tile of the match HUD (prompt 095): the order grid, the
## support column, the buttons beside the minimap and in the top bar.
##
## It shows a symbol of `hud_icons.gd` instead of text, the key small in a
## corner, a dot for a lasting state (hold fire, repair running), and, for
## support, a field under the square with the price or the seconds left and
## a dark sector over the square while it cools down. Takes no keyboard
## focus.
##
## Look: a rounded plate without a frame, gold while on (pressed or a
## lasting state). A `ghost` tile lies on a panel (order grid, camera
## buttons of the info field): only a faint ground, brighter under the
## pointer; greyed it keeps no ground and a pale symbol. An `accent` tile
## draws its symbol in gold (orders that only show when they fit).

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudIcons := preload("res://scenes/hud/hud_icons.gd")

## Symbol id (`hud_icons.gd`).
var icon_id := "":
	set(value):
		if icon_id != value:
			icon_id = value
			queue_redraw()
## Key shown in the corner ("" for none).
var key_text := "":
	set(value):
		if key_text != value:
			key_text = value
			queue_redraw()
## Stock count badge shown in the bottom-right corner ("" for none, prompt 232).
var badge_text := "":
	set(value):
		if badge_text != value:
			badge_text = value
			queue_redraw()
## Lasting state: a dot in the corner and a gold frame.
var marked := false:
	set(value):
		if marked != value:
			marked = value
			queue_redraw()
## Text of the field under the square (support tiles; "" without field).
var footer := "":
	set(value):
		if footer != value:
			footer = value
			queue_redraw()
var footer_color := HudTheme.GOLD
## Part of the cooldown left, 0..1: a dark sector over the symbol.
var cooldown := 0.0:
	set(value):
		if not is_equal_approx(cooldown, value):
			cooldown = value
			queue_redraw()
## Turn of the symbol in radians (the north needle).
var turn := 0.0:
	set(value):
		if not is_equal_approx(turn, value):
			turn = value
			queue_redraw()
## On a panel: faint ground instead of a dark plate.
var ghost := false:
	set(value):
		if ghost != value:
			ghost = value
			queue_redraw()
## Symbol in gold.
var accent := false:
	set(value):
		if accent != value:
			accent = value
			queue_redraw()
## Whether the tile has a footer field at all (its height is kept).
var with_footer := false
## Side of the square at HUD factor 1.
var side := HudTheme.TILE_PX


func _init(id := "", tile_side := HudTheme.TILE_PX, has_footer := false) -> void:
	icon_id = id
	side = tile_side
	with_footer = has_footer
	focus_mode = Control.FOCUS_NONE
	for state: StringName in [&"normal", &"hover", &"pressed", &"hover_pressed", &"disabled", &"focus"]:
		add_theme_stylebox_override(state, StyleBoxEmpty.new())
	apply_scale()


## Height of the footer field in pixels.
func footer_height() -> float:
	return HudTheme.px(16.0) if with_footer else 0.0


func apply_scale() -> void:
	var s := HudTheme.px(side)
	custom_minimum_size = Vector2(s, s + footer_height())
	size = custom_minimum_size
	queue_redraw()


func _draw() -> void:
	var square := Rect2(Vector2.ZERO, Vector2(size.x, size.y - footer_height()))
	var on := button_pressed or marked
	var fill := HudTheme.GHOST if ghost else HudTheme.TILE
	if disabled:
		fill = Color(0, 0, 0, 0) if ghost else Color(HudTheme.TILE, 0.55)
	elif on:
		fill = HudTheme.GHOST_ON if ghost else HudTheme.TILE_ON
	elif is_hovered():
		fill = HudTheme.GHOST_HOVER if ghost else HudTheme.TILE_HOVER
	if fill.a > 0.0:
		draw_style_box(HudTheme.plate(fill), square)
	var color := HudTheme.GOLD if accent else HudTheme.TEXT
	if disabled:
		color = Color(HudTheme.TEXT_DIM, 0.35)
	elif on:
		color = HudTheme.GOLD.lightened(0.35)
	var inset := square.size.x * 0.2
	HudIcons.draw(self, icon_id, square.grow(-inset), color, maxf(1.6 * HudTheme.scale * side / HudTheme.TILE_PX, 1.2), turn)
	if cooldown > 0.0:
		var middle := square.get_center()
		var radius := square.size.x * 0.5 - 2.0
		var points := PackedVector2Array([middle])
		var steps := maxi(ceili(32.0 * cooldown), 2)
		for k in steps + 1:
			points.append(middle + Vector2.UP.rotated(-TAU * cooldown * k / steps) * radius)
		draw_colored_polygon(points, Color(0, 0, 0, 0.62))
	if marked:
		draw_circle(Vector2(square.size.x - 6.0, 6.0) , maxf(2.5 * HudTheme.scale, 2.0), HudTheme.GOLD)
	var font := ThemeDB.fallback_font
	if not key_text.is_empty():
		var key_size := HudTheme.font(9)
		draw_string(font, Vector2(3.0, square.size.y - 3.0), key_text, HORIZONTAL_ALIGNMENT_LEFT, square.size.x - 4.0, key_size,
			Color(HudTheme.TEXT_DIM, 0.45 if disabled else 0.95))
	if not badge_text.is_empty():
		var badge_size := HudTheme.font(10)
		draw_string(font, Vector2(2.0, square.size.y - 3.0), badge_text, HORIZONTAL_ALIGNMENT_RIGHT, square.size.x - 5.0, badge_size,
			Color(HudTheme.GOLD, 0.45 if disabled else 1.0))
	if with_footer:
		var field := Rect2(0.0, square.size.y + 1.0, size.x, footer_height() - 1.0)
		draw_style_box(HudTheme.plate(HudTheme.TILE), field)
		var text_size := HudTheme.font(9)
		draw_string(font, Vector2(0.0, field.position.y + field.size.y * 0.5 + text_size * 0.36), footer, HORIZONTAL_ALIGNMENT_CENTER,
			size.x, text_size, Color(footer_color, 0.5) if disabled and cooldown <= 0.0 else footer_color)
