extends Control
## Preview of a map (prompt 201): the top view in the style of the minimap
## (`SimHost.map_preview_build`: terrain, hill shading, roads, forest,
## buildings, water, bridge decks) with the starts of one start layout
## (number in the team's color), the supply points, a scale bar and a north
## arrow; or, with `show_relief`, the small turnable 3D relief
## (`map_relief.gd`). Used by the map picker, the single-player setup, the
## lobbies and the missions screen. Presentation only.
##
## Pictures are built from the map files without starting a match, in a
## Rust thread (`map_preview_start` / `map_preview_poll`); until one is
## there the control shows a placeholder. Built pictures are cached in
## `user://map_previews/<name>.png` with `<name>.dat` (key from the
## modification times and sizes of the map files, the relief heights, the
## build time); a changed map file, a new `CACHE_VERSION` or `invalidate`
## (the map editor after saving) builds it again.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const MapRelief := preload("res://scenes/menu/map_relief.gd")

const CACHE_DIR := "user://map_previews"
## Raise when the picture changes, so old cached ones are rebuilt.
const CACHE_VERSION := 1
const MAP_FILES: Array[String] = ["map.toml", "height.png", "terrain.png", "objects.csv"]
## Start marks by team (team 0 blue, 1 red, like the minimap's allies and
## enemies), more teams in further colors.
const TEAM_COLORS: Array[Color] = [
	Color(0.35, 0.6, 1.0), Color(1.0, 0.3, 0.22), Color(0.4, 0.85, 0.4), Color(0.95, 0.78, 0.3),
	Color(0.75, 0.5, 0.95), Color(0.3, 0.85, 0.85),
]
const SUPPLY_COLOR := Color("e8d27a")
const OUTLINE := Color(0.06, 0.06, 0.05, 0.9)
## Scale bar lengths to choose from (m).
const SCALE_STEPS: Array[int] = [50, 100, 200, 250, 500, 1000, 2000, 5000]

## Finished previews by map name: `texture`, `relief_columns`,
## `relief_rows`, `relief`, `size_m`, `ms`, `key`; or `error`.
static var _loaded: Dictionary = {}
## Maps whose picture is being built: name → `{dir, key, started_us}`.
static var _pending: Dictionary = {}
## `SimHost.map_preview_info` by map name (with the key it was read for).
static var _infos: Dictionary = {}
## Measured times by map name for the self-test and the docs:
## `build_ms` (in the thread), `cold_ms` (request to picture, uncached),
## `warm_ms` (from the disk cache).
static var timings: Dictionary = {}

## Emitted when the picture of `map_name` is there.
signal preview_ready(map_name: String)

## The map shown ("" = none) and the start layout whose starts are marked.
var map_name := ""
var layout := "standard"
## Whether the 3D relief is shown instead of the top view.
var show_relief := false:
	set(value):
		show_relief = value
		_update_relief()
## Height exaggeration of the relief.
var exaggeration := 2.0:
	set(value):
		exaggeration = value
		if _relief != null:
			_relief.exaggeration = value

var _relief: MapRelief
var _info: Dictionary = {}


func _init() -> void:
	clip_contents = true
	custom_minimum_size = Vector2(160, 160)
	mouse_filter = Control.MOUSE_FILTER_PASS


## Shows `name` with the starts of `variant` (`random` and unknown names
## mark the standard layout).
func show_map(name: String, variant := "standard") -> void:
	map_name = name
	layout = variant
	_info = info(name)
	if not name.is_empty() and exists(name):
		request(name)
	_update_relief()
	queue_redraw()


## Whether the picture of the shown map is there (or failed).
func is_ready() -> bool:
	return _loaded.has(map_name)


## The finished preview of the shown map, {} while it is being built.
func entry() -> Dictionary:
	return _loaded.get(map_name, {})


## Starts of the shown layout: `player`, `team`, `position` (map meters).
func marked_starts() -> Array:
	return _layout().get("starts", [])


