"""Exports the opened .blend file to binary glTF (.glb) for Godot.

Run headless:
    blender -b assets-src/blender/<name>.blend --python tools/export_blender.py

By default the result goes to godot/assets/models/<name>.glb. Use
`-- --output <path>` to choose another file.

Exported: meshes, materials and the object hierarchy with modifiers applied.
Not exported: cameras and lights (the game scene provides its own).
"""

import argparse
import sys
from pathlib import Path

import bpy

REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_DIR = REPO_ROOT / "godot" / "assets" / "models"


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="target .glb file")
    return parser.parse_args(argv)


def main():
    blend = bpy.data.filepath
    if not blend:
        sys.exit("export_blender.py: open a .blend file first (blender -b <file> --python ...)")

    args = parse_args()
    output = args.output or MODELS_DIR / (Path(blend).stem + ".glb")
    output.parent.mkdir(parents=True, exist_ok=True)

    result = bpy.ops.export_scene.gltf(
        filepath=str(output),
        export_format="GLB",
        export_apply=True,
        export_yup=True,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        use_selection=False,
    )
    if "FINISHED" not in result:
        sys.exit(f"export_blender.py: export failed: {result}")
    print(f"Exported {blend} -> {output}")


main()
