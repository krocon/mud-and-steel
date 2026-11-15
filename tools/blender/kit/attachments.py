"""Attachments: skirts, spare tracks, tools, jerrycans, lights, antennas,
exhausts, tow hooks, marking and effect sockets.

Positions are model space unless noted. Most parts take `parent` (default
Hull) so they can also go on the turret or a carriage.
"""

import math

from .core import surface_matrix
from .registry import part


@part("skirts")
def skirts(model, p, ctx):
    """Side skirts on hangers (Panzer IV 'Schürzen', 5 mm plates).

    Parameters: `y` [rear, front], `bottom_z`, `top_z`, `plates` (count per
    side), `gap` (distance outside the fenders), `thickness`.
    Adds `Skirt_L`, `Skirt_R` (children of Hull). The catalog width of most
    vehicles excludes skirts: list `Skirt_*` in the blueprint `[validate]`.
    """
    y0, y1 = p.req("y")
    zb, zt = p.req("bottom_z"), p.req("top_z")
    n = p.get("plates", 5)
    gap = p.get("gap", 0.12)
    t = p.get("thickness", 0.01)
    x = ctx["half_width"] + gap
    seg = (y1 - y0) / n
    for side, s in ((-1, "L"), (1, "R")):
        mb = model.mb(f"Skirt_{s}", parent="Hull", pivot=(side * x, (y0 + y1) / 2, zt))
        for i in range(n):
            a = y0 + i * seg + 0.01
            b = a + seg - 0.02
            mb.box((side * (x + t / 2), (a + b) / 2, (zb + zt) / 2), (t, b - a, zt - zb), "Paint")
        # hanger rail and brackets
        mb.box((side * (x - 0.02), (y0 + y1) / 2, zt - 0.03), (0.04, y1 - y0, 0.03), "Paint")
        for i in range(n + 1):
            y = y0 + i * seg
            mb.box((side * (x - gap / 2), y, zt - 0.03), (gap, 0.04, 0.03), "Metal")


@part("turret_skirt")
def turret_skirt(model, p, ctx):
    """Plate skirt around the turret sides and rear ('Turmschürze').

    Parameters: `offset` (distance from the turret wall), `bottom_z`, `top_z`
    (relative to turret base), `front_y` (relative to ring center, where the
    side plates begin), `thickness`, `doors` (hatches in the side plates).
    Adds `TurretSkirt` (child of Turret).
    """
    plan = ctx["turret_plan"]
    off = p.get("offset", 0.18)
    zb = ctx["turret_z"] + p.req("bottom_z")
    zt = ctx["turret_z"] + p.req("top_z")
    fy = ctx["turret_y"] + p.req("front_y")
    t = p.get("thickness", 0.008)
    ty = ctx["turret_y"]
    mb = model.mb("TurretSkirt", parent="Turret", pivot=(0.0, ty, zt))
    xs = [x for x, _ in plan]
    ys = [y for _, y in plan]
    hw = max(abs(x) for x in xs) + off
    ry = ty + min(ys) - off * 1.4
    # left, right and rear plates (rear corners chamfered)
    ch = 0.3
    path = [(-hw, fy), (-hw, ry + ch), (-hw + ch, ry), (hw - ch, ry), (hw, ry + ch), (hw, fy)]
    for (xa, ya), (xb, yb) in zip(path, path[1:]):
        pts = [(xa, ya, zb), (xb, yb, zb), (xb, yb, zt), (xa, ya, zt)]
        dx, dy = yb - ya, -(xb - xa)
        ln = math.hypot(dx, dy)
        mb.extrude(pts, (dx / ln * t, dy / ln * t, 0), "Paint", bevel=0.003)
    for k in (-1, 1):
        for yy in (fy - 0.1, ry + 0.5):
            mb.box((k * (hw - off / 2), yy, zt - 0.1), (off, 0.04, 0.04), "Metal")
    if p.get("doors", True):
        for k in (-1, 1):
            mb.box((k * (hw + 0.012), fy - 0.45, (zb + zt) / 2), (0.012, 0.42, (zt - zb) * 0.7), "Paint")
    return {"turret_skirt_half_width": hw}


