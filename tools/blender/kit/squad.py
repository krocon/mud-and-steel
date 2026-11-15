"""Infantry squads: a blueprint with `assembly = "squad"` lists soldiers of
prompt 005 (`infantry/<role>.glb`); the build writes a Godot scene that places
them in formation (prompt 006). Pure Python, no bpy.

Blueprint (`assets-src/blueprints/<faction>/<squad_id>.toml`):

    unit = "su_inf_rifle_squad"
    assembly = "squad"
    formation = "wedge"        # wedge | line | column | pair
    spacing = 2.5              # m between neighbours
    [[member]]
    role = "squad_leader"      # role from tools/blender/soldier/loadouts.py
    weapon = "PPSh-41"         # catalog weapon name this soldier carries
    count = 1                  # optional, default 1

Output `godot/assets/models/<faction>/<squad_id>.tscn`: root `Node3D` named
after the id with script `SquadModel` (res://scenes/tools/squad_model.gd),
meta `extras` (the catalog values, same keys as kit models) and `source_hash`,
one instance of `infantry/<role>.glb` per soldier named `M<NN>_<role>`.
The first member stands at the formation point (leader); front is -Z (Godot).
"""

import json
import sys
from pathlib import Path

from . import catalog
from .sourcehash import source_hash

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from soldier import loadouts  # noqa: E402

SCRIPT = "res://scenes/tools/squad_model.gd"
FORMATIONS = ("wedge", "line", "column", "pair")


def is_squad(faction, unit_id):
    return catalog.blueprint(faction, unit_id).get("assembly") == "squad"


def tscn_path(faction, unit_id):
    return catalog.MODEL_DIR / faction / f"{unit_id}.tscn"


def soldier_glb(faction, role):
    return catalog.MODEL_DIR / faction / "infantry" / f"{role}.glb"


def soldier_blend(faction, role):
    return catalog.BLEND_DIR / faction / "infantry" / f"{role}.blend"


def members(bp):
    """Expanded member list: [(role, catalog weapon name), ...] in blueprint order."""
    out = []
    for i, m in enumerate(bp.get("member", [])):
        unknown = set(m) - {"role", "weapon", "count"}
        if unknown:
            raise KeyError(f"[[member]] #{i + 1}: unknown keys {sorted(unknown)}")
        out += [(m["role"], m["weapon"])] * int(m.get("count", 1))
    return out


def slots(formation, spacing, n):
    """Formation slots (x right, y forward) in meters, centered on the origin."""
    if formation not in FORMATIONS:
        raise ValueError(f"unknown formation {formation!r}; known: {', '.join(FORMATIONS)}")
    pts = []
    for i in range(n):
        if formation == "wedge":
            row, side = (i + 1) // 2, (-1 if i % 2 else 1)
            pts.append((side * row * spacing * 0.8 if i else 0.0, -row * spacing * 0.6))
        elif formation == "line":
            pts.append(((i - (n - 1) / 2) * spacing, 0.0))
        elif formation == "column":
            pts.append((0.0, -i * spacing))
        else:  # pair: two abreast, further pairs behind
            pts.append(((-0.5 if i % 2 == 0 else 0.5) * spacing, -(i // 2) * spacing))
    cx = (max(p[0] for p in pts) + min(p[0] for p in pts)) / 2
    cy = (max(p[1] for p in pts) + min(p[1] for p in pts)) / 2
    return [(round(x - cx, 3), round(y - cy, 3)) for x, y in pts]


def extras(u, mem):
    e = catalog.summary(u)
    e["cat_squad_size"] = u.get("squad_size", len(mem))
    e["squad_members"] = [r for r, _ in mem]
    return e


def build(faction, unit_id):
    """Writes the squad scene. Returns its path."""
    u = catalog.unit(faction, unit_id)
    bp = catalog.blueprint(faction, unit_id)
    mem = members(bp)
    pts = slots(bp.get("formation", "wedge"), float(bp.get("spacing", 2.0)), len(mem))
    roles = sorted({r for r, _ in mem})
    for r in roles:
        loadouts.loadout(faction, r)  # raises for unknown roles
        if not soldier_glb(faction, r).exists():
            raise FileNotFoundError(f"{soldier_glb(faction, r)} missing (run `just soldier {faction} {r}`)")
    res = {r: i + 2 for i, r in enumerate(roles)}
    lines = [f"[gd_scene load_steps={len(roles) + 2} format=3]", ""]
    lines.append(f'[ext_resource type="Script" path="{SCRIPT}" id="1_squad"]')
    for r in roles:
        lines.append(f'[ext_resource type="PackedScene" path="res://assets/models/{faction}/infantry/{r}.glb" '
                     f'id="{res[r]}_{r}"]')
    lines += ["", f'[node name="{unit_id}" type="Node3D"]', 'script = ExtResource("1_squad")',
              f'faction = "{faction}"',
              f"metadata/extras = {json.dumps(extras(u, mem), ensure_ascii=False)}",
              f'metadata/source_hash = "{source_hash(faction, unit_id)}"', ""]
    for i, ((r, _), (x, y)) in enumerate(zip(mem, pts)):
        # Blender/kit model space (x right, y forward) -> Godot (x right, -z forward)
        lines += [f'[node name="M{i + 1:02d}_{r}" parent="." instance=ExtResource("{res[r]}_{r}")]',
                  f"transform = Transform3D(1, 0, 0, 0, 1, 0, 0, 0, 1, {x + 0.0:g}, 0, {0.0 - y:g})", ""]
    path = tscn_path(faction, unit_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines))
    return path


def validate(faction, unit_id):
    """Errors of one squad (list of strings) and stats."""
    errors, stats = [], {}
    u = catalog.unit(faction, unit_id)
    bp = catalog.blueprint(faction, unit_id)
    try:
        mem = members(bp)
        slots(bp.get("formation", "wedge"), float(bp.get("spacing", 2.0)), len(mem))
    except (KeyError, ValueError) as exc:
        return [str(exc)], stats
    stats["members"] = len(mem)
    if u.get("squad_size") != len(mem):
        errors.append(f"{len(mem)} members, catalog squad_size {u.get('squad_size')}")
    want = {w["name"]: w.get("count", 1) for w in u.get("weapon", [])}
    have = {}
    for _, w in mem:
        have[w] = have.get(w, 0) + 1
    if want != have:
        errors.append(f"member weapons {have} differ from the catalog {want}")
    for r in sorted({r for r, _ in mem}):
        try:
            loadouts.loadout(faction, r)
        except KeyError as exc:
            errors.append(str(exc))
            continue
        for p in (soldier_blend(faction, r), soldier_glb(faction, r)):
            if not p.exists():
                errors.append(f"{p.relative_to(catalog.REPO_ROOT)} missing (run `just soldier {faction} {r}`)")
    path = tscn_path(faction, unit_id)
    if not path.exists():
        errors.append(f"{path.relative_to(catalog.REPO_ROOT)} missing (run `just unit {faction} {unit_id}`)")
    elif f'metadata/source_hash = "{source_hash(faction, unit_id)}"' not in path.read_text():
        errors.append(f"squad scene is stale (run `just unit {faction} {unit_id}`)")
    expected = f"assets/models/{faction}/{unit_id}.tscn"
    if u.get("model", "") != expected:
        errors.append(f"catalog entry model = {u.get('model', '')!r}, expected {expected!r}")
    return errors, stats
