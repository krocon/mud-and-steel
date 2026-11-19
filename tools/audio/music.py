#!/usr/bin/env python3
"""Composes and renders the game's music and ambience (prompt 030).

    python3 tools/audio/music.py                 # everything (`just music`)
    python3 tools/audio/music.py calm_01 wind    # only pieces/layers whose name starts so
    python3 tools/audio/music.py --wav-dir DIR   # also write every file as wav into DIR

Writes
    godot/assets/audio/music/<piece>.mp3            stereo, encoded with lame
    godot/assets/audio/ambience/<layer>/<layer>_NN.wav   loops and one-shots

Everything is made here with numpy, no samples, presets or downloads:

- Music: hand-written chord progressions per piece, voiced for a few
  additive instruments (string pad, low strings, a soft piano-like pluck,
  timpani), melodies drawn from the chord and scale with a seeded random
  walk, a convolution hall, a gentle limiter. Pieces end in a fade and are
  cross-faded by the game (`scenes/audio/music.gd`), so they do not loop.
- Ambience beds (wind per biome, insects, leaves, water, rain) are built
  periodic like the loops of `synth.py`: noise is shaped in the frequency
  domain of the whole loop and events wrap around its end, so the loop has
  no seam. They carry a `smpl` loop chunk. Birds and distant rumble are
  one-shots the game scatters in time and space.

The output is deterministic for a numpy and lame version. Files of other
origin replace a piece: `assets-src/music/<piece>.(wav|mp3|ogg|flac)` is
converted with ffmpeg instead of composing it (its source belongs in
`docs/SOURCES.md`).

Private files (prompt 069 layout, the repository `../mud-and-steel-sfx` or
`MUD_AND_STEEL_SFX`): `music/<mood>_<name>.*` becomes
`music/priv_<mood>_<name>.mp3` (mixed to the same loudness, faded at the
end) and `ambience/<layer>/*.*` becomes `ambience/<layer>/priv_<name>.wav`
(converted to the layer's rate and channels; beds crossfaded into a loop).
The game plays only the `priv_` files of a mood or layer where there are
any (`music.gd`, `ambience.gd`); they are gitignored.

Requires numpy and `lame` (or ffmpeg with libmp3lame). Afterwards run
`just import` so Godot imports new files.
"""

from __future__ import annotations

import os
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
MUSIC_DIR = ROOT / "godot" / "assets" / "audio" / "music"
AMBIENCE_DIR = ROOT / "godot" / "assets" / "audio" / "ambience"
EXTERNAL_MUSIC = ROOT / "assets-src" / "music"
PRIVATE_DIR = Path(os.environ.get("MUD_AND_STEEL_SFX", ROOT.parent / "mud-and-steel-sfx"))
PRIVATE_PREFIX = "priv_"
SOURCE_EXTENSIONS = {".wav", ".mp3", ".ogg", ".flac"}
# Ambience layers played at a position (mono); the other beds are stereo.
MONO_LAYERS = {"leaves", "water"}
# Seconds of crossfade that close a private bed into a loop.
LOOP_CROSSFADE = 2.0
# Seconds of fade at the end of a private piece.
PIECE_FADE = 3.0
MUSIC_SR = 44100
BED_SR = 32000
WIND_SR = 22050  # wind has little above 9 kHz
SHOT_SR = 44100
PEAK_DBFS = -1.0
# Loudness of every piece (RMS of the mix), so the game's crossfades
# between them do not jump.
MUSIC_RMS_DBFS = -21.0
LAME_QUALITY = "5"  # VBR, about 130 kbit/s for this material


# --- Signal helpers -------------------------------------------------------------


def secs(seconds: float, sr: int) -> int:
    return int(round(seconds * sr))


def filt(x: np.ndarray, gain, sr: int) -> np.ndarray:
    """Circular filter in the frequency domain (periodic for loops)."""
    spectrum = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1.0 / sr)
    return np.fft.irfft(spectrum * gain(f), len(x))


def lp(fc: float, order: int = 2):
    return lambda f: 1.0 / np.sqrt(1.0 + (f / fc) ** (2 * order))


def hp(fc: float, order: int = 2):
    return lambda f: 1.0 / np.sqrt(1.0 + (fc / np.maximum(f, 1e-3)) ** (2 * order))


def bp(lo: float, hi: float, order: int = 2):
    low, high = hp(lo, order), lp(hi, order)
    return lambda f: low(f) * high(f)


def peak(fc: float, q: float):
    return lambda f: 1.0 / (1.0 + ((f - fc) / (fc / q)) ** 2)


def pink(f: np.ndarray) -> np.ndarray:
    return 1.0 / np.sqrt(np.maximum(f, 20.0))


def smooth_noise(rng: np.random.Generator, n: int, sr: int, fc: float) -> np.ndarray:
    """Periodic slow random curve in 0..1 (gusts, swells)."""
    x = filt(rng.standard_normal(n), lp(fc, 4), sr)
    x -= x.min()
    return x / (x.max() + 1e-12)


def add_wrapped(out: np.ndarray, x: np.ndarray, start: int) -> None:
    """Adds `x` at `start`, wrapping around the end (periodic loops)."""
    n = len(out)
    idx = (start + np.arange(len(x))) % n
    np.add.at(out, idx, x)


def add_at(out: np.ndarray, x: np.ndarray, start: int) -> None:
    end = min(len(out), start + len(x))
    if start < end:
        out[start:end] += x[: end - start]


def midi(note: float) -> float:
    return 440.0 * 2.0 ** ((note - 69.0) / 12.0)


# --- Instruments (music) ----------------------------------------------------------

SR = MUSIC_SR
TABLE = 4096


def wavetable(freq: float, amps, cutoff: float) -> np.ndarray:
    """One period of a tone with harmonic amplitudes `amps`, darkened
    above `cutoff` Hz and band-limited below Nyquist."""
    phase = np.arange(TABLE) / TABLE * 2 * np.pi
    table = np.zeros(TABLE)
    for k, a in enumerate(amps, start=1):
        fk = freq * k
        if fk > SR * 0.45:
            break
        table += a / (1.0 + (fk / cutoff) ** 2) * np.sin(k * phase)
    return table / (np.max(np.abs(table)) + 1e-12)


