#!/usr/bin/env python3
"""Synthesizes the game's sounds (prompt 029) and runs every sound file
through one pipeline.

    python3 tools/audio/synth.py            # all classes (`just audio`)
    python3 tools/audio/synth.py shot_mg    # only these classes
    python3 tools/audio/synth.py --wav-dir DIR   # also write into DIR
    python3 tools/audio/synth.py --private  # only what the private repository adds (release builds)

Reads the class list from `godot/assets/audio/sounds.json` and writes
`godot/assets/audio/<class>/<class>_NN.wav` (16-bit mono, 44.1 kHz).

Every sound is made from noise, sine partials, envelopes and filters with
numpy (no samples, no downloads): a recipe per class below, `variants`
files per class with their own random seed and small parameter changes
against repetition. The output is deterministic for a numpy version.

Recorded or generated files of any other origin go to
`assets-src/audio/<class>/` (wav directly; ogg, mp3, flac through ffmpeg if
installed). They take the same way: mono, 44.1 kHz, silence trimmed, short
fades, peak normalized, written as `<class>/ext_<name>.wav`; a class with
such files gets no synthesized ones. Their source belongs in
`docs/SOURCES.md`.

Loops (classes with `"loop": true`) are built periodic: every partial has a
whole number of cycles in the loop and noise is shaped in the frequency
domain of the whole loop, so the end runs into the start without a seam.
The file carries a `smpl` loop chunk, which Godot's importer reads.

Keyed folders (prompt 053) hold files that only exist as recordings, with
no synthesized fallback of their own:

    assets-src/audio/weapon/<slug>/*        one weapon type (slug: catalog weapon
                                            name without parentheses, lower case, every run of
                                            other characters `_`: "MG 42" -> mg_42)
    assets-src/audio/voice/<faction>/<callout>/*   battle voices (de, su, us;
                                            callouts: see docs/ASSETS.md)

They are prepared the same way into `godot/assets/audio/weapon/...` and
`.../voice/...` on every full run (or with the argument `weapon` / `voice`).
A weapon without files plays its synthesized class; a callout without
files stays silent.

Spoken announcements (prompt 244, "We are under attack!", "Enemy aircraft
sighted!") are keyed the same
way, `godot/assets/audio/announce/<faction>/<message>/`, with the argument
`announce`. Besides `assets-src/audio/announce/<faction>/<message>/*` they
come straight from the output of `tools/voice_lines.py` in the private
repository, `elevenlabs/<english line>/<de|en|ru>/*.mp3` (`ANNOUNCE` names
the folder of each message, `ANNOUNCE_FACTIONS` the faction of each
language), and are brought to the same loudness (`ANNOUNCE_RMS_DBFS`). A
faction without files plays the message's alarm sound (`battle_sound.gd`).

    python3 tools/audio/synth.py --weapons  # list the weapon slugs of the catalog

Private sounds (prompt 069): files whose license forbids handing them on
as single files (ElevenLabs output, Pixabay, Sonniss) live in a private
repository with the layout of `assets-src/audio/`, cloned next to this one
as `../mud-and-steel-sfx` (or where `MUD_AND_STEEL_SFX` points). When it is
there, its files are prepared too, as `priv_<name>.wav` beside the public
files of the class or weapon. The game plays only the `priv_` files of a
folder that has some (`sound_library.gd`); they are gitignored, so a clone
without the private repository plays the public files.

Requires numpy. Afterwards run `just import` so Godot imports new files.
"""

from __future__ import annotations

import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import wave
from pathlib import Path

import numpy as np

SR = 44100
ROOT = Path(__file__).resolve().parents[2]
AUDIO_DIR = ROOT / "godot" / "assets" / "audio"
MANIFEST = AUDIO_DIR / "sounds.json"
EXTERNAL_DIR = ROOT / "assets-src" / "audio"
PRIVATE_DIR = Path(os.environ.get("MUD_AND_STEEL_SFX", ROOT.parent / "mud-and-steel-sfx"))
CATALOG_DIR = ROOT / "data" / "catalog"
KEYED = ["weapon", "voice", "announce"]
# Spoken announcements (prompt 244): message -> folder of its voice lines in
# the private repository (`elevenlabs/<folder>/<language>/`, written by its
# `tools/voice_lines.py`). A further message is one more entry here and in
# `battle_sound.gd ANNOUNCE_FALLBACK`.
ANNOUNCE = {
    "under_attack": "we are under attack",
    "enemy_aircraft": "enemy aircraft spotted",
    "retreat": "fall back",
}
# Language of a voice line -> the faction that speaks it (`uk` falls back to
# `us` in the game, `battle_voices.gd VOICE_FALLBACK`).
ANNOUNCE_FACTIONS = {"de": "de", "en": "us", "ru": "su"}
# Loudness of every announcement: RMS of its loud 50 ms blocks (those within
# 20 dB of the loudest), so all languages and speakers are equally loud.
ANNOUNCE_RMS_DBFS = -19.0
SOURCE_EXTENSIONS = {".wav", ".ogg", ".mp3", ".flac"}
PEAK_DBFS = -1.0


# --- Building blocks ---------------------------------------------------------


class Ctx:
    """Random source and parameter jitter of one variant."""

    def __init__(self, seed: int, variant: int):
        self.rng = np.random.default_rng(seed)
        self.variant = variant

    def jit(self, value: float, share: float = 0.08) -> float:
        """`value` changed by up to +- `share`."""
        return value * (1.0 + self.rng.uniform(-share, share))

    def noise(self, n: int) -> np.ndarray:
        return self.rng.standard_normal(n)


def samples(seconds: float) -> int:
    return int(round(seconds * SR))


def times(n: int) -> np.ndarray:
    return np.arange(n) / SR


def filt(x: np.ndarray, gain) -> np.ndarray:
    """Filters `x` in the frequency domain (circular: pad one-shots or
    filter noise before the envelope)."""
    spectrum = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1.0 / SR)
    return np.fft.irfft(spectrum * gain(f), len(x))


def lp(fc: float, order: int = 2):
    return lambda f: 1.0 / np.sqrt(1.0 + (f / fc) ** (2 * order))


def hp(fc: float, order: int = 2):
    return lambda f: 1.0 / np.sqrt(1.0 + (fc / np.maximum(f, 1e-3)) ** (2 * order))


def bp(lo: float, hi: float, order: int = 2):
    low, high = hp(lo, order), lp(hi, order)
    return lambda f: low(f) * high(f)


