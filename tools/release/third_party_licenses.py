#!/usr/bin/env python3
"""Writes THIRD_PARTY_LICENSES.txt for a release build (prompt 044).

Lists every Rust crate the game's GDExtension (`crates/bridge`, default
features, all target platforms) links, with version, license and source, and
the license texts the crates ship (LICENSE*, COPYING*, NOTICE*). Adds the
Steamworks API library. Godot's own licenses are shown in the game from the
engine itself (Credits menu), so they are not repeated here.

Usage: python3 tools/release/third_party_licenses.py [output]
(default: target/export/THIRD_PARTY_LICENSES.txt). Standard library only.
"""

import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ROOT_PACKAGE = "bridge"
LICENSE_PREFIXES = ("license", "licence", "copying", "notice", "unlicense")

STEAMWORKS = """\
Steamworks API (steam_api library)
    Copyright Valve Corporation. Redistributed as part of the Steamworks SDK
    (version 1.64, via the crate steamworks-sys) under the Steamworks SDK
    Access Agreement: https://partner.steamgames.com/documentation/sdk_access_agreement
"""


def metadata():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)


def shipped_packages(meta):
    """Ids of the root package's normal (non-dev, non-build) dependencies, transitively."""
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(p["id"] for p in meta["packages"] if p["name"] == ROOT_PACKAGE and p["source"] is None)
    seen, todo = set(), [root]
    while todo:
        node = nodes[todo.pop()]
        for dep in node["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]) and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                todo.append(dep["pkg"])
    # Our own workspace crates are covered by LICENSE.
    return sorted(
        (packages[i] for i in seen if packages[i]["source"] is not None),
        key=lambda p: (p["name"], p["version"]),
    )


def license_files(package):
    folder = os.path.dirname(package["manifest_path"])
    found = []
    for name in sorted(os.listdir(folder)):
        path = os.path.join(folder, name)
        if os.path.isfile(path) and name.lower().startswith(LICENSE_PREFIXES):
            with open(path, encoding="utf-8", errors="replace") as f:
                found.append((name, f.read().strip()))
    if package.get("license_file"):
        path = os.path.join(folder, package["license_file"])
        if os.path.isfile(path) and not any(n == os.path.basename(path) for n, _ in found):
            with open(path, encoding="utf-8", errors="replace") as f:
                found.append((os.path.basename(path), f.read().strip()))
    return found


def main():
    output = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "export", "THIRD_PARTY_LICENSES.txt")
    packages = shipped_packages(metadata())
    lines = [
        "Mud and Steel: third-party licenses",
        "",
        "The game includes the following third-party software. Godot Engine and",
        "its components are listed in the game under Credits > Godot Engine.",
        "",
        STEAMWORKS,
        "Rust crates (%d):" % len(packages),
        "",
    ]
    for p in packages:
        source = p.get("repository") or "https://crates.io/crates/%s" % p["name"]
        lines.append("    %s %s  (%s)  %s" % (p["name"], p["version"], p.get("license") or "see license file", source))
    lines.append("")
    texts = {}
    for p in packages:
        for name, text in license_files(p):
            texts.setdefault(text, []).append("%s %s (%s)" % (p["name"], p["version"], name))
    lines.append("License texts as shipped by the crates (identical texts are shown once):")
    lines.append("")
    for text, owners in texts.items():
        lines.append("=" * 78)
        lines.append("Used by: " + ", ".join(owners))
        lines.append("-" * 78)
        lines.append(text)
        lines.append("")
    missing = [p for p in packages if not license_files(p)]
    if missing:
        lines.append("=" * 78)
        lines.append("Crates without a license file; their SPDX license applies (https://spdx.org/licenses/):")
        for p in missing:
            lines.append("    %s %s: %s" % (p["name"], p["version"], p.get("license")))
    os.makedirs(os.path.dirname(os.path.abspath(output)), exist_ok=True)
    with open(output, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print("%s: %d crates, %d distinct license texts, %d without a license file" % (output, len(packages), len(texts), len(missing)))


if __name__ == "__main__":
    main()
