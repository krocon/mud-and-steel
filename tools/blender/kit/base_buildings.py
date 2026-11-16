"""Base buildings (prompt 056): vehicle works, barracks, supply depot.

The base buildings of prompt 023 borrowed village houses. These generators
give them a look of their own, built on the `house` generator of prompt 007
(walls, openings, roof and the three damage levels come from there) with
what makes them military or industrial added on top:

- `steel_hall`   vehicle works: a large hall clad in corrugated steel on a
                 brick plinth, steel pilasters, a roof lantern with glazing,
                 a big sliding door at the gable and a brick chimney.
- `barracks`     a long timber barracks hut: tarred boards, a porch over the
                 door, stove pipes through the roof, duckboards.
- `depot`        an open timber store shed with a steel roof, stacks of
                 crates at one end and 200 l drums at the other.

The variant's box is the catalog box; the house takes a part of it and the
additions the rest. Like every env generator, `fit` scales the result
exactly into the box (damage variants keep the intact plan, `fit = false`).
"""

from mathutils import Vector

from .buildings import house
from .env import Box, env_part, rng
from .registry import Params

METAL_STEEL, METAL_RUST, METAL_GALVANIZED, METAL_DARK = 0, 1, 2, 3


def _roof_to_metal(mb, kind):
    """Turns the roof covering of `mb` into corrugated steel sheets."""
    if mb is None or "Roof" not in mb.slots:
        return
    roof = mb.slots.index("Roof")
    metal = mb._mat("Metal")
    for f in mb.bm.faces:
        if f.material_index == roof:
            f.material_index = metal
            f[mb.kind] = kind


def _crate(mb, x, y, z0, length, width, height):
    """A plain plank crate with a darker lid band (cheap: 24 triangles)."""
    mb.style(kind=0)
    mb.box((x, y, z0 + height / 2), (width * 0.97, length * 0.97, height * 0.97), "Wood")
    mb.style(kind=2)
    mb.box((x, y, z0 + height * 0.8), (width, length * 0.3, height * 0.12), "Wood")


def _drum(mb, base, axis, radius, height, kind=0):
    """A 200 l drum: one 8-sided cylinder in the Paint slot."""
    a = Vector(base)
    d = Vector(axis).normalized()
    mb.style(kind=kind)
    mb.cylinder(a, a + d * height, radius, seg=8, mat="Paint")


def _house_params(p, values):
    """Parameters for `house`: the variant's `seed` and `damage` plus the
    fixed plan of the building type."""
    table = {"seed": p.get("seed", 1), "damage": p.get("damage", 0)}
    table.update(values)
    return Params(table, p.where)


def _parts(model, name):
    """The walls and roof builders of a house variant (roof may be None)."""
    walls = model.mb(f"{name}_Walls", parent=name)
    roof = model.parts.get(f"{name}_Roof", {}).get("mb")
    return walls, roof


@env_part("steel_hall")
def steel_hall(model, name, p, box):
    """Vehicle works: steel-clad hall. Parameters: `seed`, `damage`
    (0, 1, 2); the plan follows `house` (ridge along Y)."""
    damage = p.get("damage", 0)
    r = rng(p.get("seed", 1) + 101)
    hall_h = box.height * 0.8
    hp = _house_params(p, {
        "wall": "Metal", "wall_kind": METAL_GALVANIZED, "wall_t": 0.25, "plinth": 1.0,
        "storeys": 1, "storey_h": hall_h * 0.62, "knee": 0.0, "eave": 0.35, "verge": 0.25,
        "windows_long": 6, "windows_short": 0, "window": [2.2, 1.0], "sill": 2.6,
        "door": ["-x", 1.2], "attic_windows": 0, "roof_kind": 1,
        "chimney": False,
    })
    house(model, name, hp, Box(box.length, box.width, hall_h))
    walls, roof = _parts(model, name)
    _roof_to_metal(roof, METAL_GALVANIZED if damage == 0 else METAL_RUST)
    x0, x1 = -box.hw + 0.35, box.hw - 0.35
    y0, y1 = -box.hl + 0.25, box.hl - 0.25
    wall_top = 1.0 + hall_h * 0.62
    # Steel pilasters every 4 m along the long walls and at the corners.
    walls.style(kind=METAL_DARK)
    n = max(2, int((y1 - y0) / 4.0))
    for i in range(n + 1):
        y = y0 + (y1 - y0) * i / n
        for x, s in ((x0, -1), (x1, 1)):
            top = wall_top if damage < 2 or r.random() < 0.5 else wall_top * r.uniform(0.3, 0.8)
            walls.box((x + s * 0.14, y, top / 2), (0.22, 0.3, top), "Metal")
    # Sliding door at the +Y gable: a dark steel leaf on a rail.
    if damage < 2:
        dw, dh = (x1 - x0) * 0.55, min(4.6, wall_top - 0.4)
        off = 0.0 if damage == 0 else dw * 0.35
        walls.style(kind=METAL_DARK)
        walls.box((off, y1 + 0.18, dh / 2 + 0.05), (dw, 0.12, dh), "Metal")
        walls.box((0, y1 + 0.26, dh + 0.25), (dw * 2.0, 0.12, 0.16), "Metal")
    # Roof lantern along the ridge: glazing under a small steel roof.
    if roof is not None and damage < 2:
        ridge = hall_h - 0.14
        lx = (x1 - x0) * 0.16
        ly0, ly1 = y0 + 2.0, y1 - 2.0
        roof.style(kind=0)
        roof.box((0, (ly0 + ly1) / 2, ridge + 0.35), (lx * 2, ly1 - ly0, 0.7), "Glass")
        roof.style(kind=METAL_GALVANIZED)
        roof.box((0, (ly0 + ly1) / 2, ridge + 0.78), (lx * 2 + 0.5, ly1 - ly0 + 0.3, 0.16), "Metal")
    # Brick chimney at the -Y gable, up to the box height.
    walls.style(kind=3)
    cx, cy = x0 + 1.4, y0 - 0.9
    top = box.height if damage == 0 else (box.height * 0.75 if damage == 1 else 3.0)
    walls.box((cx, cy, top / 2), (1.4, 1.4, top), "Stone")
    walls.box((cx, cy, top - 0.3), (1.7, 1.7, 0.35), "Stone")


