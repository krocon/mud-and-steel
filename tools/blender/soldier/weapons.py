"""Low-poly small arms, built from parameters with real dimensions.

Weapon space: origin at the right-hand grip (pistol grip or wrist of the stock),
+Y towards the muzzle, +Z up. Every weapon defines the points
- `grip_r` (always the origin) and `grip_l` (support hand),
- `butt` (shoulder contact; launchers: shoulder rest),
- `muzzle` (becomes the `Socket_Muzzle` empty, +Y forward),
- `ground` (contact point when the weapon stands on the ground: mortars, bipods).

Dimensions (overall length, barrel length, caliber) are real values. Sources are
listed per weapon in `source`; values the sources disagree on, or that were estimated,
are named in `uncertain` (same rules as the unit catalog, docs/SOURCES.md).
"""

import math

from mathutils import Matrix, Vector

from .mesh import MeshBuilder

W = {"Weapon": 1.0}

WIKI = "https://en.wikipedia.org/wiki/"

WEAPONS = {
    # --- de ---------------------------------------------------------------------------
    "kar98k": dict(kind="rifle", length=1.110, barrel=0.600, pull=0.33, full_stock=True,
                   source=[WIKI + "Karabiner_98k"]),
    "kar98k_zf39": dict(kind="rifle", length=1.110, barrel=0.600, pull=0.33, full_stock=True,
                        scope=0.27, source=[WIKI + "Karabiner_98k", WIKI + "Zielfernrohr_39"]),
    "mp40": dict(kind="smg", length=0.833, barrel=0.251, pull=0.26, mag=(0.11, 0.25),
                 folding_stock=True, source=[WIKI + "MP_40"]),
    "mg42": dict(kind="mg", length=1.220, barrel=0.533, pull=0.30, bipod=0.80, jacket=True,
                 belt=True, source=[WIKI + "MG_42"]),
    "panzerschreck": dict(kind="launcher", length=1.640, caliber=0.088, rear=0.62,
                          shield=True, source=[WIKI + "Panzerschreck"]),
    "legrw36": dict(kind="mortar", barrel=0.465, caliber=0.050, base=(0.30, 0.22),
                    source=[WIKI + "5_cm_leGrW_36"]),
    "fmw41": dict(kind="flame", length=0.70, source=[WIKI + "Flammenwerfer_41"],
                  uncertain=["length"]),
    # --- su ---------------------------------------------------------------------------
    "mosin": dict(kind="rifle", length=1.232, barrel=0.730, pull=0.33, full_stock=True,
                  source=[WIKI + "Mosin–Nagant"]),
    "mosin_pu": dict(kind="rifle", length=1.232, barrel=0.730, pull=0.33, full_stock=True,
                     scope=0.17, source=[WIKI + "Mosin–Nagant", WIKI + "PU_scope"]),
    "ppsh41": dict(kind="smg", length=0.843, barrel=0.269, pull=0.30, drum=0.115, wood_stock=True,
                   jacket=True, source=[WIKI + "PPSh-41"]),
    "dp27": dict(kind="mg", length=1.270, barrel=0.605, pull=0.32, bipod=0.78, pan=0.25,
                 wood_stock=True, jacket=True, source=[WIKI + "DP-27"]),
    "ptrd41": dict(kind="atr", length=2.000, barrel=1.350, pull=0.32, bipod=0.55,
                   source=[WIKI + "PTRD-41"]),
    "pm40": dict(kind="mortar", barrel=0.630, caliber=0.050, base=(0.32, 0.25),
                 source=[WIKI + "50-PM_40"], uncertain=["barrel"]),
    "roks3": dict(kind="flame", length=0.95, rifle_like=True, source=[WIKI + "ROKS-3"],
                  uncertain=["length"]),
    # --- us ---------------------------------------------------------------------------
    "m1_garand": dict(kind="rifle", length=1.100, barrel=0.610, pull=0.33, full_stock=True,
                      source=[WIKI + "M1_Garand"]),
    "m1903a4": dict(kind="rifle", length=1.105, barrel=0.610, pull=0.33, full_stock=True,
                    scope=0.29, source=[WIKI + "M1903_Springfield"]),
    "thompson": dict(kind="smg", length=0.813, barrel=0.267, pull=0.30, mag=(0.07, 0.22),
                     wood_stock=True, source=[WIKI + "Thompson_submachine_gun"]),
    "bar": dict(kind="mg", length=1.214, barrel=0.610, pull=0.32, bipod=0.75, mag=(0.03, 0.17),
                wood_stock=True, source=[WIKI + "M1918_Browning_automatic_rifle"]),
    "m1919a4": dict(kind="mg", length=0.964, barrel=0.610, pull=0.10, tripod=True, jacket=True,
                    belt=True, source=[WIKI + "M1919_Browning_machine_gun"]),
    "bazooka": dict(kind="launcher", length=1.550, caliber=0.060, rear=0.55,
                    source=[WIKI + "Bazooka"]),
    "m2_mortar": dict(kind="mortar", barrel=0.726, caliber=0.060, base=(0.33, 0.26),
                      source=[WIKI + "M2_mortar"]),
    "m2_flame": dict(kind="flame", length=0.90, source=[WIKI + "M2_flamethrower"],
                     uncertain=["length"]),
}


