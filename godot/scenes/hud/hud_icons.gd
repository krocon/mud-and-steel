extends RefCounted
## Symbols of the match HUD (prompt 095), all drawn here: every symbol is
## a few strokes on a 24 x 24 grid, one stroke width, light on dark. No
## picture files and nothing taken from other games or icon sets
## (docs/ARCHITECTURE.md, "HUD (presentation only)").
##
## A symbol is a text of primitives separated by `|`:
## `l x,y x,y ...` open line, `g ...` closed outline, `f ...` filled shape,
## `c x,y r` circle, `d x,y r` filled disc, `a x,y r from to` arc (degrees,
## clockwise from east), `t text` a letter in the middle.

const GRID := 24.0
const MAN := "c 12,8 4 | a 12,22 8 180 360"
const PLANE := "l 12,3 12,20 | l 3,13 12,9 21,13 | l 8,20 12,18 16,20"
const GUN := "c 9,16 4 | l 9,14 21,8 | l 9,17 2,21"
const HOUSE := "g 4,21 4,11 12,4 20,11 20,21 | l 10,21 10,15 14,15 14,21"
const ICONS: Dictionary[String, String] = {
	# Orders, first page.
	"move": "l 4,12 19,12 | l 14,7 19,12 14,17",
	"attack_move": "l 3,12 13,12 | l 9,7 14,12 9,17 | l 15,7 20,12 15,17",
	"patrol": "l 5,9 19,9 | l 15,5 19,9 15,13 | l 19,16 5,16 | l 9,12 5,16 9,20",
	"secure": "g 12,3 20,6 20,12 12,21 4,12 4,6 | d 12,11 2",
	"stop": "g 8,3 16,3 21,8 21,16 16,21 8,21 3,16 3,8 | f 9,9 15,9 15,15 9,15",
	"attack": "c 12,12 7 | l 12,2 12,8 | l 12,16 12,22 | l 2,12 8,12 | l 16,12 22,12",
	"attack_ground": "c 12,10 5 | l 12,3 12,7 | l 12,13 12,17 | l 5,10 9,10 | l 15,10 19,10 | l 3,21 21,21",
	"attack_object": "g 3,21 3,12 9,7 15,12 15,21 | l 7,21 7,16 11,16 11,21 | l 16,7 21,3 | l 18,10 22,10 | l 14,2 15,5",
	"barrage": "l 6,3 6,15 | l 3,12 6,16 9,12 | l 12,5 12,17 | l 9,14 12,18 15,14 | l 18,3 18,15 | l 15,12 18,16 21,12 | l 3,21 21,21",
	"smoke": "c 8,13 4 | c 14,10 5 | c 17,15 4 | l 5,20 20,20",
	"fire": "c 12,12 7 | d 12,12 2 | l 4,20 20,4",
	# Fire discipline (prompt 176): crosshair open, crossed, with an arrow
	# back to the shooter; the limits: a short range ring, a hull.
	"fire_at_will": "c 12,12 7 | d 12,12 2 | l 12,2 12,6 | l 12,18 12,22 | l 2,12 6,12 | l 18,12 22,12",
	"hold_fire": "c 12,12 7 | d 12,12 2 | l 4,20 20,4",
	"return_fire": "c 13,12 6 | d 13,12 2 | l 3,20 9,14 | l 3,15 3,20 8,20",
	"close_only": "c 12,12 9 | c 12,12 4 | d 12,12 1.5",
	"vehicles_only": "g 3,15 21,15 19,19 5,19 | g 7,15 8,11 15,11 16,15 | l 15,12 21,10",
	"stance": "d 12,5 2.5 | l 12,8 12,16 | l 12,16 8,22 | l 12,16 16,22 | l 7,11 17,11",
	"garrison": "g 5,21 5,11 12,5 19,11 19,21 | l 12,10 12,18 | l 9,15 12,18 15,15",
	"back": "l 20,12 5,12 | l 10,7 5,12 10,17",
	# Stances and formations.
	"stance_stand": "d 12,5 2.5 | l 12,8 12,16 | l 12,16 8,22 | l 12,16 16,22 | l 7,11 17,11",
	"stance_kneel": "d 11,6 2.5 | l 11,9 11,15 | l 11,15 17,15 17,21 | l 11,15 7,21 | l 11,11 17,10",
	"stance_prone": "d 5,15 2.5 | l 8,16 20,17 | l 20,17 22,20 | l 9,16 12,20 | l 2,21 22,21",
	"stance_auto": "l 6,20 12,4 18,20 | l 8.5,14 15.5,14",
	"formation_0": "l 7,19 12,5 17,19 | l 9,14 15,14",
	"formation_1": "d 5,12 2.5 | d 12,12 2.5 | d 19,12 2.5",
	"formation_2": "d 12,6 2.5 | d 6,17 2.5 | d 18,17 2.5",
	"formation_3": "d 12,5 2.5 | d 12,12 2.5 | d 12,19 2.5",
	"formation_4": "d 12,7 2 | d 7,12 2 | d 17,12 2 | d 3,17 2 | d 21,17 2",
	"formation_5": "d 6,6 2 | d 17,8 2 | d 10,14 2 | d 19,18 2 | d 5,19 2",
	# Context column: transport, crossing, engineers (a spade).
	"disembark": "g 3,5 15,5 15,13 3,13 | c 6,16 1.5 | c 12,16 1.5 | l 19,7 19,19 | l 16,16 19,19 22,16",
	"unlimber": GUN,
	"camouflage": "l 3,19 8,8 12,15 16,6 21,19 | l 3,19 21,19",
	"bridge_lay": "l 2,13 22,13 | l 6,13 6,20 | l 18,13 18,20 | l 12,3 12,9 | l 9,6 12,9 15,6",
	# Boats (prompt 119): a hull running onto the shore, and backing off.
	"boat_land": "l 2,12 14,12 17,16 4,16 2,12 | l 17,16 22,20 | l 14,12 16,7",
	"boat_retract": "l 8,12 20,12 22,16 10,16 8,12 | l 8,8 2,8 | l 5,5 2,8 5,11",
	"bridge_pickup": "l 2,13 22,13 | l 6,13 6,20 | l 18,13 18,20 | l 12,9 12,3 | l 9,6 12,3 15,6",
	"works": "l 4,20 13,11 | g 12,8 17,3 21,7 16,12 | l 3,17 7,21",
	"positions": "l 2,10 7,10 9,17 15,17 17,10 22,10 | l 12,3 12,9 | l 9,6 15,6",
	"work_mines_at": "c 12,13 7 | d 12,13 2 | l 12,3 12,6",
	"work_mines_ap": "c 12,15 4 | l 12,6 12,11 | l 8,7 10,11 | l 16,7 14,11",
	"work_wire": "c 7,12 4 | c 12,12 4 | c 17,12 4 | l 2,19 22,19",
	"work_hedgehogs": "l 4,20 20,4 | l 4,4 20,20 | l 12,2 12,22",
	"work_teeth": "g 2,20 6,10 10,20 | g 9,20 12,10 15,20 | g 14,20 18,10 22,20",
	"work_clear": "c 12,12 6 | l 4,4 20,20 | l 20,4 4,20",
	"work_bridge": "l 2,9 22,9 | a 7.5,9 5 0 180 | a 16.5,9 5 0 180",
	"work_foxhole": "l 2,10 7,10 9,17 15,17 17,10 22,10 | d 12,12 2",
	"work_trench": "l 2,7 9,7 9,13 15,13 15,7 22,7 | l 2,18 22,18",
	"work_comm_trench": "l 3,21 9,13 15,13 21,3 | l 8,21 12,17 18,17 23,8",
	"work_mg_nest": "a 12,18 8 180 360 | l 12,14 20,6 | l 3,18 21,18",
	"work_dugout": "g 3,20 3,12 12,6 21,12 21,20 | f 9,20 9,14 15,14 15,20",
	"work_vehicle_pit": "l 2,9 6,9 8,18 16,18 18,9 22,9 | g 9,11 15,11 15,15 9,15",
	"work_camo_net": "l 3,8 21,8 | l 3,14 21,14 | l 8,3 8,19 | l 15,3 15,19 | l 3,20 21,20",
	"work_bunker": "g 3,20 3,10 7,6 17,6 21,10 21,20 | f 8,12 16,12 16,14 8,14",
	# Base building.
	"build": "l 5,20 14,11 | f 11,6 15,3 21,9 17,12",
	"repair": "l 4,21 13,12 | l 6,23 15,14 | a 17,8 5 30 260 | l 4,21 6,23",
	"sell": "g 3,12 12,3 21,3 21,12 12,21 | c 16.5,7.5 1.5",
	"cancel_last": "l 6,6 18,18 | l 18,6 6,18",
	# Camera buttons of the info field: center (four arrows to a dot),
	# follow (a dot in four corners).
	"center": "d 12,12 2 | l 12,2 12,8 | l 10,6 12,8 14,6 | l 12,22 12,16 | l 10,18 12,16 14,18 | l 2,12 8,12 | l 6,10 8,12 6,14 | l 22,12 16,12 | l 18,10 16,12 18,14",
	"follow": "d 12,12 2.5 | l 3,8 3,3 8,3 | l 16,3 21,3 21,8 | l 21,16 21,21 16,21 | l 8,21 3,21 3,16",
	# Camera views beside the minimap: overview (four corners pointing
	# out), close and flat (an eye low over the ground), the jump to the
	# fighting (crossed swords).
	"overview": "l 3,8 3,3 8,3 | l 3,3 9,9 | l 16,3 21,3 21,8 | l 21,3 15,9 | l 21,16 21,21 16,21 | l 21,21 15,15 | l 8,21 3,21 3,16 | l 3,21 9,15",
	"close_view": "l 2,19 22,19 | a 12,13 7 200 340 | a 12,9 7 20 160 | d 12,11 2",
	"action_jump": "l 4,4 17,17 | l 14,20 20,14 | l 20,4 7,17 | l 4,14 10,20",
	# Minimap buttons, help, menu.
	"north": "f 12,2 16,13 12,11 8,13 | g 12,22 16,13 12,11 8,13",
	"zoom": "c 10,10 6 | l 15,15 21,21",
	"home": HOUSE,
	"pause": "f 7,5 10,5 10,19 7,19 | f 14,5 17,5 17,19 14,19",
	"help": "t ?",
	"menu": "l 5,7 19,7 | l 5,12 19,12 | l 5,17 19,17",
	# Resources and unit values.
	"fuel": "a 12,14 6 -30 210 | l 6.8,11 12,3 17.2,11",
	"ammo": "g 9,21 9,9 12,3 15,9 15,21 | l 9,17 15,17",
	"manpower": MAN,
	"health": "g 12,20 4,12 4,7 7,4 10,4 12,7 14,4 17,4 20,7 20,12",
	"soldiers": MAN,
	# Support.
	"air_recon": "a 12,19 11.4 218 322 | a 12,5 11.4 38 142 | c 12,12 2.5",
	"air_recon_loiter": "a 12,12 8 60 360 | l 17.5,9.5 20,12 22.5,9.5 | c 12,12 2.5",
	"air_strafe": PLANE,
	"air_attack": PLANE + " | l 5,15 5,21 | l 19,15 19,21",
	"air_bomb": "g 9,8 15,8 16,14 12,21 8,14 | l 9,8 8,3 16,3 15,8 | l 12,8 12,3",
	"air_airborne": "a 12,11 8 180 360 | l 4,11 20,11 | l 4,11 12,21 | l 20,11 12,21 | l 12,11 12,21",
	"strike": "g 12,2 15,8 15,17 9,17 9,8 | l 9,14 5,20 | l 15,14 19,20 | l 12,17 12,22",
	# Kinds of units (the tabs of the picker).
	"kind_infantry": MAN,
	"kind_vehicles": "g 3,14 21,14 19,19 5,19 | g 8,9 15,9 16,14 7,14 | l 15,11 22,10",
	"kind_guns": GUN,
	"kind_air": PLANE,
	"kind_base": HOUSE,
	"kind_defense": "g 3,20 3,10 7,6 17,6 21,10 21,20 | f 8,12 16,12 16,14 8,14",
	"kind_other": "g 6,6 18,6 18,18 6,18",
	# State marks over units (prompt 105, `state_marks.gd`): one clear
	# shape each, readable at 14 px.
	"state_burning": "g 12,2 17,9 19,15 16,20 12,22 8,20 5,15 7,9 10,12 | l 12,21 10,17 12,13 14,17 12,21",
	"state_immobile": "c 12,12 7 | l 12,5 12,19 | l 5,12 19,12 | l 3,21 21,3",
	"state_track": "l 9,6 2,6 2,18 9,18 | l 15,6 22,6 22,18 15,18 | l 5,6 5,18 | l 19,6 19,18 | l 11,4 13,9 10,13 13,20",
	"state_suppressed": "a 12,19 7 180 360 | l 3,19 21,19 | l 12,2 12,9 | l 8,6 12,10 16,6",
	"state_no_ammo": "g 9,21 9,9 12,3 15,9 15,21 | l 9,17 15,17 | l 4,21 20,3",
	"state_no_fuel": "a 12,14 6 -30 210 | l 6.8,11 12,3 17.2,11 | l 4,21 20,3",
	# A unit stopped because it got stuck (prompt 173): a wall and an
	# arrow that stops at it.
	# Fire discipline of own units (prompt 176).
	"state_hold_fire": "c 12,12 7 | l 4,20 20,4",
	"state_return_fire": "c 13,12 6 | l 3,20 9,14 | l 3,15 3,20 8,20",
	"state_stuck": "l 18,3 18,21 | l 21,3 21,21 | l 3,12 13,12 | l 9,8 13,12 9,16",
	"state_service": "l 4,21 13,12 | l 6,23 15,14 | a 17,8 5 30 260 | l 4,21 6,23 | l 18,16 18,22 | l 15,19 21,19",
}

