"""Builds every unit that has a blueprint (`just units`).

    blender -b --factory-startup --python tools/blender/build_all.py -- [--render DIR]

Each unit is built in a fresh scene within one Blender process.
"""

import argparse
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog, squad  # noqa: E402
from kit.build import build_model, clear_scene, export_glb  # noqa: E402


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser()
    ap.add_argument("--render", type=Path)
    args = ap.parse_args(argv)
    units = catalog.all_blueprints()
    for faction, uid in units:
        if squad.is_squad(faction, uid):  # infantry squad: Godot scene only
            squad.build(faction, uid)
            print(f"build_all: {faction}/{uid} (squad)")
            continue
        bpy.ops.wm.read_homefile(use_empty=True)
        clear_scene()
        root = build_model(faction, uid)
        blend = catalog.blend_path(faction, uid)
        blend.parent.mkdir(parents=True, exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=str(blend), compress=True)
        export_glb(catalog.glb_path(faction, uid))
        print(f"build_all: {faction}/{uid}")
        if args.render:
            from kit import render

            render.turntable(root, args.render / f"{uid}_turntable.png")
    print(f"build_all: {len(units)} unit(s) built")


try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
