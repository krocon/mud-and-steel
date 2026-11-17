"""Smoke test of the part kit (`just kit-test`).

Builds demo units that together use every registered part (also those no
real blueprint uses yet: halftrack, wheeled vehicle, aircraft; prompt 008:
casemate vehicles, armored car, AA gun, mortar, bomber, dive bomber, biplane
and the fortification generators) with made-up catalog values, and checks that each builds, has geometry, bakes and stays
inside a sane triangle count. Nothing is saved. Exit code 1 on failure.

    blender -b --factory-startup --python tools/blender/kit_test.py [-- --render DIR]
"""

import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import load_parts  # noqa: E402
from kit.bake import bake  # noqa: E402
from kit.build import clear_scene  # noqa: E402
from kit.core import Model, descendants, tri_count  # noqa: E402
from kit.registry import ASSEMBLIES, PARTS, build_part  # noqa: E402

USED = set()


def demo_halftrack(model):
    """Halftrack: front wheel axle plus interleaved rear track unit."""
    ctx = {"half_width": 1.05, "track_outer_x": 1.0, "hull_length": 5.8, "height": 2.0}
    model.mb("Hull")
    run(model, "halftrack_rear", {
        "track_width": 0.28, "road_wheel_diameter": 0.62, "road_wheels": [-0.2, -0.75, -1.3, -1.85],
        "sprocket": {"y": 0.45, "z": 0.45, "diameter": 0.5}, "idler": {"y": -2.4, "z": 0.4, "diameter": 0.5},
        "interleaved": True}, ctx)
    run(model, "wheel_axle", {"y": 2.0, "diameter": 0.85, "width": 0.18, "track": 1.6, "index": 5}, ctx)
    run(model, "plate_hull", {
        "lower": [[-2.9, 0.4], [2.6, 0.4], [2.9, 0.9], [-2.9, 0.9]],
        "upper": [[-2.9, 0.9], [2.9, 0.9], [2.3, 1.55], [-2.9, 1.7]], "upper_side_slope_deg": 15,
        "fender_z": 1.0}, dict(ctx, track_inner_x=0.72, track_top_z=0.98, half_width=1.05))


def demo_truck(model):
    ctx = {"half_width": 1.1, "hull_length": 6.0}
    model.mb("Hull")
    run(model, "box_prop", {"center": [0, -0.8, 1.6], "size": [2.1, 3.6, 1.0], "material": "Canvas"}, ctx)
    run(model, "box_prop", {"center": [0, 1.6, 1.3], "size": [1.8, 1.4, 1.2]}, ctx)
    run(model, "wheel_axle", {"y": 1.8, "diameter": 0.9, "width": 0.22, "track": 1.6}, ctx)
    run(model, "wheel_axle", {"y": -1.5, "diameter": 0.9, "width": 0.22, "track": 1.6, "index": 2, "twin": True,
                              "spokes": 6}, ctx)
    run(model, "jerrycans", {"origin": [1.0, -2.0, 0.9], "count": 3, "step": [0, 0.17, 0], "upright": False}, ctx)
    run(model, "cylinder_prop", {"from": [-1.0, 0.5, 0.7], "to": [-1.0, 1.0, 0.7], "radius": 0.25}, ctx)


def demo_tank(model):
    """Tank using the parts the four real blueprints do not use."""
    ctx = {"half_width": 1.4, "track_outer_x": 1.4, "hull_length": 6.0, "height": 2.6, "length": 7.0,
           "width": 2.8}
    model.mb("Hull")
    run(model, "vvss_bogie", {
        "track_width": 0.42, "road_wheel_diameter": 0.5, "road_wheels": [1.7, 1.2, 0.25, -0.25, -1.2, -1.7],
        "sprocket": {"y": 2.5, "z": 0.7, "diameter": 0.66}, "idler": {"y": -2.5, "z": 0.45, "diameter": 0.56}},
        ctx)
    run(model, "plate_hull", {
        "lower": [[-2.9, 0.45], [2.4, 0.45], [2.95, 1.0], [-2.95, 1.0]],
        "upper": [[-2.95, 1.0], [2.95, 1.0], [2.0, 1.8], [-2.95, 1.6]], "cast": True}, ctx)
    ctx.update(turret_y=0.2, turret_z=ctx["hull_top_z"], turret_height=0.75, turret_ring_r=0.85)
    run(model, "cast_turret", {"length": 2.0, "width": 1.8}, ctx)
    run(model, "turret_hatch", {"x": 0.3, "y": 0.0, "diameter": 0.5, "socket": True}, ctx)
    run(model, "hull_hatch", {"center": [0.5, 1.6], "size": [0.5, 0.5], "socket": True}, ctx)
    run(model, "deck_box", {"center": [0, -1.5], "size": [1, 1, 0.05]}, ctx)
    ctx["overall_front_y"] = 3.6
    run(model, "tank_gun", {"trunnion": [0.8, 0.35], "mantlet": {"kind": "round", "size": [0.6, 0.3, 0.4]},
                            "segments": [[0.5, 0.06, 0.06], ["fit", 0.055, 0.05]], "muzzle_brake": "slotted"}, ctx)
    run(model, "mg", {"mount": "pintle", "position": [0.3, -0.5, 0.4]}, ctx)
    run(model, "headlight", {"position": [1.0, 2.8, 1.1], "blackout": True}, ctx)
    run(model, "antenna", {"position": [-1.0, -1.5, 1.6]}, ctx)
    run(model, "exhaust", {"position": [0.5, -2.95, 1.0], "kind": "pipe"}, ctx)
    run(model, "tow_hook", {"position": [0, -2.95, 0.6]}, ctx)
    run(model, "socket", {"name": "Socket_Passenger_", "position": [0, -1, 1.7]}, ctx)
    for kind in ("axe", "wire_cutter", "sledgehammer"):
        run(model, "tool", {"kind": kind, "position": [-1.3, 0.0, 1.05]}, ctx)
    for kind in ("single_baffle", "double_baffle", "thread_protector"):
        from kit.weapons import muzzle_brake

        muzzle_brake(model.mb("Hull"), 0.0, 3.0, 0.05, kind, x=2.0)
    for kind in ("box", "cast_box", "rotor"):
        from kit.weapons import mantlet

        mantlet(model.mb("Hull"), kind, 0.0, 3.0, (0.6, 0.2, 0.4), x=-2.0)


