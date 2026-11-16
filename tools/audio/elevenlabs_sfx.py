#!/usr/bin/env python3
"""Generates sound effects with the ElevenLabs API (prompt 069).

    python3 tools/audio/elevenlabs_sfx.py                 # list the entries and their cost
    python3 tools/audio/elevenlabs_sfx.py first           # plan for the group `first` (no call)
    python3 tools/audio/elevenlabs_sfx.py first --yes     # call the API (`just sfx-generate first --yes`)

Reads `tools/audio/sfx_prompts.toml` (target class, prompt, length, loop,
number of variants) and writes `docs/.sfx/generated/<class>/<name>_NN.<ext>`.
Next to every file it writes `<file>.json` with the prompt, the model, the
date and the request parameters: that is the row for `docs/SOURCES.md` when
a file is taken over into `assets-src/audio/`.

Nothing is generated without `--yes`: the default run only prints the calls
and the estimated credits, so the maintainer can approve the cost first. A
run makes at most 12 calls (`--max N` raises it). Files that already exist
are skipped, so a repeated run costs nothing.

The API key comes only from the environment variable `ELEVENLABS_API_KEY`.
It is never written to a file or printed.

API (read 2026-11-16, https://elevenlabs.io/docs/api-reference/text-to-sound-effects/convert):
POST /v1/sound-generation, header `xi-api-key`, query `output_format`, body
`text`, `duration_seconds` (0.5 to 30), `prompt_influence` (0 to 1, default
0.3), `loop`, `model_id`. Cost: 40 credits per second when the duration is
set (https://elevenlabs.io/docs/overview/capabilities/sound-effects).
WAV 48 kHz is offered for non-looping effects; loops come as MP3.

Needs only the standard library.
"""

from __future__ import annotations

import datetime
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROMPTS = ROOT / "tools" / "audio" / "sfx_prompts.toml"
OUT_DIR = ROOT / "docs" / ".sfx" / "generated"
API_URL = "https://api.elevenlabs.io/v1/sound-generation"
KEY_VARIABLE = "ELEVENLABS_API_KEY"
CREDITS_PER_SECOND = 40
MAX_CALLS = 12
DEFAULTS = {"model": "eleven_text_to_sound_v2", "prompt_influence": 0.3, "variants": 1, "loop": False}


def parse_value(text: str):
    text = text.strip()
    if text.startswith('"') and text.endswith('"'):
        return json.loads(text)
    if text in ("true", "false"):
        return text == "true"
    return float(text) if "." in text else int(text)


def read_prompts(path: Path) -> tuple[dict, list[dict]]:
    """Reads the small TOML subset the list uses: `key = value` lines with
    strings, numbers and booleans, and `[[sound]]` tables. (Python 3.10 has
    no tomllib, and the tools install nothing.)"""
    try:
        import tomllib
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        return {k: v for k, v in data.items() if k != "sound"}, data.get("sound", [])
    except ModuleNotFoundError:
        pass
    top: dict = {}
    sounds: list[dict] = []
    target = top
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line == "[[sound]]":
            target = {}
            sounds.append(target)
            continue
        key, sep, value = line.partition("=")
        if not sep:
            raise SystemExit(f"{path.name}:{number}: expected `key = value`")
        target[key.strip()] = parse_value(value)
    return top, sounds


def planned_calls(top: dict, sounds: list[dict], selection: list[str]) -> list[dict]:
    calls = []
    for sound in sounds:
        entry = {**DEFAULTS, **top, **sound}
        for field in ("name", "group", "class", "prompt"):
            if field not in entry:
                raise SystemExit(f"sfx_prompts.toml: a [[sound]] lacks `{field}`")
        if selection and entry["group"] not in selection and entry["name"] not in selection:
            continue
        loop = bool(entry["loop"])
        extension = "mp3" if loop else "wav"
        for variant in range(1, int(entry["variants"]) + 1):
            calls.append({
                **entry,
                "variant": variant,
                "output_format": entry.get("output_format", "mp3_44100_128" if loop else "wav_48000"),
                "path": OUT_DIR / entry["class"] / f"{entry['name']}_{variant:02d}.{extension}",
            })
    return calls


def credits(call: dict) -> int | None:
    seconds = call.get("duration_seconds")
    return None if seconds is None else round(seconds * CREDITS_PER_SECOND)


def generate(call: dict, key: str) -> bytes:
    body = {
        "text": call["prompt"],
        "model_id": call["model"],
        "prompt_influence": call["prompt_influence"],
        "loop": bool(call["loop"]),
    }
    if call.get("duration_seconds") is not None:
        body["duration_seconds"] = call["duration_seconds"]
    request = urllib.request.Request(
        f"{API_URL}?output_format={call['output_format']}",
        data=json.dumps(body).encode("utf-8"),
        headers={"xi-api-key": key, "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=180) as response:
        return response.read()


def sidecar(call: dict) -> dict:
    return {
        "tool": "ElevenLabs Sound Effects API (POST /v1/sound-generation)",
        "model": call["model"],
        "prompt": call["prompt"],
        "duration_seconds": call.get("duration_seconds"),
        "prompt_influence": call["prompt_influence"],
        "loop": bool(call["loop"]),
        "output_format": call["output_format"],
        "class": call["class"],
        "date": datetime.date.today().isoformat(),
    }


def main(argv: list[str]) -> int:
    go = "--yes" in argv
    limit = MAX_CALLS
    selection = []
    arguments = iter(a for a in argv if a != "--yes")
    for argument in arguments:
        if argument == "--max":
            limit = int(next(arguments))
        elif argument.startswith("--"):
            raise SystemExit(f"unknown option {argument}")
        else:
            selection.append(argument)

    top, sounds = read_prompts(PROMPTS)
    calls = planned_calls(top, sounds, selection)
    if not calls:
        groups = sorted({s.get("group", "?") for s in sounds})
        raise SystemExit(f"nothing selected; groups: {', '.join(groups)}")
    pending = [c for c in calls if not c["path"].exists()]

    total = 0
    unknown = 0
    for call in calls:
        cost = credits(call)
        state = "exists" if call["path"].exists() else "new"
        if state == "new":
            total += cost or 0
            unknown += cost is None
        length = f"{call['duration_seconds']:.1f} s" if call.get("duration_seconds") is not None else "auto"
        print(f"{state:6s} {str(call['path'].relative_to(ROOT)):62s} {length:7s} "
              f"{'loop ' if call['loop'] else ''}{cost if cost is not None else '?'} credits")
    note = f" plus {unknown} call(s) of automatic length (cost not documented)" if unknown else ""
    print(f"{len(pending)} call(s) to make, about {total} credits{note}.")

    if not go:
        print("Dry run. Add --yes to call the API.")
        return 0
    if len(pending) > limit:
        raise SystemExit(f"{len(pending)} calls exceed the limit of {limit} per run; select fewer or pass --max N")
    key = os.environ.get(KEY_VARIABLE, "").strip()
    if not key:
        raise SystemExit(f"set the environment variable {KEY_VARIABLE} first")

    for call in pending:
        try:
            audio = generate(call, key)
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", "replace")[:500]
            raise SystemExit(f"{call['name']}: HTTP {error.code}: {detail}")
        call["path"].parent.mkdir(parents=True, exist_ok=True)
        call["path"].write_bytes(audio)
        Path(f"{call['path']}.json").write_text(json.dumps(sidecar(call), indent=2) + "\n", encoding="utf-8")
        print(f"wrote  {call['path'].relative_to(ROOT)} ({len(audio) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
