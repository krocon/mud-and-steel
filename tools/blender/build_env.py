"""Builds environment objects (prompt 007) from their blueprints.

    blender -b --factory-startup --python-exit-code 1 --python tools/blender/build_env.py -- [<id> ...] [--render DIR]

Without ids every environment blueprint (`assembly = "env"` in
assets-src/blueprints/<faction>/) is built: the neutral objects and, since
prompt 008, faction fortifications (`de_bunker_tobruk`, `*_minefield_*`; the
faction is the id prefix). Writes assets-src/blender/<faction>/<id>.blend and
godot/assets/models/<faction>/<id>.glb; with --render also
DIR/<id>_turntable.png. See docs/ASSETS.md ("Environment objects") for the
blueprint format.
"""

import argparse
import json
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import buildings, catalog, env, forts, load_parts, parts_neutral, vegetation  # noqa: E402,F401

load_parts()  # faction modules (parts_<faction>_guns.py) may register env generators
from kit.build import clear_scene, export_glb  # noqa: E402
from kit.core import descendants  # noqa: E402
from kit.registry import Params  # noqa: E402
from kit.sourcehash import source_hash  # noqa: E402

META = ("name", "part", "size", "dims_m", "fit", "role", "damage_of", "center", "height_m")


def faction_of(uid):
    """Catalog faction of an environment object: the id prefix (de_, su_, us_), else neutral."""
    return catalog.env_faction(uid)


def variant_box(u, vt):
    if "dims_m" in vt:
        L, W, H = vt["dims_m"]
        return env.Box(L, W, H)
    s = vt.get("size", [1.0, 1.0, 1.0])
    return env.Box(catalog.mm(u, "length_mm") * s[0], catalog.mm(u, "width_mm") * s[1],
                   catalog.env_height(u, vt) * s[2])


def build_env_model(uid):
    FACTION = faction_of(uid)
    u = catalog.unit(FACTION, uid)
    bp = catalog.blueprint(FACTION, uid)
    if bp.get("assembly") != "env":
        raise ValueError(f"{uid}: not an environment blueprint")
    sunken = bool(bp.get("sunken", False))
    model = env.EnvModel(uid)
    variants = bp.get("variant", [])
    if not variants:
        raise ValueError(f"{uid}: blueprint has no [[variant]]")
    info = []
    for i, vt in enumerate(variants):
        name = vt.get("name")
        part = vt.get("part")
        where = f"[[variant]] #{i + 1} ({name})"
        if not name or not part:
            raise KeyError(f"{where}: needs name and part")
        if part not in env.ENV_PARTS:
            raise KeyError(f"{where}: unknown env part {part!r}; known: {', '.join(sorted(env.ENV_PARTS))}")
        box = variant_box(u, vt)
        p = Params({k: v for k, v in vt.items() if k not in META}, where)
        env.ENV_PARTS[part](model, name, p, box)
        bad = p.unused()
        if bad:
            raise KeyError(f"{where}: unknown parameters {bad} for env part {part}")
        names = model.variant_parts(name)
        if not names:
            raise RuntimeError(f"{where}: generator built nothing")
        center = vt.get("center", "ground" if u["category"] in ("env_tree", "env_bush") else "bounds")
        if vt.get("fit", True):
            env.fit(model, names, box, sunken, center)
        else:
            env.ground(model, names) if not sunken else None
        far = f"{name}_Far"
        if far in model.parts:
            env.fit(model, [far], box, sunken, center)
            model.part_props.setdefault(far, {}).update({"env_far_of": name})
        props = model.part_props.setdefault(name, {})
        props.update({"env_variant": name, "env_role": vt.get("role", "variant")})
        if "damage_of" in vt:
            props["env_damage_of"] = vt["damage_of"]
        info.append({"name": name, "role": vt.get("role", "variant"), "far": far in model.parts})
    model.props.update(catalog.summary(u))
    model.props["env_kit_version"] = env.ENV_KIT_VERSION
    model.props["env_variants"] = json.dumps(info)
    model.props["env_sunken"] = sunken
    model.props["blueprint"] = f"assets-src/blueprints/{FACTION}/{uid}.toml"
    model.props["source_hash"] = source_hash(FACTION, uid)
    coll = bpy.data.collections.new(uid)
    bpy.context.scene.collection.children.link(coll)
    root = model.finish(coll)
    bpy.context.view_layer.update()
    objs = [root] + descendants(root)
    env.bake_env(objs)
    for o in objs:
        if o.type == "MESH":
            env.foliage_normals(o)
    return root


def build_and_save(uid, render_dir=None):
    FACTION = faction_of(uid)
    bpy.ops.wm.read_homefile(use_empty=True)
    clear_scene()
    root = build_env_model(uid)
    blend = catalog.blend_path(FACTION, uid)
    glb = catalog.glb_path(FACTION, uid)
    blend.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend), compress=True)
    export_glb(glb)
    print(f"build_env: {uid} -> {glb.relative_to(catalog.REPO_ROOT)}")
    if render_dir:
        from kit import render

        render.turntable(root, Path(render_dir) / f"{uid}_turntable.png")
    return root


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--render", type=Path)
    args = ap.parse_args(argv)
    ids = args.ids or [uid for _, uid in catalog.all_blueprints(env=True)]
    for uid in ids:
        build_and_save(uid, args.render)
    print(f"build_env: {len(ids)} object(s) built")


try:
    main()
except Exception as exc:
    import traceback

    traceback.print_exc()
    print(f"build_env: FAILED: {exc}", file=sys.stderr)
    sys.exit(1)
