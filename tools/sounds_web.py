#!/usr/bin/env python3
"""Serve the sound page (web/sounds/) from the repository root, or check its remarks.

The page lists every audio file under godot/assets/audio/ with a remark on when the
game plays it (web/sounds/usage.toml), its origin and the manifest values of its class.
On start this tool writes web/sounds/index.json (gitignored), the only file the page
loads besides the audio files themselves, then serves the repository root on
http://localhost:8138/web/sounds/ (`--port N`, `--no-open`). Usage: `just sounds-web`.

`--check` (in `just check` and CI): exit code 1 if a folder with audio files has no
remark, a remark has no folder, or a manifest class has no folder. It looks at folder
and file names only, so it runs without Git LFS and without the private sound
repository; `priv_*` files do not count.

Local only: no audio file is copied under web/ (private `priv_*` sounds must not be
passed on as single files, docs/AUDIO_LIBRARY.md).
"""

import argparse
import functools
import http.server
import json
import pathlib
import re
import struct
import sys
import wave
import webbrowser

ROOT = pathlib.Path(__file__).resolve().parent.parent
AUDIO = ROOT / "godot" / "assets" / "audio"
WEB = ROOT / "web" / "sounds"
USAGE = WEB / "usage.toml"
INDEX = WEB / "index.json"
EXTENSIONS = (".wav", ".ogg", ".mp3")
PRIVATE = "priv_"
# Folders under godot/assets/audio/ that are not manifest classes.
KEYED = ("weapon", "ambience", "music", "voice", "announce")
# Loop layers of ambience.gd (LOOPS); the others are one-shots.
AMBIENCE_LOOPS = {"wind_forest", "wind_desert", "wind_winter", "insects_meadow", "insects_desert",
                  "leaves", "water", "rain"}

# Page sections in order (key, German title); classes go by name, see section_of().
SECTIONS = [
    ("shots", "Schüsse"),
    ("weapons", "Waffenaufnahmen"),
    ("crack", "Überschall und Nachhall"),
    ("impacts", "Einschläge und Treffer"),
    ("explosions", "Explosionen und Feuer"),
    ("vehicles", "Fahrzeuge und Flugzeuge"),
    ("objects", "Gebäude und Objekte"),
    ("ui", "Meldungen und Oberfläche"),
    ("voices", "Stimmen"),
    ("ambience", "Ambiente"),
    ("music", "Musik"),
]
# Class name prefix or name -> section (first match wins).
CLASS_SECTIONS = [
    ("shot_", "shots"),
    ("crack_", "crack"),
    ("explosion_tail", "crack"),
    ("hit_", "impacts"),
    ("shell_incoming", "impacts"),
    ("explosion_", "explosions"),
    ("flak_", "explosions"),
    ("fire", "explosions"),
    ("engine_", "vehicles"),
    ("tracks", "vehicles"),
    ("building_", "objects"),
    ("tree_", "objects"),
    ("wall_", "objects"),
    ("fence_", "objects"),
    ("crater", "objects"),
    ("ui_", "ui"),
]


def section_of(name):
    for prefix, section in CLASS_SECTIONS:
        if name.startswith(prefix):
            return section
    return None


def read_usage(path=USAGE):
    """`{table: {key: text}}` from usage.toml: [table] headers, `key = "text"` lines.

    Python 3.10 has no tomllib, so this reads only the subset the file uses and
    rejects anything else.
    """
    tables, current = {}, None
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        header = re.fullmatch(r"\[([a-z_]+)\]", text)
        if header:
            current = tables.setdefault(header.group(1), {})
            continue
        entry = re.fullmatch(r'([A-Za-z0-9_]+)\s*=\s*("(?:[^"\\]|\\.)*")', text)
        if entry is None or current is None:
            raise SystemExit(f"{path.relative_to(ROOT)}:{number}: expected [table] or key = \"text\"")
        current[entry.group(1)] = json.loads(entry.group(2))
    return tables


def audio_files(folder):
    if not folder.is_dir():
        return []
    return sorted(p for p in folder.iterdir() if p.is_file() and p.suffix.lower() in EXTENSIONS)


def public(files):
    return [p for p in files if not p.name.startswith(PRIVATE)]


