#!/usr/bin/env python3
"""Checks that an export will ship the private sounds (docs/RELEASE.md).

    python3 -I tools/release/check_private_sounds.py

The voice lines (spoken announcements, prompt 244) and the other sounds of
the private repository `mud-and-steel-sfx` are in the game only as gitignored
`godot/assets/audio/**/priv_*` files, prepared by `just audio-private` and
imported by `just import`. A release built without them would be silent where
a voice should speak, and nothing else would notice. So `tools/release/export.sh`
runs this before every release export and stops on a problem:

- every message of `ANNOUNCE` in `tools/audio/synth.py` has a prepared file
  for every faction of `ANNOUNCE_FACTIONS`;
- where the private repository is there (`../mud-and-steel-sfx` or
  `MUD_AND_STEEL_SFX`): as many prepared files as it has takes, so a new
  take is not forgotten;
- every `priv_*` file has its `.import` file (else Godot leaves it out).

Exit code 1 on a problem. Needs only the standard library.
"""

import ast
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AUDIO = ROOT / "godot" / "assets" / "audio"
SYNTH = ROOT / "tools" / "audio" / "synth.py"
PRIVATE_DIR = Path(os.environ.get("MUD_AND_STEEL_SFX", ROOT.parent / "mud-and-steel-sfx"))
SOURCE_EXTENSIONS = {".wav", ".ogg", ".mp3", ".flac"}
FIX = "run `just audio-private` and `just import` with the private sound repository in place"


def synth_tables() -> dict:
    """`ANNOUNCE` and `ANNOUNCE_FACTIONS` of synth.py, read without importing it (numpy)."""
    tables = {}
    for node in ast.parse(SYNTH.read_text(encoding="utf-8")).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            if node.targets[0].id in ("ANNOUNCE", "ANNOUNCE_FACTIONS"):
                tables[node.targets[0].id] = ast.literal_eval(node.value)
    return tables


def main() -> int:
    tables = synth_tables()
    problems = []
    lines = 0
    for message, folder in tables["ANNOUNCE"].items():
        for language, faction in tables["ANNOUNCE_FACTIONS"].items():
            out = AUDIO / "announce" / faction / message
            prepared = sorted(out.glob("priv_*.wav")) if out.is_dir() else []
            lines += len(prepared)
            if not prepared:
                problems.append(f"no voice line '{message}' for {faction} in {out.relative_to(ROOT)}/")
            takes = PRIVATE_DIR / "elevenlabs" / folder / language
            if takes.is_dir():
                count = len([p for p in takes.iterdir() if p.suffix.lower() in SOURCE_EXTENSIONS])
                if prepared and count != len(prepared):
                    problems.append(f"voice line '{message}' for {faction}: {len(prepared)} prepared, "
                                    f"{count} take(s) in the private repository")
    private = sorted(p for p in AUDIO.rglob("priv_*") if p.suffix != ".import")
    for path in private:
        if not path.with_name(path.name + ".import").exists():
            problems.append(f"{path.relative_to(ROOT)} is not imported")
    for problem in problems:
        print(f"private sounds: {problem}", file=sys.stderr)
    if problems:
        print(f"private sounds: {len(problems)} problem(s); {FIX}", file=sys.stderr)
        return 1
    print(f"private sounds: ok ({lines} voice lines of {len(tables['ANNOUNCE'])} messages, "
          f"{len(private)} private files in all)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
