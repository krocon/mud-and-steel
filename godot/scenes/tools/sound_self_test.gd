extends RefCounted
## Self-test part for sound (prompt 029), run by `self_test.gd` after the
## combat, destruction and crater tests with its helpers (`_check`,
## `_frames`, `_ids_of`, `_positions_of`). Headless Godot mixes into a dummy
## driver, so this checks the bookkeeping, not what is heard:
##
## - Library: every sound class has a loadable file, loops loop.
## - Wiring: the earlier tests asked for shots, hits, a vehicle explosion,
##   a collapsing building, a falling tree and a crater, and played sounds.
## - Buses: the volume settings reach the buses, a muted slider and a lost
##   window focus mute.
## - Fog: a sound at a point the player does not see plays muffled.
## - Engines: own tanks near the listener get engine and track loops.
## - Voice limit: a battle of 200 events per frame for 30 frames never uses
##   more voices than the pools hold nor starts more sounds per frame than
##   the budget, and stays cheap.
## - Notices: orders and selection played their UI sounds.
## - Stage 2 (prompt 053): weapon folders fall back, far sounds wait for
##   the speed of sound, supersonic bullets crack, surroundings and zoom
##   set the Effects chain, callouts without files only count.
## - Signals (prompt 192): three rocket strike sirens held at once (own,
##   enemy, a rebuilt view) and three alarms 1.7 s apart (prompt 244: the
##   "under attack" announcement of a faction without recordings, which
##   is the alarm sound) never sound twice at the same time anywhere in
##   the tree; a second trigger does not restart the sounding one;
##   released, the siren fades out at the end of its cycle; explosions in
##   the same frame still overlap.
## - Announcements (prompt 244): a spoken one is no signal and sounds over
##   the siren, but never two at once.

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const SoundClasses := preload("res://scenes/audio/sound_classes.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const StrikeView := preload("res://scenes/world/strike_view.gd")

const STRESS_FRAMES := 30
const STRESS_EVENTS_PER_FRAME := 200
## Average cost of one combat event in the sound code (debug build).
const MAX_USEC_PER_EVENT := 60.0


func run(test: Node) -> void:
	var main: Node3D = test.main
	var sound: Node3D = main.battle_sound
	test._check(sound != null, "no battle sound in the match")
	if sound == null:
		return
	_library(test)
	_wiring(test, sound)
	_buses(test)
	await _fog(test, main, sound)
	await _engines(test, main, sound)
	await _stress(test, main, sound)
	await _acoustics(test, main, sound)
	await _signals(test, sound)
	test._check(sound.notices.get("ui_order", 0) > 0 and sound.notices.get("ui_select", 0) > 0,
		"no order or selection notice: %s" % sound.notices)
	test._check(UiSound.instance() != null and UiSound.play("ui_click"), "UI sounds do not play")
	print("self-test: sound ok (%d classes, played %s, dropped %s, culled %d, muffled %d, peak voices %s)" % [
		SoundLibrary.classes().size(), _total(sound.played), sound.dropped, sound.culled, sound.muffled, sound.peak_voices
	])


func _library(test: Node) -> void:
	for problem in SoundLibrary.check():
		test._check(false, problem)
	for name: String in SoundLibrary.classes():
		if not SoundLibrary.spec(name).get("loop", false):
			continue
		for stream: AudioStream in SoundLibrary.streams(name):
			var wav := stream as AudioStreamWAV
			test._check(wav == null or wav.loop_mode != AudioStreamWAV.LOOP_DISABLED, "loop '%s' does not loop" % name)


func _wiring(test: Node, sound: Node3D) -> void:
	var asked: Dictionary = sound.requested
	var shots := 0
	var hits := 0
	for name: String in asked:
		if name.begins_with("shot_"):
			shots += asked[name]
		elif name.begins_with("hit_"):
			hits += asked[name]
	test._check(shots > 0 and hits > 0, "combat asked for no shot or hit sound: %s" % asked)
	for name in ["explosion_vehicle", "building_collapse", "tree_fall", "crater"]:
		test._check(asked.get(name, 0) > 0, "no '%s' sound asked for: %s" % [name, asked])
	test._check(_total(sound.played) > 0, "no sound played during the match")


func _buses(test: Node) -> void:
	var saved: Dictionary = Settings.values.duplicate()
	for bus: String in SoundBuses.BUSES:
		test._check(AudioServer.get_bus_index(bus) >= 0, "audio bus '%s' missing" % bus)
	Settings.set_value("effects_volume", 0.5)
	var effects := AudioServer.get_bus_index("Effects")
	test._check(absf(AudioServer.get_bus_volume_db(effects) - linear_to_db(0.5)) < 0.01,
		"effects volume 0.5 set the bus to %.2f dB" % AudioServer.get_bus_volume_db(effects))
	Settings.set_value("ui_volume", 0.0)
	test._check(AudioServer.is_bus_mute(AudioServer.get_bus_index("UI")), "UI volume 0 did not mute its bus")
	Settings.set_value("mute_unfocused", true)
	SoundBuses.set_focused(false)
	var master := AudioServer.get_bus_index("Master")
	test._check(AudioServer.is_bus_mute(master), "lost focus did not mute")
	SoundBuses.set_focused(true)
	test._check(not AudioServer.is_bus_mute(master), "focus did not unmute")
	Settings.values = saved
	Settings.apply()
	test._check(not AudioServer.is_bus_mute(AudioServer.get_bus_index("UI")), "UI bus still muted after restoring")


func _fog(test: Node, main: Node3D, sound: Node3D) -> void:
	var visible_at: Callable = sound.visible_at
	sound.visible_at = func(_point: Vector3) -> bool: return false
	await test._frames(1)
	var before: int = sound.muffled
	var at: Vector3 = sound.listener.global_position
	at.y = main.sim_host.height_at(at.x, at.z)
	var ok: bool = sound.play_at("shot_cannon_large", at)
	sound.visible_at = visible_at
	test._check(ok and sound.muffled == before + 1, "a hidden shot did not play muffled")


func _engines(test: Node, main: Node3D, sound: Node3D) -> void:
	var own: Array[int] = test._ids_of(true)
	var alive: PackedByteArray = main.sim_host.unit_alive()
	var living: Array[int] = []
	var indices: Array[int] = test._indices(own)
	for k in own.size():
		if alive[indices[k]] != 0:
			living.append(own[k])
	if living.is_empty():
		return
	main.camera_rig.snap_to(test._positions_of(living)[0], 60.0, 0.0)
	await test._frames(30)
	test._check(sound.active_loops() > 0, "no engine loop near the own tanks")
	var engines := 0
	for name: String in sound.played:
		if name.begins_with("engine_"):
			engines += sound.played[name]
	test._check(engines > 0 and sound.played.get("tracks", 0) > 0, "no engine or track loop started: %s" % sound.played)


func _stress(test: Node, main: Node3D, sound: Node3D) -> void:
	var sim_host: SimHost = main.sim_host
	var own: Array[int] = test._ids_of(true)
	var squads: Array[int] = test._ids_of(true, true)
	var shooters: Array[int] = own + squads
	var center: Vector3 = sound.listener.global_position
	var players_before := _players(sound)
	var groups: Dictionary = SoundLibrary.groups()
	var budget := 0
	for group: String in groups:
		if group != "ui":
			budget += int(groups[group].get("per_frame", 3))
	var played_before := _total(sound.played)
	var usec := 0
	for frame in STRESS_FRAMES:
		var began := Time.get_ticks_usec()
		for k in STRESS_EVENTS_PER_FRAME:
			var at := center + Vector3(randf_range(-200.0, 200.0), 0.0, randf_range(-200.0, 200.0))
			at.y = sim_host.height_at(at.x, at.z)
			var caliber: float = [7.92, 7.92, 7.92, 20.0, 75.0, 88.0][k % 6]
			if k % 2 == 0:
				sound.on_combat_event({"type": "shot", "unit": shooters[k % shooters.size()], "weapon": k % 3,
					"position": at, "caliber_mm": caliber})
			else:
				var surface: String = ["ground", "unit", "object"][k % 3]
				sound.on_combat_event({"type": "hit", "surface": surface, "target": own[k % own.size()] if surface == "unit" else -1,
					"position": at, "caliber_mm": caliber, "penetrated": k % 5 == 0})
		usec += Time.get_ticks_usec() - began
		var busy: Dictionary = sound.busy_voices()
		for group: String in busy:
			test._check(busy[group] <= sound.pool_size(group), "group '%s' uses %d of %d voices" % [group, busy[group], sound.pool_size(group)])
		await test._frames(1)
	var started := _total(sound.played) - played_before
	test._check(started <= STRESS_FRAMES * budget, "%d sounds started in %d frames, budget %d per frame" % [started, STRESS_FRAMES, budget])
	test._check(started > 0, "the stress battle played nothing")
	test._check(_players(sound) == players_before, "voice players grew from %d to %d" % [players_before, _players(sound)])
	var per_event := float(usec) / float(STRESS_FRAMES * STRESS_EVENTS_PER_FRAME)
	test._check(per_event < MAX_USEC_PER_EVENT, "a combat event costs %.1f µs of sound code" % per_event)
	print("self-test: sound stress: %d events in %d frames, %d sounds started, %.1f µs per event, peak voices %s" % [
		STRESS_FRAMES * STRESS_EVENTS_PER_FRAME, STRESS_FRAMES, started, per_event, sound.peak_voices
	])


## Stage 2 (prompt 053): weapon folders, speed of sound, supersonic crack,
## surroundings, zoom filter, voices without files.
func _acoustics(test: Node, main: Node3D, sound: Node3D) -> void:
	var acoustics: Node = sound.acoustics
	test._check(acoustics != null and sound.voices != null, "no acoustics or voices in the battle sound")
	if acoustics == null:
		return
	test._check(SoundClasses.weapon_slug("MG 42") == "mg_42" and SoundClasses.weapon_slug("Browning M2HB (AA)") == "browning_m2hb",
		"weapon slugs: %s, %s" % [SoundClasses.weapon_slug("MG 42"), SoundClasses.weapon_slug("Browning M2HB (AA)")])
	test._check(SoundLibrary.spec("shot_mg@no_such_weapon") == SoundLibrary.spec("shot_mg") and not SoundLibrary.has_files("shot_mg@no_such_weapon"),
		"a weapon without recordings does not fall back to its class")
	var rig: Node3D = main.camera_rig
	var focus: Vector3 = rig.focus
	var distance: float = rig.distance
	rig.snap_to(focus, 60.0, 0.0)
	await test._frames(40)
	# Speed of sound: 400 m away waits 400 / 686 s.
	var far_at := focus + Vector3(400.0, 0.0, 0.0)
	far_at.y = main.sim_host.height_at(far_at.x, far_at.z)
	var wait: float = acoustics.delay_of(far_at)
	test._check(absf(wait - 400.0 / 686.0) < 0.02, "a sound 400 m away waits %.2f s" % wait)
	# Let the loud close shots of the stress part ring out first: a far
	# shot cannot take a voice from them (it passed before only while the
	# match's own battle played large guns, prompt 050).
	var settle := 0
	while sound.busy_voices().get("cannon", 0) > 0 and settle < 300:
		settle += 1
		await test._frames(1)
	var played_before: int = sound.played.get("shot_cannon_large", 0)
	var delayed_before: int = acoustics.delayed
	sound.play_far("shot_cannon_large", far_at)
	test._check(acoustics.delayed == delayed_before + 1 and sound.played.get("shot_cannon_large", 0) == played_before,
		"a far shot did not wait for its sound")
	await test._frames(int(wait * 60.0) + 10)
	test._check(sound.played.get("shot_cannon_large", 0) > played_before, "the delayed shot never played")
	# Supersonic crack: a bullet passing 3 m from the ear on this tick.
	var ear: Vector3 = rig.focus + Vector3.UP * 1.5
	var cracks: int = acoustics.cracks
	acoustics.scan_cracks(PackedVector3Array([ear + Vector3(-5.0, 0.5, 3.0)]), PackedVector3Array([Vector3(1500.0, 0.0, 0.0)]), PackedFloat32Array([7.92]), 0.05)
	test._check(acoustics.cracks == cracks + 1, "a passing supersonic bullet did not crack")
	await test._frames(1)
	cracks = acoustics.cracks
	acoustics.scan_cracks(PackedVector3Array([ear + Vector3(-5.0, 0.5, 3.0), ear + Vector3(5.0, 0.0, 0.0)]),
		PackedVector3Array([Vector3(300.0, 0.0, 0.0), Vector3(1500.0, 0.0, 0.0)]), PackedFloat32Array([7.92, 7.92]), 0.05)
	test._check(acoustics.cracks == cracks, "a subsonic or receding bullet cracked")
	# Surroundings and the Effects chain.
	var shares: Dictionary = acoustics.surroundings
	var total := float(shares.open) + float(shares.forest) + float(shares.village)
	test._check(absf(total - 1.0) < 0.01, "surrounding shares sum to %.2f: %s" % [total, shares])
	var chain := SoundBuses.ensure_effects()
	var bus := AudioServer.get_bus_index("Effects")
	test._check(AudioServer.get_bus_effect_count(bus) == 2, "Effects bus has %d effects" % AudioServer.get_bus_effect_count(bus))
	# Zoom: the overview is darker.
	var near_cutoff: float = (chain[0] as AudioEffectLowPassFilter).cutoff_hz
	rig.snap_to(focus, 500.0, 0.0)
	await test._frames(3)
	var far_cutoff: float = (chain[0] as AudioEffectLowPassFilter).cutoff_hz
	test._check(far_cutoff < near_cutoff * 0.5 and acoustics.far > 0.9, "zoom filter: %.0f Hz near, %.0f Hz far" % [near_cutoff, far_cutoff])
	rig.snap_to(focus, distance, 0.0)
	# Voices: no files ship, a callout only counts.
	var voices: Node = sound.voices
	var missing: int = voices.missing
	var said: bool = voices.say("de", "contact", focus)
	test._check(not said and voices.missing == missing + 1, "a callout without files played or was not counted")
	print("self-test: sound stage 2 ok (delayed %d, queue dropped %d, cracks %d, surroundings %s, aircraft loops %d, air events %d, callouts %s)" % [
		acoustics.delayed, acoustics.queue_dropped, acoustics.cracks, shares, acoustics.aircraft_loops, acoustics.air_events, voices.asked
	])


## Prompt 192: at most one signal (siren, alarm) at a time in the game.
func _signals(test: Node, sound: Node3D) -> void:
	var ui: Node = UiSound.instance()
	test._check(ui != null, "no UI sound node for the signals")
	if ui == null:
		return
	# An alarm of the match before may still sound (8 s at most).
	var settle := 0
	while not str(ui.signal_playing()).is_empty() and settle < 720:
		settle += 1
		await test._frames(1)
	test._check(str(ui.signal_playing()).is_empty(), "a signal still sounds: %s" % ui.signal_playing())
	var siren: AudioStream = StrikeView._siren_wav
	test._check(siren != null, "the strike view built no siren")
	if siren == null:
		return
	var started_before: int = _total(ui.signals_started)
	var absorbed_before: int = ui.signals_absorbed
	var faded_before: int = ui.signals_faded
	var keys := ["%s:own" % StrikeView.SIREN_KEY, "%s:enemy" % StrikeView.SIREN_KEY, "%s:rebuilt" % StrikeView.SIREN_KEY]
	var peak := 0
	var position_before := -1.0
	var restarted := false
	var at: Vector3 = sound.listener.global_position
	var explosions := 0
	# The spoken announcement, or in a clone without the private sound
	# repository a notice in its place: any stream shows the one voice.
	var line: String = sound.announce_key("under_attack", "de")
	if not line.contains("@"):
		line = "ui_unit_ready"
	var spoken_before: int = ui.spoken
	var speech_dropped_before: int = ui.speech_dropped
	var spoke := false
	var spoke_twice := false
	var over_siren := false
	var alarms_before: int = sound.announced.get("ui_under_attack", 0)
	for frame in 360:
		# Strikes start 0.5 s apart; three alarms 1.7 s apart.
		var strike := int(frame / 30.0)
		if frame % 30 == 0 and strike < keys.size():
			UiSound.hold_signal(keys[strike], true, siren, StrikeView.SIREN_DB, &"Effects")
		if frame % 100 == 0 and frame > 0:
			sound.announce("under_attack", "zz")
		if frame == 150:
			spoke = UiSound.speak(line)
			spoke_twice = UiSound.speak(line)
		if frame == 152:
			over_siren = ui.speaking() and str(ui.signal_playing()).begins_with(StrikeView.SIREN_KEY)
		if frame == 120:
			for k in 3:
				sound.play_at("hit_shell", at + Vector3(4.0 * k, 0.0, 0.0))
		if frame == 121:
			explosions = int(sound.busy_voices().get("explosion", 0))
		await test._frames(1)
		if frame % 4 == 0:
			peak = maxi(peak, _signal_voices(test, ui))
		var pos: float = (ui._signal as AudioStreamPlayer).get_playback_position()
		if frame > 40 and pos + 0.5 < position_before and position_before < siren.get_length() - 0.3:
			restarted = true
		position_before = pos
	var started: int = _total(ui.signals_started) - started_before
	var absorbed: int = ui.signals_absorbed - absorbed_before
	test._check(peak <= 1, "%d signals sounded at once" % peak)
	test._check(started == 1 and str(ui.signal_playing()).begins_with(StrikeView.SIREN_KEY),
		"signals started %d times, sounding '%s'" % [started, ui.signal_playing()])
	test._check(absorbed >= 3 and not restarted, "alarms absorbed %d, siren restarted %s" % [absorbed, restarted])
	test._check(sound.announced.get("ui_under_attack", 0) == alarms_before + 3,
		"a faction without recordings announced %d times with the alarm sound, expected 3" % (
			sound.announced.get("ui_under_attack", 0) - alarms_before))
	test._check(spoke and over_siren, "the announcement '%s' over the siren: started %s, both sounding %s" % [line, spoke, over_siren])
	test._check(not spoke_twice and ui.spoken == spoken_before + 1 and ui.speech_dropped == speech_dropped_before + 1,
		"two announcements at once: second started %s, spoken %d, left out %d" % [
			spoke_twice, ui.spoken - spoken_before, ui.speech_dropped - speech_dropped_before])
	test._check(explosions >= 2, "explosions in one frame use %d voices" % explosions)
	# Released by all three, it fades out at the end of a cycle.
	for key: String in keys:
		UiSound.hold_signal(key, false)
	var waited := 0
	while not str(ui.signal_playing()).is_empty() and waited < int((siren.get_length() + 1.0) * 60.0):
		waited += 1
		await test._frames(1)
	test._check(str(ui.signal_playing()).is_empty() and ui.signals_faded == faded_before + 1,
		"released siren still sounds after %d frames (faded %d)" % [waited, ui.signals_faded - faded_before])
	print("self-test: signals ok (peak %d at once, started %d, absorbed %d, explosion voices %d, faded out after %.1f s; announcement '%s' over the siren)" % [
		peak, started, absorbed, explosions, waited / 60.0, line])


## Players anywhere in the tree playing a signal stream.
func _signal_voices(test: Node, ui: Node) -> int:
	var streams: Array = ui.signal_streams()
	var n := 0
	var root: Node = test.get_tree().root
	for type: String in ["AudioStreamPlayer", "AudioStreamPlayer2D", "AudioStreamPlayer3D"]:
		for player: Node in root.find_children("*", type, true, false):
			if player.get(&"playing") and streams.has(player.get(&"stream")):
				n += 1
	return n


func _players(sound: Node) -> int:
	return sound.find_children("*", "AudioStreamPlayer3D", false, false).size()


func _total(counts: Dictionary) -> int:
	var n := 0
	for key in counts:
		n += int(counts[key])
	return n
