extends RefCounted
## Procedural textures for the effects (prompt 019): smoke puff, fire puff,
## spark, earth clod and scorch mark. Built once at run time from noise, so
## the effects need no image files and no downloads. Presentation only.

const SIZE := 128

static var _cache: Dictionary[String, Texture2D] = {}


## Soft, cloudy puff: radial falloff times fractal noise. White, so the
## particle color and the instance tint color it.
static func smoke() -> Texture2D:
	return _cached("smoke", func() -> Image: return _puff(SIZE, 7, 0.55, 1.5))


## Denser, sharper puff for flames and fireballs.
static func fire() -> Texture2D:
	return _cached("fire", func() -> Image: return _puff(SIZE, 11, 0.4, 1.4))


## Round glow with a hot core (sparks, muzzle flash).
static func spark() -> Texture2D:
	return _cached("spark", func() -> Image:
		var image := Image.create(32, 32, false, Image.FORMAT_RGBA8)
		for y in 32:
			for x in 32:
				var r := Vector2(x - 15.5, y - 15.5).length() / 15.5
				var a := clampf(1.0 - r, 0.0, 1.0)
				image.set_pixel(x, y, Color(1, 1, 1, a * a * (0.5 + 0.5 * a)))
		return image)


## Star-shaped flash for the muzzle: a bright core with a few rays.
static func flash() -> Texture2D:
	return _cached("flash", func() -> Image:
		var image := Image.create(64, 64, false, Image.FORMAT_RGBA8)
		for y in 64:
			for x in 64:
				var d := Vector2(x - 31.5, y - 31.5)
				var r := d.length() / 31.5
				var rays := pow(absf(cos(d.angle() * 3.0)), 8.0) * 0.6
				var a := clampf(1.0 - r / (0.45 + rays), 0.0, 1.0)
				image.set_pixel(x, y, Color(1, 1, 1, a * a))
		return image)


## Irregular opaque clump (earth clods, debris sprites).
static func clod() -> Texture2D:
	return _cached("clod", func() -> Image:
		var noise := _noise(5, 0.09)
		var image := Image.create(32, 32, false, Image.FORMAT_RGBA8)
		for y in 32:
			for x in 32:
				var r := Vector2(x - 15.5, y - 15.5).length() / 15.5
				var n := noise.get_noise_2d(x, y) * 0.5 + 0.5
				var a := 1.0 if r < 0.45 + 0.4 * n else 0.0
				var shade := 0.7 + 0.3 * n
				image.set_pixel(x, y, Color(shade, shade, shade, a))
		return image)


## Dark burn mark with ragged edges and a few radial streaks (decals).
static func scorch() -> Texture2D:
	return _cached("scorch", func() -> Image:
		var noise := _noise(13, 0.05)
		var image := Image.create(SIZE, SIZE, false, Image.FORMAT_RGBA8)
		var half := SIZE * 0.5 - 0.5
		for y in SIZE:
			for x in SIZE:
				var d := Vector2(x - half, y - half)
				var r := d.length() / half
				var n := noise.get_noise_2d(x, y) * 0.5 + 0.5
				var streak := pow(absf(sin(d.angle() * 7.0 + n * 3.0)), 6.0) * 0.25
				var a := clampf((1.0 - r) * 1.6 + n * 0.5 - 0.35 + streak * (1.0 - r), 0.0, 1.0)
				# Darker, almost black core; brown-grey edge.
				var core := clampf(1.0 - r * 1.4, 0.0, 1.0)
				var c := Color(0.09, 0.075, 0.06).lerp(Color(0.02, 0.018, 0.016), core)
				image.set_pixel(x, y, Color(c.r, c.g, c.b, a * 0.92))
		image.generate_mipmaps()
		return image)


static func _cached(key: String, build: Callable) -> Texture2D:
	if not _cache.has(key):
		var image: Image = build.call()
		if not image.has_mipmaps():
			image.generate_mipmaps()
		_cache[key] = ImageTexture.create_from_image(image)
	return _cache[key]


static func _noise(seed_value: int, frequency: float) -> FastNoiseLite:
	var noise := FastNoiseLite.new()
	noise.seed = seed_value
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 4
	noise.frequency = frequency
	return noise


## A puff: alpha = radial falloff (with noisy edge) × noise; the color gets a
## little darker where the noise is thin, which reads as volume.
static func _puff(size: int, seed_value: int, edge_noise: float, falloff: float) -> Image:
	var noise := _noise(seed_value, 3.2 / size)
	var image := Image.create(size, size, false, Image.FORMAT_RGBA8)
	var half := size * 0.5 - 0.5
	for y in size:
		for x in size:
			var r := Vector2(x - half, y - half).length() / half
			var n := noise.get_noise_2d(x, y) * 0.5 + 0.5
			var edge := clampf(1.0 - r - (n - 0.5) * edge_noise, 0.0, 1.0)
			var a := pow(edge, falloff) * (0.55 + 0.45 * n)
			var shade := 0.78 + 0.22 * n
			image.set_pixel(x, y, Color(shade, shade, shade, clampf(a * 2.0, 0.0, 1.0)))
	return image


static func clear_cache() -> void:
	_cache.clear()
