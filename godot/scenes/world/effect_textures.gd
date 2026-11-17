extends RefCounted
## Procedural textures for the effects (prompt 019): smoke puff, fire puff,
## spark, earth clod and scorch mark; since prompt 072 also billows (a 2 x 2
## atlas of cauliflower clusters with a baked normal), earth streaks and a
## ray burst. Built once at run time from noise, so the effects need no image
## files and no downloads. Presentation only.

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


## Billows (prompt 072): a 2 x 2 atlas of clusters of 6 to 9 round lumps,
## like the cauliflower smoke of the reference (lumps of 1.2 to 1.8 m in a
## cloud of 6 to 12 m). Per pixel: R, G = the lump's normal (x right, y up,
## 0.5 = flat), B = shade (dark in the creases between lumps and in thin
## noise), A = density with a noisy edge. `effect_library.gd` lights it with
## the real sun ("billow" shaders) or colours it as fire ("fireball").
static func billow() -> Texture2D:
	return _cached("billow", func() -> Image:
		var cell := SIZE
		var image := Image.create(cell * 2, cell * 2, false, Image.FORMAT_RGBA8)
		var noise := _noise(23, 4.5 / cell)
		var rng := RandomNumberGenerator.new()
		rng.seed = 72
		for c in 4:
			var ox := (c % 2) * cell
			var oy := (c / 2) * cell
			# Lumps: centre (cell units, -1..1, y up) and radius.
			var lumps: Array[Vector3] = [Vector3(0.0, -0.05, 0.5)]
			for k in rng.randi_range(5, 8):
				var a := rng.randf() * TAU
				var d := rng.randf_range(0.2, 0.5)
				lumps.append(Vector3(cos(a) * d, sin(a) * d * 0.85 + 0.05, rng.randf_range(0.24, 0.4)))
			for y in cell:
				for x in cell:
					var p := Vector2((x + 0.5) / cell * 2.0 - 1.0, 1.0 - (y + 0.5) / cell * 2.0)
					var n := noise.get_noise_2d(x + ox, y + oy) * 0.5 + 0.5
					# The lump whose surface is highest here wins.
					# Soft union of the lumps: height = highest surface, normal =
					# blend weighted towards the highest one, so the seams between
					# lumps stay soft (hard seams read as facets).
					var best := -1.0
					var normal := Vector3.ZERO
					var weight := 0.0
					for lump in lumps:
						var q := (p - Vector2(lump.x, lump.y)) / lump.z
						var r2 := q.length_squared()
						if r2 >= 1.0:
							continue
						var h := sqrt(1.0 - r2) * lump.z
						best = maxf(best, h)
						var w := exp(h * 30.0)
						normal += Vector3(q.x, q.y, sqrt(1.0 - r2)) * w
						weight += w
					if weight > 0.0:
						normal = (normal / weight).normalized()
					var alpha := 0.0
					var shade := 1.0
					if best > 0.0:
						# Soft, noisy rim.
						alpha = clampf(best * 3.5 - 0.15 + (n - 0.5) * 1.3, 0.0, 1.0)
						alpha *= clampf((1.0 - p.length()) * 5.0, 0.0, 1.0)
						# Thin noise and the low parts of the cluster are darker.
						shade = clampf((0.6 + 0.4 * n) * (0.75 + 0.5 * best), 0.0, 1.0)
					var nx := normal.x * 0.5 + 0.5 if best > 0.0 else 0.5
					var ny := normal.y * 0.5 + 0.5 if best > 0.0 else 0.5
					image.set_pixel(ox + x, oy + y, Color(nx, ny, shade, alpha))
		return image)


## An elongated, lumpy streak of earth (the dark rays of an impact's earth
## fan, prompt 072): narrow at the bottom (the impact), wider and lumpy
## towards the tip. White, coloured by the particle and the ground tint.
static func streak() -> Texture2D:
	return _cached("streak", func() -> Image:
		var w := 32
		var h := 128
		var noise := _noise(31, 0.12)
		var image := Image.create(w, h, false, Image.FORMAT_RGBA8)
		for y in h:
			for x in w:
				var v := 1.0 - (y + 0.5) / h
				var half := 0.25 + 0.6 * v * (1.0 - v * 0.55)
				var u := absf((x + 0.5) / w * 2.0 - 1.0)
				var n := noise.get_noise_2d(x, y) * 0.5 + 0.5
				var a := clampf((half - u) * 4.0 + (n - 0.5) * 1.6, 0.0, 1.0)
				a *= clampf(v * 6.0, 0.0, 1.0) * clampf((1.0 - v) * 5.0, 0.0, 1.0)
				var shade := 0.7 + 0.3 * n
				image.set_pixel(x, y, Color(shade, shade, shade, a))
		return image)


## A burst of thin rays round a bright core (the first tenths of a second of
## a vehicle explosion, prompt 072).
static func rays() -> Texture2D:
	return _cached("rays", func() -> Image:
		var size := 128
		var noise := _noise(41, 0.05)
		var image := Image.create(size, size, false, Image.FORMAT_RGBA8)
		var half := size * 0.5 - 0.5
		for y in size:
			for x in size:
				var d := Vector2(x - half, y - half)
				var r := d.length() / half
				var angle := d.angle()
				var ray := pow(absf(cos(angle * 7.0 + noise.get_noise_1d(angle * 40.0) * 2.0)), 14.0)
				var reach := 0.35 + 0.6 * ray * (0.6 + 0.4 * (noise.get_noise_2d(angle * 30.0, 0.0) * 0.5 + 0.5))
				var a := clampf(1.0 - r / reach, 0.0, 1.0)
				var core := clampf(1.0 - r / 0.4, 0.0, 1.0)
				image.set_pixel(x, y, Color(1, 1, 1, clampf(a * a + core, 0.0, 1.0)))
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