def _rx(deg):
    return Matrix.Rotation(math.radians(deg), 3, "X")


class Weapon:
    def __init__(self, wid):
        self.id = wid
        self.spec = WEAPONS[wid]
        self.mesh = MeshBuilder(["Metal", "Wood", "Paint"])
        self.points = {"grip_r": Vector((0, 0, 0))}

    @property
    def kind(self):
        return self.spec["kind"]


BORE = 0.035  # bore axis above the grip
MORTAR_ELEVATION = 55.0  # degrees, deployed pose (animations use the same value)


def _barrel(b, y0, y1, r=0.0095, jacket=False):
    if jacket:
        b.cylinder((0, y0, BORE), (0, y1 - 0.04, BORE), r * 2.2, "Metal", W, sides=8)
        b.cylinder((0, y1 - 0.04, BORE), (0, y1, BORE), r, "Metal", W, sides=6)
    else:
        b.cylinder((0, y0, BORE), (0, y1, BORE), r, "Metal", W, sides=6)


def _stock(b, butt_y, slot="Wood", drop=0.03):
    # Butt stock from the butt plate to the wrist at the grip.
    b.box((0, butt_y + 0.10, BORE - drop - 0.02), (0.042, 0.20, 0.12), slot, W, taper=1.0)
    b.box((0, butt_y + 0.24, BORE - drop * 0.4 - 0.005), (0.036, 0.12, 0.055), slot, W)


def _pistol_grip(b, slot="Wood"):
    b.box((0, -0.01, -0.03), (0.03, 0.04, 0.10), slot, W, rot=_rx(-15))


def _bipod(b, y, folded_len=0.32):
    for sx in (-1, 1):
        top = Vector((0, y, BORE - 0.01))
        foot = Vector((sx * 0.14, y + 0.08, BORE - folded_len))
        b.cylinder(top, foot, 0.007, "Metal", W, sides=4)
    return Vector((0, y + 0.08, BORE - folded_len))


def _scope(b, y0, length):
    b.cylinder((0, y0, BORE + 0.055), (0, y0 + length, BORE + 0.055), 0.016, "Metal", W, sides=8)
    b.box((0, y0 + length * 0.5, BORE + 0.035), (0.02, length * 0.6, 0.02), "Metal", W)


def build_rifle(w):
    s, b = w.spec, w.mesh
    pull, length, barrel = s["pull"], s["length"], s["barrel"]
    butt_y = -pull
    muzzle_y = length - pull
    rec_end = muzzle_y - barrel
    _stock(b, butt_y)
    # Receiver and bolt.
    b.box((0, rec_end * 0.5 + 0.01, BORE), (0.034, rec_end + 0.04, 0.05), "Metal", W)
    b.box((0.03, 0.05, BORE + 0.02), (0.035, 0.012, 0.012), "Metal", W)
    if s.get("full_stock"):
        fore_end = muzzle_y - 0.09
        b.box((0, (rec_end + fore_end) * 0.5, BORE - 0.012), (0.04, fore_end - rec_end, 0.045),
              "Wood", W, taper=0.85)
        # Magazine well below the receiver.
        b.box((0, 0.10, BORE - 0.045), (0.03, 0.09, 0.04), "Wood", W)
    _barrel(b, rec_end, muzzle_y)
    if s.get("scope"):
        _scope(b, -0.02, s["scope"])
    w.points.update(grip_l=Vector((0, rec_end + 0.22, BORE - 0.03)),
                    butt=Vector((0, butt_y, BORE - 0.02)),
                    muzzle=Vector((0, muzzle_y, BORE)),
                    ground=Vector((0, butt_y, BORE - 0.07)))


