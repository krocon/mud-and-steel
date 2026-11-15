"""Armored cars and scout cars: wheels, armored body, optional turret (prompt 008).

Sd.Kfz. 222, BA-64B, M8 Greyhound and the M3A1 scout car. The body is a
`section_hull` (faceted plates, optional open compartment); the turret,
cupola, gun and machine guns are the tank parts.
"""

from .assemblies import _base_ctx, _run_list, _table, excludes
from .catalog import mm
from .registry import assembly, build_part


@assembly("armored_car")
def armored_car(model, bp, u, tune):
    """Wheeled armored vehicle with optional turret.

    Blueprint sections, in build order: `[layout]`, `[[axle]]` (wheel_axle
    tables front to rear, numbered automatically), `[hull]` (usually
    `section_hull`; use `height = "keep"` when a turret sets the height),
    `[turret]` (optional: plate_turret / cast_turret, `height = "fit"` =
    catalog height), `[cupola]` (optional), `[[part]]`, `[gun]` (optional
    tank_gun; one barrel segment "fit" = catalog overall length), `[[mg]]`.

    `[layout]`: `turret_y`, `turret_ring_diameter`, `turret_z` (default hull
    top), `hull_length` (default: catalog `length_hull_mm`, else the
    catalog length). Afterwards the model is moved so that the origin lies
    midway between the first and the last axle and the tires touch z = 0.
    """
    ctx, layout = _base_ctx(bp, u, tune)
    if "hull_length" in layout:
        ctx["hull_length"] = layout["hull_length"]
    else:
        ctx["hull_length"] = mm(u, "length_hull_mm") if "length_hull_mm" in u else mm(u, "length_mm")
    model.mb("Hull")
    axles = bp.get("axle", [])
    if not axles:
        raise KeyError("[[axle]]: an armored car needs at least one axle")
    for i, t in enumerate(axles):
        t = dict(t)
        name = t.pop("part", "wheel_axle")
        t.setdefault("index", i + 1)
        build_part(name, model, t, ctx, where=f"[[axle]] #{i + 1}")
    name, t = _table(bp["hull"], "[hull]")
    build_part(name, model, t, ctx, where="[hull]")

    if "turret" in bp:
        ctx["turret_y"] = layout["turret_y"]
        ctx["turret_z"] = layout.get("turret_z", ctx["hull_top_z"])
        ctx["turret_ring_r"] = layout.get("turret_ring_diameter", 1.2) / 2
        cup = bp.get("cupola")
        cupola_counts = cup is not None and "Cupola" not in excludes(bp, "height")
        turret = bp["turret"]
        if turret.get("height", "fit") == "fit":
            cup_h = (cup["height"] - 0.02) if cupola_counts else 0.0
            ctx["turret_height"] = ctx["height"] - ctx["turret_z"] - cup_h
        else:
            ctx["turret_height"] = turret["height"]
        name, t = _table(turret, "[turret]")
        build_part(name, model, t, ctx, where="[turret]")
        if cup is not None:
            build_part("cupola", model, cup, ctx, where="[cupola]")
    else:
        ctx.setdefault("turret_y", 0.0)
        ctx.setdefault("turret_z", 0.0)
        ctx.setdefault("turret_top_z", ctx["hull_top_z"])

    _run_list(model, bp, "part", ctx)
    if "gun" in bp:
        lo, _ = model.extent(excludes(bp, "length"))
        ctx["overall_front_y"] = lo.y + ctx["length"]
        name, t = _table(bp["gun"], "[gun]")
        build_part(name, model, t, ctx, where="[gun]")
    _run_list(model, bp, "mg", ctx)
    ys = [a["y"] for a in axles]
    lo, _ = model.extent()
    model.translate((0.0, -(max(ys) + min(ys)) / 2, -lo.z))
    return ctx
