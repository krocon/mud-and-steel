extends RefCounted
## Self-test part for music and ambience (prompt 030), run by
## `self_test.gd` after the sound test with its helpers (`_check`,
## `_frames`). Headless Godot mixes into a dummy driver, so this checks the
## bookkeeping, not what is heard:
##
## - Library: every music mood has a piece, pieces are compressed MP3 that
##   do not loop; every ambience layer has a file, beds loop.
## - Music: the match started calm, one shot does not switch to combat,
##   enough visible fighting does, and it goes back to calm after the fight
##   has died down (not before `MIN_COMBAT_SECONDS`).
## - Ambience: each biome plays its own wind and insects; the grid finds
##   water and wood and makes them heard; close up the details are louder
##   and the wind quieter than in the overview; combat nearby silences the
##   birds, they sing again later; a rumble plays.
##
## `flow_result` and `check_menu` are called by `menu/flow_self_test.gd`:
## in the demo battle of match 2 the music switches from calm to combat
## and ends with the victory piece; back in the menu the menu music plays.

const Music := preload("res://scenes/audio/music.gd")
const Ambience := preload("res://scenes/audio/ambience.gd")
const MatchMusic := preload("res://scenes/audio/match_music.gd")


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sound: Node3D = main.battle_sound
	if sound == null:
		return  # the sound test reports the missing battle sound
	for problem in Music.check():
		test._check(false, problem)
	for problem in Ambience.check():
		test._check(false, problem)
	_music(test, sound.match_music)
	await _ambience(test, main, sound.ambience)
	print("self-test: music and ambience ok (music %s, switches %s, peak intensity %.1f; birds %d, silenced %d, rumbles %d)" % [
		Array(Music.instance().history if Music.instance() != null else PackedStringArray()), Array(sound.match_music.switches),
		sound.match_music.peak_intensity, sound.ambience.birds_played, sound.ambience.birds_silenced, sound.ambience.rumbles
	])


func _music(test: Node, music: MatchMusic) -> void:
	var node := Music.instance()
	test._check(node != null and node.is_inside_tree(), "the music node is missing")
	if node == null:
		return
	test._check(music.switches.size() > 0 and music.switches[0] == "calm", "the match did not start calm: %s" % music.switches)
	test._check(Music.current_mood() == music.state, "music plays '%s', match wants '%s'" % [Music.current_mood(), music.state])
	test._check(node.pieces_started > 0 and not node.current_piece.is_empty(), "no music piece started")
	# Let any fighting of the earlier tests die down.
	music.intensity = 0.0
	music.advance(MatchMusic.MIN_COMBAT_SECONDS + 1.0)
	test._check(music.state == "calm", "no calm after the fighting: %s" % music.state)
	music.add(MatchMusic.WEIGHT_GUN_SHOT)
	music.advance(0.1)
	test._check(music.state == "calm", "one shot switched to combat")
	for i in 10:
		music.add(MatchMusic.WEIGHT_GUN_SHOT)
	music.advance(0.1)
	test._check(music.state == "combat" and Music.current_mood() == "combat", "ten gun shots did not start combat music")
	music.intensity = 0.0
	music.advance(1.0)
	test._check(music.state == "combat", "combat music ended right after the fight")
	music.advance(MatchMusic.MIN_COMBAT_SECONDS)
	test._check(music.state == "calm" and Music.current_mood() == "calm", "combat music did not end after the fight")


func _ambience(test: Node, main: Node3D, ambience: Ambience) -> void:
	var original: String = main.atmosphere.preset.id
	for biome: String in Ambience.BIOMES:
		main.set_biome(biome)
		await test._frames(2)
		var spec: Dictionary = Ambience.BIOMES[biome]
		test._check(ambience.biome == biome and ambience.bed_layer("wind") == spec.wind, "biome %s plays wind '%s'" % [biome, ambience.bed_layer("wind")])
		test._check(ambience.bed_layer("insects") == spec.insects, "biome %s plays insects '%s'" % [biome, ambience.bed_layer("insects")])
		test._check(ambience.targets().get("wind", Ambience.SILENT_DB) > Ambience.SILENT_DB, "no wind in %s" % biome)
	main.set_biome(original)
	await test._frames(2)
	var players := ambience.get_child_count()
	# Water and wood on the map are heard at their place.
	var host: SimHost = main.sim_host
	var water := Vector3.INF
	var wood := Vector3.INF
	var half: Vector2 = host.map_size() * 0.5
	var x := -half.x + 20.0
	while x < half.x and (water == Vector3.INF or wood == Vector3.INF):
		var z := -half.y + 20.0
		while z < half.y:
			if water == Vector3.INF and Ambience.is_water(host.terrain_at(x, z)):
				water = Vector3(x, 0.0, z)
			if wood == Vector3.INF and host.wood_density_at(x, z) >= 0.5:
				wood = Vector3(x, 0.0, z)
			z += 16.0
		x += 16.0
	for found: Array in [[water, "water", "water_share"], [wood, "leaves", "wood_share"]]:
		if found[0] == Vector3.INF:
			print("self-test: the map has no %s, its ambience is not checked" % found[1])
			continue
		ambience.center_override = found[0]
		ambience.sample()
		ambience.update_targets()
		test._check(float(ambience.get(found[2])) > 0.0 and ambience.targets()[found[1]] > Ambience.SILENT_DB,
			"%s at %s is not heard (%s %.2f)" % [found[1], found[0], found[2], ambience.get(found[2])])
	ambience.center_override = null
	# Camera height: details near, wind far (without moving the camera,
	# which the HUD test after this one relies on).
	ambience.distance_override = Ambience.NEAR_DISTANCE
	ambience.update_targets()
	var near_targets := ambience.targets()
	ambience.distance_override = Ambience.FAR_DISTANCE
	ambience.update_targets()
	var far_targets := ambience.targets()
	ambience.distance_override = null
	test._check(near_targets.wind < far_targets.wind, "wind not louder in the overview: %s / %s" % [near_targets.wind, far_targets.wind])
	if near_targets.insects > Ambience.SILENT_DB:
		test._check(near_targets.insects > far_targets.insects, "insects not louder close up")
	# Birds: silent after combat nearby, back later.
	var center: Vector3 = ambience._center()
	ambience.on_combat_event({"type": "hit", "position": center})
	test._check(not ambience.try_bird(), "a bird sang right after combat nearby")
	ambience.silent_until = -INF
	test._check(ambience.try_bird(), "no bird sang long after the fight")
	test._check(ambience.play_rumble(), "no rumble")
	test._check(ambience.get_child_count() == players, "ambience players grew from %d to %d" % [players, ambience.get_child_count()])


## Match 2 of the flow test (real combat, then victory): the demo battle
## switched to combat music and the result plays `expected`.
static func flow_result(test: Node, main: Node3D, expected: String) -> void:
	var sound: Node3D = main.battle_sound
	if sound == null:
		test._check(false, "no battle sound in match 2")
		return
	var music: MatchMusic = sound.match_music
	for i in 180:
		if Music.current_mood() == expected:
			break
		await test.get_tree().process_frame
	test._check("combat" in music.switches, "the battle never switched to combat music (peak intensity %.1f)" % music.peak_intensity)
	test._check(Music.current_mood() == expected, "the result plays '%s', not '%s'" % [Music.current_mood(), expected])
	print("flow self-test: music %s, peak intensity %.1f" % [Array(music.switches), music.peak_intensity])


## Back in the main menu, the menu music plays.
static func check_menu(test: Node) -> void:
	test._check(Music.current_mood() == "menu", "the menu plays '%s', not the menu music" % Music.current_mood())
