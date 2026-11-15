class_name UnitMarkings
extends RefCounted
## Generates marking textures at runtime: Balkenkreuz (de), red star (su),
## white star (us) and tactical numbers. Drawn from simple geometry, so no
## image files and no third-party artwork are needed. Never swastikas or SS
## runes (docs/SOURCES.md).

const SIZE := 128

static var _cache: Dictionary[String, ImageTexture] = {}

## 3x5 pixel digits, rows top to bottom, bits left to right.
const DIGITS := [
	[7, 5, 5, 5, 7], [2, 6, 2, 2, 7], [7, 1, 7, 4, 7], [7, 1, 7, 1, 7], [5, 5, 7, 1, 1],
	[7, 4, 7, 1, 7], [7, 4, 7, 5, 7], [7, 1, 1, 2, 2], [7, 5, 7, 5, 7], [7, 5, 7, 1, 7],
]


static func national_texture(faction: String) -> ImageTexture:
	var key := "nat_" + faction
	if _cache.has(key):
		return _cache[key]
	var img := Image.create_empty(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	match faction:
		"de":
			# Balkenkreuz: black cross with white edges
			_cross(img, 0.5, 0.42, Color(0.92, 0.92, 0.9))
			_cross(img, 0.42, 0.24, Color(0.05, 0.05, 0.05))
		"su":
			_star(img, 0.48, Color(0.95, 0.95, 0.92))
			_star(img, 0.43, Color(0.72, 0.08, 0.07))
		"us":
			_ring(img, 0.47, 0.41, Color(0.93, 0.93, 0.9))
			_star(img, 0.39, Color(0.93, 0.93, 0.9))
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	return tex


static func number_texture(number: int) -> ImageTexture:
	var text := str(number)
	var key := "num_" + text
	if _cache.has(key):
		return _cache[key]
	var px := 8  # pixels per font dot
	var w := text.length() * 4 * px + px
	var h := 7 * px
	var img := Image.create_empty(w, h, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	for i in text.length():
		var rows: Array = DIGITS[int(text[i])]
		for r in 5:
			for c in 3:
				if int(rows[r]) & (4 >> c):
					img.fill_rect(Rect2i(px + (i * 4 + c) * px, px + r * px, px, px), Color(0.93, 0.93, 0.9))
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	return tex


static func _cross(img: Image, half_len: float, half_width: float, col: Color) -> void:
	var c := SIZE / 2.0
	var l := int(half_len * SIZE)
	var w := int(half_width * SIZE * 0.5)
	img.fill_rect(Rect2i(int(c) - l, int(c) - w, 2 * l, 2 * w), col)
	img.fill_rect(Rect2i(int(c) - w, int(c) - l, 2 * w, 2 * l), col)


static func _star(img: Image, radius: float, col: Color) -> void:
	var pts := PackedVector2Array()
	for i in 10:
		var r := radius * SIZE * (1.0 if i % 2 == 0 else 0.382)
		var a := -PI / 2.0 + PI * i / 5.0
		pts.append(Vector2(SIZE / 2.0 + cos(a) * r, SIZE / 2.0 + sin(a) * r))
	for y in SIZE:
		for x in SIZE:
			if Geometry2D.is_point_in_polygon(Vector2(x + 0.5, y + 0.5), pts):
				img.set_pixel(x, y, col)


static func _ring(img: Image, outer: float, inner: float, col: Color) -> void:
	var c := Vector2(SIZE / 2.0, SIZE / 2.0)
	for y in SIZE:
		for x in SIZE:
			var d := Vector2(x + 0.5, y + 0.5).distance_to(c) / SIZE
			if d <= outer and d >= inner:
				img.set_pixel(x, y, col)