def osc(table: np.ndarray, freq: float, n: int, rng: np.random.Generator, vibrato: float = 0.0,
        vib_rate: float = 5.0, drift: float = 0.0) -> np.ndarray:
    t = np.arange(n) / SR
    inst = np.full(n, freq)
    if vibrato:
        # Vibrato fades in over the first half second, like a player's.
        inst = inst * (1.0 + vibrato * np.minimum(t / 0.5, 1.0) * np.sin(2 * np.pi * vib_rate * t + rng.uniform(0, 6.28)))
    if drift:
        inst = inst * (1.0 + drift * np.sin(2 * np.pi * rng.uniform(0.05, 0.15) * t + rng.uniform(0, 6.28)))
    pos = (rng.uniform(0, TABLE) + np.cumsum(inst) * TABLE / SR) % TABLE
    i = pos.astype(np.int64)
    frac = pos - i
    return table[i] * (1 - frac) + table[(i + 1) % TABLE] * frac


def adsr(n: int, attack: float, length: float, release: float) -> np.ndarray:
    t = np.arange(n) / SR
    rise = np.clip(t / max(attack, 1e-4), 0, 1)
    rise = np.sin(rise * np.pi / 2) ** 2
    fall = np.where(t < length, 1.0, np.exp(-(t - length) / max(release, 1e-4)))
    return rise * fall


SAW = [1.0 / k ** 1.15 for k in range(1, 40)]
CELLO = [1.0, 0.75, 0.55, 0.5, 0.32, 0.28, 0.2, 0.16, 0.12, 0.09, 0.07, 0.05, 0.04, 0.03]


def pad(rng, note: float, length: float, attack: float = 1.6, release: float = 2.4, cutoff: float = 1400.0) -> np.ndarray:
    """Soft string ensemble: three detuned voices, slow swell."""
    n = secs(length + release * 4, SR)
    freq = midi(note)
    table = wavetable(freq, SAW, cutoff)
    x = sum(osc(table, freq * 2 ** (c / 1200), n, rng, vibrato=0.0025, vib_rate=rng.uniform(4.5, 5.5), drift=0.0008)
            for c in (-8, 0, 7))
    return x / 3 * adsr(n, attack, length, release)


def strings(rng, note: float, length: float, attack: float = 0.25, release: float = 0.5, cutoff: float = 2200.0,
            vibrato: float = 0.004) -> np.ndarray:
    """A bowed low string (cello, bass) or a high sustained line."""
    n = secs(length + release * 4, SR)
    freq = midi(note)
    table = wavetable(freq, CELLO, cutoff)
    x = osc(table, freq, n, rng, vibrato=vibrato, vib_rate=rng.uniform(4.8, 5.6)) \
        + 0.6 * osc(table, freq * 2 ** (5 / 1200), n, rng, vibrato=vibrato)
    return x / 1.6 * adsr(n, attack, length, release)


def staccato(rng, note: float, length: float = 0.16, accent: float = 1.0) -> np.ndarray:
    """Short bowed note of the combat ostinato."""
    n = secs(length + 0.6, SR)
    freq = midi(note)
    table = wavetable(freq, CELLO, 1500 + 900 * accent)
    x = osc(table, freq, n, rng) + 0.7 * osc(table, freq * 2 ** (6 / 1200), n, rng)
    bow = filt(rng.standard_normal(n), bp(800, 3000), SR) * np.exp(-np.arange(n) / SR / 0.02) * 0.08
    return (x / 1.7 + bow) * adsr(n, 0.012, length, 0.09) * accent


def piano(rng, note: float, velocity: float = 0.7, length: float = 4.0) -> np.ndarray:
    """Soft, felt-like piano: inharmonic decaying partials of two slightly
    detuned strings and a dull hammer."""
    freq = midi(note)
    tau = 3.2 * (220.0 / freq) ** 0.35
    n = secs(min(length, tau * 3.5) + 0.4, SR)
    t = np.arange(n) / SR
    x = np.zeros(n)
    bright = 0.6 + 0.6 * velocity
    for k in range(1, 12):
        fk = freq * k * np.sqrt(1 + 0.00035 * k * k)
        if fk > 9000:
            break
        a = (1.0 / k ** (2.2 - bright)) * (1.0 / (1.0 + (fk / 2600) ** 2))
        d = tau / (1 + 0.55 * (k - 1))
        e = 0.65 * np.exp(-t / (d * 0.25)) + 0.35 * np.exp(-t / d)
        for detune in (1.0, 1.0007):
            x += a * e * np.sin(2 * np.pi * fk * detune * t + rng.uniform(0, 6.28))
    hammer = filt(rng.standard_normal(n), lp(1800), SR) * np.exp(-t / 0.006) * 0.15
    x = (x / 2 + hammer) * velocity
    # Damper at the end of the held length.
    held = min(length, t[-1])
    x *= np.where(t < held, 1.0, np.exp(-(t - held) / 0.12))
    return x * np.clip(t / 0.002, 0, 1)


def timpani(rng, note: float, velocity: float = 0.8) -> np.ndarray:
    freq = midi(note)
    n = secs(3.0, SR)
    t = np.arange(n) / SR
    glide = 1.0 + 0.025 * np.exp(-t / 0.06)
    x = np.zeros(n)
    for ratio, a, d in ((1.0, 1.0, 1.1), (1.5, 0.45, 0.6), (1.99, 0.3, 0.45), (2.44, 0.18, 0.3), (2.98, 0.1, 0.22)):
        phase = 2 * np.pi * np.cumsum(freq * ratio * glide) / SR
        x += a * np.sin(phase + rng.uniform(0, 6.28)) * np.exp(-t / d)
    mallet = filt(rng.standard_normal(n), bp(60, 900), SR) * np.exp(-t / 0.015) * 0.5
    return (x + mallet) * velocity * np.clip(t / 0.003, 0, 1)


def low_drum(rng, velocity: float = 0.6) -> np.ndarray:
    """Distant bass drum: an unpitched thud under the timpani."""
    n = secs(1.2, SR)
    t = np.arange(n) / SR
    body = np.sin(2 * np.pi * np.cumsum(48 + 30 * np.exp(-t / 0.04)) / SR) * np.exp(-t / 0.28)
    skin = filt(rng.standard_normal(n), lp(400), SR) * np.exp(-t / 0.03) * 0.4
    return (body + skin) * velocity


# --- Arrangement ------------------------------------------------------------------


