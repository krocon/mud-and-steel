"""Smoke test of the part kit (`just kit-test`).

Builds demo units that together use every registered part (also those no
real blueprint uses yet: halftrack, wheeled vehicle, aircraft) with made-up
catalog values, and checks that each builds, has geometry, bakes and stays
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


def run(model, name, params, ctx):
    USED.add(name)
    build_part(name, model, params, ctx)


def main():
    load_parts()
    failures = []
    demos = (demo_halftrack, demo_truck, demo_tank, demo_aircraft)
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
        except Exception as exc:  # report every demo, not only the first failure
            import traceback

            traceback.print_exc()
            failures.append(f"{demo.__name__}: {exc}")
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
