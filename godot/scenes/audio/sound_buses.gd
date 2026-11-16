extends RefCounted
## Audio buses (prompt 029), created in code so no bus layout file is
## needed:
##
##     Master ─┬─ Effects ── Muffled (low-pass, quieter: sounds in the fog)
##             │  (effects: zoom low-pass, reverb; prompt 053, `ensure_effects`)
##             ├─ UI
##             ├─ Music      (prompt 030)
##             └─ Ambience   (prompt 030)
##
## `apply(values)` sets every bus volume from the settings
## (`menu/settings.gd`, keys in `VOLUME_KEYS`) and mutes Master while the
## window has no focus if the player wants that.

## Bus -> the bus it sends to.
const BUSES: Dictionary[String, String] = {
	"Effects": "Master", "Muffled": "Effects", "UI": "Master", "Music": "Master", "Ambience": "Master",
}
## Settings key of each bus volume (linear 0..1).
const VOLUME_KEYS: Dictionary[String, String] = {
	"Master": "master_volume", "Effects": "effects_volume", "UI": "ui_volume",
	"Music": "music_volume", "Ambience": "ambience_volume",
}
## Sounds in the fog of war: everything above this frequency is damped.
const MUFFLED_CUTOFF_HZ := 700.0
const MUFFLED_DB := -6.0
## Longest wait for the audio thread to drop stopped playbacks (`release`).
const RELEASE_MAX_MSEC := 500

## False while the window has no focus (`ui_sound.gd`).
static var focused := true
static var _values := {}


## Creates the missing buses (idempotent).
static func ensure() -> void:
	for bus: String in BUSES:
		if AudioServer.get_bus_index(bus) >= 0:
			continue
		AudioServer.add_bus()
		var index := AudioServer.bus_count - 1
		AudioServer.set_bus_name(index, bus)
	# Sends are set after all buses exist (Muffled sends to Effects).
	for bus: String in BUSES:
		AudioServer.set_bus_send(AudioServer.get_bus_index(bus), BUSES[bus])
	var muffled := AudioServer.get_bus_index("Muffled")
	if AudioServer.get_bus_effect_count(muffled) == 0:
		var low_pass := AudioEffectLowPassFilter.new()
		low_pass.cutoff_hz = MUFFLED_CUTOFF_HZ
		AudioServer.add_bus_effect(muffled, low_pass)
		AudioServer.set_bus_volume_db(muffled, MUFFLED_DB)


## The effects of the Effects bus (prompt 053, `acoustics.gd`): a low-pass
## for the zoom and a reverb for the surroundings, created once:
## `[AudioEffectLowPassFilter, AudioEffectReverb]`.
static func ensure_effects() -> Array:
	ensure()
	var bus := AudioServer.get_bus_index("Effects")
	var low_pass: AudioEffectLowPassFilter = null
	var reverb: AudioEffectReverb = null
	for i in AudioServer.get_bus_effect_count(bus):
		var effect := AudioServer.get_bus_effect(bus, i)
		if effect is AudioEffectLowPassFilter:
			low_pass = effect
		elif effect is AudioEffectReverb:
			reverb = effect
	if low_pass == null:
		low_pass = AudioEffectLowPassFilter.new()
		low_pass.cutoff_hz = 20000.0
		AudioServer.add_bus_effect(bus, low_pass)
	if reverb == null:
		reverb = AudioEffectReverb.new()
		reverb.dry = 1.0
		reverb.wet = 0.0
		reverb.spread = 1.0
		AudioServer.add_bus_effect(bus, reverb)
	return [low_pass, reverb]


## Leaves the Effects bus neutral (end of a match).
static func reset_effects() -> void:
	var chain := ensure_effects()
	(chain[0] as AudioEffectLowPassFilter).cutoff_hz = 20000.0
	(chain[1] as AudioEffectReverb).wet = 0.0


## Sets the bus volumes from the settings values.
static func apply(values: Dictionary) -> void:
	_values = values
	ensure()
	var focus_muted: bool = not focused and bool(values.get("mute_unfocused", true))
	for bus: String in VOLUME_KEYS:
		var index := AudioServer.get_bus_index(bus)
		var volume: float = values.get(VOLUME_KEYS[bus], 1.0)
		var muted := volume <= 0.001 or (bus == "Master" and focus_muted)
		AudioServer.set_bus_mute(index, muted)
		AudioServer.set_bus_volume_db(index, linear_to_db(maxf(volume, 0.001)))


## Focus changes of the window: mutes or unmutes Master (setting
## `mute_unfocused`).
static func set_focused(on: bool) -> void:
	focused = on
	apply(_values)


## Stops `players` and gives the audio thread a moment to let go of their
## playbacks; else streams still playing when the game quits are reported
## as leaked (the thread drops stopped playbacks on its next mix).
static func release(players: Array) -> void:
	for player: Node in players:
		player.stop()
		player.set(&"stream", null)
	if players.is_empty():
		return
	# Two mixes after the stop: the first may have started before it.
	var mixes := 0
	var waited := 0
	var last := AudioServer.get_time_since_last_mix()
	while mixes < 2 and waited < RELEASE_MAX_MSEC:
		OS.delay_msec(5)
		waited += 5
		var since := AudioServer.get_time_since_last_mix()
		if since < last:
			mixes += 1
		last = since