## Supply points of the shown layout (map meters).
func marked_supply() -> PackedVector2Array:
	return _layout().get("supply", PackedVector2Array())


func _layout() -> Dictionary:
	var layouts: Array = _info.get("layouts", [])
	for l: Dictionary in layouts:
		if l["name"] == layout:
			return l
	return layouts[0] if not layouts.is_empty() else {}


func _process(_delta: float) -> void:
	if _pending.is_empty():
		return
	var was := is_ready()
	pump()
	if not was and is_ready():
		preview_ready.emit(map_name)
		_update_relief()
		queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _update_relief() -> void:
	var wanted := show_relief and is_ready() and not entry().has("error")
	if wanted and _relief == null:
		_relief = MapRelief.new()
		_relief.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		_relief.exaggeration = exaggeration
		add_child(_relief)
	if _relief != null:
		_relief.visible = wanted
		if wanted:
			_relief.show_relief(entry())
	queue_redraw()


## Where the map lies in the control: as large as fits, centered.
func map_rect() -> Rect2:
	var size_m: Vector2 = _info.get("size_m", Vector2.ONE)
	if size_m.x <= 0.0 or size_m.y <= 0.0:
		return Rect2(Vector2.ZERO, size)
	var scale := minf(size.x / size_m.x, size.y / size_m.y)
	var shown := size_m * scale
	return Rect2((size - shown) * 0.5, shown)


## Pixel of map point `p` (map meters, x east, y south).
func to_pixel(p: Vector2) -> Vector2:
	var rect := map_rect()
	var size_m: Vector2 = _info.get("size_m", Vector2.ONE)
	return rect.position + (p + size_m * 0.5) / size_m * rect.size


func _draw() -> void:
	var rect := map_rect()
	var font := get_theme_default_font()
	var font_size := get_theme_default_font_size()
	if map_name.is_empty():
		return
	var text := ""
	if not exists(map_name):
		text = tr("MAP_PREVIEW_MISSING")
	elif not is_ready():
		text = tr("MAP_PREVIEW_LOADING")
	elif entry().has("error"):
		text = tr("MAP_PREVIEW_FAILED")
	if not text.is_empty():
		draw_rect(rect, Color(MenuTheme.PANEL, 0.9))
		draw_string(font, Vector2(rect.position.x, rect.get_center().y), text, HORIZONTAL_ALIGNMENT_CENTER, rect.size.x, font_size, MenuTheme.TEXT_DIM)
		return
	if show_relief:
		return
	draw_texture_rect(entry()["texture"], rect, false)
	draw_rect(rect, MenuTheme.EDGE, false, 1.0)
	var r := clampf(rect.size.x * 0.022, 7.0, 15.0)
	for p in marked_supply():
		var at := to_pixel(p)
		var d := PackedVector2Array([at + Vector2(0, -r * 0.7), at + Vector2(r * 0.7, 0), at + Vector2(0, r * 0.7), at + Vector2(-r * 0.7, 0)])
		draw_colored_polygon(d, SUPPLY_COLOR)
		d.append(d[0])
		draw_polyline(d, OUTLINE, 1.5)
	var mark_font := maxi(roundi(r * 1.3), 9)
	for s: Dictionary in marked_starts():
		var at := to_pixel(s["position"])
		var team: int = s["team"] if s["team"] >= 0 else s["player"] % 2
		draw_circle(at, r, TEAM_COLORS[team % TEAM_COLORS.size()])
		draw_arc(at, r, 0.0, TAU, 20, OUTLINE, 2.0)
		var number := str(int(s["player"]) + 1)
		var ascent := font.get_ascent(mark_font)
		draw_string(font, at + Vector2(-r, ascent * 0.5 - 1.0), number, HORIZONTAL_ALIGNMENT_CENTER, r * 2.0, mark_font, Color.WHITE)
	_draw_scale(rect, font, font_size)
	_draw_north(rect, font, font_size)


