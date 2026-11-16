"""Factions and roles: which uniform style, headgear, equipment and weapon a soldier
gets. `build_soldier.py <faction> <role> <biome>` reads this table.

Uniform styles (period 1943-45):
- de: M43 field blouse to the upper thigh (four pockets, shoulder straps), trousers
  bloused over ankle boots with canvas gaiters.
- su: gymnastyorka pullover tunic (breast pockets, shoulder boards), breeches (flared
  thighs), tall kirza boots.
- us: M43 field jacket (four pockets), trousers bloused over canvas leggings M1938,
  service shoes.
Winter uses white over-garments through the material system (white `Cloth`), desert
lighter drill colors; see godot/assets/models/infantry/uniform_palettes.json.
"""

UNIFORMS = {
    "de": dict(tunic_hem=0.79, tunic_flare=0.012, boot_top=0.15, gaiter_top=0.25,
               gaiter_slot="Canvas", trouser_tint=(0.96, 0.96, 0.96), skirt_pockets=True),
    "su": dict(tunic_hem=0.80, tunic_flare=0.010, boot_top=0.42, breeches=0.022,
               blouse=0.008, trouser_tint=(0.94, 0.94, 0.94)),
    "us": dict(tunic_hem=0.83, tunic_flare=0.004, boot_top=0.14, gaiter_top=0.30,
               gaiter_slot="Canvas", trouser_tint=(0.80, 0.78, 0.74), skirt_pockets=True),
}

