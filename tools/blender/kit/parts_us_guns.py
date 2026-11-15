"""Faction parts of `us` for prompt 008, session `guns`: AA guns, howitzers, heavy mortars and fortifications (env generators with @env_part).

Owned by that session only (one module per parallel session, so the three
us sessions never edit the same file). Part names carry the faction prefix
like `parts_us.py`: `@part("us.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("us_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401
