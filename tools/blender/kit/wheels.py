"""Wheels, rollers, sprockets, idlers and axles.

All wheels are separate objects (they rotate around X) with the pivot at the
wheel center. The functions take model-space centers and build both the mesh
and, where needed, the object.
"""

import math

from .registry import part


def segments(r):
    """Ring resolution by radius: enough for the silhouette at RTS distance."""
    return 14 if r >= 0.36 else 10 if r >= 0.18 else 8


def disc_wheel(mb, center, r, width, side, tire="Rubber", dish=0.0, hub=0.22, face="Paint",
               tire_depth=0.12, inner_face=True, hub_cap=True):
    """Road wheel: tire (or steel rim) and a disc face with hub.

    `side` is -1 (left) or +1 (right) and decides which face points outward.
    `dish` pushes the disc center outward (cast/pressed Christie wheels).
    `inner_face` / `hub_cap` = False leave out faces that are hidden anyway
    (inner disc of a twin wheel) to save triangles.
    """
    cx, cy, cz = center
    seg = segments(r)
    x_in = cx - side * width / 2
    x_out = cx + side * width / 2
    # tire band
    mb.cylinder((x_in, cy, cz), (x_out, cy, cz), r, seg=seg, mat=tire, cap0=False, cap1=False)
    ri = r * (1.0 - tire_depth)
    inset = min(0.015, width * 0.1)
    if inner_face:
        mb.cylinder((x_in + side * inset, cy, cz), (x_in, cy, cz), ri, seg=seg, mat=face, cap0=False, cap1=True)
    else:  # plain cap closing the band on the hidden side
        mb.face(mb.verts(_ring((x_in, cy, cz), r, seg, -side)), face)
    if hub_cap:
        mb.cylinder((x_out - side * inset, cy, cz), (x_out + side * dish, cy, cz), ri, r1=r * hub * 1.6,
                    seg=seg, mat=face, cap0=False, cap1=True)
        hx = x_out + side * dish
        mb.cylinder((hx, cy, cz), (hx + side * 0.04, cy, cz), r * hub, r1=r * hub * 0.8, seg=6, mat="Metal",
                    cap0=False, cap1=True)
    else:
        mb.face(mb.verts(_ring((x_out, cy, cz), r, seg, side)), face)


def _ring(c, r, seg, facing):
    """Circle in the YZ plane; vertex order makes the face normal point to +X * facing."""
    pts = [(c[0], c[1] + math.cos(2 * math.pi * i / seg) * r, c[2] + math.sin(2 * math.pi * i / seg) * r)
           for i in range(seg)]
    return pts if facing > 0 else list(reversed(pts))


def spoked_idler(mb, center, r, width, side, spokes=6):
    """Idler: steel rim ring with spokes and hub (Panzer IV, Sherman)."""
    cx, cy, cz = center
    seg = segments(r)
    x_in = cx - side * width / 2
    x_out = cx + side * width / 2
    mb.cylinder((x_in, cy, cz), (x_out, cy, cz), r, seg=seg, mat="Metal", cap0=True, cap1=True)
    hub_r = r * 0.25
    mb.cylinder((x_out, cy, cz), (x_out + side * 0.05, cy, cz), hub_r, seg=8, mat="Paint", cap1=True)
    for i in range(spokes):
        a = 2 * math.pi * i / spokes
        y = cy + math.cos(a) * r * 0.55
        z = cz + math.sin(a) * r * 0.55
        mb.box((x_out + side * 0.012, y, z), (0.025, 0.06 + abs(math.sin(a)) * r * 0.5,
                                              0.06 + abs(math.cos(a)) * r * 0.5), "Paint")


def spoke_ribs(mb, center, r, x_face, side, spokes=6):
    """Raised spokes on the outer face of a disc wheel (spoked road wheels)."""
    _, cy, cz = center
    x = x_face + side * 0.02
    for i in range(spokes):
        a = 2 * math.pi * (i + 0.5) / spokes
        c, s = math.cos(a), math.sin(a)
        mb.cylinder((x, cy + c * r * 0.3, cz + s * r * 0.3), (x, cy + c * r, cz + s * r), 0.028, seg=4,
                    mat="Paint")
    mb.cylinder((x - side * 0.01, cy, cz), (x + side * 0.02, cy, cz), r * 0.32, seg=8, mat="Paint")


