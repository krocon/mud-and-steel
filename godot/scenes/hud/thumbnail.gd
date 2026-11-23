extends RefCounted
## Pre-rendered pictures of everything a player can build or produce
## (prompt 058): `res://assets/thumbnails/<catalog id>.png`, rendered by
## `just thumbnails` (`scenes/tools/thumbnail_render.gd`) with a transparent
## background, so the picker tiles and the unit panel draw their own
## faction-tinted backdrop behind them. Checked by `just gallery-check`.

const DIR := "res://assets/thumbnails/"
## Pixel size of every picture (3 : 2, twice the largest tile size).
const SIZE := Vector2i(240, 160)

## Backdrop colors per faction (the tile background behind the picture).
const FACTION_COLORS: Dictionary[String, Color] = {
	"de": Color(0.32, 0.33, 0.33),
	"su": Color(0.36, 0.25, 0.22),
	"us": Color(0.27, 0.32, 0.24),
	"uk": Color(0.33, 0.31, 0.24),
}
const NEUTRAL_COLOR := Color(0.28, 0.29, 0.31)

static var _cache: Dictionary[String, Texture2D] = {}


static func path(type_id: String) -> String:
	return DIR + type_id + ".png"


static func exists(type_id: String) -> bool:
	return ResourceLoader.exists(path(type_id))


## The picture of `type_id`, or null if there is none.
static func texture(type_id: String) -> Texture2D:
	if not _cache.has(type_id):
		_cache[type_id] = load(path(type_id)) as Texture2D if exists(type_id) else null
	return _cache[type_id]


## Backdrop color for `type_id` (base buildings have no faction prefix).
static func backdrop(type_id: String) -> Color:
	return FACTION_COLORS.get(type_id.get_slice("_", 0), NEUTRAL_COLOR)
