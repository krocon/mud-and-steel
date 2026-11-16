extends Control
## The five ratings of a unit type (prompt 066) as a drawing: speed,
## protection, against vehicles, against infantry, against structures.
##
## Two layouts:
## - `STRIP` (picker tile, unit panel): five cells side by side, each a
##   symbol with a bar under it as long as the rating (a mark at the half).
## - `FULL` (tooltip, unit card): a row per rating with symbol, name, bar
##   and number.
##
## Readable without color: every rating has its own symbol (chevrons, a
## shield, a tank, a soldier, a house), the amount is the bar length (and
## its number in the rows); one sand tone for all, so no color
## palette changes the meaning. A rating a type does not have is left out
## (a strip keeps the cell empty, so the columns line up between tiles); a
## rating of 0 shows its symbol dimmed.
##
## Presentation only: `ratings` is the `ratings` section of
## `SimHost.unit_stats` (`unit_stats.gd`).

enum Layout { STRIP, FULL }

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
const STRIP_CELL := Vector2(22, 15)
const STRIP_BAR := 3.0
const STRIP_TRACK := Color(0.0, 0.0, 0.0, 0.7)
const ROW_HEIGHT := 18.0
const ICON := 11.0
const BAR_HEIGHT := 6.0
const FILL := MenuTheme.ACCENT
const TRACK := Color(0.0, 0.0, 0.0, 0.45)
const DIM := Color(MenuTheme.TEXT_DIM, 0.55)

var layout := Layout.STRIP
## Rating name -> 0…100 (only those the type has).
var ratings: Dictionary = {}:
	set(value):
		ratings = value
		_resize()
		queue_redraw()
## Width of the bars in the rows.
var bar_width := 70.0


func _init(with_layout := Layout.STRIP, values: Dictionary = {}) -> void:
	layout = with_layout
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	ratings = values


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED or what == NOTIFICATION_THEME_CHANGED:
		_resize()
		queue_redraw()


## Ratings shown, in display order.
func shown() -> Array[String]:
	var out: Array[String] = []
	for rating in UnitStats.RATINGS:
		if ratings.has(rating):
			out.append(rating)
	return out


func _font_size() -> int:
	return MenuTheme.font(12)


func _label_width() -> float:
	var font := get_theme_default_font()
	var width := 0.0
	for rating in UnitStats.RATINGS:
		width = maxf(width, font.get_string_size(tr(UnitStats.RATING_NAMES[rating]), HORIZONTAL_ALIGNMENT_LEFT, -1, _font_size()).x)
	return width + 8.0


func _number_width() -> float:
	return get_theme_default_font().get_string_size("100", HORIZONTAL_ALIGNMENT_LEFT, -1, _font_size()).x + 6.0


func _resize() -> void:
	if layout == Layout.STRIP:
		# At least the tile's size; a wider strip gets longer bars.
		custom_minimum_size = custom_minimum_size.max(Vector2(STRIP_CELL.x * UnitStats.RATINGS.size(), STRIP_CELL.y))
		return
	var row := maxf(ROW_HEIGHT, _font_size() + 5.0)
	custom_minimum_size = Vector2(ICON + 6.0 + _label_width() + bar_width + _number_width(), row * shown().size())


func _draw() -> void:
	if layout == Layout.STRIP:
		_draw_strip()
	else:
		_draw_rows()


func _draw_strip() -> void:
	var cell := Vector2(size.x / UnitStats.RATINGS.size(), size.y)
	var icon := minf(cell.y - STRIP_BAR - 2.0, 10.0)
	var bar := cell.x - 4.0
	for i in UnitStats.RATINGS.size():
		var rating := UnitStats.RATINGS[i]
		if not ratings.has(rating):
			continue
		var value := clampi(int(ratings[rating]), 0, 100)
		var origin := Vector2(i * cell.x, 0.0)
		draw_symbol(self, rating, Rect2(origin + Vector2((cell.x - icon) * 0.5, 0.0), Vector2(icon, icon)),
				FILL if value > 0 else DIM)
		# The amount as a bar under the symbol, with a mark in the middle.
		var track := Rect2(origin.x + 2.0, cell.y - STRIP_BAR, bar, STRIP_BAR)
		draw_rect(track, STRIP_TRACK)
		draw_rect(Rect2(track.position, Vector2(maxf(bar * value / 100.0, 1.0 if value > 0 else 0.0), STRIP_BAR)), FILL)
		draw_rect(Rect2(track.position.x + bar * 0.5 - 0.5, track.position.y, 1.0, STRIP_BAR), Color(0.0, 0.0, 0.0, 0.6))