class Piece:
    def __init__(self, seconds: float, seed: int, tempo: float):
        self.n = secs(seconds, SR)
        self.left = np.zeros(self.n)
        self.right = np.zeros(self.n)
        self.send = np.zeros(self.n)  # mono reverb send
        self.rng = np.random.default_rng(seed)
        self.beat = 60.0 / tempo

    def add(self, x: np.ndarray, at_beat: float, pan: float = 0.0, gain: float = 1.0, reverb: float = 0.3) -> None:
        start = secs(at_beat * self.beat, SR)
        angle = (pan + 1.0) * np.pi / 4
        add_at(self.left, x * gain * np.cos(angle), start)
        add_at(self.right, x * gain * np.sin(angle), start)
        add_at(self.send, x * gain * reverb, start)

    def render(self, fade_in: float = 1.0, fade_out: float = 5.0, hall: float = 0.9) -> np.ndarray:
        # Stereo hall: decaying noise, different per side, darker with time.
        ir_n = secs(hall * 6, SR)
        t = np.arange(ir_n) / SR
        sides = []
        for _ in range(2):
            ir = self.rng.standard_normal(ir_n) * np.exp(-t / hall) * np.clip((t - 0.025) / 0.03, 0, 1)
            ir = filt(np.concatenate([ir, np.zeros(ir_n)]), lp(3500), SR)[:ir_n]
            ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
            size = self.n + ir_n
            send = filt(self.send, hp(180), SR)
            wet = np.fft.irfft(np.fft.rfft(send, size) * np.fft.rfft(ir, size), size)[: self.n]
            sides.append(wet)
        mix = np.stack([self.left + sides[0], self.right + sides[1]], axis=1)
        mix = np.stack([filt(mix[:, c], hp(35), SR) for c in range(2)], axis=1)
        rms = np.sqrt(np.mean(mix ** 2)) + 1e-12
        mix *= 10 ** (MUSIC_RMS_DBFS / 20) / rms
        # Gentle limiter: soft above -6 dBFS, peak at PEAK_DBFS.
        ceiling = 10 ** (PEAK_DBFS / 20)
        knee = 0.5
        over = np.abs(mix) > knee
        mix[over] = np.sign(mix[over]) * (knee + (ceiling - knee) * np.tanh((np.abs(mix[over]) - knee) / (ceiling - knee)))
        ramp_in = secs(fade_in, SR)
        mix[:ramp_in] *= np.linspace(0, 1, ramp_in)[:, None]
        ramp_out = secs(fade_out, SR)
        mix[-ramp_out:] *= (np.linspace(1, 0, ramp_out) ** 1.5)[:, None]
        return mix


# Chords: (bass note, pad voicing). MIDI numbers.
D_MIN9 = (38, [57, 60, 64, 65])
G_ADD9 = (43, [55, 59, 62, 69])
BB_MAJ7 = (46, [53, 57, 62, 65])
C_ADD9 = (48, [55, 60, 62, 64])
A_MIN9 = (45, [55, 59, 60, 64])
F_MAJ7 = (41, [57, 60, 64, 65])
C_MAJ = (48, [55, 60, 64, 67])
E_MIN7 = (40, [55, 59, 62, 67])
E_MIN9 = (40, [54, 59, 62, 67])
C_MAJ9 = (36, [55, 59, 62, 64])
G_ADD9_HI = (43, [57, 59, 62, 67])
D_SUS2 = (38, [57, 62, 64, 69])
D_MIN = (38, [53, 57, 62, 65])
BB_MAJ = (46, [53, 58, 62, 65])
G_MIN = (43, [55, 58, 62, 67])
A_SUS4 = (45, [57, 62, 64, 69])
A_MAJ = (45, [57, 61, 64, 69])
F_MAJ_A = (45, [53, 57, 60, 65])
G_MIN7 = (43, [53, 58, 62, 65])
C_SUS4 = (48, [53, 55, 60, 65])
C7 = (48, [52, 55, 58, 64])
F_ADD9 = (41, [55, 57, 60, 65])
D_MIN7 = (38, [53, 57, 60, 65])
E_MIN = (40, [52, 59, 64, 67])
F_MAJ = (41, [53, 57, 60, 65])
D_MAJ = (38, [54, 57, 62, 66])
B_SUS4 = (47, [52, 59, 64, 66])
B_MAJ = (47, [51, 59, 63, 66])

D_DORIAN = [62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81]
A_AEOLIAN = [60, 62, 64, 67, 69, 71, 72, 74, 76, 79, 81]  # without F: open, pentatonic-ish
E_DORIAN = [62, 64, 66, 67, 69, 71, 74, 76, 78, 79, 81]
F_MAJOR = [60, 62, 64, 65, 67, 69, 72, 74, 76, 77, 79]
D_MINOR = [62, 64, 65, 67, 69, 70, 72, 74, 76, 77]


def melody(piece: Piece, scale: list[int], chord: tuple, start_beat: float, beats: float, last: int, density: float,
           velocity: float = 0.5, pan: float = 0.25) -> int:
    """A short piano phrase over one chord: a random walk on the scale that
    lands on chord tones, notes on the beat grid, a breath at the end.
    Returns the last note."""
    rng = piece.rng
    chord_pcs = {(n % 12) for n in chord[1]} | {chord[0] % 12}
    rhythms = [[1, 1, 2], [2, 1, 1], [1.5, 0.5, 2], [3, 1], [1, 1, 1, 1], [2, 2], [0.5, 0.5, 1, 2], [4]]
    beat = 0.0
    note = last
    while beat < beats - 1.0:
        if rng.random() > density:
            beat += 2.0
            continue
        pattern = rhythms[rng.integers(len(rhythms))]
        for length in pattern:
            if beat >= beats - 1.0:
                break
            i = int(np.argmin([abs(s - note) for s in scale]))
            step = rng.choice([-2, -1, -1, 0, 1, 1, 2], p=[0.1, 0.25, 0.1, 0.05, 0.25, 0.15, 0.1])
            i = int(np.clip(i + step, 0, len(scale) - 1))
            # Strong beats prefer chord tones.
            if beat % 2 == 0 and scale[i] % 12 not in chord_pcs:
                near = [j for j in (i - 1, i + 1) if 0 <= j < len(scale) and scale[j] % 12 in chord_pcs]
                if near:
                    i = near[rng.integers(len(near))]
            note = scale[i]
            vel = velocity * (1.0 if beat % 2 == 0 else 0.8) * rng.uniform(0.85, 1.05)
            piece.add(piano(rng, note, vel, length * piece.beat * 1.6 + 0.4), start_beat + beat, pan=pan, gain=0.55, reverb=0.45)
            beat += length
        beat += 1.0  # breath
    return note