def folders():
    """`{table: {key: [files]}}` of every audio folder, the same tables as usage.toml."""
    manifest = json.loads((AUDIO / "sounds.json").read_text(encoding="utf-8"))
    out = {"class": {}, "weapon": {}, "ambience": {}, "music": {}, "voice": {}, "announce": {}}
    for path in sorted(AUDIO.iterdir()):
        if path.is_dir() and path.name not in KEYED:
            out["class"][path.name] = audio_files(path)
    for table in ("weapon", "ambience"):
        for path in sorted((AUDIO / table).glob("*/")):
            out[table][path.name] = audio_files(path)
    for path in audio_files(AUDIO / "music"):
        out["music"].setdefault(path.stem.removeprefix(PRIVATE).rsplit("_", 1)[0], []).append(path)
    # Voices: one remark per callout for all factions (voice/<faction>/<callout>/).
    for path in sorted((AUDIO / "voice").glob("*/*/")):
        out["voice"].setdefault(path.name, []).extend(audio_files(path))
    # Announcements (prompt 244): one remark per message for all factions
    # (announce/<faction>/<message>/).
    for path in sorted((AUDIO / "announce").glob("*/*/")):
        out["announce"].setdefault(path.name, []).extend(audio_files(path))
    return manifest, out


def check():
    manifest, found = folders()
    usage = read_usage()
    problems = []
    for name in manifest["classes"]:
        if not (AUDIO / name).is_dir():
            problems.append(f"manifest class '{name}' has no folder godot/assets/audio/{name}/")
        if section_of(name) is None:
            problems.append(f"manifest class '{name}' has no page section (CLASS_SECTIONS in tools/sounds_web.py)")
    for table, entries in found.items():
        remarks = usage.get(table, {})
        for key, files in entries.items():
            if public(files) and key not in remarks:
                problems.append(f"no remark for {table} '{key}' in web/sounds/usage.toml")
        for key in remarks:
            # Callouts may be described before their recordings ship (battle_voices.gd);
            # announcements are private recordings, absent in a clone without them.
            if table not in ("voice", "announce") and key not in entries:
                problems.append(f"remark for {table} '{key}' in web/sounds/usage.toml has no folder")
    for table in usage:
        if table not in found:
            problems.append(f"unknown table [{table}] in web/sounds/usage.toml")
    for problem in problems:
        print(f"sounds-web: {problem}", file=sys.stderr)
    if not problems:
        count = sum(len(public(f)) for entries in found.values() for f in entries.values())
        print(f"sounds-web: remarks complete ({count} public audio files)")
    return not problems


def wav_seconds(path):
    """Length of a WAV file, or None (e.g. a Git LFS pointer)."""
    try:
        with wave.open(str(path)) as w:
            return round(w.getnframes() / w.getframerate(), 3)
    except (wave.Error, EOFError, OSError):
        pass
    # Formats `wave` rejects (float, extensible): fmt byte rate and data size.
    try:
        data = path.read_bytes()
        if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
            return None
        at, rate, size = 12, None, None
        while at + 8 <= len(data):
            kind, length = data[at:at + 4], struct.unpack("<I", data[at + 4:at + 8])[0]
            if kind == b"fmt ":
                rate = struct.unpack("<I", data[at + 16:at + 20])[0]
            elif kind == b"data":
                size = length
            at += 8 + length + (length & 1)
        return round(size / rate, 3) if rate and size else None
    except (OSError, struct.error):
        return None


def origin(path):
    if path.name.startswith(PRIVATE):
        return "private"
    if path.name.startswith("ext_"):
        return "recording"
    return "synth"


def file_entries(files, displaces):
    """Files with length and origin; with `displaces`, a folder's `priv_*` files
    push out the others (sound_library.gd)."""
    has_private = displaces and any(p.name.startswith(PRIVATE) for p in files)
    out = []
    for p in files:
        out.append({
            "path": p.relative_to(ROOT).as_posix(),
            "file": p.name,
            "seconds": wav_seconds(p) if p.suffix.lower() == ".wav" else None,
            "origin": origin(p),
            "displaced": has_private and not p.name.startswith(PRIVATE),
        })
    return out


def weapon_slug(name):
    """Folder name of a catalog weapon: the rule of sound_classes.gd and
    tools/audio/synth.py (which needs numpy, so it is not imported here)."""
    out, gap = [], False
    for c in re.sub(r"\([^)]*\)", " ", name).lower():
        if c.isascii() and c.isalnum():
            if gap and out:
                out.append("_")
            out.append(c)
            gap = False
        else:
            gap = True
    return "".join(out)


