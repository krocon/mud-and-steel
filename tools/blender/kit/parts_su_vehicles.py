"""Faction parts of `su` for prompt 008, session `vehicles`: tanks, tank destroyers, assault guns, SPGs, armored cars and trucks.

Owned by that session only (one module per parallel session, so the three
su sessions never edit the same file). Part names carry the faction prefix
like `parts_su.py`: `@part("su.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("su_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401