def demo_aircraft(model):
    ctx = {"wingspan": 10.5, "height": 3.9}
    model.mb("Hull")
    run(model, "fuselage", {"sections": [[-6.0, 0, 0, 1.6, 2], [-4.5, 0.6, 0.8, 1.5, 2.2],
                                         [0, 1.0, 1.4, 1.5, 2.4], [2.6, 1.1, 1.1, 1.5, 2.2],
                                         [2.9, 0.7, 0.7, 1.5, 2]]}, ctx)
    run(model, "wing", {"y": 1.4, "z": 1.0, "root_chord": 2.3, "tip_chord": 1.2, "sweep_deg": 4}, ctx)
    run(model, "tail", {"y": -4.8, "z": 1.6, "span": 3.6, "root_chord": 1.2, "tip_chord": 0.7,
                        "fin_height": 1.4}, ctx)
    run(model, "propeller", {"position": [0, 3.2, 1.5], "diameter": 3.3, "blades": 3}, ctx)
    run(model, "canopy", {"y": [-0.8, 0.6], "z": 2.1, "width": 0.7, "height": 0.45, "bubble": True}, ctx)
    run(model, "landing_gear", {"main_y": 1.2, "main_track": 3.5, "main_wheel_diameter": 0.7,
                                "strut_top_z": 1.0, "tail_y": -5.6, "tail_wheel_diameter": 0.35}, ctx)


# --- prompt 008: assemblies and parts for the tier-2 units ------------------------
# These run the real assemblies with made-up catalog entries, so the fitting
# logic (casemate height, barrel length, mortar elevation) is tested too.

def fake_unit(uid, category, **dims):
    u = {"id": uid, "category": category, "crew": 4, "weapon": []}
    u.update(dims)
    return u


def run_assembly(model, name, bp, u):
    for key, val in bp.items():
        for t in (val if isinstance(val, list) else [val]):
            if isinstance(t, dict) and "part" in t:
                USED.add(t["part"])
    if "cupola" in bp:
        USED.add("cupola")
    if name in ("armored_car", "aa_gun", "mortar") and bp.get("axle"):
        USED.add("wheel_axle")
    ASSEMBLIES[name](model, bp, u, {})


MARK = {"part": "marking", "name": "Hull_L", "position": [-1.3, 0.0, 1.2], "normal": [-1, 0, 0], "kind": "national"}


def demo_casemate(model):
    """Assault gun: torsion-bar running gear, closed casemate, off-center gun."""
    u = fake_unit("kit_test_casemate", "assault_gun", length_mm=6800, length_hull_mm=5500, width_mm=2950,
                  height_mm=2160, weapon=[{"name": "MG", "caliber_mm_x10": 79}])
    bp = {
        "assembly": "casemate",
        "layout": {"fender_overhang": 0.05},
        "running_gear": {"part": "torsion_bar", "track_width": 0.40, "road_wheel_diameter": 0.52,
                         "road_wheels": [1.55, 0.95, 0.35, -0.25, -0.85, -1.45],
                         "sprocket": {"y": 2.45, "z": 0.70, "diameter": 0.62},
                         "idler": {"y": -2.55, "z": 0.62, "diameter": 0.56},
                         "return_rollers": {"y": [1.2, 0.0, -1.2], "z": 0.88, "diameter": 0.22}},
        "hull": {"part": "plate_hull", "lower": [[-2.7, 0.42], [2.2, 0.42], [2.75, 0.85], [2.75, 1.05], [-2.75, 1.05]],
                 "upper": [[-2.75, 1.04], [2.75, 1.04], [2.3, 1.30], [-2.75, 1.30]], "upper_half_width": 1.3},
        "casemate": {"part": "casemate", "plan": [[-1.2, 1.5], [1.2, 1.5], [1.2, -1.6], [-1.2, -1.6]],
                     "side_slope_deg": 10, "front_slope_deg": 30},
        "cupola": {"x": -0.5, "y": -0.5, "diameter": 0.6, "height": 0.25, "vision_blocks": 6},
        "part": [MARK, {"part": "exhaust", "position": [0, -2.8, 1.0], "kind": "muffler"},
                 {"part": "hull_hatch", "center": [0.4, -0.6], "size": [0.6, 0.5], "z": "casemate", "socket": True},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -1.5, 1.4]}],
        "gun": {"part": "tank_gun", "trunnion": [1.30, 1.55], "x": 0.15,
                "mantlet": {"kind": "saukopf", "size": [0.55, 0.45, 0.5]},
                "segments": [["fit", 0.06, 0.05]], "muzzle_brake": "double_baffle"},
        "mg": [{"part": "pintle_mg", "position": [0.5, 0.3, 2.3], "shield": {"width": 0.5, "height": 0.35, "y": 0.2}}],
    }
    run_assembly(model, "casemate", bp, u)


