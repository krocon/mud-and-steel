#!/usr/bin/env python3
"""Blender-free glTF 2.0 (.glb) asset validator for CI and `just glb-check` (prompt 216).

Parses only the JSON chunk of every `.glb` under `godot/assets/models/` using
the Python 3 standard library (Python 3.10+, no Blender, no third-party
packages). Verifies:

- Valid glTF 2.0 binary container (magic, version, chunk lengths, JSON)
- No embedded texture images (`images == []`)
- Unit node scales (`[1, 1, 1]` within 1e-3)
- Allowed material slots (docs/ASSETS.md; explicit exception for placeholder_tank.glb)
- `COLOR_0` and `TEXCOORD_0` attributes on every mesh primitive
- Animation hygiene: 0 animations in soldier GLBs + valid `anim_library` extra;
  61 expected clips in each shared library under `infantry/anims/*.glb`
- Triangle budgets per catalog category / override (vehicles total, env per
  variant and `*_Far`, soldiers body+gear <= 3000 and weapon <= 800)

Exit code 0 if all GLBs pass, 1 otherwise.
"""

from __future__ import annotations

import json
import re
import struct
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_DIR = REPO_ROOT / "godot" / "assets" / "models"
CATALOG_DIR = REPO_ROOT / "data" / "catalog"
ANIM_META_PATH = MODELS_DIR / "infantry" / "animations.json"

# Dev-only placeholder assets exempted from production slot/attribute/budget rules.
DEV_EXCEPTIONS = {
    "placeholder_tank.glb": "legacy dev placeholder built by tools/create_placeholder_tank.py",
}

VEHICLE_SLOTS = frozenset({
    "Paint", "Metal", "Rubber", "Wood", "Canvas", "Glass", "Skin", "Cloth",
})
ENV_SLOTS = frozenset({
    "Paint", "Metal", "Rubber", "Wood", "Canvas", "Glass",
    "Bark", "Foliage", "Stone", "Plaster", "Ground", "Roof", "Rock",
})
SOLDIER_SLOTS = frozenset({
    "Paint", "Metal", "Rubber", "Wood", "Canvas", "Glass",
    "Skin", "Cloth", "Leather", "Insignia",
})

# Mirror of tools/blender/kit/catalog.py (docs/ASSETS.md).
VEHICLE_BUDGETS = {
    "tank_light": 8000, "tank_medium": 8000, "tank_heavy": 8000,
    "tank_destroyer": 8000, "assault_gun": 8000, "halftrack": 8000,
    "self_propelled": 8000, "truck": 5000, "recon_vehicle": 5000,
    "gun_at": 3000, "gun_aa_light": 3000, "gun_aa_heavy": 3000,
    "gun_howitzer": 3000, "mortar_heavy": 3000,
    "aircraft_fighter": 6000, "aircraft_attack": 6000, "aircraft_recon": 6000,
    "aircraft_bomber": 10000, "bridgelayer": 8000, "aircraft_transport": 10000,
    "rocket_launcher": 8000, "boat_landing": 5000, "boat_assault": 3000,
}

# Mirror of tools/blender/kit/env.py (docs/ASSETS.md), per variant.
ENV_BUDGETS = {
    "env_tree": 2000, "env_bush": 500, "env_grass": 50, "env_rock": 300,
    "env_building": 3000, "env_bridge": 3000, "env_fence": 2000, "env_pole": 800,
    "env_prop": 800,
    "fort_bunker": 2000, "fort_trench": 2000, "fort_sandbag": 2000,
    "fort_obstacle": 2000, "fort_wire": 2000, "fort_minefield_at": 800,
    "fort_minefield_ap": 800,
}
ENV_BUDGET_OVERRIDES = {
    "env_church": 5000, "env_church_large": 5000, "env_dump_ammo": 1500,
    "env_dump_fuel": 1500, "env_mosque": 5000, "env_police_post": 5000,
    "env_rock_outcrop": 1500, "env_rock_massif": 3000,
    "env_block_desert": 6000, "env_block_desert_long": 6000, "env_block_town": 6000,
    "env_block_tenement": 6000, "env_loft_iron": 6000, "env_warehouse_quay": 6000,
    "env_house_med_row_corner": 4500, "env_town_hall_med": 6000,
    "env_castle_hall": 5000,
    "env_block_tenement_stone": 6000, "env_palace": 6000, "env_cathedral": 6000,
    "env_station_hall": 5000, "env_mine_headframe": 4000, "env_station": 4500,
    "env_spoil_heap": 600,
}
ENV_FAR_BUDGET = 200
ENV_FAR_BUDGET_OVERRIDES = {"env_rock_massif": 300}