def peak(fc: float, q: float):
    """Narrow resonance around `fc` (bandwidth fc / q)."""
    return lambda f: 1.0 / (1.0 + ((f - fc) / (fc / q)) ** 2)


def env(n: int, decay: float, attack: float = 0.0005, delay: float = 0.0) -> np.ndarray:
    """Exponential decay with time constant `decay` (s), linear attack,
    starting after `delay` seconds."""
    t = times(n) - delay
    e = np.exp(-np.maximum(t, 0.0) / decay)
    if attack > 0.0:
        e *= np.clip(t / attack, 0.0, 1.0)
    else:
        e *= t >= 0.0
    return e


def sweep(n: int, f_start: float, f_end: float, glide: float, delay: float = 0.0) -> np.ndarray:
    """Sine whose frequency glides exponentially from `f_start` to `f_end`
    with time constant `glide`."""
    t = np.maximum(times(n) - delay, 0.0)
    freq = f_end + (f_start - f_end) * np.exp(-t / glide)
    phase = 2.0 * np.pi * np.cumsum(freq) / SR
    return np.sin(phase)


def partials(n: int, freqs, decays, amps, delay: float = 0.0, ctx: Ctx | None = None) -> np.ndarray:
    """Sum of decaying sines (bells, clangs, pings)."""
    t = times(n) - delay
    out = np.zeros(n)
    for f, d, a in zip(freqs, decays, amps):
        phase = ctx.rng.uniform(0, 2 * np.pi) if ctx else 0.0
        out += a * np.sin(2 * np.pi * f * t + phase) * np.exp(-np.maximum(t, 0) / d) * (t >= 0)
    return out


def grains(ctx: Ctx, n: int, start: float, end: float, rate: float, decay: float, band, size: float = 0.004) -> np.ndarray:
    """Many short noise bursts (debris, dirt, splinters, crackle): `rate`
    per second at the start, thinning out linearly to `end`."""
    out = np.zeros(n)
    grain_n = samples(size * 6)
    t = start
    while t < end:
        density = rate * max(0.05, 1.0 - (t - start) / max(end - start, 1e-3))
        t += ctx.rng.exponential(1.0 / density)
        i = samples(t)
        if i + grain_n >= n:
            break
        g = ctx.noise(grain_n) * np.exp(-times(grain_n) / size)
        out[i:i + grain_n] += g * ctx.rng.uniform(0.2, 1.0) * np.exp(-(t - start) / decay)
    return filt(out, band)


def reverb(x: np.ndarray, ctx: Ctx, decay: float, wet: float, tone: float = 2500.0, predelay: float = 0.02) -> np.ndarray:
    """Outdoor tail: convolution with decaying, darkened noise."""
    ir_n = samples(decay * 5 + predelay)
    ir = ctx.noise(ir_n) * env(ir_n, decay, attack=0.01, delay=predelay)
    ir = filt(np.concatenate([ir, np.zeros(ir_n)]), lp(tone))[:ir_n]
    ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
    size = len(x) + ir_n
    tail = np.fft.irfft(np.fft.rfft(x, size) * np.fft.rfft(ir, size), size)
    out = np.concatenate([x, np.zeros(ir_n)]) + wet * tail * 3.0
    return out


def fit(x: np.ndarray, seconds: float) -> np.ndarray:
    n = samples(seconds)
    if len(x) >= n:
        return x[:n]
    return np.concatenate([x, np.zeros(n - len(x))])


def tone(n: int, freq: float, harmonics, delay: float = 0.0, length: float = 1.0, attack: float = 0.01,
         release: float = 0.08, vibrato: float = 0.0) -> np.ndarray:
    """A held note with given harmonic amplitudes (brass, chimes)."""
    t = times(n) - delay
    active = (t >= 0) & (t < length + release * 5)
    vib = 1.0 + vibrato * np.sin(2 * np.pi * 5.5 * np.maximum(t, 0))
    phase = 2 * np.pi * freq * np.cumsum(vib * (t >= 0)) / SR
    out = np.zeros(n)
    for k, a in enumerate(harmonics, start=1):
        out += a * np.sin(k * phase)
    shape = np.clip(t / attack, 0, 1) * np.where(t < length, 1.0, np.exp(-(t - length) / release))
    return out * shape * active


# --- Recipes: weapons ----------------------------------------------------------


def gunshot(ctx: Ctx, seconds: float, crack_hp: float, body_band, body_decay: float, boom: tuple | None,
            tail_decay: float, tail_wet: float, tail_tone: float) -> np.ndarray:
    n = samples(seconds)
    crack = filt(ctx.noise(n), hp(ctx.jit(crack_hp))) * env(n, ctx.jit(0.004, 0.2), attack=0.0002)
    body = filt(ctx.noise(n), body_band) * env(n, ctx.jit(body_decay), attack=0.001)
    x = 0.9 * crack + 1.2 * body
    if boom is not None:
        f0, f1, glide, decay, amp = boom
        x += amp * sweep(n, ctx.jit(f0), ctx.jit(f1), glide) * env(n, ctx.jit(decay), attack=0.002)
    x = reverb(x, ctx, ctx.jit(tail_decay), tail_wet, tail_tone)
    return fit(x, seconds)


def shot_rifle(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 1.2, 1800, bp(250, 3500), 0.02, (140, 70, 0.02, 0.03, 0.5), 0.3, 0.35, 2600)


def shot_mg(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 0.7, 1500, bp(200, 3000), 0.018, (130, 65, 0.02, 0.03, 0.55), 0.18, 0.25, 2200)


def shot_cannon_small(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 1.5, 1200, bp(150, 2500), 0.04, (100, 45, 0.04, 0.07, 0.8), 0.5, 0.45, 2000)


def shot_cannon_medium(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 2.6, 900, bp(80, 1800), 0.08, (70, 32, 0.06, 0.16, 1.1), 0.9, 0.55, 1500)


def shot_cannon_large(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 3.4, 700, bp(60, 1300), 0.12, (55, 26, 0.08, 0.26, 1.3), 1.3, 0.6, 1200)


def shot_howitzer(ctx: Ctx) -> np.ndarray:
    return gunshot(ctx, 4.2, 600, bp(40, 1000), 0.16, (46, 21, 0.1, 0.36, 1.5), 1.8, 0.65, 900)