def demo_open_spg(model):
    """Self-propelled howitzer: single torsion-bar wheels, open compartment, ball mantlet."""
    u = fake_unit("kit_test_open_spg", "self_propelled", length_mm=4800, width_mm=2280, height_mm=2300)
    bp = {
        "assembly": "casemate",
        "layout": {"hull_length": 4.6},
        "running_gear": {"part": "torsion_bar", "twin": False, "arm_leading": True, "tire": "Paint",
                         "track_width": 0.30, "road_wheel_diameter": 0.55,
                         "road_wheels": [1.4, 0.75, 0.1, -0.55, -1.2], "sprocket": {"y": 2.05, "z": 0.62, "diameter": 0.55},
                         "idler": {"y": -2.15, "z": 0.55, "diameter": 0.5}},
        "hull": {"part": "plate_hull", "lower": [[-2.3, 0.4], [1.9, 0.4], [2.3, 0.8], [2.3, 1.0], [-2.3, 1.0]],
                 "upper": [[-2.3, 0.99], [2.3, 0.99], [2.0, 1.25], [-2.3, 1.25]], "upper_half_width": 1.0},
        "casemate": {"part": "casemate", "open": True, "plan": [[-1.05, 0.3], [1.05, 0.3], [1.05, -2.2], [-1.05, -2.2]],
                     "side_slope_deg": 5, "front_slope_deg": 25, "rear_slope_deg": 0},
        "part": [MARK, {"part": "exhaust", "position": [0.8, -2.3, 0.9], "kind": "pipe"},
                 {"part": "crew_sockets", "parent": "Hull", "count": 3, "center": [0, -1.0], "radius": 0.5},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 1.0, 1.2]}],
        "gun": {"part": "tank_gun", "trunnion": [0.0, 1.8], "mantlet": {"kind": "ball", "size": [0.5, 0.3, 0.5]},
                "segments": [["fit", 0.07, 0.065]], "muzzle_brake": "single_baffle"},
    }
    run_assembly(model, "casemate", bp, u)


def demo_heavy_tank(model):
    """Heavy tank on interleaved road wheels (three layers)."""
    u = fake_unit("kit_test_heavy", "tank_heavy", length_mm=8450, length_hull_mm=6300, width_mm=3700, height_mm=3000)
    bp = {
        "assembly": "tank",
        "layout": {"turret_y": 0.2, "turret_ring_diameter": 1.85, "fender_overhang": 0.05},
        "running_gear": {"part": "interleaved", "rows": 3, "track_width": 0.72, "road_wheel_diameter": 0.80,
                         "road_wheels": [2.0, 1.45, 0.9, 0.35, -0.2, -0.75, -1.3, -1.85],
                         "sprocket": {"y": 2.85, "z": 0.75, "diameter": 0.84},
                         "idler": {"y": -2.85, "z": 0.70, "diameter": 0.60}},
        "hull": {"part": "plate_hull", "lower": [[-3.1, 0.45], [2.6, 0.45], [3.15, 1.0], [3.15, 1.25], [-3.15, 1.25]],
                 "upper": [[-3.15, 1.24], [3.15, 1.24], [3.1, 1.80], [-3.15, 1.80]]},
        "turret": {"part": "plate_turret", "plan": [[-0.9, 1.1], [0.9, 1.1], [1.1, 0.2], [1.0, -1.3], [-1.0, -1.3],
                                                   [-1.1, 0.2]], "side_slope_deg": 2},
        "cupola": {"x": -0.55, "y": -0.6, "diameter": 0.7, "height": 0.25, "vision_blocks": 7},
        "part": [MARK, {"part": "exhaust", "position": [0.4, -3.1, 1.6], "kind": "pipe"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -2, 1.9]}],
        "gun": {"part": "tank_gun", "trunnion": [1.0, 0.45], "mantlet": {"kind": "box", "size": [1.2, 0.2, 0.6]},
                "segments": [["fit", 0.075, 0.065]], "muzzle_brake": "double_baffle"},
        "mg": [{"part": "mg", "mount": "coax", "position": [0.25, 0.1, 0.0]}],
    }
    run_assembly(model, "tank", bp, u)


def demo_armored_car(model):
    """Six-wheeled armored car with an open-topped turret."""
    u = fake_unit("kit_test_car", "recon_vehicle", length_mm=4700, width_mm=2300, height_mm=1950)
    sec = lambda y, top: {"y": y, "bottom_z": 0.55, "bottom_hw": 0.85, "chine_z": 1.0, "chine_hw": 1.1,
                          "top_z": top, "top_hw": 0.95}  # noqa: E731
    bp = {
        "assembly": "armored_car",
        "layout": {"turret_y": -0.1, "turret_ring_diameter": 1.3},
        "axle": [{"y": 1.5, "diameter": 0.92, "width": 0.25, "track": 1.9},
                 {"y": -0.4, "diameter": 0.92, "width": 0.25, "track": 1.9},
                 {"y": -1.6, "diameter": 0.92, "width": 0.25, "track": 1.9}],
        "hull": {"part": "section_hull", "height": "keep",
                 "sections": [sec(2.35, 1.2), sec(1.2, 1.45), sec(-1.5, 1.45), sec(-2.35, 1.3)]},
        "turret": {"part": "plate_turret", "open_top": True, "plan": [[-0.7, 0.75], [0.7, 0.75], [0.8, -0.6],
                                                                      [-0.8, -0.6]], "side_slope_deg": 12},
        "part": [MARK, {"part": "exhaust", "position": [0.6, -2.3, 0.8], "kind": "pipe"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 0, 1.4]}],
        "gun": {"part": "tank_gun", "trunnion": [0.6, 0.25], "mantlet": {"kind": "round", "size": [0.4, 0.2, 0.3]},
                "segments": [["fit", 0.03, 0.025]]},
        "mg": [{"part": "mg", "mount": "coax", "position": [0.2, 0.0, 0.0]}],
    }
    run_assembly(model, "armored_car", bp, u)


