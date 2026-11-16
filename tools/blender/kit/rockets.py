"""Rocket launcher parts (prompt 062): tube bundles and rail racks of vehicle
rocket launchers (15 cm Panzerwerfer 42, BM-13-16, T34 Calliope).

One generic part, `rocket_rack`: a block of `rows` x `cols` tubes or rails,
raised by `elevation_deg` around its rear edge and turned by `yaw_deg`,
with an optional frame around it. Parented to `Hull` (fixed racks) or
`Turret` (racks that turn with the turret). Adds `Socket_Muzzle` at the
front center of the block: the presentation fires its launch flash there
(prompt 019 effects) for every rocket of a salvo.
"""

import math

from mathutils import Matrix, Vector

from .registry import part


def _frame(pivot, yaw_deg, elevation_deg):
    """Matrix from rack space (x right, y along the tubes, z up) to model space."""
    rot = Matrix.Rotation(math.radians(yaw_deg), 4, "Z") @ Matrix.Rotation(math.radians(elevation_deg), 4, "X")
    return Matrix.Translation(Vector(pivot)) @ rot


@part("rocket_rack")
def rocket_rack(model, p, ctx):
    """Block of launch tubes or rails.

    Parameters:
    - `pivot` [x, y, z]: rear end, center line, bottom of the block (model space)
    - `rows`, `cols`: tubes per column and per row
    - `pitch` [dx, dz]: center distance of the tubes sideways and upwards
    - `length`: tube or rail length
    - `diameter`: tube outer diameter (rails: rocket diameter, the rail sits
      under it)
    - `kind`: "tube" (round tubes, default) or "rail" (open rails with the
      rockets lying on top, as on the BM-13)
    - `elevation_deg` (default 0): front raised around the rear edge
    - `yaw_deg` (default 0): turned around Z (0: tubes point to +Y)
    - `skip`: list of [row, col] left empty (row 0 at the bottom)
    - `frame` (default true): two side plates and a band at both ends
    - `rockets` (default false; rails only): rocket bodies on the rails
    - `segments` (default 8): sides of a tube
    - `parent` (default "Hull"), `material` (default "Paint"); with
      `parent = "Turret"` and no turret yet, `turret_pivot` [x, y, z] (center
      of the turntable) creates `Turret` on `Hull`: the launcher of the
      Panzerwerfer 42 turns on its roof like a turret
    - `muzzle` (default true): `Socket_Muzzle` at the front center;
      `muzzle_name` names it differently (the Calliope's tank gun has the
      `Socket_Muzzle`, its launcher `Socket_Rocket_01`)
    """
    parent = p.get("parent", "Hull")
    if parent == "Turret" and not model.has("Turret"):
        model.mb("Turret", parent="Hull", pivot=tuple(p.req("turret_pivot")))
    elif "turret_pivot" in p:
        p.get("turret_pivot")
    mb = model.mb(parent)
    m = _frame(p.req("pivot"), p.get("yaw_deg", 0.0), p.get("elevation_deg", 0.0))
    rows, cols = int(p.req("rows")), int(p.req("cols"))
    dx, dz = p.req("pitch")
    length = p.req("length")
    d = p.req("diameter")
    kind = p.get("kind", "tube")
    if kind not in ("tube", "rail"):
        raise ValueError(f"rocket_rack: unknown kind {kind!r}")
    skip = {tuple(s) for s in p.get("skip", [])}
    mat = p.get("material", "Paint")
    rockets = p.get("rockets", False)
    seg = int(p.get("segments", 8))

    def at(x, y, z):
        return tuple(m @ Vector((x, y, z)))

    half_w = (cols - 1) * dx / 2
    for r in range(rows):
        for c in range(cols):
            if (r, c) in skip:
                continue
            x = c * dx - half_w
            z = r * dz + d / 2
            if kind == "tube":
                mb.cylinder(at(x, 0.0, z), at(x, length, z), d / 2, seg=seg, mat=mat, smooth=False)
            else:
                # I-section rail under the rocket: a flat beam on a web.
                rail_z = z - d / 2 - 0.02
                mb.cylinder(at(x, 0.0, rail_z), at(x, length, rail_z), 0.03, seg=4, mat="Metal",
                            smooth=False, phase=math.pi / 4)
                if rockets:
                    mb.cylinder(at(x, 0.15, z), at(x, length - 0.05, z), d / 2, seg=seg, mat=mat,
                                smooth=False)
                    mb.cylinder(at(x, length - 0.05, z), at(x, length + 0.12, z), d / 2, r1=d / 6, seg=seg,
                                mat=mat, smooth=False, cap1=False)
    top = (rows - 1) * dz + d
    if p.get("frame", True):
        side = half_w + d / 2 + 0.03
        for sx in (-side, side):
            # side frame: a beam along the tubes at the bottom and the top
            for z in (-0.02, top + 0.02):
                mb.cylinder(at(sx, 0.0, z), at(sx, length, z), 0.035, seg=4, mat="Paint", smooth=False,
                            phase=math.pi / 4)
        for y in (0.08, length - 0.08):
            # end bands: bottom and top cross bars
            for z in (-0.02, top + 0.02):
                mb.cylinder(at(-side, y, z), at(side, y, z), 0.03, seg=4, mat="Metal", smooth=False,
                            phase=math.pi / 4)
    muzzle_name = p.get("muzzle_name", "Socket_Muzzle")
    if p.get("muzzle", True):
        model.socket(muzzle_name, parent, at(0.0, length, top / 2))
    return {"rack_top": at(0.0, length, top)}