def shot_mortar(ctx: Ctx) -> np.ndarray:
    # A hollow "thoonk": a pipe resonance, little crack.
    n = samples(1.6)
    pop = filt(ctx.noise(n), bp(150, 900)) * env(n, ctx.jit(0.04), attack=0.002)
    tube = partials(n, [ctx.jit(180), ctx.jit(410), ctx.jit(690)], [0.07, 0.04, 0.025], [1.0, 0.4, 0.2], ctx=ctx)
    thump = sweep(n, 160, 80, 0.03) * env(n, 0.06, attack=0.002)
    x = 1.1 * pop + 0.7 * tube + 0.6 * thump
    return fit(reverb(x, ctx, 0.6, 0.45, 1600), 1.6)


def shot_rocket(ctx: Ctx) -> np.ndarray:
    # Launch pop, then the rocket motor's roar flying off.
    n = samples(1.6)
    t = times(n)
    pop = filt(ctx.noise(n), bp(200, 3000)) * env(n, ctx.jit(0.015), attack=0.0005)
    burn = ctx.jit(0.9, 0.1)
    motor_env = np.clip(t / 0.03, 0, 1) * np.exp(-t / (burn * 0.5)) * (t < burn)
    motor = filt(ctx.noise(n), bp(ctx.jit(500), ctx.jit(4000))) * motor_env
    x = 1.0 * pop + 0.8 * motor
    return fit(reverb(x, ctx, 0.5, 0.35, 2200), 1.6)


# --- Recipes: impacts ------------------------------------------------------------


def explosion_core(ctx: Ctx, seconds: float, body_lp: float, body_decay: float, boom: tuple, debris: float,
                   tail_decay: float, tail_wet: float) -> np.ndarray:
    n = samples(seconds)
    crack = filt(ctx.noise(n), hp(1500)) * env(n, 0.006, attack=0.0003)
    body = filt(ctx.noise(n), lp(ctx.jit(body_lp))) * env(n, ctx.jit(body_decay), attack=0.003)
    f0, f1, glide, decay, amp = boom
    sub = amp * sweep(n, ctx.jit(f0), ctx.jit(f1), glide) * env(n, ctx.jit(decay), attack=0.004)
    rain = grains(ctx, n, 0.25, 0.25 + debris, 70, debris * 0.6, bp(400, 5000), 0.003)
    x = 0.5 * crack + 1.4 * body + sub + 0.6 * rain
    return fit(reverb(x, ctx, ctx.jit(tail_decay), tail_wet, 1300), seconds)


def hit_shell(ctx: Ctx) -> np.ndarray:
    return explosion_core(ctx, 3.0, 1500, 0.12, (60, 28, 0.08, 0.2, 1.2), 1.0, 1.0, 0.55)


def hit_shell_small(ctx: Ctx) -> np.ndarray:
    return explosion_core(ctx, 1.4, 2600, 0.04, (110, 50, 0.04, 0.06, 0.7), 0.35, 0.45, 0.4)


def hit_bullet_ground(ctx: Ctx) -> np.ndarray:
    n = samples(0.35)
    thud = filt(ctx.noise(n), lp(ctx.jit(1100))) * env(n, ctx.jit(0.012), attack=0.0005)
    dirt = grains(ctx, n, 0.01, 0.15, 220, 0.08, bp(1500, 7000), 0.0015)
    zing = filt(ctx.noise(n), bp(3000, 9000)) * env(n, 0.004, attack=0.0002)
    return thud + 0.5 * dirt + 0.3 * zing


def hit_bullet_armor(ctx: Ctx) -> np.ndarray:
    n = samples(0.45)
    base = ctx.jit(2300, 0.15)
    ping = partials(n, [base, base * 1.61, base * 2.27, base * 3.01], [0.06, 0.04, 0.03, 0.02], [1.0, 0.6, 0.4, 0.25], ctx=ctx)
    click = filt(ctx.noise(n), hp(2500)) * env(n, 0.002, attack=0.0001)
    return 0.8 * ping + 0.6 * click


def hit_ricochet(ctx: Ctx) -> np.ndarray:
    n = samples(1.1)
    base = ctx.jit(700, 0.15)
    clang = partials(n, [base, base * 1.62, base * 2.7, base * 3.7], [0.25, 0.18, 0.12, 0.08], [1.0, 0.7, 0.5, 0.3], ctx=ctx)
    click = filt(ctx.noise(n), hp(1200)) * env(n, 0.004, attack=0.0002)
    # The whine of the deflected round flying off (falling pitch).
    t = times(n)
    whine_env = np.clip((t - 0.03) / 0.05, 0, 1) * np.exp(-np.maximum(t - 0.08, 0) / 0.25)
    whine = sweep(n, ctx.jit(3200), ctx.jit(1300), 0.35, delay=0.03) * whine_env
    x = 0.8 * clang + 0.6 * click + 0.35 * whine
    return fit(reverb(x, ctx, 0.4, 0.3, 3000), 1.1)


def hit_penetration(ctx: Ctx) -> np.ndarray:
    n = samples(1.6)
    base = ctx.jit(300, 0.12)
    clang = partials(n, [base, base * 1.73, base * 2.9, base * 4.5], [0.3, 0.2, 0.12, 0.07], [1.0, 0.7, 0.45, 0.3], ctx=ctx)
    crunch = filt(ctx.noise(n), lp(3200)) * env(n, ctx.jit(0.06), attack=0.001)
    thump = sweep(n, 120, 55, 0.04) * env(n, 0.1, attack=0.002)
    x = 0.7 * clang + 1.0 * crunch + 0.6 * thump
    return fit(reverb(x, ctx, 0.6, 0.35, 2000), 1.6)


def hit_shell_water(ctx: Ctx) -> np.ndarray:
    n = samples(2.0)
    t = times(n)
    rise = np.clip(t / 0.03, 0, 1) * np.exp(-np.maximum(t - 0.03, 0) / ctx.jit(0.35))
    splash = filt(ctx.noise(n), bp(300, 5000)) * rise
    thump = sweep(n, 90, 40, 0.06) * env(n, 0.15, attack=0.003)
    drops = grains(ctx, n, 0.3, 1.6, 90, 0.8, bp(1500, 8000), 0.002)
    x = 1.0 * splash + 0.9 * thump + 0.5 * drops
    return fit(reverb(x, ctx, 0.7, 0.4, 1800), 2.0)


def hit_object(ctx: Ctx) -> np.ndarray:
    n = samples(0.9)
    thud = filt(ctx.noise(n), lp(ctx.jit(900))) * env(n, ctx.jit(0.035), attack=0.001)
    crack = filt(ctx.noise(n), hp(2000)) * env(n, 0.005, attack=0.0002)
    splinters = grains(ctx, n, 0.01, 0.4, 150, 0.2, bp(800, 6000), 0.002)
    x = 1.1 * thud + 0.6 * crack + 0.5 * splinters
    return fit(reverb(x, ctx, 0.35, 0.3, 2200), 0.9)


