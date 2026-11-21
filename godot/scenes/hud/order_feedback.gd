extends Control
## Feedback at the mouse pointer (prompt 175). Presentation only: the
## simulation decides, this shows.
##
## - **Dropped orders**: `SimHost.take_order_events` lists the local
##   player's orders the simulation dropped, each with a reason
##   (`ORDER_REJECTED_<REASON>`). The reason shows beside the pointer for
##   `HINT_SECONDS`, goes into the event feed (kind "rejected") and a quiet
##   click sounds (at most every `SOUND_GAP` seconds).
## - **Penetration light**: while own armed units are selected and the
##   pointer is over a visible enemy, a small light beside the pointer
##   tells whether the selection's active (or first armed) unit penetrates
##   it at their distance (`SimHost.penetration_view`, the hit formula
##   without the random spread): green from the front, yellow only from
##   the side or rear, red not at all. Three dots show front, side, rear.

const EventFeed := preload("res://scenes/hud/event_feed.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")

const HINT_SECONDS := 2.5
const SOUND_GAP := 0.4
## Seconds between two looks at the unit under the pointer.
const HOVER_REFRESH := 0.15
const OFFSET := Vector2(18, -30)
const COLORS := {
	"front": Color(0.45, 0.9, 0.45),
	"side": Color(0.98, 0.82, 0.3),
	"none": Color(1.0, 0.42, 0.38),
}
const HINT_COLOR := Color(1.0, 0.78, 0.5)

var main: Node3D
var sim_host: SimHost
var feed: EventFeed
## Text beside the pointer and when it goes (self-test reads them).
var hint := ""
var hint_until := 0.0
## Hover light: enemy id (-1: none), faces (front, side, rear) and verdict
## ("front", "side", "none").
var hover_target := -1
var faces := PackedByteArray()
var verdict := ""
## Dropped orders shown so far (self-test).
var shown := 0

var _now := 0.0
var _sound_at := -1e9
var _hover_left := 0.0


func setup(main_node: Node3D, event_feed: EventFeed) -> void:
	main = main_node
	sim_host = main.sim_host
	feed = event_feed
	name = "OrderFeedback"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _process(delta: float) -> void:
	_now += delta
	if sim_host == null:
		return
	for event: Dictionary in sim_host.take_order_events():
		show_rejected(str(event.get("reason", "")), int(event.get("unit", -1)))
	_hover_left -= delta
	if _hover_left <= 0.0:
		_hover_left = HOVER_REFRESH
		_update_hover()
	queue_redraw()


## Shows the reason of a dropped order (also the self-test).
func show_rejected(reason: String, unit: int) -> void:
	if reason.is_empty() or sim_host.is_replay():
		return
	hint = tr("ORDER_REJECTED_" + reason.to_upper())
	hint_until = _now + HINT_SECONDS
	shown += 1
	var at := Vector3.ZERO
	var index := sim_host.unit_ids().find(unit) if unit >= 0 else -1
	if index >= 0:
		at = sim_host.current_positions()[index]
	elif main.camera_rig != null:
		at = main.camera_rig.focus
	feed.post("rejected", hint, at)
	if _now - _sound_at >= SOUND_GAP:
		_sound_at = _now
		UiSound.play("ui_click")


func _update_hover() -> void:
	hover_target = -1
	verdict = ""
	var selection: Control = main.selection
	if selection == null or selection.selected.is_empty() or sim_host.is_replay():
		return
	var shooter := _shooter(selection)
	if shooter < 0:
		return
	var mouse := get_viewport().get_mouse_position()
	var target: int = selection._unit_at(mouse, false)
	if target < 0 or sim_host.relation(_owner_of(target)) == 1:
		return
	faces = sim_host.penetration_view(shooter, target)
	if faces.size() != 3:
		return
	hover_target = target
	verdict = "front" if faces[0] != 0 else ("side" if faces[1] != 0 or faces[2] != 0 else "none")


## The selection's active unit if it is armed and own, else its first
## armed own unit; -1 if none.
func _shooter(selection: Control) -> int:
	var me := sim_host.local_player()
	var active: int = selection.active_unit
	var candidates: Array[int] = []
	if active >= 0:
		candidates.append(active)
	candidates.append_array(selection.selected)
	for id in candidates:
		if _owner_of(id) != me:
			continue
		if not sim_host.unit_fire_status(id).is_empty():
			return id
	return -1


func _owner_of(id: int) -> int:
	var index := sim_host.unit_ids().find(id)
	return sim_host.unit_owners()[index] if index >= 0 else -1


func _draw() -> void:
	var font := get_theme_default_font()
	var size := MenuTheme.font(14)
	var mouse := get_viewport().get_mouse_position()
	var at := mouse + OFFSET
	if _now < hint_until and not hint.is_empty():
		var alpha := clampf((hint_until - _now) / 0.5, 0.0, 1.0)
		var width := font.get_string_size(hint, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		draw_rect(Rect2(at - Vector2(6, size + 2), Vector2(width + 12, size + 10)), Color(0, 0, 0, 0.6 * alpha))
		draw_string(font, at, hint, HORIZONTAL_ALIGNMENT_LEFT, -1, size, Color(HINT_COLOR, alpha))
		at.y += size + 12
	if hover_target < 0 or verdict.is_empty():
		return
	var text := tr("HUD_PENETRATION_" + verdict.to_upper())
	var color: Color = COLORS[verdict]
	var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
	var box := Rect2(at - Vector2(6, size + 2), Vector2(width + 12 + 3 * 14 + 8, size + 10))
	draw_rect(box, Color(0, 0, 0, 0.6))
	var dot := at + Vector2(4, -size * 0.35)
	for k in 3:
		var c: Color = COLORS["front"] if faces[k] != 0 else COLORS["none"]
		draw_circle(dot, 5.0, c)
		dot.x += 14.0
	draw_string(font, at + Vector2(3 * 14 + 8, 0), text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)
