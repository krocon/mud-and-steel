#!/usr/bin/env python3
"""Serve the unit browser (web/units/) from the repository root, or sync its copy.

Served locally, the page reads the catalog, the camouflage palettes and the models
straight from the repository; the only generated files are the price table and the
game values of the simulation (`cli economy-table` -> web/units/economy.md,
`cli unit-stats` -> web/units/stats.json). Usage: `just units-web`.

The website serves web/units/ as its root, so there the page finds the same files
under web/units/ itself. `--sync` (`just units-web-sync`) copies every file the page
loads to that place, removes copies that are no longer needed and writes the prices
and game values.
"""

import argparse
import functools
import http.server
import pathlib
import re
import shutil
import subprocess
import threading
import webbrowser

ROOT = pathlib.Path(__file__).resolve().parent.parent


class Handler(http.server.SimpleHTTPRequestHandler):
    # Keep-alive: the page loads a few hundred small files.
    protocol_version = "HTTP/1.1"

    def end_headers(self):
        # Always show the current catalog and models.
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_GET(self):
        if self.path == "/":
            self.send_response(302)
            self.send_header("Location", "/web/units/")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        super().do_GET()

    def log_message(self, *args):
        pass


class Server(http.server.ThreadingHTTPServer):
    # The page asks for many thumbnails and models at once.
    request_queue_size = 256
    daemon_threads = True


WEB = ROOT / "web" / "units"
# Top-level folders of the copy under web/units/ (everything in them is synced).
MIRRORED = ("data", "godot")


def write_prices():
    """Runs in the background when serving: cargo may wait for another build."""
    out = WEB / "economy.md"
    try:
        table = subprocess.run(
            ["cargo", "run", "-q", "-p", "cli", "--", "economy-table"],
            cwd=ROOT, check=True, stdout=subprocess.PIPE,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"units-web: no prices ({error})", flush=True)
        return
    out.write_bytes(table)
    return True


def write_stats():
    """Game values of every unit, computed by the simulation (`cli unit-stats`)."""
    try:
        cards = subprocess.run(
            ["cargo", "run", "-q", "-p", "cli", "--", "unit-stats"],
            cwd=ROOT, check=True, stdout=subprocess.PIPE,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"units-web: no game values ({error})", flush=True)
        return
    (WEB / "stats.json").write_bytes(cards)
    return True


def write_generated():
    """Both generated files, one cargo run after the other."""
    prices = write_prices()
    return write_stats() and prices


def page_files():
    """Repository paths of every file the page loads (see app.js and viewer.js)."""
    catalogs = [f"data/catalog/{f}.toml" for f in ("de", "su", "us", "uk", "neutral")]
    files = set(catalogs) | {
        "data/economy.toml",
        "godot/assets/models/infantry/uniform_palettes.json",
        "godot/assets/models/infantry/animations.json",
    }
    files |= {p.relative_to(ROOT).as_posix() for p in (ROOT / "godot/assets/models/infantry/anims").glob("*.glb")}
    # Vehicle database (prompt 064): technical data per faction.
    files |= {f"data/vehicles/{f}.json" for f in ("de", "su", "us")}
    files |= {p.relative_to(ROOT).as_posix() for p in (ROOT / "godot/assets/materials/camo").glob("*.tres")}
    files |= {p.relative_to(ROOT).as_posix() for p in (ROOT / "godot/assets/thumbnails").glob("*.png")}
    for source in catalogs + ["data/economy.toml"]:
        for model in re.findall(r'^model\s*=\s*"([^"]+)"', (ROOT / source).read_text(), re.M):
            files.add(f"godot/{model}")
            if model.endswith(".tscn"):
                scene = (ROOT / "godot" / model).read_text()
                files |= {f"godot/{p}" for p in re.findall(r'\[ext_resource [^\]]*path="res://([^"]+)"', scene)}
    return files


def sync():
    """Copies the page's files to web/units/ and removes copies nobody loads."""
    wanted = page_files()
    missing = sorted(f for f in wanted if not (ROOT / f).is_file())
    copied = removed = 0
    for f in sorted(wanted - set(missing)):
        source, target = ROOT / f, WEB / f
        if target.is_file() and target.read_bytes() == source.read_bytes():
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        copied += 1
    for top in MIRRORED:
        for target in sorted((WEB / top).rglob("*"), reverse=True):
            if target.is_file() and target.relative_to(WEB).as_posix() not in wanted:
                target.unlink()
                removed += 1
            elif target.is_dir() and not any(target.iterdir()):
                target.rmdir()
    print(f"units-web: {len(wanted)} files, {copied} copied, {removed} removed", flush=True)
    for f in missing:
        print(f"units-web: missing in the repository: {f}", flush=True)
    # Every catalog entry with a model gets a list picture (`just thumbnails --only=...`).
    ids = []
    for source in [f"data/catalog/{f}.toml" for f in ("de", "su", "us", "uk", "neutral")]:
        for block in (ROOT / source).read_text().split("[[unit]]")[1:]:
            entry = re.search(r'^id\s*=\s*"([^"]+)"', block, re.M)
            if entry and re.search(r'^model\s*=\s*"[^"]', block, re.M):
                ids.append(entry.group(1))
    no_thumb = [i for i in ids if not (ROOT / f"godot/assets/thumbnails/{i}.png").is_file()]
    if no_thumb:
        print(f"units-web: no thumbnail, run: just thumbnails --only={','.join(no_thumb)}", flush=True)
    return write_generated() and not missing


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8137)
    parser.add_argument("--no-open", action="store_true", help="do not open the browser")
    parser.add_argument("--sync", action="store_true", help="sync the copy under web/units/ and exit")
    args = parser.parse_args()
    if args.sync:
        raise SystemExit(0 if sync() else 1)

    threading.Thread(target=write_generated, daemon=True).start()
    handler = functools.partial(Handler, directory=str(ROOT))
    server = Server(("127.0.0.1", args.port), handler)
    address = f"http://localhost:{args.port}/web/units/"
    print(f"units-web: {address} (Ctrl+C stops)", flush=True)
    if not args.no_open:
        webbrowser.open(address)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