def explosion_vehicle(ctx: Ctx) -> np.ndarray:
    seconds = 4.8
    n = samples(seconds)
    x = explosion_core(ctx, seconds, 900, 0.35, (50, 24, 0.12, 0.5, 1.4), 1.5, 1.6, 0.6)[:n]
    # Ammunition cooking off and metal debris landing.
    for _ in range(ctx.rng.integers(2, 5)):
        at = ctx.rng.uniform(0.35, 1.8)
        pop = filt(ctx.noise(n), lp(2200)) * env(n, ctx.jit(0.05), attack=0.001, delay=at)
        x += ctx.rng.uniform(0.25, 0.6) * pop
    for _ in range(ctx.rng.integers(4, 8)):
        at = ctx.rng.uniform(0.5, 2.5)
        f = ctx.rng.uniform(500, 1500)
        x += 0.12 * partials(n, [f, f * 1.7, f * 2.6], [0.08, 0.05, 0.03], [1.0, 0.5, 0.3], delay=at, ctx=ctx)
    return x


def shell_incoming(ctx: Ctx) -> np.ndarray:
    # The whistle of a shell in its last 1.5 s: falling pitch, growing louder.
    seconds = 1.5
    n = samples(seconds)
    t = times(n)
    rise = (t / seconds) ** 2.5
    whistle = sweep(n, ctx.jit(1500, 0.12), ctx.jit(650, 0.1), 0.9) * rise
    air = filt(ctx.noise(n), bp(600, 3000)) * rise
    return 0.8 * whistle + 0.25 * air


def crater(ctx: Ctx) -> np.ndarray:
    # Earth and stones raining down after the impact.
    n = samples(1.4)
    rain = grains(ctx, n, 0.0, 1.2, 160, 0.6, bp(200, 2500), 0.004)
    rumble = filt(ctx.noise(n), lp(250)) * env(n, 0.3, attack=0.05)
    return rain + 0.4 * rumble


# --- Recipes: map objects --------------------------------------------------------


def building_damage(ctx: Ctx) -> np.ndarray:
    n = samples(2.0)
    crash = filt(ctx.noise(n), lp(ctx.jit(1200))) * env(n, ctx.jit(0.12), attack=0.003)
    debris = grains(ctx, n, 0.05, 1.5, 120, 0.7, bp(300, 4500), 0.005)
    glass = grains(ctx, n, 0.05, 0.6, 60, 0.3, bp(3500, 10000), 0.0015)
    x = 1.0 * crash + 0.8 * debris + 0.25 * glass
    return fit(reverb(x, ctx, 0.6, 0.35, 1800), 2.0)


def building_collapse(ctx: Ctx) -> np.ndarray:
    seconds = 4.5
    n = samples(seconds)
    t = times(n)
    rumble_env = np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.25, 0) / ctx.jit(1.0))
    rumble = filt(ctx.noise(n), lp(ctx.jit(300))) * rumble_env
    debris = grains(ctx, n, 0.05, 3.0, 200, 1.3, bp(200, 4000), 0.006)
    thuds = np.zeros(n)
    for _ in range(ctx.rng.integers(5, 9)):
        at = ctx.rng.uniform(0.0, 2.2)
        thuds += ctx.rng.uniform(0.3, 1.0) * filt(ctx.noise(n), lp(500)) * env(n, 0.08, attack=0.002, delay=at)
    x = 1.3 * rumble + 0.8 * debris + 0.7 * thuds
    return fit(reverb(x, ctx, 0.9, 0.35, 1200), seconds)


def tree_fall(ctx: Ctx) -> np.ndarray:
    seconds = 3.4
    n = samples(seconds)
    t = times(n)
    fall = ctx.jit(2.0, 0.1)
    # Creaking fibers: a buzzy, wobbling tone through a wooden body.
    wobble = ctx.jit(190) * (1.0 + 0.25 * np.sin(2 * np.pi * 3.1 * t) + 0.1 * ctx.noise(n).cumsum() / np.sqrt(n))
    phase = 2 * np.pi * np.cumsum(wobble) / SR
    saw = 2.0 * ((phase / (2 * np.pi)) % 1.0) - 1.0
    creak_env = np.clip(t / 0.2, 0, 1) * np.clip((fall * 0.65 - t) / 0.2, 0, 1)
    creak = filt(saw * (0.6 + 0.4 * np.abs(np.sin(2 * np.pi * 7 * t))), bp(250, 2200)) * creak_env
    cracks = grains(ctx, n, 0.0, fall * 0.7, 25, 3.0, hp(1200), 0.0012)
    whoosh_env = np.clip((t - fall * 0.55) / (fall * 0.45), 0, 1) ** 2 * (t < fall)
    whoosh = filt(ctx.noise(n), bp(300, 2500)) * whoosh_env
    thud = filt(ctx.noise(n), lp(400)) * env(n, 0.15, attack=0.003, delay=fall)
    thud += 0.8 * sweep(n, 80, 40, 0.05, delay=fall) * env(n, 0.18, attack=0.003, delay=fall)
    branches = grains(ctx, n, fall, fall + 0.8, 250, 0.4, bp(800, 6000), 0.003)
    x = 0.35 * creak + 0.5 * cracks + 0.35 * whoosh + 1.2 * thud + 0.6 * branches
    return fit(reverb(x, ctx, 0.5, 0.3, 2000), seconds)


def wall_break(ctx: Ctx) -> np.ndarray:
    n = samples(1.4)
    thud = filt(ctx.noise(n), lp(ctx.jit(700))) * env(n, ctx.jit(0.06), attack=0.002)
    debris = grains(ctx, n, 0.02, 1.0, 140, 0.5, bp(300, 4000), 0.005)
    x = 1.1 * thud + 0.8 * debris
    return fit(reverb(x, ctx, 0.4, 0.3, 1800), 1.4)


def fence_crush(ctx: Ctx) -> np.ndarray:
    n = samples(0.7)
    cracks = grains(ctx, n, 0.0, 0.35, 60, 0.3, bp(700, 7000), 0.0015)
    thud = filt(ctx.noise(n), lp(800)) * env(n, 0.03, attack=0.001)
    return 1.0 * cracks + 0.5 * thud


# --- Recipes: loops ----------------------------------------------------------------

LOOP_SECONDS = 4.0


def loop_freq(freq: float, seconds: float) -> float:
    """`freq` rounded so it has a whole number of cycles in the loop."""
    return max(1, round(freq * seconds)) / seconds


