extends Node
## Battle voices (prompt 053): short callouts of squads near the camera,
## in the language of their faction. Created by `battle_sound.gd`, which
## passes its combat events on; presentation only.
##
## Files: `assets/audio/voice/<faction>/<callout>/*.wav`, prepared by
## `just audio voice` from `assets-src/audio/voice/<faction>/<callout>/`
## (recorded or provided by the user, origin in docs/SOURCES.md). **None
## ship yet**: a callout without files only counts (`missing`). Tactical
## calls only, no slogans (wording: docs/ASSETS.md).
##
## [provisional] Triggers, all only up to `MAX_CAMERA` m camera distance,
## for a visible squad within `RADIUS` m of the focus:
##
## | Callout | When |
## |---|---|
## | `contact` | a squad fires after `CONTACT_QUIET` s without firing (enemy spotted) |
## | `reloading` | one in `RELOAD_ONE_IN` shots of a squad |
## | `take_cover` | a shell (from 20 mm) hits a squad |
## | `wounded` | a bullet hits a squad |
##
## At most one callout every `GLOBAL_COOLDOWN` s, per squad every
## `SQUAD_COOLDOWN` s. Faction: the prefix of the unit type id (`de`, `su`,
## `us`).

const SoundLibrary := preload("res://scenes/audio/sound_library.gd")

const CALLOUTS: Array[String] = ["contact", "take_cover", "reloading", "wounded"]
const FACTIONS: Array[String] = ["de", "su", "us"]
const MAX_CAMERA := 80.0
const RADIUS := 50.0
const CONTACT_QUIET := 20.0
const RELOAD_ONE_IN := 12
const GLOBAL_COOLDOWN := 4.0
const SQUAD_COOLDOWN := 12.0
const SHELL_MM := 20.0

var battle_sound: Node3D
## Callouts asked for, played, and left out for want of files (self-test).
var asked: Dictionary[String, int] = {}
var said: Dictionary[String, int] = {}
var missing := 0

var _time := 0.0
var _last_any := -INF
var _last_squad: Dictionary[int, float] = {}
var _last_shot: Dictionary[int, float] = {}


func setup(sound: Node3D) -> void:
	battle_sound = sound


func _process(delta: float) -> void:
	_time += delta


## A combat event (from `battle_sound.on_combat_event`).
func on_combat_event(event: Dictionary) -> void:
	if battle_sound.acoustics.camera_distance > MAX_CAMERA:
		return
	match event["type"]:
		"shot":
			var unit: int = event["unit"]
			var before: float = _last_shot.get(unit, -INF)
			_last_shot[unit] = _time
			if _time - before >= CONTACT_QUIET:
				_call(unit, "contact", event["position"])
			elif randi() % RELOAD_ONE_IN == 0:
				_call(unit, "reloading", event["position"])
		"hit":
			var target: int = event.get("target", -1)
			if target < 0 or event.get("surface", "") != "unit":
				return
			var shell := float(event.get("caliber_mm", 0.0)) >= SHELL_MM
			_call(target, "take_cover" if shell else "wounded", event["position"])


## Plays `callout` for the squad `unit` at `at` if the rules allow it.
func _call(unit: int, callout: String, at: Vector3) -> void:
	if _time - _last_any < GLOBAL_COOLDOWN or _time - float(_last_squad.get(unit, -INF)) < SQUAD_COOLDOWN:
		return
	var focus: Vector3 = battle_sound.acoustics.focus
	if Vector2(at.x - focus.x, at.z - focus.z).length() > RADIUS:
		return
	var info: Dictionary = battle_sound.unit_info(unit)
	if not str(info.get("category", "")).begins_with("inf_"):
		return
	if not battle_sound.visible_at.call(at):
		return
	say(faction_of(str(info.get("id", ""))), callout, at)
	_last_any = _time
	_last_squad[unit] = _time


## Plays a callout of a faction at `at`; false without files.
func say(faction: String, callout: String, at: Vector3) -> bool:
	asked[callout] = asked.get(callout, 0) + 1
	var key := "voice@%s/%s" % [faction, callout]
	if not SoundLibrary.has_files(key):
		missing += 1
		return false
	if battle_sound.play_at(key, at):
		said[callout] = said.get(callout, 0) + 1
		return true
	return false


## Faction of a catalog unit type id ("de_inf_rifle" → "de").
static func faction_of(type_id: String) -> String:
	return type_id.get_slice("_", 0)