## Scale bar in the bottom left corner: a round length of about a quarter
## of the map's width.
func _draw_scale(rect: Rect2, font: Font, font_size: int) -> void:
	var size_m: Vector2 = _info.get("size_m", Vector2.ONE)
	var length := SCALE_STEPS[0]
	for step in SCALE_STEPS:
		if step <= size_m.x * 0.28:
			length = step
	var px := length / size_m.x * rect.size.x
	var small := maxi(roundi(font_size * 0.75), 10)
	var origin := rect.position + Vector2(10, rect.size.y - 12)
	var label := "%d m" % length if length < 1000 else "%d km" % int(length / 1000.0)
	var text_width := font.get_string_size(label, HORIZONTAL_ALIGNMENT_LEFT, -1, small).x
	var box := maxf(px, text_width)
	draw_rect(Rect2(origin - Vector2(4, small + 6), Vector2(box + 8, small + 12)), Color(0, 0, 0, 0.45))
	draw_line(origin, origin + Vector2(px, 0), Color.WHITE, 2.0)
	for x in [0.0, px]:
		draw_line(origin + Vector2(x, -5), origin + Vector2(x, 1), Color.WHITE, 2.0)
	draw_string(font, origin + Vector2((box - text_width) * 0.5, -6), label, HORIZONTAL_ALIGNMENT_LEFT, -1, small, Color.WHITE)


## North arrow in the top right corner (north is up: row 0 of the picture).
func _draw_north(rect: Rect2, font: Font, font_size: int) -> void:
	var small := maxi(roundi(font_size * 0.75), 10)
	var at := Vector2(rect.end.x - 18, rect.position.y + 14 + small)
	draw_circle(at + Vector2(0, -4), 13.0, Color(0, 0, 0, 0.45))
	var arrow := PackedVector2Array([at + Vector2(0, -14), at + Vector2(6, 4), at + Vector2(0, 0), at + Vector2(-6, 4)])
	draw_colored_polygon(arrow, Color.WHITE)
	draw_string(font, at + Vector2(-10, 4 + small), "N", HORIZONTAL_ALIGNMENT_CENTER, 20, small, Color.WHITE)  # l10n-ignore (compass letter)


# --- Pictures and cache (static) ---------------------------------------


## Folder of map `name` ("" if it has none, e.g. a map only the host has).
static func folder(name: String) -> String:
	var dir := GamePaths.map_dir(name)
	return dir if FileAccess.file_exists(dir.path_join("map.toml")) else ""


## Whether the map `name` is there to be shown.
static func exists(name: String) -> bool:
	return not folder(name).is_empty()


## `SimHost.map_preview_info` of map `name` (cached until its files
## change): `size_m`, `biome`, `weather`, `daytime`, `layouts`.
static func info(name: String) -> Dictionary:
	if not exists(name):
		return {}
	var key := cache_key(name)
	var known: Dictionary = _infos.get(name, {})
	if known.get("key", "") != key:
		known = {"key": key, "info": SimHost.map_preview_info(folder(name))}
		_infos[name] = known
	return known["info"]


## Identifies the state of the map's files: modification time and size
## of each, with the cache version.
static func cache_key(name: String) -> String:
	var dir := folder(name)
	var parts: PackedStringArray = ["v%d" % CACHE_VERSION]
	for file in MAP_FILES:
		var path := dir.path_join(file)
		var handle := FileAccess.open(path, FileAccess.READ)
		if handle == null:
			continue
		parts.append("%s:%d:%d" % [file, FileAccess.get_modified_time(path), handle.get_length()])
	return "|".join(parts)