def circular_noise(ctx: Ctx, n: int, gain) -> np.ndarray:
    return filt(ctx.noise(n), gain)


def circular_place(n: int, positions, amps, ir: np.ndarray) -> np.ndarray:
    """Impulses at sample `positions` convolved circularly with `ir`."""
    pulses = np.zeros(n)
    for p, a in zip(positions, amps):
        pulses[int(p) % n] += a
    padded = np.zeros(n)
    padded[:len(ir)] = ir
    return np.fft.irfft(np.fft.rfft(pulses) * np.fft.rfft(padded), n)


def engine(ctx: Ctx, f0: float, tone_lp: float, roughness: float, sub: float, clank: float = 0.0) -> np.ndarray:
    n = samples(LOOP_SECONDS)
    f0 = loop_freq(ctx.jit(f0, 0.03), LOOP_SECONDS)
    count = int(round(f0 * LOOP_SECONDS))
    period = n / count
    # Firing pulses with a little jitter in time and strength.
    positions = [k * period + ctx.rng.normal(0, period * 0.03) for k in range(count)]
    amps = 1.0 + roughness * ctx.rng.standard_normal(count)
    ir_n = samples(0.03)
    ir = filt(np.concatenate([ctx.noise(ir_n), np.zeros(ir_n)]), lp(900))[:ir_n] * np.exp(-times(ir_n) / 0.006)
    exhaust = circular_place(n, positions, amps, ir)
    t = times(n)
    rumble = sub * np.sin(2 * np.pi * loop_freq(f0 / 2, LOOP_SECONDS) * t)
    rumble += 0.5 * sub * np.sin(2 * np.pi * f0 * t + 0.7)
    mech = circular_noise(ctx, n, bp(400, 3500)) * (0.6 + 0.4 * np.sin(2 * np.pi * f0 * t))
    x = exhaust / (np.std(exhaust) + 1e-9) + rumble + 0.25 * mech / (np.std(mech) + 1e-9)
    if clank > 0.0:
        x += clank * tracks_core(ctx, n, 9.0)
    return filt(x, lp(tone_lp))


def engine_wheel(ctx: Ctx) -> np.ndarray:
    return engine(ctx, 42, 2600, 0.15, 0.5)


def engine_halftrack(ctx: Ctx) -> np.ndarray:
    return engine(ctx, 36, 2200, 0.18, 0.6, clank=0.25)


def engine_track_light(ctx: Ctx) -> np.ndarray:
    return engine(ctx, 46, 2300, 0.2, 0.6)


def engine_track_heavy(ctx: Ctx) -> np.ndarray:
    return engine(ctx, 27, 1500, 0.22, 0.9)


def tracks_core(ctx: Ctx, n: int, rate: float) -> np.ndarray:
    count = int(round(rate * n / SR))
    period = n / count
    positions = [k * period + ctx.rng.normal(0, period * 0.08) for k in range(count)]
    amps = ctx.rng.uniform(0.5, 1.0, count)
    ir_n = samples(0.06)
    base = ctx.jit(900, 0.1)
    ir = partials(ir_n, [base, base * 1.53, base * 2.31, base * 3.4], [0.012, 0.009, 0.007, 0.005], [1.0, 0.7, 0.5, 0.3], ctx=ctx)
    ir += 0.6 * ctx.noise(ir_n) * np.exp(-times(ir_n) / 0.004)
    clicks = circular_place(n, positions, amps, ir)
    return clicks / (np.std(clicks) + 1e-9)


def tracks(ctx: Ctx) -> np.ndarray:
    n = samples(LOOP_SECONDS)
    clatter = tracks_core(ctx, n, 13.0)
    squeal = circular_noise(ctx, n, peak(ctx.jit(1800), 60))
    squeal /= np.std(squeal) + 1e-9
    grind = circular_noise(ctx, n, bp(100, 900))
    grind /= np.std(grind) + 1e-9
    return filt(clatter + 0.15 * squeal + 0.4 * grind, lp(4500))


def fire(ctx: Ctx) -> np.ndarray:
    seconds = 6.0
    n = samples(seconds)
    t = times(n)
    roar = circular_noise(ctx, n, lp(450))
    roar /= np.std(roar) + 1e-9
    roar *= 0.75 + 0.15 * np.sin(2 * np.pi * loop_freq(0.5, seconds) * t) + 0.1 * np.sin(2 * np.pi * loop_freq(1.3, seconds) * t + 1.0)
    count = int(30 * seconds)
    positions = ctx.rng.uniform(0, n, count)
    amps = ctx.rng.exponential(0.5, count)
    ir_n = samples(0.01)
    ir = ctx.noise(ir_n) * np.exp(-times(ir_n) / 0.0015)
    crackle = filt(circular_place(n, positions, amps, ir), bp(900, 5000))
    crackle /= np.std(crackle) + 1e-9
    return roar + 0.35 * crackle



# --- Recipes: stage 2 (prompt 053) ------------------------------------------------


def n_wave(n: int, length: float, at: float = 0.0) -> np.ndarray:
    """The pressure signature of a supersonic projectile: a sudden rise, a
    linear fall through zero and a sudden return (`length` seconds)."""
    out = np.zeros(n)
    i0 = samples(at)
    m = max(2, samples(length))
    out[i0:i0 + m] = np.linspace(1.0, -1.0, m)[:max(0, min(m, n - i0))]
    return out


def crack_bullet(ctx: Ctx) -> np.ndarray:
    # The snap of a bullet passing close by: a sub-millisecond N-wave, the
    # hiss of the air it tears, a slap from the ground.
    n = samples(0.6)
    snap = n_wave(n, ctx.jit(0.0005, 0.25), 0.002)
    snap = filt(snap, hp(400)) * 1.5
    hiss = filt(ctx.noise(n), bp(2500, 12000)) * env(n, ctx.jit(0.006), attack=0.0002, delay=0.002)
    x = snap + 0.35 * hiss
    return fit(reverb(x, ctx, ctx.jit(0.12), 0.25, 5000, predelay=0.008), 0.6)


def crack_shell(ctx: Ctx) -> np.ndarray:
    # A tank or gun shell passing: a heavier N-wave and a tearing whoosh.
    n = samples(1.0)
    snap = filt(n_wave(n, ctx.jit(0.0022, 0.2), 0.12), hp(150)) * 1.4
    t = times(n)
    swell = np.exp(-((t - 0.12) / ctx.jit(0.07)) ** 2)
    whoosh = filt(ctx.noise(n), bp(500, 5000)) * swell
    x = snap + 0.45 * whoosh
    return fit(reverb(x, ctx, ctx.jit(0.25), 0.3, 3500, predelay=0.012), 1.0)


