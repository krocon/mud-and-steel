"""Headgear and equipment, rigidly skinned to one bone each ("attachments at bones").

All parts of one soldier go into a single skinned `Gear` mesh (one draw call per
material). Positions fit the base body (body.py); the stature shape keys move gear
together with the body.

Headgear: Stahlhelm M35/M40, Feldmütze M43 (de); SSh-40, pilotka (su); M1 helmet (us).
Equipment: belt, ammunition pouches, bread bag, canteen, entrenching tool, gas mask
canister/bag, packs, radio, medical bags, flamethrower tanks. No Red Cross anywhere
(docs/SOURCES.md): medics get a plain white armband (`Insignia`) and bags.
"""

import math

from mathutils import Matrix, Vector

from .body import HEAD_CENTER
from .mesh import MeshBuilder

HEAD = {"Head": 1.0}
HIPS = {"Hips": 1.0}
WAIST = {"Hips": 0.5, "Spine": 0.5}
CHEST = {"Chest": 1.0}
UPPER = {"UpperChest": 1.0}


def _rx(deg):
    return Matrix.Rotation(math.radians(deg), 3, "X")


def _ry(deg):
    return Matrix.Rotation(math.radians(deg), 3, "Y")


def _rz(deg):
    return Matrix.Rotation(math.radians(deg), 3, "Z")


# -- headgear --------------------------------------------------------------------------


def _shell(b, rings, slot, sides=12, scale=(1.0, 1.0), top=1.84, y0=0.0):
    """Helmet shell: rings of (radius, z, front_lift). front_lift raises the ring at
    the front (+Y), e.g. the visor cut-out of the Stahlhelm."""
    c = HEAD_CENTER + Vector((0, y0, 0))
    idx = []
    for r, z, lift in rings:
        ring = []
        for s in range(sides):
            a = 2.0 * math.pi * s / sides
            front = max(0.0, math.sin(a)) ** 2
            co = Vector((c.x + math.cos(a) * r * scale[0], c.y + math.sin(a) * r * scale[1],
                         z + lift * front))
            ring.append(b.vert(co, HEAD))
        idx.append(ring)
    for i in range(len(idx) - 1):
        for s in range(sides):
            t = (s + 1) % sides
            b.face((idx[i][s], idx[i][t], idx[i + 1][t], idx[i + 1][s]), slot)
    apex = b.vert((c.x, c.y, top), HEAD)
    for s in range(sides):
        t = (s + 1) % sides
        b.face((idx[-1][s], idx[-1][t], apex), slot)
    # Underside of the brim/skirt so it is not see-through from below.
    inner = []
    r0, z0, lift0 = rings[0]
    for s in range(sides):
        a = 2.0 * math.pi * s / sides
        front = max(0.0, math.sin(a)) ** 2
        co = Vector((c.x + math.cos(a) * 0.088, c.y + math.sin(a) * 0.104, z0 + 0.012 + lift0 * front))
        inner.append(b.vert(co, HEAD))
    for s in range(sides):
        t = (s + 1) % sides
        b.face((idx[0][t], idx[0][s], inner[s], inner[t]), slot)


def helmet_m35(b):
    _shell(b, [(0.152, 1.645, 0.055), (0.130, 1.672, 0.048), (0.120, 1.705, 0.022),
               (0.119, 1.745, 0.0), (0.106, 1.790, 0.0), (0.070, 1.826, 0.0)],
           "Paint", scale=(0.9, 1.0), top=1.838, y0=-0.005)


def helmet_ssh40(b):
    _shell(b, [(0.136, 1.672, 0.012), (0.124, 1.686, 0.012), (0.120, 1.725, 0.0),
               (0.112, 1.775, 0.0), (0.082, 1.818, 0.0)],
           "Paint", scale=(0.9, 1.0), top=1.836)


def helmet_m1(b):
    _shell(b, [(0.152, 1.690, 0.0), (0.124, 1.696, 0.0), (0.121, 1.732, 0.0),
               (0.111, 1.780, 0.0), (0.080, 1.822, 0.0)],
           "Paint", scale=(0.9, 1.04), top=1.836)


