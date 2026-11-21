extends RefCounted
## Kill sequences (prompt 105): a destroyed vehicle does not vanish in one
## bang. The presentation plays a short series of stages instead, chosen by
## the vehicle's class and the killing hit. Data and the deterministic
## rolls only; `effects.gd` plays the stages, `battle_sound.gd` gives each
## its sound.
##
## Stages:
## - `flash`: the killing hit with a jet of flame from the hatches.
## - `smoke`: dark smoke pours out of hatches and engine (the crew bails out
##   now, in the simulation already at the killing tick, prompt 049).
## - `fire`: the fire takes hold (the wreck's flames, light, fire loop).
## - `cookoff`: the ammunition goes up (the vehicle explosion; with an
##   ammunition explosion of the simulation, `StrikeEvent::AmmoExplosion`,
##   the big blast and the flying turret of `blast_effects.gd`).
## - `flare`: a fuel tank bursts: a big gout of flame, no blast (vehicles
##   that burn out).
## - `pop`: the few rounds at a towed gun go off: a small explosion.
##
## Table (real seconds after the killing tick: start + spread x roll):
##
## | profile | flash | smoke | fire | then |
## |---|---|---|---|---|
## | armor (tanks, tank destroyers, assault guns, self-propelled guns) | 0 | 0.25-0.45 | 1.2-1.9 | cookoff 2.6-4.4 |
## | armor_heavy (killing hit >= HEAVY_KILL_MM or ammunition explosion) | 0 | - | 0.6-1.0 | cookoff 0.15-0.3 |
## | light (halftracks, armored cars; LIGHT_COOKOFF of them) | 0 | 0.2-0.4 | 0.9-1.5 | cookoff 2.0-3.5 |
## | light_burn (the other light vehicles) | 0 | 0.2-0.4 | 0.9-1.5 | flare 2.0-3.4 |
## | light_heavy | 0 | - | 0.2-0.4 | flare 0.9-1.7 |
## | soft (trucks, rocket trucks) | 0 | 0.2-0.35 | 0.7-1.2 | flare 1.8-3.2 |
## | soft_heavy | 0 | - | 0.15-0.3 | flare 0.8-1.6 |
## | gun (towed guns, heavy mortars) | 0 | 0.15-0.3 | 1.3-1.9 | pop 0.8-1.6 |
## | gun_heavy | 0 | - | 0.5-0.9 | pop 0.1-0.25 |
##
## Units of other classes (buildings, aircraft, hand-made types) keep the
## single explosion of prompt 019. The rolls come from the unit id and the
## killing tick (`roll`), the same on every machine and in every replay;
## never from Godot's randomness.

const CLASSES: Dictionary[String, String] = {
	"tank_light": "armor", "tank_medium": "armor", "tank_heavy": "armor", "tank_destroyer": "armor",
	"assault_gun": "armor", "self_propelled": "armor",
	"halftrack": "light", "recon_vehicle": "light", "bridgelayer": "light",
	"truck": "soft", "rocket_launcher": "soft",
	"gun_at": "gun", "gun_aa_light": "gun", "gun_aa_heavy": "gun", "gun_howitzer": "gun", "mortar_heavy": "gun",
}
## Per profile: [stage, start s, spread s].
const STAGES: Dictionary[String, Array] = {
	"armor": [["flash", 0.0, 0.0], ["smoke", 0.25, 0.2], ["fire", 1.2, 0.7], ["cookoff", 2.6, 1.8]],
	"armor_heavy": [["flash", 0.0, 0.0], ["cookoff", 0.15, 0.15], ["fire", 0.6, 0.4]],
	"light": [["flash", 0.0, 0.0], ["smoke", 0.2, 0.2], ["fire", 0.9, 0.6], ["cookoff", 2.0, 1.5]],
	"light_burn": [["flash", 0.0, 0.0], ["smoke", 0.2, 0.2], ["fire", 0.9, 0.6], ["flare", 2.0, 1.4]],
	"light_heavy": [["flash", 0.0, 0.0], ["fire", 0.2, 0.2], ["flare", 0.9, 0.8]],
	"soft": [["flash", 0.0, 0.0], ["smoke", 0.2, 0.15], ["fire", 0.7, 0.5], ["flare", 1.8, 1.4]],
	"soft_heavy": [["flash", 0.0, 0.0], ["fire", 0.15, 0.15], ["flare", 0.8, 0.8]],
	"gun": [["flash", 0.0, 0.0], ["smoke", 0.15, 0.15], ["pop", 0.8, 0.8], ["fire", 1.3, 0.6]],
	"gun_heavy": [["flash", 0.0, 0.0], ["pop", 0.1, 0.15], ["fire", 0.5, 0.4]],
}
## Share of light vehicles whose ammunition cooks off (the rest burn out).
const LIGHT_COOKOFF := 0.4
## A killing hit from this caliber (mm) skips stages (the heavy guns of
## `effects.gd`, HEAVY_CALIBER).
const HEAVY_KILL_MM := 85.0
## Roll salt of the light vehicles' cook-off.
const COOKOFF_SALT := 99


## Kill class of a catalog category, "" for the single explosion.
static func kill_class(category: String) -> String:
	return CLASSES.get(category, "")


## Profile (a key of STAGES) of a unit of `category` killed at `tick`;
## `heavy`: a heavy killing hit or an ammunition explosion. "" for the
## single explosion.
static func profile(category: String, heavy: bool, unit: int, tick: int) -> String:
	var kind := kill_class(category)
	match kind:
		"":
			return ""
		"light":
			if heavy:
				return "light_heavy"
			return "light" if roll(unit, tick, COOKOFF_SALT) < LIGHT_COOKOFF else "light_burn"
		_:
			return kind + "_heavy" if heavy else kind


## The stages of `profile` for `unit` killed at `tick`: [[seconds, stage],
## ...] by time.
static func stages(profile_id: String, unit: int, tick: int) -> Array:
	var out: Array = []
	var table: Array = STAGES.get(profile_id, [])
	for k in table.size():
		var stage: Array = table[k]
		out.append([float(stage[1]) + float(stage[2]) * roll(unit, tick, k), str(stage[0])])
	out.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	return out


## A number in [0, 1) from a unit id, a tick and a salt: an integer hash,
## the same on every machine (no Godot randomness, no float in the mix).
static func roll(unit: int, tick: int, salt: int) -> float:
	var h := ((unit & 0xFFFFF) * 0x9E3779B1 + (tick & 0xFFFFFF) * 0x85EBCA77 + (salt & 0xFFFF) * 0xC2B2AE3D) & 0xFFFFFFFF
	h ^= h >> 15
	h = (h * 0x2C1B3C6D) & 0xFFFFFFFF
	h ^= h >> 12
	h = (h * 0x297A2D39) & 0xFFFFFFFF
	h ^= h >> 15
	return float(h & 0xFFFFFF) / float(0x1000000)