def pads(piece: Piece, chords: list[tuple], start_beat: float, beats_per_chord: float, gain: float = 0.22,
         bass: bool = True, cutoff: float = 1400.0) -> None:
    for c, (low, voicing) in enumerate(chords):
        at = start_beat + c * beats_per_chord
        length = beats_per_chord * piece.beat
        for v, note in enumerate(voicing):
            piece.add(pad(piece.rng, note, length + 0.3, cutoff=cutoff), at, pan=-0.5 + v / max(len(voicing) - 1, 1),
                      gain=gain, reverb=0.5)
        if bass:
            piece.add(strings(piece.rng, low, length - 0.2, attack=0.8, release=1.2, cutoff=900, vibrato=0.0015),
                      at, pan=-0.1, gain=0.5, reverb=0.25)


def calm_piece(name: str, seed: int, tempo: float, chords: list[tuple], scale: list[int], cycles: int,
               beats_per_chord: float = 8.0, cello_cycle: int = 2) -> np.ndarray:
    beats = len(chords) * beats_per_chord * cycles + beats_per_chord
    piece = Piece(beats * 60 / tempo + 6.0, seed, tempo)
    last = scale[len(scale) // 2]
    for cycle in range(cycles):
        base = cycle * len(chords) * beats_per_chord
        pads(piece, chords, base, beats_per_chord, gain=0.2 if cycle == 0 else 0.24)
        for c, chord in enumerate(chords):
            at = base + c * beats_per_chord
            if cycle == 0 and c < 2:
                continue  # intro: pads only
            last = melody(piece, scale, chord, at, beats_per_chord, last, density=0.55 if cycle % 2 else 0.8)
            if cycle == cello_cycle:
                top = chord[1][1] - 12
                piece.add(strings(piece.rng, top, beats_per_chord * piece.beat * 0.9, attack=1.0, release=1.5,
                                  cutoff=1800, vibrato=0.004), at + 1, pan=-0.35, gain=0.32, reverb=0.4)
    # Ending: the first chord once more, let ring.
    end = cycles * len(chords) * beats_per_chord
    pads(piece, chords[:1], end, beats_per_chord, gain=0.22)
    piece.add(piano(piece.rng, chords[0][1][-1] + 12, 0.45, 6.0), end + 1, pan=0.25, gain=0.5, reverb=0.6)
    return piece.render(fade_in=2.0, fade_out=6.0)


def chord_tone(chord: tuple, degree: str) -> int:
    """Root ("r"), third ("3", the fourth of a sus chord), fifth ("5") or
    octave ("8") of a chord, above its bass note."""
    root = chord[0]
    if degree == "r":
        return root
    if degree == "8":
        return root + 12
    if degree == "5":
        return root + 7
    for note in chord[1]:
        if (note - root) % 12 in (3, 4):
            return root + (note - root) % 12
    return root + 5


def timpani_note(low: int) -> int:
    """The chord's root in the timpani range (A1..A2)."""
    pc = low % 12
    return 36 + pc if 36 + pc <= 45 else 24 + pc


def combat_piece(seed: int, tempo: float, chords: list[tuple], ostinato: list[str], cycles: int) -> np.ndarray:
    """Restrained tension: a low string ostinato in eighths on the chord's
    root, third and fifth, timpani on the strong beats, pads and a
    sustained high line that come and go."""
    beats_per_chord = 4.0
    cycle_beats = len(chords) * beats_per_chord
    beats = cycle_beats * cycles + 4
    piece = Piece(beats * 60 / tempo + 5.0, seed, tempo)
    rng = piece.rng
    for cycle in range(cycles):
        base = cycle * cycle_beats
        layer_pads = cycle >= 1
        layer_line = cycle in (2, cycles - 1) and cycles > 2
        if layer_pads:
            pads(piece, chords, base, beats_per_chord, gain=0.16, bass=False, cutoff=1100)
        for c, (low, voicing) in enumerate(chords):
            at = base + c * beats_per_chord
            # Ostinato in the octave around D2..A2.
            base_note = low - 12 if low > 45 else low
            chord_low = (base_note, voicing)
            for e in range(8):
                note = chord_tone(chord_low, ostinato[e % len(ostinato)])
                accent = 1.0 if e % 4 == 0 else 0.7 if e % 2 == 0 else 0.55
                piece.add(staccato(rng, note, 0.14 * 96 / tempo + 0.04, accent), at + e * 0.5, pan=-0.2, gain=0.45,
                          reverb=0.18)
            piece.add(timpani(rng, timpani_note(low), 0.9), at, pan=0.1, gain=0.6,
                      reverb=0.3)
            if c % 2 == 1:
                piece.add(timpani(rng, timpani_note(low), 0.55), at + 2.5,
                          pan=0.1, gain=0.6, reverb=0.3)
            if cycle >= 1:
                piece.add(low_drum(rng, 0.6), at + 2, pan=0.0, gain=0.4, reverb=0.25)
            if layer_line:
                top = voicing[-1] + 12 if voicing[-1] + 12 <= 81 else voicing[-1]
                piece.add(strings(rng, top, beats_per_chord * piece.beat, attack=0.6, release=0.8, cutoff=3000,
                                  vibrato=0.005), at, pan=0.35, gain=0.18, reverb=0.5)
    end = cycles * cycle_beats
    low = chords[0][0]
    piece.add(timpani(rng, timpani_note(low), 1.0), end, gain=0.7, reverb=0.5)
    piece.add(strings(rng, low, 3.0, attack=0.05, release=1.5, cutoff=1200), end, gain=0.5, reverb=0.4)
    return piece.render(fade_in=0.5, fade_out=3.5)


def result_piece(seed: int, tempo: float, chords: list[tuple], lengths: list[float], scale: list[int],
                 line: list[tuple]) -> np.ndarray:
    """Short closing piece: chords of given lengths (beats) and a fixed
    melody `line` of (beat, note, beats) on piano and cello."""
    beats = sum(lengths) + 2
    piece = Piece(beats * 60 / tempo + 7.0, seed, tempo)
    at = 0.0
    for chord, length in zip(chords, lengths):
        pads(piece, [chord], at, length, gain=0.22)
        at += length
    for beat, note, length in line:
        piece.add(piano(piece.rng, note, 0.55, length * piece.beat + 0.5), beat, pan=0.2, gain=0.6, reverb=0.5)
        piece.add(strings(piece.rng, note - 12, length * piece.beat, attack=0.4, release=1.0, cutoff=1600),
                  beat, pan=-0.3, gain=0.16, reverb=0.4)
    return piece.render(fade_in=0.8, fade_out=6.0)


def menu_01() -> np.ndarray:
    return calm_piece("menu_01", 3001, 64, [D_MIN9, G_ADD9, BB_MAJ7, C_ADD9], D_DORIAN, cycles=4)


def calm_01() -> np.ndarray:
    return calm_piece("calm_01", 3101, 58, [A_MIN9, F_MAJ7, C_MAJ, E_MIN7], A_AEOLIAN, cycles=3, cello_cycle=1)


def calm_02() -> np.ndarray:
    return calm_piece("calm_02", 3102, 60, [E_MIN9, C_MAJ9, G_ADD9_HI, D_SUS2], E_DORIAN, cycles=3, cello_cycle=2)


def combat_01() -> np.ndarray:
    return combat_piece(3201, 96, [D_MIN, D_MIN, BB_MAJ, C_ADD9, D_MIN, G_MIN, BB_MAJ, A_SUS4], ["r", "r", "5", "r", "3", "r", "5", "8"], 4)


def combat_02() -> np.ndarray:
    return combat_piece(3202, 88, [E_MIN, F_MAJ, E_MIN, D_MAJ, C_MAJ, D_MAJ, B_SUS4, B_MAJ], ["r", "r", "3", "r", "5", "r", "3", "r"], 4)


def victory_01() -> np.ndarray:
    chords = [D_MIN7, BB_MAJ7, F_MAJ_A, G_MIN7, C_SUS4, C7, F_ADD9]
    lengths = [4, 4, 4, 4, 2, 2, 10]
    line = [(0, 69, 2), (2, 72, 2), (4, 74, 3), (7, 72, 1), (8, 69, 4), (12, 70, 2), (14, 74, 2), (16, 72, 4),
            (20, 70, 2), (22, 67, 2), (24, 69, 8)]
    return result_piece(3301, 70, chords, lengths, F_MAJOR, line)


def defeat_01() -> np.ndarray:
    chords = [D_MIN, BB_MAJ, G_MIN, A_SUS4, A_MAJ, D_MIN]
    lengths = [4, 4, 4, 2, 2, 10]
    line = [(0, 65, 3), (3, 64, 1), (4, 62, 4), (8, 62, 2), (10, 58, 2), (12, 57, 2), (14, 61, 2), (16, 62, 8)]
    return result_piece(3302, 54, chords, lengths, D_MINOR, line)


PIECES = {f.__name__: f for f in [menu_01, calm_01, calm_02, combat_01, combat_02, victory_01, defeat_01]}


# --- Ambience ----------------------------------------------------------------------


def wind(rng, sr: int, seconds: float, band, gust_rate: float, gust_floor: float, howl: list[tuple] | None,
         hiss: float = 0.0) -> np.ndarray:
    """Stereo wind bed: pink noise in `band`, gusts as a slow periodic
    curve, optional howling resonances and sand hiss."""
    n = secs(seconds, sr)
    gust = smooth_noise(rng, n, sr, gust_rate)
    sides = []
    for side in range(2):
        g = np.roll(gust, secs(0.4 * side, sr))
        body = filt(rng.standard_normal(n), lambda f: band(f) * pink(f), sr)
        body /= np.std(body) + 1e-12
        x = body * (gust_floor + (1 - gust_floor) * g)
        if howl:
            for fc, q, amount in howl:
                tone = filt(rng.standard_normal(n), lambda f, fc=fc, q=q: peak(fc, q)(f), sr)
                tone /= np.std(tone) + 1e-12
                x += amount * tone * g ** 2
        if hiss:
            h = filt(rng.standard_normal(n), bp(2500, 9000), sr)
            h /= np.std(h) + 1e-12
            x += hiss * h * g ** 1.5
        sides.append(x)
    return np.stack(sides, axis=1)


def wind_forest(rng) -> np.ndarray:
    return wind(rng, WIND_SR, 20.0, bp(80, 900, 1), 0.12, 0.35, None, hiss=0.08)


def wind_desert(rng) -> np.ndarray:
    return wind(rng, WIND_SR, 20.0, bp(150, 2000, 1), 0.2, 0.25, [(620, 6, 0.12)], hiss=0.35)


def wind_winter(rng) -> np.ndarray:
    return wind(rng, WIND_SR, 20.0, bp(60, 1200, 1), 0.15, 0.3, [(420, 9, 0.35), (690, 11, 0.25), (980, 14, 0.12)], hiss=0.12)


def chirp_train(rng, sr: int, n: int, carrier: float, pulse_hz: float, pulses: int, chirp_rate: float, level: float,
                out: np.ndarray) -> None:
    """One cricket: groups of `pulses` short sine pulses, wrapped into the loop."""
    pulse = secs(0.6 / pulse_hz, sr)
    tp = np.arange(pulse) / sr
    shape = np.sin(np.pi * tp / tp[-1]) ** 2
    t = rng.uniform(0, n / sr)
    for _ in range(int(chirp_rate * n / sr)):
        f = carrier * rng.uniform(0.99, 1.01)
        for p in range(pulses):
            x = np.sin(2 * np.pi * f * tp) * shape * level * rng.uniform(0.8, 1.0)
            add_wrapped(out, x, secs(t + p / pulse_hz, sr))
        t += 1.0 / chirp_rate * rng.uniform(0.9, 1.1)


def insects_meadow(rng) -> np.ndarray:
    n = secs(20.0, BED_SR)
    sides = [np.zeros(n), np.zeros(n)]
    for i in range(7):
        side = sides[i % 2]
        chirp_train(rng, BED_SR, n, rng.uniform(3800, 5200), rng.uniform(28, 40), int(rng.integers(2, 5)),
                    rng.uniform(1.5, 3.0), rng.uniform(0.3, 1.0) * (0.5 if i >= 4 else 1.0), side)
    for side in sides:
        side += 0.04 * filt(rng.standard_normal(n), bp(2500, 7000), BED_SR)
    return np.stack(sides, axis=1)


def insects_desert(rng) -> np.ndarray:
    """Cicadas: a buzzing band, amplitude-modulated fast, in slow swells."""
    n = secs(20.0, BED_SR)
    t = np.arange(n) / BED_SR
    sides = []
    for side in range(2):
        x = np.zeros(n)
        for _ in range(3):
            band = filt(rng.standard_normal(n), bp(rng.uniform(4200, 5200), rng.uniform(6500, 7800), 3), BED_SR)
            band /= np.std(band) + 1e-12
            rate = round(rng.uniform(90, 140) * 20.0) / 20.0  # whole cycles in the loop
            buzz = 0.5 + 0.5 * np.sin(2 * np.pi * rate * t)
            swell = smooth_noise(rng, n, BED_SR, 0.25) ** 2
            x += band * buzz * (0.15 + 0.85 * swell)
        sides.append(x)
    return np.stack(sides, axis=1)


def leaves(rng) -> np.ndarray:
    """Rustling foliage (mono, placed at the wood's center): dense tiny
    noise grains whose density follows gusts."""
    n = secs(20.0, BED_SR)
    gust = smooth_noise(rng, n, BED_SR, 0.2)
    out = np.zeros(n)
    grain_n = secs(0.012, BED_SR)
    tg = np.arange(grain_n) / BED_SR
    count = int(20.0 * 900)
    starts = rng.uniform(0, n, count).astype(np.int64)
    keep = rng.random(count) < 0.15 + 0.85 * gust[starts]
    for s in starts[keep]:
        g = rng.standard_normal(grain_n) * np.exp(-tg / rng.uniform(0.001, 0.004))
        add_wrapped(out, g * rng.uniform(0.1, 1.0), int(s))
    out = filt(out, bp(1200, 9000), BED_SR)
    bed = filt(rng.standard_normal(n), bp(600, 5000), BED_SR)
    out = out / (np.std(out) + 1e-12) + 0.3 * bed / (np.std(bed) + 1e-12) * (0.3 + 0.7 * gust)
    return out


def water(rng) -> np.ndarray:
    """A river (mono, placed at the water's center): flowing noise and
    many small bubbles with a rising pitch."""
    n = secs(20.0, BED_SR)
    out = np.zeros(n)
    count = int(20.0 * 60)
    for _ in range(count):
        length = rng.uniform(0.01, 0.04)
        m = secs(length * 4, BED_SR)
        tb = np.arange(m) / BED_SR
        f0 = rng.uniform(350, 1600)
        freq = f0 * (1 + 0.8 * tb / (length * 4))
        x = np.sin(2 * np.pi * np.cumsum(freq) / BED_SR) * np.exp(-tb / length) * rng.uniform(0.1, 1.0)
        add_wrapped(out, x, int(rng.integers(n)))
    flow = filt(rng.standard_normal(n), lambda f: bp(120, 3000, 1)(f) * pink(f), BED_SR)
    swell = smooth_noise(rng, n, BED_SR, 0.3)
    out = out / (np.std(out) + 1e-12) * 0.6 + flow / (np.std(flow) + 1e-12) * (0.7 + 0.3 * swell)
    return out


def rain(rng) -> np.ndarray:
    n = secs(20.0, BED_SR)
    sides = []
    for side in range(2):
        out = np.zeros(n)
        drop_n = secs(0.006, BED_SR)
        td = np.arange(drop_n) / BED_SR
        for s in rng.integers(0, n, int(20.0 * 700)):
            add_wrapped(out, rng.standard_normal(drop_n) * np.exp(-td / 0.0012) * rng.uniform(0.05, 1.0), int(s))
        out = filt(out, bp(1500, 10000), BED_SR)
        bed = filt(rng.standard_normal(n), lambda f: bp(400, 8000, 1)(f) * pink(f), BED_SR)
        sides.append(out / (np.std(out) + 1e-12) * 0.6 + bed / (np.std(bed) + 1e-12))
    return np.stack(sides, axis=1)


def bird_note(rng, sr: int, f0: float, f1: float, length: float, harmonic: float = 0.15, trill: float = 0.0) -> np.ndarray:
    n = secs(length, sr)
    t = np.arange(n) / sr
    freq = f0 + (f1 - f0) * (t / length) ** rng.uniform(0.6, 1.6)
    if trill:
        freq *= 1 + 0.06 * np.sin(2 * np.pi * trill * t)
    phase = 2 * np.pi * np.cumsum(freq) / sr
    shape = np.sin(np.pi * t / length) ** 1.5
    return (np.sin(phase) + harmonic * np.sin(2 * phase)) * shape


def songbird(rng) -> np.ndarray:
    """A phrase of 4-12 whistled notes: slides, trills and repeats."""
    sr = SHOT_SR
    out = np.zeros(secs(3.0, sr))
    t = 0.0
    base = rng.uniform(2500, 4500)
    kind = rng.integers(3)
    for i in range(int(rng.integers(4, 12))):
        if kind == 0:  # descending whistles
            f0 = base * rng.uniform(1.0, 1.4)
            note = bird_note(rng, sr, f0, f0 * rng.uniform(0.6, 0.85), rng.uniform(0.06, 0.14))
        elif kind == 1:  # fast trill of the same note
            note = bird_note(rng, sr, base, base * 1.15, 0.035, trill=0)
        else:  # warble
            f0 = base * rng.uniform(0.8, 1.5)
            note = bird_note(rng, sr, f0, f0 * rng.uniform(0.8, 1.3), rng.uniform(0.05, 0.12), trill=rng.uniform(30, 60))
        add_at(out, note * rng.uniform(0.5, 1.0), secs(t, sr))
        t += len(note) / sr + (0.02 if kind == 1 else rng.uniform(0.03, 0.12))
        if t > 2.6:
            break
    return out


def crow(rng) -> np.ndarray:
    """Two or three harsh caws (winter)."""
    sr = SHOT_SR
    out = np.zeros(secs(2.5, sr))
    t = 0.0
    for _ in range(int(rng.integers(2, 4))):
        length = rng.uniform(0.25, 0.4)
        n = secs(length, sr)
        tc = np.arange(n) / sr
        f = rng.uniform(480, 620) * (1 - 0.15 * tc / length)
        phase = 2 * np.pi * np.cumsum(f) / sr
        x = sum((1 / k) * np.sin(k * phase) for k in range(1, 14))
        x = x * (1 + 0.6 * rng.standard_normal(n) * 0.3)
        x = filt(np.concatenate([x, np.zeros(n)]), lambda fr: bp(700, 2400)(fr) + 0.5 * peak(1400, 4)(fr), sr)[:n]
        x *= np.sin(np.pi * tc / length) ** 0.7
        add_at(out, x, secs(t, sr))
        t += length + rng.uniform(0.18, 0.35)
    return out


def tit_call(rng) -> np.ndarray:
    """Thin high "see-see" calls (winter)."""
    sr = SHOT_SR
    out = np.zeros(secs(1.6, sr))
    t = 0.0
    f = rng.uniform(6500, 8000)
    for _ in range(int(rng.integers(2, 5))):
        note = bird_note(rng, sr, f, f * 0.93, rng.uniform(0.09, 0.16), harmonic=0.05)
        add_at(out, note, secs(t, sr))
        t += 0.25
    return out


def lark(rng) -> np.ndarray:
    """Dry short trills (desert)."""
    sr = SHOT_SR
    out = np.zeros(secs(2.0, sr))
    t = 0.0
    f = rng.uniform(3000, 4200)
    for i in range(int(rng.integers(6, 14))):
        note = bird_note(rng, sr, f * (1 - 0.02 * i), f * (0.9 - 0.02 * i), 0.04, harmonic=0.25)
        add_at(out, note * (1 - 0.04 * i), secs(t, sr))
        t += 0.06
    return out


def rumble(rng) -> np.ndarray:
    """Distant thunder or far artillery: low noise in a few swells."""
    sr = SHOT_SR
    n = secs(7.0, sr)
    t = np.arange(n) / sr
    x = np.zeros(n)
    for _ in range(int(rng.integers(2, 5))):
        start = rng.uniform(0.0, 2.5)
        e = np.clip((t - start) / rng.uniform(0.05, 0.4), 0, 1) * np.exp(-np.maximum(t - start, 0) / rng.uniform(0.6, 1.6))
        x += e * rng.uniform(0.4, 1.0)
    body = filt(rng.standard_normal(n), lambda f: bp(25, 160, 2)(f) * pink(f), sr)
    body /= np.std(body) + 1e-12
    out = body * x
    return filt(np.concatenate([out, np.zeros(n)]), lp(220, 2), sr)[:n]


# name -> (recipe, variants, loop, sample rate)
AMBIENCE = {
    "wind_forest": (wind_forest, 1, True, WIND_SR),
    "wind_desert": (wind_desert, 1, True, WIND_SR),
    "wind_winter": (wind_winter, 1, True, WIND_SR),
    "insects_meadow": (insects_meadow, 1, True, BED_SR),
    "insects_desert": (insects_desert, 1, True, BED_SR),
    "leaves": (leaves, 1, True, BED_SR),
    "water": (water, 1, True, BED_SR),
    "rain": (rain, 1, True, BED_SR),
    "birds_forest": (songbird, 8, False, SHOT_SR),
    "birds_desert": (lark, 3, False, SHOT_SR),
    "birds_winter": (lambda rng: crow(rng) if rng.random() < 0.5 else tit_call(rng), 4, False, SHOT_SR),
    "rumble": (rumble, 3, False, SHOT_SR),
}


# --- Output -------------------------------------------------------------------------


def normalize(x: np.ndarray, loop: bool, sr: int) -> np.ndarray:
    x = np.asarray(x, dtype=np.float64)
    if loop:
        x = x - np.mean(x, axis=0)
    else:
        mono = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), axis=1)
        loud = np.nonzero(mono > np.max(mono) * 10 ** (-60 / 20))[0]
        if len(loud):
            x = x[loud[0]: loud[-1] + 1]
        fade = min(secs(0.03, sr), len(x))
        ramp = np.linspace(1, 0, fade)
        x[-fade:] *= ramp if x.ndim == 1 else ramp[:, None]
    return x * (10 ** (PEAK_DBFS / 20) / (np.max(np.abs(x)) + 1e-12))