def demo_aa_gun(model):
    """Heavy AA gun on a cruciform platform with two bogies kept attached."""
    u = fake_unit("kit_test_flak", "gun_aa_heavy", length_mm=7600, width_mm=2300, height_mm=2400, crew=8)
    bp = {
        "assembly": "aa_gun",
        "carriage": {"part": "cruciform_platform", "length": 6.0, "center": [1.4, 0.35]},
        "axle": [{"y": 2.2, "diameter": 0.9, "width": 0.2, "track": 1.6},
                 {"y": -2.2, "diameter": 0.9, "width": 0.2, "track": 1.6}],
        "mount": {"part": "aa_mount", "seats": [[-0.6, -0.3, 1.1], [0.6, -0.3, 1.1]],
                  "handwheels": [[-0.55, 0.1, 1.0], [0.55, 0.1, 1.0]],
                  "shield": {"width": 1.6, "height": 0.9, "bottom_z": 1.0, "y": 0.7}},
        "gun": {"part": "field_gun", "trunnion": [0.0, 1.75], "cradle": [1.4, 0.3, 0.3], "recoil": [1.2, 0.07],
                "segments": [["fit", 0.07, 0.06]], "muzzle_brake": "none"},
        "part": [{"part": "crew_sockets", "radius": 2.6},
                 {"part": "tow_hook", "position": [0, -3.0, 0.5], "parent": "Carriage"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 0, 1.0], "parent": "Carriage"},
                 {"part": "marking", "name": "Shield", "kind": "number", "parent": "Shield",
                  "position": [-0.5, 0.75, 1.5], "normal": [0, 1, 0], "size": 0.3}],
    }
    run_assembly(model, "aa_gun", bp, u)


def demo_mortar(model):
    """Heavy mortar: round baseplate, fitted elevation and bipod, transport wheels."""
    u = fake_unit("kit_test_mortar", "mortar_heavy", length_mm=1900, width_mm=1300, height_mm=1600, crew=6)
    bp = {
        "assembly": "mortar",
        "baseplate": {"part": "baseplate", "shape": "round", "diameter": 0.9},
        "tube": {"part": "mortar_tube", "length": 1.862, "caliber": 0.12},
        "bipod": {"part": "mortar_bipod"},
        "axle": [{"y": 0.2, "diameter": 0.6, "width": 0.1, "track": 0.9}],
        "part": [{"part": "crew_sockets", "radius": 1.6},
                 {"part": "tow_hook", "position": [0, -0.5, 0.3], "parent": "Carriage"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 0, 0.5], "parent": "Carriage"},
                 {"part": "marking", "name": "Base", "kind": "number", "parent": "Carriage",
                  "position": [0, -0.2, 0.06], "normal": [0, 0, 1], "size": 0.2}],
    }
    run_assembly(model, "mortar", bp, u)
    rect = Model("kit_test_rect_base")  # rectangular baseplate and fixed elevation
    rect.mb("Carriage")
    ctx = {"height": 1.2, "length": 1.5, "width": 0.8, "unit": u}
    run(rect, "baseplate", {"shape": "rect", "size": [0.7, 0.5]}, ctx)
    run(rect, "mortar_tube", {"length": 1.22, "caliber": 0.107, "elevation_deg": 60}, ctx)
    for p_ in rect.parts.values():
        p_["mb"].bm.free()


