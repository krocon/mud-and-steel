extends Control
## Resource curves of one player for the result screen (prompt 045): the
## account of fuel, ammunition and manpower over the match, from the
## samples of `SimHost.match_stats()` (every 30 s of game time). Drawn by
## hand: three polylines on a dim frame, no axes text (the legend is a
## label of the result screen).

## Line colors of fuel, ammunition and manpower (also used by the legend).
const COLORS: Array[Color] = [Color("d9a441"), Color("c8665a"), Color("8fb1c9")]
const FRAME := Color(1, 1, 1, 0.18)

## Array of sample dictionaries (`seconds`, `account`).
var samples: Array = []


func _init() -> void:
	custom_minimum_size = Vector2(420, 110)


func set_samples(new_samples: Array) -> void:
	samples = new_samples
	queue_redraw()


func _draw() -> void:
	var area := Rect2(Vector2(4, 4), size - Vector2(8, 8))
	draw_rect(area, FRAME, false, 1.0)
	if samples.size() < 2:
		return
	var last_second: float = maxf(float(samples[-1]["seconds"]), 1.0)
	var top := 1.0
	for sample: Dictionary in samples:
		for value: int in sample["account"]:
			top = maxf(top, float(value))
	for resource in 3:
		var points := PackedVector2Array()
		for sample: Dictionary in samples:
			var x: float = area.position.x + area.size.x * float(sample["seconds"]) / last_second
			var account: PackedInt64Array = sample["account"]
			var y: float = area.end.y - area.size.y * float(account[resource]) / top
			points.append(Vector2(x, y))
		draw_polyline(points, COLORS[resource], 2.0, true)
