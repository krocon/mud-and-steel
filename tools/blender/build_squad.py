"""Assembles infantry squads from the soldiers of prompt 005 (faction-agnostic).

A squad is not a new mesh: it is a Godot scene that places the soldier models
(`godot/assets/models/<faction>/infantry/<role>.glb`) in a formation. The
composition is a small TOML file next to the blueprints:

    assets-src/blueprints/<faction>/squads/<id>.toml

    unit = "us_inf_rifle_squad"      # catalog id, must match the file name
    [[member]]
    role = "squad_leader"            # role from tools/blender/soldier/loadouts.py
    position = [0.0, 2.0]            # meters, x right, y forward (Blender model space)
    heading_deg = 0                  # optional, turn to the left (counter-clockwise)
    clip = "idle"                    # optional gallery clip (animations.json)
    note = "squad leader (M1)"       # what the member is in the T/O&E

The sub-folder keeps squads out of the kit tools, which only read
`blueprints/<faction>/*.toml`.

Output: `godot/assets/models/<faction>/<id>.tscn` (root Node3D named after the id,
meta `extras` with the catalog values like the kit's glTF extras, plus
`squad = true`, `squad_roles`, `squad_notes`, `squad_clips`, `source_hash`).
The catalog entry gets `model = "assets/models/<faction>/<id>.tscn"`.

Run (pure Python, Blender is only used for its Python 3.11 with tomllib):
    blender -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- <faction> <id>
    blender -b ... --python tools/blender/build_squad.py -- --all
    blender -b ... --python tools/blender/build_squad.py -- --check   (validate only)
    blender -b ... --python tools/blender/build_squad.py -- --roles [<faction>]
        (overview, prompt 008: every role of the faction with its weapon and whether its
        soldier .glb is built, and the catalog infantry entries that have no squad file yet)

Checks: member count = catalog `squad_size`, every role exists for the faction and
its .glb is built, clips exist, members at least 0.8 m apart, the .tscn is current.
"""

import hashlib
import json
import math
import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from kit import catalog  # noqa: E402  (pure Python module)
from soldier import loadouts  # noqa: E402  (pure Python module)

CLIPS = REPO_ROOT / "godot" / "assets" / "models" / "infantry" / "animations.json"
MIN_SPACING = 0.8


def squad_dir(faction):
    return catalog.BLUEPRINT_DIR / faction / "squads"


def squad_path(faction, unit_id):
    return squad_dir(faction) / f"{unit_id}.toml"


def scene_path(faction, unit_id):
    return catalog.MODEL_DIR / faction / f"{unit_id}.tscn"


def model_field(faction, unit_id):
    return f"assets/models/{faction}/{unit_id}.tscn"


def all_squads():
    out = []
    for faction in catalog.FACTIONS:
        d = squad_dir(faction)
        if d.is_dir():
            out += [(faction, p.stem) for p in sorted(d.glob("*.toml"))]
    return out


def load(faction, unit_id):
    path = squad_path(faction, unit_id)
    with open(path, "rb") as f:
        sq = tomllib.load(f)
    if sq.get("unit") != unit_id:
        raise ValueError(f"{path}: 'unit' must be {unit_id!r}")
    return sq


def source_hash(faction, unit_id):
    u = dict(catalog.unit(faction, unit_id))
    u.pop("model", None)
    h = hashlib.sha256()
    h.update(squad_path(faction, unit_id).read_bytes())
    h.update(json.dumps(u, sort_keys=True, ensure_ascii=False).encode())
    return h.hexdigest()[:16]


def soldier_glb(faction, role):
    return catalog.MODEL_DIR / faction / "infantry" / f"{role}.glb"


def check(faction, unit_id, sq=None):
    """Returns a list of errors (empty = ok)."""
    errors = []
    try:
        u = catalog.unit(faction, unit_id)
        sq = sq or load(faction, unit_id)
    except Exception as exc:  # noqa: BLE001
        return [str(exc)]
    members = sq.get("member", [])
    size = u.get("squad_size")
    if size is None:
        errors.append("catalog entry has no squad_size")
    elif len(members) != size:
        errors.append(f"{len(members)} members, catalog squad_size {size}")
    clips = json.loads(CLIPS.read_text()).get("clips", {}) if CLIPS.exists() else {}
    allowed = {"role", "position", "heading_deg", "clip", "note"}
    for i, m in enumerate(members, start=1):
        bad = sorted(set(m) - allowed)
        if bad:
            errors.append(f"member {i}: unknown keys {bad}")
        role = m.get("role")
        if role not in loadouts.ROLES or faction not in loadouts.ROLES[role]:
            errors.append(f"member {i}: no role {role!r} for {faction}")
        elif not soldier_glb(faction, role).exists():
            errors.append(f"member {i}: {soldier_glb(faction, role).relative_to(REPO_ROOT)} missing "
                          f"(run `just soldier {faction} {role}`)")
        clip = m.get("clip", "idle")
        if clips and clip not in clips:
            errors.append(f"member {i}: unknown clip {clip!r}")
        if len(m.get("position", [])) != 2:
            errors.append(f"member {i}: position must be [x, y]")
    pos = [m.get("position", [0, 0]) for m in members]
    for i in range(len(pos)):
        for j in range(i + 1, len(pos)):
            if math.dist(pos[i], pos[j]) < MIN_SPACING:
                errors.append(f"members {i + 1} and {j + 1} closer than {MIN_SPACING} m")
    tscn = scene_path(faction, unit_id)
    if not tscn.exists():
        errors.append(f"{tscn.relative_to(REPO_ROOT)} missing (run `just squad {faction} {unit_id}`)")
    elif f'"source_hash": "{source_hash(faction, unit_id)}"' not in tscn.read_text():
        errors.append(f"{tscn.name} is stale: squad file or catalog entry changed (run `just squad {faction} "
                      f"{unit_id}`)")
    if u.get("model") != model_field(faction, unit_id):
        errors.append(f"catalog entry model = {u.get('model')!r}, expected {model_field(faction, unit_id)!r}")
    return errors