def demo_bomber(model):
    """Twin-engine bomber: glazed nose, nacelles, bomb bay, turrets, shadow proxy."""
    u = fake_unit("kit_test_bomber", "aircraft_bomber", length_mm=16100, wingspan_mm=20600, height_mm=4900, crew=6,
                  weapon=[{"name": "500 lb bomb", "count": 6}])
    bp = {
        "assembly": "aircraft",
        "fuselage": {"part": "fuselage", "glass_from_y": 6.4,
                     "sections": [[-8.6, 0, 0, 2.4, 2], [-6.0, 0.8, 1.0, 2.3, 2.4], [0, 1.4, 1.9, 2.1, 2.6],
                                  [5.0, 1.3, 1.7, 2.0, 2.4], [6.4, 1.1, 1.4, 2.0, 2.2], [7.5, 0.4, 0.5, 2.0, 2]]},
        "wing": {"part": "wing", "y": 2.0, "z": 2.3, "root_chord": 3.4, "tip_chord": 1.3, "dihedral_deg": 4},
        "tail": {"part": "tail", "y": -6.8, "z": 2.6, "span": 6.7, "root_chord": 1.8, "tip_chord": 1.2,
                 "fin_height": 1.9, "twin_fins": 3.2},
        "canopy": {"part": "canopy", "y": [2.2, 4.0], "z": 2.75, "width": 1.0, "height": 0.4},
        "landing_gear": {"part": "landing_gear", "nose": True, "main_y": 1.0, "main_track": 5.0,
                         "main_wheel_diameter": 1.1, "strut_top_z": 1.9, "tail_y": 5.5, "tail_wheel_diameter": 0.7,
                         "tail_strut_top_z": 1.6},
        "propeller": [{"part": "propeller", "name": "Propeller_L", "position": [-2.5, 4.4, 2.2], "diameter": 3.8},
                      {"part": "propeller", "name": "Propeller_R", "position": [2.5, 4.4, 2.2], "diameter": 3.8}],
        "part": [{"part": "nacelle", "x": 2.5, "y": [-0.5, 4.2], "z": 2.2, "diameter": 1.3},
                 {"part": "bomb_bay", "y": [-0.5, 1.5], "width": 0.9, "z": 1.3},
                 {"part": "gun_turret", "position": [0, 0.5, 3.0], "diameter": 0.9},
                 {"part": "gun_turret", "position": [0, -8.3, 2.5], "diameter": 0.7, "guns": 2,
                  "direction": [0, -1, 0]},
                 {"part": "flex_mg", "position": [-0.7, -3.5, 2.5], "direction": [-1, 0, 0], "twin": True},
                 {"part": "fixed_guns", "muzzles": [[0.4, 6.9, 1.8], [0.55, 6.6, 1.6]], "length": 0.4},
                 {"part": "shadow_proxy"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [2.5, 1.0, 2.2]},
                 {"part": "marking", "name": "Wing_L", "kind": "national", "position": [-7, 1.5, 2.9],
                  "normal": [0, 0, 1], "size": 1.0}],
    }
    run_assembly(model, "aircraft", bp, u)


def demo_dive_bomber(model):
    """Inverted gull wing, spatted fixed gear, centerline bomb, rockets, rear gun."""
    u = fake_unit("kit_test_stuka", "aircraft_attack", length_mm=11500, wingspan_mm=15000, height_mm=3900, crew=2)
    bp = {
        "assembly": "aircraft",
        "fuselage": {"part": "fuselage", "sections": [[-6.5, 0, 0, 2.2, 2], [-4.0, 0.7, 0.9, 2.1, 2.6],
                                                     [0, 1.1, 1.5, 2.0, 2.8], [3.5, 1.1, 1.3, 2.0, 2.4],
                                                     [4.3, 0.6, 0.6, 2.0, 2]]},
        "wing": {"part": "wing", "y": 1.6, "z": 1.5, "root_chord": 2.8, "tip_chord": 1.2, "dihedral_deg": 8,
                 "kink": {"at": 0.3, "inner_dihedral_deg": -12}},
        "tail": {"part": "tail", "y": -5.2, "z": 2.4, "span": 4.0, "root_chord": 1.4, "tip_chord": 0.9,
                 "fin_height": 1.3},
        "canopy": {"part": "canopy", "y": [-1.6, 1.0], "z": 2.7, "width": 0.8, "height": 0.5},
        "landing_gear": {"part": "landing_gear", "fixed": True, "spats": True, "main_y": 1.0, "main_track": 3.3,
                         "main_wheel_diameter": 0.84, "strut_top_z": 1.25, "tail_y": -5.6,
                         "tail_wheel_diameter": 0.35},
        "propeller": [{"part": "propeller", "position": [0, 4.5, 2.0], "diameter": 3.4}],
        "part": [{"part": "bomb_load", "bombs": [[0, 0.6, 1.25]], "length": 2.0, "diameter": 0.4, "swing_arm": True},
                 {"part": "bomb_load", "bombs": [[2.6, 1.0, 1.0]], "length": 1.0, "diameter": 0.2},
                 {"part": "rocket_rails", "rockets": [[3.6, 1.4, 1.2], [4.0, 1.4, 1.25]]},
                 {"part": "flex_mg", "position": [0, -1.8, 2.8], "twin": True},
                 {"part": "fixed_guns", "muzzles": [[2.0, 2.2, 1.2]], "radius": 0.02},
                 {"part": "shadow_proxy"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 2.0, 2.0]},
                 {"part": "marking", "name": "Hull_L", "kind": "national", "position": [-0.5, -3.0, 2.1],
                  "normal": [-1, 0, 0], "size": 0.8}],
    }
    run_assembly(model, "aircraft", bp, u)