# Basic equipment every soldier of a faction carries.
BASE_GEAR = {
    "de": ["belt_leather", "y_straps", "pouches_de", "bread_bag", "canteen", "shovel_side",
           "gasmask_can", "bayonet"],
    "su": ["belt_leather", "ammo_pouch_su", "gasmask_bag", "canteen", "shovel_side",
           "veshmeshok", "skatka"],
    "us": ["us_cartridge_belt", "y_straps_canvas", "first_aid_us", "canteen", "us_haversack"],
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
        # catalog us_inf_command: the radio operator carries the M1 carbine (prompt 042)
        "us": dict(weapon="m1_carbine", extra=["radio"], drop=["us_haversack"]),
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

# --- prompt 008: roles for the tier-2 teams (all factions) --------------------------
# Composition per catalog entry (data/catalog/<faction>.toml, inf_* tier 2). Pistol-only
# crew members are unarmed in the animation sense (pistol in the holster), like the
# medic; infantry_preview.gd lists them in UNARMED_ROLES.
_TIER2 = {
    # heavy machine gun on its mount (Lafette 42, Sokolov, tripod M1917A1)
    "hmg_gunner": {
        "de": dict(weapon="mg42_lafette", extra=["holster", "mg_pouch"], drop=["pouches_de", "bayonet"]),
        "su": dict(weapon="maxim", drop=["veshmeshok"]),
        "us": dict(weapon="m1917a1", extra=["holster"]),
    },
    # medium mortar (8 cm s.GrW 34, 82-BM-37, 81 mm M1), assembled
    "mortar_gunner_medium": {
        "de": dict(weapon="sgrw34", extra=["holster"], drop=["pouches_de"]),
        "su": dict(weapon="bm37", drop=["veshmeshok"]),
        "us": dict(weapon="m1_81mm", extra=["holster"]),
    },
    # pistol-armed crew member with an ammunition case (mortar and HMG teams)
    "ammo_carrier": {
        "de": dict(weapon=None, extra=["holster", "ammo_case"], drop=["pouches_de", "bayonet", "gasmask_can"]),
        "us": dict(weapon=None, extra=["holster", "ammo_case"], drop=["us_haversack"]),
    },
    # carbine-armed crew member (Mosin M1938, M1 carbine); de: the Kar98k is the carbine
    "carbineer": {
        "su": dict(weapon="mosin_m38"),
        "us": dict(weapon="m1_carbine"),
    },
    # scout / sniper spotter: rifle or SMG and binoculars, field cap
    "scout": {
        "de": dict(weapon="kar98k", extra=["binoculars", "stick_grenade"], headgear="cap"),
        "su": dict(weapon="ppsh41", extra=["binoculars"], headgear="cap", drop=["skatka"]),
        "us": dict(weapon="m1_garand", extra=["binoculars"]),
    },
    # submachine gunner (de recon team: second MP 40)
    "smg_gunner": {
        "de": dict(weapon="mp40", drop=["pouches_de"], extra=["mg_pouch"]),
        "us": dict(weapon="thompson"),
    },
    # assault engineer: demolition charge, wire cutters, Bangalore torpedo (us)
    "engineer": {
        "de": dict(weapon="kar98k", extra=["demo_charge", "wire_cutters"], drop=["bread_bag"]),
        "su": dict(weapon="mosin_m38", extra=["demo_charge", "wire_cutters"], drop=["veshmeshok"]),
        "us": dict(weapon="m1_garand", extra=["demo_charge", "bangalore"], drop=["us_haversack"]),
    },
    # platoon or company officer: map case, binoculars, cap
    "officer": {
        "de": dict(weapon="mp40", extra=["map_case", "binoculars", "holster"], drop=["pouches_de", "bayonet"],
                   headgear="cap"),
        # su: TT-33 drawn (prompt 042; before: unarmed with the holster)
        "su": dict(weapon="tt33", extra=["map_case", "binoculars", "holster"], drop=["ammo_pouch_su", "skatka"],
                   headgear="cap"),
        # us: M1 carbine and the M1911A1 in the holster (catalog us_inf_command)
        "us": dict(weapon="m1_carbine", extra=["map_case", "binoculars", "holster"]),
    },
    # second man of an anti-tank team: rifle and spare rockets (de also a Panzerfaust)
    "at_assistant": {
        "de": dict(weapon="kar98k", extra=["rocket_pack", "panzerfaust"], drop=["gasmask_can"]),
        "su": dict(weapon="mosin", extra=["ammo_case"], drop=["veshmeshok"]),
        "us": dict(weapon="m1_carbine", extra=["rocket_pack"], drop=["us_haversack"]),
    },
}
for _role, _per in _TIER2.items():
    for _faction, _spec in _per.items():
        if _faction in ROLES.get(_role, {}):
            raise ValueError(f"role {_role} for {_faction} defined twice")
        ROLES.setdefault(_role, {})[_faction] = _spec

TIER2_ROLES = tuple(_TIER2)

# --- prompt 042: pistol-armed roles ----------------------------------------------------
# Krankenträger of the Sanitätstrupp (catalog de_inf_medic: P 38): P 38 in the hand,
# plain white armband and a medical bag (no Red Cross, docs/SOURCES.md).
ROLES.setdefault("stretcher_bearer", {})["de"] = dict(
    weapon="p38", extra=["holster", "medic_bags", "armband"], drop=["pouches_de", "bayonet"])

# --- prompt 049: crews that bailed out of a destroyed vehicle --------------------------
# Catalog *_inf_tank_crew: one submachine gun carried in the vehicle (commander) and
# pistols for the others; no packs, field cap (us: the M1 helmet is the faction "cap").
# Panzer wrap, padded tank helmet and tanker jacket are not modeled [provisional].
_CREW_DROP = {
    "de": ["y_straps", "pouches_de", "bread_bag", "shovel_side", "gasmask_can", "bayonet"],
    "su": ["ammo_pouch_su", "gasmask_bag", "shovel_side", "veshmeshok", "skatka"],
    "us": ["us_haversack"],
}
_CREW = {
    "crew_commander": {"de": "mp40", "su": "ppsh41", "us": "thompson"},
    "crewman": {"de": "p38", "su": "tt33", "us": "m1911a1"},
}
for _role, _per in _CREW.items():
    for _faction, _weapon in _per.items():
        ROLES.setdefault(_role, {})[_faction] = dict(
            weapon=_weapon, extra=["holster"], drop=_CREW_DROP[_faction], headgear="cap")

CREW_ROLES = tuple(_CREW)