@part("spare_tracks")
def spare_tracks(model, p, ctx):
    """Spare track links mounted on a plate.

    Parameters: `origin` [x, y, z] (first link center), `direction` [x, y, z]
    (step between links), `count`, `link_size` [x, y, z], `parent`.
    """
    ox, oy, oz = p.req("origin")
    dx, dy, dz = p.req("direction")
    n = p.get("count", 4)
    sx, sy, sz = p.get("link_size", [0.4, 0.13, 0.04])
    mb = model.mb(p.get("parent", "Hull"))
    for i in range(n):
        c = (ox + dx * i, oy + dy * i, oz + dz * i)
        faces = mb.box(c, (sx, sy, sz), "Metal")
        for f in faces:
            f[mb.track] = 0
        mb.box((c[0], c[1], c[2] + sz / 2 + 0.005), (sx * 0.2, sy * 0.5, 0.02), "Metal")
    mb.box((ox + dx * (n - 1) / 2, oy + dy * (n - 1) / 2, oz + dz * (n - 1) / 2),
           (max(abs(dx) * n, 0.03), max(abs(dy) * n, 0.03), 0.02), "Metal")


@part("tool")
def tool(model, p, ctx):
    """Pioneer tools strapped to the hull: shovel, axe, crowbar, jack, wire cutter.

    Parameters: `kind`, `position` [x, y, z], `axis` "x" or "y" (long axis),
    `length`, `parent`.
    """
    kind = p.req("kind")
    x, y, z = p.req("position")
    ax = p.get("axis", "y")
    L = p.get("length", {"shovel": 1.1, "axe": 0.8, "crowbar": 1.2, "jack": 0.45, "wire_cutter": 0.6,
                         "sledgehammer": 0.9}.get(kind, 0.8))
    mb = model.mb(p.get("parent", "Hull"))

    def along(t):
        return (x, y + t, z) if ax == "y" else (x + t, y, z)

    mb.cylinder(along(-L / 2), along(L / 2), 0.018, seg=6, mat="Wood" if kind in ("shovel", "axe",
                                                                                  "sledgehammer") else "Metal")
    if kind == "shovel":
        c = along(L / 2 - 0.12)
        mb.box(c, (0.22, 0.25, 0.015) if ax == "y" else (0.25, 0.22, 0.015), "Metal")
    elif kind in ("axe", "sledgehammer"):
        c = along(L / 2 - 0.05)
        mb.box(c, (0.16, 0.06, 0.04) if ax == "y" else (0.06, 0.16, 0.04), "Metal")
    elif kind == "jack":
        mb.cylinder((x, y, z - 0.05), (x, y, z + 0.08), 0.07, seg=8, mat="Paint")
    for t in (-L / 3, L / 3):
        mb.box(along(t), (0.05, 0.03, 0.05), "Metal")


@part("jerrycans")
def jerrycans(model, p, ctx):
    """Row of 20 l jerrycans (0.47 x 0.35 x 0.16 m).

    Parameters: `origin` [x, y, z] (bottom center of the first can), `step`
    [x, y, z], `count`, `upright` (default true), `parent`.
    """
    ox, oy, oz = p.req("origin")
    sx, sy, sz = p.get("step", [0.17, 0.0, 0.0])
    n = p.get("count", 2)
    up = p.get("upright", True)
    mb = model.mb(p.get("parent", "Hull"))
    size = (0.16, 0.35, 0.47) if up else (0.35, 0.47, 0.16)
    for i in range(n):
        c = (ox + sx * i, oy + sy * i, oz + sz * i + size[2] / 2)
        mb.box(c, size, "Paint", bevel=0.015)
        mb.box((c[0], c[1] + size[1] * 0.3, c[2] + size[2] / 2 + 0.02), (0.05, 0.05, 0.04), "Metal")


@part("headlight")
def headlight(model, p, ctx):
    """Headlight with glass front. Parameters: `position`, `diameter`, `parent`, `blackout`."""
    x, y, z = p.req("position")
    r = p.get("diameter", 0.18) / 2
    mb = model.mb(p.get("parent", "Hull"))
    mb.cylinder((x, y - r * 0.8, z), (x, y + r * 0.3, z), r * 0.7, r1=r, seg=10, mat="Paint")
    mb.cylinder((x, y + r * 0.3, z), (x, y + r * 0.35, z), r * 0.92, seg=10, mat="Glass", cap0=False)
    mb.box((x, y - r * 0.5, z - r * 1.1), (0.04, 0.04, r * 1.2), "Paint")
    if p.get("blackout", False):
        mb.box((x, y + r * 0.4, z + r * 0.6), (r * 1.8, 0.02, r * 0.5), "Paint")