def demo_biplane(model):
    """Biplane: upper wing, struts, fixed gear with a tail skid."""
    u = fake_unit("kit_test_biplane", "aircraft_recon", length_mm=8200, wingspan_mm=11400, height_mm=3100, crew=2)
    bp = {
        "assembly": "aircraft",
        "fuselage": {"part": "fuselage", "sections": [[-5.0, 0, 0, 1.5, 2], [-3.0, 0.5, 0.6, 1.4, 3],
                                                     [0, 0.9, 1.1, 1.3, 3], [2.4, 0.9, 0.9, 1.3, 2.6],
                                                     [2.9, 0.5, 0.5, 1.3, 2]]},
        "wing": {"part": "wing", "y": 1.4, "z": 0.95, "root_chord": 1.6, "tip_chord": 1.6, "span": 10.6,
                 "dihedral_deg": 2},
        "tail": {"part": "tail", "y": -4.0, "z": 1.6, "span": 3.2, "root_chord": 1.1, "tip_chord": 0.8,
                 "fin_height": 1.0},
        "landing_gear": {"part": "landing_gear", "fixed": True, "skid": True, "main_y": 1.3, "main_track": 1.8,
                         "main_wheel_diameter": 0.7, "strut_top_z": 0.95, "tail_y": -4.6, "tail_strut_top_z": 1.0},
        "propeller": [{"part": "propeller", "position": [0, 3.1, 1.3], "diameter": 2.8, "blades": 2}],
        "part": [{"part": "upper_wing", "y": 1.7, "z": 2.4, "root_chord": 1.6, "tip_chord": 1.6, "x0": 0.4},
                 {"part": "wing_struts", "struts": [[[2.8, 1.0, 1.0], [2.8, 1.2, 2.45]], [[0.35, 1.0, 1.7],
                                                                                         [0.4, 1.2, 2.4]]]},
                 {"part": "shadow_proxy"},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, 1.5, 1.4]},
                 {"part": "marking", "name": "Wing_L", "kind": "national", "position": [-4, 1.0, 2.5],
                  "normal": [0, 0, 1], "size": 0.8}],
    }
    run_assembly(model, "aircraft", bp, u)


# --- prompt 063: parts for the vehicles of the reference folder ---------------------

def demo_multi_turret(model):
    """Multi-turret heavy tank (T-35 type): small wheels in skirted bogies,
    main turret plus gun and MG sub-turrets."""
    u = fake_unit("kit_test_multi", "tank_heavy", length_mm=9700, width_mm=3200, height_mm=3430,
                  weapon=[{"name": "MG", "caliber_mm_x10": 76, "count": 3}])
    bp = {
        "assembly": "tank",
        "layout": {"turret_y": 0.3, "turret_ring_diameter": 1.6, "fender_overhang": 0.1, "hull_length": 9.5},
        "running_gear": {"part": "small_wheel_bogie", "per_bogie": 2, "spring": "coil", "track_width": 0.52,
                         "road_wheel_diameter": 0.45,
                         "road_wheels": [3.0, 2.45, 1.75, 1.2, 0.5, -0.05, -0.75, -1.3, -2.0, -2.55],
                         "sprocket": {"y": -4.3, "z": 0.75, "diameter": 0.7},
                         "idler": {"y": 4.35, "z": 0.75, "diameter": 0.7},
                         "return_rollers": {"y": [2.6, 1.0, -0.6, -2.2], "z": 1.0, "diameter": 0.22}},
        "hull": {"part": "plate_hull", "lower": [[-4.6, 0.45], [4.1, 0.45], [4.75, 1.0], [4.75, 1.25], [-4.75, 1.25]],
                 "upper": [[-4.75, 1.24], [4.75, 1.24], [4.4, 1.9], [-4.75, 1.9]], "upper_half_width": 1.2},
        "turret": {"part": "plate_turret", "plan": [[-0.85, 0.9], [0.85, 0.9], [1.0, 0.0], [0.85, -1.0], [-0.85, -1.0],
                                                   [-1.0, 0.0]], "side_slope_deg": 4},
        "part": [MARK, {"part": "exhaust", "position": [0.4, -4.7, 1.6], "kind": "pipe"},
                 {"part": "skirts", "y": [-4.0, 3.8], "bottom_z": 0.55, "top_z": 1.25, "plates": 1, "gap": 0.0},
                 {"part": "sub_turret", "position": [0.75, 2.1], "diameter": 1.1, "height": 0.55,
                  "gun": {"length": 1.0, "radius": 0.03, "mantlet": [0.25, 0.15, 0.2]}, "mgs": 1},
                 {"part": "sub_turret", "position": [-0.75, -1.6], "diameter": 1.1, "height": 0.55,
                  "gun": {"length": 1.0, "radius": 0.03, "direction_deg": 180}},
                 {"part": "sub_turret", "position": [-0.75, 2.1], "plan": [[-0.35, 0.4], [0.35, 0.4], [0.4, -0.35],
                                                                          [-0.4, -0.35]],
                  "height": 0.5, "mgs": 1, "hatch": False},
                 {"part": "turret_hatch", "x": 0.3, "y": -0.3, "diameter": 0.5, "socket": True},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -3, 2.0]}],
        "gun": {"part": "tank_gun", "trunnion": [0.85, 0.4], "mantlet": {"kind": "box", "size": [0.5, 0.15, 0.4]},
                "segments": [[1.2, 0.05, 0.045]]},
        "mg": [{"part": "mg", "mount": "coax", "position": [0.2, 0.1, 0.0]}],
    }
    run_assembly(model, "tank", bp, u)


