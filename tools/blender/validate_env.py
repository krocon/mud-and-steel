"""Validates environment objects (prompt 007) against catalog and blueprint.

    blender -b --factory-startup --python-exit-code 1 --python tools/blender/validate_env.py -- [<id> ...]

Part of `just validate-assets`. Checks per object:

- .blend and .glb exist and are current (source hash of blueprint + catalog)
- root empty named after the id at the origin; `env_kit_version` extras
- one direct child per blueprint variant: a mesh, or an empty whose meshes are
  `<Variant>_<Part>` (Walls, Roof, Floor, Debris, Tower, Deck, Pier, Rail);
  every tree variant has a `<Variant>_Far` mesh; empties below are Socket_*
- scale 1 and no rotation everywhere; only env material slots; baked `Color`
- triangle budget per variant (docs/ASSETS.md), far meshes <= 200
- size of every variant within +-2 % of its box (catalog x `size`, or
  `dims_m`); damaged variants (`damage_of`) not larger than their intact one;
  sunken objects (trenches) are measured below z = 0
- lowest point at z = 0 (sunken: at -height), ground contact centered
- the .glb has the same objects, bounds, vertex colors and materials
"""

import re
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog, env  # noqa: E402
from kit.core import tri_count, world_bbox  # noqa: E402
from kit.sourcehash import source_hash  # noqa: E402

FACTION = "neutral"
TOL = 0.02
PARTS = ("Walls", "Roof", "Floor", "Debris", "Tower", "Deck", "Pier", "Rail")
SOCKET = re.compile(r"Socket_(Wire|Fire)_\d\d$")


def kids(o):
    out, stack = [], list(o.children)
    while stack:
        c = stack.pop()
        out.append(c)
        stack.extend(c.children)
    return out


def box_of(u, vt):
    if "dims_m" in vt:
        return tuple(vt["dims_m"])
    s = vt.get("size", [1, 1, 1])
    return (catalog.mm(u, "length_mm") * s[0], catalog.mm(u, "width_mm") * s[1], catalog.mm(u, "height_mm") * s[2])


def measure(meshes, sunken):
    if not sunken:
        lo, hi = world_bbox(meshes)
        return lo, hi, (hi.y - lo.y, hi.x - lo.x, hi.z - lo.z)
    pts = []
    for o in meshes:
        mw = o.matrix_world
        pts += [mw @ v.co for v in o.data.vertices]
    below = [p for p in pts if p.z <= 1e-4]
    lo = Vector((min(p.x for p in below), min(p.y for p in below), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in below), max(p.y for p in below), 0.0))
    return lo, hi, (hi.y - lo.y, hi.x - lo.x, -lo.z)


def check_blend(uid, u, bp, errors, stats):
    root = bpy.data.objects.get(uid)
    if root is None or root.type != "EMPTY" or root.parent is not None:
        errors.append(f"no root empty named {uid}")
        return None
    if root.matrix_world.translation.length > 1e-6 or any(abs(s - 1) > 1e-6 for s in root.scale) or \
            root.matrix_world.to_3x3().to_quaternion().angle > 1e-5:
        errors.append("root must sit at the origin with identity rotation and scale")
    if root.get("env_kit_version") != env.ENV_KIT_VERSION:
        errors.append("root lacks env_kit_version (not built by build_env.py)")
    if root.get("source_hash") != source_hash(FACTION, uid):
        errors.append(f"model is stale: blueprint or catalog entry changed (run `just env {uid}`)")
    objs = kids(root)
    strays = [o.name for o in bpy.data.objects if o is not root and o not in objs]
    if strays:
        errors.append(f"objects outside the root hierarchy: {strays}")
    sunken = bool(bp.get("sunken", False))
    is_tree = u["category"] == "env_tree"
    budget = env.budget(u)
    stats["budget"] = budget
    variants = bp.get("variant", [])
    names = {vt["name"] for vt in variants}
    allowed_direct = names | {f"{n}_Far" for n in names}
    for o in root.children:
        if o.name not in allowed_direct and not SOCKET.match(o.name):
            errors.append(f"{o.name}: not a variant of the blueprint")
    for o in objs:
        if any(abs(s - 1) > 1e-6 for s in o.scale):
            errors.append(f"{o.name}: scale not applied")
        if o.type == "MESH" and o.matrix_basis.to_3x3().to_quaternion().angle > 1e-5:
            errors.append(f"{o.name}: rotation not applied")
        if o.type == "EMPTY":
            if o.name not in names and not SOCKET.match(o.name):
                errors.append(f"{o.name}: unknown empty")
            continue
        if o.type != "MESH":
            errors.append(f"{o.name}: unexpected type {o.type}")
            continue
        for s in o.material_slots:
            m = s.material.name if s.material else None
            if m not in env.ALLOWED_SLOTS:
                errors.append(f"{o.name}: material {m!r} is not an environment slot")
        ca = o.data.color_attributes.get("Color")
        if ca is None or ca.domain != "CORNER":
            errors.append(f"{o.name}: baked corner color attribute 'Color' missing")
    sizes = {}
    tris_all = []
    for vt in variants:
        n = vt["name"]
        node = bpy.data.objects.get(n)
        if node is None or node.parent is not root:
            errors.append(f"variant {n} missing below the root")
            continue
        if node.type == "MESH":
            meshes = [node]
        else:
            meshes = [c for c in node.children if c.type == "MESH"]
            for c in node.children:
                if c.type == "MESH" and not any(c.name == f"{n}_{p}" for p in PARTS):
                    errors.append(f"{c.name}: part name must be {n}_<{'|'.join(PARTS)}>")
            if not meshes:
                errors.append(f"variant {n}: no meshes")
                continue
        tris = sum(tri_count(m) for m in meshes)
        tris_all.append(tris)
        if tris > budget:
            errors.append(f"variant {n}: {tris} triangles, budget {budget}")
        far = bpy.data.objects.get(f"{n}_Far")
        if is_tree and far is None:
            errors.append(f"tree variant {n}: {n}_Far missing")
        if far is not None and tri_count(far) > env.FAR_BUDGET:
            errors.append(f"{far.name}: {tri_count(far)} triangles, far budget {env.FAR_BUDGET}")
        lo, hi, size = measure(meshes, sunken)
        sizes[n] = size
        if "damage_of" in vt:
            ref = sizes.get(vt["damage_of"])
            if ref is None:
                errors.append(f"variant {n}: damage_of {vt['damage_of']!r} must be an earlier variant")
            else:
                for ax, a, b in zip("LWH", size, ref):
                    lim = b * (1.0 + TOL) if ax == "H" else b * 1.15
                    if a > lim:
                        errors.append(f"variant {n}: {ax} {a:.3f} larger than {vt['damage_of']} ({b:.3f})")
        else:
            target = box_of(u, vt)
            for ax, a, b in zip("LWH", size, target):
                if abs(a - b) > TOL * b:
                    errors.append(f"variant {n}: {ax} {a:.3f} m, expected {b:.3f} m (more than 2 % off)")
        if not sunken:
            if abs(lo.z) > 0.005:
                errors.append(f"variant {n}: lowest point at z = {lo.z:.3f}, must be 0")
            ground = []
            for m in meshes:
                mw = m.matrix_world
                ground += [mw @ v.co for v in m.data.vertices if (mw @ v.co).z < lo.z + 0.03]
            # chained segments touch the ground only with their start post
            if ground and "damage_of" not in vt and vt.get("role") != "segment":
                gx = (min(p.x for p in ground) + max(p.x for p in ground)) / 2
                gy = (min(p.y for p in ground) + max(p.y for p in ground)) / 2
                tol = 0.05 + 0.08 * max(size[0], size[1])
                if abs(gx) > tol or abs(gy) > tol:
                    errors.append(f"variant {n}: ground contact centered at ({gx:.2f}, {gy:.2f}), tolerance {tol:.2f}")
        elif abs(-lo.z - box_of(u, vt)[2]) > TOL * box_of(u, vt)[2] and "damage_of" not in vt:
            errors.append(f"variant {n}: bottom at z = {lo.z:.3f}, expected -{box_of(u, vt)[2]:.3f}")
    stats["tris"] = max(tris_all) if tris_all else 0
    stats["variants"] = len(variants)
    stats["size"] = sizes.get(variants[0]["name"]) if variants else None
    stats["target"] = box_of(u, variants[0]) if variants else None
    lo, hi = world_bbox([o for o in objs if o.type == "MESH"])
    return {"names": {o.name for o in objs}, "bbox": (lo, hi)}


