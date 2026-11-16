"""Validates built unit models against the catalog and docs/ASSETS.md.

Run headless (or `just validate-assets`):
    blender -b --factory-startup --python tools/blender/validate.py -- [<faction>/<id> ...]

Without arguments every blueprint in assets-src/blueprints/ is checked. Exits
with code 1 if any model fails. Checks per model:

- .blend and .glb exist and were built from the current blueprint and
  catalog entry (source hash stored on the root object)
- root empty named after the catalog id, at the origin, identity transform
- every object: scale 1; meshes: no rotation; names from the naming table;
  parents as in the hierarchy table
- required parts and sockets per category (prompt 008: casemate vehicles,
  anti-aircraft guns and mortars by their blueprint assembly), one MG object
  per machine gun in the catalog, Socket_Muzzle on the muzzle, crew sockets
  for guns and mortars; aircraft: width = catalog wingspan, the footprint
  is only centered across (the origin lies under the center of gravity)
- only the material slots of docs/ASSETS.md; baked `Color` attribute
- triangle budget (LOD0) of the category
- length, width, height within +-2 % of the catalog (objects listed in the
  blueprint's [validate] exclude_* are left out, with their children)
- origin on the ground (lowest point z = 0) at the footprint center
- the .glb contains the same objects, the same bounds and vertex colors
"""

import re
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog  # noqa: E402
from kit.core import SLOTS, tri_count, world_bbox  # noqa: E402
from kit.sourcehash import source_hash  # noqa: E402

TOLERANCE = 0.02

# name pattern -> allowed parent name patterns ("<root>" = the root empty)
NAMES = {
    r"Hull": [r"<root>"],
    r"Carriage": [r"<root>"],
    r"Bridge": [r"Hull"],  # carried bridge of a bridge-laying tank (prompt 055)
    r"Turret": [r"Hull"],
    r"Cupola": [r"Turret", r"Hull"],
    r"Mount": [r"Carriage"],
    r"TurretSkirt": [r"Turret"],
    r"Gun": [r"Turret", r"Hull", r"Carriage", r"Mount"],
    r"Shield": [r"Carriage", r"Hull", r"Mount"],
    r"Wheel_[LR]_\d\d": [r"Hull", r"Carriage", r"<root>"],
    r"(Sprocket|Idler)_[LR]": [r"Hull"],
    r"Roller_[LR]_\d\d": [r"Hull"],
    r"Track_[LR]": [r"Hull"],
    r"Skirt_[LR]": [r"Hull"],
    r"MG_\d\d": [r"Turret", r"Hull", r"Gun", r"GunTurret_\d\d"],
    r"GunTurret_\d\d": [r"Hull"],
    r"(Bomb|Rocket)_\d\d": [r"Hull"],
    r"BayDoor_[LR]": [r"Hull"],
    r"ShadowProxy": [r"<root>"],
    r"Hatch_\d\d": [r"Hull", r"Turret", r"Cupola"],
    r"Antenna_\d\d": [r"Hull", r"Turret"],
    r"Propeller(_[LR])?": [r"<root>"],
    r"Aileron_[LR]|Elevator|Rudder(_[LR])?": [r"<root>"],
    r"Gear_(L|R|Tail|Nose)": [r"<root>"],
}
SOCKETS = re.compile(
    r"Socket_(Muzzle(_\d\d)?|Exhaust_\d\d|Hatch_\d\d|Crew_\d\d|Passenger_\d\d|Tow|Fire_\d\d|Marking_\w+|"
    r"Gun_\d\d|Bomb_\d\d|Rocket_\d\d)$"
)

