"""Soviet faction parts (prefix `su.`)."""

import math

from .registry import part


def _glacis_frame(ctx, y):
    """Point and normal on the sloped front plate of the upper hull at y."""
    upper = ctx["hull_upper"]
    front = max(upper, key=lambda p: (p[0], -p[1]))
    top = max((p for p in upper if p[1] >= max(z for _, z in upper) - 1e-6), key=lambda p: p[0])
    (y0, z0), (y1, z1) = front, top
    t = (y - y0) / (y1 - y0)
    z = z0 + (z1 - z0) * t
    ny, nz = (z1 - z0), -(y1 - y0)
    ln = math.hypot(ny, nz)
    return z, (ny / ln, nz / ln), math.atan2(y0 - y1, z1 - z0)


@part("su.glacis_hatch")
def glacis_hatch(model, p, ctx):
    """Driver's hatch in the 60 degree glacis (T-34), own object with socket.

    Parameters: `x`, `y` (center), `size` [x, along the plate].
    """
    x, y = p.req("x"), p.req("y")
    sx, sl = p.get("size", [0.55, 0.6])
    z, (ny, nz), _ = _glacis_frame(ctx, y)
    # plate corners along the glacis direction
    dy, dz = -nz, ny  # along the plate, pointing up the slope
    pts = []
    for a, b in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
        pts.append((x + a * sx / 2, y + b * dy * sl / 2, z + b * dz * sl / 2))
    hinge = (x, y + dy * sl / 2, z + dz * sl / 2)
    name = model.next_index("Hatch_")
    mb = model.mb(name, parent="Hull", pivot=hinge)
    mb.extrude(pts, (0, ny * 0.05, nz * 0.05), "Paint", bevel=0.008)
    for k in (-1, 1):
        mb.box((x + k * 0.12, y + ny * 0.06, z + nz * 0.06), (0.1, 0.05, 0.05), "Glass")
    model.socket(model.next_index("Socket_Hatch_"), "Hull", (x, y, z))
    return {}


@part("su.glacis_ball_mg")
def glacis_ball_mg(model, p, ctx):
    """Bow DT machine gun in an armored ball on the glacis. Parameters: `x`, `y`."""
    from .weapons import machine_gun

    x, y = p.req("x"), p.req("y")
    z, (ny, nz), _ = _glacis_frame(ctx, y)
    name = model.next_index("MG_")
    mb = model.mb(name, parent="Hull", pivot=(x, y, z))
    mb.revolve((x, y - 0.05, z), (0, 1, 0), [(0, 0.11), (0.08, 0.13), (0.16, 0.1), (0.2, 0.05)], seg=10,
               mat="Paint")
    tip = machine_gun(mb, x, y + 0.2, z, 0.5, jacket=True)
    model.socket(f"Socket_Muzzle_{int(name[3:]) + 1:02d}", name, (x, tip, z))


@part("su.side_fuel_tanks")
def side_fuel_tanks(model, p, ctx):
    """External cylindrical fuel tanks on the rear hull sides (T-34 from 1942).

    Parameters: `y` (list of tank centers), `length`, `diameter`, `z`, `x`
    (half distance; default on the sloped side plates).
    """
    L = p.get("length", 0.65)
    r = p.get("diameter", 0.35) / 2
    z = p.req("z")
    x = p.req("x")
    mb = model.mb("Hull")
    for y in p.req("y"):
        for side in (-1, 1):
            mb.cylinder((side * x, y - L / 2, z), (side * x, y + L / 2, z), r, seg=10, mat="Paint")
            for k in (-1, 1):  # strap bands
                yy = y + k * L * 0.3
                mb.cylinder((side * x, yy - 0.015, z), (side * x, yy + 0.015, z), r * 1.04, seg=10, mat="Metal")
