#!/usr/bin/env python3
"""Vehicle reference data: reader, database writer and repository check.

The reference folder docs/.vorlagen/fahrzeuge/ (not in git, loaded with
`just vehicle-refs`) holds one data.json per vehicle with text values and uneven keys.
This tool reads them into one uniform shape (integers, unit in the field name, like
the catalog) and

  --write      writes data/vehicles/{de,su,us}.json and docs/VEHICLE_REFS.md
               (`just vehicle-db`; needs the reference folder, so it never runs in CI)
  --check      checks the committed database against its schema, data/vehicles/index.json
               and the catalog (`just vehicle-db-check`; needs only the repository)

Python 3 without dependencies. The schema is documented in data/vehicles/SCHEMA.md;
`SCHEMA` below is the machine-readable form and fixes the key order of the output.
"""

import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REFS = ROOT / "docs" / ".vorlagen" / "fahrzeuge"
DB = ROOT / "data" / "vehicles"
INDEX = DB / "index.json"
INVENTORY = ROOT / "docs" / "VEHICLE_REFS.md"
FACTIONS = {"GER": "de", "RU": "su", "US": "us"}
ABSENT = {"", "-", "n.a.", "n.a", "none", "unknown"}

# ----------------------------------------------------------------------------------
# Schema: section -> field -> type. Order here is the order in the files.
# "int"/"str"/"bool" scalars, "strs" list of strings. Every section may also hold
# "text": {field: original text} for values the number alone does not express.
# ----------------------------------------------------------------------------------
SCHEMA = {
    "general": {
        "designation": "str", "other_designations": "str", "manufacturer": "str",
        "production_quantity": "int", "production_period": "str", "crew": "int",
        "length_mm": "int", "length_hull_mm": "int", "width_mm": "int",
        "width_skirts_mm": "int", "height_mm": "int", "barrel_overhang_mm": "int",
        "mass_kg": "int", "radio": "str",
    },
    "firepower": {
        "main_gun": "str", "main_gun_caliber_mm_x10": "int", "main_gun_length_cal_x10": "int",
        "main_ammo": "int", "traverse_deg": "int", "traverse_drive": "str",
        "elevation_min_deg_x10": "int", "elevation_max_deg_x10": "int",
        "traverse_time_360_s": "int", "sight": "str", "secondary": "str",
        "secondary_ammo": "int",
    },
    "mobility": {
        "engine": "str", "engine_type": "str", "displacement_cc": "int", "power_hp": "int",
        "power_rpm": "int", "power_to_weight_hp_per_t_x10": "int", "gearbox": "str",
        "gears_forward": "int", "gears_reverse": "int", "fuel": "str",
        "fuel_capacity_l": "int", "consumption_road_l_per_100km": "int",
        "consumption_offroad_l_per_100km": "int", "range_road_km": "int",
        "range_offroad_km": "int", "speed_road_kmh_x10": "int",
        "speed_offroad_kmh_x10": "int", "speed_water_kmh_x10": "int",
        "track_width_mm": "int", "track_ground_contact_mm": "int", "track_links": "int",
        "ground_pressure_psi_x10": "int", "ground_pressure_kpa_x10": "int",
        "ground_clearance_mm": "int", "turning_radius_mm": "int", "gradient_deg": "int",
        "vertical_obstacle_mm": "int", "fording_mm": "int", "trench_crossing_mm": "int",
    },
}
TOP = ["id", "name", "type", "family", "source", "general", "firepower", "mobility", "armor", "notes"]
SOURCE = ["name", "url", "fetched", "references"]
ARMOR_PARTS = [
    "hull", "hull_upper", "hull_lower", "superstructure", "turret", "main_turret",
    "upper_turret", "lower_turret", "gun_turrets", "mg_turrets", "mantlet", "gun_shield",
]
ARMOR_FACES = ["front", "side", "rear", "top_bottom"]
ARMOR_CELL = ["mm", "mm_min", "mm_max", "deg", "text"]

# Plausible limits; a number outside them is kept as text only and reported.
LIMITS = {
    "crew": (1, 12), "length_mm": (2000, 12000), "length_hull_mm": (2000, 11000),
    "width_mm": (1200, 4000), "width_skirts_mm": (1200, 4000), "height_mm": (1000, 4000),
    "barrel_overhang_mm": (0, 4000), "mass_kg": (1500, 200000),
    "power_hp": (20, 1500), "track_width_mm": (100, 1200),
    "track_ground_contact_mm": (1000, 7000), "ground_clearance_mm": (100, 700),
    "turning_radius_mm": (0, 30000), "fording_mm": (300, 8000),
    "vertical_obstacle_mm": (200, 1500), "trench_crossing_mm": (500, 5000),
    "speed_road_kmh_x10": (100, 900), "speed_offroad_kmh_x10": (50, 700),
    "speed_water_kmh_x10": (20, 150),
}