def explosion_tail(ctx: Ctx) -> np.ndarray:
    # The long roll of a big blast: low rumble, echoes off woods and hills.
    seconds = 5.0
    n = samples(seconds)
    t = times(n)
    roll = np.clip(t / 0.12, 0, 1) * np.exp(-t / ctx.jit(1.3))
    x = filt(ctx.noise(n), lp(ctx.jit(260))) * roll
    for _ in range(ctx.rng.integers(3, 6)):
        at = ctx.rng.uniform(0.3, 2.2)
        x += ctx.rng.uniform(0.25, 0.5) * filt(ctx.noise(n), lp(400)) * env(n, ctx.jit(0.35), attack=0.05, delay=at)
    x += 0.5 * sweep(n, 45, 24, 0.4) * env(n, 0.8, attack=0.03)
    return fit(x, seconds)


def flak_burst(ctx: Ctx) -> np.ndarray:
    # A heavy flak shell bursting high up: sharp, little body, open sky.
    n = samples(2.6)
    crack = filt(ctx.noise(n), hp(1000)) * env(n, 0.008, attack=0.0003)
    body = filt(ctx.noise(n), bp(120, 2000)) * env(n, ctx.jit(0.06), attack=0.002)
    boom = sweep(n, ctx.jit(90), ctx.jit(40), 0.05) * env(n, ctx.jit(0.12), attack=0.003)
    x = 0.9 * crack + 1.0 * body + 0.8 * boom
    return fit(reverb(x, ctx, ctx.jit(0.8), 0.6, 1600, predelay=0.04), 2.6)


def prop_engine(ctx: Ctx, engines: list[tuple[float, float]], tone_lp: float, air: float) -> np.ndarray:
    """Propeller aircraft: per engine `(blade pass Hz, level)`, the blade
    pass with its harmonics, the cylinders' buzz four times above, and the
    rush of air. Slightly detuned engines beat against each other."""
    n = samples(LOOP_SECONDS)
    t = times(n)
    x = np.zeros(n)
    for f, level in engines:
        f = loop_freq(f, LOOP_SECONDS)
        for k, a in enumerate([1.0, 0.7, 0.45, 0.3, 0.2, 0.12], start=1):
            x += level * a * np.sin(2 * np.pi * loop_freq(f * k, LOOP_SECONDS) * t + ctx.rng.uniform(0, 2 * np.pi))
        buzz_f = loop_freq(f * 4.0, LOOP_SECONDS)
        buzz = np.sign(np.sin(2 * np.pi * buzz_f * t)) * (0.6 + 0.4 * np.sin(2 * np.pi * f * t))
        x += level * 0.25 * filt(buzz, lp(1800))
    rush = circular_noise(ctx, n, bp(300, 4000))
    x = x / (np.std(x) + 1e-9) + air * rush / (np.std(rush) + 1e-9)
    return filt(x, lp(tone_lp))


def engine_aircraft(ctx: Ctx) -> np.ndarray:
    return prop_engine(ctx, [(ctx.jit(68, 0.03), 1.0)], 3200, 0.45)


def engine_bomber(ctx: Ctx) -> np.ndarray:
    base = ctx.jit(46, 0.03)
    return prop_engine(ctx, [(base, 1.0), (base + 0.75, 0.9), (base + 1.5, 0.6)], 2200, 0.35)


def fire_large(ctx: Ctx) -> np.ndarray:
    # A burning house or wood: a deep roar that swells with the draught,
    # dense crackle and timber popping.
    seconds = 8.0
    n = samples(seconds)
    t = times(n)
    roar = circular_noise(ctx, n, lp(280))
    roar /= np.std(roar) + 1e-9
    roar *= 0.7 + 0.2 * np.sin(2 * np.pi * loop_freq(0.25, seconds) * t) + 0.1 * np.sin(2 * np.pi * loop_freq(0.9, seconds) * t + 2.0)
    count = int(55 * seconds)
    ir_n = samples(0.012)
    ir = ctx.noise(ir_n) * np.exp(-times(ir_n) / 0.002)
    crackle = filt(circular_place(n, ctx.rng.uniform(0, n, count), ctx.rng.exponential(0.5, count), ir), bp(700, 4500))
    crackle /= np.std(crackle) + 1e-9
    pops_n = int(1.2 * seconds)
    pop_ir_n = samples(0.08)
    pop_ir = filt(np.concatenate([ctx.noise(pop_ir_n), np.zeros(pop_ir_n)]), lp(1500))[:pop_ir_n] * np.exp(-times(pop_ir_n) / 0.02)
    pops = circular_place(n, ctx.rng.uniform(0, n, pops_n), ctx.rng.uniform(0.5, 1.0, pops_n), pop_ir)
    pops /= np.std(pops) + 1e-9
    return 1.2 * roar + 0.4 * crackle + 0.25 * pops

# --- Recipes: UI ---------------------------------------------------------------------


def ui_click(ctx: Ctx) -> np.ndarray:
    n = samples(0.07)
    f = 1800 * (1.0 + 0.06 * ctx.variant)
    blip = partials(n, [f, f * 2.0], [0.012, 0.006], [1.0, 0.3])
    click = filt(ctx.noise(n), hp(3000)) * env(n, 0.0015, attack=0.0001)
    return blip + 0.3 * click


def ui_select(ctx: Ctx) -> np.ndarray:
    n = samples(0.16)
    f = 1150 * (1.0 + 0.05 * ctx.variant)
    return partials(n, [f], [0.025], [1.0]) + partials(n, [f * 1.335], [0.03], [0.9], delay=0.05)


def ui_order(ctx: Ctx) -> np.ndarray:
    n = samples(0.2)
    squelch = filt(ctx.noise(n), bp(800, 3500)) * env(n, 0.012, attack=0.0005)
    f = 880 * (1.0 + 0.05 * ctx.variant)
    chirp = sweep(n, f, f * 1.5, 0.03, delay=0.025) * env(n, 0.05, attack=0.003, delay=0.025)
    return 0.4 * squelch + 0.8 * chirp


def bell(n: int, freq: float, delay: float, decay: float = 0.6) -> np.ndarray:
    return partials(n, [freq, freq * 2.0, freq * 2.76, freq * 5.4], [decay, decay * 0.6, decay * 0.4, decay * 0.2], [1.0, 0.35, 0.25, 0.1], delay=delay)


