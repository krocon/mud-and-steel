"""Builds a unit model from its blueprint: .blend, .glb and preview renders.

Run headless (or use `just unit <faction> <id>`):
    blender -b --factory-startup --python tools/blender/build_unit.py -- <faction> <id> [--render DIR]

Reads   assets-src/blueprints/<faction>/<id>.toml and data/catalog/<faction>.toml
Writes  assets-src/blender/<faction>/<id>.blend
        godot/assets/models/<faction>/<id>.glb
        with --render: DIR/<id>_turntable.png (4 views, Eevee)
"""

import argparse
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog  # noqa: E402
from kit.build import build_model, clear_scene, export_glb  # noqa: E402


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("faction")
    ap.add_argument("unit_id")
    ap.add_argument("--render", type=Path, help="directory for the turntable preview")
    return ap.parse_args(argv)


def main():
    args = parse_args()
    clear_scene()
    root = build_model(args.faction, args.unit_id)
    blend = catalog.blend_path(args.faction, args.unit_id)
    glb = catalog.glb_path(args.faction, args.unit_id)
    blend.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend), compress=True)
    export_glb(glb)
    print(f"build_unit: {root.name} -> {blend.relative_to(catalog.REPO_ROOT)}, {glb.relative_to(catalog.REPO_ROOT)}")
    if args.render:
        from kit import render

        out = render.turntable(root, args.render / f"{args.unit_id}_turntable.png")
        print(f"build_unit: preview {out}")


try:
    main()
except Exception as exc:  # report and fail the Blender process
    import traceback

    traceback.print_exc()
    print(f"build_unit: FAILED: {exc}", file=sys.stderr)
    sys.exit(1)