def write_wav(path: Path, x: np.ndarray, sr: int, loop: bool) -> None:
    channels = 1 if x.ndim == 1 else x.shape[1]
    frames = len(x)
    pcm = np.clip(np.round(x * 32767), -32768, 32767).astype("<i2").tobytes()
    chunks = b"fmt " + struct.pack("<IHHIIHH", 16, 1, channels, sr, sr * 2 * channels, 2 * channels, 16)
    if loop:
        smpl = struct.pack("<IIIIIIIII", 0, 0, int(1e9 / sr), 60, 0, 0, 0, 1, 0)
        smpl += struct.pack("<IIIIII", 0, 0, 0, frames - 1, 0, 0)
        chunks += b"smpl" + struct.pack("<I", len(smpl)) + smpl
    chunks += b"data" + struct.pack("<I", len(pcm)) + pcm
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"RIFF" + struct.pack("<I", 4 + len(chunks)) + b"WAVE" + chunks)


def encode_mp3(wav: Path, mp3: Path) -> None:
    mp3.parent.mkdir(parents=True, exist_ok=True)
    lame = shutil.which("lame")
    if lame:
        subprocess.run([lame, "--quiet", "-V", LAME_QUALITY, "--noreplaygain", str(wav), str(mp3)], check=True)
        return
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg:
        subprocess.run([ffmpeg, "-loglevel", "error", "-y", "-i", str(wav), "-c:a", "libmp3lame", "-q:a", LAME_QUALITY,
                        str(mp3)], check=True)
        return
    raise SystemExit("needs `lame` or ffmpeg with libmp3lame to encode the music")