def ui_build_complete(ctx: Ctx) -> np.ndarray:
    n = samples(1.2)
    return bell(n, 523.25, 0.0) + bell(n, 659.25, 0.12) + bell(n, 783.99, 0.24, 0.8)


def ui_unit_ready(ctx: Ctx) -> np.ndarray:
    n = samples(0.9)
    return bell(n, 587.33, 0.0, 0.4) + bell(n, 783.99, 0.13, 0.6)


def ui_under_attack(ctx: Ctx) -> np.ndarray:
    n = samples(0.8)
    odd = [1.0, 0.0, 0.33, 0.0, 0.2, 0.0, 0.14]
    x = tone(n, 440, odd, 0.0, 0.14, attack=0.005, release=0.02)
    x += tone(n, 370, odd, 0.17, 0.14, attack=0.005, release=0.02)
    x += tone(n, 440, odd, 0.36, 0.14, attack=0.005, release=0.02)
    x += tone(n, 370, odd, 0.53, 0.14, attack=0.005, release=0.03)
    return filt(x, lp(3000))


def ui_unit_lost(ctx: Ctx) -> np.ndarray:
    n = samples(0.9)
    soft = [1.0, 0.3, 0.1]
    return tone(n, 329.63, soft, 0.0, 0.18, release=0.05) + tone(n, 220.0, soft, 0.2, 0.3, release=0.15)


def brass(n: int, freq: float, delay: float, length: float) -> np.ndarray:
    return tone(n, freq, [1.0, 0.7, 0.5, 0.35, 0.22, 0.12, 0.07], delay, length, attack=0.03, release=0.12, vibrato=0.004)


def ui_victory(ctx: Ctx) -> np.ndarray:
    # A bugle call: only notes of the harmonic series of C.
    n = samples(2.6)
    x = brass(n, 392.0, 0.0, 0.16) + brass(n, 523.25, 0.2, 0.16) + brass(n, 659.25, 0.4, 0.16)
    x += brass(n, 783.99, 0.6, 1.2)
    return filt(reverb(x, ctx, 0.5, 0.25, 4000), lp(5000))[:n]


def ui_defeat(ctx: Ctx) -> np.ndarray:
    n = samples(2.8)
    dark = [1.0, 0.5, 0.25, 0.12]
    x = tone(n, 392.0, dark, 0.0, 0.5, attack=0.05, release=0.15)
    x += tone(n, 311.13, dark, 0.6, 0.5, attack=0.05, release=0.15)
    x += tone(n, 261.63, dark, 1.2, 1.0, attack=0.05, release=0.3)
    return filt(reverb(x, ctx, 0.6, 0.3, 3000), lp(3500))[:n]


RECIPES = {name: globals()[name] for name in [
    "shot_rifle", "shot_mg", "shot_cannon_small", "shot_cannon_medium", "shot_cannon_large", "shot_mortar",
    "shot_howitzer", "shot_rocket", "hit_bullet_ground", "hit_bullet_armor", "hit_shell_small", "hit_shell", "hit_shell_water",
    "hit_ricochet", "hit_penetration", "hit_object", "explosion_vehicle", "shell_incoming", "crater", "building_damage",
    "building_collapse", "tree_fall", "wall_break", "fence_crush", "engine_wheel", "engine_halftrack",
    "engine_track_light", "engine_track_heavy", "tracks", "fire", "ui_click", "ui_select", "ui_order",
    "ui_build_complete", "ui_unit_ready", "ui_under_attack", "ui_unit_lost", "ui_victory", "ui_defeat",
    "crack_bullet", "crack_shell", "explosion_tail", "flak_burst", "engine_aircraft", "engine_bomber", "fire_large",
]}


# --- Pipeline ------------------------------------------------------------------------


def finish(x: np.ndarray, loop: bool) -> np.ndarray:
    """Trims trailing silence (one-shots), fades, normalizes the peak."""
    x = np.asarray(x, dtype=np.float64)
    x = x - np.mean(x) if loop else x
    if not loop:
        level = np.abs(x)
        threshold = np.max(level) * 10 ** (-60 / 20)
        loud = np.nonzero(level > threshold)[0]
        if len(loud):
            x = x[loud[0]:loud[-1] + 1]
        fade_in = min(samples(0.002), len(x))
        x[:fade_in] *= np.linspace(0, 1, fade_in)
        fade_out = min(samples(0.03), len(x))
        x[-fade_out:] *= np.linspace(1, 0, fade_out)
    top = np.max(np.abs(x)) + 1e-12
    return x * (10 ** (PEAK_DBFS / 20) / top)


def write_wav(path: Path, x: np.ndarray, loop: bool) -> None:
    pcm = np.clip(np.round(x * 32767), -32768, 32767).astype("<i2").tobytes()
    chunks = b"fmt " + struct.pack("<IHHIIHH", 16, 1, 1, SR, SR * 2, 2, 16)
    if loop:
        # smpl chunk with one forward loop over the whole file.
        smpl = struct.pack("<IIIIIIIII", 0, 0, int(1e9 / SR), 60, 0, 0, 0, 1, 0)
        smpl += struct.pack("<IIIIII", 0, 0, 0, len(x) - 1, 0, 0)
        chunks += b"smpl" + struct.pack("<I", len(smpl)) + smpl
    chunks += b"data" + struct.pack("<I", len(pcm)) + pcm
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"RIFF" + struct.pack("<I", 4 + len(chunks)) + b"WAVE" + chunks)


def read_external(path: Path) -> np.ndarray:
    """Mono float samples at SR of any file (wav directly, else ffmpeg)."""
    if path.suffix.lower() == ".wav":
        try:
            with wave.open(str(path)) as w:
                if w.getsampwidth() == 2 and w.getframerate() == SR:
                    data = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768
                    return data.reshape(-1, w.getnchannels()).mean(axis=1)
        except wave.Error:
            pass
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        raise SystemExit(f"{path}: needs ffmpeg to convert (or save it as 16-bit 44.1 kHz wav)")
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "x.wav"
        subprocess.run([ffmpeg, "-loglevel", "error", "-i", str(path), "-ac", "1", "-ar", str(SR), "-c:a", "pcm_s16le", str(out)], check=True)
        with wave.open(str(out)) as w:
            return np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768


def trim_leading(x: np.ndarray) -> np.ndarray:
    level = np.abs(x)
    loud = np.nonzero(level > np.max(level) * 10 ** (-50 / 20))[0]
    return x[max(0, loud[0] - samples(0.002)):] if len(loud) else x