def build_smg(w):
    s, b = w.spec, w.mesh
    pull, length, barrel = s["pull"], s["length"], s["barrel"]
    butt_y = -pull
    muzzle_y = length - pull
    rec_end = muzzle_y - barrel
    if s.get("folding_stock"):
        # Folding steel stock: two struts and a butt plate.
        for z in (BORE + 0.01, BORE - 0.05):
            b.box((0, butt_y * 0.5, z), (0.01, -butt_y, 0.012), "Metal", W)
        b.box((0, butt_y, BORE - 0.02), (0.03, 0.012, 0.09), "Metal", W)
        _pistol_grip(b, "Metal")
    else:
        _stock(b, butt_y)
        if not s.get("wood_stock"):
            _pistol_grip(b)
    b.box((0, rec_end * 0.5, BORE), (0.034, rec_end + 0.02, 0.045), "Metal", W)
    if s.get("wood_stock") and s["kind"] == "smg":
        b.box((0, rec_end * 0.4, BORE - 0.03), (0.04, rec_end * 0.8, 0.035), "Wood", W)
    _barrel(b, rec_end, muzzle_y, r=0.008, jacket=s.get("jacket", False))
    if "mag" in s:
        y, mlen = s["mag"]
        b.box((0, y, BORE - 0.02 - mlen / 2), (0.022, 0.035, mlen), "Metal", W, rot=_rx(-4))
    if "drum" in s:
        d = s["drum"]
        b.cylinder((-0.022, 0.08, BORE - 0.07), (0.022, 0.08, BORE - 0.07), d, "Metal", W, sides=10)
    if s.get("thompson_grip") or s.get("wood_stock"):
        _pistol_grip(b)
    w.points.update(grip_l=Vector((0, rec_end * 0.55 + 0.03, BORE - 0.03)),
                    butt=Vector((0, butt_y, BORE - 0.02)),
                    muzzle=Vector((0, muzzle_y, BORE)),
                    ground=Vector((0, butt_y, BORE - 0.07)))


