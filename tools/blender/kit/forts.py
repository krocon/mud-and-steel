"""Faction field fortifications (prompt 008): Tobruk ring stand, log-and-earth
bunkers, minefield markings.

Environment generators like those of `parts_neutral` (`@env_part`), for the
catalog entries `fort_bunker`, `fort_minefield_at`, `fort_minefield_ap` of
the factions (`de_bunker_tobruk`, `su_bunker_dzot`, `us_bunker_mg`,
`*_minefield_*`). They are built by `build_env.py` like the neutral objects
and live in the faction folders (`assets-src/blueprints/<faction>/`,
`godot/assets/models/<faction>/`). Front (embrasure) faces +Y.

Materials: concrete is `Stone` kind 2, logs `Wood` kind 0, earth cover
`Ground` kind 0 / spoil kind 1, sandbags `Canvas`, mines `Paint`.
"""

import math
import random

from mathutils import Vector

from .env import env_part


@env_part("tobruk")
def tobruk(model, name, p, box):
    """Tobruk ring stand (Ringstand 58c type): a concrete block sunk to the
    roof, only the slab with the round fighting opening and the entrance
    shows above ground, banked with earth.

    Parameters: `opening` (inner diameter of the ring, default 0.8),
    `collar` (height of the ring collar above the slab, default 0.12),
    `entrance` (true: stair opening at the rear, default true), `berm`
    (earth bank around the slab, fraction of the box, default 0.18).
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    berm = p.get("berm", 0.18)
    ro = p.get("opening", 0.8) / 2
    collar = p.get("collar", 0.12)
    sl, sw = L * (1 - 2 * berm), W * (1 - 2 * berm)
    slab_h = H - collar
    # earth bank: from the slab edge down to the box edge
    mb.style(kind=1)
    outer = [(-W / 2, -L / 2), (W / 2, -L / 2), (W / 2, L / 2), (-W / 2, L / 2)]
    inner = [(-sw / 2, -sl / 2), (sw / 2, -sl / 2), (sw / 2, sl / 2), (-sw / 2, sl / 2)]
    vo = mb.verts([(x, y, 0.0) for x, y in outer])
    vi = mb.verts([(x, y, slab_h * 0.85) for x, y in inner])
    for i in range(4):
        j = (i + 1) % 4
        mb.face([vo[i], vo[j], vi[j], vi[i]], "Ground")
    mb.face(list(reversed(vo)), "Ground")
    mb.face(vi, "Ground")
    # concrete slab
    mb.style(kind=2)
    mb.box((0, 0, slab_h * 0.85 + slab_h * 0.075), (sw * 1.02, sl * 1.02, slab_h * 0.15), "Stone", bevel=0.03)
    # ring collar around the opening, slightly forward of the center
    cy = sl * 0.12
    mb.cylinder((0, cy, slab_h - 0.01), (0, cy, H), ro + 0.18, seg=14, mat="Stone", smooth=False)
    mb.style(kind=3)
    mb.cylinder((0, cy, H - 0.06), (0, cy, H + 0.001), ro, seg=14, mat="Metal", cap0=True, cap1=True)
    if p.get("entrance", True):
        # dark stair opening at the rear with low concrete cheeks
        mb.style(kind=3)
        mb.box((0, -sl * 0.36, slab_h + 0.004), (0.8, sl * 0.2, 0.008), "Metal")
        mb.style(kind=2)
        for k in (-1, 1):
            mb.box((k * 0.5, -sl * 0.36, slab_h + collar * 0.4), (0.2, sl * 0.22, collar * 0.8), "Stone")


@env_part("log_bunker")
def log_bunker(model, name, p, box):
    """Log-and-earth field bunker (Soviet DZOT, US field MG emplacement):
    an earth mound over a log frame, embrasure in the front (+Y) face,
    entrance trench stub at the rear.

    Parameters: `walls` "logs" (log cribbing around the embrasure, default)
    or "sandbags" (sandbag front wall), `embrasure` [width, height]
    (default [1.2, 0.3]), `seed` (mound noise, default 1).
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    walls = p.get("walls", "logs")
    ew, eh = p.get("embrasure", [1.2, 0.3])
    rng = random.Random(p.get("seed", 1))
    # earth mound, flattened on top and at the ground
    mb.style(kind=0)
    mb.blob((0, -L * 0.04, H * 0.15), (W * 0.5, L * 0.46, H * 0.9), "Ground", seg=10, rings=5, rng=rng,
            rough=0.06, flat_bottom=0.0)
    # front wall around the embrasure
    fy = L * 0.42
    ez = H * 0.45
    if walls == "logs":
        mb.style(kind=0)
        for i, z in enumerate((ez - eh / 2 - 0.12, ez + eh / 2 + 0.12)):
            mb.tube([(-ew / 2 - 0.6, fy, z), (ew / 2 + 0.6, fy, z)], [0.12, 0.11], 6, "Wood", cap_start=True)
        for k in (-1, 1):
            mb.tube([(k * (ew / 2 + 0.12), fy, 0.0), (k * (ew / 2 + 0.12), fy, ez + eh / 2 + 0.2)], [0.11, 0.1], 6,
                    "Wood", cap_start=True)
    elif walls == "sandbags":
        for row in range(3):
            z = 0.12 + row * 0.24 if row < 2 else ez + eh / 2 + 0.12
            n = 5
            for i in range(n):
                x = (i - (n - 1) / 2) * (ew + 0.8) / n
                if row < 2 and abs(x) < ew / 2 and z > ez - eh / 2:
                    continue
                mb.box((x, fy, z), ((ew + 0.8) / n * 0.95, 0.30, 0.22), "Canvas", bevel=0.03)
    else:
        raise ValueError(f"log_bunker: walls must be logs or sandbags, not {walls!r}")
    # dark embrasure slot
    mb.style(kind=3)
    mb.box((0, fy + 0.02, ez), (ew, 0.05, eh), "Metal")
    # entrance trench stub at the rear, low spoil walls
    mb.style(kind=1)
    for k in (-1, 1):
        mb.box((k * 0.55, -L * 0.42, 0.12), (0.3, L * 0.16, 0.24), "Ground", bevel=0.05)