@env_part("barracks")
def barracks(model, name, p, box):
    """Timber barracks hut. Parameters: `seed`, `damage`; tarred boards
    (Wood kind dark), plank roof, porch, stove pipes."""
    damage = p.get("damage", 0)
    hp = _house_params(p, {
        "wall": "Wood", "wall_kind": 2, "wall_t": 0.2, "plinth": 0.6, "storeys": 1,
        "storey_h": 3.0, "knee": 1.2, "eave": 0.6, "verge": 0.4, "windows_long": 6,
        "windows_short": 1, "window": [1.0, 1.1], "door": ["-x", 1.4], "attic_windows": 1,
        "roof_kind": 3, "chimney": False,
    })
    house(model, name, hp, Box(box.length, box.width - 1.2, box.height))
    walls, roof = _parts(model, name)
    x0 = -(box.width - 1.2) / 2 + 0.6
    # Porch over the door on the -X long wall: posts and a lean-to roof.
    if damage < 2:
        walls.style(kind=1)
        for y in (-1.6, 1.6):
            walls.box((x0 - 1.15, y, 1.55), (0.14, 0.14, 2.5), "Wood")
        walls.box((x0 - 0.6, 0, 2.9), (1.4, 4.0, 0.1), "Wood")
        walls.box((x0 - 0.6, 0, 0.45), (1.4, 3.6, 0.12), "Wood")
    # Duckboards along the door side.
    walls.style(kind=0)
    for k in range(5):
        y = -box.hl + 1.0 + k * (box.length - 2.0) / 4
        walls.box((x0 - 0.6 + (0.0 if damage < 2 else 0.2), y, 0.06), (0.9, 2.2, 0.08), "Wood")
    # Stove pipes through the roof.
    if roof is not None and damage == 0:
        roof.style(kind=METAL_DARK)
        for y in (-box.hl * 0.45, box.hl * 0.45):
            roof.cylinder((0.9, y, box.height * 0.7), (0.9, y, box.height - 0.05), 0.12, seg=8, mat="Metal")


@env_part("depot")
def depot(model, name, p, box):
    """Supply depot: store shed with a steel roof and wide doors on both
    long sides, crate stacks at the -Y end, 200 l drums at the +Y end.
    Parameters: `seed`, `damage`."""
    damage = p.get("damage", 0)
    r = rng(p.get("seed", 1) + 57)
    shed_l = box.length * 0.62
    hp = _house_params(p, {
        "wall": "Wood", "wall_kind": 0, "wall_t": 0.2, "plinth": 0.4, "storeys": 1,
        "storey_h": 3.8, "knee": 0.6, "eave": 0.6, "verge": 0.5, "windows_long": 2,
        "windows_short": 1, "barn_doors": [3.6, 3.4], "attic_windows": 0, "roof_kind": 1,
        "chimney": False,
    })
    house(model, name, hp, Box(shed_l, box.width * 0.8, box.height))
    walls, roof = _parts(model, name)
    _roof_to_metal(roof, METAL_RUST if damage else METAL_STEEL)
    stores = model.mb(f"{name}_Debris", parent=name) if damage else walls
    end = shed_l / 2 + 0.4
    span = box.hl - end - 0.2
    # Crate stacks on pallets at -Y.
    for i, x in enumerate((-box.hw * 0.55, 0.0, box.hw * 0.55)):
        layers = 3 if damage == 0 else (2 if i != 1 else 1)
        if damage == 2 and r.random() < 0.5:
            layers = 1
        for k in range(layers):
            for j in range(2):
                y = -end - 0.55 - j * (span * 0.5)
                if -y > box.hl - 0.3:
                    continue
                _crate(stores, x + r.uniform(-0.05, 0.05), y, k * 0.62, 1.1, 1.6, 0.6)
    # Drum rows at +Y: standing in rows, a few lying.
    R = 0.292
    for row in range(3):
        for col in range(6):
            x = -box.hw * 0.75 + col * (2 * R + 0.12)
            y = end + 0.5 + row * (2 * R + 0.12)
            if y + R > box.hl - 0.1:
                continue
            if damage and r.random() < 0.3 * damage:
                continue
            _drum(stores, (x, y, 0.0), (0, 0, 1), R, 0.868, kind=3 if col % 3 == 0 else 0)
    for k in range(3):
        x = box.hw * 0.35 + k * 0.62
        _drum(stores, (x, end + 0.4, R), (0, 1, 0), R, 0.86, kind=3)
    if damage == 0:
        # A tarpaulin over the middle crate stack.
        stores.style(kind=0)
        stores.box((0.0, -end - 0.55 - span * 0.25, 3 * 0.62 + 0.05), (1.9, span * 0.9, 0.05), "Canvas")
