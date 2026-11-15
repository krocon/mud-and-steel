"""Casemates: tracked vehicles whose gun sits in the hull (prompt 008).

Tank destroyers and assault guns (Jagdpanzer 38(t), StuG III, SU-85,
ISU-152) have a closed, sloped superstructure; self-propelled guns (Wespe,
M7 Priest, SU-76M) an open-topped fighting compartment. Both are the
`casemate` part on the `Hull` object, and the `casemate` assembly builds the
whole vehicle without a turret. The `Gun` is a child of `Hull` and elevates
(and traverses within its limited arc) around its trunnion.

`wall_shell` (thick open walls from stacked outlines) is shared with open
turrets (`plate_turret` with `open_top`) and other open bodies.
"""

import math

import bmesh

from .assemblies import _base_ctx, _run_list, _table, excludes
from .catalog import mm
from .registry import assembly, build_part, part
from .turret import offset_polygon


def wall_shell(mb, rings, t, mat="Paint", floor=False, wear=True):
    """Open-topped walls of thickness `t` through stacked outlines.

    `rings` are closed outlines [(x, y, z), ...] from bottom to top (same
    vertex count, convex in plan). The inner wall is the outline moved
    inward by `t`; the rim joins outer and inner at the top, and the bottom
    annulus closes the shell. `floor` = True adds a floor plate at the bottom.
    Returns the faces.
    """
    inner = []
    for r in rings:
        z = [p[2] for p in r]
        off = offset_polygon([(p[0], p[1]) for p in r], t)
        inner.append([(x, y, zz) for (x, y), zz in zip(off, z)])
    vo = [mb.verts(r) for r in rings]
    vi = [mb.verts(r) for r in inner]
    n = len(rings[0])
    faces = []
    for a, b in zip(vo, vo[1:]):
        for i in range(n):
            j = (i + 1) % n
            faces.append(mb.face([a[i], a[j], b[j], b[i]], mat))
    for a, b in zip(vi, vi[1:]):
        for i in range(n):
            j = (i + 1) % n
            faces.append(mb.face([a[i], b[i], b[j], a[j]], mat))
    for o, i_ in ((vo[-1], vi[-1]), (vo[0], vi[0])):
        for i in range(n):
            j = (i + 1) % n
            f = mb.face([o[i], o[j], i_[j], i_[i]], mat, wear=wear)
            faces.append(f)
    bmesh.ops.recalc_face_normals(mb.bm, faces=faces)
    if floor:
        z0 = rings[0][0][2]
        fl = offset_polygon([(p[0], p[1]) for p in rings[0]], t * 1.5)
        mb.extrude([(x, y, z0) for x, y in fl], (0, 0, 0.02), mat)
    return faces


def _top_plan(plan, h, side_slope, front_slope, rear_slope):
    top = offset_polygon(plan, h * math.tan(math.radians(side_slope))) if side_slope else list(plan)
    ys = [y for _, y in plan]
    mid = (max(ys) + min(ys)) / 2
    out = []
    for x, y in top:
        if y > mid:
            y -= h * math.tan(math.radians(front_slope))
        else:
            y += h * math.tan(math.radians(rear_slope))
        out.append((x, y))
    return out


@part("casemate")
def casemate(model, p, ctx):
    """Superstructure of a casemate vehicle on the hull (closed or open).

    Parameters:
    - `plan` [[x, y], ...]: outline at the base, model space (convex).
    - `top_plan` [[x, y], ...]: outline at the roof (same point count);
      default: `plan` with the slopes below applied.
    - `side_slope_deg`, `front_slope_deg`, `rear_slope_deg`: inward slope
      from vertical of the side, front (y above the plan center) and rear
      plates (default 15 / 30 / 10).
    - `z`: base height (default hull top), `height`: m or "fit" (catalog
      height; the assembly passes it), `bevel`.
    - `open`: true = open-topped fighting compartment (walls of `wall`
      thickness, default 0.025 m, with a floor), false = closed roof.
    Builds into `Hull`. Returns `casemate_top_z`, `casemate_plan`,
    `casemate_top_plan`, `casemate_z` (`hull_hatch` takes `z = "casemate"`
    for roof hatches; `hull_top_z` stays the deck height).
    """
    plan = [tuple(v) for v in p.req("plan")]
    z0 = p.get("z", ctx.get("hull_top_z", 1.0))
    h = p.get("height", "fit")
    h = ctx["casemate_height_fit"](z0) if h == "fit" else h
    if h <= 0.05:
        raise ValueError(f"casemate: no room for the superstructure (height {h:.3f} m)")
    if "top_plan" in p:
        top = [tuple(v) for v in p.get("top_plan")]
        if len(top) != len(plan):
            raise ValueError("casemate: top_plan needs as many points as plan")
    else:
        top = _top_plan(plan, h, p.get("side_slope_deg", 15.0), p.get("front_slope_deg", 30.0),
                        p.get("rear_slope_deg", 10.0))
    mb = model.mb("Hull")
    rings = [[(x, y, z0) for x, y in plan], [(x, y, z0 + h) for x, y in top]]
    if p.get("open", False):
        wall_shell(mb, rings, p.get("wall", 0.025), floor=True)
    else:
        p.get("wall")
        mb.loft(rings, "Paint", bevel=p.get("bevel", 0.015))
    return {"casemate_top_z": z0 + h, "casemate_plan": plan, "casemate_top_plan": top, "casemate_z": z0}


@assembly("casemate")
def casemate_vehicle(model, bp, u, tune):
    """Tracked vehicle without turret, gun in the hull (tank destroyers,
    assault guns, self-propelled guns).

    Blueprint sections, in build order: `[layout]`, `[running_gear]`,
    `[hull]` (plate_hull: the lower hull and the parts of the upper hull
    outside the casemate), `[casemate]` (casemate part; `height = "fit"`
    makes its roof or wall top the catalog height), `[cupola]` (optional,
    on the casemate roof: `z` defaults to the casemate top, `x`/`y` are
    model space), `[[part]]`, `[gun]` (tank_gun, `parent` defaults to Hull;
    `trunnion` = [y, z] and `x` are model space; one barrel segment "fit"
    = catalog overall length), `[[mg]]` (pintle_mg, mg with mount "ball").

    `[layout]`: `fender_overhang`, `hull_length` (only without catalog
    `length_hull_mm`).
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

    cup = bp.get("cupola")
    cupola_counts = cup is not None and "Cupola" not in excludes(bp, "height")
    cup_h = (cup["height"] - 0.02) if cupola_counts else 0.0
    ctx["casemate_height_fit"] = lambda z0: ctx["height"] - z0 - cup_h
    name, t = _table(bp["casemate"], "[casemate]")
    build_part(name, model, t, ctx, where="[casemate]")
    # cupola and pintle parts use the turret context: the casemate roof at the origin
    ctx["turret_y"] = 0.0
    ctx["turret_z"] = ctx["casemate_top_z"]
    ctx["turret_top_z"] = ctx["casemate_top_z"]
    if cup is not None:
        build_part("cupola", model, dict(cup, parent="Hull"), ctx, where="[cupola]")

    _run_list(model, bp, "part", ctx)

    lo, _ = model.extent(excludes(bp, "length"))
    ctx["overall_front_y"] = lo.y + ctx["length"]
    ctx["turret_z"] = 0.0  # gun trunnion in model space
    name, t = _table(bp["gun"], "[gun]")
    t.setdefault("parent", "Hull")
    build_part(name, model, t, ctx, where="[gun]")
    _run_list(model, bp, "mg", ctx)
    return ctx