def demo_sponson_gun(model):
    """Medium tank with a 75 mm sponson gun, a 37 mm turret gun and an MG
    sub-turret on the turret roof (M3 Lee type)."""
    u = fake_unit("kit_test_sponson", "tank_medium", length_mm=6120, length_hull_mm=5640, width_mm=2720,
                  height_mm=3120, weapon=[{"name": "MG", "caliber_mm_x10": 76, "count": 1}])
    bp = {
        "assembly": "tank",
        "layout": {"turret_y": 0.45, "turret_ring_diameter": 1.4, "fender_overhang": 0.03},
        "running_gear": {"part": "vvss_bogie", "track_width": 0.42, "road_wheel_diameter": 0.51,
                         "road_wheels": [1.85, 1.35, 0.25, -0.25, -1.35, -1.85],
                         "sprocket": {"y": 2.55, "z": 0.72, "diameter": 0.64},
                         "idler": {"y": -2.6, "z": 0.45, "diameter": 0.55}},
        "hull": {"part": "plate_hull", "lower": [[-2.8, 0.45], [2.3, 0.45], [2.82, 1.0], [2.82, 1.1], [-2.82, 1.1]],
                 "upper": [[-2.82, 1.09], [2.82, 1.09], [2.5, 1.9], [-2.82, 1.95]]},
        "turret": {"part": "cast_turret", "length": 1.7, "width": 1.6, "height": 0.6},
        "part": [MARK,
                 {"part": "sponson_gun", "x": 0.75, "y": 2.6, "z": 1.55, "sponson": [0.75, 0.9, 0.7], "cast": True,
                  "mantlet": {"kind": "round", "size": [0.45, 0.2, 0.4]},
                  "segments": [["fit", 0.055, 0.05]], "muzzle_brake": "thread_protector"},
                 {"part": "exhaust", "position": [0.4, -2.85, 1.4], "kind": "pipe"},
                 {"part": "sub_turret", "parent": "Turret", "position": [0.0, 0.2], "z": 2.55, "diameter": 0.6,
                  "height": 0.3, "mgs": 1},
                 {"part": "turret_hatch", "x": -0.4, "y": -0.3, "diameter": 0.45, "socket": True},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -2, 2.0]}],
        "gun": {"part": "tank_gun", "trunnion": [0.7, 0.3], "mantlet": {"kind": "round", "size": [0.35, 0.15, 0.3]},
                "segments": [[1.4, 0.025, 0.022]], "muzzle_brake": "thread_protector"},
    }
    run_assembly(model, "tank", bp, u)


def demo_amphibious(model):
    """Amphibious tankette (T-37A type): small wheels with an outer girder,
    float boxes over the tracks, offset MG turret, rear propeller."""
    u = fake_unit("kit_test_amphibious", "tank_light", length_mm=3750, width_mm=2000, height_mm=1820,
                  weapon=[{"name": "DT", "caliber_mm_x10": 76}])
    bp = {
        "assembly": "tank",
        "layout": {"turret_y": 0.3, "turret_ring_diameter": 0.9, "hull_length": 3.75},
        "running_gear": {"part": "small_wheel_bogie", "bogies": [[0, 1], [2, 3]], "girder": True,
                         "track_width": 0.2, "road_wheel_diameter": 0.36, "road_wheels": [0.9, 0.5, -0.3, -0.7],
                         "sprocket": {"y": 1.45, "z": 0.45, "diameter": 0.45},
                         "idler": {"y": -1.45, "z": 0.4, "diameter": 0.4},
                         "return_rollers": {"y": [0.6, -0.6], "z": 0.62, "diameter": 0.15}},
        "hull": {"part": "plate_hull", "lower": [[-1.85, 0.35], [1.5, 0.3], [1.87, 0.75], [1.87, 0.9], [-1.87, 0.9]],
                 "upper": [[-1.87, 0.89], [1.87, 0.89], [1.6, 1.25], [-1.87, 1.25]], "upper_half_width": 0.7},
        "turret": {"part": "plate_turret", "plan": [[-0.4, 0.4], [0.4, 0.4], [0.4, -0.4], [-0.4, -0.4]],
                   "side_slope_deg": 3},
        "part": [{"part": "amphibious_floats", "y": [1.6, -1.7], "z": [0.62, 0.95], "width": 0.2},
                 dict(MARK, position=[-1.0, 0.0, 0.8]), {"part": "exhaust", "position": [0.3, -1.85, 1.0], "kind": "pipe"},
                 {"part": "screw_propeller", "position": [0.0, -1.95, 0.45], "diameter": 0.4, "tunnel": True},
                 {"part": "turret_hatch", "x": 0.0, "y": -0.1, "diameter": 0.4, "socket": True},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -1, 1.3]}],
        "gun": {"part": "tank_gun", "trunnion": [0.4, 0.25], "mantlet": {"kind": "ball", "size": [0.18, 0.1, 0.18]},
                "segments": [["fit", 0.012, 0.012]]},
    }
    run_assembly(model, "tank", bp, u)


def demo_hvss(model):
    """Medium tank on horizontal volute spring bogies (M4A3E8 type)."""
    u = fake_unit("kit_test_hvss", "tank_medium", length_mm=7540, length_hull_mm=6270, width_mm=3000, height_mm=2970)
    bp = {
        "assembly": "tank",
        "layout": {"turret_y": 0.2, "turret_ring_diameter": 1.75, "fender_overhang": 0.02},
        "running_gear": {"part": "hvss_bogie", "track_width": 0.58, "road_wheel_diameter": 0.52,
                         "road_wheels": [2.05, 1.6, 0.3, -0.15, -1.45, -1.9],
                         "sprocket": {"y": 2.75, "z": 0.8, "diameter": 0.66},
                         "idler": {"y": -2.75, "z": 0.48, "diameter": 0.6}},
        "hull": {"part": "plate_hull", "lower": [[-3.1, 0.45], [2.6, 0.45], [3.13, 1.0], [3.13, 1.1], [-3.13, 1.1]],
                 "upper": [[-3.13, 1.09], [3.13, 1.09], [2.2, 1.95], [-3.13, 1.95]], "cast": True},
        "turret": {"part": "cast_turret", "length": 2.3, "width": 1.9},
        "part": [MARK, {"part": "exhaust", "position": [0.4, -3.1, 1.3], "kind": "pipe"},
                 {"part": "turret_hatch", "x": -0.4, "y": -0.3, "diameter": 0.5, "socket": True},
                 {"part": "socket", "name": "Socket_Fire_", "position": [0, -2, 2.0]}],
        "gun": {"part": "tank_gun", "trunnion": [0.9, 0.35], "mantlet": {"kind": "rotor", "size": [0.9, 0.2, 0.5]},
                "segments": [["fit", 0.055, 0.05]], "muzzle_brake": "single_baffle"},
    }
    run_assembly(model, "tank", bp, u)


