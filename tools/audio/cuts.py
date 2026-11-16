#!/usr/bin/env python3
"""Cuts the game files of prompt 069 out of the downloaded recordings.

    python3 tools/audio/cuts.py            # all classes
    python3 tools/audio/cuts.py tracks     # only these

Reads the Freesound files in `docs/.sfx/freesound/` (gitignored raw
material; pages and licenses: docs/SOURCES.md) and writes the finished
cuts to `assets-src/audio/<class>/<author><id>_NN.wav` (mono, 44.1 kHz,
16 bit). `just audio <class>` then normalizes them into the game. The cuts
are committed, so this script is only needed to change a cut.

Private classes (named explicitly: `cuts.py shot_mg`) are cut from files
that must not be in the public repository and are written to the private
one (`../mud-and-steel-sfx`, see tools/audio/synth.py).

What it adds to a recording is synthesized here (own work): a low body
under shots that lack bass, a crack on a shot that lacks highs, the muffled
bang inside the hull after a penetration. Variants of a single recording
differ in pitch (resampled a few percent) and in these layers.

Needs numpy and ffmpeg.
"""

from __future__ import annotations

import os
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np

SR = 44100
ROOT = Path(__file__).resolve().parents[2]
SFX = ROOT / "docs" / ".sfx"
RAW = SFX / "freesound"
OUT = ROOT / "assets-src" / "audio"
PRIVATE = Path(os.environ.get("MUD_AND_STEEL_SFX", ROOT.parent / "mud-and-steel-sfx"))


def load(rel: str, root: Path = RAW) -> np.ndarray:
    path = root / rel
    if not path.exists():
        raise SystemExit(f"{path} is missing (raw downloads are not in git; see docs/SOURCES.md)")
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-ac", "1", "-ar", str(SR), "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(np.float64)


def cut(x: np.ndarray, start: float, length: float, fade: float = 0.25) -> np.ndarray:
    """`length` seconds from `start`, a little before the onset kept out,
    faded out over the last `fade` seconds."""
    part = x[int(start * SR):int((start + length) * SR)].copy()
    n = min(int(fade * SR), len(part))
    part[-n:] *= np.cos(np.linspace(0, np.pi / 2, n)) ** 2
    return part


def pitch(x: np.ndarray, factor: float) -> np.ndarray:
    """Resampled: factor > 1 is higher and shorter."""
    positions = np.arange(0, len(x) - 1, factor)
    return np.interp(positions, np.arange(len(x)), x)


def band(x: np.ndarray, lo: float = 0.0, hi: float = SR / 2) -> np.ndarray:
    """Zero-phase band filter with soft edges (half an octave)."""
    spectrum = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1 / SR) + 1e-9
    gain = np.ones_like(f)
    if lo > 0:
        gain *= np.clip(np.log2(f / lo) * 2 + 1, 0, 1)
    if hi < SR / 2:
        gain *= np.clip(np.log2(hi / f) * 2 + 1, 0, 1)
    return np.fft.irfft(spectrum * gain, len(x))


def norm(x: np.ndarray) -> np.ndarray:
    return x / (np.max(np.abs(x)) + 1e-12)


def mix(*layers: tuple[np.ndarray, float, float]) -> np.ndarray:
    """Sums (samples, gain, delay in seconds) layers."""
    n = max(len(x) + int(delay * SR) for x, _, delay in layers)
    out = np.zeros(n)
    for x, gain, delay in layers:
        start = int(delay * SR)
        out[start:start + len(x)] += x * gain
    return out


def thump(seed: int, f_start: float, f_end: float, decay: float) -> np.ndarray:
    """Low body of a shot or blast: a falling sine with a little noise."""
    rng = np.random.default_rng(seed)
    n = int(decay * 6 * SR)
    t = np.arange(n) / SR
    freq = f_end + (f_start - f_end) * np.exp(-t / (decay * 0.5))
    body = np.sin(2 * np.pi * np.cumsum(freq) / SR)
    rumble = band(rng.standard_normal(n), 25, f_start * 1.5)
    return norm((body + 0.4 * norm(rumble)) * np.exp(-t / decay))


def crack(seed: int, decay: float) -> np.ndarray:
    """Muzzle crack: a very short burst of high noise over a mid body."""
    rng = np.random.default_rng(seed)
    n = int(0.35 * SR)
    t = np.arange(n) / SR
    snap = band(rng.standard_normal(n), 2500) * np.exp(-t / decay)
    body = band(rng.standard_normal(n), 500, 4000) * np.exp(-t / (decay * 3))
    return norm(snap + 0.6 * body)