NUM = r"\d+(?:\.\d+)?"
# A trailing imperial equivalent ("(22' 4\")", "(7300 lbs)") does not make a value unclear.
IMPERIAL = r"(?:\s*\([^()]*(?:ft|in\b|'|\"|lbs|psi)[^()]*\))?"


class Reader:
    """Collects one entry and what could not be converted cleanly."""

    def __init__(self, folder):
        self.folder = folder
        self.warnings = []
        self.text_only = []
        self.sections = {name: {} for name in SCHEMA}
        self.texts = {name: {} for name in SCHEMA}

    def put(self, section, field, value, text=None, clean=True):
        """Stores a number and, unless `clean`, the original text next to it."""
        low, high = LIMITS.get(field, (None, None))
        if value is not None and low is not None and not low <= value <= high:
            self.warnings.append(f"{section}.{field}: {value} outside {low}..{high} ({text!r})")
            value, clean = None, False
        if value is not None:
            self.sections[section][field] = value
        if not clean and text is not None:
            self.texts[section][field] = text
            if value is None:
                self.text_only.append(f"{section}.{field}: {text!r}")

    def string(self, section, field, raw):
        if not absent(raw):
            self.sections[section][field] = tidy(raw)

    def number(self, section, field, raw, unit=r"", scale=1.0):
        """`<number> <unit>`; anything more (ranges, 'or', '*', '[1]') keeps its text."""
        if absent(raw):
            return
        raw = tidy(raw)
        clean = re.fullmatch(rf"({NUM})\s*(?:{unit})?{IMPERIAL}", raw, re.I)
        if clean:
            self.put(section, field, rounded(float(clean.group(1)) * scale), raw)
            return
        loose = re.match(rf"(?:about |over |approx\. )?({NUM})(?![\d.\-/&]|\s*(?:or|to)\b)", raw, re.I)
        value = rounded(float(loose.group(1)) * scale) if loose else None
        self.put(section, field, value, raw, clean=False)

    def pair(self, section, fields, raw, unit, scale=1.0):
        """`a/b <unit>` (on/off road); a single value is the first field."""
        if absent(raw):
            return
        raw = tidy(raw)
        body = re.sub(rf"\s*(?:{unit})\s*$", "", raw, flags=re.I)
        body = re.sub(r"\s*on road$", "", body, flags=re.I)
        both = re.fullmatch(rf"({NUM}|n\.a\.)\s*(?:on)?\s*/\s*({NUM}|n\.a\.)(?:\s*off road)?", body, re.I)
        if both:
            for field, part in zip(fields, both.groups()):
                if part != "n.a.":
                    self.put(section, field, rounded(float(part) * scale), raw)
            return
        self.number(section, fields[0], body, scale=scale)
        if fields[0] in self.texts[section]:
            self.texts[section][fields[0]] = raw

    def entry(self):
        out = {}
        for name in SCHEMA:
            section = {f: self.sections[name][f] for f in SCHEMA[name] if f in self.sections[name]}
            if self.texts[name]:
                section["text"] = {f: self.texts[name][f] for f in SCHEMA[name] if f in self.texts[name]}
            out[name] = section
        return out


def absent(raw):
    return raw is None or str(raw).strip().lower() in ABSENT


def tidy(raw):
    return re.sub(r"\s+", " ", str(raw)).strip()


def rounded(value):
    return int(round(value))


def pick(section, *prefixes):
    """First key of `section` that starts with one of `prefixes` (keys vary in suffix)."""
    for prefix in prefixes:
        for key, value in section.items():
            if key == prefix or key.startswith(prefix):
                return value
    return None


