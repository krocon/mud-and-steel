"""Whole-unit builders. Each assembly reads the main dimensions from the
catalog, runs the blueprint's parts in a fixed order and fits the remaining
free values (barrel length, turret height, trail length) to the catalog.

Assemblies take a `tune` dict: corrections the generator measured in a first
pass (e.g. hatches that stick out above the turret roof) and applies in the
second pass. See `build.build_model`.
"""

from .catalog import mm
from .registry import assembly, build_part


def _table(t, where):
    t = dict(t)
    name = t.pop("part", None)
    if not name:
        raise KeyError(f"{where}: missing 'part'")
    return name, t


def _run_list(model, bp, key, ctx):
    for i, t in enumerate(bp.get(key, [])):
        name, params = _table(t, f"[[{key}]] #{i + 1}")
        build_part(name, model, params, ctx, where=f"[[{key}]] #{i + 1} ({name})")


def excludes(bp, axis):
    return list(bp.get("validate", {}).get(f"exclude_{axis}", []))


def _base_ctx(bp, u, tune):
    layout = bp.get("layout", {})
    # aircraft have no catalog width, only the wingspan
    width = mm(u, "width_mm") if "width_mm" in u or "wingspan_mm" not in u else mm(u, "wingspan_mm")
    ctx = {
        "unit": u,
        "length": mm(u, "length_mm"),
        "width": width,
        "height": mm(u, "height_mm") + tune.get("height", 0.0),
        "half_width": width / 2,
        "layout": layout,
    }
    return ctx, layout


@assembly("tank")
def tank(model, bp, u, tune):
    """Tracked vehicle with turret (tanks, also usable for turreted SPGs).

    Blueprint sections, in build order: `[layout]`, `[running_gear]`,
    `[hull]`, `[turret]`, `[cupola]` (optional), `[[part]]` (any parts),
    `[gun]`, `[[mg]]`.

    `[layout]`: `turret_y` (ring center), `turret_ring_diameter`,
    `turret_z` (default: hull top), `fender_overhang` (fenders outside the
    tracks), `hull_length` (only if the catalog has no `length_hull_mm`).
    """
    ctx, layout = _base_ctx(bp, u, tune)
    if "length_hull_mm" in u:
        if "hull_length" in layout:
            raise ValueError("[layout] hull_length: the catalog already has length_hull_mm")
        ctx["hull_length"] = mm(u, "length_hull_mm")
    else:
        ctx["hull_length"] = layout["hull_length"]
    ctx["track_outer_x"] = ctx["half_width"] - layout.get("fender_overhang", 0.0)

    model.mb("Hull")
    name, t = _table(bp["running_gear"], "[running_gear]")
    build_part(name, model, t, ctx, where="[running_gear]")
    name, t = _table(bp["hull"], "[hull]")
    build_part(name, model, t, ctx, where="[hull]")

    ctx["turret_y"] = layout["turret_y"]
    ctx["turret_z"] = layout.get("turret_z", ctx["hull_top_z"])
    ctx["turret_ring_r"] = layout.get("turret_ring_diameter", 1.6) / 2
    cup = bp.get("cupola")
    ex_h = excludes(bp, "height")
    cupola_counts = cup is not None and "Cupola" not in ex_h
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

    _run_list(model, bp, "part", ctx)

    lo, _ = model.extent(excludes(bp, "length"))
    ctx["overall_front_y"] = lo.y + ctx["length"]
    name, t = _table(bp["gun"], "[gun]")
    build_part(name, model, t, ctx, where="[gun]")
    _run_list(model, bp, "mg", ctx)
    return ctx


@assembly("towed_gun")
def towed_gun(model, bp, u, tune):
    """Towed gun: `[gun]` (field_gun), `[carriage]` (split_trail_carriage),
    `[shield]` (gun_shield, optional), `[[part]]`.

    The axle is built at y = 0, the trails are fitted so that muzzle to spade
    end equals the catalog length, then the model is moved so that the origin
    lies midway between axle and spades (center of the footprint).
    """
    ctx, layout = _base_ctx(bp, u, tune)
    model.mb("Carriage")
    name, t = _table(bp["gun"], "[gun]")
    build_part(name, model, t, ctx, where="[gun]")
    ctx["trail_length_fit"] = lambda ay: ctx["length"] - (ctx["gun_tip_y"] - ay)
    name, t = _table(bp["carriage"], "[carriage]")
    t.setdefault("axle_y", 0.0)
    build_part(name, model, t, ctx, where="[carriage]")
    if "shield" in bp:
        name, t = _table(bp["shield"], "[shield]")
        t.setdefault("width", ctx["width"])
        build_part(name, model, t, ctx, where="[shield]")
    _run_list(model, bp, "part", ctx)
    center = ctx["carriage_rear_y"] / 2  # midway between axle (y = 0) and spades
    model.translate((0.0, -center, 0.0))
    return ctx


@assembly("aircraft")
def aircraft(model, bp, u, tune):
    """Aircraft: `[fuselage]`, `[wing]`, `[tail]`, `[[propeller]]`,
    `[canopy]`, `[landing_gear]`, `[[part]]`. `[layout] cg_chord` (fraction
    of the root chord behind the wing leading edge, default 0.3) places the
    origin under the center of gravity.
    """
    ctx, layout = _base_ctx(bp, u, tune)
    ctx["wingspan"] = mm(u, "wingspan_mm")
    model.mb("Hull")
    for key in ("fuselage", "wing", "tail", "canopy", "landing_gear"):
        if key in bp:
            name, t = _table(bp[key], f"[{key}]")
            build_part(name, model, t, ctx, where=f"[{key}]")
    _run_list(model, bp, "propeller", ctx)
    _run_list(model, bp, "part", ctx)
    cg = ctx["wing_y"] - layout.get("cg_chord", 0.3) * bp["wing"]["root_chord"]
    lo, _ = model.extent()
    model.translate((0.0, -cg, -lo.z))
    return ctx