def weapon_class(slug):
    """The class a weapon recording replaces, from the catalog (sound_classes.gd shot_class)."""
    for source in sorted((ROOT / "data" / "catalog").glob("*.toml")):
        for block in source.read_text(encoding="utf-8").split("[[unit]]")[1:]:
            category = re.search(r'^category\s*=\s*"([^"]+)"', block, re.M)
            for m in re.finditer(r'\{\s*name\s*=\s*"([^"]+)"([^}]*)\}', block):
                if weapon_slug(m.group(1)) != slug:
                    continue
                caliber = re.search(r"caliber_mm_x10\s*=\s*(\d+)", m.group(2))
                rate = re.search(r"rate_rpm\s*=\s*(\d+)", m.group(2))
                return shot_class(category.group(1) if category else "",
                                  int(caliber.group(1)) / 10 if caliber else 0.0,
                                  int(rate.group(1)) if rate else 0, m.group(1).lower())
    return None


def shot_class(category, caliber, rate, name):
    """Mirror of sound_classes.gd shot_class (the page only shows its manifest values)."""
    if caliber < 20:
        return "shot_mg" if rate >= 300 else "shot_rifle"
    if category == "inf_at":
        return "shot_rocket"
    if category.startswith("inf_mortar") or category == "mortar_heavy" or any(
            w in name for w in ("mortar", "granatwerfer", "grw")):
        return "shot_mortar"
    if category == "gun_howitzer" or (category == "self_propelled" and caliber >= 100):
        return "shot_howitzer"
    return "shot_cannon_small" if caliber < 37 else "shot_cannon_medium" if caliber < 75 else "shot_cannon_large"


def build_index():
    manifest, found = folders()
    usage = read_usage()
    specs = manifest["classes"]
    rows = {key: [] for key, _ in SECTIONS}

    def row(section, name, anchor, remark, files, spec=None, loop=False, displaces=True, note=None):
        rows[section].append({
            "name": name, "anchor": anchor, "remark": remark, "spec": spec, "loop": loop,
            "note": note, "files": file_entries(files, displaces),
        })

    for name, files in found["class"].items():
        if not files:
            continue
        spec = specs.get(name)
        row(section_of(name) or "objects", name, name, usage.get("class", {}).get(name, ""), files,
            spec, bool(spec and spec.get("loop")))
    for slug, files in found["weapon"].items():
        if not files:
            continue
        base = weapon_class(slug)
        row("weapons", f"weapon/{slug}", f"weapon-{slug}", usage.get("weapon", {}).get(slug, ""), files,
            specs.get(base), note=f"Werte von {base}" if base else "keine Katalogwaffe")
    for callout, files in found["voice"].items():
        if files:
            row("voices", f"voice/*/{callout}", f"voice-{callout}", usage.get("voice", {}).get(callout, ""),
                files, manifest.get("voice"))
    # A row per faction, so the languages can be compared.
    for path in sorted((AUDIO / "announce").glob("*/*/")):
        files = audio_files(path)
        if files:
            name = f"announce/{path.parent.name}/{path.name}"
            row("voices", name, name.replace("/", "-"), usage.get("announce", {}).get(path.name, ""),
                files, manifest.get("announce"))
    for layer, files in found["ambience"].items():
        if files:
            row("ambience", f"ambience/{layer}", f"ambience-{layer}", usage.get("ambience", {}).get(layer, ""),
                files, loop=layer in AMBIENCE_LOOPS, note="Pegel im Code (ambience.gd)")
    for mood, files in found["music"].items():
        row("music", f"music/{mood}", f"music-{mood}", usage.get("music", {}).get(mood, ""), files,
            note="Pegel −8 dB im Code (music.gd)")
    index = {
        "sections": [{"key": key, "title": title, "rows": rows[key]} for key, title in SECTIONS if rows[key]],
    }
    INDEX.write_text(json.dumps(index, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    count = sum(len(r["files"]) for s in index["sections"] for r in s["rows"])
    print(f"sounds-web: {count} files in {sum(len(s['rows']) for s in index['sections'])} rows", flush=True)
    return index


class Handler(http.server.SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_GET(self):
        if self.path == "/":
            self.send_response(302)
            self.send_header("Location", "/web/sounds/")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        super().do_GET()

    def log_message(self, *args):
        pass


class Server(http.server.ThreadingHTTPServer):
    request_queue_size = 256
    daemon_threads = True


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=8138)
    parser.add_argument("--no-open", action="store_true", help="do not open the browser")
    parser.add_argument("--check", action="store_true", help="check the remarks and exit")
    args = parser.parse_args()
    if args.check:
        raise SystemExit(0 if check() else 1)
    if not check():
        print("sounds-web: serving anyway; rows without a remark show none", flush=True)
    build_index()
    handler = functools.partial(Handler, directory=str(ROOT))
    server = Server(("127.0.0.1", args.port), handler)
    address = f"http://localhost:{args.port}/web/sounds/"
    print(f"sounds-web: {address} (Ctrl+C stops)", flush=True)
    if not args.no_open:
        webbrowser.open(address)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