def read_general(r, g):
    r.string("general", "designation", pick(g, "Formal Designation"))
    r.string("general", "other_designations", pick(g, "Other Designation"))
    r.string("general", "manufacturer", pick(g, "Manufacturer"))
    r.number("general", "production_quantity", g.get("Production Quantity"))
    r.string("general", "production_period", g.get("Production Period"))
    r.number("general", "crew", g.get("Crew"))
    # Length: one value is the overall length; two are overall and hull in either order.
    for key, raw in g.items():
        if not key.lower().startswith(("length", "overall length")) or absent(raw):
            continue
        raw = tidy(raw)
        two = re.match(rf"({NUM})\s*/\s*({NUM})", raw)
        if two:
            a, b = (rounded(float(v) * 1000) for v in two.groups())
            ordered = "hull/overall" not in key.lower()
            clean = (a >= b) == ordered
            r.put("general", "length_mm", max(a, b), raw, clean)
            r.put("general", "length_hull_mm", min(a, b), raw, clean)
        else:
            r.number("general", "length_mm", raw, unit="m", scale=1000)
    width = pick(g, "Width")
    if not absent(width):
        width = tidy(width)
        two = re.fullmatch(rf"({NUM})\s*/\s*({NUM})", width)
        if two:
            r.put("general", "width_mm", rounded(float(two.group(1)) * 1000), width)
            r.put("general", "width_skirts_mm", rounded(float(two.group(2)) * 1000), width)
        else:
            r.number("general", "width_mm", width, unit="m", scale=1000)
    r.number("general", "height_mm", pick(g, "Height"), unit="m", scale=1000)
    r.number("general", "barrel_overhang_mm", pick(g, "Barrel Overhang"), unit="m", scale=1000)
    r.number("general", "mass_kg", pick(g, "Combat Weight"), unit="kg")
    r.string("general", "radio", pick(g, "Radio"))


def read_firepower(r, f):
    gun = pick(f, "Primary Armament")
    if not absent(gun):
        gun = tidy(gun)
        r.string("firepower", "main_gun", gun)
        # A turret label ("Main Turret: ...") or a choice of models leaves the caliber
        # clear; a choice of calibers or barrel lengths does not.
        body = re.sub(r"^[\w ]+:\s*", "", gun)
        caliber = re.match(rf"(?:\d+ x )?({NUM})\s*(mm|cm)", body)
        if caliber and not re.search(rf"\bor {NUM}\s*(?:mm|cm)", body):
            factor = 100 if caliber.group(2) == "cm" else 10
            r.put("firepower", "main_gun_caliber_mm_x10", rounded(float(caliber.group(1)) * factor))
        elif caliber:
            r.put("firepower", "main_gun_caliber_mm_x10", None, gun, clean=False)
        lengths = re.findall(rf"L/({NUM})", body)
        if len(lengths) == 1:
            r.put("firepower", "main_gun_length_cal_x10", rounded(float(lengths[0]) * 10))
        elif lengths:
            r.put("firepower", "main_gun_length_cal_x10", None, gun, clean=False)
    r.number("firepower", "main_ammo", pick(f, "Ammunition Carried (Primary"))
    traverse = f.get("Traverse (degrees)")
    if not absent(traverse):
        traverse = tidy(traverse)
        drive = re.match(r"(Manual|Power|Hydraulic|Electric)\b", traverse, re.I)
        full = re.fullmatch(r"(?:[\w ]+: ?)?(?:\w+ \()?360°?\)?", traverse)
        sides = re.fullmatch(rf"(?:\w+ \()?({NUM})° ?L\W*(?:to |or )?({NUM})° ?R\)?", traverse)
        if drive:
            r.sections["firepower"]["traverse_drive"] = drive.group(1).lower()
        if full:
            r.put("firepower", "traverse_deg", 360)
        elif sides:
            r.put("firepower", "traverse_deg", rounded(float(sides.group(1)) + float(sides.group(2))), traverse)
        else:
            r.put("firepower", "traverse_deg", None, traverse, clean=False)
    elevation = f.get("Elevation (degrees)")
    if not absent(elevation):
        elevation = tidy(elevation)
        span = re.fullmatch(rf"([-+]?{NUM})°?\s*to\s*([-+]?{NUM})°?", elevation)
        if span:
            r.put("firepower", "elevation_min_deg_x10", rounded(float(span.group(1)) * 10))
            r.put("firepower", "elevation_max_deg_x10", rounded(float(span.group(2)) * 10))
        else:
            r.put("firepower", "elevation_min_deg_x10", None, elevation, clean=False)
    r.number("firepower", "traverse_time_360_s", f.get("Traverse speed (360°)"), unit=r"sec\.?")
    r.string("firepower", "sight", pick(f, "Sight"))
    r.string("firepower", "secondary", f.get("Secondary Armament"))
    r.number("firepower", "secondary_ammo", f.get("Ammunition Carried (Secondary Armament)"))


