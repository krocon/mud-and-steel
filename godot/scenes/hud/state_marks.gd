extends Control
## State marks over units (prompt 105): small symbols above the health bar
## that tell a unit's state without selecting it: burning (standing in the
## flames of a fire), immobilized (at or below `immobile_pct` hit points),
## track off (mine), suppressed or pinned down (infantry), out of
## ammunition, out of fuel, being repaired or resupplied, stopped because
## it got stuck (prompt 173, `SimHost.stuck_units`: until a new order),
## and for own units their fire discipline when it is not "fire at will"
## (prompt 176: return fire, hold fire; least important).
##
## Sparse on purpose: at most MAX_MARKS marks side by side, the most
## important first (ORDER); from FAR_M camera distance only the most
## important one. The rest stays in the unit panel. Enemies (and neutral
## units) show only what can be seen from outside (ENEMY_MARKS): never out
## of ammunition or fuel, never being serviced. Units hidden by the fog of
## war (their node is hidden, or their point is not visible) get no mark.
## The setting "State marks" (`Settings.STATE_MARK_MODES`) shows them for
## every unit, only for selected units, or never.
##
## Presentation only: everything is read from SimHost (`unit_immobile`,
## `tracks_off`, `unit_suppressed`, `unit_ammo`, `unit_fuel`,
## `unit_health`, `fires`); "being serviced" is derived here from health,
## ammunition or fuel rising (resupply and repair run once per game second).
## The states are rescanned every SCAN_SECONDS; drawing every frame only
## projects the units' positions. Created by `combat_view.gd` beside the
## health bars.

