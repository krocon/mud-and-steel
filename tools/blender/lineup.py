"""Renders all built models side by side with the 1.80 m reference figure.

    blender -b --factory-startup --python tools/blender/lineup.py -- <output.png>

Appends every model with a blueprint (assets-src/blender/<faction>/<id>.blend)
and the `ScaleReference` collection of assets-src/blender/reference/
scale_reference.blend into an empty scene, places them in a row and renders
an orthographic side view (true proportions) and a perspective view.
Also prints the measured sizes so the line-up can be checked in text form.
"""

import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog, render  # noqa: E402
from kit.core import descendants, world_bbox  # noqa: E402

REFERENCE = catalog.REPO_ROOT / "assets-src" / "blender" / "reference" / "scale_reference.blend"


def append_objects(path):
    with bpy.data.libraries.load(str(path), link=False) as (src, dst):
        dst.objects = list(src.objects)
    objs = [o for o in dst.objects if o is not None]
    for o in objs:
        bpy.context.scene.collection.objects.link(o)
    return objs


def merge_slot_materials():
    """Appending renames shared slot materials (Paint.001 ...); map them back."""
    for mat in list(bpy.data.materials):
        base, _, suffix = mat.name.partition(".")
        if suffix.isdigit() and base in bpy.data.materials:
            mat.user_remap(bpy.data.materials[base])
            bpy.data.materials.remove(mat)


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = Path(argv[0] if argv else "build/renders/lineup.png").resolve()
    bpy.ops.wm.read_homefile(use_empty=True)
    roots = []
    for faction, uid in catalog.all_blueprints():
        blend = catalog.blend_path(faction, uid)
        if not blend.exists():
            print(f"lineup: skipping {uid} (not built)")
            continue
        append_objects(blend)
        roots.append(bpy.data.objects[uid])
    merge_slot_materials()
    ref_root = bpy.data.objects.new("Reference_180", None)
    bpy.context.scene.collection.objects.link(ref_root)
    # turn the reference so that its 10 m scale bar runs along the row (-Y)
    ref_root.rotation_euler.z = -1.5707963
    for o in append_objects(REFERENCE):
        if o.parent is None:
            o.parent = ref_root
    bpy.context.view_layer.update()
    print("lineup: measured sizes incl. skirts, antennas, cupolas (length x width x height, m)")
    for r in roots + [ref_root]:
        lo, hi = world_bbox([r] + descendants(r))
        d = hi - lo
        print(f"  {r.name:22} {d.y:6.2f} x {d.x:5.2f} x {d.z:5.2f}")
    path = render.lineup(roots, out, reference=ref_root)
    print(f"lineup: {path}")


try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
