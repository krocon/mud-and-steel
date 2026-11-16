extends Node
## Destroyed map objects (prompt 020). Presentation only.
##
## Reads the object stage events of the simulation every frame
## (`SimHost.take_object_events`): `map_objects.gd` swaps the drawn variant
## (damaged house, ruin, falling tree with stump, gap, flattened fence) and
## `effects.gd` plays debris and dust. On setup, objects that already differ
## from their map variant (`SimHost.changed_objects`) are drawn in their
## stage without animation. Created by `main.gd`.

var main: Node3D
var sim_host: SimHost
## Events handled since the start, per stage (self-test, tools).
var damaged := 0
var destroyed := 0
## The last events (self-test).
var recent: Array[Dictionary] = []


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	for state: Dictionary in sim_host.changed_objects():
		main.map_objects.set_object_state(state, false)


func _process(_delta: float) -> void:
	if sim_host == null:
		return
	for event: Dictionary in sim_host.take_object_events():
		main.map_objects.set_object_state(event, true)
		# Burned down (prompt 041): sooty already, black once burned out.
		if event.cause == "fire":
			main.map_objects.set_charred(event.object, 0.6)
		if main.effects != null:
			main.effects.object_event(event)
		if main.battle_sound != null:
			main.battle_sound.object_event(event)
		match event.stage:
			"damaged":
				damaged += 1
			"destroyed":
				destroyed += 1
		recent.append(event)
		if recent.size() > 32:
			recent.pop_front()