## The finished preview of `name` if it is in memory or valid on disk,
## else {}.
static func cached(name: String) -> Dictionary:
	var key := cache_key(name)
	var entry: Dictionary = _loaded.get(name, {})
	if not entry.is_empty() and entry.get("key", "") == key:
		return entry
	_loaded.erase(name)
	var started := Time.get_ticks_usec()
	var data_path := _cache_path(name, "dat")
	var png_path := _cache_path(name, "png")
	if not FileAccess.file_exists(data_path) or not FileAccess.file_exists(png_path):
		return {}
	var file := FileAccess.open(data_path, FileAccess.READ)
	var data: Variant = file.get_var() if file != null else null
	if not data is Dictionary or (data as Dictionary).get("key", "") != key:
		return {}
	var image := Image.load_from_file(ProjectSettings.globalize_path(png_path))
	if image == null or image.is_empty():
		return {}
	entry = data
	entry["texture"] = ImageTexture.create_from_image(image)
	_loaded[name] = entry
	_time(name, "warm_ms", (Time.get_ticks_usec() - started) / 1000.0)
	return entry


## The preview of `name` if it is there; else starts building it in the
## background and returns {} (`pump` collects it).
static func request(name: String) -> Dictionary:
	var entry := cached(name)
	if not entry.is_empty() or not exists(name):
		return entry
	if not _pending.has(name):
		var dir := folder(name)
		_pending[name] = {"dir": dir, "key": cache_key(name), "started_us": Time.get_ticks_usec()}
		SimHost.map_preview_start(dir)
	return {}


## Collects the pictures built in the background.
static func pump() -> void:
	for name: String in _pending.keys():
		var job: Dictionary = _pending[name]
		var result: Dictionary = SimHost.map_preview_poll(job["dir"])
		if result.is_empty():
			continue
		_pending.erase(name)
		_store(name, result, job["key"])
		_time(name, "cold_ms", (Time.get_ticks_usec() - int(job["started_us"])) / 1000.0)


## Builds the preview of `name` now, on this thread (self-test, export).
static func build_now(name: String) -> Dictionary:
	if not exists(name):
		return {}
	var started := Time.get_ticks_usec()
	_store(name, SimHost.map_preview_build(folder(name)), cache_key(name))
	_time(name, "cold_ms", (Time.get_ticks_usec() - started) / 1000.0)
	return _loaded[name]


## Forgets the cached preview of `name` (the map editor after saving it).
static func invalidate(name: String) -> void:
	_loaded.erase(name)
	_infos.erase(name)
	for ext in ["png", "dat"]:
		var path := _cache_path(name, ext)
		if FileAccess.file_exists(path):
			DirAccess.remove_absolute(ProjectSettings.globalize_path(path))


## Bytes the cache takes on disk.
static func cache_bytes() -> int:
	var total := 0
	var dir := DirAccess.open(CACHE_DIR)
	if dir == null:
		return 0
	for file in dir.get_files():
		var handle := FileAccess.open(CACHE_DIR.path_join(file), FileAccess.READ)
		if handle != null:
			total += handle.get_length()
	return total


static func _store(name: String, result: Dictionary, key: String) -> void:
	if result.has("error") or int(result.get("width", 0)) <= 0:
		push_warning("map preview of %s failed: %s" % [name, result.get("error", "empty picture")])
		_loaded[name] = {"error": result.get("error", "empty"), "key": key}
		return
	var image := Image.create_from_data(result["width"], result["height"], false, Image.FORMAT_RGB8, result["rgb"])
	var entry := {
		"key": key,
		"width": result["width"],
		"height": result["height"],
		"relief_columns": result["relief_columns"],
		"relief_rows": result["relief_rows"],
		"relief": result["relief"],
		"size_m": result["size_m"],
		"ms": result["ms"],
	}
	_time(name, "build_ms", result["ms"])
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(CACHE_DIR))
	if image.save_png(_cache_path(name, "png")) == OK:
		var file := FileAccess.open(_cache_path(name, "dat"), FileAccess.WRITE)
		if file != null:
			file.store_var(entry)
	entry["texture"] = ImageTexture.create_from_image(image)
	_loaded[name] = entry


static func _cache_path(name: String, ext: String) -> String:
	return CACHE_DIR.path_join("%s.%s" % [name, ext])


static func _time(name: String, what: String, ms: float) -> void:
	var t: Dictionary = timings.get(name, {})
	t[what] = ms
	timings[name] = t
