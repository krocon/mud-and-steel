extends Control
## Rank insignia of a unit (prompt 050), drawn in the style of the time:
##
## - `de`: silver-grey chevrons pointing down on a field-grey patch (the
##   sleeve "Winkel" of the Heer), the elite with a pip above them;
## - `us`: khaki chevrons pointing up on an olive patch (sleeve chevrons);
## - `su`: a khaki shoulder board with red piping and red transverse
##   stripes (the "lychki" of NCO shoulder boards from 1943);
## - `uk`: white tape chevrons pointing down on a khaki battledress patch
##   (the British army's sleeve chevrons; prompt 117).
##
## Ranks are the game's (`data/experience.toml`), not the army ranks the
## insignia belonged to: one, two, three marks for experienced, veteran and
## elite. A recruit gets a plain patch. Presentation only.

var faction := "de":
	set(value):
		faction = value
		queue_redraw()
var rank := 0:
	set(value):
		rank = value
		queue_redraw()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_PASS


func _draw() -> void:
	draw_badge(self, Rect2(Vector2.ZERO, size), faction, rank)


## Draws the insignia of `rank` for `faction` into `rect` of `canvas`.
static func draw_badge(canvas: CanvasItem, rect: Rect2, faction: String, rank: int) -> void:
	match faction:
		"su":
			_shoulder_board(canvas, rect, rank)
		"us":
			_chevrons(canvas, rect, rank, Color(0.24, 0.25, 0.17), Color(0.8, 0.72, 0.45), true)
		"uk":
			_chevrons(canvas, rect, rank, Color(0.36, 0.32, 0.22), Color(0.9, 0.88, 0.82), false, true)
		_:
			_chevrons(canvas, rect, rank, Color(0.36, 0.37, 0.29), Color(0.8, 0.8, 0.76), false)


static func _chevrons(canvas: CanvasItem, rect: Rect2, rank: int, field: Color, mark: Color,
		up: bool, three := false) -> void:
	canvas.draw_rect(rect, field)
	canvas.draw_rect(rect, Color(0, 0, 0, 0.6), false, 1.0)
	# German sleeve insignia have at most two chevrons; the elite gets a pip.
	# British chevrons point down too, up to three (prompt 117).
	var pip_for_elite := not up and not three
	var count := mini(rank, 2) if pip_for_elite else mini(rank, 3)
	if count <= 0:
		return
	var width := maxf(rect.size.y * 0.13, 1.5)
	var gap := rect.size.y * 0.22
	var depth := rect.size.y * 0.24
	var pad := rect.size.x * 0.18
	var total := (count - 1) * gap + depth
	var top := rect.position.y + (rect.size.y - total) / 2.0
	if pip_for_elite and rank >= 3:
		top += rect.size.y * 0.12
	for i in count:
		var y := top + i * gap
		var left := Vector2(rect.position.x + pad, y if not up else y + depth)
		var middle := Vector2(rect.get_center().x, y + depth if not up else y)
		var right := Vector2(rect.end.x - pad, left.y)
		canvas.draw_polyline(PackedVector2Array([left, middle, right]), mark, width, true)
	if pip_for_elite and rank >= 3:
		var pip := Vector2(rect.get_center().x, rect.position.y + rect.size.y * 0.2)
		canvas.draw_circle(pip, maxf(rect.size.y * 0.09, 1.5), mark)


static func _shoulder_board(canvas: CanvasItem, rect: Rect2, rank: int) -> void:
	var board := rect.grow_individual(-rect.size.x * 0.08, -1, -rect.size.x * 0.08, -1)
	canvas.draw_rect(board, Color(0.48, 0.45, 0.33))
	canvas.draw_rect(board, Color(0.6, 0.12, 0.12), false, maxf(rect.size.y * 0.08, 1.0))
	var count := mini(rank, 3)
	var height := maxf(rect.size.y * 0.11, 1.5)
	var gap := rect.size.y * 0.2
	var total := (count - 1) * gap
	for i in count:
		var y := board.get_center().y - total / 2.0 + i * gap - height / 2.0
		canvas.draw_rect(Rect2(board.position.x + 2, y, board.size.x - 4, height), Color(0.66, 0.15, 0.14))