SOLDIER_BODY_BUDGET = 3000
SOLDIER_WEAPON_BUDGET = 800
SCALE_EPS = 1e-3

_UNIT_HEADER_RE = re.compile(r"^\[\[unit\]\]\s*$")
_STR_FIELD_RE = re.compile(r'^(id|category|model)\s*=\s*"([^"]*)"')


def load_catalog_index() -> dict[str, dict[str, str]]:
    """Parses unit `id`, `category`, and `model` from `data/catalog/*.toml`.

    Pure regex parser so this script runs on Python 3.10+ without `tomllib`.
    """
    entries: dict[str, dict[str, str]] = {}
    for toml_path in sorted(CATALOG_DIR.glob("*.toml")):
        current: dict[str, str] | None = None
        for raw_line in toml_path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if _UNIT_HEADER_RE.match(line):
                if current and "id" in current:
                    entries[current["id"]] = current
                current = {}
                continue
            if line.startswith("["):
                # Subtable like [[unit.weapon]] — stop capturing top-level unit fields.
                if current and "id" in current:
                    entries[current["id"]] = current
                current = None
                continue
            if current is not None:
                m = _STR_FIELD_RE.match(line)
                if m:
                    current[m.group(1)] = m.group(2)
        if current and "id" in current:
            entries[current["id"]] = current
    return entries