def read_mobility(r, m):
    s = "mobility"
    r.string(s, "engine", m.get("Engine Make & Model"))
    kind = m.get("Type & Displacement")
    r.string(s, "engine_type", kind)
    if not absent(kind):
        litres = re.fullmatch(rf".*?({NUM})\s*(?:liters|litres|l)", tidy(kind), re.I)
        if litres and "x" not in tidy(kind).split(",")[0].lower():
            r.put(s, "displacement_cc", rounded(float(litres.group(1)) * 1000))
    power = pick(m, "Horsepower")
    if not absent(power):
        power = tidy(power)
        one = re.fullmatch(rf"({NUM})\s*hp(?:\s*@\s*(\d+)\s*rpm)?", power, re.I)
        twin = re.fullmatch(rf"2 x ({NUM})\s*hp(?:\s*@\s*(\d+)\s*rpm)?", power, re.I)
        if one or twin:
            match = one or twin
            r.put(s, "power_hp", rounded(float(match.group(1)) * (2 if twin else 1)), power, clean=bool(one))
            if match.group(2):
                r.put(s, "power_rpm", int(match.group(2)))
        else:
            first = re.match(rf"({NUM})\s*hp", power, re.I)
            r.put(s, "power_hp", rounded(float(first.group(1))) if first else None, power, clean=False)
    r.number(s, "power_to_weight_hp_per_t_x10", pick(m, "Power/Weight", "Power /Weight"), unit=r"hp/t(?:onne)?", scale=10)
    gearbox = m.get("Gearbox")
    r.string(s, "gearbox", gearbox)
    if not absent(gearbox):
        gears = re.fullmatch(r"(\d+) forward, (\d+) reverse", tidy(gearbox))
        if gears:
            r.put(s, "gears_forward", int(gears.group(1)))
            r.put(s, "gears_reverse", int(gears.group(2)))
    r.string(s, "fuel", m.get("Fuel"))
    r.number(s, "fuel_capacity_l", pick(m, "Fuel Capacity"), unit=r"l|liters|litres")
    r.pair(s, ("consumption_road_l_per_100km", "consumption_offroad_l_per_100km"), pick(m, "Mileage"), unit=r"l/100km")
    r.pair(s, ("range_road_km", "range_offroad_km"), pick(m, "Range"), unit="km")
    speed = pick(m, "Speed")
    water = any(k.startswith("Speed on road/water") for k in m)
    fields = ("speed_road_kmh_x10", "speed_water_kmh_x10" if water else "speed_offroad_kmh_x10")
    if not absent(speed):
        r.pair(s, fields, re.sub(r"\s*\(max\.?\)", "", tidy(speed)), unit=r"km/h", scale=10)
    for field, key in (("track_width_mm", "Track Width"), ("track_ground_contact_mm", "Track Ground Contact")):
        raw = m.get(key)
        if absent(raw):
            continue
        raw = tidy(raw)
        cm = re.fullmatch(rf"({NUM})\s*cm{IMPERIAL}", raw)
        if cm:
            r.put(s, field, rounded(float(cm.group(1)) * 10), raw)
        else:
            # No unit or several values: never guess the unit.
            r.put(s, field, None, raw, clean=False)
    links = pick(m, "Track Links", "No. of Links")
    r.number(s, "track_links", links, unit=r"/track")
    pressure = m.get("Ground Pressure")
    if not absent(pressure):
        pressure = tidy(pressure)
        psi = re.fullmatch(rf"(?:{NUM} kg/cm² \()?({NUM})\s*psi\)?", pressure)
        if psi:
            r.put(s, "ground_pressure_psi_x10", rounded(float(psi.group(1)) * 10))
            r.put(s, "ground_pressure_kpa_x10", rounded(float(psi.group(1)) * 68.94757))
        else:
            r.put(s, "ground_pressure_psi_x10", None, pressure, clean=False)
    r.number(s, "ground_clearance_mm", pick(m, "Ground Clearance"), unit="m", scale=1000)
    r.number(s, "turning_radius_mm", pick(m, "Turning Radius"), unit="m", scale=1000)
    r.number(s, "gradient_deg", pick(m, "Gradient"), unit="°")
    r.number(s, "vertical_obstacle_mm", pick(m, "Vertical Obstacle"), unit="m", scale=1000)
    r.number(s, "fording_mm", pick(m, "Fording"), unit="m", scale=1000)
    r.number(s, "trench_crossing_mm", pick(m, "Trench Crossing"), unit="m", scale=1000)


def armor_part(name):
    key = re.sub(r"\s+", "_", name.strip().lower())
    return {"mantlets": "mantlet"}.get(key, key)


