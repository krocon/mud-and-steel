"""Factions and roles: which uniform style, headgear, equipment and weapon a soldier
gets. `build_soldier.py <faction> <role> <biome>` reads this table.

Uniform styles (period 1943-45):
- de: M43 field blouse to the upper thigh, trousers, ankle boots with canvas gaiters.
- su: gymnastyorka pullover tunic, breeches (flared thighs), tall kirza boots.
- us: M41/M43 field jacket, trousers, canvas leggings M1938, service shoes.
Winter uses white over-garments through the material system (white `Cloth`), desert
lighter drill colors; see godot/assets/models/infantry/uniform_palettes.json.
"""

UNIFORMS = {
    "de": dict(tunic_hem=0.79, tunic_flare=0.012, boot_top=0.15, gaiter_top=0.25,
               gaiter_slot="Canvas", trouser_tint=(0.96, 0.96, 0.96)),
    "su": dict(tunic_hem=0.80, tunic_flare=0.010, boot_top=0.42, breeches=0.022,
               trouser_tint=(0.94, 0.94, 0.94)),
    "us": dict(tunic_hem=0.83, tunic_flare=0.004, boot_top=0.14, gaiter_top=0.28,
               gaiter_slot="Canvas", trouser_tint=(0.80, 0.78, 0.74)),
}

# Basic equipment every soldier of a faction carries.
BASE_GEAR = {
    "de": ["belt_leather", "y_straps", "pouches_de", "bread_bag", "canteen", "shovel_side",
           "gasmask_can", "bayonet"],
    "su": ["belt_leather", "ammo_pouch_su", "gasmask_bag", "canteen", "shovel_side",
           "veshmeshok", "skatka"],
    "us": ["us_cartridge_belt", "first_aid_us", "canteen", "us_haversack"],
}

HELMET = {"de": "m35", "su": "ssh40", "us": "m1"}
CAP = {"de": "m43_cap", "su": "pilotka", "us": "m1"}

# role -> per faction: weapon, extra gear, gear to drop, headgear ("helmet"/"cap")
ROLES = {
    "rifleman": {
        "de": dict(weapon="kar98k", extra=["stick_grenade"]),
        "su": dict(weapon="mosin"),
        "us": dict(weapon="m1_garand"),
    },
    "squad_leader": {
        "de": dict(weapon="mp40", extra=["map_case", "binoculars"], drop=["pouches_de"]),
        "su": dict(weapon="ppsh41", extra=["map_case"], headgear="cap"),
        "us": dict(weapon="thompson", extra=["binoculars"]),
    },
    "mg_gunner": {
        "de": dict(weapon="mg42", extra=["holster", "mg_pouch"], drop=["pouches_de", "bayonet"]),
        "su": dict(weapon="dp27", extra=["pan_carrier"], drop=["veshmeshok"]),
        "us": dict(weapon="m1919a4", extra=["holster"]),
    },
    "auto_rifleman": {
        "us": dict(weapon="bar"),
    },
    "at_gunner": {
        "de": dict(weapon="panzerschreck", extra=["holster"], drop=["pouches_de"]),
        "su": dict(weapon="ptrd41"),
        "us": dict(weapon="bazooka"),
    },
    "sniper": {
        "de": dict(weapon="kar98k_zf39", headgear="cap", drop=["gasmask_can"]),
        "su": dict(weapon="mosin_pu", headgear="cap", drop=["skatka"]),
        "us": dict(weapon="m1903a4"),
    },
    "mortar_gunner": {
        "de": dict(weapon="legrw36", extra=["holster"], drop=["pouches_de"]),
        "su": dict(weapon="pm40"),
        "us": dict(weapon="m2_mortar", extra=["holster"]),
    },
    "medic": {
        "de": dict(weapon=None, extra=["medic_bags", "armband"], drop=["pouches_de", "bayonet"]),
        "su": dict(weapon=None, extra=["medic_bags", "armband"], drop=["ammo_pouch_su"]),
        "us": dict(weapon=None, extra=["medic_bags", "armband"]),
    },
    "radio_operator": {
        "de": dict(weapon="kar98k", extra=["radio"], drop=["gasmask_can"]),
        "su": dict(weapon="ppsh41", extra=["radio"], drop=["veshmeshok"]),
        "us": dict(weapon="m1_garand", extra=["radio"], drop=["us_haversack"]),
    },
    "flamethrower": {
        "de": dict(weapon="fmw41", extra=["flame_tanks", "holster"],
                   drop=["gasmask_can", "pouches_de"]),
        "su": dict(weapon="roks3", extra=["flame_tanks"], drop=["veshmeshok", "skatka"]),
        "us": dict(weapon="m2_flame", extra=["flame_tanks", "holster"], drop=["us_haversack"]),
    },
}

FACTIONS = ["de", "su", "us"]


def loadout(faction, role):
    if role not in ROLES or faction not in ROLES[role]:
        raise KeyError(f"no role {role!r} for faction {faction!r}")
    r = ROLES[role][faction]
    items = [i for i in BASE_GEAR[faction] if i not in r.get("drop", [])]
    items += r.get("extra", [])
    headgear = CAP[faction] if r.get("headgear") == "cap" else HELMET[faction]
    return dict(uniform=UNIFORMS[faction], headgear=headgear, items=items, weapon=r["weapon"])


def all_combinations():
    for role, per_faction in ROLES.items():
        for faction in FACTIONS:
            if faction in per_faction:
                yield faction, role


# --- de squad roles (prompt 006) ---------------------------------------------------------
# MG-Schütze 2 (assistant gunner): P 38 in the holster, spare barrel, belt drum and one
# ammunition box. Unarmed in the animation sense (no weapon in the hands), like the medic.
# Schütze 3 (ammunition bearer): Kar98k and two ammunition boxes on the back.
ROLES.setdefault("mg_assistant", {})["de"] = dict(
    weapon=None, extra=["holster", "belt_drum_de", "mg_ammo_box_de", "barrel_case_de"],
    drop=["pouches_de", "bayonet", "gasmask_can"])
ROLES.setdefault("ammo_bearer", {})["de"] = dict(
    weapon="kar98k", extra=["mg_ammo_boxes_de"], drop=["gasmask_can"])

# --- su (prompt 006): roles for the tier-1 squads -----------------------------------
# DP-27 assistant gunner: rifle plus the pan carrier with spare magazines.
ROLES.setdefault("assistant_gunner", {})["su"] = dict(weapon="mosin", extra=["pan_carrier"],
                                                      drop=["veshmeshok"])
# Rifleman with a PPSh-41 (the rifle squad's second submachine gun, catalog su.toml).
ROLES.setdefault("smg_gunner", {})["su"] = dict(weapon="ppsh41")
