"""Reads the unit catalog and blueprints. Pure Python (tomllib), no bpy."""

import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
CATALOG_DIR = REPO_ROOT / "data" / "catalog"
BLUEPRINT_DIR = REPO_ROOT / "assets-src" / "blueprints"
BLEND_DIR = REPO_ROOT / "assets-src" / "blender"
MODEL_DIR = REPO_ROOT / "godot" / "assets" / "models"
FACTIONS = ("de", "su", "us", "uk", "neutral")

# Triangle budgets of docs/ASSETS.md (LOD0) per catalog category.
BUDGETS = {
    "tank_light": 8000, "tank_medium": 8000, "tank_heavy": 8000, "tank_destroyer": 8000,
    "assault_gun": 8000, "halftrack": 8000, "self_propelled": 8000,
    "truck": 5000, "recon_vehicle": 5000,
    "gun_at": 3000, "gun_aa_light": 3000, "gun_aa_heavy": 3000, "gun_howitzer": 3000,
    "mortar_heavy": 3000,
    "aircraft_fighter": 6000, "aircraft_attack": 6000, "aircraft_recon": 6000,
    "aircraft_bomber": 10000,
    # prompt 055
    "bridgelayer": 8000, "aircraft_transport": 10000,
    # prompt 062
    "rocket_launcher": 8000,
}


def load_faction(faction):
    with open(CATALOG_DIR / f"{faction}.toml", "rb") as f:
        return {u["id"]: u for u in tomllib.load(f).get("unit", [])}


def unit(faction, unit_id):
    units = load_faction(faction)
    if unit_id not in units:
        raise KeyError(f"{unit_id} not in data/catalog/{faction}.toml")
    return units[unit_id]


def blueprint_path(faction, unit_id):
    return BLUEPRINT_DIR / faction / f"{unit_id}.toml"


def blueprint(faction, unit_id):
    path = blueprint_path(faction, unit_id)
    with open(path, "rb") as f:
        bp = tomllib.load(f)
    if bp.get("unit") != unit_id:
        raise ValueError(f"{path}: 'unit' must be {unit_id!r}")
    return bp


def all_blueprints(env=False):
    """(faction, id) of every unit blueprint, sorted.

    Environment blueprints (`assembly = "env"`, prompt 007) have their own
    builder and validator (build_env.py, validate_env.py) and are left out;
    `env=True` returns only those.
    """
    out = []
    for faction in FACTIONS:
        d = BLUEPRINT_DIR / faction
        if d.is_dir():
            for p in sorted(d.glob("*.toml")):
                if is_env_blueprint(p) == env:
                    out.append((faction, p.stem))
    return out


def is_env_blueprint(path):
    with open(path, "rb") as f:
        return tomllib.load(f).get("assembly") == "env"


def blend_path(faction, unit_id):
    return BLEND_DIR / faction / f"{unit_id}.blend"


def glb_path(faction, unit_id):
    return MODEL_DIR / faction / f"{unit_id}.glb"


def mm(u, key):
    """Catalog length in meters (catalog stores millimeters)."""
    if key not in u:
        raise KeyError(f"{u['id']}: catalog field {key} missing")
    return u[key] / 1000.0


def small_arms_count(u):
    """Number of machine guns (caliber up to 13 mm) the model must mount."""
    n = 0
    for w in u.get("weapon", []):
        cal = w.get("caliber_mm_x10", 0)
        if 0 < cal <= 130:
            n += w.get("count", 1)
    return n


def summary(u):
    """Catalog values shown in the Godot model gallery (stored as glTF extras)."""
    keys = (
        "id", "faction", "category", "name_de", "name_en", "designation", "year_from",
        "year_to", "length_mm", "length_hull_mm", "width_mm", "height_mm", "wingspan_mm",
        "mass_kg", "crew", "speed_road_kmh_x10", "speed_offroad_kmh_x10", "range_road_km",
        "power_hp", "armor_hull_front_mm", "armor_hull_side_mm", "armor_turret_front_mm",
    )
    out = {f"cat_{k}": u[k] for k in keys if k in u}
    weapons = [
        (f"{w.get('count', 1)}× " if w.get("count", 1) > 1 else "") + w["name"]
        for w in u.get("weapon", [])
    ]
    out["cat_weapons"] = "; ".join(weapons)
    return out


def env_faction(uid):
    """Faction of an environment object (prompt 008): faction fortifications
    carry the faction prefix (`de_bunker_tobruk`), everything else is neutral."""
    for f in FACTIONS:
        if f != "neutral" and uid.startswith(f + "_"):
            return f
    return "neutral"


def env_height(u, vt):
    """Box height of an environment variant: the catalog height, or the
    variant's `height_m` where the catalog height is 0 (minefields lie flush
    with the ground; their markers stand `height_m` high)."""
    if u.get("height_mm", 0) == 0:
        if "height_m" not in vt:
            raise KeyError(f"{u['id']}: catalog height is 0, the variant needs height_m (marker height)")
        return float(vt["height_m"])
    if "height_m" in vt:
        raise KeyError(f"{u['id']}: height_m is only for catalog entries with height 0")
    return mm(u, "height_mm")