def armor_cell(raw):
    """`100mm @80°` -> mm and deg; layered plates add up; ranges give mm_min/mm_max."""
    if absent(raw):
        return None
    raw = tidy(raw)
    cell = {}
    exact = re.fullmatch(r"(\d+)\s*mm(?:\s*@\s*(\d+)°)?", raw)
    if exact:
        cell["mm"] = int(exact.group(1))
        if exact.group(2):
            cell["deg"] = int(exact.group(2))
        return cell
    plates = re.match(r"(\d+)\+(\d+)\s*mm", raw)
    span = re.match(r"(\d+)-(\d+)\s*mm", raw)
    single = re.match(r"(\d+)\s*mm", raw)
    if plates:
        cell["mm"] = int(plates.group(1)) + int(plates.group(2))
    elif span:
        cell["mm_min"], cell["mm_max"] = int(span.group(1)), int(span.group(2))
    elif single:
        cell["mm"] = int(single.group(1))
    angle = re.search(r"@\s*(\d+)°$", raw)
    if angle:
        cell["deg"] = int(angle.group(1))
    cell["text"] = raw
    return cell


def read_armor(r, a):
    armor = {}
    for name, faces in a.items():
        part = armor_part(name)
        if part not in ARMOR_PARTS:
            r.warnings.append(f"armor: unknown part {name!r}")
            continue
        if not isinstance(faces, dict):
            if not absent(faces):
                armor[part] = {"text": tidy(faces)}
                r.text_only.append(f"armor.{part}: {tidy(faces)!r}")
            continue
        cells = {}
        for face, raw in faces.items():
            key = {"Front": "front", "Side": "side", "Rear": "rear", "Top/Bottom": "top_bottom"}.get(face)
            if key is None:
                r.warnings.append(f"armor.{part}: unknown face {face!r}")
                continue
            cell = armor_cell(raw)
            if cell is not None:
                cells[key] = {k: cell[k] for k in ARMOR_CELL if k in cell}
                if "mm" not in cell and "mm_min" not in cell:
                    r.text_only.append(f"armor.{part}.{key}: {cell['text']!r}")
        if cells:
            armor[part] = {k: cells[k] for k in ARMOR_FACES if k in cells}
    return {p: armor[p] for p in ARMOR_PARTS if p in armor}


def read_notes(raw):
    if raw is None:
        return []
    values = raw.values() if isinstance(raw, dict) else [raw] if isinstance(raw, str) else raw
    return [tidy(v) for v in values if not absent(v)]


def read_folder(folder, vehicle_id):
    """One reference folder -> (entry, warnings, text-only values)."""
    data = json.loads((REFS / folder / "data.json").read_text(encoding="utf-8"))
    r = Reader(folder)
    read_general(r, data.get("general_data", {}))
    read_firepower(r, data.get("firepower", {}))
    read_mobility(r, data.get("mobility_characteristics", {}))
    armor = read_armor(r, data.get("armor_protection", {}))
    entry = {
        "id": vehicle_id,
        "name": tidy(data.get("title") or data["name"]),
        "type": tidy(data.get("general_data", {}).get("Type", "")),
        "family": tidy(data.get("family", "")),
        "source": {
            "name": tidy(data.get("source", "")),
            "url": data.get("source_url", ""),
            "fetched": data.get("fetched", ""),
            "references": [tidy(x) for x in data.get("references", [])],
        },
        **r.entry(),
        "armor": armor,
    }
    notes = read_notes(data.get("notes"))
    if notes:
        entry["notes"] = notes
    drawings = len(data.get("images", []))
    return entry, r.warnings, r.text_only, drawings


# ----------------------------------------------------------------------------------
# Output
# ----------------------------------------------------------------------------------
def dump(entries):
    """Stable text: two spaces, schema key order, one armor cell per line."""
    text = json.dumps(entries, ensure_ascii=False, indent=2)
    # Collapse the innermost objects (armor cells) onto one line each.
    text = re.sub(
        r"\{\n\s+(\"(?:mm|mm_min|mm_max|deg|text)\": [^{}\n]+(?:,\n\s+\"(?:mm|mm_min|mm_max|deg|text)\": [^{}\n]+)*)\n\s+\}",
        lambda m: "{ " + re.sub(r",\n\s+", ", ", m.group(1)) + " }",
        text,
    )
    return text + "\n"


def load_index():
    return json.loads(INDEX.read_text(encoding="utf-8"))