def build_mg(w):
    s, b = w.spec, w.mesh
    pull, length, barrel = s["pull"], s["length"], s["barrel"]
    butt_y = -pull
    muzzle_y = length - pull
    rec_end = muzzle_y - barrel
    if pull > 0.15:
        _stock(b, butt_y, "Wood" if s.get("wood_stock") else "Metal")
    else:
        # Spade grip of a tripod gun.
        b.box((0, butt_y, BORE - 0.01), (0.05, 0.03, 0.06), "Metal", W)
    _pistol_grip(b, "Metal")
    b.box((0, rec_end * 0.5, BORE), (0.05, rec_end + 0.02, 0.065), "Metal", W)
    if s.get("wood_stock") and "pan" in s:
        b.box((0, rec_end + 0.08, BORE - 0.02), (0.045, 0.16, 0.04), "Wood", W)
    _barrel(b, rec_end, muzzle_y, r=0.011, jacket=s.get("jacket", False))
    if "mag" in s:
        y, mlen = s["mag"]
        b.box((0, y, BORE - 0.03 - mlen / 2), (0.03, 0.06, mlen), "Metal", W)
    if "pan" in s:
        d = s["pan"]
        b.cylinder((0, 0.10, BORE + 0.035), (0, 0.10, BORE + 0.06), d / 2, "Metal", W, sides=12)
    if s.get("belt"):
        b.box((-0.05, 0.03, BORE - 0.02), (0.06, 0.05, 0.03), "Metal", W)
    if "water_jacket" in s:
        # water-cooled barrel jacket (Maxim, M1917A1) over most of the barrel
        b.cylinder((0, rec_end, BORE), (0, muzzle_y - 0.05, BORE), s["water_jacket"], "Metal", W, sides=10)
    ground = Vector((0, butt_y, BORE - 0.07))
    if "bipod" in s:
        ground = _bipod(b, s["bipod"] - pull)
    if s.get("tripod") == "heavy":
        # heavy tripod (Lafette 42, M1917A1): long front leg, two rear legs, cradle
        h = s.get("tripod_height", 0.42)
        pivot = Vector((0, 0.12, BORE - 0.06))
        b.box(pivot, (0.07, 0.22, 0.06), "Metal", W)
        b.cylinder(pivot, pivot + Vector((0, 0.70, -h)), 0.014, "Metal", W, sides=4)
        for sx in (-1, 1):
            b.cylinder(pivot, pivot + Vector((sx * 0.45, -0.55, -h)), 0.014, "Metal", W, sides=4)
        ground = pivot + Vector((0, 0.0, -h))
    elif s.get("sokolov"):
        # Sokolov wheeled mount with shield: two small wheels, trail, shield
        h = 0.36
        pivot = Vector((0, 0.10, BORE - 0.06))
        b.box(pivot + Vector((0, 0, -h * 0.5)), (0.05, 0.06, h), "Metal", W)
        axle = pivot + Vector((0, 0.05, -h + 0.13))
        for sx in (-1, 1):
            b.cylinder(axle + Vector((sx * 0.24, 0, 0)), axle + Vector((sx * 0.28, 0, 0)), 0.13, "Metal", W,
                       sides=10)
        b.cylinder(axle + Vector((-0.24, 0, 0)), axle + Vector((0.24, 0, 0)), 0.012, "Metal", W, sides=4)
        b.box(pivot + Vector((0, -0.45, -h + 0.08)), (0.05, 0.75, 0.04), "Metal", W)
        sw, sh = s["shield"]
        b.box(pivot + Vector((0, 0.30, 0.05)), (sw, 0.006, sh), "Paint", W)
        ground = axle + Vector((0, 0, -0.13))
    elif s.get("tripod"):
        # Low tripod M2: front leg and two rear legs.
        pivot = Vector((0, 0.10, BORE - 0.05))
        b.box(pivot, (0.05, 0.08, 0.05), "Metal", W)
        b.cylinder(pivot, pivot + Vector((0, 0.45, -0.30)), 0.012, "Metal", W, sides=4)
        for sx in (-1, 1):
            b.cylinder(pivot, pivot + Vector((sx * 0.38, -0.40, -0.30)), 0.012, "Metal", W, sides=4)
        ground = pivot + Vector((0, 0.0, -0.30))
    grip_l = (Vector((0, butt_y + 0.12, BORE - 0.05)) if pull > 0.15
              else Vector((-0.06, butt_y + 0.02, BORE)))
    w.points.update(grip_l=grip_l, butt=Vector((0, butt_y, BORE - 0.02)),
                    muzzle=Vector((0, muzzle_y, BORE)), ground=ground,
                    fore=Vector((0, rec_end + 0.12, BORE - 0.02)))


def build_atr(w):
    # PTRD-41 anti-tank rifle: long barrel with muzzle brake, padded stock, bipod.
    s, b = w.spec, w.mesh
    pull, length, barrel = s["pull"], s["length"], s["barrel"]
    butt_y = -pull
    muzzle_y = length - pull
    rec_end = muzzle_y - barrel
    _stock(b, butt_y, "Metal")
    b.box((0, butt_y + 0.02, BORE - 0.02), (0.06, 0.04, 0.13), "Wood", W)
    b.cylinder((0, -0.05, BORE), (0, rec_end, BORE), 0.03, "Metal", W, sides=8)
    _pistol_grip(b, "Wood")
    b.cylinder((0, rec_end, BORE), (0, muzzle_y - 0.08, BORE), 0.018, "Metal", W, sides=8,
               r1=0.014)
    b.cylinder((0, muzzle_y - 0.08, BORE), (0, muzzle_y, BORE), 0.028, "Metal", W, sides=8)
    ground = _bipod(b, s["bipod"] - pull, 0.30)
    w.points.update(grip_l=Vector((0, rec_end - 0.15, BORE - 0.03)),
                    butt=Vector((0, butt_y, BORE - 0.02)),
                    muzzle=Vector((0, muzzle_y, BORE)), ground=ground)


