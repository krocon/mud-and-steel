"""Registry of parts and assemblies.

A *part* is a function `fn(model, params, ctx) -> dict | None` that adds geometry
and sockets to a `core.Model`. `params` is the blueprint table of the part,
`ctx` a dict with derived values of the assembly (catalog dimensions, hull top,
turret ring position, ...). The returned dict is merged into `ctx`, so later
parts can build on earlier ones.

An *assembly* builds a whole unit: `fn(model, blueprint, unit) -> None`.

Generic parts have plain names (`leaf_spring_bogie`), faction parts a prefix
(`de.pz4_turret`). Parts are only added, never renamed (docs/ASSETS.md,
"Parallel work").
"""

PARTS = {}
ASSEMBLIES = {}


def part(name):
    def deco(fn):
        if name in PARTS:
            raise ValueError(f"part {name} registered twice")
        PARTS[name] = fn
        return fn

    return deco


def assembly(name):
    def deco(fn):
        if name in ASSEMBLIES:
            raise ValueError(f"assembly {name} registered twice")
        ASSEMBLIES[name] = fn
        return fn

    return deco


def build_part(name, model, table, ctx, where=None):
    """Runs a part. Unknown blueprint parameters are errors (typo protection)."""
    if name not in PARTS:
        raise KeyError(f"unknown part {name!r}; known: {', '.join(sorted(PARTS))}")
    params = Params(table, where or name)
    result = PARTS[name](model, params, ctx)
    bad = params.unused()
    if bad:
        raise KeyError(f"{params.where}: unknown parameters {bad} for part {name}")
    if result:
        ctx.update(result)
    return result


class Params:
    """Read access to a blueprint table with defaults and clear errors."""

    def __init__(self, table, where):
        self.t = dict(table or {})
        self.where = where
        self.used = set()

    def get(self, key, default=None):
        self.used.add(key)
        return self.t.get(key, default)

    def req(self, key):
        self.used.add(key)
        if key not in self.t:
            raise KeyError(f"{self.where}: missing parameter {key!r}")
        return self.t[key]

    def __contains__(self, key):
        return key in self.t

    def unused(self):
        return sorted(set(self.t) - self.used - {"part"})