def external_piece(name: str) -> Path | None:
    if not EXTERNAL_MUSIC.is_dir():
        return None
    for path in sorted(EXTERNAL_MUSIC.glob(name + ".*")):
        if path.suffix.lower() in {".wav", ".mp3", ".ogg", ".flac"}:
            return path
    return None


def make_piece(name: str, extra_dir: Path | None) -> None:
    out = MUSIC_DIR / f"{name}.mp3"
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / f"{name}.wav"
        source = external_piece(name)
        if source is not None:
            ffmpeg = shutil.which("ffmpeg")
            if ffmpeg is None:
                raise SystemExit(f"{source}: needs ffmpeg to convert")
            subprocess.run([ffmpeg, "-loglevel", "error", "-y", "-i", str(source), "-ac", "2", "-ar", str(MUSIC_SR),
                            "-c:a", "pcm_s16le", str(wav)], check=True)
            origin = " (external)"
        else:
            write_wav(wav, PIECES[name](), MUSIC_SR, False)
            origin = ""
        encode_mp3(wav, out)
        if extra_dir is not None:
            extra_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy(wav, extra_dir / wav.name)
    print(f"music    {name:16s} {out.stat().st_size / 1e6:5.2f} MB{origin}")


def read_audio(path: Path, sr: int, channels: int) -> np.ndarray:
    """Float samples of any file at `sr` with `channels` (ffmpeg)."""
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        raise SystemExit(f"{path}: needs ffmpeg to convert")
    raw = subprocess.run([ffmpeg, "-v", "error", "-i", str(path), "-ac", str(channels), "-ar", str(sr), "-f", "f32le",
                          "-"], capture_output=True, check=True).stdout
    x = np.frombuffer(raw, dtype=np.float32).astype(np.float64)
    return x if channels == 1 else x.reshape(-1, channels)