def build_launcher(w):
    s, b = w.spec, w.mesh
    length, cal, rear = s["length"], s["caliber"], s["rear"]
    r = cal / 2 + 0.004
    axis_z = 0.10  # tube axis above the grip
    y0, y1 = -rear, length - rear
    b.cylinder((0, y0, axis_z), (0, y1, axis_z), r, "Paint", W, sides=10)
    b.cylinder((0, y0, axis_z), (0, y0 + 0.08, axis_z), r + 0.012, "Metal", W, sides=10)
    b.cylinder((0, y1 - 0.05, axis_z), (0, y1, axis_z), r + 0.006, "Metal", W, sides=10)
    _pistol_grip(b, "Wood")
    b.box((0, 0.0, axis_z * 0.55), (0.02, 0.03, axis_z * 0.9), "Metal", W)
    # Shoulder rest and front grip.
    b.box((0, -0.18, axis_z - r - 0.02), (0.05, 0.12, 0.03), "Wood", W)
    grip_l = Vector((0, 0.33, axis_z - r - 0.06))
    b.box(grip_l + Vector((0, 0, 0.03)), (0.03, 0.04, 0.08), "Wood", W)
    if s.get("shield"):
        b.box((-0.06, 0.12, axis_z + 0.02), (0.36, 0.008, 0.38), "Paint", W)
    w.points.update(grip_l=grip_l, butt=Vector((0, -0.10, axis_z - 0.02)),
                    muzzle=Vector((0, y1, axis_z)), ground=Vector((0, y0, axis_z - r)))


def build_mortar(w):
    """Light mortar, assembled. Weapon space: origin at the carry point (tube middle),
    +Y along the tube towards the muzzle. `ground` is the baseplate center."""
    s, b = w.spec, w.mesh
    lb, cal = s["barrel"], s["caliber"]
    r = cal / 2 + 0.006
    y0, y1 = -lb * 0.5, lb * 0.5
    b.cylinder((0, y0, 0), (0, y1, 0), r, "Paint", W, sides=10)
    b.cylinder((0, y1 - 0.03, 0), (0, y1, 0), r + 0.006, "Metal", W, sides=10)
    # Deployed elevation; baseplate and bipod feet then stand on the ground (Z = 0).
    theta = math.radians(MORTAR_ELEVATION)
    axis_y = Vector((0, math.cos(theta), math.sin(theta)))
    axis_z = Vector((0, -math.sin(theta), math.cos(theta)))
    bw, bd = s["base"]
    b.box((0, y0 - 0.02, 0), (bw, bd, 0.025), "Metal", W, rot=_rx(-MORTAR_ELEVATION))
    h = lb * 0.72
    top = Vector((0, y0 + h, 0))
    top_world = axis_y * h
    for sx in (-1, 1):
        d = Vector((sx * 0.20, top_world.y + 0.18, 0.0)) - top_world
        foot = top + Vector((d.x, d.dot(axis_y), d.dot(axis_z)))
        b.cylinder(top, foot, 0.009, "Metal", W, sides=4)
    b.box(top + Vector((0, 0, -0.03)), (0.07, 0.03, 0.05), "Metal", W)
    w.points.update(grip_l=Vector((0, y0 + 0.12, -r)), butt=Vector((0, y0, 0)),
                    muzzle=Vector((0, y1, 0)), ground=Vector((0, y0 - 0.035, 0)))


def build_flame(w):
    s, b = w.spec, w.mesh
    length = s["length"]
    y0, y1 = -0.15, length - 0.15
    b.cylinder((0, y0, BORE), (0, y1, BORE), 0.022, "Paint", W, sides=8)
    b.cylinder((0, y1 - 0.08, BORE), (0, y1, BORE), 0.03, "Metal", W, sides=8)
    _pistol_grip(b, "Wood")
    grip_l = Vector((0, 0.32, BORE - 0.05))
    b.box(grip_l + Vector((0, 0, 0.02)), (0.03, 0.04, 0.08), "Wood", W)
    if s.get("rifle_like"):
        _stock(b, -0.28)
    # Hose stub towards the back pack.
    b.cylinder((0, y0, BORE - 0.01), (0.03, y0 - 0.12, BORE - 0.12), 0.016, "Paint", W, sides=6)
    w.points.update(grip_l=grip_l, butt=Vector((0, y0, BORE - 0.02)),
                    muzzle=Vector((0, y1, BORE)), ground=Vector((0, y0, BORE - 0.07)))