def level_speech(x: np.ndarray, rms_dbfs: float) -> np.ndarray:
    """Scales speech so its loud blocks have the RMS `rms_dbfs`, but never
    above the peak limit."""
    n = samples(0.05)
    if len(x) < n:
        return x
    rms = np.sqrt((x[: len(x) // n * n].reshape(-1, n) ** 2).mean(axis=1))
    loud = rms[rms > rms.max() * 10 ** (-20 / 20)]
    gain = 10 ** (rms_dbfs / 20) / (np.sqrt((loud ** 2).mean()) + 1e-12)
    return x * min(gain, 10 ** (PEAK_DBFS / 20) / (np.max(np.abs(x)) + 1e-12))


def file_slug(name: str) -> str:
    """A file name of lower case letters, digits and `_` (download names of
    voice lines carry spaces and commas)."""
    import re
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def class_seed(name: str, variant: int) -> int:
    return (sum(ord(c) * 131 ** i for i, c in enumerate(name)) + variant * 7919) % (2 ** 32)


def weapon_slug(name: str) -> str:
    """Folder name of a catalog weapon (same rule as `sound_classes.gd`):
    without parentheses (mounts: "Browning M2HB (AA)" -> browning_m2hb)."""
    import re
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


def list_weapons() -> int:
    """Prints every catalog weapon with a caliber: slug, name, files."""
    import re
    seen: dict[str, str] = {}
    for path in sorted(CATALOG_DIR.glob("*.toml")):
        for m in re.finditer(r'\{\s*name\s*=\s*"([^"]+)"[^}]*caliber_mm_x10', path.read_text()):
            seen.setdefault(weapon_slug(m.group(1)), m.group(1))
    for slug in sorted(seen):
        folder = EXTERNAL_DIR / "weapon" / slug
        count = len([p for p in folder.glob("*") if p.suffix.lower() in SOURCE_EXTENSIONS]) if folder.is_dir() else 0
        print(f"{slug:28s} {seen[slug]:28s} {count} file(s) in assets-src/audio/weapon/{slug}/")
    return 0


def prepare_keyed(kind: str) -> int:
    """Prepares `assets-src/audio/<kind>/**` into `godot/assets/audio/<kind>/**`
    (one folder per weapon or per faction and callout). Returns the files."""
    src_root = EXTERNAL_DIR / kind
    out_root = AUDIO_DIR / kind
    if out_root.is_dir():
        for old in out_root.rglob("*.wav"):
            old.unlink()
    # (prefix, folder under `out_root`, source file)
    sources: list[tuple[str, Path, Path]] = []
    for prefix, root in (("ext", src_root), ("priv", PRIVATE_DIR / kind)):
        if root.is_dir():
            sources += [(prefix, path.parent.relative_to(root), path) for path in sorted(root.rglob("*"))]
    if kind == "announce":
        for message, folder in ANNOUNCE.items():
            for language, faction in ANNOUNCE_FACTIONS.items():
                lines = PRIVATE_DIR / "elevenlabs" / folder / language
                sources += [("priv", Path(faction) / message, path) for path in sorted(lines.glob("*"))]
    total = 0
    for prefix, rel, path in sources:
        if path.suffix.lower() not in SOURCE_EXTENSIONS:
            continue
        x = finish(trim_leading(read_external(path)), False)
        stem = path.stem
        if kind == "announce":
            x = level_speech(x, ANNOUNCE_RMS_DBFS)
            stem = file_slug(stem)
        write_wav(out_root / rel / f"{prefix}_{stem}.wav", x, False)
        total += 1
    print(f"{kind:20s} {total} file(s) (external, keyed)")
    return total


def main(argv: list[str]) -> int:
    if "--weapons" in argv:
        return list_weapons()
    extra_dir = None
    if "--wav-dir" in argv:
        i = argv.index("--wav-dir")
        extra_dir = Path(argv[i + 1])
        argv = argv[:i] + argv[i + 2:]
    manifest = json.loads(MANIFEST.read_text())
    classes = manifest["classes"]
    wanted = argv or list(classes) + KEYED
    if "--private" in argv:
        # Only the classes and keyed folders with private files (their public
        # files are written again: the game plays the private ones there).
        if not PRIVATE_DIR.is_dir():
            raise SystemExit(f"--private: no private sound repository at {PRIVATE_DIR} (or MUD_AND_STEEL_SFX)")
        wanted = [n for n in list(classes) + KEYED if (PRIVATE_DIR / n).is_dir() or n == "announce"]
    total = 0
    for name in wanted:
        if name in KEYED:
            total += prepare_keyed(name)
            continue
        if name not in classes:
            raise SystemExit(f"unknown sound class '{name}' (see {MANIFEST})")
        spec = classes[name]
        loop = bool(spec.get("loop", False))
        out_dir = AUDIO_DIR / name
        external = sorted(p for p in (EXTERNAL_DIR / name).glob("*") if p.suffix.lower() in {".wav", ".ogg", ".mp3", ".flac"}) if (EXTERNAL_DIR / name).is_dir() else []
        # Old generated files of this class go; the import files of files
        # that come back keep their uid.
        if out_dir.is_dir():
            for old in out_dir.glob("*.wav"):
                old.unlink()
        files: list[tuple[str, np.ndarray]] = []
        if external:
            for path in external:
                files.append((f"ext_{path.stem}", finish(trim_leading(read_external(path)), loop)))
        else:
            recipe = RECIPES.get(name)
            if recipe is None:
                raise SystemExit(f"no recipe and no files in {EXTERNAL_DIR / name} for '{name}'")
            for v in range(int(spec.get("variants", 1))):
                files.append((f"{name}_{v + 1:02d}", finish(recipe(Ctx(class_seed(name, v), v)), loop)))
        private = sorted(p for p in (PRIVATE_DIR / name).glob("*") if p.suffix.lower() in SOURCE_EXTENSIONS) if (PRIVATE_DIR / name).is_dir() else []
        for path in private:
            files.append((f"priv_{path.stem}", finish(trim_leading(read_external(path)), loop)))
        for stem, x in files:
            write_wav(out_dir / f"{stem}.wav", x, loop)
            if extra_dir is not None:
                write_wav(extra_dir / f"{stem}.wav", x, loop)
            total += 1
        print(f"{name:20s} {len(files)} file(s){' (external)' if external else ''}{f', {len(private)} private' if private else ''}")
    # Import files of variants that no longer exist.
    for stale in AUDIO_DIR.rglob("*.wav.import"):
        if not stale.with_suffix("").exists():
            stale.unlink()
    print(f"{total} files in {AUDIO_DIR.relative_to(ROOT)}; run `just import` next")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
