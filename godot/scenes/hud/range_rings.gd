extends Control
## Range and sight circles of the selection (prompt 099), drawn on the HUD
## like the supply rings, following the ground, above the fog layer.
##
## Per selected unit:
## - fire range: the longest-reaching direct weapon (`unit_info`
##   `weapons[].range_m`; for tanks the main gun), a thin solid line;
## - indirect fire (mortars, howitzers, rocket launchers): minimum and
##   maximum range (`unit_indirect`) instead, the minimum dotted;
## - flak (`gun_aa_*`): the anti-aircraft range (`unit_aa_range`), its own
##   color;
## - sight: the sight range on level ground (`unit_sight_range`, weather
##   applied, without the downhill bonus), dotted pale blue.
## A tractor shows the circles of the gun it tows, and a vehicle those of
## the squads riding in it, around its own position, dashed and pale: they
## cannot fire like this.
##
## A circle is only a circle, not a field of view: woods, houses and slopes
## that block the sight or the shot are not cut out.
##
## Settings `range_circles` and `sight_circles` (`settings.gd`): `always`
## with a selection, `key` only while `show_ranges` is held, `never`. Up to
## `MAX_UNITS` selected units each get their circles, beyond that only the
## active unit (prompt 094). Equal circles of units close together (within
## `MERGE` of the radius) are drawn once, around their mean center.

const Settings := preload("res://scenes/menu/settings.gd")