def sprocket(mb, center, r, width, side, teeth=13, rings=2):
    """Drive sprocket: slim toothed outer rim, plain inner rim, recessed hub and spokes (prompt 215)."""
    cx, cy, cz = center
    depth = r * 0.12
    ring_w = 0.045
    vis_teeth = min(teeth, 12)
    x_in = cx - side * width / 2
    x_out = cx + side * width / 2
    # Outer toothed ring (bare metal reads crisply against painted hull/skirts)
    xo_ring = x_out - side * (ring_w / 2) if rings == 2 else cx
    mb.gear((xo_ring, cy, cz), ring_w, r, vis_teeth, depth, mat="Metal")
    if rings == 2:
        # Inner ring sits behind the track: plain 10-gon drum instead of a second gear prism
        xi_ring = x_in + side * (ring_w / 2)
        mb.cylinder((xi_ring - side * ring_w / 2, cy, cz), (xi_ring + side * ring_w / 2, cy, cz),
                    r - depth * 0.35, seg=10, mat="Metal", cap0=True, cap1=False)
    # Central painted hub drum + metallic hub cap + 4 spoke ribs (flush with x_out so width never grows)
    mb.cylinder((x_in, cy, cz), (x_out - side * 0.03, cy, cz), r * 0.58, seg=10,
                mat="Paint", cap0=False, cap1=True)
    mb.cylinder((x_out - side * 0.04, cy, cz), (x_out, cy, cz), r * 0.30, r1=r * 0.22,
                seg=8, mat="Metal", cap0=False, cap1=True)
    for i in range(4):
        a = math.pi * i / 4.0
        c, s = math.cos(a), math.sin(a)
        mb.cylinder((x_out - side * 0.022, cy - c * r * 0.56, cz - s * r * 0.56),
                    (x_out - side * 0.022, cy + c * r * 0.56, cz + s * r * 0.56),
                    0.020, seg=4, mat="Paint", cap0=False, cap1=False)


def return_roller(mb, center, r, width, side, rubber=True):
    cx, cy, cz = center
    x_in = cx - side * width / 2
    x_out = cx + side * width / 2
    mb.cylinder((x_in, cy, cz), (x_out, cy, cz), r, seg=8, mat="Rubber" if rubber else "Paint")
    mb.cylinder((x_out, cy, cz), (x_out + side * 0.03, cy, cz), r * 0.4, seg=6, mat="Metal")


def tire_wheel(mb, center, r, width, side, rim_r=0.55, spokes=0):
    """Pneumatic or solid tire with a pressed steel rim (trucks, guns, recon cars)."""
    cx, cy, cz = center
    seg = segments(r)
    x_in = cx - side * width / 2
    x_out = cx + side * width / 2
    bulge = width * 0.08
    rings_t = [(-0.5, 0.86), (-0.4, 0.98), (0.4, 0.98), (0.5, 0.86)]
    mb.revolve((cx, cy, cz), (side, 0, 0), [(t * width, r * f) for t, f in rings_t], seg=seg, mat="Rubber",
               cap_mat="Rubber")
    mb.cylinder((x_out - side * 0.005, cy, cz), (x_out + side * bulge, cy, cz), r * rim_r, r1=r * rim_r * 0.55,
                seg=seg, mat="Paint", cap0=False)
    mb.cylinder((x_in + side * 0.005, cy, cz), (x_in - side * bulge * 0.5, cy, cz), r * rim_r, seg=seg,
                mat="Paint", cap0=False)
    hub = r * rim_r * 0.4
    mb.cylinder((x_out + side * bulge, cy, cz), (x_out + side * (bulge + 0.04), cy, cz), hub, r1=hub * 0.7,
                seg=8, mat="Metal", cap0=False)
    for i in range(spokes):
        a = 2 * math.pi * i / spokes
        mb.box((x_out + side * bulge * 0.5, cy + math.cos(a) * r * rim_r * 0.7, cz + math.sin(a) * r * rim_r * 0.7),
               (0.02, 0.05, 0.05), "Metal")


@part("wheel_axle")
def wheel_axle(model, p, ctx):
    """A pair of tire wheels on one axle (trucks, recon cars, gun carriages).

    Parameters: `y`, `diameter`, `width`, `track` (center distance of the two
    wheels), optional `parent` (default Hull), `index` (two-digit wheel number),
    `twin` (dual rear tires), `axle_radius`, `spokes`.
    """
    parent = p.get("parent", "Hull")
    y = p.req("y")
    r = p.req("diameter") / 2
    w = p.req("width")
    track = p.req("track")
    idx = p.get("index", 1)
    twin = p.get("twin", False)
    spokes = p.get("spokes", 0)
    for side, s in ((-1, "L"), (1, "R")):
        x = side * track / 2
        name = f"Wheel_{s}_{idx:02d}"
        mb = model.mb(name, parent=parent, pivot=(x, y, r))
        tire_wheel(mb, (x, y, r), r, w, side, spokes=spokes)
        if twin:
            tire_wheel(mb, (x - side * w * 1.05, y, r), r, w, -side)
    ar = p.get("axle_radius", 0.05)
    model.mb(parent).cylinder((-track / 2 + w / 2, y, r), (track / 2 - w / 2, y, r), ar, seg=8, mat="Metal")
    return {"axle_z": r}