def read_glb_json(path: Path) -> tuple[dict | None, str | None]:
    """Reads and validates the GLB container header and JSON chunk."""
    size = path.stat().st_size
    if size < 20:
        return None, f"file too small ({size} bytes)"
    with path.open("rb") as f:
        header = f.read(20)
        magic = header[:4]
        if magic != b"glTF":
            if header.startswith(b"version https://git-lfs.github.com/spec/"):
                return None, "unexpanded Git LFS pointer (run `git lfs pull`)"
            return None, f"invalid GLB magic {magic!r}"
        version, total_len, chunk0_len, chunk0_type = struct.unpack("<IIII", header[4:20])
        if version != 2:
            return None, f"unsupported glTF version {version} (expected 2)"
        if total_len != size:
            return None, f"GLB header length {total_len} != file size {size}"
        if chunk0_type != 0x4E4F534A:  # 'JSON'
            return None, f"first chunk is not JSON (0x{chunk0_type:08x})"
        if 20 + chunk0_len > size:
            return None, f"JSON chunk length {chunk0_len} exceeds file size {size}"
        json_bytes = f.read(chunk0_len)
    try:
        gltf = json.loads(json_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        return None, f"invalid JSON chunk: {exc}"
    if not isinstance(gltf, dict):
        return None, "JSON root is not an object"
    asset_ver = gltf.get("asset", {}).get("version", "")
    if not str(asset_ver).startswith("2."):
        return None, f"asset.version is {asset_ver!r} (expected 2.x)"
    return gltf, None


def prim_triangles(prim: dict, accessors: list[dict]) -> int:
    """Computes triangle count of a glTF mesh primitive from accessor metadata."""
    mode = prim.get("mode", 4)
    if "indices" in prim:
        count = accessors[prim["indices"]]["count"]
    else:
        pos_idx = prim.get("attributes", {}).get("POSITION")
        if pos_idx is None:
            return 0
        count = accessors[pos_idx]["count"]
    if mode == 4:  # TRIANGLES
        return count // 3
    if mode in (5, 6):  # TRIANGLE_STRIP, TRIANGLE_FAN
        return max(0, count - 2)
    return 0


def mesh_triangles(mesh_idx: int, meshes: list[dict], accessors: list[dict]) -> int:
    return sum(prim_triangles(p, accessors) for p in meshes[mesh_idx].get("primitives", []))


def subtree_triangles(node_idx: int, nodes: list[dict], meshes: list[dict], accessors: list[dict]) -> int:
    total = 0
    stack = [node_idx]
    while stack:
        idx = stack.pop()
        node = nodes[idx]
        if "mesh" in node:
            total += mesh_triangles(node["mesh"], meshes, accessors)
        stack.extend(node.get("children", ()))
    return total


def check_common(gltf: dict, errors: list[str]) -> None:
    """Checks common invariants for every GLB: no images, unit node scales."""
    images = gltf.get("images", [])
    if images:
        errors.append(f"contains {len(images)} embedded image(s) (expected 0)")
    for idx, node in enumerate(gltf.get("nodes", [])):
        scale = node.get("scale")
        if scale is not None:
            if len(scale) != 3 or any(abs(float(s) - 1.0) > SCALE_EPS for s in scale):
                name = node.get("name", f"node[{idx}]")
                errors.append(f"node {name!r} has non-unit scale {tuple(round(float(s), 4) for s in scale)}")


def check_materials_and_attrs(gltf: dict, allowed_slots: frozenset[str], errors: list[str]) -> None:
    """Checks material slot names and required COLOR_0 / TEXCOORD_0 attributes."""
    materials = gltf.get("materials", [])
    for idx, mat in enumerate(materials):
        name = mat.get("name", "")
        if name not in allowed_slots:
            errors.append(f"material[{idx}] {name!r} is not an allowed slot")
    meshes = gltf.get("meshes", [])
    if not meshes:
        errors.append("no meshes in GLB")
        return
    for midx, mesh in enumerate(meshes):
        mname = mesh.get("name", f"mesh[{midx}]")
        prims = mesh.get("primitives", [])
        if not prims:
            errors.append(f"mesh {mname!r} has no primitives")
        for pidx, prim in enumerate(prims):
            attrs = prim.get("attributes", {})
            if "COLOR_0" not in attrs:
                errors.append(f"mesh {mname!r} prim[{pidx}]: COLOR_0 missing")
            if "TEXCOORD_0" not in attrs:
                errors.append(f"mesh {mname!r} prim[{pidx}]: TEXCOORD_0 missing")
            if "material" not in prim:
                errors.append(f"mesh {mname!r} prim[{pidx}]: no material assigned")


def find_scene_roots(gltf: dict) -> list[int]:
    scenes = gltf.get("scenes", [])
    if not scenes:
        return []
    scene_idx = gltf.get("scene", 0)
    if 0 <= scene_idx < len(scenes):
        return list(scenes[scene_idx].get("nodes", []))
    return list(scenes[0].get("nodes", []))


def check_anim_library(path: Path, gltf: dict, expected_clips: set[str], errors: list[str]) -> None:
    if gltf.get("meshes"):
        errors.append(f"shared animation library contains {len(gltf['meshes'])} mesh(es) (expected 0)")
    anims = gltf.get("animations", [])
    names = {a.get("name", "") for a in anims}
    missing = sorted(expected_clips - names)
    if missing:
        errors.append(f"missing {len(missing)} clip(s): {missing[:5]}")
    if len(anims) != len(expected_clips):
        errors.append(f"has {len(anims)} animation(s), expected {len(expected_clips)}")


def check_soldier(path: Path, gltf: dict, anim_libs: set[str], errors: list[str]) -> None:
    check_materials_and_attrs(gltf, SOLDIER_SLOTS, errors)
    anims = gltf.get("animations", [])
    if anims:
        errors.append(f"soldier GLB embeds {len(anims)} animation(s) (expected 0; use shared anim library)")
    nodes = gltf.get("nodes", [])
    meshes = gltf.get("meshes", [])
    accessors = gltf.get("accessors", [])

    # Check anim_library root extra
    roots = find_scene_roots(gltf)
    anim_lib = None
    for r in roots:
        extras = nodes[r].get("extras")
        if isinstance(extras, dict) and "anim_library" in extras:
            anim_lib = extras["anim_library"]
            break
    if not anim_lib:
        errors.append("root extra 'anim_library' missing")
    elif anim_lib not in anim_libs:
        errors.append(f"anim_library {anim_lib!r} not found in infantry/anims/")

    # Triangle budgets: Body + Gear <= 3000, Weapon_* <= 800
    body_gear_tris = 0
    for node in nodes:
        if "mesh" not in node:
            continue
        name = node.get("name", "")
        tris = mesh_triangles(node["mesh"], meshes, accessors)
        if name.startswith("Weapon_"):
            if tris > SOLDIER_WEAPON_BUDGET:
                errors.append(f"{name}: {tris} triangles > weapon budget {SOLDIER_WEAPON_BUDGET}")
        else:
            body_gear_tris += tris
    if body_gear_tris > SOLDIER_BODY_BUDGET:
        errors.append(f"body+gear: {body_gear_tris} triangles > budget {SOLDIER_BODY_BUDGET}")


def check_env(uid: str, category: str, gltf: dict, errors: list[str]) -> None:
    check_materials_and_attrs(gltf, ENV_SLOTS, errors)
    nodes = gltf.get("nodes", [])
    meshes = gltf.get("meshes", [])
    accessors = gltf.get("accessors", [])
    budget = ENV_BUDGET_OVERRIDES.get(uid, ENV_BUDGETS.get(category, 2000))
    far_budget = ENV_FAR_BUDGET_OVERRIDES.get(uid, ENV_FAR_BUDGET)

    roots = find_scene_roots(gltf)
    variant_nodes: list[int] = []
    if len(roots) == 1 and nodes[roots[0]].get("children"):
        variant_nodes = list(nodes[roots[0]]["children"])
    else:
        variant_nodes = roots

    for vidx in variant_nodes:
        vname = nodes[vidx].get("name", f"node[{vidx}]")
        if vname.startswith("Socket_"):
            continue
        tris = subtree_triangles(vidx, nodes, meshes, accessors)
        if vname.endswith("_Far"):
            if tris > far_budget:
                errors.append(f"far variant {vname!r}: {tris} triangles > far budget {far_budget}")
        elif tris > budget:
            errors.append(f"variant {vname!r}: {tris} triangles > budget {budget}")


def check_vehicle(uid: str, category: str, gltf: dict, errors: list[str]) -> None:
    check_materials_and_attrs(gltf, VEHICLE_SLOTS, errors)
    meshes = gltf.get("meshes", [])
    accessors = gltf.get("accessors", [])
    total_tris = sum(mesh_triangles(i, meshes, accessors) for i in range(len(meshes)))
    budget = VEHICLE_BUDGETS.get(category)
    if budget is None:
        errors.append(f"no vehicle triangle budget defined for category {category!r}")
    elif total_tris > budget:
        errors.append(f"{total_tris} triangles > budget {budget} ({category})")


def main() -> int:
    t0 = time.perf_counter()
    if not MODELS_DIR.is_dir():
        print(f"glb_check: models directory not found: {MODELS_DIR}", file=sys.stderr)
        return 1

    catalog = load_catalog_index()
    if not ANIM_META_PATH.is_file():
        print(f"glb_check: missing {ANIM_META_PATH.relative_to(REPO_ROOT)}", file=sys.stderr)
        return 1
    anim_meta = json.loads(ANIM_META_PATH.read_text(encoding="utf-8"))
    expected_clips = set(anim_meta.get("clips", {}).keys())

    glbs = sorted(MODELS_DIR.rglob("*.glb"))
    if not glbs:
        print("glb_check: no .glb files found", file=sys.stderr)
        return 1

    anims_dir = MODELS_DIR / "infantry" / "anims"
    anim_libs = {p.stem for p in anims_dir.glob("*.glb")} if anims_dir.is_dir() else set()

    counts = {"vehicle": 0, "env": 0, "soldier": 0, "anim_lib": 0, "exception": 0}
    failed = 0

    for path in glbs:
        rel = path.relative_to(MODELS_DIR).as_posix()
        parts = path.relative_to(MODELS_DIR).parts
        errors: list[str] = []

        gltf, parse_err = read_glb_json(path)
        if parse_err is not None or gltf is None:
            errors.append(parse_err or "failed to parse GLB")
        else:
            check_common(gltf, errors)
            if rel in DEV_EXCEPTIONS:
                counts["exception"] += 1
                if not gltf.get("meshes"):
                    errors.append("placeholder GLB has no meshes")
            elif len(parts) == 3 and parts[0] == "infantry" and parts[1] == "anims":
                counts["anim_lib"] += 1
                check_anim_library(path, gltf, expected_clips, errors)
            elif len(parts) == 3 and parts[1] == "infantry":
                counts["soldier"] += 1
                check_soldier(path, gltf, anim_libs, errors)
            elif len(parts) == 2:
                uid = path.stem
                entry = catalog.get(uid)
                if entry is None:
                    errors.append(f"no catalog entry found for {uid!r}")
                else:
                    category = entry.get("category", "")
                    if parts[0] == "neutral" or category.startswith(("env_", "fort_")):
                        counts["env"] += 1
                        check_env(uid, category, gltf, errors)
                    else:
                        counts["vehicle"] += 1
                        check_vehicle(uid, category, gltf, errors)
            else:
                errors.append(f"unexpected model path layout: {rel}")

        if errors:
            failed += 1
            print(f"FAIL {rel}")
            for e in errors:
                print(f"  - {e}")

    elapsed = time.perf_counter() - t0
    summary = (
        f"{len(glbs)} GLBs checked in {elapsed:.2f}s "
        f"({counts['vehicle']} vehicles, {counts['env']} env, "
        f"{counts['soldier']} soldiers, {counts['anim_lib']} anim libs, "
        f"{counts['exception']} dev exception)"
    )
    if failed:
        print(f"glb_check: {failed} file(s) failed ({summary})", file=sys.stderr)
        return 1
    print(f"glb_check: ok — {summary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