def inner_bang(seed: int) -> np.ndarray:
    """Muffled bang inside a steel hull after a penetration."""
    rng = np.random.default_rng(seed)
    n = int(0.9 * SR)
    t = np.arange(n) / SR
    return norm(band(rng.standard_normal(n), 40, 260) * np.exp(-t / 0.16) * (1 - np.exp(-t / 0.004)))


def save(sound_class: str, name: str, x: np.ndarray, root: Path = OUT) -> None:
    x = norm(x) * 10 ** (-1 / 20)
    path = root / sound_class / f"{name}.wav"
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(np.round(x * 32767).astype("<i2").tobytes())
    print(f"{path}  {len(x) / SR:.2f} s")


def clear(sound_class: str, root: Path = OUT) -> None:
    folder = root / sound_class
    if folder.is_dir():
        for old in folder.glob("*.wav"):
            old.unlink()


# --- classes ---------------------------------------------------------------------------

def shot_cannon_small() -> None:
    """25 mm autocannon (qubodup 854183): one clean shot; variants by pitch
    with a short low body, because the recording has almost no bass."""
    shot = load("shot_cannon_small/qubodup854183.mp3")
    for i, factor in enumerate([1.0, 0.95, 1.05, 0.9]):
        body = thump(11 + i, 150, 70, 0.07)
        save("shot_cannon_small", f"qubodup854183_{i + 1:02d}", mix((pitch(shot, factor), 1.0, 0.0), (body, 0.45, 0.0)))


def shot_cannon_medium() -> None:
    """76 mm field gun, 1958 (YleArkisto 332935): the five shots of the
    tape. The tape has no highs, so each shot gets a synthesized crack."""
    tape = load("shot_cannon_medium/ylearkisto332935.mp3")
    for i, start in enumerate([0.71, 6.68, 12.30, 19.17, 26.96]):
        shot = cut(tape, start, 2.6, 0.8)
        save("shot_cannon_medium", f"ylearkisto332935_{i + 1:02d}", mix((norm(shot), 1.0, 0.0), (crack(21 + i, 0.012), 0.5, 0.0)))


def shot_cannon_large() -> None:
    """120 mm tank gun on a range (qubodup 189344): the first four shots.
    The video sound has no bass, so each shot gets a low body."""
    reel = load("shot_cannon_large/qubodup189344.mp3")
    for i, (start, length) in enumerate([(0.14, 3.4), (3.81, 3.4), (8.40, 2.9), (11.39, 3.1)]):
        shot = cut(reel, start, length, 0.8)
        body = thump(31 + i, 95, 38, 0.22)
        save("shot_cannon_large", f"qubodup189344_{i + 1:02d}", mix((norm(shot), 1.0, 0.0), (body, 0.8, 0.0)))


def hit_penetration() -> None:
    """Steel door hit (LPA134 329515) as the shell striking the plate, then
    the muffled bang inside the hull."""
    clang = cut(load("hit_armor/lpa134329515.mp3"), 0.0, 1.5, 0.6)
    for i, factor in enumerate([1.0, 0.9, 1.12]):
        save("hit_penetration", f"lpa134329515_{i + 1:02d}",
             mix((pitch(clang, factor), 1.0, 0.0), (inner_bang(41 + i), 0.9, 0.035)))


def hit_ricochet() -> None:
    """.22 rifle with ricochet (C-V 523403): the bang stands for the strike,
    the whine flies off. Lower pitches for the heavier shots."""
    whole = load("hit_ricochet/cv523403.mp3")
    for i, factor in enumerate([0.8, 0.7, 0.9]):
        save("hit_ricochet", f"cv523403_{i + 1:02d}", pitch(whole, factor))


def explosion(sound_class: str, factor: float, length: float, debris_gain: float, starts: list[float]) -> None:
    blast = load("explosion/modusmogulus737321.mp3")
    scrap = load("debris/samsterbirdies587443.mp3")
    for i, start in enumerate(starts):
        boom = cut(pitch(blast, factor * (1.0 + 0.03 * (i - 1))), 0.0, length, length * 0.4)
        layers = [(norm(boom), 1.0, 0.0)]
        if debris_gain > 0:
            layers.append((norm(band(cut(scrap, start, 2.3, 0.8), 300)), debris_gain, 0.35))
        save(sound_class, f"modusmogulus737321_{i + 1:02d}", mix(*layers))


