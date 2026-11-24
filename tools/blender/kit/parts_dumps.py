"""Supply dumps that burn and explode (prompt 041, data/fire.toml).

`dump_ammo`: rows of wooden ammunition crates on dunnage, part of a row
under a tarpaulin. `dump_fuel`: standing 200 l drums in a loose grid on
dunnage. Both have a `burnt` variant (`burnt = true`): what is left after
the fire, low, charred (Wood kind 3 "charred", Metal kind 1 "rust") and
scattered. No markings or symbols.

Crates and drums are simpler than the single props of `parts_neutral`
(one box per crate column, eight-sided drums without rolling hoops), so a
dump of 40 drums stays within its triangle budget.
"""

import math

from .env import env_part, rng

DRUM_R = 0.292
DRUM_H = 0.876


@env_part("dump_ammo")
def dump_ammo(model, name, p, box):
    """Two rows of crate stacks along Y on dunnage. Parameters: `seed`,
    `layers` (crate layers, default 4), `burnt` (default false)."""
    r = rng(p.get("seed", 1))
    burnt = p.get("burnt", False)
    layers = p.get("layers", 4)
    mb = model.mb(name)
    wood = 3 if burnt else 0
    row_w = box.width * 0.36
    gap = box.width - 2 * row_w
    crate_l = 0.78
    crate_h = 0.3
    dunnage = 0.12
    cols = max(1, int(box.length / (crate_l + 0.04)))
    y0 = -cols * (crate_l + 0.04) / 2
    for row in (-1, 1):
        x = row * (row_w + gap) / 2
        # dunnage beams across the row
        mb.style(kind=wood)
        for k in range(3):
            y = -box.hl * 0.85 + k * box.hl * 0.85
            mb.box((x, y, dunnage / 2), (row_w + 0.2, 0.12, dunnage), "Wood")
        for c in range(cols):
            y = y0 + (c + 0.5) * (crate_l + 0.04)
            n = layers
            if burnt:
                n = 1 if r.random() < 0.6 else 0
            elif r.random() < 0.15:
                n = layers - 1
            for k in range(n):
                dx = r.uniform(-0.03, 0.03)
                dy = r.uniform(-0.02, 0.02)
                h = crate_h * (0.55 if burnt else 1.0)
                z = dunnage + k * crate_h + h / 2
                mb.box((x + dx, y + dy, z), (row_w, crate_l, h * 0.97), "Wood")
                # lid seam: a slightly darker strip on top
                if k == n - 1 and not burnt:
                    mb.box((x + dx, y + dy, z + h / 2), (row_w * 0.98, 0.04, 0.012), "Wood")
    if burnt:
        # scattered charred boards and a few crates thrown off
        for _ in range(8):
            x = r.uniform(-box.hw * 0.9, box.hw * 0.9)
            y = r.uniform(-box.hl * 0.9, box.hl * 0.9)
            mb.box((x, y, 0.03), (r.uniform(0.1, 0.25), r.uniform(0.5, 1.0), 0.05), "Wood")
    else:
        # tarpaulin over the first half of one row
        mb.style(kind=0)
        x = (row_w + gap) / 2
        top = dunnage + layers * crate_h
        mb.box((x, -box.hl * 0.45, top + 0.02), (row_w + 0.12, box.length * 0.5, 0.04), "Canvas")
        for s in (-1, 1):
            mb.box((x + s * (row_w / 2 + 0.06), -box.hl * 0.45, top - 0.25),
                   (0.02, box.length * 0.5, 0.5), "Canvas")


