"""Renders all built models side by side with the 1.80 m reference figure.

    blender -b --factory-startup --python tools/blender/lineup.py -- <output.png> [<faction>]

Appends every model with a blueprint (assets-src/blender/<faction>/<id>.blend)
and the `ScaleReference` collection of assets-src/blender/reference/
scale_reference.blend into an empty scene, places them in a row and renders
an orthographic side view (true proportions) and a perspective view.
Infantry squads (`assembly = "squad"`) are placed as their soldiers in the
squad formation. An optional faction limits the line-up to that faction.
Also prints the measured sizes so the line-up can be checked in text form.
"""

import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog, render, squad  # noqa: E402
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


def append_squad(faction, uid):
    """Appends the squad's soldiers in formation under an empty named `uid`."""
    bp = catalog.blueprint(faction, uid)
    mem = squad.members(bp)
    pts = squad.slots(bp.get("formation", "wedge"), float(bp.get("spacing", 2.0)), len(mem))
    root = bpy.data.objects.new(uid, None)
    bpy.context.scene.collection.objects.link(root)
    for (role, _), (x, y) in zip(mem, pts):
        for o in append_objects(squad.soldier_blend(faction, role)):
            if o.parent is None:
                o.parent = root
                o.location = (x, y, 0.0)
    return root


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = Path(argv[0] if argv else "build/renders/lineup.png").resolve()
    only = argv[1] if len(argv) > 1 else None
    bpy.ops.wm.read_homefile(use_empty=True)
    roots = []
    for faction, uid in catalog.all_blueprints():
        if only and faction != only:
            continue
        if squad.is_squad(faction, uid):
            roots.append(append_squad(faction, uid))
            continue
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