def write():
    if not REFS.is_dir():
        sys.exit(f"vehicle-db: {REFS.relative_to(ROOT)} is missing; load it with `just vehicle-refs`")
    index = load_index()
    folders = sorted(p.parent.relative_to(REFS).as_posix() for p in REFS.glob("*/*/*/data.json"))
    unlisted = [f for f in folders if f not in index]
    gone = [f for f in index if f not in folders]
    if unlisted or gone:
        for f in unlisted:
            print(f"vehicle-db: not in index.json: {f}")
        for f in gone:
            print(f"vehicle-db: in index.json but no folder: {f}")
        sys.exit(1)
    catalog = catalog_entries()
    by_faction = {code: [] for code in FACTIONS.values()}
    warnings, text_only, rows = [], [], []
    for folder in folders:
        row = index[folder]
        entry, w, t, drawings = read_folder(folder, row["id"])
        rows.append((folder, row, entry, drawings))
        if row["status"] == "skipped":
            continue
        by_faction[FACTIONS[folder.split("/")[0]]].append(entry)
        warnings += [f"{row['id']}: {x}" for x in w]
        text_only += [f"{row['id']}: {x}" for x in t]
    for code, entries in by_faction.items():
        entries.sort(key=lambda e: e["id"])
        (DB / f"{code}.json").write_text(dump(entries), encoding="utf-8")
    INVENTORY.write_text(inventory(rows, catalog), encoding="utf-8")
    print("vehicle-db: " + ", ".join(f"{code} {len(e)}" for code, e in by_faction.items())
          + f" entries; {len(warnings)} warnings; {len(text_only)} values as text only")
    for line in warnings:
        print(f"  warning: {line}")
    for line in text_only:
        print(f"  text only: {line}")


def inventory(rows, catalog):
    lines = [
        "# Vehicle references",
        "",
        "Generated by `just vehicle-db` from `data/vehicles/index.json` and the reference",
        "folder `docs/.vorlagen/fahrzeuge/` (not in the repository). Do not edit by hand:",
        "change `index.json` and run the command again.",
        "",
        "Status: `existing` = catalog entry exists, `new` = proposed catalog entry (prompt 063;",
        "already in the vehicle database), `skipped` = not taken over (reason given).",
        "",
        "| Reference folder | Id | Status | Type | Built | Period | Drawings | Remark |",
        "|---|---|---|---|---|---|---|---|",
    ]
    counts = {}
    for folder, row, entry, drawings in rows:
        counts[row["status"]] = counts.get(row["status"], 0) + 1
        general = entry["general"]
        built = general.get("text", {}).get("production_quantity", general.get("production_quantity", ""))
        remark = row.get("reason", "")
        if row["status"] == "existing" and row["id"] not in catalog:
            remark = "MISSING IN CATALOG"
        if drawings == 0:
            remark = (remark + "; " if remark else "") + "no drawing"
        lines.append(f"| `{folder}` | `{row['id']}` | {row['status']} | {entry['type']} | {built} | "
                     f"{general.get('production_period', '')} | {drawings} | {remark} |")
    lines += ["", "Totals: " + ", ".join(f"{n} {s}" for s, n in sorted(counts.items())) + ".", ""]
    return "\n".join(lines)


# ----------------------------------------------------------------------------------
# Check (repository only)
# ----------------------------------------------------------------------------------
def catalog_entries():
    """The catalog values the check compares; a small reader for the fixed TOML layout."""
    entries = {}
    for code in FACTIONS.values():
        text = (ROOT / "data" / "catalog" / f"{code}.toml").read_text(encoding="utf-8")
        for block in text.split("[[unit]]")[1:]:
            entry = {}
            for key, value in re.findall(r'^(\w+) = (-?\d+|"(?:[^"\\]|\\.)*")\s*$', block, re.M):
                entry[key] = int(value) if value[0] != '"' else json.loads(value)
            uncertain = re.search(r"^uncertain = \[(.*?)\]", block, re.M | re.S)
            entry["uncertain"] = re.findall(r'"([^"]+)"', uncertain.group(1)) if uncertain else []
            weapons = re.search(r"^weapon = \[\n(.*?)^\]", block, re.M | re.S)
            entry["weapon"] = []
            for line in (weapons.group(1).splitlines() if weapons else []):
                fields = dict(re.findall(r'(\w+) = (-?\d+|"(?:[^"\\]|\\.)*")', line))
                entry["weapon"].append({k: int(v) if v[0] != '"' else json.loads(v) for k, v in fields.items()})
            if "id" in entry:
                entries[entry["id"]] = entry
    return entries


