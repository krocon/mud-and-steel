"""Faction parts of `su` for prompt 008, session `guns`: AA guns, howitzers, heavy mortars and fortifications (env generators with @env_part).

Owned by that session only (one module per parallel session, so the three
su sessions never edit the same file). Part names carry the faction prefix
like `parts_su.py`: `@part("su.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("su_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

import math
import random

from mathutils import Vector

from .env import env_part
from .forts import minefield
from .registry import part  # noqa: F401


@env_part("su_minefield_ap")
def su_minefield_ap(model, name, p, box):
    """Soviet anti-personnel minefield section: the perimeter marking of the
    shared `minefield` generator (pickets, boards, perimeter wire, no generic
    mines) with PMD-6 wooden box mines lying flush and POMZ-2 stake mines on
    short wooden stakes, each with a tripwire to a peg.

    Parameters: `pmd` (number of modeled PMD-6 boxes, default 8), `pomz`
    (number of modeled POMZ-2 stakes, default 3), `stake` (POMZ-2 height
    above ground, default 0.30), `signs`, `wire` (as `minefield`), `seed`.
    The variant needs `height_m` (sign height) because the catalog height
    is 0. The full mine count is a game value, not geometry.
    """
    n_pmd = p.get("pmd", 8)
    n_pomz = p.get("pomz", 3)
    stake = p.get("stake", 0.30)
    seed = p.get("seed", 1)
    # perimeter only: the shared generator with no representative mines
    p.t["visible"] = 0
    p.t["mine"] = "ap"
    minefield(model, name, p, box)
    mb = model.mb(name)
    L, W = box.length, box.width
    rng = random.Random(seed + 101)

    def spot():
        return (rng.random() - 0.5) * W * 0.8, (rng.random() - 0.5) * L * 0.8

    # PMD-6: wooden box ~0.19 x 0.09 x 0.065 m with a hinged lid, flush
    mb.style(kind=0)
    for _ in range(n_pmd):
        x, y = spot()
        a = rng.random() * math.pi
        u = Vector((math.cos(a), math.sin(a), 0.0))
        mb.oriented_box((x, y, 0.03), tuple(u), (0, 0, 1), (0.19, 0.09, 0.06), "Wood")
    # POMZ-2: cast-iron fragmentation body (60 mm, 130 mm high) on a stake,
    # tripwire to a peg about 2 m away
    for _ in range(n_pomz):
        x, y = spot()
        mb.style(kind=0)
        mb.box((x, y, (stake - 0.13) / 2), (0.04, 0.04, stake - 0.13), "Wood")
        mb.style(kind=3)
        mb.cylinder((x, y, stake - 0.13), (x, y, stake), 0.03, seg=6, mat="Metal", smooth=False)
        a = rng.random() * 2 * math.pi
        px, py = x + math.cos(a) * 2.0, y + math.sin(a) * 2.0
        mb.tube([(x, y, stake - 0.03), (px, py, 0.10)], [0.004, 0.004], 3, "Metal", cap_start=True)
        mb.style(kind=0)
        mb.box((px, py, 0.06), (0.03, 0.03, 0.12), "Wood")