ENV_DEMOS = [
    ("tobruk", {}, (5.0, 4.0, 0.6)),
    ("log_bunker", {"walls": "logs"}, (6.0, 5.0, 1.0)),
    ("log_bunker", {"walls": "sandbags"}, (3.0, 2.5, 0.9)),
    ("minefield", {"mine": "at"}, (20.0, 20.0, 1.0)),
    ("minefield", {"mine": "ap", "visible": 12}, (20.0, 20.0, 1.0)),
]


def env_demos(failures):
    """Builds the fortification generators of kit/forts.py into env models."""
    from kit import env, forts
    from kit.registry import Params

    for i, (name, params, dims) in enumerate(ENV_DEMOS):
        clear_scene()
        model = env.EnvModel(f"kit_test_env_{i}")
        try:
            p = Params(dict(params), name)
            env.ENV_PARTS[name](model, "Variant", p, env.Box(*dims))
            if p.unused():
                raise KeyError(f"unknown parameters {p.unused()}")
            names = model.variant_parts("Variant")
            env.fit(model, names, env.Box(*dims))
            root = model.finish()
            objs = [root] + descendants(root)
            bpy.context.view_layer.update()
            env.bake_env(objs)
            tris = sum(tri_count(o) for o in objs)
            if not 20 < tris <= 2000:
                failures.append(f"env {name}: implausible triangle count {tris}")
            print(f"  ok   env {name:12} {tris:6} tris")
            if render_dir():
                from kit import render

                render.turntable(root, render_dir() / f"env_{i}_{name}.png")
        except Exception as exc:
            import traceback

            traceback.print_exc()
            failures.append(f"env {name}: {exc}")
    unused = sorted(n for n, fn in env.ENV_PARTS.items() if fn.__module__ == forts.__name__ and
                    n not in {d[0] for d in ENV_DEMOS})
    if unused:
        failures.append(f"fortification generators not covered: {unused}")


def run(model, name, params, ctx):
    USED.add(name)
    build_part(name, model, params, ctx)


def render_dir():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    return Path(argv[argv.index("--render") + 1]) if "--render" in argv else None


def main():
    load_parts()
    failures = []
    out = render_dir()
    demos = (demo_halftrack, demo_truck, demo_tank, demo_aircraft, demo_casemate, demo_open_spg, demo_heavy_tank,
             demo_armored_car, demo_aa_gun, demo_mortar, demo_bomber, demo_dive_bomber, demo_biplane,
             demo_multi_turret, demo_sponson_gun, demo_amphibious, demo_hvss)
    for i, demo in enumerate(demos):
        clear_scene()
        model = Model(f"kit_test_{demo.__name__}")
        try:
            demo(model)
            root = model.finish()
            objs = [root] + descendants(root)
            bpy.context.view_layer.update()
            bake(objs)
            tris = sum(tri_count(o) for o in objs)
            meshes = [o for o in objs if o.type == "MESH"]
            empty = [o.name for o in meshes if not o.data.polygons]
            if empty:
                failures.append(f"{demo.__name__}: empty meshes {empty}")
            if not 100 < tris < 12000:
                failures.append(f"{demo.__name__}: implausible triangle count {tris}")
            print(f"  ok   {demo.__name__:16} {len(meshes):3} meshes {tris:6} tris")
            if out:
                from kit import render

                render.turntable(root, out / f"{demo.__name__}.png")
            for o in objs:
                if o.type == "EMPTY" and o.name.startswith("Socket_") and o.parent is None:
                    failures.append(f"{demo.__name__}: socket {o.name} without parent")
        except Exception as exc:  # report every demo, not only the first failure
            import traceback

            traceback.print_exc()
            failures.append(f"{demo.__name__}: {exc}")
    env_demos(failures)
    # parts used by the real blueprints are covered by `just units`
    from kit import catalog

    for faction, uid in catalog.all_blueprints():
        bp = catalog.blueprint(faction, uid)
        for key, val in bp.items():
            tables = val if isinstance(val, list) else [val]
            for t in tables:
                if isinstance(t, dict) and "part" in t:
                    USED.add(t["part"])
        if "cupola" in bp:
            USED.add("cupola")
    unused = sorted(set(PARTS) - USED)
    if unused:
        failures.append(f"parts not covered by any test or blueprint: {unused}")
    undocumented = sorted(n for n, fn in list(PARTS.items()) + list(ASSEMBLIES.items()) if not fn.__doc__)
    if undocumented:
        failures.append(f"parts without docstring: {undocumented}")
    if failures:
        for f in failures:
            print(f"kit_test: FAIL {f}")
        sys.exit(1)
    print(f"kit_test: {len(PARTS)} parts, {len(ASSEMBLIES)} assemblies ok")


main()
