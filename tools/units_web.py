#!/usr/bin/env python3
"""Serve the unit browser (web/units/) from the repository root.

The page reads the catalog, the camouflage palettes and the models straight from
the repository, so the only generated file is the price table of the simulation
(`cli economy-table` -> build/units-web/economy.md). Usage: `just units-web`.
"""

import argparse
import functools
import http.server
import pathlib
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


def write_prices():
    """Runs in the background: cargo may wait for another build to finish."""
    out = ROOT / "build" / "units-web" / "economy.md"
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        table = subprocess.run(
            ["cargo", "run", "-q", "-p", "cli", "--", "economy-table"],
            cwd=ROOT, check=True, stdout=subprocess.PIPE,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"units-web: no prices ({error})", flush=True)
        return
    out.write_bytes(table)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8137)
    parser.add_argument("--no-open", action="store_true", help="do not open the browser")
    args = parser.parse_args()

    threading.Thread(target=write_prices, daemon=True).start()
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