def private_files(rel: str) -> list[Path]:
    folder = PRIVATE_DIR / rel
    return sorted(p for p in folder.glob("*") if p.suffix.lower() in SOURCE_EXTENSIONS) if folder.is_dir() else []


def crossfade_loop(x: np.ndarray, sr: int) -> np.ndarray:
    """Folds the last `LOOP_CROSSFADE` seconds into the start (equal power),
    so the end runs into the start without a seam."""
    n = min(secs(LOOP_CROSSFADE, sr), len(x) // 4)
    ramp = np.linspace(0, np.pi / 2, n)
    if x.ndim == 2:
        ramp = ramp[:, None]
    loop = x[:len(x) - n].copy()
    loop[:n] = loop[:n] * np.sin(ramp) + x[len(x) - n:] * np.cos(ramp)
    return loop


def make_private_layer(name: str, extra_dir: Path | None) -> int:
    _, _, loop, sr = AMBIENCE[name]
    files = private_files(f"ambience/{name}")
    for path in files:
        x = read_audio(path, sr, 1 if name in MONO_LAYERS or not loop else 2)
        x = normalize(crossfade_loop(x, sr) if loop else x, loop, sr)
        stem = PRIVATE_PREFIX + path.stem
        write_wav(AMBIENCE_DIR / name / f"{stem}.wav", x, sr, loop)
        if extra_dir is not None:
            write_wav(extra_dir / f"{stem}.wav", x, sr, loop)
    return len(files)


def make_private_pieces(argv: list[str], extra_dir: Path | None) -> None:
    moods = sorted({n.rsplit("_", 1)[0] for n in PIECES})
    chosen = [m for m in moods if not argv or any(m.startswith(a) or a.startswith(m) for a in argv)]
    for old in MUSIC_DIR.glob(PRIVATE_PREFIX + "*.mp3"):
        if old.stem[len(PRIVATE_PREFIX):].rsplit("_", 1)[0] in chosen:
            old.unlink()
    for path in private_files("music"):
        mood = path.stem.rsplit("_", 1)[0]
        if mood not in moods:
            print(f"music    {path.name}: unknown mood '{mood}' (name it <mood>_<name>, moods {moods}), skipped")
            continue
        if mood not in chosen:
            continue
        x = read_audio(path, MUSIC_SR, 2)
        fade = secs(PIECE_FADE, MUSIC_SR)
        x[-fade:] *= np.cos(np.linspace(0, np.pi / 2, fade))[:, None] ** 2
        rms = np.sqrt(np.mean(x ** 2)) + 1e-12
        gain = min(10 ** (MUSIC_RMS_DBFS / 20) / rms, 10 ** (PEAK_DBFS / 20) / (np.max(np.abs(x)) + 1e-12))
        name = PRIVATE_PREFIX + path.stem
        out = MUSIC_DIR / f"{name}.mp3"
        with tempfile.TemporaryDirectory() as tmp:
            wav = Path(tmp) / f"{name}.wav"
            write_wav(wav, x * gain, MUSIC_SR, False)
            encode_mp3(wav, out)
            if extra_dir is not None:
                extra_dir.mkdir(parents=True, exist_ok=True)
                shutil.copy(wav, extra_dir / wav.name)
        print(f"music    {name:16s} {out.stat().st_size / 1e6:5.2f} MB (private, {20 * np.log10(gain):+.1f} dB)")


def make_layer(name: str, extra_dir: Path | None) -> None:
    recipe, variants, loop, sr = AMBIENCE[name]
    out_dir = AMBIENCE_DIR / name
    if out_dir.is_dir():
        for old in out_dir.glob("*.wav"):
            old.unlink()
    for v in range(variants):
        rng = np.random.default_rng(sum(ord(c) * 31 ** i for i, c in enumerate(name)) % 2 ** 32 + v * 7919)
        x = normalize(recipe(rng), loop, sr)
        stem = f"{name}_{v + 1:02d}"
        write_wav(out_dir / f"{stem}.wav", x, sr, loop)
        if extra_dir is not None:
            write_wav(extra_dir / f"{stem}.wav", x, sr, loop)
    private = make_private_layer(name, extra_dir)
    print(f"ambience {name:16s} {variants} file(s){' (loop)' if loop else ''}{f', {private} private' if private else ''}")


def main(argv: list[str]) -> int:
    extra_dir = None
    if "--wav-dir" in argv:
        i = argv.index("--wav-dir")
        extra_dir = Path(argv[i + 1])
        argv = argv[:i] + argv[i + 2:]
    names = list(PIECES) + list(AMBIENCE)
    wanted = [n for n in names if not argv or any(n.startswith(a) for a in argv)]
    if not wanted:
        raise SystemExit(f"nothing matches {argv}; pieces: {list(PIECES)}, ambience: {list(AMBIENCE)}")
    for name in wanted:
        if name in PIECES:
            make_piece(name, extra_dir)
        else:
            make_layer(name, extra_dir)
    if any(n in PIECES for n in wanted):
        make_private_pieces(argv, extra_dir)
    for stale in list(AMBIENCE_DIR.glob("*/*.wav.import")) + list(MUSIC_DIR.glob("*.mp3.import")):
        if not stale.with_suffix("").exists():
            stale.unlink()
    print("run `just import` next")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