def cap_m43(b):
    _shell(b, [(0.104, 1.715, 0.0), (0.103, 1.770, 0.0), (0.098, 1.800, 0.0),
               (0.090, 1.812, 0.0)],
           "Cloth", sides=10, scale=(0.9, 1.0), top=1.814)
    # Short visor.
    b.box(HEAD_CENTER + Vector((0, 0.105, 0.035)), (0.13, 0.06, 0.008), "Cloth", HEAD,
          rot=_rx(-12))


def pilotka(b):
    _shell(b, [(0.102, 1.728, 0.0), (0.096, 1.772, 0.0), (0.062, 1.806, 0.0)],
           "Cloth", sides=10, scale=(0.75, 1.08), top=1.812)


HEADGEAR = {
    "m35": helmet_m35,
    "m43_cap": cap_m43,
    "ssh40": helmet_ssh40,
    "pilotka": pilotka,
    "m1": helmet_m1,
}


# -- equipment ---------------------------------------------------------------------------


def belt(b, slot, height=0.045, thick=0.012):
    rings = [((0, -0.005, 1.0 - height / 2), 0.172 + thick, 0.120 + thick, WAIST),
             ((0, -0.005, 1.0 + height / 2), 0.168 + thick, 0.117 + thick, WAIST)]
    b.tube(rings, slot, sides=12, cap_start=False, cap_end=False)


def y_straps(b, slot):
    for sx in (-1, 1):
        # Front strap from belt to shoulder, back strap from shoulder to belt.
        b.box((sx * 0.085, 0.125, 1.21), (0.03, 0.01, 0.40), slot, CHEST, rot=_ry(-sx * 4) @ _rx(-6))
        b.box((sx * 0.06, -0.122, 1.21), (0.03, 0.01, 0.40), slot, CHEST, rot=_ry(sx * 8) @ _rx(4))
        b.box((sx * 0.11, 0.0, 1.462), (0.03, 0.24, 0.01), slot, UPPER)


def pouches_de(b):
    # Two triple cartridge pouches (Patronentaschen 33) left and right of the buckle.
    for sx in (-1, 1):
        for i in range(3):
            x = sx * (0.04 + i * 0.033)
            b.box((x, 0.132, 0.99), (0.03, 0.035, 0.07), "Leather", HIPS)


def stick_grenade(b):
    b.cylinder((-0.03, 0.128, 0.93), (-0.03, 0.14, 1.12), 0.012, "Wood", WAIST, sides=6)
    b.cylinder((-0.03, 0.14, 1.12), (-0.03, 0.142, 1.19), 0.032, "Paint", CHEST, sides=8)


def bread_bag(b, slot="Canvas", side=1):
    b.box((side * 0.15, -0.075, 0.92), (0.05, 0.17, 0.17), slot, HIPS, taper=0.9)


def canteen(b, slot="Paint", side=1, cover="Canvas"):
    b.cylinder((side * 0.175, -0.12, 0.84), (side * 0.175, -0.12, 0.99), 0.045, cover, HIPS,
               sides=8)
    b.cylinder((side * 0.175, -0.12, 0.99), (side * 0.175, -0.12, 1.01), 0.016, "Metal", HIPS,
               sides=6)


def shovel_side(b, side=-1):
    # Folding/short entrenching tool on the hip, blade down in a case.
    b.box((side * 0.165, -0.06, 0.80), (0.035, 0.15, 0.20), "Leather", HIPS)
    b.cylinder((side * 0.165, -0.06, 0.90), (side * 0.165, -0.06, 1.06), 0.014, "Wood", HIPS,
               sides=6)


def gasmask_can(b):
    # Gasmaskenbüchse: fluted canister on the back, slanted.
    rot = _ry(-20)
    p0 = Vector((-0.05, -0.17, 0.82))
    p1 = p0 + rot @ Vector((0, 0, 0.26))
    b.cylinder(p0, p1, 0.058, "Paint", HIPS, sides=10)


def gasmask_bag(b, side=-1):
    b.box((side * 0.17, 0.02, 0.88), (0.06, 0.18, 0.2), "Canvas", HIPS, taper=0.9)


def bayonet_frog(b, side=-1):
    b.box((side * 0.13, -0.09, 0.87), (0.02, 0.035, 0.30), "Metal", HIPS, rot=_rx(8))


