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

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")
const SoundBuses := preload("res://scenes/audio/sound_buses.gd")
const UiSound := preload("res://scenes/audio/ui_sound.gd")
const Settings := preload("res://scenes/menu/settings.gd")

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


func _players(sound: Node) -> int:
	return sound.find_children("*", "AudioStreamPlayer3D", false, false).size()


func _total(counts: Dictionary) -> int:
	var n := 0
	for key in counts:
		n += int(counts[key])
	return n