def build_pistol(w):
    """Self-loading pistol (prompt 042): grip at the origin, slide above it.
    Overall and barrel length are real values; slide height and grip size are
    modeling estimates (about 3 x 11 cm grip, 3 cm slide)."""
    s, b = w.spec, w.mesh
    length, barrel = s["length"], s["barrel"]
    bore = 0.032
    back = -0.035
    muzzle_y = back + length
    # Frame and grip (raked back like every pistol of the three).
    b.box((0, -0.012, -0.035), (0.028, 0.034, 0.105), s.get("grip", "Wood"), W, rot=_rx(-14))
    b.box((0, (back + muzzle_y - barrel * 0.4) * 0.5, bore - 0.022),
          (0.024, muzzle_y - barrel * 0.4 - back, 0.018), "Metal", W)
    # Slide over the barrel, trigger guard below it.
    b.box((0, (back + muzzle_y) * 0.5, bore), (0.026, length, 0.03), "Metal", W)
    b.box((0, 0.035, bore - 0.04), (0.008, 0.045, 0.02), "Metal", W)
    w.points.update(grip_l=Vector((-0.012, -0.01, -0.045)),
                    butt=Vector((0, -0.03, -0.085)),
                    muzzle=Vector((0, muzzle_y, bore)),
                    ground=Vector((0, -0.03, -0.085)))


BUILDERS = {
    "rifle": build_rifle, "smg": build_smg, "mg": build_mg, "atr": build_atr,
    "launcher": build_launcher, "mortar": build_mortar, "flame": build_flame,
    "pistol": build_pistol,
}


def build_weapon(wid):
    w = Weapon(wid)
    BUILDERS[w.kind](w)
    p = w.points
    p.setdefault("fore", p["grip_l"])
    p["center"] = (p["butt"] + p["muzzle"]) * 0.5
    return w


# --- prompt 008: weapons for the tier-2 infantry teams ----------------------------------
# Dimensions from the cited articles; mount dimensions (tripod legs, Sokolov wheels and
# shield, baseplates) are modeling estimates from photographs and marked uncertain.
WEAPONS.update({
    "mg42_lafette": dict(kind="mg", length=1.220, barrel=0.533, pull=0.30, jacket=True, belt=True,
                         tripod="heavy", tripod_height=0.42,
                         source=[WIKI + "MG_42", WIKI + "Lafette_34"], uncertain=["tripod_height"]),
    "maxim": dict(kind="mg", length=1.107, barrel=0.721, pull=0.10, water_jacket=0.055, belt=True,
                  sokolov=True, shield=(0.42, 0.38),
                  source=[WIKI + "PM_M1910"], uncertain=["shield"]),
    "m1917a1": dict(kind="mg", length=0.981, barrel=0.610, pull=0.10, water_jacket=0.048, belt=True,
                    tripod="heavy", tripod_height=0.40,
                    source=[WIKI + "M1917_Browning_machine_gun"], uncertain=["tripod_height"]),
    "m1_carbine": dict(kind="rifle", length=0.904, barrel=0.458, pull=0.33, full_stock=True,
                       source=[WIKI + "M1_carbine"]),
    "mosin_m38": dict(kind="rifle", length=1.016, barrel=0.514, pull=0.33, full_stock=True,
                      source=[WIKI + "Mosin–Nagant"]),
    "sgrw34": dict(kind="mortar", barrel=1.143, caliber=0.081, base=(0.47, 0.40),
                   source=[WIKI + "8_cm_Granatwerfer_34"], uncertain=["base"]),
    "bm37": dict(kind="mortar", barrel=1.220, caliber=0.082, base=(0.50, 0.50),
                 source=[WIKI + "82-BM-37"], uncertain=["barrel", "base"]),
    "m1_81mm": dict(kind="mortar", barrel=1.257, caliber=0.081, base=(0.50, 0.42),
                    source=[WIKI + "M1_mortar"], uncertain=["base"]),
})