static var _parsed: Dictionary[String, Array] = {}


static func has(id: String) -> bool:
	return ICONS.has(id)


## Draws symbol `id` into the largest square in the middle of `rect`,
## turned by `turn` radians around its middle.
static func draw(canvas: CanvasItem, id: String, rect: Rect2, color: Color, width := 1.6, turn := 0.0) -> void:
	if not ICONS.has(id):
		return
	if not _parsed.has(id):
		_parsed[id] = _parse(ICONS[id])
	var side := minf(rect.size.x, rect.size.y)
	var unit := side / GRID
	var middle := rect.get_center()
	var place := Transform2D(turn, Vector2(unit, unit), 0.0, middle) * Transform2D(0.0, -Vector2(GRID, GRID) * 0.5)
	for part: Array in _parsed[id]:
		var kind: String = part[0]
		var points: PackedVector2Array = place * (part[1] as PackedVector2Array)
		var numbers: PackedFloat32Array = part[2]
		match kind:
			"l":
				canvas.draw_polyline(points, color, width, true)
			"g":
				points.append(points[0])
				canvas.draw_polyline(points, color, width, true)
			"f":
				canvas.draw_colored_polygon(points, color)
			"c":
				canvas.draw_arc(points[0], numbers[0] * unit, 0.0, TAU, 24, color, width, true)
			"d":
				canvas.draw_circle(points[0], numbers[0] * unit, color)
			"a":
				canvas.draw_arc(points[0], numbers[0] * unit, deg_to_rad(numbers[1]) + turn, deg_to_rad(numbers[2]) + turn, 20, color, width, true)
			"t":
				var size := roundi(side * 0.8)
				var font := ThemeDB.fallback_font
				var text: String = part[3]
				var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size)
				canvas.draw_string(font, middle + Vector2(-text_size.x * 0.5, size * 0.36), text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)


static func _parse(text: String) -> Array:
	var parts: Array = []
	for piece: String in text.split("|"):
		var words := piece.strip_edges().split(" ", false)
		var points := PackedVector2Array()
		var numbers := PackedFloat32Array()
		for word: String in words.slice(1):
			if word.contains(","):
				points.append(Vector2(word.get_slice(",", 0).to_float(), word.get_slice(",", 1).to_float()))
			else:
				numbers.append(word.to_float())
		parts.append([words[0], points, numbers, words[1] if words[0] == "t" else ""])
	return parts