## Selected units that get circles; more: only the active one.
const MAX_UNITS := 8
## Equal circles whose centers are closer than this share of the radius
## are drawn once, around their mean center.
const MERGE := 0.3
const SEGMENTS := 72
## Meters above the ground.
const LIFT := 0.5
const WIDTH := 1.6
## Colors readable on woods, desert, snow, town and at night (light lines
## with a dark shadow line under them).
const FIRE_COLOR := Color(1.0, 0.86, 0.35, 0.85)
const MIN_COLOR := Color(1.0, 0.55, 0.3, 0.85)
const AA_COLOR := Color(0.55, 0.85, 1.0, 0.85)
const SIGHT_COLOR := Color(0.8, 0.92, 1.0, 0.8)
const SHADOW := Color(0.0, 0.0, 0.0, 0.45)
## Circles of towed guns and riding squads: paler.
const TOWED_ALPHA := 0.55
## Line styles: dash length and gap in segments (0: solid).
const STYLE_SOLID := 0
const STYLE_DOTTED := 1
const STYLE_TOWED := 2

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_draw` in microseconds (hud.gd).
var usec := 0
## Circles drawn in the last frame, and their radii (self-test).
var rings := 0
var radii: Array[float] = []
## Per unit id: [fire range, min indirect, max indirect, aa range, sight].
var _ranges := {}
## Per vehicle id: the towed gun and the riders (ids), rebuilt once per tick.
var _linked := {}
var _linked_tick := -1


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "RangeRings"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _process(_delta: float) -> void:
	queue_redraw()


## Whether a circle kind (`range_circles`, `sight_circles`) shows now.
static func wanted(setting: String) -> bool:
	match Settings.get_value(setting):
		"always":
			return true
		"key":
			return InputMap.has_action("show_ranges") and Input.is_action_pressed("show_ranges")
	return false


func _draw() -> void:
	var began := Time.get_ticks_usec()
	rings = 0
	radii.clear()
	var fire := wanted("range_circles")
	var sight := wanted("sight_circles")
	var selected: Array[int] = main.selection.selected
	if selected.is_empty() or not (fire or sight):
		usec += Time.get_ticks_usec() - began
		return
	var units: Array[int] = selected
	if selected.size() > MAX_UNITS:
		var active: int = main.selection.active_unit
		units = [active if selected.has(active) else selected[0]]
	var camera: Camera3D = main.camera
	_refresh_links()
	var done: Array = []
	for id: int in units:
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		var at := node.global_position
		_unit(camera, id, at, STYLE_SOLID, 1.0, fire, sight, done)
		# Riders and the towed gun: their circles around this vehicle.
		for linked: int in _linked.get(id, []):
			_unit(camera, linked, at, STYLE_TOWED, TOWED_ALPHA, fire, false, done)
	for c: Array in done:
		var center: Vector3 = c[0] / float(c[5])
		if draw_ring(self, camera, sim_host, center, c[1], c[2], c[3], c[4]):
			rings += 1
			radii.append(c[1])
	usec += Time.get_ticks_usec() - began


## Who rides in or is towed by whom, from the bridge once per tick.
func _refresh_links() -> void:
	var tick := sim_host.current_tick()
	if tick == _linked_tick:
		return
	_linked_tick = tick
	_linked.clear()
	var ids := sim_host.unit_ids()
	var carriers := sim_host.unit_carriers()
	for k in mini(ids.size(), carriers.size()):
		if carriers[k] >= 0:
			if not _linked.has(carriers[k]):
				_linked[carriers[k]] = []
			_linked[carriers[k]].append(ids[k])


func _unit(camera: Camera3D, id: int, at: Vector3, style: int, alpha: float,
		fire: bool, sight: bool, done: Array) -> void:
	var r := ranges_of(sim_host, id, _ranges)
	if fire:
		if r[2] > 0.0:
			_once(camera, at, r[2], FIRE_COLOR, style, alpha, done)
			if r[1] > 0.0:
				_once(camera, at, r[1], MIN_COLOR, STYLE_DOTTED if style == STYLE_SOLID else style, alpha, done)
		elif r[0] > 0.0:
			_once(camera, at, r[0], FIRE_COLOR, style, alpha, done)
		if r[3] > 0.0:
			_once(camera, at, r[3], AA_COLOR, style, alpha, done)
	if sight and r[4] > 0.0:
		_once(camera, at, r[4], SIGHT_COLOR, STYLE_DOTTED, alpha, done)


## Collects a circle; one of the same kind whose center lies within
## `MERGE` of the radius of another takes part in that one, drawn around
## their mean center (a tight group shows one circle, not eight).
func _once(_camera: Camera3D, at: Vector3, radius: float, color: Color, style: int,
		alpha: float, done: Array) -> void:
	for c: Array in done:
		if is_equal_approx(c[1], radius) and c[2] == color and c[3] == style and c[4] == alpha \
				and (c[0] / float(c[5])).distance_to(at) < radius * MERGE:
			c[0] += at
			c[5] += 1
			return
	done.append([at, radius, color, style, alpha, 1])


## Ranges of unit `id` in meters: [fire, indirect min, indirect max, flak,
## sight], cached in `cache` (catalog values do not change in a match).
static func ranges_of(host: SimHost, id: int, cache: Dictionary) -> Array:
	if cache.has(id):
		return cache[id]
	var info := host.unit_info(id)
	var fire := 0.0
	for w: Dictionary in info.get("weapons", []):
		fire = maxf(fire, float(w.get("range_m", 0.0)))
	var indirect := host.unit_indirect(id)
	var aa := 0.0
	if str(info.get("category", "")).begins_with("gun_aa"):
		aa = host.unit_aa_range(id)
	var r := [fire, indirect.x, indirect.y, aa, host.unit_sight_range(id)]
	if not info.is_empty():
		cache[id] = r
	return r


## The fire circles of unit `id` around `at` (the target preview of a gun
## team uses it); returns how many it drew.
static func draw_unit_rings(canvas: CanvasItem, camera: Camera3D, host: SimHost, id: int,
		at: Vector3, style: int, alpha: float) -> int:
	var r := ranges_of(host, id, {})
	var n := 0
	var fire: float = r[2] if r[2] > 0.0 else r[0]
	if fire > 0.0 and draw_ring(canvas, camera, host, at, fire, FIRE_COLOR, style, alpha * TOWED_ALPHA):
		n += 1
	if r[3] > 0.0 and draw_ring(canvas, camera, host, at, r[3], AA_COLOR, style, alpha * TOWED_ALPHA):
		n += 1
	return n


## One circle on the ground around `at`, every point at the terrain height;
## `style` solid, dotted or dashed (towed). Parts behind the camera are left
## out; false if nothing of it is in front of the camera.
static func draw_ring(canvas: CanvasItem, camera: Camera3D, host: SimHost, at: Vector3,
		radius: float, color: Color, style: int, alpha := 1.0) -> bool:
	var segments := SEGMENTS if radius > 60.0 else SEGMENTS / 2
	var points := PackedVector2Array()
	var front: Array[bool] = []
	for k in segments + 1:
		var angle := TAU * k / segments
		var p := at + Vector3(cos(angle), 0.0, sin(angle)) * radius
		p.y = host.height_at(p.x, p.z) + LIFT
		var seen := not camera.is_position_behind(p)
		front.append(seen)
		points.append(camera.unproject_position(p) if seen else Vector2.ZERO)
	var line := Color(color, color.a * alpha)
	var shade := Color(SHADOW, SHADOW.a * alpha)
	# Dotted: every other segment; towed: two of three; solid: all.
	var period := 1 if style == STYLE_SOLID else (2 if style == STYLE_DOTTED else 3)
	var pairs := PackedVector2Array()
	for k in segments:
		if not (front[k] and front[k + 1]):
			continue
		if period > 1 and k % period == period - 1:
			continue
		pairs.append(points[k])
		pairs.append(points[k + 1])
	if pairs.is_empty():
		return false
	canvas.draw_multiline(pairs, shade, WIDTH + 2.0, true)
	canvas.draw_multiline(pairs, line, WIDTH, true)
	return true
