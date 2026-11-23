extends RefCounted
## Common look of the match HUD (prompt 095): dark, slightly translucent
## panels with rounded corners and hardly a border, tiles as rounded plates
## without a frame (on a panel only a faint ground: "ghost" tiles), gold
## for costs, accents and lasting states, red for shortage, blue for
## cooldowns. Built in code like
## `menu_theme.gd`, whose text colors it keeps.
##
## One factor, `scale`, sizes every HUD part: the font size setting times
## how much room the interface has (`BASE` is the size the layout is drawn
## for). `hud.gd` sets it every frame and tells the parts when it changed
## (`apply_scale`). The factor is capped by the room, so the fixed corners
## never grow into each other on a small window.

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")

const PANEL := Color(0.06, 0.065, 0.055, 0.8)
const PANEL_BORDER := Color(0.66, 0.64, 0.5, 0.16)
const TILE := Color(0.03, 0.035, 0.03, 0.86)
const TILE_BORDER := Color(0.6, 0.58, 0.46, 0.5)
const TILE_HOVER := Color(0.15, 0.16, 0.12, 0.92)
const TILE_ON := Color(0.3, 0.25, 0.11, 0.95)
## Ghost tiles on a panel: a faint ground, brighter under the pointer,
## gold while on.
const GHOST := Color(1, 1, 1, 0.045)
const GHOST_HOVER := Color(1, 1, 1, 0.13)
const GHOST_ON := Color(0.86, 0.72, 0.37, 0.34)
const GOLD := Color("dcb95e")
const BLUE := Color("86b8ea")
const RED := Color("e45a42")
const TEXT := MenuTheme.TEXT
const TEXT_DIM := MenuTheme.TEXT_DIM

## Interface size the layout is drawn for, and sizes at factor 1.
const BASE := Vector2(1280, 720)
const TILE_PX := 46.0
const SMALL_TILE_PX := 34.0
const GAP := 4.0
const PAD := 6.0
const MARGIN := 8.0
## Corner radius of panels and tiles at factor 1.
const RADIUS := 8.0
const TILE_RADIUS := 7.0
## Seconds a part takes to fade in or out.
const FADE := 0.12
## Strip above the minimap (times `scale`) for the camera control of prompt
## 096 (`camera_gizmo.gd`): its dial, and `GIZMO_GAP` under it, which clears
## the north mark on the minimap's frame. The messages stay above the strip.
const GIZMO_ROOM := 88.0
const GIZMO_GAP := 12.0

static var scale := 1.0
static var _serif: Font
static var _plates: Dictionary[String, StyleBoxFlat] = {}


## The HUD factor for an interface of `screen` pixels.
static func scale_for(screen: Vector2) -> float:
	var room := minf(screen.x / BASE.x, screen.y / BASE.y)
	return clampf(MenuTheme.font_scale * clampf(room, 0.6, 1.0), 0.6, maxf(room * 1.15, 0.6))


static func px(value: float) -> float:
	return roundf(value * scale)


## Side of an order tile.
static func tile() -> float:
	return px(TILE_PX)


static func font(size: float) -> int:
	return maxi(roundi(size * scale), 8)


static func box(fill: Color, border: Color, margin: float, radius := TILE_RADIUS) -> StyleBoxFlat:
	var flat := StyleBoxFlat.new()
	flat.bg_color = fill
	flat.border_color = border
	flat.set_border_width_all(1 if border.a > 0.0 else 0)
	flat.set_corner_radius_all(int(px(radius)))
	flat.corner_detail = 6
	flat.anti_aliasing = true
	flat.set_content_margin_all(margin)
	return flat


## A rounded plate without a border (tiles), cached per color and size.
static func plate(fill: Color) -> StyleBoxFlat:
	var key := "%s/%d" % [fill.to_html(), int(px(TILE_RADIUS))]
	if not _plates.has(key):
		_plates[key] = box(fill, Color(0, 0, 0, 0), 0.0)
	return _plates[key]


static func panel_box() -> StyleBoxFlat:
	return box(PANEL, PANEL_BORDER, px(PAD), RADIUS)