# --- prompt 042: pistols ----------------------------------------------------------------
# Overall and barrel length from the cited articles (infobox values).
WEAPONS.update({
    "p38": dict(kind="pistol", length=0.216, barrel=0.125, grip="Metal",
                source=[WIKI + "Walther_P38"]),
    "tt33": dict(kind="pistol", length=0.195, barrel=0.116, grip="Metal",
                 source=[WIKI + "TT_pistol"]),
    "m1911a1": dict(kind="pistol", length=0.216, barrel=0.127, grip="Wood",
                    source=[WIKI + "M1911_pistol"]),
})


# --- prompt 117: uk (Great Britain 1944) ------------------------------------------------
# Overall and barrel lengths from the cited articles (fetched for prompt 117);
# mounts, baseplates and the PIAT's shape are modeling estimates (uncertain). The PIAT
# (a spigot mortar with a box-like body) is drawn as a short launcher tube.
WEAPONS.update({
    "lee_enfield_no4": dict(kind="rifle", length=1.129, barrel=0.640, pull=0.33, full_stock=True,
                            source=[WIKI + "Lee–Enfield"]),
    "lee_enfield_no4t": dict(kind="rifle", length=1.129, barrel=0.640, pull=0.33, full_stock=True,
                             scope=0.28, source=[WIKI + "Lee–Enfield"], uncertain=["scope"]),
    "sten": dict(kind="smg", length=0.762, barrel=0.196, pull=0.26, side_mag=(0.07, 0.24),
                 folding_stock=True, source=[WIKI + "Sten"]),
    "sten_mk5": dict(kind="smg", length=0.762, barrel=0.196, pull=0.26, side_mag=(0.07, 0.24),
                     wood_stock=True, source=[WIKI + "Sten"], uncertain=["length", "barrel"]),
    "bren": dict(kind="mg", length=1.160, barrel=0.635, pull=0.32, bipod=0.82, top_mag=(0.06, 0.20),
                 wood_stock=True, source=[WIKI + "Bren_light_machine_gun"]),
    "vickers": dict(kind="mg", length=1.120, barrel=0.710, pull=0.10, water_jacket=0.055, belt=True,
                    tripod="heavy", tripod_height=0.44,
                    source=[WIKI + "Vickers_machine_gun"], uncertain=["tripod_height"]),
    "piat": dict(kind="launcher", length=0.990, caliber=0.083, rear=0.40,
                 source=[WIKI + "PIAT"], uncertain=["caliber", "rear"]),
    "mortar_2in": dict(kind="mortar", barrel=0.530, caliber=0.051, base=(0.16, 0.12),
                       source=[WIKI + "Two-inch_mortar"], uncertain=["barrel", "base"]),
    "mortar_3in": dict(kind="mortar", barrel=1.190, caliber=0.081, base=(0.42, 0.42),
                       source=[WIKI + "ML_3-inch_mortar"], uncertain=["base"]),
    "lifebuoy": dict(kind="flame", length=0.75, source=[WIKI + "Flamethrower,_Portable,_No_2"],
                     uncertain=["length"]),
    "enfield_no2": dict(kind="pistol", length=0.260, barrel=0.127, grip="Wood",
                        source=[WIKI + "Enfield_No._2"]),
})


def _uk_magazines(w):
    """Side magazine (Sten: horizontal to the left) and top magazine (Bren: curved,
    standing up and leaning forward) of the prompt 117 weapons."""
    s, b = w.spec, w.mesh
    if "side_mag" in s:
        y, mlen = s["side_mag"]
        b.box((-0.02 - mlen / 2, y, BORE), (mlen, 0.035, 0.022), "Metal", W)
    if "top_mag" in s:
        y, mlen = s["top_mag"]
        b.box((0, y, BORE + 0.03 + mlen / 2), (0.03, 0.07, mlen), "Metal", W, rot=_rx(18))


def _with_uk_magazines(builder):
    def build(w):
        builder(w)
        _uk_magazines(w)
    build.__doc__ = builder.__doc__
    return build


BUILDERS["smg"] = _with_uk_magazines(build_smg)
BUILDERS["mg"] = _with_uk_magazines(build_mg)