@env_part("dump_fuel")
def dump_fuel(model, name, p, box):
    """Standing 200 l drums in a grid with gaps, on two dunnage beams per
    row. Parameters: `seed`, `missing` (share of empty places, default
    0.15), `burnt` (default false: burst, tipped and lying drums)."""
    r = rng(p.get("seed", 1))
    burnt = p.get("burnt", False)
    missing = p.get("missing", 0.15)
    mb = model.mb(name)
    pitch = DRUM_R * 2 + 0.016
    nx = max(1, int(box.width / pitch))
    ny = max(1, int(box.length / pitch))
    x0 = -nx * pitch / 2
    y0 = -ny * pitch / 2
    dunnage = 0.024
    mb.style(kind=3 if burnt else 0)
    for i in range(nx):
        x = x0 + (i + 0.5) * pitch
        mb.box((x, 0.0, dunnage / 2), (0.08, ny * pitch, dunnage), "Wood")
    for i in range(nx):
        for j in range(ny):
            if r.random() < missing:
                continue
            x = x0 + (i + 0.5) * pitch
            y = y0 + (j + 0.5) * pitch
            if burnt:
                roll = r.random()
                mb.style(kind=1)
                if roll < 0.45:
                    continue
                if roll < 0.75:
                    # tipped over, lying along a random direction
                    a = r.uniform(0, math.pi)
                    d = (math.cos(a) * DRUM_H / 2, math.sin(a) * DRUM_H / 2)
                    mb.cylinder((x - d[0], y - d[1], DRUM_R), (x + d[0], y + d[1], DRUM_R), DRUM_R,
                                seg=8, mat="Metal", smooth=True)
                    continue
                h = DRUM_H * r.uniform(0.5, 0.9)
                mb.cylinder((x, y, dunnage), (x, y, dunnage + h), DRUM_R * 0.97, seg=8, mat="Metal",
                            smooth=True)
                continue
            mb.style(kind=3 if r.random() < 0.3 else 0)
            mb.cylinder((x, y, dunnage), (x, y, dunnage + DRUM_H), DRUM_R * 0.985, seg=8, mat="Paint",
                        smooth=True)


@env_part("tent")
def tent(model, name, p, box):
    """Army tent of a camp behind the lines (prompt 124). Parameters:
    `shape` = `pyramidal` (square, low walls, a pyramid roof on a center
    pole) or `ridge` (a wall tent: walls and a gabled roof along Y),
    `wall` (wall height in m, default 0.9). Canvas, a closed door flap at
    -Y, guy ropes to stakes. No markings."""
    shape = p.get("shape", "pyramidal")
    wall = p.get("wall", 0.9)
    mb = model.mb(name)
    hw, hl, h = box.hw, box.hl, box.height
    # Tent canvas (kind 1, olive drab): the hessian of sandbags was far too
    # light for a whole tent (prompt 202).
    mb.style(kind=1)
    if shape == "pyramidal":
        def ring(z, s):
            return [(-hw * s, -hl * s, z), (hw * s, -hl * s, z), (hw * s, hl * s, z), (-hw * s, hl * s, z)]

        mb.loft([ring(0.0, 1.0), ring(wall, 1.0), ring(h - 0.1, 0.05)], "Canvas")
        mb.cylinder((0.0, 0.0, h - 0.12), (0.0, 0.0, h + 0.05), 0.035, seg=6, mat="Wood")
        corners = [(sx * hw, sy * hl) for sx in (-1, 1) for sy in (-1, 1)]
    else:
        front = [(-hw, -hl, 0.0), (hw, -hl, 0.0), (hw, -hl, wall), (0.0, -hl, h), (-hw, -hl, wall)]
        mb.extrude(front, (0.0, box.length, 0.0), "Canvas")
        # ridge pole ends and the uprights at both gables
        for y in (-hl, hl):
            mb.cylinder((0.0, y, h - 0.05), (0.0, y + (0.12 if y < 0 else -0.12), h - 0.05), 0.03, seg=6, mat="Wood")
        corners = [(sx * hw, y) for sx in (-1, 1) for y in (-hl * 0.6, 0.0, hl * 0.6)]
    # door flap: a slightly raised panel on the front wall
    mb.box((0.0, -hl - 0.015, wall * 0.48), (min(1.0, box.width * 0.3), 0.03, wall * 0.95), "Canvas")
    # guy ropes from the eaves to stakes 1.2 m out
    for x, y in corners:
        sx = 1.0 if x > 0 else -1.0
        top = (x, y, wall)
        stake = (x + sx * 1.2, y, 0.0)
        mb.cylinder(top, stake, 0.012, seg=4, mat="Canvas", cap0=False, cap1=False)
        mb.box((stake[0], stake[1], 0.12), (0.05, 0.05, 0.25), "Wood")
