extends RefCounted
## Self-test part for trees pushed over by vehicles (prompt 106), run by
## `self_test.gd` after the track test with its helpers (`_check`,
## `_frames`, `_ids_of`, `_positions_of`).
##
## The rule itself is tested in `crates/sim/tests/tree_crush.rs`; here the
## presentation of a crush event of a tree near the own tanks (made up from
## the tree's state, as `SimHost.take_object_events` delivers it):
## - In sight it falls with the push (faster, then a bounce on the crown:
##   it springs back a few degrees and comes to rest at 86°),
##   throws leaves and torn-up earth and is heard (`tree_fall`).
## - Out of sight (an enemy tank in the fog) neither effects nor sound give
##   it away.
## Afterwards the tree is drawn standing again (the simulation never felled
## it).

const TREES: Array[String] = ["env_tree_deciduous", "env_tree_birch", "env_tree_conifer"]


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sim_host: SimHost = main.sim_host
	var effects: Node = main.effects
	var sound: Node = main.battle_sound
	var objects: Node3D = main.map_objects
	var own: Array[int] = test._ids_of(true)
	var center: Vector3 = test._positions_of(own)[0] if not own.is_empty() else Vector3.ZERO
	var tree := -1
	for kind in TREES:
		for index: int in sim_host.destructible_objects_near(kind, center.x, center.z, 600.0):
			if str(sim_host.object_state(index).get("stage", "")) == "intact":
				tree = index
				break
		if tree >= 0:
			break
	test._check(tree >= 0, "no standing tree near the own tanks for the push test")
	if tree < 0:
		return
	var event: Dictionary = sim_host.object_state(tree).duplicate()
	event.stage = "destroyed"
	event.remains = "fallen"
	event.fall_deg = 90
	event.cause = "crushed"
	event.unit = -1
	event.position = sim_host.object_position(tree)
	var effects_visible: Callable = effects.visible_at
	var sound_visible: Callable = sound.visible_at if sound != null else Callable()

	# In sight.
	effects.visible_at = func(_point: Vector3) -> bool: return true
	if sound != null:
		sound.visible_at = effects.visible_at
	var pushed: int = effects.counts.get("object_tree_pushed", 0)
	var heard: int = sound.requested.get("tree_fall", 0) if sound != null else 0
	objects.set_object_state(event, true)
	effects.object_event(event)
	if sound != null:
		sound.object_event(event)
	test._check(effects.counts.get("object_tree_pushed", 0) == pushed + 1, "a pushed tree in sight threw no leaves")
	if sound != null:
		test._check(sound.requested.get("tree_fall", 0) == heard + 1, "a pushed tree in sight was not heard")
	var node: Node3D = objects.swapped.get(tree)
	test._check(node != null and node.get_child_count() == 2, "the pushed tree is not drawn as stump and falling tree")
	if node != null and node.get_child_count() == 2:
		var pivot: Node3D = node.get_child(1)
		var rest := deg_to_rad(86.0)
		var landed := false
		var rebound := rest
		# Push (1.1 s) and bounce (0.43 s) at the game's speed.
		var began := Time.get_ticks_msec()
		while Time.get_ticks_msec() - began < 2200:
			var a := pivot.basis.get_rotation_quaternion().get_angle()
			test._check(a <= rest + 0.02, "the pushed tree tipped past its rest angle: %.1f°" % rad_to_deg(a))
			if a >= rest - 0.01:
				landed = true
			elif landed:
				rebound = minf(rebound, a)
			await test.get_tree().process_frame
		var angle := pivot.basis.get_rotation_quaternion().get_angle()
		test._check(landed and absf(angle - rest) < 0.02, "the pushed tree lies at %.1f° instead of 86°" % rad_to_deg(angle))
		test._check(rest - rebound > deg_to_rad(2.0), "the pushed tree did not bounce on its crown")

	# Out of sight: nothing gives the crushing vehicle away.
	effects.visible_at = func(_point: Vector3) -> bool: return false
	if sound != null:
		sound.visible_at = effects.visible_at
	pushed = effects.counts.get("object_tree_pushed", 0)
	heard = sound.requested.get("tree_fall", 0) if sound != null else 0
	var hidden: int = sound.hidden_crushes if sound != null else 0
	effects.object_event(event)
	if sound != null:
		sound.object_event(event)
	test._check(effects.counts.get("object_tree_pushed", 0) == pushed, "a tree pushed over out of sight threw leaves")
	if sound != null:
		test._check(sound.requested.get("tree_fall", 0) == heard, "a tree pushed over out of sight was heard")
		test._check(sound.hidden_crushes == hidden + 1, "the silent crush was not counted")

	effects.visible_at = effects_visible
	if sound != null:
		sound.visible_at = sound_visible
	objects.set_object_state(sim_host.object_state(tree), false)
	test._check(not objects.swapped.has(tree), "the tree is not drawn standing again")
	print("self-test: tree push ok (object %d)" % tree)