def check_glb(glb, ref, errors):
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(glb))
    names = {o.name for o in bpy.data.objects}
    missing = ref["names"] - names
    if missing:
        errors.append(f"glb lacks objects {sorted(missing)}")
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    bpy.context.view_layer.update()
    lo, hi = world_bbox(meshes)
    rlo, rhi = ref["bbox"]
    if (lo - rlo).length > 0.003 or (hi - rhi).length > 0.003:
        errors.append("glb bounds differ from the .blend")
    for o in meshes:
        if not o.data.color_attributes:
            errors.append(f"glb {o.name}: vertex colors missing")
        for s in o.material_slots:
            if s.material and s.material.name not in env.ALLOWED_SLOTS:
                errors.append(f"glb {o.name}: material {s.material.name}")


def validate(uid):
    errors, stats = [], {}
    try:
        u = catalog.unit(FACTION, uid)
        bp = catalog.blueprint(FACTION, uid)
    except Exception as exc:
        return [str(exc)], stats
    blend, glb = catalog.blend_path(FACTION, uid), catalog.glb_path(FACTION, uid)
    for p in (blend, glb):
        if not p.exists():
            errors.append(f"{p.relative_to(catalog.REPO_ROOT)} missing (run `just env {uid}`)")
    if errors:
        return errors, stats
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    ref = check_blend(uid, u, bp, errors, stats)
    if ref:
        check_glb(glb, ref, errors)
    expected = f"assets/models/{FACTION}/{uid}.glb"
    if u.get("model", "") != expected:
        errors.append(f"catalog entry model = {u.get('model', '')!r}, expected {expected!r}")
    return errors, stats


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ids = argv or [uid for _, uid in catalog.all_blueprints(env=True)]
    # every neutral catalog entry needs an environment model
    missing = sorted(set(catalog.load_faction(FACTION)) - set(ids)) if not argv else []
    failed = 0
    print(f"validate_env: {len(ids)} object(s)")
    for uid in ids:
        errors, st = validate(uid)
        dims = ""
        if st.get("size"):
            dims = "  ".join(f"{k} {a:.2f}/{b:.2f}" for k, a, b in zip("LWH", st["size"], st["target"]))
        tri = f"{st.get('tris', 0)}/{st.get('budget', 0)} tris" if "tris" in st else ""
        print(f"  {'FAIL' if errors else 'ok':4} {uid:24} {st.get('variants', 0)} var  {tri:15} {dims}")
        for e in errors:
            print(f"         - {e}")
        failed += bool(errors)
    for uid in missing:
        print(f"  FAIL {uid:24} no environment blueprint in assets-src/blueprints/neutral/")
        failed += 1
    if failed:
        print(f"validate_env: {failed} object(s) failed")
        sys.exit(1)
    print("validate_env: all objects ok")


try:
    main()
except SystemExit:
    raise
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