def sturmgepaeck(b):
    # Assault pack frame with mess tin and rolled shelter quarter.
    b.box((0, -0.15, 1.22), (0.20, 0.03, 0.22), "Leather", CHEST)
    b.box((0, -0.18, 1.25), (0.15, 0.05, 0.12), "Paint", CHEST)
    b.cylinder((-0.12, -0.165, 1.12), (0.12, -0.165, 1.12), 0.035, "Canvas", CHEST, sides=8)


def veshmeshok(b):
    # Soviet duffel rucksack, sack shape.
    b.ellipsoid((0, -0.20, 1.22), (0.14, 0.08, 0.15), "Canvas", CHEST, rings=5, sides=8)


def skatka(b):
    # Rolled greatcoat over the left shoulder to the right hip.
    n = 12
    rings = []
    tilt = _ry(-34)
    for i in range(n + 1):
        a = 2.0 * math.pi * i / n
        p = Vector((math.cos(a) * 0.30, math.sin(a) * 0.17, 0.0))
        p = tilt @ p + Vector((0.0, -0.005, 1.20))
        w = UPPER if p.z > 1.32 else (CHEST if p.z > 1.08 else HIPS)
        rings.append((p, 0.04, 0.04, w))
    rings = rings[:-1]
    # Close the loop by repeating the first ring.
    rings.append(rings[0])
    b.tube(rings, "Canvas", sides=6, cap_start=False, cap_end=False)


def ammo_pouch_su(b):
    for sx in (-1, 1):
        b.box((sx * 0.08, 0.13, 0.99), (0.07, 0.04, 0.075), "Leather", HIPS)


def us_cartridge_belt(b):
    belt(b, "Canvas", height=0.065, thick=0.014)
    for i in range(5):
        for sx in (-1, 1):
            a = math.radians(70 - i * 22)
            x = sx * math.cos(a) * 0.19
            y = math.sin(a) * 0.135
            rot = _rz(sx * (90 - math.degrees(a)) * -1)
            b.box((x, y, 0.985), (0.055, 0.03, 0.07), "Canvas", HIPS, rot=rot)
    # Suspenders.
    y_straps(b, "Canvas")


def us_haversack(b):
    b.box((0, -0.16, 1.22), (0.22, 0.07, 0.30), "Canvas", CHEST, taper=0.95)
    # Entrenching tool M1943 strapped below/beside the haversack.
    b.box((0.0, -0.20, 1.02), (0.14, 0.03, 0.16), "Canvas", HIPS)
    b.cylinder((0, -0.20, 1.10), (0, -0.20, 1.34), 0.014, "Wood", CHEST, sides=6)


def first_aid_pouch_us(b):
    b.box((0.05, 0.14, 0.95), (0.08, 0.035, 0.05), "Canvas", HIPS)


def map_case(b):
    b.box((-0.16, 0.06, 0.90), (0.03, 0.18, 0.13), "Leather", HIPS)


def binoculars(b):
    for sx in (-1, 1):
        b.cylinder((sx * 0.03, 0.135, 1.27), (sx * 0.03, 0.135, 1.37), 0.026, "Paint", CHEST,
                   sides=8)


def pistol_holster(b, side=-1):
    b.box((side * 0.135, 0.09, 0.95), (0.04, 0.11, 0.12), "Leather", HIPS, rot=_rz(side * 30))


def medic_bags(b):
    # Two plain medical pouches at the front of the belt, no markings.
    for sx in (-1, 1):
        b.box((sx * 0.09, 0.14, 0.96), (0.11, 0.05, 0.11), "Leather", HIPS)


def armband(b):
    # Plain white armband on the left upper arm (no Red Cross, docs/SOURCES.md).
    from .skeleton import BONES
    head, tail = Vector(BONES["LeftUpperArm"][1]), Vector(BONES["LeftUpperArm"][2])
    d = (tail - head)
    c0 = head + d * 0.40
    c1 = head + d * 0.62
    b.tube([(c0, 0.062, 0.064, {"LeftUpperArm": 1.0}), (c1, 0.058, 0.060, {"LeftUpperArm": 1.0})],
           "Insignia", sides=7, cap_start=False, cap_end=False)