def _uid(glb):
    imp = glb.with_name(glb.name + ".import")
    if imp.exists():
        m = re.search(r'^uid="(uid://[a-z0-9]+)"', imp.read_text(), re.M)
        if m:
            return m.group(1)
    return None


def _gd(value):
    """Godot text-resource literal for metadata values (str, int, float, bool, list)."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(_gd(v) for v in value) + "]"
    raise TypeError(f"cannot write {value!r}")


def _transform(x, y, heading_deg):
    # Blender model space (x right, y forward, z up) -> Godot (x right, -z forward, y up).
    a = math.radians(heading_deg)
    c, s = math.cos(a), math.sin(a)
    # rotation around Godot Y by +a (a left turn, front is -Z): basis columns
    # X = (c, 0, -s), Y = (0, 1, 0), Z = (s, 0, c)
    basis = (c, 0.0, -s, 0.0, 1.0, 0.0, s, 0.0, c)
    nums = [round(v, 6) + 0.0 for v in basis] + [round(x, 4) + 0.0, 0.0, round(-y, 4) + 0.0]
    return "Transform3D(" + ", ".join(f"{v:g}" for v in nums) + ")"


def build(faction, unit_id):
    sq = load(faction, unit_id)
    errors = [e for e in check(faction, unit_id, sq) if "missing (run `just squad" not in e
              and "is stale" not in e and "catalog entry model" not in e]
    if errors:
        raise SystemExit(f"build_squad: {faction}/{unit_id}:\n  - " + "\n  - ".join(errors))
    u = catalog.unit(faction, unit_id)
    members = sq["member"]
    extras = catalog.summary(u)
    extras.update({
        "cat_squad_size": u["squad_size"],
        "squad": True,
        "squad_roles": [m["role"] for m in members],
        "squad_notes": [m.get("note", "") for m in members],
        "squad_clips": [m.get("clip", "idle") for m in members],
        "squad_file": str(squad_path(faction, unit_id).relative_to(REPO_ROOT)),
        "source_hash": source_hash(faction, unit_id),
    })
    roles = sorted({m["role"] for m in members})
    lines = [f"[gd_scene load_steps={len(roles) + 1} format=3]", ""]
    ids = {}
    for i, role in enumerate(roles, start=1):
        glb = soldier_glb(faction, role)
        res = f"res://{glb.relative_to(REPO_ROOT / 'godot').as_posix()}"
        uid = _uid(glb)
        uid_attr = f' uid="{uid}"' if uid else ""
        ids[role] = f"{i}_{role}"
        lines.append(f'[ext_resource type="PackedScene"{uid_attr} path="{res}" id="{ids[role]}"]')
    lines += ["", f'[node name="{unit_id}" type="Node3D"]',
              "metadata/extras = {" + ", ".join(f"{_gd(k)}: {_gd(v)}" for k, v in extras.items()) + "}", ""]
    for i, m in enumerate(members, start=1):
        x, y = m["position"]
        lines.append(f'[node name="Member_{i:02d}" parent="." instance=ExtResource("{ids[m["role"]]}")]')
        lines.append(f"transform = {_transform(x, y, m.get('heading_deg', 0.0))}")
        lines.append("")
    out = scene_path(faction, unit_id)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines))
    xs = [m["position"][0] for m in members]
    ys = [m["position"][1] for m in members]
    print(f"build_squad: {faction}/{unit_id}: {len(members)} members ({', '.join(roles)}), "
          f"formation {max(xs) - min(xs):.1f} x {max(ys) - min(ys):.1f} m -> {out.relative_to(REPO_ROOT)}")


def roles_report(factions):
    """Prints the roles per faction and the infantry catalog entries without squad file."""
    for faction in factions:
        print(f"roles {faction}:")
        for role, per in sorted(loadouts.ROLES.items()):
            if faction not in per:
                continue
            spec = per[faction]
            built = "built" if soldier_glb(faction, role).exists() else "NOT BUILT"
            weapon = spec["weapon"] or "unarmed (pistol holster / none)"
            print(f"  {role:22} {weapon:34} {built}")
        have = {uid for f, uid in all_squads() if f == faction}
        todo = sorted(uid for uid, u in catalog.load_faction(faction).items()
                      if u["category"].startswith("inf_") and uid not in have)
        print(f"  infantry entries without squad file: {', '.join(todo) if todo else 'none'}")


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if argv and argv[0] == "--roles":
        roles_report(argv[1:] or ["de", "su", "us"])
        return
    if not argv or argv[0] in ("--all", "--check"):
        squads = all_squads()
        if argv and argv[0] == "--check":
            failed = 0
            print(f"validate squads: {len(squads)} squad(s)")
            for faction, uid in squads:
                errors = check(faction, uid)
                print(f"  {'FAIL' if errors else 'ok':4} {faction}/{uid}")
                for e in errors:
                    print(f"         - {e}")
                failed += bool(errors)
            if failed:
                print(f"validate squads: {failed} squad(s) failed")
                sys.exit(1)
            print("validate squads: all squads ok")
            return
        for faction, uid in squads:
            build(faction, uid)
        return
    if len(argv) != 2:
        sys.exit("usage: build_squad.py -- <faction> <id> | --all | --check | --roles [<faction>]")
    build(argv[0], argv[1])


try:
    main()
except SystemExit:
    raise
except Exception:
    import traceback

    traceback.print_exc()
    sys.exit(1)