REQUIRED = {
    "tank": ["Hull", "Turret", "Gun", "Track_L", "Track_R", "Wheel_L_01", "Wheel_R_01", "Sprocket_L",
             "Sprocket_R", "Idler_L", "Idler_R", "Socket_Muzzle", "Socket_Exhaust_01", "Socket_Hatch_01",
             "Socket_Fire_01"],
    "gun": ["Carriage", "Gun", "Wheel_L_01", "Wheel_R_01", "Socket_Muzzle", "Socket_Tow", "Socket_Fire_01"],
    "aircraft": ["Hull", "Gear_L", "Gear_R"],
    # prompt 008
    "casemate": ["Hull", "Gun", "Track_L", "Track_R", "Wheel_L_01", "Wheel_R_01", "Sprocket_L", "Sprocket_R",
                 "Idler_L", "Idler_R", "Socket_Muzzle", "Socket_Exhaust_01", "Socket_Fire_01"],
    "aa_gun": ["Carriage", "Mount", "Gun", "Socket_Muzzle", "Socket_Fire_01"],
    "mortar": ["Carriage", "Gun", "Socket_Muzzle", "Socket_Fire_01"],
}
# groups whose models need national markings and as many MG objects as the catalog lists
MARKED = ("tank", "casemate", "aircraft")
CREWED = ("gun", "aa_gun", "mortar")
BY_ASSEMBLY = {"casemate": "casemate", "aa_gun": "aa_gun", "mortar": "mortar"}


def group(category, assembly_name=None):
    if assembly_name in BY_ASSEMBLY:
        return BY_ASSEMBLY[assembly_name]
    if category.startswith(("tank_", "assault_gun", "self_propelled")):
        return "tank"
    if category.startswith(("gun_", "mortar_heavy")):
        return "gun"
    if category.startswith("aircraft_"):
        return "aircraft"
    return "vehicle"


def descendants(root):
    out, stack = [], list(root.children)
    while stack:
        o = stack.pop()
        out.append(o)
        stack.extend(o.children)
    return out


def excluded(obj, patterns):
    import fnmatch

    while obj is not None:
        if any(fnmatch.fnmatchcase(obj.name, p) for p in patterns):
            return True
        obj = obj.parent
    return False


def is_identity_rot(o):
    return o.rotation_mode == "XYZ" and all(abs(a) < 1e-6 for a in o.rotation_euler) or \
        o.matrix_basis.to_3x3().normalized().to_quaternion().angle < 1e-5


