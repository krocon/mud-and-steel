"""Builds a unit from its blueprint into the current Blender scene."""

import bpy

from . import catalog, load_parts
from .assemblies import excludes
from .bake import bake_vehicle
from .core import Model, descendants
from .registry import ASSEMBLIES
from .sourcehash import source_hash

KIT_VERSION = 2


def clear_scene():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    for coll in list(bpy.data.collections):
        bpy.data.collections.remove(coll)
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.cameras, bpy.data.lights):
        for item in list(block):
            if item.users == 0:
                block.remove(item)


def _free(model):
    for p in model.parts.values():
        p["mb"].bm.free()


def build_model(faction, unit_id, collection=None):
    """Builds the unit and returns its root object.

    Runs the assembly up to three times: after each pass the overall height
    is measured (without the blueprint's height exclusions) and the
    difference to the catalog is fed back as `tune["height"]`, which the
    assembly applies to its free height value (turret roof, shield top).
    """
    load_parts()
    u = catalog.unit(faction, unit_id)
    bp = catalog.blueprint(faction, unit_id)
    asm_name = bp.get("assembly")
    if asm_name not in ASSEMBLIES:
        raise KeyError(f"{unit_id}: unknown assembly {asm_name!r}; known: {', '.join(sorted(ASSEMBLIES))}")
    asm = ASSEMBLIES[asm_name]
    target_h = catalog.mm(u, "height_mm")
    tune = {}
    model = None
    for _ in range(3):
        if model is not None:
            _free(model)
        model = Model(unit_id)
        asm(model, bp, u, tune)
        _, hi = model.extent(excludes(bp, "height"))
        err = target_h - hi.z
        if abs(err) < 0.0005:
            break
        tune["height"] = tune.get("height", 0.0) + err
    model.props.update(catalog.summary(u))
    model.props["kit_version"] = KIT_VERSION
    model.props["blueprint"] = f"assets-src/blueprints/{faction}/{unit_id}.toml"
    model.props["source_hash"] = source_hash(faction, unit_id)
    if collection is None:
        collection = bpy.data.collections.new(unit_id)
        bpy.context.scene.collection.children.link(collection)
    root = model.finish(collection)
    bpy.context.view_layer.update()
    bake_vehicle([root] + descendants(root))
    return root


def export_glb(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    result = bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        export_apply=True,
        export_yup=True,
        export_materials="EXPORT",
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=False,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        use_selection=False,
    )
    if "FINISHED" not in result:
        raise RuntimeError(f"glTF export failed: {result}")