def check_shape(entry, where, errors):
    def fail(message):
        errors.append(f"{where}: {message}")

    for key in entry:
        if key not in TOP:
            fail(f"unknown field {key!r}")
    for key in ("id", "name", "type", "family", "source", "general", "firepower", "mobility", "armor"):
        if key not in entry:
            fail(f"missing {key!r}")
            return
    for key in ("id", "name", "type", "family"):
        if not isinstance(entry[key], str):
            fail(f"{key} must be a string")
    source = entry["source"]
    if list(source) != SOURCE:
        fail(f"source must have exactly {SOURCE}")
    elif not (source["url"].startswith("https://") and re.fullmatch(r"\d{4}-\d\d-\d\d", source["fetched"])):
        fail("source.url must be https and source.fetched a date")
    for name, fields in SCHEMA.items():
        section = entry[name]
        for key, value in section.items():
            if key == "text":
                for field, text in value.items():
                    if field not in fields or not isinstance(text, str):
                        fail(f"{name}.text.{field}: unknown field or not a string")
                continue
            kind = fields.get(key)
            if kind is None:
                fail(f"unknown field {name}.{key}")
            elif kind == "int" and (not isinstance(value, int) or isinstance(value, bool)):
                fail(f"{name}.{key} must be an integer")
            elif kind == "str" and not isinstance(value, str):
                fail(f"{name}.{key} must be a string")
        order = [k for k in list(fields) + ["text"] if k in section]
        if list(section) != order:
            fail(f"{name}: keys out of schema order")
    for part, faces in entry["armor"].items():
        if part not in ARMOR_PARTS:
            fail(f"unknown armor part {part!r}")
            continue
        if list(faces) == ["text"]:
            continue
        for face, cell in faces.items():
            if face not in ARMOR_FACES:
                fail(f"armor.{part}: unknown face {face!r}")
            elif not cell or any(k not in ARMOR_CELL for k in cell):
                fail(f"armor.{part}.{face}: unknown or empty cell")
            elif any(not isinstance(v, int) for k, v in cell.items() if k != "text"):
                fail(f"armor.{part}.{face}: numbers must be integers")
    if "notes" in entry and not (isinstance(entry["notes"], list) and all(isinstance(n, str) for n in entry["notes"])):
        fail("notes must be a list of strings")
    if [k for k in TOP if k in entry] != list(entry):
        fail("top-level keys out of schema order")


def thickness(entry, parts, face):
    """Thickest plate of `face` over `parts` (the catalog keeps the thickest plate)."""
    best = None
    for part in parts:
        cell = entry["armor"].get(part, {}).get(face)
        if isinstance(cell, dict):
            value = cell.get("mm", cell.get("mm_max"))
            if value is not None and (best is None or value > best):
                best = value
    return best


# Catalog field -> (database value, relative tolerance in percent). A casemate has no
# turret: its superstructure is what the catalog calls the turret.
def comparisons(entry):
    g, f, m = entry["general"], entry["firepower"], entry["mobility"]
    hull = ("hull", "hull_upper", "hull_lower")
    top = ("turret", "main_turret") if any(p in entry["armor"] for p in ("turret", "main_turret")) else ("superstructure",)
    hull_length = g.get("length_hull_mm")
    if hull_length is None and "length_mm" in g and "barrel_overhang_mm" in g:
        hull_length = g["length_mm"] - g["barrel_overhang_mm"]
    # The catalog's "upper front plate" is the source's superstructure front where it
    # lists one (its "Hull" is then the lower hull).
    armor = entry["armor"]
    front = next((armor[p]["front"] for p in ("superstructure", "hull_upper", "hull")
                  if isinstance(armor.get(p, {}).get("front"), dict)), {})
    return {
        "length_mm": (g.get("length_mm"), 5),
        "length_hull_mm": (hull_length, 5),
        "width_mm": (g.get("width_skirts_mm", g.get("width_mm")), 5),
        "height_mm": (g.get("height_mm"), 5),
        "mass_kg": (g.get("mass_kg"), 5),
        "crew": (g.get("crew"), 0),
        "speed_road_kmh_x10": (m.get("speed_road_kmh_x10"), 10),
        "speed_offroad_kmh_x10": (m.get("speed_offroad_kmh_x10"), 25),
        "range_road_km": (m.get("range_road_km"), 15),
        "power_hp": (m.get("power_hp"), 5),
        "armor_hull_front_mm": (thickness(entry, hull + ("superstructure",), "front"), 10),
        "armor_hull_side_mm": (thickness(entry, hull + ("superstructure",), "side"), 10),
        "armor_hull_rear_mm": (thickness(entry, hull + ("superstructure",), "rear"), 10),
        "armor_turret_front_mm": (thickness(entry, top + ("mantlet", "gun_shield"), "front"), 10),
        "armor_turret_side_mm": (thickness(entry, top, "side"), 10),
        "armor_turret_rear_mm": (thickness(entry, top, "rear"), 10),
        # The source measures from the horizontal, the catalog from the vertical.
        "armor_hull_front_deg": (90 - front["deg"] if "deg" in front else None, None),
        "weapon.caliber_mm_x10": (f.get("main_gun_caliber_mm_x10"), 1),
        "weapon.ammo_carried": (f.get("main_ammo"), 10),
    }


