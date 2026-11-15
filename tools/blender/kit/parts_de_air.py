"""Faction parts of `de` for prompt 008, session `air`: aircraft.

Owned by that session only (one module per parallel session, so the three
de sessions never edit the same file). Part names carry the faction prefix
like `parts_de.py`: `@part("de.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("de_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401