@part("antenna")
def antenna(model, p, ctx):
    """Whip antenna with base. Parameters: `position`, `length`, `parent`.

    Adds `Antenna_NN` (own object so it can bend/hide). Usually excluded from
    the height check (blueprint `[validate]`).
    """
    x, y, z = p.req("position")
    L = p.get("length", 2.0)
    parent = p.get("parent", "Hull")
    name = model.next_index("Antenna_")
    mb = model.mb(name, parent=parent, pivot=(x, y, z))
    mb.cylinder((x, y, z - 0.02), (x, y, z + 0.12), 0.04, seg=8, mat="Metal")
    mb.cylinder((x, y, z + 0.12), (x, y, z + L), 0.008, r1=0.004, seg=4, mat="Metal", cap0=False)


@part("exhaust")
def exhaust(model, p, ctx):
    """Exhaust muffler or pipe with `Socket_Exhaust_NN` at the outlet.

    Parameters: `position` [x, y, z] (center), `kind` "muffler" (horizontal
    drum across X) or "pipe" (pointing back/up), `length`, `diameter`,
    `outlet` [x, y, z] (optional, default end of the part), `parent`.
    """
    x, y, z = p.req("position")
    kind = p.get("kind", "muffler")
    L = p.get("length", 0.6)
    r = p.get("diameter", 0.25) / 2
    parent = p.get("parent", "Hull")
    mb = model.mb(parent)
    if kind == "muffler":
        mb.cylinder((x - L / 2, y, z), (x + L / 2, y, z), r, seg=10, mat="Metal")
        out = (x + L / 2 + 0.05, y, z)
        mb.cylinder((x + L / 2, y, z), out, r * 0.3, seg=6, mat="Metal")
    else:
        out = (x, y - L, z)
        mb.cylinder((x, y, z), out, r, seg=8, mat="Metal")
    out = tuple(p.get("outlet", out))
    model.socket(model.next_index("Socket_Exhaust_"), parent, out)


@part("tow_hook")
def tow_hook(model, p, ctx):
    """Tow hook or shackles. Parameters: `position`, `parent`, `pair` (two
    shackles), `socket` (adds `Socket_Tow`, default true; only one per model)."""
    x, y, z = p.req("position")
    parent = p.get("parent", "Hull")
    mb = model.mb(parent)
    xs = (x - 0.4, x + 0.4) if p.get("pair", False) else (x,)
    sign = 1 if y > 0 else -1
    for xx in xs:
        mb.box((xx, y + sign * 0.04, z), (0.06, 0.1, 0.12), "Metal")
    if p.get("socket", True):
        model.socket("Socket_Tow", parent, (x, y + sign * 0.1, z))


@part("box_prop")
def box_prop(model, p, ctx):
    """Generic box (stowage bin, air filter, plate). Parameters: `center`, `size`, `material`, `bevel`, `parent`."""
    mb = model.mb(p.get("parent", "Hull"))
    mb.box(tuple(p.req("center")), tuple(p.req("size")), p.get("material", "Paint"), bevel=p.get("bevel", 0.01))


@part("cylinder_prop")
def cylinder_prop(model, p, ctx):
    """Generic cylinder (fuel drum, air cleaner, periscope). Parameters: `from`, `to`, `radius`, `material`, `parent`, `segments`."""
    mb = model.mb(p.get("parent", "Hull"))
    mb.cylinder(tuple(p.req("from")), tuple(p.req("to")), p.req("radius"), seg=p.get("segments", 10),
                mat=p.get("material", "Paint"))


@part("marking")
def marking(model, p, ctx):
    """Decal socket for national markings or numbers (docs/ASSETS.md "Markings").

    Parameters: `name` (suffix, e.g. "Turret_L"), `position`, `normal`
    (surface normal pointing out), `kind` "national" | "number", `size`
    (decal edge in m), `parent`. The empty's local +Z is the surface normal
    and +Y points up, so a Godot Decal child projects straight onto the plate.
    """
    name = "Socket_Marking_" + p.req("name")
    pos = p.req("position")
    m = surface_matrix(pos, p.req("normal"))
    model.socket(name, p.get("parent", "Hull"), pos, matrix=m, marking=p.get("kind", "national"),
                 marking_size=p.get("size", 0.5))


@part("socket")
def socket(model, p, ctx):
    """Any other socket: Socket_Fire_NN, Socket_Crew_NN, Socket_Passenger_NN ...

    Parameters: `name` (full name, or a prefix ending in "_" for the next free
    index, e.g. "Socket_Fire_"), `position`, `parent`.
    """
    name = p.req("name")
    if name.endswith("_"):
        name = model.next_index(name)
    model.socket(name, p.get("parent", "Hull"), p.req("position"))