def check_blend(faction, uid, u, bp, errors, stats):
    root = bpy.data.objects.get(uid)
    if root is None:
        errors.append(f"no root object named {uid}")
        return None
    if root.type != "EMPTY" or root.parent is not None:
        errors.append("root must be an empty without parent")
    if root.matrix_world.translation.length > 1e-6 or not is_identity_rot(root) or \
            any(abs(s - 1) > 1e-6 for s in root.scale):
        errors.append("root must sit at the origin with identity rotation and scale")
    if root.get("source_hash") != source_hash(faction, uid):
        errors.append("model is stale: blueprint or catalog entry changed since the build (run `just unit "
                      f"{faction} {uid}`)")
    objs = descendants(root)
    strays = [o.name for o in bpy.data.objects if o is not root and o not in objs]
    if strays:
        errors.append(f"objects outside the root hierarchy: {strays}")
    names = {o.name for o in objs}

    for o in objs:
        if any(abs(s - 1) > 1e-6 for s in o.scale):
            errors.append(f"{o.name}: scale not applied {tuple(o.scale)}")
        parent = o.parent.name if o.parent is not root else "<root>"
        if o.type == "EMPTY":
            if not SOCKETS.match(o.name):
                errors.append(f"{o.name}: unknown empty (sockets are Socket_*)")
            if o.children:
                errors.append(f"{o.name}: sockets must not have children")
            continue
        if o.type != "MESH":
            errors.append(f"{o.name}: unexpected object type {o.type}")
            continue
        if not is_identity_rot(o):
            errors.append(f"{o.name}: rotation not applied")
        for pat, parents in NAMES.items():
            if re.fullmatch(pat, o.name):
                if not any(re.fullmatch(pp, parent) for pp in parents):
                    errors.append(f"{o.name}: parent {parent} not allowed ({' / '.join(parents)})")
                break
        else:
            errors.append(f"{o.name}: name not in the docs/ASSETS.md naming table")
        mats = [s.material.name if s.material else None for s in o.material_slots]
        if not mats:
            errors.append(f"{o.name}: no material")
        for m in mats:
            base = m.split(".")[0] if m else None
            if base not in SLOTS or m != base:
                errors.append(f"{o.name}: material {m!r} is not a slot ({', '.join(SLOTS)})")
        ca = o.data.color_attributes.get("Color")
        if ca is None or ca.domain != "CORNER":
            errors.append(f"{o.name}: baked corner color attribute 'Color' missing")

    g = group(u["category"], bp.get("assembly"))
    for req in REQUIRED.get(g, []):
        if req not in names:
            errors.append(f"required part or socket {req} missing")
    if g == "casemate" and "Socket_Hatch_01" not in names and "Socket_Crew_01" not in names:
        errors.append("casemate vehicle needs Socket_Hatch_01 (closed) or Socket_Crew_01 (open compartment)")
    if g in ("aa_gun", "mortar") and any(n.startswith("Wheel_") for n in names) and "Socket_Tow" not in names:
        errors.append("model has wheels but no Socket_Tow")
    if g in MARKED:
        if not any(n.startswith("Socket_Marking_") for n in names):
            errors.append("no Socket_Marking_* for national markings")
    if g in CREWED:
        for i in range(1, u.get("crew", 0) + 1):
            if f"Socket_Crew_{i:02d}" not in names:
                errors.append(f"Socket_Crew_{i:02d} missing (catalog crew {u['crew']})")
        if g == "gun" and "Shield" not in names and "armor_turret_front_mm" in u:
            errors.append("catalog lists a shield (armor_turret_front_mm) but the model has no Shield")
    mgs = sorted(n for n in names if re.fullmatch(r"MG_\d\d", n))
    need = catalog.small_arms_count(u) if g in ("tank", "casemate") else 0
    if len(mgs) < need:
        errors.append(f"{len(mgs)} MG objects, catalog lists {need} machine guns")
    for n in mgs:
        sock = f"Socket_Muzzle_{int(n[3:]) + 1:02d}"
        if sock not in names:
            errors.append(f"{n}: muzzle socket {sock} missing")

    muzzle = bpy.data.objects.get("Socket_Muzzle")
    gun = bpy.data.objects.get("Gun")
    if muzzle and gun and g == "mortar":
        # elevated tube: the socket must sit on the tube end farthest from the pivot
        piv = gun.matrix_world.translation
        far = max((gun.matrix_world @ v.co for v in gun.data.vertices), key=lambda q: (q - piv).length)
        if (muzzle.matrix_world.translation - far).length > 0.15:
            errors.append("Socket_Muzzle not at the mortar tube end")
    elif muzzle and gun:
        _, ghi = world_bbox([gun])
        if abs(muzzle.matrix_world.translation.y - ghi.y) > 0.03:
            errors.append(f"Socket_Muzzle not at the muzzle (y {muzzle.matrix_world.translation.y:.3f}, "
                          f"barrel tip {ghi.y:.3f})")

    meshes = [o for o in objs if o.type == "MESH"]
    tris = sum(tri_count(o) for o in meshes)
    budget = catalog.BUDGETS.get(u["category"])
    stats["tris"] = tris
    stats["budget"] = budget
    if budget and tris > budget:
        errors.append(f"{tris} triangles, budget {budget}")

    val = bp.get("validate", {})
    width_key = "wingspan_mm" if g == "aircraft" else "width_mm"
    for axis, key, idx in (("length", "length_mm", 1), ("width", width_key, 0), ("height", "height_mm", 2)):
        sel = [o for o in meshes if not excluded(o, val.get(f"exclude_{axis}", []))]
        lo, hi = world_bbox(sel)
        size = hi[idx] - lo[idx]
        target = catalog.mm(u, key)
        stats[axis] = (size, target)
        if abs(size - target) > TOLERANCE * target:
            errors.append(f"{axis} {size:.3f} m, catalog {target:.3f} m (more than {TOLERANCE:.0%} off)")

    lo, hi = world_bbox(meshes)
    if abs(lo.z) > 0.005:
        errors.append(f"lowest point at z = {lo.z:.3f} m, must be 0 (origin on the ground)")
    ground = []
    for o in meshes:
        mw = o.matrix_world
        ground += [mw @ v.co for v in o.data.vertices if (mw @ v.co).z < lo.z + 0.03]
    if ground:
        gx = (min(p.x for p in ground) + max(p.x for p in ground)) / 2
        gy = (min(p.y for p in ground) + max(p.y for p in ground)) / 2
        stats["footprint_center"] = (gx, gy)
        tol_y = 0.05 * catalog.mm(u, "length_mm")
        if g == "aircraft":
            tol_y = float("inf")  # origin under the center of gravity, not the wheel base
        if abs(gx) > 0.03 or abs(gy) > tol_y:
            errors.append(f"footprint center at ({gx:.3f}, {gy:.3f}), origin must be at its center "
                          f"(tolerance 0.03 / {tol_y:.2f} m)")
    return {"names": names, "bbox": (lo, hi)}


