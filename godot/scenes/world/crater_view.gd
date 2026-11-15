extends Node
## Craters (prompt 021). Presentation only.
##
## The simulation deforms the map; this node reads its crater events every
## frame (`SimHost.take_crater_events`) and brings the drawing up to date:
##
## - Terrain: the changed sample rectangles are collected (overlapping ones
##   merged) and uploaded together with one `terrain.regions_changed`.
##   Godot uploads whole textures, so uploads are spaced: at least `MIN_INTERVAL` apart, and
##   eight times the cost of the last upload (large maps), at most
##   `MAX_INTERVAL`. Units already stand on the new ground meanwhile.
## - Decoration: `map_objects.craters_changed` rebuilds the decoration
##   squares over the craters; `scatter_deco` leaves deformed samples empty.
##
## The impact itself (earth fountain, dust, scorch decal) comes from the
## ground hit of the combat events (`effects.gd`, prompt 019).
## Created by `main.gd`.

const MIN_INTERVAL := 0.1
const MAX_INTERVAL := 0.5

var main: Node3D
var sim_host: SimHost
## Craters seen since the start (self-test, tools).
var craters := 0
## Terrain uploads for craters, and the cost of the last one (ms).
var uploads := 0
var last_upload_ms := 0.0
## The last events (self-test).
var recent: Array[Dictionary] = []

## Rectangles of samples not uploaded yet.
var _pending: Array[Rect2i] = []
var _wait := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host


func _process(delta: float) -> void:
	if sim_host == null:
		return
	for event: Dictionary in sim_host.take_crater_events():
		var rect: Rect2i = event.rect
		_add_pending(rect)
		if main.map_objects != null:
			main.map_objects.craters_changed(rect)
		if main.battle_sound != null:
			main.battle_sound.crater_event(event)
		craters += 1
		recent.append(event)
		if recent.size() > 32:
			recent.pop_front()
	_wait -= delta
	if not _pending.is_empty() and _wait <= 0.0:
		flush()


func _add_pending(rect: Rect2i) -> void:
	# Rectangles that overlap after growing by the terrain's filter margin
	# are read in one go.
	for k in _pending.size():
		if _pending[k].grow(8).intersects(rect):
			_pending[k] = _pending[k].merge(rect)
			return
	_pending.append(rect)


## Uploads the pending rectangles now (self-test, tools).
func flush() -> void:
	if _pending.is_empty():
		return
	var started := Time.get_ticks_usec()
	main.terrain.regions_changed(_pending)
	last_upload_ms = (Time.get_ticks_usec() - started) / 1000.0
	uploads += 1
	_pending = []
	_wait = clampf(last_upload_ms * 0.008, MIN_INTERVAL, MAX_INTERVAL)