def radio(b, faction):
    if faction == "us":
        size, z = (0.33, 0.18, 0.44), 1.20  # SCR-300
    elif faction == "su":
        size, z = (0.30, 0.16, 0.30), 1.24  # RB-M transceiver
    else:
        size, z = (0.30, 0.17, 0.38), 1.22  # Feldfunksprecher / Torn.Fu
    b.box((0, -0.12 - size[1] / 2, z), size, "Paint", CHEST)
    # Antenna (whip, slanted back).
    top = Vector((0.11, -0.25, z + size[2] / 2))
    b.cylinder(top, top + Vector((0.0, -0.12, 0.85)), 0.006, "Metal", CHEST, sides=4, r1=0.004)


def flame_tanks(b, faction):
    rot = Matrix.Identity(3)
    if faction == "de":
        # Flammenwerfer 41: fuel tank and smaller pressure tank side by side.
        b.cylinder((-0.07, -0.20, 0.92), (-0.07, -0.20, 1.42), 0.085, "Paint", CHEST, sides=10)
        b.cylinder((0.10, -0.19, 0.98), (0.10, -0.19, 1.36), 0.055, "Paint", CHEST, sides=8)
    elif faction == "su":
        # ROKS-3: one flat fuel tank and a small gas cylinder.
        b.box((0, -0.20, 1.18), (0.30, 0.12, 0.44), "Paint", CHEST, rot=rot)
        b.cylinder((0.0, -0.28, 1.00), (0.0, -0.28, 1.34), 0.04, "Metal", CHEST, sides=8)
    else:
        # M2-2: two fuel tanks with the pressure sphere between them.
        for sx in (-1, 1):
            b.cylinder((sx * 0.10, -0.20, 0.94), (sx * 0.10, -0.20, 1.44), 0.08, "Paint", CHEST,
                       sides=10)
        b.ellipsoid((0, -0.27, 1.20), (0.06, 0.06, 0.06), "Metal", CHEST, rings=4, sides=8)


def mg_pouch(b):
    b.box((0.13, 0.08, 0.95), (0.03, 0.10, 0.07), "Leather", HIPS, rot=_rz(30))


def pan_carrier(b):
    # DP-27 pan magazine carrier on the back.
    b.box((0, -0.17, 1.20), (0.26, 0.08, 0.26), "Canvas", CHEST)


def rocket_pack(b):
    b.box((0, -0.17, 1.18), (0.18, 0.12, 0.40), "Wood", CHEST)


ITEMS = {
    "belt_leather": lambda b, f: belt(b, "Leather"),
    "y_straps": lambda b, f: y_straps(b, "Leather"),
    "pouches_de": lambda b, f: pouches_de(b),
    "stick_grenade": lambda b, f: stick_grenade(b),
    "bread_bag": lambda b, f: bread_bag(b),
    "canteen": lambda b, f: canteen(b),
    "shovel_side": lambda b, f: shovel_side(b),
    "gasmask_can": lambda b, f: gasmask_can(b),
    "gasmask_bag": lambda b, f: gasmask_bag(b),
    "bayonet": lambda b, f: bayonet_frog(b),
    "sturmgepaeck": lambda b, f: sturmgepaeck(b),
    "veshmeshok": lambda b, f: veshmeshok(b),
    "skatka": lambda b, f: skatka(b),
    "ammo_pouch_su": lambda b, f: ammo_pouch_su(b),
    "us_cartridge_belt": lambda b, f: us_cartridge_belt(b),
    "us_haversack": lambda b, f: us_haversack(b),
    "first_aid_us": lambda b, f: first_aid_pouch_us(b),
    "map_case": lambda b, f: map_case(b),
    "binoculars": lambda b, f: binoculars(b),
    "holster": lambda b, f: pistol_holster(b),
    "medic_bags": lambda b, f: medic_bags(b),
    "armband": lambda b, f: armband(b),
    "radio": radio,
    "flame_tanks": flame_tanks,
    "mg_pouch": lambda b, f: mg_pouch(b),
    "pan_carrier": lambda b, f: pan_carrier(b),
    "rocket_pack": lambda b, f: rocket_pack(b),
}


def build_gear(faction, headgear, items):
    b = MeshBuilder(["Paint", "Leather", "Canvas"])
    HEADGEAR[headgear](b)
    for item in items:
        ITEMS[item](b, faction)
    return b