const HudIcons := preload("res://scenes/hud/hud_icons.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const Settings := preload("res://scenes/menu/settings.gd")

## Mark ids by importance.
const ORDER: Array[String] = ["burning", "immobile", "track", "retreat", "suppressed", "stuck", "no_ammo", "no_fuel", "service", "return_fire", "hold_fire"]
## What an enemy shows (visible from outside).
const ENEMY_MARKS: Array[String] = ["burning", "immobile", "track", "retreat", "suppressed"]
const COLORS: Dictionary[String, Color] = {
	"burning": Color(1.0, 0.55, 0.15),
	"immobile": Color(0.95, 0.32, 0.25),
	"track": Color(0.95, 0.32, 0.25),
	"retreat": Color(1.0, 0.72, 0.18),
	"suppressed": Color(0.92, 0.9, 0.84),
	"no_ammo": Color(0.95, 0.75, 0.25),
	"no_fuel": Color(0.45, 0.7, 1.0),
	"service": Color(0.45, 0.9, 0.45),
	"stuck": Color(0.95, 0.8, 0.55),
	"return_fire": Color(0.85, 0.85, 0.95),
	"hold_fire": Color(0.85, 0.85, 0.95),
}
const MAX_MARKS := 2
## Camera distance (m) from which only the most important mark shows.
const FAR_M := 260.0
## Size of one mark (px) and the gap between two.
const SIZE := 15.0
const GAP := 3.0
## Meters above the ground (the health bars' LIFT) and pixels above the bar.
const LIFT := 3.6
const ABOVE_BAR := 7.0
## How long a unit counts as being serviced after its health, ammunition or
## fuel last rose (real seconds; resupply ticks once per game second).
const SERVICE_SECONDS := 2.5
const SCAN_SECONDS := 0.25

var view: Node3D
## Marks drawn in the last frame, and per kind; kinds drawn over enemies
## (self-test).
var drawn := 0
var drawn_kinds: Dictionary[String, int] = {}
var enemy_kinds: Dictionary[String, int] = {}
## Unit id -> its state kinds in ORDER (all of them, before the enemy and
## distance filters), from the last scan.
var states: Dictionary[int, Array] = {}
## Unit id -> cover grade (1 = light, 2 = heavy/garrison; prompt 233).
var cover_grades: Dictionary[int, int] = {}
## Unit id -> true when pinned down (suppressed == 2, prompt 236).
var pinned: Dictionary[int, bool] = {}
## Cover shields and pinned capsules drawn in the last frame (self-test).
var shields_drawn := 0
var pinned_capsules_drawn := 0
var _serviced: Dictionary[int, float] = {}
var _stock: Dictionary[int, Vector3] = {}
var _scan_left := 0.0
var _time := 0.0
var _draw_key := []


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func _process(delta: float) -> void:
	_time += delta
	_scan_left -= delta
	var rescanned := false
	if _scan_left <= 0.0:
		_scan_left = SCAN_SECONDS
		scan()
		rescanned = true
	if view == null or view.main == null or view.sim_host == null:
		return
	var mode: String = Settings.get_value("state_marks")
	var active := mode != "never" and (not states.is_empty() or not cover_grades.is_empty())
	var key := [
		active, mode, states.size(), cover_grades.size(), pinned.size(),
		view.main.camera.global_transform if active else Transform3D.IDENTITY,
		view.sim_host.current_tick() if active else 0,
		view.main.selection.selected.hash() if mode == "selected" else 0,
		RelationColors.palette, HudTheme.scale,
	]
	if rescanned or key != _draw_key:
		_draw_key = key
		queue_redraw()


## The marks unit with `kinds` (in ORDER) shows: all for own and allied
## units, ENEMY_MARKS for others; one when `far`, else MAX_MARKS.
static func marks_of(kinds: Array, own: bool, far: bool) -> Array[String]:
	var out: Array[String] = []
	for kind: String in kinds:
		if own or kind in ENEMY_MARKS:
			out.append(kind)
			if out.size() >= (1 if far else MAX_MARKS):
				break
	return out


## Reads every unit's state from the simulation.
func scan() -> void:
	var sim_host: SimHost = view.sim_host
	var ids := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var health := sim_host.unit_health()
	var ammo := sim_host.unit_ammo()
	var fuel := sim_host.unit_fuel()
	var immobile := sim_host.unit_immobile()
	var retreating := sim_host.unit_retreating()
	var suppressed := sim_host.unit_suppressed()
	var footprints := sim_host.unit_footprints()
	var positions := sim_host.current_positions()
	var tracks: Dictionary[int, bool] = {}
	for id in sim_host.tracks_off():
		tracks[id] = true
	var modes := sim_host.unit_fire_modes()
	var stuck: Dictionary[int, bool] = {}
	for id in sim_host.stuck_units():
		stuck[id] = true
	var fires := sim_host.fires()
	var cover := sim_host.unit_cover_grades()
	states.clear()
	cover_grades.clear()
	pinned.clear()
	for i in ids.size():
		var id := ids[i]
		if alive[i] == 0:
			_stock.erase(id)
			continue
		if i < cover.size() and cover[i] > 0:
			cover_grades[id] = cover[i]
		var stock := Vector3(health[i], ammo[i], fuel[i])
		var last: Vector3 = _stock.get(id, stock)
		if stock.x > last.x + 0.0001 or stock.y > last.y or stock.z > last.z:
			_serviced[id] = _time + SERVICE_SECONDS
		_stock[id] = stock
		var kinds: Array[String] = []
		if _in_fire(positions[i], footprints[i] * 0.5, fires):
			kinds.append("burning")
		if immobile.size() > i and immobile[i] != 0:
			kinds.append("immobile")
		if tracks.has(id):
			kinds.append("track")
		if retreating.size() > i and retreating[i] != 0:
			kinds.append("retreat")
		if suppressed.size() > i and suppressed[i] != 0:
			kinds.append("suppressed")
			if suppressed[i] >= 2:
				pinned[id] = true
		if stuck.has(id):
			kinds.append("stuck")
		if ammo[i] == 0:
			kinds.append("no_ammo")
		if fuel[i] == 0:
			kinds.append("no_fuel")
		if _serviced.get(id, -1.0) > _time:
			kinds.append("service")
		if i < modes.size() and modes[i] == 2:
			kinds.append("return_fire")
		elif i < modes.size() and modes[i] == 1:
			kinds.append("hold_fire")
		if not kinds.is_empty():
			states[id] = kinds
	for id: int in _serviced.keys():
		if _serviced[id] <= _time:
			_serviced.erase(id)


## Whether a unit of radius `radius` at `at` stands in the flames of a fire
## (`SimHost.fires`: x, ground height, z, flame radius).
static func _in_fire(at: Vector3, radius: float, fires: PackedVector4Array) -> bool:
	for f in fires:
		var reach := f.w + radius
		if Vector2(f.x - at.x, f.z - at.z).length_squared() < reach * reach:
			return true
	return false


func _draw() -> void:
	drawn = 0
	shields_drawn = 0
	pinned_capsules_drawn = 0
	drawn_kinds.clear()
	enemy_kinds.clear()
	var mode: String = Settings.get_value("state_marks")
	if mode == "never" or (states.is_empty() and cover_grades.is_empty()):
		return
	var sim_host: SimHost = view.sim_host
	var camera: Camera3D = view.main.camera
	var selection: Control = view.main.selection
	var effects: Node = view.main.get("effects")
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var me := sim_host.local_player()
	var bar_half := HudTheme.px(32.0) * 0.5
	var shield_size := HudTheme.px(12.0)
	var font := get_theme_default_font()
	var pin_font_sz := int(round(HudTheme.px(10.5)))
	var pin_text := tr("HUD_PINNED_BADGE")
	var pin_text_w := font.get_string_size(pin_text, HORIZONTAL_ALIGNMENT_LEFT, -1, pin_font_sz).x
	for i in ids.size():
		var id := ids[i]
		var has_state := states.has(id)
		var cover_grade: int = cover_grades.get(id, 0)
		if alive[i] == 0 or (not has_state and cover_grade == 0) or (mode == "selected" and not selection.is_selected(id)):
			continue
		var node: Node3D = view.main._unit_nodes.get(id)
		if node == null or not node.is_visible_in_tree():
			continue
		var own: bool = owners[i] == me or sim_host.relation(owners[i]) == 1
		if not own and effects != null and not effects.visible_at.call(node.global_position):
			continue
		var world := node.global_position + Vector3.UP * LIFT
		if camera.is_position_behind(world):
			continue
		var screen := camera.unproject_position(world)
		# Prompt 233: compact cover shield beside the left end of the health bar
		# (independent of the 2-slot warning row above the bar).
		if cover_grade > 0:
			var col := RelationColors.cover_heavy() if cover_grade >= 2 else RelationColors.cover_light()
			var s_rect := Rect2(screen + Vector2(-bar_half - shield_size - 3.0, -shield_size * 0.45), Vector2(shield_size, shield_size))
			HudIcons.draw(self, "cover_shield", s_rect.grow(1.2), Color(0, 0, 0, 0.85), 1.4)
			HudIcons.draw(self, "cover_shield", s_rect, col, 1.2)
			shields_drawn += 1
		if not has_state:
			continue
		var far := camera.global_position.distance_to(node.global_position) > FAR_M
		var marks := marks_of(states[id], own, far)
		if marks.is_empty():
			continue
		var is_pinned := pinned.has(id)
		var width := marks.size() * SIZE + (marks.size() - 1) * GAP
		var at := screen + Vector2(-width * 0.5, -ABOVE_BAR - SIZE)
		for kind in marks:
			var rect := Rect2(at, Vector2(SIZE, SIZE))
			draw_circle(rect.get_center(), SIZE * 0.5 + 1.5, Color(0, 0, 0, 0.62))
			var col: Color = Color(1.0, 0.36, 0.22) if (kind == "suppressed" and is_pinned) else COLORS[kind]
			HudIcons.draw(self, "state_" + kind, rect.grow(-1.5), col, 1.4)
			at.x += SIZE + GAP
			drawn += 1
			drawn_kinds[kind] = drawn_kinds.get(kind, 0) + 1
			if not own:
				enemy_kinds[kind] = enemy_kinds.get(kind, 0) + 1
		# Prompt 236: prominent high-contrast "PINNED" capsule at close/medium
		# range for pinned squads (both own and spotted enemies).
		if is_pinned and not far:
			var pad_x := HudTheme.px(5.0)
			var pad_y := HudTheme.px(2.0)
			var cw := pin_text_w + pad_x * 2.0
			var ch := float(pin_font_sz) + pad_y * 2.0
			var crect := Rect2(Vector2(screen.x - cw * 0.5, screen.y - ABOVE_BAR - SIZE - ch - HudTheme.px(3.0)), Vector2(cw, ch))
			draw_rect(crect, Color(0.14, 0.03, 0.03, 0.88))
			draw_rect(crect, Color(1.0, 0.38, 0.20, 0.92), false, 1.2)
			draw_string(font, crect.position + Vector2(pad_x, pad_y + float(pin_font_sz) * 0.88),
					pin_text, HORIZONTAL_ALIGNMENT_LEFT, -1, pin_font_sz, Color(1.0, 0.84, 0.72))
			pinned_capsules_drawn += 1