## Gives a text button of a HUD panel the common look.
static func style_button(button: Button, size := 12.0) -> void:
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override(&"font_size", font(size))
	var none := Color(0, 0, 0, 0)
	button.add_theme_stylebox_override(&"normal", box(TILE, none, px(4)))
	button.add_theme_stylebox_override(&"hover", box(TILE_HOVER, none, px(4)))
	button.add_theme_stylebox_override(&"pressed", box(TILE_ON, none, px(4)))
	button.add_theme_stylebox_override(&"disabled", box(Color(TILE, 0.6), none, px(4)))


## A serif face for names, from the fonts of the system (no font file in
## the project); Godot's own font where there is none.
static func serif() -> Font:
	if _serif == null:
		var face := SystemFont.new()
		face.font_names = PackedStringArray(["Georgia", "Palatino", "Times New Roman", "DejaVu Serif", "Liberation Serif", "serif"])  # l10n-ignore
		face.allow_system_fallback = true
		_serif = face
	return _serif


## Whether `part` shows or is fading in (a part fading out counts as gone).
static func shown(part: Control) -> bool:
	return part.visible and bool(part.get_meta(&"hud_shown", true))


## Fades `part` in or out; it takes no room of others either way.
static func fade(part: Control, show: bool) -> void:
	if show == bool(part.get_meta(&"hud_shown", part.visible)):
		return
	part.set_meta(&"hud_shown", show)
	if part.has_meta(&"hud_tween"):
		var old: Tween = part.get_meta(&"hud_tween")
		if old != null and old.is_valid():
			old.kill()
	if not part.is_inside_tree():
		part.visible = show
		part.modulate.a = 1.0
		return
	if show and not part.visible:
		part.modulate.a = 0.0
		part.visible = true
	var tween := part.create_tween()
	tween.tween_property(part, "modulate:a", 1.0 if show else 0.0, FADE)
	if not show:
		tween.tween_callback(func() -> void: part.visible = false)
	part.set_meta(&"hud_tween", tween)


## The faction of catalog id `type_id` ("" for base buildings and neutral
## things).
static func faction_of(type_id: String) -> String:
	var prefix := type_id.get_slice("_", 0)
	return prefix if prefix in ["de", "us", "su", "uk"] else ""


## Draws the national marking of `faction` around `center`, as faint as
## `alpha`: the straight-armed bar cross of German vehicles, the star in a
## disc of US vehicles, the red star, the RAF roundel (uk, prompt 117). Own drawings (docs/SOURCES.md).
static func draw_emblem(canvas: CanvasItem, faction: String, center: Vector2, radius: float, alpha: float) -> void:
	var light := Color(1, 1, 1, alpha)
	match faction:
		"de":
			var arm := radius * 0.3
			for across: bool in [false, true]:
				var half := Vector2(arm, radius) if across else Vector2(radius, arm)
				canvas.draw_rect(Rect2(center - half, half * 2.0), light)
			for across: bool in [false, true]:
				var half := Vector2(arm * 0.55, radius * 0.94) if across else Vector2(radius * 0.94, arm * 0.55)
				canvas.draw_rect(Rect2(center - half, half * 2.0), Color(0, 0, 0, alpha * 2.5))
		"us":
			canvas.draw_arc(center, radius, 0.0, TAU, 48, light, maxf(radius * 0.06, 1.0), true)
			canvas.draw_colored_polygon(_star(center, radius * 0.86), light)
		"su":
			canvas.draw_colored_polygon(_star(center, radius), Color(1.0, 0.25, 0.2, alpha * 1.4))
		"uk":
			canvas.draw_circle(center, radius, Color(0.3, 0.45, 0.9, alpha * 1.4))
			canvas.draw_circle(center, radius * 0.66, light)
			canvas.draw_circle(center, radius * 0.33, Color(1.0, 0.25, 0.2, alpha * 1.4))


static func _star(center: Vector2, radius: float) -> PackedVector2Array:
	var points := PackedVector2Array()
	for k in 10:
		var r := radius if k % 2 == 0 else radius * 0.4
		points.append(center + Vector2.UP.rotated(TAU * k / 10.0) * r)
	return points
