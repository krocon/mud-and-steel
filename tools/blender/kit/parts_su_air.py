"""Faction parts of `su` for prompt 008, session `air`: aircraft.

Owned by that session only (one module per parallel session, so the three
su sessions never edit the same file). Part names carry the faction prefix
like `parts_su.py`: `@part("su.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("su_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

import math

from .registry import part


@part("su.propeller")
def propeller_phased(model, p, ctx):
    """Propeller with spinner like `propeller`, plus a blade phase angle.

    The generic part puts the first blade horizontal (+X). Catalog heights of
    some Soviet aircraft (Il-2M3, Pe-2, Po-2) are only reached with one blade
    pointing straight up, the usual way the height is measured on drawings;
    the static blade position is arbitrary in game because the propeller spins.

    Parameters: `position` [x, y, z] (hub), `diameter`, `blades` (default 3),
    `phase_deg` (angle of the first blade from +X towards +Z, default 90 =
    straight up), `name` (default Propeller; Propeller_L/_R for twins),
    `spinner` (spinner radius, default 0.17), `spinner_length` (default 0.5).
    Proposed generalization: a `phase_deg` parameter on the shared `propeller`.
    """
    x, y, z = p.req("position")
    r = p.req("diameter") / 2
    n = p.get("blades", 3)
    phase = math.radians(p.get("phase_deg", 90.0))
    name = p.get("name", "Propeller")
    sr = p.get("spinner", 0.17)
    sl = p.get("spinner_length", 0.5)
    mb = model.mb(name, pivot=(x, y, z))
    mb.revolve((x, y - sl / 2, z), (0, 1, 0),
               [(0, sr), (sl * 0.4, sr * 0.94), (sl * 0.8, sr * 0.53), (sl, 0.01)], seg=12, mat="Paint")
    for i in range(n):
        a = phase + 2 * math.pi * i / n
        d = (math.cos(a), math.sin(a))
        tip = (x + d[0] * r, y, z + d[1] * r)
        root = (x + d[0] * 0.12, y, z + d[1] * 0.12)
        w = 0.11
        pts = [(root[0] - d[1] * w, y + 0.02, root[2] + d[0] * w), (tip[0] - d[1] * w * 0.5, y, tip[2] + d[0] * w * 0.5),
               (tip[0] + d[1] * w * 0.5, y - 0.02, tip[2] - d[0] * w * 0.5),
               (root[0] + d[1] * w, y - 0.02, root[2] - d[0] * w)]
        mb.extrude(pts, (0, -0.03, 0), "Metal")