def check_glb(glb, ref, errors):
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(glb))
    objs = [o for o in bpy.data.objects]
    names = {o.name for o in objs}
    missing = ref["names"] - names
    if missing:
        errors.append(f"glb lacks objects {sorted(missing)}")
    meshes = [o for o in objs if o.type == "MESH"]
    bpy.context.view_layer.update()
    lo, hi = world_bbox(meshes)
    rlo, rhi = ref["bbox"]
    if (lo - rlo).length > 0.002 or (hi - rhi).length > 0.002:
        errors.append(f"glb bounds {tuple(round(v, 3) for v in lo)}..{tuple(round(v, 3) for v in hi)} differ "
                      "from the .blend")
    for o in meshes:
        if not o.data.color_attributes:
            errors.append(f"glb {o.name}: vertex colors missing")
            break
        for s in o.material_slots:
            if s.material and s.material.name not in SLOTS:
                errors.append(f"glb {o.name}: material {s.material.name}")


def validate(faction, uid):
    errors, stats = [], {}
    blend = catalog.blend_path(faction, uid)
    glb = catalog.glb_path(faction, uid)
    try:
        u = catalog.unit(faction, uid)
        bp = catalog.blueprint(faction, uid)
    except Exception as exc:
        return [str(exc)], stats
    for p in (blend, glb):
        if not p.exists():
            errors.append(f"{p.relative_to(catalog.REPO_ROOT)} missing (run `just unit {faction} {uid}`)")
    if errors:
        return errors, stats
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    ref = check_blend(faction, uid, u, bp, errors, stats)
    if ref:
        check_glb(glb, ref, errors)
    model = u.get("model", "")
    expected = f"assets/models/{faction}/{uid}.glb"
    if model != expected:
        errors.append(f"catalog entry model = {model!r}, expected {expected!r}")
    return errors, stats


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    units = [tuple(a.split("/", 1)) for a in argv] if argv else catalog.all_blueprints()
    failed = 0
    print(f"validate: {len(units)} model(s)")
    for faction, uid in units:
        errors, st = validate(faction, uid)
        dims = "  ".join(f"{k[0].upper()} {st[k][0]:.3f}/{st[k][1]:.3f}" for k in ("length", "width", "height")
                         if k in st)
        tri = f"{st['tris']}/{st['budget']} tris" if "tris" in st else ""
        status = "FAIL" if errors else "ok"
        print(f"  {status:4} {faction}/{uid:20} {tri:16} {dims}")
        for e in errors:
            print(f"         - {e}")
        failed += bool(errors)
    if failed:
        print(f"validate: {failed} model(s) failed")
        sys.exit(1)
    print("validate: all models ok")


try:
    main()
except SystemExit:
    raise
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