func _draw_rows() -> void:
	var font := get_theme_default_font()
	var font_size := _font_size()
	var row := maxf(ROW_HEIGHT, font_size + 5.0)
	var label_width := _label_width()
	var number_width := _number_width()
	var bar := maxf(size.x - ICON - 6.0 - label_width - number_width, 20.0)
	var y := 0.0
	for rating in shown():
		var value := clampi(int(ratings[rating]), 0, 100)
		var middle := y + row * 0.5
		draw_symbol(self, rating, Rect2(0.0, middle - ICON * 0.5, ICON, ICON), FILL if value > 0 else DIM)
		var x := ICON + 6.0
		var baseline := middle + font.get_ascent(font_size) * 0.5 - font.get_descent(font_size) * 0.5
		if label_width > 0.0:
			draw_string(font, Vector2(x, baseline), tr(UnitStats.RATING_NAMES[rating]), HORIZONTAL_ALIGNMENT_LEFT, label_width, font_size, MenuTheme.TEXT)
			x += label_width
		var track := Rect2(x, middle - BAR_HEIGHT * 0.5, bar, BAR_HEIGHT)
		draw_rect(track, TRACK)
		draw_rect(Rect2(track.position, Vector2(bar * value / 100.0, BAR_HEIGHT)), FILL)
		# Quarter marks, so a length reads without the number.
		for quarter in [0.25, 0.5, 0.75]:
			var at: float = track.position.x + bar * quarter
			draw_line(Vector2(at, track.position.y), Vector2(at, track.end.y), Color(0.0, 0.0, 0.0, 0.55), 1.0)
		draw_string(font, Vector2(x + bar, baseline), str(value), HORIZONTAL_ALIGNMENT_RIGHT, number_width, font_size, MenuTheme.TEXT)
		y += row


## Draws the symbol of `rating` into the square `rect` of `canvas`.
static func draw_symbol(canvas: CanvasItem, rating: String, rect: Rect2, color: Color) -> void:
	var p := rect.position
	var s := rect.size.x
	match rating:
		"speed":
			# Two chevrons pointing right.
			for offset: float in [0.0, 0.45]:
				canvas.draw_polyline(PackedVector2Array([
					p + Vector2((0.08 + offset) * s, 0.15 * s), p + Vector2((0.5 + offset) * s, 0.5 * s),
					p + Vector2((0.08 + offset) * s, 0.85 * s)]), color, maxf(s * 0.16, 1.4))
		"protection":
			# A shield.
			canvas.draw_colored_polygon(PackedVector2Array([
				p + Vector2(0.1 * s, 0.08 * s), p + Vector2(0.9 * s, 0.08 * s), p + Vector2(0.9 * s, 0.5 * s),
				p + Vector2(0.5 * s, 0.98 * s), p + Vector2(0.1 * s, 0.5 * s)]), color)
		"anti_vehicle":
			# A tank from the side: tracks, hull, turret, gun.
			canvas.draw_rect(Rect2(p + Vector2(0.0, 0.62 * s), Vector2(s, 0.3 * s)), color)
			canvas.draw_rect(Rect2(p + Vector2(0.12 * s, 0.42 * s), Vector2(0.76 * s, 0.22 * s)), color)
			canvas.draw_rect(Rect2(p + Vector2(0.3 * s, 0.18 * s), Vector2(0.36 * s, 0.26 * s)), color)
			canvas.draw_rect(Rect2(p + Vector2(0.62 * s, 0.24 * s), Vector2(0.38 * s, 0.1 * s)), color)
		"anti_infantry":
			# A soldier: head and shoulders.
			canvas.draw_circle(p + Vector2(0.5 * s, 0.26 * s), 0.22 * s, color)
			canvas.draw_colored_polygon(PackedVector2Array([
				p + Vector2(0.5 * s, 0.46 * s), p + Vector2(0.92 * s, 0.98 * s), p + Vector2(0.08 * s, 0.98 * s)]), color)
		"anti_structure":
			# A house.
			canvas.draw_colored_polygon(PackedVector2Array([
				p + Vector2(0.5 * s, 0.02 * s), p + Vector2(1.0 * s, 0.48 * s), p + Vector2(0.0, 0.48 * s)]), color)
			canvas.draw_rect(Rect2(p + Vector2(0.14 * s, 0.48 * s), Vector2(0.72 * s, 0.5 * s)), color)
