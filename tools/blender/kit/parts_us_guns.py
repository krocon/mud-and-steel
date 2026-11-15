"""Faction parts of `us` for prompt 008, session `guns`: AA guns, howitzers, heavy mortars and fortifications (env generators with @env_part).

Owned by that session only (one module per parallel session, so the three
us sessions never edit the same file). Part names carry the faction prefix
like `parts_us.py`: `@part("us.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("us_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401

import math  # noqa: E402
import random  # noqa: E402

from .env import env_part  # noqa: E402


def _bag_row(mb, x0, x1, y, z, bag, rng, skip=None, yaw=0.06):
    """One course of sandbags from x0 to x1 along X at depth y, bottom z.

    `bag` = [length, depth, height]; `skip` = (x_min, x_max) leaves a gap
    (embrasure, entrance). Bags get a small random yaw and offset so that a
    wall does not look like masonry.
    """
    bl, bd, bh = bag
    n = max(1, round((x1 - x0) / bl))
    step = (x1 - x0) / n
    for i in range(n):
        x = x0 + (i + 0.5) * step
        if skip and skip[0] < x < skip[1]:
            continue
        a = (rng.random() - 0.5) * 2 * yaw
        mb.oriented_box((x + (rng.random() - 0.5) * 0.03, y + (rng.random() - 0.5) * 0.04, z + bh / 2),
                        (math.cos(a), math.sin(a), 0), (0, 0, 1), (step * 0.96, bh, bd), "Canvas")


@env_part("us_mg_emplacement")
def us_mg_emplacement(model, name, p, box):
    """Covered MG emplacement after FM 5-15 (1944): the pit is dug in, above
    ground stand a sandbag front wall with a wide firing slit, a log roof
    spanning the pit (log ends showing at the front and sides) under a layer
    of earth with a sandbag rim, and an earth berm banked against sides and
    rear with the entrance gap at the rear. Front (embrasure) faces +Y.

    Parameters: `embrasure` [width, height] of the firing slit (default
    [1.0, 0.22]), `logs` (number of roof logs, default 13), `log_diameter`
    (default 0.15), `bag` [length, depth, height] of one filled sandbag
    (default [0.46, 0.28, 0.13]), `entrance` (width of the rear gap,
    default 0.7), `seed` (default 1). Builds roughly into the box; the env
    builder fits it exactly.
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    ew, eh = p.get("embrasure", [1.0, 0.22])
    n_logs = p.get("logs", 13)
    ld = p.get("log_diameter", 0.15)
    bag = p.get("bag", [0.46, 0.28, 0.13])
    ent = p.get("entrance", 0.7)
    rng = random.Random(p.get("seed", 1))
    bh = bag[2]
    # heights: wall courses up to the slit, slit, roof logs, earth cap with bag rim
    sill = 3 * bh
    roof_z = sill + eh
    cap_z = roof_z + ld
    top = H
    front_y = L / 2 - bag[1] / 2
    rear_y = -L / 2 + 0.45
    hw = W / 2 - 0.42  # inner half width of the roof (the berm takes the rest)
    # earth berm banked against the sides and the rear (wedges), up to the log
    # ends; the rear berm leaves the entrance gap
    mb.style(kind=0)
    berm_top = roof_z + ld * 0.6
    y_end = front_y - bag[1] / 2
    for k in (-1, 1):
        sec = [(k * W / 2, y_end, 0.0), (k * hw, y_end, 0.0), (k * hw, y_end, berm_top)]
        mb.extrude(sec if k > 0 else list(reversed(sec)), (0, -(y_end + L / 2), 0), "Ground")
    for k in (-1, 1):
        x0, x1 = k * (ent / 2 + bag[1]), k * (hw + 0.1)
        sec = [(x0, -L / 2, 0.0), (x0, rear_y, 0.0), (x0, rear_y, berm_top)]
        mb.extrude(sec if k < 0 else list(reversed(sec)), (x1 - x0, 0, 0), "Ground")
    # sandbag front wall: three courses under the slit, cheeks beside the slit
    for c in range(3):
        off = (bag[0] / 2) * (c % 2)
        _bag_row(mb, -W / 2 + 0.12 + off * 0.5, W / 2 - 0.12 - off * 0.5, front_y, c * bh, bag, rng)
    for c in range(2):
        z = sill + c * bh
        if z + bh > roof_z + 0.02:
            break
        _bag_row(mb, -W / 2 + 0.2, W / 2 - 0.2, front_y, z, bag, rng, skip=(-ew / 2 - 0.1, ew / 2 + 0.1))
    # dark firing slit
    mb.style(kind=3)
    mb.box((0, front_y - bag[1] * 0.3, sill + eh / 2), (ew, 0.04, eh), "Metal")
    # log roof: logs across the pit (along X), from just behind the front wall to the rear berm
    mb.style(kind=0)
    y_front = front_y + bag[1] * 0.2
    span = y_front - rear_y
    for i in range(n_logs):
        y = y_front - (i + 0.5) * span / n_logs
        r = ld / 2 * (0.9 + 0.2 * rng.random())
        x = hw + 0.10 + 0.08 * rng.random()
        mb.tube([(-x, y, roof_z + r), (x, y, roof_z + r)], [r, r * 0.9], 6, "Wood", cap_start=True)
    # earth cap on the logs and a rim of sandbags around it
    mb.style(kind=1)
    cap_h = top - cap_z
    mb.box((0, (y_front + rear_y) / 2 - 0.05, cap_z + cap_h * 0.4), (2 * hw - 0.1, span - 0.25, cap_h * 0.8), "Ground")
    mb.style(kind=0)
    rim = [bag[0], bag[1] * 0.9, min(bh, cap_h)]
    _bag_row(mb, -hw, hw, y_front - rim[1] / 2, cap_z, rim, rng)
    for k in (-1, 1):
        n = max(1, round((span - rim[1]) / rim[0]))
        for i in range(n):
            y = y_front - rim[1] - (i + 0.5) * (span - rim[1]) / n
            a = (rng.random() - 0.5) * 0.12
            mb.oriented_box((k * (hw - rim[1] / 2), y, cap_z + rim[2] / 2), (math.sin(a), math.cos(a), 0),
                            (0, 0, 1), ((span - rim[1]) / n * 0.96, rim[2], rim[1]), "Canvas")
    # entrance at the rear: dark opening under the roof between sandbag cheeks
    mb.style(kind=3)
    mb.box((0, rear_y + 0.01, roof_z / 2), (ent, 0.02, roof_z), "Metal")
    mb.style(kind=0)
    for k in (-1, 1):
        for c in range(int(roof_z / bh)):
            mb.box((k * (ent / 2 + bag[1] / 2), (rear_y - L / 2) / 2, c * bh + bh / 2),
                   (bag[1], rear_y + L / 2, bh * 0.95), "Canvas")