# -- de squad roles (prompt 006) ------------------------------------------------------------
# Sizes are modeling estimates from photographs; exact dimensions are not in the
# catalog sources.


def mg_ammo_box_de(b, x=0.0, y=-0.205, z=1.20):
    """Patronenkasten (300-round belt box for MG 34/42), about 0.28 x 0.19 x 0.10 m,
    carried flat on the back."""
    b.box((x, y, z), (0.19, 0.10, 0.28), "Paint", CHEST)
    b.box((x, y, z + 0.15), (0.12, 0.02, 0.02), "Metal", CHEST)  # handle


def belt_drum_de(b):
    """Gurttrommel 34 (50-round belt drum) on the right front of the belt."""
    b.cylinder((0.07, 0.15, 0.90), (0.145, 0.15, 0.90), 0.055, "Paint", HIPS, sides=10)


def barrel_case_de(b):
    """Laufschützer: spare-barrel case slung diagonally across the back."""
    p0 = Vector((-0.17, -0.27, 0.98))
    p1 = Vector((0.16, -0.27, 1.52))
    b.cylinder(p0, p1, 0.04, "Paint", CHEST, sides=8)


ITEMS["mg_ammo_box_de"] = lambda b, f: mg_ammo_box_de(b)
ITEMS["mg_ammo_boxes_de"] = lambda b, f: (mg_ammo_box_de(b, x=-0.10), mg_ammo_box_de(b, x=0.10))
ITEMS["belt_drum_de"] = lambda b, f: belt_drum_de(b)
ITEMS["barrel_case_de"] = lambda b, f: barrel_case_de(b)


# -- prompt 008: equipment for the tier-2 roles -----------------------------------------
# Sizes are modeling estimates from photographs.


def demo_charge(b, faction):
    """Demolition charge: de 'geballte Ladung' (stick grenade heads bundled round a
    complete grenade) at the left hip; su / us canvas satchel charge on the back."""
    if faction == "de":
        for i in range(5):
            a = i * 2 * math.pi / 5
            c = Vector((-0.16 + math.cos(a) * 0.04, 0.06 + math.sin(a) * 0.04, 0.92))
            b.cylinder(c, c + Vector((0, 0, 0.075)), 0.03, "Paint", HIPS, sides=6)
        b.cylinder((-0.16, 0.06, 0.92), (-0.16, 0.06, 1.05), 0.012, "Wood", HIPS, sides=4)
    else:
        b.box((0.0, -0.19, 1.05), (0.26, 0.10, 0.20), "Canvas", CHEST)
        b.box((0.0, -0.19, 1.17), (0.18, 0.02, 0.03), "Canvas", CHEST)  # carrying strap


def bangalore(b):
    """Bangalore torpedo section (about 1.5 m tube, 54 mm) slung diagonally on the back."""
    b.cylinder((-0.22, -0.24, 0.80), (0.20, -0.24, 1.62), 0.027, "Paint", CHEST, sides=6)


def wire_cutters(b):
    b.box((0.15, 0.04, 0.93), (0.03, 0.05, 0.16), "Metal", HIPS)


def panzerfaust(b):
    """Panzerfaust 60 slung on the back: tube (1.045 m) with the warhead up."""
    p0 = Vector((0.12, -0.22, 0.85))
    p1 = Vector((0.12, -0.22, 1.62))
    b.cylinder(p0, p1, 0.025, "Paint", CHEST, sides=6)
    b.cylinder(p1, p1 + Vector((0, 0, 0.15)), 0.075, "Paint", CHEST, sides=8, r1=0.03)


def ammo_case(b):
    """Ammunition case on the back (mortar bombs, MG belts): wooden or steel box."""
    b.box((0, -0.20, 1.18), (0.26, 0.12, 0.34), "Wood", CHEST)
    b.box((0, -0.20, 1.37), (0.12, 0.02, 0.02), "Metal", CHEST)


ITEMS["demo_charge"] = demo_charge
ITEMS["bangalore"] = lambda b, f: bangalore(b)
ITEMS["wire_cutters"] = lambda b, f: wire_cutters(b)
ITEMS["panzerfaust"] = lambda b, f: panzerfaust(b)
ITEMS["ammo_case"] = lambda b, f: ammo_case(b)