def explosion_vehicle() -> None:
    """Designed blast from own recordings (modusmogulus 737321), a little
    lower, with falling scrap metal (SamsterBirdies 587443) as debris."""
    explosion("explosion_vehicle", 0.9, 5.5, 0.3, [2.46, 7.99, 12.11])


def hit_shell() -> None:
    """The same blast, shorter: a high-explosive shell bursting."""
    explosion("hit_shell", 1.08, 2.6, 0.0, [0, 0, 0, 0])


def hit_shell_small() -> None:
    """Higher and shorter still: 20 to 36 mm shells."""
    explosion("hit_shell_small", 1.4, 1.2, 0.0, [0, 0, 0])


def explosion_tail() -> None:
    """Explosion in the woods at a distance, echoing (YleArkisto 256296)."""
    echo = load("explosion_tail/ylearkisto256296.mp3")
    for i, factor in enumerate([1.0, 0.92, 1.07]):
        save("explosion_tail", f"ylearkisto256296_{i + 1:02d}", pitch(cut(echo, 0.38, 4.9, 1.5), factor))


def tracks() -> None:
    """Tanks on a street, tracks creaking, 1969 (YleArkisto 386660): the
    steadiest 8 s without the engine's low end, crossfaded into a loop."""
    street = band(load("tracked/ylearkisto386660.mp3"), 350)
    length, fade = int(8.0 * SR), int(0.6 * SR)
    start = int(48.0 * SR)
    part = street[start:start + length + fade].copy()
    ramp = np.linspace(0, np.pi / 2, fade)
    loop = part[:length]
    loop[:fade] = part[:fade] * np.sin(ramp) + part[length:] * np.cos(ramp)
    save("tracks", "ylearkisto386660_01", loop)


def onsets(x: np.ndarray, gap: float = 0.06) -> list[int]:
    """Sample positions of the shots of a burst."""
    hop = int(0.002 * SR)
    step = np.diff(x, prepend=0.0)
    n = len(step) // hop
    energy = np.sqrt((step[:n * hop].reshape(n, hop) ** 2).mean(axis=1))
    out: list[int] = []
    for i in range(8, n):
        if energy[i] > 0.25 * energy.max() and energy[i] > 2.5 * np.median(energy[i - 8:i]) and (not out or (i * hop - out[-1]) > gap * SR):
            out.append(i * hop)
    return out


def shot_mg() -> None:
    """Private: single shots out of a machine-gun burst generated with
    ElevenLabs (docs/.sfx/MG-Sound, file #1, about 10 rounds per second).
    Each variant is one shot from the middle of the burst, running into the
    tail of the last shot, so a single shot rings out and a burst does not
    play a burst of its own. Written to the private repository."""
    files = sorted((SFX / "MG-Sound").glob("*#1-*.wav"))
    if not files:
        raise SystemExit("docs/.sfx/MG-Sound/*#1-*.wav is missing")
    burst = load(files[0].name, SFX / "MG-Sound")
    starts = onsets(burst)
    spacing = int(np.median(np.diff(starts)))
    lead, fade = int(0.002 * SR), int(0.015 * SR)
    tail = burst[starts[-1] + spacing - fade:starts[-1] + spacing + int(1.2 * SR)].copy()
    ramp = np.linspace(0, np.pi / 2, fade)
    clear("shot_mg", PRIVATE)
    for i, k in enumerate([len(starts) // 5, 2 * len(starts) // 5, 3 * len(starts) // 5, 4 * len(starts) // 5]):
        shot = burst[starts[k] - lead:starts[k] - lead + spacing].copy()
        shot[-fade:] = shot[-fade:] * np.cos(ramp) + tail[:fade] * np.sin(ramp)
        save("shot_mg", f"elevenlabs_mg1_{i + 1:02d}", np.concatenate([shot, cut(tail[fade:], 0.0, 1.2, 0.6)]), PRIVATE)


CLASSES = {f.__name__: f for f in [
    shot_cannon_small, shot_cannon_medium, shot_cannon_large, hit_penetration, hit_ricochet,
    explosion_vehicle, hit_shell, hit_shell_small, explosion_tail, tracks, shot_mg,
]}
PRIVATE_CLASSES = {"shot_mg"}


def main(argv: list[str]) -> int:
    for name in argv or list(CLASSES):
        if name not in CLASSES:
            raise SystemExit(f"unknown class '{name}'; known: {', '.join(CLASSES)}")
        if name in PRIVATE_CLASSES:
            if not argv:
                continue  # private classes only when named
        else:
            clear(name)
        CLASSES[name]()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