@env_part("minefield")
def minefield(model, name, p, box):
    """Marked minefield area (catalog height 0: the mines lie flush with the
    ground). Builds the perimeter markers (pickets with warning signs and a
    single trip wire between them) and a few representative mines, half
    buried; the full mine count is a game value, not geometry.

    Parameters: `mine` "at" (Teller-type disc) or "ap" (bounding mine
    tops), `visible` (number of modeled mines, default 9), `signs` (pickets
    per side, default 3), `wire` (perimeter wire, default true), `seed`.
    The variant needs `height_m` (sign height) because the catalog height
    is 0.
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    kind = p.get("mine", "at")
    n_vis = p.get("visible", 9)
    per = p.get("signs", 3)
    rng = random.Random(p.get("seed", 1))
    # perimeter pickets and boards
    posts = []
    for i in range(per):
        f = i / per
        posts += [(-W / 2 + W * f, -L / 2), (W / 2, -L / 2 + L * f), (W / 2 - W * f, L / 2), (-W / 2, L / 2 - L * f)]
    for x, y in posts:
        mb.style(kind=0)
        mb.box((x, y, H * 0.5), (0.05, 0.05, H), "Wood")
        mb.style(kind=1)
        n = Vector((x, y, 0)).normalized()
        mb.oriented_box((x + n.x * 0.04, y + n.y * 0.04, H - 0.12), (-n.y, n.x, 0), (0, 0, 1), (0.30, 0.18, 0.01),
                        "Paint")
    if p.get("wire", True):
        mb.style(kind=3)
        ring = [(-W / 2, -L / 2), (W / 2, -L / 2), (W / 2, L / 2), (-W / 2, L / 2)]
        for i in range(4):
            a, b = ring[i], ring[(i + 1) % 4]
            mb.tube([(a[0], a[1], H * 0.6), (b[0], b[1], H * 0.6)], [0.006, 0.006], 3, "Metal", cap_start=True)
    # representative mines, half buried
    mb.style(kind=0)
    for i in range(n_vis):
        x = (rng.random() - 0.5) * W * 0.8
        y = (rng.random() - 0.5) * L * 0.8
        if kind == "at":
            mb.cylinder((x, y, 0.0), (x, y, 0.05), 0.16, seg=8, mat="Paint", smooth=False)
            mb.cylinder((x, y, 0.05), (x, y, 0.07), 0.04, seg=4, mat="Paint", smooth=False)
        elif kind == "ap":
            mb.cylinder((x, y, 0.0), (x, y, 0.03), 0.05, seg=6, mat="Paint", smooth=False)
            mb.style(kind=3)
            mb.cylinder((x, y, 0.03), (x, y, 0.06), 0.006, seg=3, mat="Metal")
            mb.style(kind=0)
        else:
            raise ValueError(f"minefield: mine must be at or ap, not {kind!r}")
    _ = math