def check_catalog(entry, catalog, errors, report):
    unit = catalog[entry["id"]]
    for field, (value, tolerance) in comparisons(entry).items():
        if field.startswith("weapon."):
            ours = (unit["weapon"] or [{}])[0].get(field.split(".")[1])
            flag = "weapon"
        else:
            ours, flag = unit.get(field), field
        if value is None or ours is None:
            continue
        if tolerance is None:  # degrees: absolute tolerance
            off = abs(value - ours) > 10
            shown = f"{abs(value - ours)}°"
        else:
            off = abs(value - ours) * 100 > tolerance * max(abs(ours), 1)
            shown = f"{(value - ours) * 100 / max(abs(ours), 1):+.0f} %"
        if not off:
            continue
        line = f"{entry['id']}: {field} catalog {ours}, database {value} ({shown})"
        if flag in unit["uncertain"]:
            report.append(line)
        else:
            errors.append(line + f"; fix the catalog or list {flag!r} in `uncertain` with the reason in `note`")


def check(verbose):
    errors, report = [], []
    index = load_index()
    catalog = catalog_entries()
    seen = {}
    for folder, row in index.items():
        if set(row) - {"id", "status", "reason"} or row.get("status") not in ("existing", "new", "skipped"):
            errors.append(f"index.json: {folder}: needs id and status existing/new/skipped")
            continue
        if row["status"] == "skipped" and not row.get("reason"):
            errors.append(f"index.json: {folder}: skipped needs a reason")
        if row["id"] in seen:
            errors.append(f"index.json: id {row['id']} used by {seen[row['id']]} and {folder}")
        seen[row["id"]] = folder
        if not row["id"].startswith(FACTIONS[folder.split("/")[0]] + "_"):
            errors.append(f"index.json: {folder}: id {row['id']} has the wrong faction prefix")
        if (row["status"] == "existing") != (row["id"] in catalog):
            errors.append(f"index.json: {folder}: status {row['status']} but id {row['id']} "
                          f"is {'in' if row['id'] in catalog else 'not in'} the catalog")
    wanted = {row["id"] for row in index.values() if row.get("status") in ("existing", "new")}
    found = set()
    for code in FACTIONS.values():
        path = DB / f"{code}.json"
        text = path.read_text(encoding="utf-8")
        entries = json.loads(text)
        if text != dump(entries):
            errors.append(f"{path.name}: not in the stable format of `just vehicle-db`")
        ids = [e.get("id", "") for e in entries]
        if ids != sorted(ids):
            errors.append(f"{path.name}: entries are not sorted by id")
        for entry in entries:
            where = f"{path.name}: {entry.get('id', '?')}"
            before = len(errors)
            check_shape(entry, where, errors)
            if len(errors) > before:
                continue
            if entry["id"] in found:
                errors.append(f"{where}: duplicate id")
            found.add(entry["id"])
            if not entry["id"].startswith(code + "_"):
                errors.append(f"{where}: wrong file for this id")
            if entry["id"] not in wanted:
                errors.append(f"{where}: not listed as existing/new in index.json")
            elif entry["id"] in catalog:
                check_catalog(entry, catalog, errors, report)
    for missing in sorted(wanted - found):
        errors.append(f"index.json lists {missing} but the database has no entry")
    in_catalog = sum(1 for i in found if i in catalog)
    print(f"vehicle-db-check: {len(found)} entries ({in_catalog} in the catalog), "
          f"{len(report)} accepted deviations, {len(errors)} errors")
    if verbose:
        for line in report:
            print(f"  accepted (uncertain): {line}")
    for line in errors:
        print(f"  error: {line}")
    return not errors


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--write", action="store_true", help="write the database from the reference folder")
    parser.add_argument("--check", action="store_true", help="check the committed database")
    parser.add_argument("--verbose", action="store_true", help="with --check: list accepted deviations")
    args = parser.parse_args()
    if not (args.write or args.check):
        parser.error("give --write or --check")
    if args.write:
        write()
    if args.check and not check(args.verbose):
        sys.exit(1)


if __name__ == "__main__":
    main()
