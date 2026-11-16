extends RefCounted
## Colors of own, allied and enemy units and markers (prompt 043), with
## palettes for color vision deficiencies. Static; the palette is the setting
## `color_palette` (`settings.gd`). Presentation only.
##
## [provisional] Palettes, from the Okabe-Ito set (distinct for the common
## deficiencies, also in grey):
##
## | Palette | Own | Ally | Enemy |
## |---|---|---|---|
## | `standard` | green | blue | red |
## | `red_green` (deuteranopia, protanopia) | sky blue | yellow | orange |
## | `blue_yellow` (tritanopia) | bluish green | reddish purple | vermilion |
##
## Minimap, health bars, base markers, mines, strike areas and the tint over
## other players' models use these; a palette change shows on the next match
## for tints set at its start.

const PALETTES: Array[String] = ["standard", "red_green", "blue_yellow"]
const COLORS := {
	"standard": [Color(0.35, 0.95, 0.4), Color(0.35, 0.6, 1.0), Color(1.0, 0.3, 0.22)],
	"red_green": [Color(0.34, 0.71, 0.91), Color(0.94, 0.89, 0.26), Color(0.9, 0.62, 0.0)],
	"blue_yellow": [Color(0.0, 0.75, 0.55), Color(0.8, 0.47, 0.65), Color(0.84, 0.37, 0.0)],
}

## Palette in use; set by `settings.gd` when settings apply.
static var palette := "standard"


static func own(alpha := 1.0) -> Color:
	return Color(COLORS[palette][0], alpha)


static func ally(alpha := 1.0) -> Color:
	return Color(COLORS[palette][1], alpha)


static func enemy(alpha := 1.0) -> Color:
	return Color(COLORS[palette][2], alpha)


## Color of `player` seen by `me` (`relation`: 1 = ally, as `SimHost.relation`).
static func of(player: int, me: int, relation: int, alpha := 1.0) -> Color:
	if player == me:
		return own(alpha)
	return ally(alpha) if relation == 1 else enemy(alpha)


## `standard` in the standard palette (each view keeps its tuned shade),
## otherwise the palette's own color with the alpha of `standard`.
static func own_or(standard: Color) -> Color:
	return standard if palette == "standard" else own(standard.a)


static func ally_or(standard: Color) -> Color:
	return standard if palette == "standard" else ally(standard.a)


static func enemy_or(standard: Color) -> Color:
	return standard if palette == "standard" else enemy(standard.a)
